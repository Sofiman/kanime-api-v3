use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::HttpResponse;
use mongodb::bson::serde_helpers::hex_string_as_object_id;
use serde::{Serialize, Deserialize};
use serde_json::json;

pub struct AppState {
    pub app_name: String,
    pub version_info: String,
    pub mongodb: mongodb::Client,
    pub meilisearch: meilisearch_sdk::Client,
    pub redis: redis::Client,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum KErrorType {
    Forbidden,
    BadRequest,
    InternalError,
    NotFound,
}

pub struct KError;

#[allow(dead_code)]
impl KError {
    pub fn bad_request(details: &'_ str) -> HttpResponse {
        HttpResponse::BadRequest().json(json!({
            "error": KErrorType::BadRequest,
            "errorDescription": details,
        }))
    }

    pub fn not_found() -> HttpResponse {
        HttpResponse::BadRequest().json(json!({
            "error": KErrorType::NotFound,
            "errorDescription": "Not Found",
        }))
    }

    pub fn internal_error(details: &'_ str) -> HttpResponse {
        HttpResponse::BadRequest().json(json!({
            "error": KErrorType::InternalError,
            "errorDescription": details,
        }))
    }

    pub fn forbidden() -> HttpResponse {
        HttpResponse::Forbidden().json(json!({
            "error": KErrorType::Forbidden,
            "errorDescription": "Forbidden",
        }))
    }

    pub fn db_error() -> HttpResponse {
        HttpResponse::BadRequest().json(json!({
            "error": KErrorType::InternalError,
            "errorDescription": "Could not retrieve data from database",
        }))
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
            inner: value.inner,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MangaReleaseInfo {
    author: String,
    volumes: u16,
    chapters: u16,
    release_year: u16,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeReleaseInfo {
    studios: Vec<String>,
    seasons: u16,
    episodes: u16,
    release_year: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum SeasonKind {
    Season,
    Movie,
    Oav,
    SpinOff,
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
    content: String,
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
    last_update: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSeriesPatch {
    titles: Option<Vec<String>>,
    manga: Option<MangaReleaseInfo>,
    anime: Option<AnimeReleaseInfo>,
    mapping: Option<Vec<SeasonMapping>>,
}

impl AnimeSeriesPatch {
    pub fn is_empty(&self) -> bool {
        self.titles.is_none() && self.manga.is_none() && self.anime.is_none() &&
            self.mapping.is_none()
    }

    pub fn merge(self, dst: &mut AnimeSeries) {
        if let Some(titles) = self.titles {
            dst.titles = titles;
        }
        if let Some(manga) = self.manga {
            dst.manga = manga;
        }
        if let Some(anime) = self.anime {
            dst.anime = anime;
        }
        if let Some(mapping) = self.mapping {
            dst.mapping = mapping;
        }
    }
}

// Meilisearch related
pub const ANIME_PRIMARY_KEY: &str = "id";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSeriesSearchEntry {
    id: String,
    titles: Vec<String>,
    author: String,
    poster: CachedImage,
}

impl From<WithOID<AnimeSeries>> for AnimeSeriesSearchEntry {
    fn from(value: WithOID<AnimeSeries>) -> Self {
        AnimeSeriesSearchEntry {
            id: value.id,
            titles: value.inner.titles,
            author: value.inner.manga.author,
            poster: value.inner.poster
        }
    }
}

impl From<WithID<AnimeSeries>> for AnimeSeriesSearchEntry {
    fn from(value: WithID<AnimeSeries>) -> Self {
        AnimeSeriesSearchEntry {
            id: value.id,
            titles: value.inner.titles,
            author: value.inner.manga.author,
            poster: value.inner.poster
        }
    }
}

pub fn get_search_entry() -> AnimeSeriesSearchEntry {
    AnimeSeriesSearchEntry {
        id: "63b44f977ef2f272e15f61ca".to_string(),
        titles: vec!["Tokyo Revengers".to_string()],
        author: "Ken Wakui".to_string(),
        poster: CachedImage::with_placeholder(
            "d07f449fdeb9e559e19095db31da14ff".to_string(),
            "TFOBAk}sIT9r?ZI=u,$zKK#lNYx[".to_string(),
        ),
    }
}

pub fn get_anime() -> AnimeSeries {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap().as_millis() as u64;
    AnimeSeries {
        titles: vec!["Tokyo Revengers".to_string()],
        poster: CachedImage::with_placeholder(
            "d07f449fdeb9e559e19095db31da14ff".to_string(),
            "TFOBAk}sIT9r?ZI=u,$zKK#lNYx[".to_string(),
        ),
        manga: MangaReleaseInfo {
            author: "Ken Wakui".to_string(),
            volumes: 30,
            chapters: 270,
            release_year: 2017,
        },
        anime: AnimeReleaseInfo {
            studios: vec!["Liden Films".to_string()],
            seasons: 1,
            episodes: 24,
            release_year: 2021,
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
