mod types;
mod routes;
use std::string::ToString;
use actix_web::{web, App, HttpServer, middleware};
use actix_web::middleware::Logger;
use types::AppState;
use serde_json::json;
use env_logger::Env;
use log::info;

const MAJOR_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_MAJOR");
const MINOR_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_MINOR");
const PATCH_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_PATCH");

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    info!("Starting server on 0.0.0.0:80");
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: String::from("Actix Web"),
                version_info: json!({
                    "major": MAJOR_VERSION.unwrap_or("0"),
                    "minor": MINOR_VERSION.unwrap_or("1"),
                    "patch": PATCH_VERSION.unwrap_or("0")
                }).to_string()
            }))
            .wrap(Logger::new("%a %r » %s ~%Dms")
                .log_target("http"))
            .wrap(middleware::Compress::default())
            .configure(routes::configure)
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}
