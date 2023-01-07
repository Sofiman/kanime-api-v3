use crate::types::*;
use std::str::FromStr;
use actix_web::{guard, get, web, Responder, HttpResponse};
use validator::Validate;
use mongodb::bson::{doc,oid::ObjectId};
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use log::{error, info};

const DB_NAME: &str = "Kanime3";
const COLL_NAME: &str = "animes";

#[derive(Deserialize, Serialize, Validate, Debug, Clone)]
pub struct SearchQuery {
    #[validate(length(min = 1, max = 128))]
    query: String,
    limit: Option<u16>
}

async fn search_animes(query: SearchQuery, _app: web::Data<AppState>) -> HttpResponse {
    match query.validate() {
        Ok(_) => (),
        Err(e) => return KError::bad_request(e.to_string())
    };
    HttpResponse::Ok().json(query)
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
    cfg.service(web::resource("/search")
        .guard(guard::Header("content-type", "application/json"))
        .route(web::post().to(search_anime_json)));
    cfg.service(web::resource("/search")
        .guard(guard::Header("content-type", "application/x-www-form-urlencoded"))
        .route(web::post().to(search_anime_form)));

    cfg.service(fetch_anime_details);
}