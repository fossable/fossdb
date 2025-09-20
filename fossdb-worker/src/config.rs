use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub couchdb_url: String,
    pub couchdb_username: String,
    pub couchdb_password: String,
    pub libraries_io_api_key: Option<String>,
    pub scraper_interval_hours: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            couchdb_url: env::var("COUCHDB_URL")
                .unwrap_or_else(|_| "http://localhost:5984".to_string()),
            couchdb_username: env::var("COUCHDB_USERNAME")
                .unwrap_or_else(|_| "admin".to_string()),
            couchdb_password: env::var("COUCHDB_PASSWORD")
                .unwrap_or_else(|_| "password".to_string()),
            libraries_io_api_key: env::var("LIBRARIES_IO_API_KEY").ok(),
            scraper_interval_hours: env::var("SCRAPER_INTERVAL_HOURS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .unwrap_or(1),
        }
    }
}