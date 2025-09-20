use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub couchdb_url: String,
    pub couchdb_username: String,
    pub couchdb_password: String,
    pub jwt_secret: String,
    pub server_port: u16,
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
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-change-this".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
        }
    }
}