use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_path: String,
    pub jwt_secret: String,
    #[allow(dead_code)]
    pub server_port: u16,
    pub libraries_io_api_key: Option<String>,
    pub collector_interval_hours: u64,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_address: String,
    pub smtp_from_name: String,
    pub email_enabled: bool,
}

impl Config {
    pub fn from_env() -> Self {
        // Require JWT_SECRET to be set - no insecure defaults
        let jwt_secret = env::var("JWT_SECRET").expect(
            "JWT_SECRET environment variable must be set. Generate a secure random string.",
        );

        Self {
            database_path: env::var("DATABASE_PATH").unwrap_or_else(|_| "./foss.db".to_string()),
            jwt_secret,
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            libraries_io_api_key: env::var("LIBRARIES_IO_API_KEY").ok(),
            collector_interval_hours: env::var("COLLECTOR_INTERVAL_HOURS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .unwrap_or(1),
            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            smtp_username: env::var("SMTP_USERNAME").unwrap_or_default(),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_default(),
            smtp_from_address: env::var("SMTP_FROM_ADDRESS")
                .unwrap_or_else(|_| "noreply@fossdb.org".to_string()),
            smtp_from_name: env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "FossDB".to_string()),
            email_enabled: env::var("EMAIL_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        }
    }
}
