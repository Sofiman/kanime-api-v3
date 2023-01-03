use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::HttpResponse;
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Serialize, Deserialize};

pub struct AppState {
    pub app_name: String,
    pub version_info: String,
    pub mongodb: mongodb::Client
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum KErrorType {
    InternalError,
    NotFound,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KError {
    pub error: KErrorType,
    pub error_description: String
}

#[allow(dead_code)]
impl KError {
    pub fn bad_request(details: String) -> HttpResponse {
        HttpResponse::BadRequest().json(Self {
            error: KErrorType::InternalError,
            error_description: details
        })
    }

    pub fn not_found() -> HttpResponse {
        HttpResponse::BadRequest().json(Self {
            error: KErrorType::NotFound,
            error_description: "Not Found".to_string()
        })
    }

    pub fn internal_error(details: String) -> HttpResponse {
        HttpResponse::BadRequest().json(Self {
            error: KErrorType::InternalError,
            error_description: details
        })
    }

    pub fn db_error() -> HttpResponse {
        HttpResponse::BadRequest().json(Self {
            error: KErrorType::InternalError,
            error_description: "Could not retrieve data from database".to_string()
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct WithOID<T> {
    #[serde(rename = "_id")]
    #[serde(with = "hex_string_as_object_id")]
    pub id: String,
    #[serde(flatten)]
    inner: T,
}

impl<T> WithOID<T> {
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[derive(Serialize, Deserialize)]
pub struct WithID<T> {
    pub id: String,
    #[serde(flatten)]
    inner: T,
}

impl<T> WithID<T> {
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> From<WithOID<T>> for WithID<T> {
    fn from(value: WithOID<T>) -> Self {
        WithID {
            id: value.id,
            inner: value.inner
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MangaReleaseInfo {
    author: String,
    volumes: u16,
    chapters: u16,
    release_year: u16
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeReleaseInfo {
    studios: Vec<String>,
    seasons: u16,
    episodes: u16,
    release_year: u16
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum SeasonKind {
    Season,
    Movie,
    Oav,
    SpinOff
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeasonMapping {
    kind: SeasonKind,
    label: String,
    start_episode: u16,
    end_episode: u16,
    start_chapter: u16,
    end_chapter: u16,
    start_volume: u16,
    end_volume: u16,
    pinned_note: Option<Note>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    timestamp: u64,
    author: String,
    content: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CachedImage {
    key: String,
    placeholder: Option<String>
}

impl CachedImage {
    pub fn new(key: String) -> Self {
        Self { key, placeholder: None }
    }

    pub fn with_placeholder(key: String, placeholder: String) -> Self {
        Self { key, placeholder: Some(placeholder) }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn placeholder(&self) -> Option<&str> {
        match &self.placeholder {
            Some(placeholder) => Some(placeholder),
            None => None
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSeries {
    titles: Vec<String>,
    poster: CachedImage,
    manga: MangaReleaseInfo,
    anime: AnimeReleaseInfo,
    mapping: Vec<SeasonMapping>,
    last_update: u64
}

pub fn get_anime() -> AnimeSeries {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap().as_millis() as u64;
    AnimeSeries {
        titles: vec!["Tokyo Revengers".to_string()],
        poster: CachedImage {
            key: "d07f449fdeb9e559e19095db31da14ff".to_string(),
            placeholder: Some("TFOBAk}sIT9r?ZI=u,$zKK#lNYx[".to_string())
        },
        manga: MangaReleaseInfo {
            author: "Ken Wakui".to_string(),
            volumes: 30,
            chapters: 270,
            release_year: 2017
        },
        anime: AnimeReleaseInfo {
            studios: vec!["Liden Films".to_string()],
            seasons: 1,
            episodes: 24,
            release_year: 2021
        },
        mapping: vec![
            SeasonMapping {
                kind: SeasonKind::Season,
                label: "Season 1".to_string(),
                start_episode: 1,
                end_episode: 24,
                start_chapter: 1,
                end_chapter: 73,
                start_volume: 1,
                end_volume: 8,
                pinned_note: None,
            }
        ],
        last_update: now,
    }
}
