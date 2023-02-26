use actix_web::{guard, get, web::{self, Data, Json, Path, Form}, Responder, HttpResponse};
use mongodb::{bson::{doc, oid::ObjectId}, results::InsertOneResult};
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result, anyhow, bail};
use log::{error, warn, info};
use meilisearch_sdk::errors::{Error, ErrorCode, MeilisearchError};
use mongodb::{Client, options::FindOptions};
use actix_easy_multipart::MultipartForm;
use actix_easy_multipart::tempfile::Tempfile;
use std::fs::File;

use crate::gen::anime::*;
use crate::types::*;
use crate::middlewares::auth::{Role, RequireRoleGuard};

const CACHE_KEY_ALPHABET: &str = "ABCDEFGHIJKMNOPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz0123456789";

const DB_NAME: &str = "Kanime3";
const COLL_NAME: &str = "animes";
const ANIMES_INDEX: &str = "animes";
const ANIMES_INDEX_BATCH_SIZE: usize = 32;
const ANIMES_SEARCH_QUERY_MIN_LEN: usize = 2;
const ANIMES_SEARCH_QUERY_MAX_LEN: usize = 128;
const ANIMES_SEARCH_DEFAULT_LIMIT: u32 = 10;
const ANIMES_SEARCH_SOFT_LIMIT: u32 = 100;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SearchQuery {
    query: String,
    offset: Option<u32>,
    limit: Option<u32>,
}

impl SearchQuery {
    pub fn validate(&self) -> bool {
        self.query.len() >= ANIMES_SEARCH_QUERY_MIN_LEN &&
            self.query.len() <= ANIMES_SEARCH_QUERY_MAX_LEN
    }
}

fn to_oid(id: String) -> Option<ObjectId> {
    if id.len() != 24 { // ObjectId length
        return None;
    }
    ObjectId::parse_str(&id).ok()
}

pub async fn sync_meilisearch(mongodb: &Client, meilisearch: &meilisearch_sdk::Client) -> Result<()> {
    let index = match meilisearch.get_index(ANIMES_INDEX).await {
        Ok(index) => index,
        Err(Error::Meilisearch(MeilisearchError { error_code: ErrorCode::IndexNotFound, .. })) => {
            let index = meilisearch
                .create_index(ANIMES_INDEX, Some(ANIME_PRIMARY_KEY)).await?
                .wait_for_completion(&meilisearch, None, None).await?
                .try_make_index(&meilisearch)
                .map_err(|t| anyhow!("Failed to create index `{ANIMES_INDEX}`: {t:?}"))?;
            info!(target: "meilisearch","Successfully created index `{ANIMES_INDEX}`");

            index.set_searchable_attributes(&["titles", "author"]).await?
                .wait_for_completion(&meilisearch, None, None).await?;
            info!(target: "meilisearch","Setup completed for index `{ANIMES_INDEX}`");
            index
        },
        Err(e) => bail!("{e}"),
    };

    let col: mongodb::Collection<WithOID<AnimeSeries>> = mongodb.database(DB_NAME).collection(COLL_NAME);
    let anime_count = col.count_documents(None, None).await? as usize;

    let index_stats = index.get_stats().await?;
    if index_stats.number_of_documents == anime_count {
        return Ok(());
    }
    info!(target: "meilisearch",
        "Sync required for index `{ANIMES_INDEX}`: entry count mismatch, expected {anime_count} but found {}",
        index_stats.number_of_documents);

    let mut cur = col
        .find(doc! {}, FindOptions::builder()
            .batch_size(ANIMES_INDEX_BATCH_SIZE as u32).build())
        .await?;
    let mut queue: Vec<AnimeSeriesSearchEntry>
        = Vec::with_capacity(ANIMES_INDEX_BATCH_SIZE);
    while cur.advance().await? {
        let current: WithOID<AnimeSeries> = cur.deserialize_current()?;
        queue.push(current.into());
        if queue.len() == ANIMES_INDEX_BATCH_SIZE {
            index.add_or_replace(&queue, Some(ANIME_PRIMARY_KEY)).await?
                .wait_for_completion(&meilisearch, None, None).await?;
            queue.clear();
        }
    }
    if !queue.is_empty() {
        index.add_or_replace(&queue, Some(ANIME_PRIMARY_KEY)).await?
            .wait_for_completion(&meilisearch, None, None).await?;
    }
    info!(target: "meilisearch", "Sync completed successfully!");

    Ok(())
}

