use meilisearch_sdk::Client;
use serde::Deserialize;
#[allow(dead_code)]

pub const CONFIG_FILE: &str = "config.toml";
pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_MONGO_PORT: u16 = 27017;
pub const DEFAULT_REDIS_PORT: u16 = 6379;

#[derive(Deserialize)]
pub struct Config<'ha, 'moa, 'mob, 'moc, 'ra, 'rb, 'rc, 'msa, 'msb, 'cf> {
    pub debug: Option<bool>,
    #[serde(borrow)]
    pub http: HttpConfig<'ha>,
    #[serde(borrow)]
    pub mongodb: MongoDBConfig<'moa, 'mob, 'moc>,
    #[serde(borrow)]
    pub redis: RedisConfig<'ra, 'rb, 'rc>,
    #[serde(borrow)]
    pub meilisearch: MeilisearchConfig<'msa, 'msb>,
    #[serde(borrow)]
    pub cache_folder: &'cf str,
}

#[derive(Deserialize, Clone)]
pub struct HttpConfig<'a> {
    pub host: &'a str,
    pub port: Option<u16>,
}

impl From<HttpConfig<'_>> for (String, u16) {
    fn from(value: HttpConfig<'_>) -> Self {
        (value.host.to_string(), value.port.unwrap_or(DEFAULT_PORT))
    }
}

#[derive(Deserialize)]
pub struct MongoDBConfig<'a, 'b, 'c> {
    pub host: &'a str,
    pub port: Option<u16>,
    pub username: &'b str,
    pub password: &'c str,
}

impl MongoDBConfig<'_, '_, '_> {
    pub fn with_client_name(&self, app_name: &str) -> String {
        let mut uri = self.to_string();
        uri.push_str("?appname=");
        uri.push_str(&url_escape::encode_fragment(app_name));
        uri
    }
}

impl ToString for MongoDBConfig<'_, '_, '_> {
    fn to_string(&self) -> String {
        use url_escape::{encode_fragment, encode_path};
        format!("mongodb://{}:{}@{}:{}/",
                encode_fragment(self.username),
                encode_fragment(self.password),
                encode_path(self.host),
                self.port.unwrap_or(DEFAULT_MONGO_PORT))
    }
}

#[derive(Deserialize)]
pub struct RedisConfig<'a, 'b, 'c> {
    pub host: &'a str,
    pub port: Option<u16>,
    pub username: &'b str,
    pub password: &'c str,
}

impl ToString for RedisConfig<'_, '_, '_> {
    fn to_string(&self) -> String {
        use url_escape::{encode_fragment, encode_path};
        format!("redis://{}:{}@{}:{}",
                encode_fragment(self.username),
                encode_fragment(self.password),
                encode_path(self.host),
                self.port.unwrap_or(DEFAULT_REDIS_PORT))
    }
}

#[derive(Deserialize)]
pub struct MeilisearchConfig<'a, 'b> {
    pub host: &'a str,
    pub master_key: &'b str,
    pub auto_sync: Option<bool>
}

impl MeilisearchConfig<'_, '_> {
    pub fn as_client(&self) -> Client {
        Client::new(self.host, self.master_key)
    }
}
