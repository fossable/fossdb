use anyhow::Result;
use axum::{
    Router,
    response::Json,
    routing::{get, post, put},
};
use clap::Parser;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use fossdb_server::{AppState, config::Config, db::Database, handlers, middleware};
use fossdb_server::{Package, PackageVersion, User, Vulnerability, TimelineEvent};

#[cfg(feature = "email")]
use fossdb_server::{email, notifications};

use fossdb_server::websocket;

/// FossDB - Free Software Database
#[derive(Parser, Debug)]
#[command(name = "fossdb")]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Start the API server (default)
    #[cfg(feature = "api-server")]
    Serve,
    /// Export database tables to JSON files
    #[cfg(feature = "db")]
    Export {
        #[arg(short, long, default_value = ".")]
        output_dir: PathBuf,
        #[arg(short, long)]
        table: Option<String>,
    },
    /// Import database table from JSON file
    #[cfg(feature = "db")]
    Import {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(long, default_value_t = false)]
        merge: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config = Config::from_env();

    match args.command {
        #[cfg(feature = "db")]
        Some(Commands::Export { output_dir, table }) => {
            return export_database(&config, output_dir, table).await;
        }
        #[cfg(feature = "db")]
        Some(Commands::Import { input, merge }) => {
            return import_database(&config, input, merge).await;
        }
        #[cfg(feature = "api-server")]
        Some(Commands::Serve) => {
            return start_server(config).await;
        }
        None => {
            #[cfg(feature = "api-server")]
            return start_server(config).await;
            #[cfg(not(feature = "api-server"))]
            {
                std::future::pending::<()>().await;
                Ok(())
            }
        }
    }
}

async fn start_server(config: Config) -> Result<()> {
    let db = Database::new(&config.database_path)?;
    let db = Arc::new(db);

    let num_packages = db.get_all_packages()?.len();
    let num_versions = db.get_all_versions()?.len();
    let num_users = db.get_all_users()?.len();
    let num_vulnerabilities = db.get_all_vulnerabilities()?.len();
    let num_timeline_events = db.get_all_timeline_events()?.len();

    info!("Database statistics:");
    info!("  Packages: {}", num_packages);
    info!("  Versions: {}", num_versions);
    info!("  Users: {}", num_users);
    info!("  Vulnerabilities: {}", num_vulnerabilities);
    info!("  Timeline Events: {}", num_timeline_events);

    let broadcaster = Arc::new(websocket::TimelineBroadcaster::new());

    if let Err(e) =
        fossdb_server::db_listener::spawn_package_version_listener(db.clone(), broadcaster.clone())
    {
        error!("Failed to initialize database listener: {}", e);
    }

    let state = AppState {
        db: db.clone(),
        broadcaster: broadcaster.clone(),
        collector_api_key: config.collector_api_key.clone(),
        worker_api_key: config.worker_api_key.clone(),
        claimed_versions: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
    };

    // Email notifications
    #[cfg(feature = "email")]
    if config.email_enabled {
        info!("Starting notification processor...");
        let email_service = Arc::new(
            email::EmailService::new(config.clone())
                .expect("Failed to initialize email service"),
        );
        let processor = notifications::NotificationProcessor::new(db.clone(), email_service);
        tokio::spawn(async move {
            loop {
                if let Err(e) = processor.process_new_releases().await {
                    error!("Notification processing error: {}", e);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5 * 60)).await;
            }
        });
    }
    #[cfg(feature = "email")]
    if !config.email_enabled {
        info!("Email disabled, notification processor not started");
    }

    // Timeline event purge task
    let purge_db = db.clone();
    let retention_days = config.timeline_retention_days;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(24 * 60 * 60)).await;
            info!("Running timeline event purge (retention: {} days)", retention_days);
            match purge_db.purge_old_timeline_events(chrono::Duration::days(retention_days as i64)) {
                Ok(count) => {
                    if count > 0 {
                        info!("Purged {} old timeline events", count);
                    }
                }
                Err(e) => error!("Failed to purge old timeline events: {}", e),
            }
        }
    });

    // Internal worker API on separate port
    let worker_api = Router::new()
        .route("/packages/next", get(handlers::worker::get_task))
        .route("/analysis", post(handlers::worker::submit_analysis))
        .route("/packages/:package_id/analyses", get(handlers::worker::get_package_analyses))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::worker_auth_middleware,
        ))
        .with_state(state.clone());

    let worker_listener = tokio::net::TcpListener::bind(
        format!("0.0.0.0:{}", config.worker_port)
    ).await?;
    info!("Worker API running on http://0.0.0.0:{}", config.worker_port);
    tokio::spawn(async move {
        if let Err(e) = axum::serve(worker_listener, worker_api).await {
            error!("Worker API server error: {}", e);
        }
    });

    // Internal collector API on separate port
    let collector_api = Router::new()
        .route("/packages", get(handlers::collector::get_package))
        .route("/packages", post(handlers::collector::upsert_package))
        .route("/packages/:name/timestamp", put(handlers::collector::update_package_timestamp))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::collector_auth_middleware,
        ))
        .with_state(state.clone());

    let collector_listener = tokio::net::TcpListener::bind(
        format!("0.0.0.0:{}", config.collector_port)
    ).await?;
    info!("Collector API running on http://0.0.0.0:{}", config.collector_port);
    tokio::spawn(async move {
        if let Err(e) = axum::serve(collector_listener, collector_api).await {
            error!("Collector API server error: {}", e);
        }
    });

    // Public API
    let protected = Router::new()
        .route("/api/packages", post(handlers::packages::create_package))
        .route("/api/users/subscriptions", get(handlers::users::get_subscriptions))
        .route("/api/users/subscriptions", post(handlers::users::add_subscription))
        .route(
            "/api/users/subscriptions/{package_name}",
            axum::routing::delete(handlers::users::remove_subscription),
        )
        .route(
            "/api/users/subscriptions/{package_name}/notifications",
            put(handlers::users::update_package_notification),
        )
        .route("/api/users/settings/notifications", get(handlers::users::get_notification_settings))
        .route("/api/users/settings/notifications", put(handlers::users::update_notification_settings))
        .layer(axum::middleware::from_fn(middleware::auth_middleware))
        .with_state(state.clone());

    let timeline_route = Router::new()
        .route("/api/users/timeline", get(handlers::users::get_timeline))
        .layer(axum::middleware::from_fn(middleware::optional_auth_middleware))
        .with_state(state.clone());

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/stats", get(handlers::analytics::get_db_stats))
        .route("/api/packages", get(handlers::packages::list_packages))
        .route("/api/packages/{id}", get(handlers::packages::get_package))
        .route("/api/packages/{id}/versions", get(handlers::packages::get_package_versions))
        .route("/api/packages/{id}/subscribers", get(handlers::packages::get_package_subscriber_count))
        .route("/api/auth/register", post(handlers::auth::register))
        .route("/api/auth/register-form", post(handlers::auth::register_form))
        .route("/api/auth/login", post(handlers::auth::login))
        .route("/api/auth/login-form", post(handlers::auth::login_form))
        .route("/api/analytics", get(handlers::analytics::get_analytics))
        .route("/api/analytics/languages", get(handlers::analytics::get_language_trends))
        .route("/api/analytics/security", get(handlers::analytics::get_security_report))
        .route("/ws/timeline", get(websocket::timeline_websocket_handler))
        .merge(timeline_route)
        .merge(protected)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "fossdb"
    }))
}