async fn search_animes(query: SearchQuery, app: Data<AppState>) -> HttpResponse {
    if !query.validate() {
        return KError::bad_request("Query length must be between 2 and 128 characters");
    }

    let results = app.meilisearch
        .index(ANIMES_INDEX)
        .search()
        .with_query(&query.query)
        .with_offset(query.offset.unwrap_or(0) as usize)
        .with_limit(query.limit.unwrap_or(ANIMES_SEARCH_DEFAULT_LIMIT)
            .min(ANIMES_SEARCH_SOFT_LIMIT) as usize)
        .execute()
        .await;

    match results {
        Ok(docs) => {
            let docs: Vec<AnimeSeriesSearchEntry> = docs.hits.into_iter()
                .map(|r| r.result).collect();
            HttpResponse::Ok().json(docs)
        }
        Err(e) => {
            error!("Could not search: {e:?}");
            KError::internal_error("Could not perform search")
        }
    }
}

pub async fn search_anime_form(form: Form<SearchQuery>, app: Data<AppState>) -> impl Responder {
    search_animes(form.into_inner(), app).await
}

pub async fn search_anime_json(json: Json<SearchQuery>, app: Data<AppState>) -> impl Responder {
    search_animes(json.into_inner(), app).await
}

async fn find_anime(anime_id: &ObjectId, app: &AppState) -> Result<Option<WithOID<AnimeSeries>>> {
    let collection = app.mongodb.database(DB_NAME)
        .collection(COLL_NAME);
    collection.find_one(doc! { "_id": anime_id }, None)
        .await.context("Finding anime with the specified ID")
}

#[get("/anime/{id}")]
pub async fn fetch_anime_details(path: Path<String>, app: Data<AppState>) -> impl Responder {
    let Some(anime_id) = to_oid(path.into_inner()) else {
        return KError::bad_request("The provided ID is not valid");
    };
    match find_anime(&anime_id, &app).await {
        Ok(Some(anime)) => {
            let renamed: WithID<AnimeSeries> = anime.into();
            HttpResponse::Ok().json(renamed)
        },
        Ok(None) => KError::not_found(),
        Err(e) => {
            error!("Could not find anime: {e:?}");
            KError::db_error()
        }
    }
}

async fn send_anime_to_meili(anime: AnimeSeriesSearchEntry, app: &AppState) -> Result<()> {
    app.meilisearch.get_index(ANIMES_INDEX)
        .await?
        .add_or_replace(&[anime], Some(ANIME_PRIMARY_KEY))
        .await?
        .wait_for_completion(&app.meilisearch, None, None)
        .await?;
    Ok(())
}

#[derive(MultipartForm)]
struct AnimeMultipartCandidate {
    candidate: actix_easy_multipart::json::Json<AnimeSeriesCandidate>,
    poster: Tempfile,
}

