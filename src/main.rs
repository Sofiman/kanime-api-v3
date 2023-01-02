mod types;
mod config;
mod routes;

use config::*;
use std::fs;
use std::string::ToString;
use actix_web::{web, App, HttpServer, middleware};
use actix_web::middleware::Logger;
use types::AppState;
use serde_json::json;
use env_logger::Env;
use log::info;
use mongodb::Client;
use gethostname::gethostname;

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
    let name: String = gethostname().into_string().unwrap_or_else(|_| "kanime-api-v3".to_string());
    info!("Starting server as `{}`", name);

    info!(target: "mongodb", "Connecting to db as `{}` ...", name);
    let client = Client::with_uri_str(config.mongodb.with_client_name(&name))
        .await
        .expect("Error: Failed to connect to MongoDB");
    info!(target: "mongodb", "Successfully connected!");

    info!(target: "http", "Listenning on {}:{}", addr.0, addr.1);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: name.clone(),
                version_info: json!({
                    "major": MAJOR_VERSION.unwrap_or("3"),
                    "minor": MINOR_VERSION.unwrap_or("0"),
                    "patch": PATCH_VERSION.unwrap_or("0")
                }).to_string(),
                mongodb: client.clone()
            }))
            .wrap(Logger::new("%a %r » %s ~%Dms").log_target("http"))
            .wrap(middleware::Compress::default())
            .configure(routes::configure)
    })
    .bind(addr)?
    .run()
    .await
}
