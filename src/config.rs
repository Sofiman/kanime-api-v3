use serde::Deserialize;
#[allow(dead_code)]

pub const CONFIG_FILE: &str = "config.toml";
pub const DEFAULT_PORT: u16 = 80;

#[derive(Deserialize)]
pub struct Config {
    pub debug: Option<bool>,
    pub http: HttpConfig,
    pub mongodb: MongoDBConfig,
    pub redis: RedisConfig,
}

#[derive(Deserialize, Clone)]
pub struct HttpConfig {
    pub host: String,
    pub port: Option<u16>,
}

impl From<HttpConfig> for (String, u16) {
    fn from(value: HttpConfig) -> Self {
        (value.host, value.port.unwrap_or(DEFAULT_PORT))
    }
}

#[derive(Deserialize)]
pub struct MongoDBConfig {
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
}

impl MongoDBConfig {
    pub fn with_client_name(&self, app_name: &str) -> String {
        let mut uri = self.to_string();
        uri.push_str("?appname=");
        uri.push_str(&url_escape::encode_fragment(app_name));
        uri
    }
}

impl ToString for MongoDBConfig {
    fn to_string(&self) -> String {
        use url_escape::{encode_fragment, encode_path};
        format!("mongodb://{}:{}@{}:{}/",
                encode_fragment(&self.username),
                self.password,
                encode_path(&self.host),
                self.port.unwrap_or(27017))
    }
}

#[derive(Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
}