async fn push_anime(form: MultipartForm<AnimeMultipartCandidate>, app: Data<AppState>) -> HttpResponse {
    let form = form.into_inner();
    let mut anime = {
        let key: String = random_string::generate(20, CACHE_KEY_ALPHABET);
        let candidate = form.candidate.into_inner();
        candidate.into_anime(CachedImage::new(key))
    };

    let poster = form.poster;
    match poster.content_type.as_ref().map(AsRef::as_ref) {
        Some("image/webp") | Some("image/png") => {
            match export_poster(anime.poster.key().to_string(), poster.file.path(), &app.cache_folder) {
                Ok(ci) => {
                    anime.poster = ci;
                    export_presenter(&anime, &app.cache_folder)
                        .unwrap_or_else(|_| warn!("Could not generate presenter"));
                },
                Err(e) => {
                    error!("Could not export poster: {e:?}");
                    poster.file.close().unwrap_or_else(|_| warn!("Could not delete temp file"));
                    return KError::internal_error("Could not generate image set")
                }
            }
            poster.file.close().unwrap_or_else(|_| warn!("Could not delete temp file"));
        },
        _ => {
            poster.file.close().unwrap_or_else(|_| warn!("Could not delete temp file"));
            return KError::bad_request("Only webp or png images are supported")
        }
    }

    let collection: mongodb::Collection<AnimeSeries> =
        app.mongodb.database(DB_NAME).collection(COLL_NAME);
    match collection.insert_one(&anime, None).await {
        Ok(InsertOneResult { inserted_id, .. }) => {
            let inserted_id = inserted_id.as_object_id()
                .expect("Value must be ObjectId").to_hex();
            let anime = WithID::new(inserted_id, anime);
            if let Err(e) = send_anime_to_meili(anime.clone().into(), &app).await {
                warn!("Could not add pushed anime to meilisearch: {e:?}");
            }
            HttpResponse::Created().json(anime)
        },
        Err(e) => {
            // TODO: delete generated poster files
            error!("Could not push anime to db: {e:?}");
            KError::db_error()
        }
    }
}

#[derive(MultipartForm)]
struct AnimeMultipartPatch {
    patch: actix_easy_multipart::json::Json<AnimeSeriesPatch>,
    poster: Option<Tempfile>,
}

async fn apply_anime_search_entry_patch(app: &AppState, patch: AnimeSeriesSearchEntryPatch) -> Result<()> {
    app.meilisearch.get_index(ANIMES_INDEX)
        .await?
        .add_or_update(&[patch], Some(ANIME_PRIMARY_KEY))
        .await?
        .wait_for_completion(&app.meilisearch, None, None)
        .await?;
    Ok(())
}

async fn apply_anime_patch(anime_id: &ObjectId, app: &AppState, mut patch: AnimeSeriesPatch)
    -> Result<bool> {
    let collection: mongodb::Collection<AnimeSeries> =
        app.mongodb.database(DB_NAME).collection(COLL_NAME);
    let res = collection
        .update_one(doc! { "_id": anime_id }, doc! { "$set": patch.seal()? }, None)
        .await
        .context("Updating anime with the specified ID")?;
    if res.matched_count == 0 {
        return Ok(false);
    }
    if let Some(patch) = AnimeSeriesSearchEntryPatch::from_patch(anime_id.to_hex(), patch) {
        apply_anime_search_entry_patch(app, patch).await
            .unwrap_or_else(|e| warn!("Could not update meilisearch index: {e:?}"));
    }
    Ok(true)
}