fn export_table<T: Serialize>(table_name: &str, data: Vec<T>, output_path: &Path) -> Result<()> {
    info!("Exporting {}...", table_name);
    eprintln!("Exporting {} {} to {}...", data.len(), table_name, output_path.display());
    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(output_path, json)?;
    eprintln!("✓ Exported {} {}", data.len(), table_name);
    Ok(())
}

async fn export_database(config: &Config, output_dir: PathBuf, table: Option<String>) -> Result<()> {
    let db = Database::new(&config.database_path)?;
    std::fs::create_dir_all(&output_dir)?;

    let tables_to_export = if let Some(table_name) = table {
        vec![table_name]
    } else {
        vec![
            "packages".to_string(),
            "versions".to_string(),
            "users".to_string(),
            "vulnerabilities".to_string(),
            "timeline_events".to_string(),
        ]
    };

    for table_name in tables_to_export {
        let output_path = output_dir.join(format!("{}.json", table_name));
        match table_name.as_str() {
            "packages" => export_table("packages", db.get_all_packages()?, &output_path)?,
            "versions" => export_table("versions", db.get_all_versions()?, &output_path)?,
            "users" => export_table("users", db.get_all_users()?, &output_path)?,
            "vulnerabilities" => export_table("vulnerabilities", db.get_all_vulnerabilities()?, &output_path)?,
            "timeline_events" => export_table("timeline events", db.get_all_timeline_events()?, &output_path)?,
            _ => {
                eprintln!("Error: Unknown table '{}'", table_name);
                return Err(anyhow::anyhow!("Unknown table: {}", table_name));
            }
        }
    }

    eprintln!("\nExport completed successfully!");
    Ok(())
}

async fn import_database(config: &Config, input: PathBuf, merge: bool) -> Result<()> {
    let db = Database::new(&config.database_path)?;

    let table_name = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    info!("Importing {} (merge: {})...", table_name, merge);
    eprintln!("Reading from: {}", input.display());

    let json = std::fs::read_to_string(&input)?;

    macro_rules! import_with_progress {
        ($data:expr, $type_name:expr, $get_method:ident, $insert_method:ident) => {{
            eprintln!("Found {} {} to import", $data.len(), $type_name);

            if !merge {
                eprintln!("WARNING: This will replace existing {}!", $type_name);
                eprintln!("Press Ctrl+C within 5 seconds to cancel...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }

            let total = $data.len();
            for (idx, item) in $data.into_iter().enumerate() {
                if merge && db.$get_method(item.id)?.is_some() {
                    continue;
                }
                db.$insert_method(item)?;

                if (idx + 1) % 100 == 0 || idx + 1 == total {
                    eprint!("\rImporting {}: {}/{}", $type_name, idx + 1, total);
                    use std::io::Write;
                    std::io::stderr().flush()?;
                }
            }
            eprintln!("\n✓ Imported {} {}", total, $type_name);
        }};
    }

    match table_name {
        "packages" => {
            let data: Vec<Package> = serde_json::from_str(&json)?;
            import_with_progress!(data, "packages", get_package, insert_package);
        }
        "versions" => {
            let data: Vec<PackageVersion> = serde_json::from_str(&json)?;
            import_with_progress!(data, "versions", get_version, insert_version);
        }
        "users" => {
            let data: Vec<User> = serde_json::from_str(&json)?;
            import_with_progress!(data, "users", get_user, insert_user);
        }
        "vulnerabilities" => {
            let data: Vec<Vulnerability> = serde_json::from_str(&json)?;
            import_with_progress!(data, "vulnerabilities", get_vulnerability, insert_vulnerability);
        }
        "timeline_events" => {
            let data: Vec<TimelineEvent> = serde_json::from_str(&json)?;
            import_with_progress!(data, "timeline events", get_timeline_event, insert_timeline_event);
        }
        _ => {
            eprintln!("Error: Unknown table '{}'", table_name);
            return Err(anyhow::anyhow!("Unknown table: {}", table_name));
        }
    }

    eprintln!("\nImport completed successfully!");
    Ok(())
}
