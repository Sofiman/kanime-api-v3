use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use actix_web::HttpResponse;
use mongodb::bson::{self, serde_helpers::hex_string_as_object_id};
use serde::{Serialize, Deserialize};
use serde_json::json;

pub struct AppState {
    pub app_name: String,
    pub version_info: String,
    pub mongodb: mongodb::Client,
    pub meilisearch: meilisearch_sdk::Client,
    pub redis: redis::Client,
    pub cache_folder: PathBuf
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

impl KError {
    pub fn bad_request(details: &'_ str) -> HttpResponse {
        HttpResponse::BadRequest().json(json!({
            "error": KErrorType::BadRequest,
            "errorDescription": details,
        }))
    }

    pub fn not_found() -> HttpResponse {
        HttpResponse::NotFound().json(json!({
            "error": KErrorType::NotFound,
            "errorDescription": "Not Found",
        }))
    }

    pub fn internal_error(details: &'_ str) -> HttpResponse {
        HttpResponse::InternalServerError().json(json!({
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
        HttpResponse::InternalServerError().json(json!({
            "error": KErrorType::InternalError,
            "errorDescription": "Could not retrieve data from database",
        }))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

impl<T> AsRef<T> for WithOID<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithID<T> {
    pub id: String,
    #[serde(flatten)]
    inner: T,
}

impl<T> WithID<T> {

    pub fn new(id: String, inner: T) -> Self {
        Self { id, inner }
    }

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

impl<T> AsRef<T> for WithID<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MangaReleaseInfo {
    pub author: String,
    pub volumes: u16,
    pub chapters: u16,
    pub release_year: u16,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeReleaseInfo {
    pub studios: Vec<String>,
    pub seasons: u16,
    pub episodes: u16,
    pub release_year: u16,
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
    pub titles: Vec<String>,
    pub poster: CachedImage,
    pub manga: MangaReleaseInfo,
    pub anime: AnimeReleaseInfo,
    pub mapping: Vec<SeasonMapping>,
    pub updated_on: u64,
    pub created_on: u64,
}

impl AsRef<AnimeSeries> for AnimeSeries {
    fn as_ref(&self) -> &AnimeSeries {
        &self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSeriesCandidate {
    pub titles: Vec<String>,
    pub manga: MangaReleaseInfo,
    pub anime: AnimeReleaseInfo,
    pub mapping: Vec<SeasonMapping>,
}

impl AnimeSeriesCandidate {
    pub fn into_anime(self, poster: CachedImage) -> AnimeSeries {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("The time can never be earlier than the Unix epoch")
            .as_millis() as u64;
        AnimeSeries {
            titles: self.titles,
            poster,
            manga: self.manga,
            anime: self.anime,
            mapping: self.mapping,
            updated_on: now,
            created_on: now
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSeriesPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    titles: Option<Vec<String>>,

    #[serde(skip_deserializing)]
    #[serde(skip_serializing_if = "Option::is_none")]
    poster: Option<CachedImage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    manga: Option<MangaReleaseInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    anime: Option<AnimeReleaseInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    mapping: Option<Vec<SeasonMapping>>,

    #[serde(skip_deserializing)]
    updated_on: u64,
}

impl AnimeSeriesPatch {
    pub fn is_empty(&self) -> bool {
        self.titles.is_none() && self.poster.is_none() && self.manga.is_none()
            && self.anime.is_none() && self.mapping.is_none()
    }

    pub fn has_presenter_changes(&self) -> bool {
        self.titles.is_some() || self.manga.is_some() || self.anime.is_some()
    }

    pub fn set_poster(&mut self, poster: CachedImage) {
        self.poster = Some(poster);
    }

    pub fn seal(&mut self) -> Result<bson::Document, bson::ser::Error> {
        self.updated_on = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("The time can never be earlier than the Unix epoch")
            .as_millis() as u64;
        bson::to_document(self)
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

#[derive(Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSeriesSearchEntryPatch {
    id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    titles: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    poster: Option<CachedImage>,
}

impl AnimeSeriesSearchEntryPatch {
    pub fn from_patch(id: String, p: AnimeSeriesPatch) -> Option<Self> {
        if p.titles.is_none() && p.manga.is_none() && p.poster.is_none() {
            return None;
        }
        Some(Self {
            id,
            titles: p.titles,
            author: p.manga.map(|manga| manga.author),
            poster: p.poster
        })
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
        updated_on: now,
        created_on: now,
    }
}
