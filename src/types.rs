use std::time::{SystemTime, UNIX_EPOCH};
use mongodb::bson::{self, serde_helpers::hex_string_as_object_id};
use serde::{Serialize, Deserialize};

pub struct AppState {
    pub app_name: String,
    pub version_info: String,
    pub mongodb: mongodb::Client
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
pub struct AnimeSeries {
    #[serde(rename = "_id")]
    #[serde(with = "hex_string_as_object_id")]
    id: String,
    titles: Vec<String>,
    manga: MangaReleaseInfo,
    anime: AnimeReleaseInfo,
    mapping: Vec<SeasonMapping>,
    last_update: u64
}

pub fn get_anime() -> AnimeSeries {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap().as_millis() as u64;
    AnimeSeries {
        id: bson::oid::ObjectId::new().to_hex(),
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
                pinned_note: None,
            }
        ],
        last_update: now,
    }
}
