pub mod anime;
use actix_web::{web, HttpResponse};
use actix_web::http::header::ContentType;
use crate::types::AppState;

pub async fn get_version(data: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(ContentType::json())
        .body(data.version_info.clone())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/version", web::get().to(get_version));

    anime::configure(cfg);
}
