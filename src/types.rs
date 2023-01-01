use std::fmt::{Debug, Display, Formatter};
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use serde::{Serialize, Deserialize};

pub struct AppState {
    pub app_name: String,
    pub version_info: String
}

struct KError {
    err: anyhow::Error,
}

impl Debug for KError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.err)
    }
}

impl Display for KError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.err)
    }
}

impl actix_web::error::ResponseError for KError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::new(self.status_code())
    }
}
impl From<anyhow::Error> for KError {
    fn from(err: anyhow::Error) -> KError {
        KError { err }
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
    note: Option<String>,
    note_author: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSeries {
    id: String,
    titles: Vec<String>,
    manga: MangaReleaseInfo,
    anime: AnimeReleaseInfo,
    mapping: Vec<SeasonMapping>,
}

pub fn get_anime(id: String) -> AnimeSeries {
    AnimeSeries {
        id,
        titles: vec!["Tokyo Revengers".to_string()],
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
                note: None,
                note_author: None
            }
        ]
    }
}
