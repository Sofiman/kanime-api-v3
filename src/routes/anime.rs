use crate::types::*;
use actix_web::{get, post, web, Responder, Result};

#[post("/search")]
pub async fn search_anime(data: web::Data<AppState>) -> String {
    let app_name = &data.app_name; // <- get app_name
    format!("Search for `{app_name}`") // <- response with app_name
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