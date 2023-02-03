use actix_web::{guard, get, web, Responder, HttpResponse};
use validator::Validate;
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result, anyhow, bail};
use log::{error, warn, info};
use meilisearch_sdk::errors::{Error, ErrorCode, MeilisearchError};
use mongodb::{Client, options::FindOptions};
use std::fs::File;

use crate::types::*;
use crate::middlewares::auth::{KanimeAuth, RequireAdminGuard};

const DB_NAME: &str = "Kanime3";
const COLL_NAME: &str = "animes";
const ANIMES_INDEX: &str = "animes";
const ANIMES_SEARCH_DEFAULT_LIMIT: u32 = 10;
const ANIMES_SEARCH_SOFT_LIMIT: u32 = 100;

#[derive(Deserialize, Serialize, Validate, Debug, Clone)]
pub struct SearchQuery {
    #[validate(length(min = 1, max = 128))]
    query: String,
    offset: Option<u32>,
    limit: Option<u32>,
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
            info!("Successfully created index `{ANIMES_INDEX}`");

            index.set_searchable_attributes(&["titles", "author"]).await?
                .wait_for_completion(&meilisearch, None, None).await?;
            info!("Setup completed for index `{ANIMES_INDEX}`");
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

    let batch_size = 32;
    let mut cur = col.find(doc! {}, FindOptions::builder().batch_size(batch_size).build())
        .await?;
    let batch_size = batch_size as usize;
    let mut queue: Vec<AnimeSeriesSearchEntry> = Vec::with_capacity(batch_size);
    while cur.advance().await? {
        let current: WithOID<AnimeSeries> = cur.deserialize_current()?;
        queue.push(current.into());
        if queue.len() == batch_size {
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

async fn search_animes(query: SearchQuery, app: web::Data<AppState>) -> HttpResponse {
    match query.validate() {
        Ok(_) => (),
        Err(e) => return KError::bad_request(e.to_string())
    };

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
            error!("Could not search: {e}");
            KError::internal_error("Could not perform search".to_string())
        }
    }
}

pub async fn search_anime_form(form: web::Form<SearchQuery>, app: web::Data<AppState>) -> impl Responder {
    search_animes(form.into_inner(), app).await
}

pub async fn search_anime_json(json: web::Json<SearchQuery>, app: web::Data<AppState>) -> impl Responder {
    search_animes(json.into_inner(), app).await
}

async fn find_anime(anime_id: &ObjectId, app: web::Data<AppState>) -> Result<Option<WithOID<AnimeSeries>>> {
    let collection = app.mongodb.database(DB_NAME)
        .collection(COLL_NAME);
    collection.find_one(doc! { "_id": anime_id }, None)
        .await.context("Finding anime with the specified ID")
}

#[get("/anime/{id}")]
pub async fn fetch_anime_details(path: web::Path<String>, app: web::Data<AppState>) -> impl Responder {
    let anime_id = path.into_inner();
    if anime_id.len() != 24 {
        return KError::bad_request("The provided ID is not valid".to_string());
    }
    let Ok(anime_id) = ObjectId::parse_str(&anime_id) else {
        return KError::bad_request("The provided ID is not valid".to_string());
    };
    match find_anime(&anime_id, app).await {
        Ok(Some(anime)) => {
            let renamed: WithID<AnimeSeries> = anime.into();
            HttpResponse::Ok().json(renamed)
        },
        Ok(None) => KError::not_found(),
        Err(e) => {
            error!("Could not find anime:\n{e:?}");
            KError::db_error()
        }
    }
}

async fn push_anime(payload: web::Json<AnimeSeries>, _app: web::Data<AppState>) -> HttpResponse {
    warn!("todo: Push {payload:?}");
    HttpResponse::Ok().body("TODO: push anime")
}

async fn update_anime(path: web::Path<String>, payload: web::Json<AnimeSeries>,
    _app: web::Data<AppState>) -> HttpResponse {
    warn!("todo: Update {path:?} with {payload:?}");
    HttpResponse::Ok().body("TODO: update anime")
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

async fn delete_anime(path: web::Path<String>, app: web::Data<AppState>) -> HttpResponse {
    let anime_id = path.into_inner();
    if anime_id.len() != 24 {
        return KError::bad_request("The provided ID is not valid".to_string());
    }
    let Ok(anime_id) = ObjectId::parse_str(&anime_id) else {
        return KError::bad_request("The provided ID is not valid".to_string());
    };
    match find_anime(&anime_id, app.clone()).await {
        Ok(Some(anime)) => {
            let anime: WithID<AnimeSeries> = anime.into();
            create_backup(&anime).unwrap_or_else(|_| error!("Could not save backup file"));

            let collection: mongodb::Collection<WithOID<AnimeSeries>> =
                app.mongodb.database(DB_NAME).collection(COLL_NAME);
            match collection.delete_one(doc! { "_id": anime_id }, None).await {
                Ok(mongodb::results::DeleteResult { deleted_count: 1, .. }) =>
                {
                    // TODO: Remove from meilisearch too
                    HttpResponse::NoContent().finish()
                }
                Ok(_) => KError::internal_error("Anime was found but not deleted".to_string()),
                Err(e) => KError::internal_error(e.to_string())
            }
        },
        Ok(None) => KError::not_found(),
        Err(e) => {
            error!("Could not find anime:\n{e:?}");
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

    cfg.service(web::resource("/s/anime")
        .wrap(KanimeAuth())
        .route(web::post().guard(RequireAdminGuard).to(push_anime)));

    cfg.service(web::resource("/s/anime/{id}")
        .wrap(KanimeAuth())
        .route(web::patch().guard(RequireAdminGuard).to(update_anime))
        .route(web::delete().guard(RequireAdminGuard).to(delete_anime)));

    cfg.service(fetch_anime_details);
}
