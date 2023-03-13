use actix_web::{web::{self, Data}, HttpResponse};
use anyhow::Result;
use serde::{self, Deserialize};
use mongodb::options::FindOptions;
use std::fs::File;
use std::io::{Write, BufWriter};
use log::{info, error};
use futures::stream::TryStreamExt;
use mongodb::bson::{doc, serde_helpers::hex_string_as_object_id};
use chrono::{Utc, TimeZone};

use crate::middlewares::auth::{Role, RequireRoleGuard};
use crate::types::{AppState, KError};

const DB_NAME: &str = "Kanime3";
const COLL_NAME: &str = "animes";
const ANIME_SITEMAP_FILE: &str = "anime_index.xml";
const ANIMES_SITEMAP_BATCH_SIZE: u32 = 32;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    #[serde(rename = "_id")]
    #[serde(with = "hex_string_as_object_id")]
    pub id: String,
    pub updated_on: u64,
}

fn write_escaped(out: &mut dyn Write, s: &str) -> Result<()> {
    for chr in s.chars() {
        match chr {
            '&' => write!(out, "&amp;")?,
            '\'' => write!(out, "&apos;")?,
            '>' => write!(out, "&gt;")?,
            '<' => write!(out, "&lt;")?,
            c => write!(out, "{c}")?
        }
    }
    Ok(())
}

pub async fn build_sitemap(app: &AppState) -> Result<()> {
    let col: mongodb::Collection<Metadata> =
        app.mongodb.database(DB_NAME).collection(COLL_NAME);
    let mut cursor = col
        .find(None, FindOptions::builder()
            .batch_size(ANIMES_SITEMAP_BATCH_SIZE)
            .projection(doc! { "_id": 1, "updatedOn": 1 })
            .build())
        .await?;

    let domain = &app.domain;
    let path = app.cache_folder.clone().join(ANIME_SITEMAP_FILE);
    let mut f = BufWriter::new(File::create(path)?);
    write!(f, r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#)?;
    while let Some(doc) = cursor.try_next().await? {
        write!(f, "<url>")?;
        {
            write!(f, "<loc>https://{domain}/anime/")?;
            write_escaped(&mut f, &doc.id)?;
            write!(f, "</loc>")?;

            match Utc.timestamp_millis_opt(doc.updated_on as i64).latest() {
                Some(dt) => write!(f, "<lastmod>{}</lastmod>", dt.to_rfc3339())?,
                _ => write!(f, "<changefreq>monthly</changefreq>")?
            }
        }
        write!(f, "</url>")?;
    }
    write!(f, "</urlset>")?;
    info!("Successfully built sitemap");
    Ok(())
}

async fn update_sitemap(app: Data<AppState>) -> HttpResponse {
    match build_sitemap(&app).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!("Could not generate anime index sitemap: {e:?}");
            KError::internal_error("Could not generate anime index sitemap")
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    let admin_only = RequireRoleGuard(Role::Admin);
    cfg.service(web::resource("/s/seo/sitemap")
        .route(web::post().guard(admin_only).to(update_sitemap)));
}
