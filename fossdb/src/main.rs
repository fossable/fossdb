use axum::{
    Router,
    response::Json,
    routing::{get, post},
};
use clap::Parser;
use serde_json::{Value, json};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

mod auth;
mod client;
mod config;
mod db;
mod email;
mod handlers;
mod id_generator;
mod middleware;
mod models;
mod notifications;
mod scraper_models;
mod scrapers;
mod websocket;

use db::Database;

/// FossDB - A database for tracking free and open source software packages
#[derive(Parser, Debug)]
#[command(name = "fossdb")]
#[command(version, about, long_about = None)]
struct Args {
    /// Disable background scrapers
    #[arg(long, default_value_t = false)]
    no_scrapers: bool,
}

#[derive(Clone)]
pub struct AppState {
    db: Arc<Database>,
    broadcaster: Arc<websocket::TimelineBroadcaster>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config = config::Config::from_env();

    // Initialize native_db
    let db = Database::new(&config.database_path)?;
    let db = Arc::new(db);

    // Log database statistics
    let num_packages = db.get_all_packages()?.len();
    let num_versions = db.get_all_versions()?.len();
    let num_users = db.get_all_users()?.len();
    let num_vulnerabilities = db.get_all_vulnerabilities()?.len();
    let num_timeline_events = db.get_all_timeline_events()?.len();

    tracing::info!("Database statistics:");
    tracing::info!("  Packages: {}", num_packages);
    tracing::info!("  Versions: {}", num_versions);
    tracing::info!("  Users: {}", num_users);
    tracing::info!("  Vulnerabilities: {}", num_vulnerabilities);
    tracing::info!("  Timeline Events: {}", num_timeline_events);

    // Initialize timeline broadcaster
    let broadcaster = Arc::new(websocket::TimelineBroadcaster::new());

    let state = AppState {
        db: db.clone(),
        broadcaster: broadcaster.clone(),
    };

    // Initialize scrapers (if not disabled)
    if !args.no_scrapers {
        tracing::info!("Starting background scrapers...");

        let client = reqwest::Client::builder().user_agent("fossdb").build()?;
        let crates_scraper = scrapers::crates_io::CratesIoScraper::new(client.clone());

        let mut scrapers: Vec<Arc<dyn scraper_models::Scraper + Send + Sync>> =
            vec![Arc::new(crates_scraper)];

        // Optional: libraries.io scraper
        if let Some(api_key) = config.libraries_io_api_key.clone() {
            let libraries_scraper =
                scrapers::libraries_io::LibrariesIoScraper::new(client.clone(), api_key);
            scrapers.push(Arc::new(libraries_scraper));
        }

        // Spawn one background task per scraper
        for scraper in scrapers {
            let db = db.clone();
            let broadcaster_clone = broadcaster.clone();
            let interval_hours = config.scraper_interval_hours;
            tokio::spawn(async move {
                run_scraper_loop(scraper, db, broadcaster_clone, interval_hours).await
            });
        }
        // Initialize notification processor
        if config.email_enabled {
            tracing::info!("Starting notification processor...");

            let email_service = Arc::new(
                email::EmailService::new(config.clone())
                    .expect("Failed to initialize email service"),
            );

            let processor = notifications::NotificationProcessor::new(db.clone(), email_service);

            let notification_interval_minutes = 5;

            tokio::spawn(async move {
                loop {
                    if let Err(e) = processor.process_new_releases().await {
                        tracing::error!("Notification processing error: {}", e);
                    }

                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        notification_interval_minutes * 60,
                    ))
                    .await;
                }
            });
        } else {
            tracing::info!("Email disabled, notification processor not started");
        }
    } else {
        tracing::info!("Scrapers disabled via --no-scrapers flag");
    }

    // Protected routes that require authentication
    let protected = Router::new()
        .route("/api/packages", post(handlers::packages::create_package))
        .route(
            "/api/users/subscriptions",
            get(handlers::users::get_subscriptions),
        )
        .route(
            "/api/users/subscriptions",
            post(handlers::users::add_subscription),
        )
        .route(
            "/api/users/subscriptions/{package_name}",
            axum::routing::delete(handlers::users::remove_subscription),
        )
        .route(
            "/api/users/settings/notifications",
            get(handlers::users::get_notification_settings),
        )
        .route(
            "/api/users/settings/notifications",
            axum::routing::put(handlers::users::update_notification_settings),
        )
        .layer(axum::middleware::from_fn(middleware::auth_middleware))
        .with_state(state.clone());

    // Timeline route with optional auth - shows global timeline for logged-out users,
    // personal timeline for logged-in users
    let timeline_route = Router::new()
        .route("/api/users/timeline", get(handlers::users::get_timeline))
        .layer(axum::middleware::from_fn(
            middleware::optional_auth_middleware,
        ))
        .with_state(state.clone());

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/stats", get(handlers::analytics::get_db_stats))
        .route("/api/packages", get(handlers::packages::list_packages))
        .route("/api/packages/{id}", get(handlers::packages::get_package))
        .route("/api/auth/register", post(handlers::auth::register))
        .route(
            "/api/auth/register-form",
            post(handlers::auth::register_form),
        )
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/login-form", post(handlers::auth::login_form))
        .route("/api/analytics", get(handlers::analytics::get_analytics))
        .route(
            "/api/analytics/languages",
            get(handlers::analytics::get_language_trends),
        )
        .route(
            "/api/analytics/security",
            get(handlers::analytics::get_security_report),
        )
        .route("/ws/timeline", get(websocket::timeline_websocket_handler))
        .merge(timeline_route)
        .merge(protected)
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
    broadcaster: Arc<websocket::TimelineBroadcaster>,
    interval_hours: u64,
) {
    let scraper_name = scraper.name();

    loop {
        tracing::info!("Starting scraper: {}", scraper_name);

        match scraper.scrape(db.clone(), broadcaster.clone()).await {
            Ok(()) => {
                tracing::info!("Scraper {} completed successfully", scraper_name);
            }
            Err(e) => {
                tracing::error!("Scraper {} failed: {}", scraper_name, e);
            }
        }

        let sleep_duration = tokio::time::Duration::from_secs(interval_hours * 3600);
        tracing::info!(
            "Scraper {} sleeping for {} hours",
            scraper_name,
            interval_hours
        );
        tokio::time::sleep(sleep_duration).await;
    }
}
