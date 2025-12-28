use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_path: String,
    pub jwt_secret: String,
    pub server_port: u16,
    pub libraries_io_api_key: Option<String>,
    pub scraper_interval_hours: u64,
}

impl Config {
    pub fn from_env() -> Self {
        // Require JWT_SECRET to be set - no insecure defaults
        let jwt_secret = env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable must be set. Generate a secure random string.");

        Self {
            database_path: env::var("DATABASE_PATH").unwrap_or_else(|_| "./foss.db".to_string()),
            jwt_secret,
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            libraries_io_api_key: env::var("LIBRARIES_IO_API_KEY").ok(),
            scraper_interval_hours: env::var("SCRAPER_INTERVAL_HOURS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .unwrap_or(1),
        }
    }
}
