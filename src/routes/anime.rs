use crate::types::*;
use actix_web::{get, post, web, Responder, Result};
use serde::Serialize;

#[derive(Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    id: String,
    title: String,
    poster_url: String,
    poster_hash: String,
}

#[post("/search")]
pub async fn search_anime(_app: web::Data<AppState>) -> Result<impl Responder> {
    let results = vec![
        SearchResult {
            id: "tokyo_revengers".to_string(),
            title: "Tokyo Revengers".to_string(),
            poster_url: "https://kanime.fr/media/cache/d07f449fdeb9e559e19095db31da14ff".to_string(),
            poster_hash: "blurhash".to_string()
        }
    ];
    Ok(web::Json(results))
}

#[get("/anime/{id}")]
pub async fn fetch_anime_details(path: web::Path<String>, _app: web::Data<AppState>) -> Result<impl Responder> {
    let anime_id = path.into_inner();
    let result = get_anime(anime_id);
    Ok(web::Json(result))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(search_anime);
    cfg.service(fetch_anime_details);
}