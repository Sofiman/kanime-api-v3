use std::str::FromStr;
use crate::types::*;
use actix_web::{get, post, web, Responder, HttpResponse};
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use serde::Serialize;
use anyhow::{Context, Result};
use log::error;

const DB_NAME: &str = "Kanime3";
const COLL_NAME: &str = "animes";

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    id: String,
    title: String,
    poster: CachedImage,
}

#[post("/search")]
pub async fn search_anime(_app: web::Data<AppState>) -> impl Responder {
    let results = vec![
        SearchResult {
            id: "63b44f977ef2f272e15f61ca".to_string(),
            title: "Tokyo Revengers".to_string(),
            poster: CachedImage::with_placeholder(
                "d07f449fdeb9e559e19095db31da14ff".to_string(),
                "TFOBAk}sIT9r?ZI=u,$zKK#lNYx[".to_string()
            ),
        }
    ];
    web::Json(results)
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
    let Ok(anime_id) = ObjectId::from_str(&anime_id) else {
        return KError::bad_request("The provided ID is not valid".to_string());
    };
    match find_anime(&anime_id, app).await {
        Ok(Some(anime)) => {
            let renamed: WithID<AnimeSeries> = anime.into();
            HttpResponse::Ok().json(renamed)
        },
        Ok(None) => KError::not_found(),
        Err(e) => {
            error!("Could not find anime:\n{:?}", e);
            KError::db_error()
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(search_anime);
    cfg.service(fetch_anime_details);
}