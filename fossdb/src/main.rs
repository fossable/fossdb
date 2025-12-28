use axum::{
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

mod models;
mod db;
mod handlers;
mod auth;
mod config;
mod id_generator;
mod scrapers;
mod scraper_models;

use db::Database;

#[derive(Clone)]
pub struct AppState {
    db: Arc<Database>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    
    tracing_subscriber::fmt::init();

    let config = config::Config::from_env();

    // Initialize native_db
    let db = Database::new(&config.database_path)?;
    let db = Arc::new(db);

    let state = AppState {
        db: db.clone(),
    };

    // Initialize scrapers
    let client = reqwest::Client::new();
    let crates_scraper = scrapers::crates_io::CratesIoScraper::new(client.clone());

    let mut scrapers: Vec<Arc<dyn scraper_models::Scraper + Send + Sync>> = vec![
        Arc::new(crates_scraper),
    ];

    // Optional: libraries.io scraper
    if let Some(api_key) = config.libraries_io_api_key.clone() {
        let libraries_scraper = scrapers::libraries_io::LibrariesIoScraper::new(client.clone(), api_key);
        scrapers.push(Arc::new(libraries_scraper));
    }

    // Spawn one background task per scraper
    for scraper in scrapers {
        let db = db.clone();
        let interval_hours = config.scraper_interval_hours;
        tokio::spawn(async move {
            run_scraper_loop(scraper, db, interval_hours).await
        });
    }

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/packages", get(handlers::packages::list_packages))
        .route("/api/packages/:id", get(handlers::packages::get_package))
        .route("/api/packages", post(handlers::packages::create_package))
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/register-form", post(handlers::auth::register_form))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/login-form", post(handlers::auth::login_form))
        .route("/api/users/timeline", get(handlers::users::get_timeline))
        .route("/api/users/subscriptions", get(handlers::users::get_subscriptions))
        .route("/api/users/subscriptions", post(handlers::users::add_subscription))
        .route("/api/analytics", get(handlers::analytics::get_analytics))
        .route("/api/analytics/languages", get(handlers::analytics::get_language_trends))
        .route("/api/analytics/security", get(handlers::analytics::get_security_report))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "fossdb"
    }))
}

async fn run_scraper_loop(
    scraper: Arc<dyn scraper_models::Scraper + Send + Sync>,
    db: Arc<Database>,
    interval_hours: u64,
) {
    use chrono::Utc;

    let scraper_name = scraper.name();

    loop {
        tracing::info!("Starting scraper: {}", scraper_name);

        match scraper.scrape().await {
            Ok(packages) => {
                tracing::info!("Found {} packages from {}", packages.len(), scraper_name);

                for package_data in packages {
                    // Check if package already exists
                    match db.get_package_by_name(&package_data.name) {
                        Ok(Some(_)) => {
                            tracing::debug!("Package {} already exists, skipping", package_data.name);
                            continue;
                        }
                        Ok(None) => {
                            // Package doesn't exist, save it
                            let now = Utc::now();

                            let package = models::Package {
                                id: 0,  // Will be auto-generated
                                name: package_data.name.clone(),
                                description: package_data.description,
                                homepage: package_data.homepage,
                                repository: package_data.repository,
                                license: package_data.license,
                                maintainers: package_data.maintainers,
                                tags: package_data.tags,
                                created_at: now,
                                updated_at: now,
                                submitted_by: Some("scraper".to_string()),
                                platform: package_data.platform,
                                language: package_data.language,
                                status: package_data.status,
                                dependents_count: package_data.dependents_count,
                                rank: package_data.rank,
                            };

                            match db.insert_package(package) {
                                Ok(saved_package) => {
                                    tracing::info!("Saved package: {}", saved_package.name);

                                    // Save versions
                                    for version_data in package_data.versions {
                                        let version = models::PackageVersion {
                                            id: 0,  // Will be auto-generated
                                            package_id: saved_package.id,
                                            version: version_data.version.clone(),
                                            release_date: version_data.release_date,
                                            download_url: version_data.download_url,
                                            checksum: version_data.checksum,
                                            dependencies: version_data.dependencies,
                                            vulnerabilities: Vec::new(),
                                            changelog: version_data.changelog,
                                            created_at: now,
                                        };

                                        if let Err(e) = db.insert_version(version) {
                                            tracing::error!("Failed to save version {} for package {}: {}",
                                                version_data.version, saved_package.name, e);
                                        } else {
                                            tracing::debug!("Saved version {} for package {}",
                                                version_data.version, saved_package.name);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to save package {} from {}: {}",
                                        package_data.name, scraper_name, e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to check if package {} exists: {}", package_data.name, e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Scraper {} failed: {}", scraper_name, e);
            }
        }

        let sleep_duration = tokio::time::Duration::from_secs(interval_hours * 3600);
        tracing::info!("Scraper {} sleeping for {} hours", scraper_name, interval_hours);
        tokio::time::sleep(sleep_duration).await;
    }
}