async fn patch_anime(path: Path<String>, form: MultipartForm<AnimeMultipartPatch>,
    app: Data<AppState>) -> HttpResponse {
    let Some(anime_id) = to_oid(path.into_inner()) else {
        return KError::bad_request("The provided ID is not valid");
    };
    let form = form.into_inner();
    let mut patch = form.patch.into_inner();
    if patch.is_empty() && form.poster.is_none() {
        return KError::bad_request("Patch is empty")
    }

    if let Some(poster) = form.poster {
        match poster.content_type.as_ref().map(AsRef::as_ref) {
            Some("image/webp") | Some("image/png") => {
                let Ok(Some(anime)) = find_anime(&anime_id, &app).await else {
                    poster.file.close().unwrap_or_else(|_| warn!("Could not delete temp file"));
                    return KError::bad_request("The provided ID is not valid");
                };
                let key = anime.as_ref().poster.key().to_string();
                match export_poster(key, poster.file.path(), &app.cache_folder) {
                    Ok(ci) => {
                        patch.set_poster(ci);
                        export_presenter(&anime, &app.cache_folder)
                            .unwrap_or_else(|_| warn!("Could not generate presenter"));
                    },
                    Err(e) => {
                        error!("Could not export poster: {e:?}");
                        if patch.is_empty() {
                            poster.file.close().unwrap_or_else(|_| warn!("Could not delete temp file"));
                            return KError::internal_error("Could not generate image set")
                        }
                    }
                }
                poster.file.close().unwrap_or_else(|_| warn!("Could not delete temp file"));
            },
            _ => {
                poster.file.close().unwrap_or_else(|_| warn!("Could not delete temp file"));
                return KError::bad_request("Only webp or png images are supported")
            }
        }
    } else if patch.has_presenter_changes() {
        let Ok(Some(anime)) = find_anime(&anime_id, &app).await else {
            return KError::bad_request("The provided ID is not valid");
        };
        match export_presenter(anime, &app.cache_folder) {
            Ok(()) => info!("Successfully updated presenter for `{}`", anime_id.to_hex()),
            Err(e) => warn!("Could not generate presenter image: {e:?}")
        }
    }

    match apply_anime_patch(&anime_id, &app, patch).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => KError::not_found(),
        Err(e) => {
            error!("Could not find anime:\n{e:?}");
            KError::db_error()
        }
    }
}

fn create_backup(anime: &WithID<AnimeSeries>) -> anyhow::Result<()> {
    let backup = File::create(format!("{}.deleted.json", anime.id))?;
    match serde_json::to_writer(backup, &anime) {
        Err(_) => {
            let json = serde_json::to_string(&anime)?;
            warn!("Could not save backup file, anime = `{json}`");
            Ok(())
        }
        _ => {
            info!("Successfully backed up deleted anime");
            Ok(())
        }
    }
}

async fn find_and_delete(anime_id: &ObjectId, app: &AppState) -> Result<Option<WithOID<AnimeSeries>>> {
    let collection: mongodb::Collection<WithOID<AnimeSeries>> =
        app.mongodb.database(DB_NAME).collection(COLL_NAME);
    collection.find_one_and_delete(doc! { "_id": anime_id }, None).await
        .context("Find one and delete anime")
}

async fn delete_from_meili(anime_id: &str, app: &AppState) -> Result<()> {
    app.meilisearch.get_index(ANIMES_INDEX).await?
        .delete_document(anime_id).await?
        .wait_for_completion(&app.meilisearch, None, None).await?;
    Ok(())
}

async fn delete_anime(path: Path<String>, app: Data<AppState>) -> HttpResponse {
    let Some(anime_id) = to_oid(path.into_inner()) else {
        return KError::bad_request("The provided ID is not valid");
    };
    match find_and_delete(&anime_id, &app).await {
        Ok(Some(anime)) => {
            let anime: WithID<AnimeSeries> = anime.into();
            create_backup(&anime)
                .unwrap_or_else(|e| error!("Could not save backup file `{anime:?}`: {e:?}"));

            if let Err(e) = delete_from_meili(&anime.id, &app).await {
                warn!("Could not remove deleted anime from meilisearch: {e:?}");
            }

            HttpResponse::NoContent().finish()
        },
        Ok(None) => KError::not_found(),
        Err(e) => {
            error!("Could not find anime: {e:?}");
            KError::db_error()
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/search")
        .guard(guard::Header("content-type", "application/json"))
        .route(web::post().to(search_anime_json)));
    cfg.service(web::resource("/search")
        .guard(guard::Header("content-type", "application/x-www-form-urlencoded"))
        .route(web::post().to(search_anime_form)));

    let admin_only = RequireRoleGuard(Role::Admin);
    cfg.service(web::resource("/s/anime")
        .route(web::post().guard(admin_only).to(push_anime)));

    cfg.service(web::resource("/s/anime/{id}")
        .route(web::patch().guard(admin_only).to(patch_anime))
        .route(web::delete().guard(admin_only).to(delete_anime)));

    cfg.service(fetch_anime_details);
}
