#[allow(dead_code)]
mod types;
#[allow(dead_code)]
mod config;
mod routes;
mod middlewares;
mod gen;

use config::*;
use std::{fs, path::Path};
use std::string::ToString;
use actix_web::{web, App, HttpServer, middleware, HttpRequest, HttpResponse, http::Method};
use actix_web::middleware::{Condition, Logger};
use serde_json::json;
use env_logger::Env;
use log::{error, info, warn};
use mongodb::Client;
use gethostname::gethostname;

use types::{AppState, KError};
use middlewares::ip::CloudflareClientIp;
use middlewares::auth::{KanimeAuth, pick_user_id};

const MAJOR_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_MAJOR");
const MINOR_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_MINOR");
const PATCH_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_PATCH");

async fn default_endpoint(req: HttpRequest) -> HttpResponse {
    match req.method() {
        &Method::OPTIONS => HttpResponse::NoContent().finish(),
        _ => KError::not_found()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    info!("Reading config...");

    let raw_config = fs::read_to_string(CONFIG_FILE)?;
    let config: Config = toml::from_str(&raw_config)?;
    let addr: (String, u16) = config.http.clone().into();
    let name: String = gethostname().into_string()
        .unwrap_or_else(|_| "kanime-api-v3".to_string());
    info!("Starting server as `{name}`");

    let mongodb = Client::with_uri_str(config.mongodb.with_client_name(&name))
        .await.expect("Error: Failed to connect to MongoDB");
    info!(target: "mongodb", "MongoDB client setup done!");

    let redis = redis::Client::open(config.redis.clone())
        .expect("Could not connect to redis");
    info!(target: "redis", "Redis client setup done!");

    let meilisearch: meilisearch_sdk::Client = config.meilisearch.as_client();
    if meilisearch.is_healthy().await {
        info!(target: "meilisearch", "Successfully connected!");
        if config.meilisearch.auto_sync.unwrap_or(true) {
            if let Err(e) = routes::anime::sync_meilisearch(&mongodb, &meilisearch).await {
                 error!("Could not perform auto-sync: {e}");
            }
        }
    } else {
        warn!(target: "meilisearch", "No signs of life...");
    }

    let cache_folder = Path::new(&config.cache_folder).to_path_buf();

    info!(target: "http", "Listening on {}:{}", addr.0, addr.1);
    let debug = config.debug.unwrap_or(false);
    let domain = config.domain.to_string();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: name.clone(),
                domain: domain.clone(),
                version_info: json!({
                    "major": MAJOR_VERSION.unwrap_or("3"),
                    "minor": MINOR_VERSION.unwrap_or("0"),
                    "patch": PATCH_VERSION.unwrap_or("0")
                }).to_string(),
                mongodb: mongodb.clone(),
                meilisearch: meilisearch.clone(),
                redis: redis.clone(),
                cache_folder: cache_folder.clone()
            }))
            .wrap(Logger::new("%a %r %{UID}xi » %s ~%Dms")
                .custom_request_replace("UID", pick_user_id)
                .log_target("http"))
            .wrap(middleware::Compress::default())
            .wrap(Condition::new(!debug, CloudflareClientIp))
            .wrap(KanimeAuth)
            .wrap(middleware::DefaultHeaders::new()
                .add(("Access-Control-Allow-Origin", "*"))
                .add(("Access-Control-Allow-Headers", "Content-Type, Accept"))
                .add(("Access-Control-Allow-Methods", "GET, POST, OPTIONS")))
            .default_service(web::to(default_endpoint))
            .configure(routes::configure)
    })
    .bind(addr)?
    .run()
    .await
}
