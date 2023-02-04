#[allow(dead_code)] // only for the types
mod types;
mod config;
mod routes;
mod middlewares;

use config::*;
use std::fs;
use std::string::ToString;
use actix_web::{web, App, HttpServer, middleware};
use actix_web::middleware::{Condition, Logger};
use types::AppState;
use serde_json::json;
use env_logger::Env;
use log::{error, info, warn};
use mongodb::Client;
use gethostname::gethostname;

use middlewares::ip::CloudflareClientIp;
use middlewares::auth::{KanimeAuth, pick_user_id};

const MAJOR_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_MAJOR");
const MINOR_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_MINOR");
const PATCH_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION_PATCH");

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

    info!(target: "mongodb", "Connecting to db...");
    let mongodb = Client::with_uri_str(config.mongodb.with_client_name(&name))
        .await.expect("Error: Failed to connect to MongoDB");
    info!(target: "mongodb", "Successfully connected!");

    let redis = redis::Client::open(config.redis.to_string())
        .expect("Could not connect to redis");
    info!(target: "redis", "Redis client setup done!");

    let meilisearch: meilisearch_sdk::Client = config.meilisearch.as_client();
    if meilisearch.is_healthy().await {
        info!(target: "meilisearch", "Successfully connected!");
        if config.meilisearch.auto_sync.unwrap_or(true) {
            match routes::anime::sync_meilisearch(&mongodb, &meilisearch).await {
                Err(e) => error!("Could not perform auto-sync: {e}"),
                _ => (),
            }
        }
    } else {
        warn!(target: "meilisearch", "No signs of life...");
    }

    info!(target: "http", "Listening on {}:{}", addr.0, addr.1);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: name.clone(),
                version_info: json!({
                    "major": MAJOR_VERSION.unwrap_or("3"),
                    "minor": MINOR_VERSION.unwrap_or("0"),
                    "patch": PATCH_VERSION.unwrap_or("0")
                }).to_string(),
                mongodb: mongodb.clone(),
                meilisearch: meilisearch.clone(),
                redis: redis.clone(),
            }))
            .wrap(Logger::new("%a %r %{UID}xi » %s ~%Dms")
                .custom_request_replace("UID", pick_user_id)
                .log_target("http"))
            .wrap(middleware::Compress::default())
            .wrap(Condition::new(!config.debug.unwrap_or(false),
                                 CloudflareClientIp))
            .wrap(KanimeAuth)
            .configure(routes::configure)
    })
    .bind(addr)?
    .run()
    .await
}
