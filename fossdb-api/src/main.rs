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

use db::Database;

#[derive(Clone)]
pub struct AppState {
    db: Arc<Database>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    
    tracing_subscriber::fmt::init();

    // Wait for CouchDB to be ready
    let db = loop {
        match Database::new().await {
            Ok(db) => break db,
            Err(e) => {
                tracing::info!("Waiting for CouchDB to be ready: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    };
    let state = AppState {
        db: Arc::new(db),
    };

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
        "service": "fossdb-api"
    }))
}
