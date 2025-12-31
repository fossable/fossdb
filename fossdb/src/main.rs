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
mod collector_models;
mod collectors;
mod websocket;

use db::Database;

/// FossDB - A database for tracking free and open source software packages
#[derive(Parser, Debug)]
#[command(name = "fossdb")]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Disable background collectors (only for serve command)
    #[arg(long, default_value_t = false)]
    no_collectors: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Start the FossDB server (default)
    Serve {
        /// Disable background collectors
        #[arg(long, default_value_t = false)]
        no_collectors: bool,
    },
    /// Export database tables to JSON files
    Export {
        /// Output directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        output_dir: std::path::PathBuf,

        /// Specific table to export (packages, versions, users, vulnerabilities, timeline_events)
        #[arg(short, long)]
        table: Option<String>,
    },
    /// Import database table from JSON file
    Import {
        /// Input file path (e.g., packages.json)
        #[arg(short, long)]
        input: std::path::PathBuf,

        /// Merge with existing data instead of replacing
        #[arg(long, default_value_t = false)]
        merge: bool,
    },
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

    // Handle subcommands
    match args.command {
        Some(Commands::Export { output_dir, table }) => {
            return export_database(&config, output_dir, table).await;
        }
        Some(Commands::Import { input, merge }) => {
            return import_database(&config, input, merge).await;
        }
        Some(Commands::Serve { no_collectors }) => {
            return start_server(config, no_collectors).await;
        }
        None => {
            // Default to serve with args.no_collectors
            return start_server(config, args.no_collectors).await;
        }
    }
}

async fn start_server(config: config::Config, no_collectors: bool) -> anyhow::Result<()> {
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

    // Initialize collectors (if not disabled)
    if !no_collectors {
        tracing::info!("Starting background collectors...");

        let client = reqwest::Client::builder().user_agent("fossdb").build()?;
        let crates_collector = collectors::crates_io::CratesIoCollector::new(client.clone());

        let mut collectors: Vec<Arc<dyn collector_models::Collector + Send + Sync>> =
            vec![Arc::new(crates_collector)];

        // Optional: libraries.io collector
        if let Some(api_key) = config.libraries_io_api_key.clone() {
            let libraries_collector =
                collectors::libraries_io::LibrariesIoCollector::new(client.clone(), api_key);
            collectors.push(Arc::new(libraries_collector));
        }

        // Spawn one background task per collector
        for collector in collectors {
            let db = db.clone();
            let broadcaster_clone = broadcaster.clone();
            let interval_hours = config.collector_interval_hours;
            tokio::spawn(async move {
                run_collector_loop(collector, db, broadcaster_clone, interval_hours).await
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
        tracing::info!("Collectors disabled via --no-collectors flag");
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
            "/api/users/subscriptions/{package_name}/notifications",
            axum::routing::put(handlers::users::update_package_notification),
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
        .route("/api/packages/{id}/versions", get(handlers::packages::get_package_versions))
        .route("/api/packages/{id}/subscribers", get(handlers::packages::get_package_subscriber_count))
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

async fn run_collector_loop(
    collector: Arc<dyn collector_models::Collector + Send + Sync>,
    db: Arc<Database>,
    broadcaster: Arc<websocket::TimelineBroadcaster>,
    interval_hours: u64,
) {
    let collector_name = collector.name();

    loop {
        tracing::info!("Starting collector: {}", collector_name);

        match collector.collect(db.clone(), broadcaster.clone()).await {
            Ok(()) => {
                tracing::info!("Collector {} completed successfully", collector_name);
            }
            Err(e) => {
                tracing::error!("Collector {} failed: {}", collector_name, e);
            }
        }

        let sleep_duration = tokio::time::Duration::from_secs(interval_hours * 3600);
        tracing::info!(
            "Collector {} sleeping for {} hours",
            collector_name,
            interval_hours
        );
        tokio::time::sleep(sleep_duration).await;
    }
}

async fn export_database(config: &config::Config, output_dir: std::path::PathBuf, table: Option<String>) -> anyhow::Result<()> {
    let db = Database::new(&config.database_path)?;

    // Create output directory if it doesn't exist
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
            "packages" => {
                tracing::info!("Exporting packages...");
                let data = db.get_all_packages()?;
                eprintln!("Exporting {} packages to {}...", data.len(), output_path.display());

                let json = serde_json::to_string_pretty(&data)?;
                std::fs::write(&output_path, json)?;

                eprintln!("✓ Exported {} packages", data.len());
            }
            "versions" => {
                tracing::info!("Exporting versions...");
                let data = db.get_all_versions()?;
                eprintln!("Exporting {} versions to {}...", data.len(), output_path.display());

                let json = serde_json::to_string_pretty(&data)?;
                std::fs::write(&output_path, json)?;

                eprintln!("✓ Exported {} versions", data.len());
            }
            "users" => {
                tracing::info!("Exporting users...");
                let data = db.get_all_users()?;
                eprintln!("Exporting {} users to {}...", data.len(), output_path.display());

                let json = serde_json::to_string_pretty(&data)?;
                std::fs::write(&output_path, json)?;

                eprintln!("✓ Exported {} users", data.len());
            }
            "vulnerabilities" => {
                tracing::info!("Exporting vulnerabilities...");
                let data = db.get_all_vulnerabilities()?;
                eprintln!("Exporting {} vulnerabilities to {}...", data.len(), output_path.display());

                let json = serde_json::to_string_pretty(&data)?;
                std::fs::write(&output_path, json)?;

                eprintln!("✓ Exported {} vulnerabilities", data.len());
            }
            "timeline_events" => {
                tracing::info!("Exporting timeline events...");
                let data = db.get_all_timeline_events()?;
                eprintln!("Exporting {} timeline events to {}...", data.len(), output_path.display());

                let json = serde_json::to_string_pretty(&data)?;
                std::fs::write(&output_path, json)?;

                eprintln!("✓ Exported {} timeline events", data.len());
            }
            _ => {
                eprintln!("Error: Unknown table '{}'. Valid tables: packages, versions, users, vulnerabilities, timeline_events", table_name);
                return Err(anyhow::anyhow!("Unknown table: {}", table_name));
            }
        }
    }

    eprintln!("\nExport completed successfully!");

    Ok(())
}

async fn import_database(config: &config::Config, input: std::path::PathBuf, merge: bool) -> anyhow::Result<()> {
    let db = Database::new(&config.database_path)?;

    // Determine table name from filename
    let table_name = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    tracing::info!("Importing {} (merge: {})...", table_name, merge);
    eprintln!("Reading from: {}", input.display());

    let json = std::fs::read_to_string(&input)?;

    match table_name {
        "packages" => {
            let data: Vec<models::Package> = serde_json::from_str(&json)?;
            eprintln!("Found {} packages to import", data.len());

            if !merge {
                eprintln!("WARNING: This will replace existing packages!");
                eprintln!("Press Ctrl+C within 5 seconds to cancel...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }

            let total = data.len();
            for (idx, item) in data.into_iter().enumerate() {
                if merge && db.get_package(item.id)?.is_some() {
                    continue;
                }
                db.insert_package(item)?;

                // Progress indicator every 100 items or at the end
                if (idx + 1) % 100 == 0 || idx + 1 == total {
                    eprint!("\rImporting packages: {}/{}", idx + 1, total);
                    use std::io::Write;
                    std::io::stderr().flush()?;
                }
            }
            eprintln!("\n✓ Imported {} packages", total);
        }
        "versions" => {
            let data: Vec<models::PackageVersion> = serde_json::from_str(&json)?;
            eprintln!("Found {} versions to import", data.len());

            if !merge {
                eprintln!("WARNING: This will replace existing versions!");
                eprintln!("Press Ctrl+C within 5 seconds to cancel...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }

            let total = data.len();
            for (idx, item) in data.into_iter().enumerate() {
                if merge && db.get_version(item.id)?.is_some() {
                    continue;
                }
                db.insert_version(item)?;

                if (idx + 1) % 100 == 0 || idx + 1 == total {
                    eprint!("\rImporting versions: {}/{}", idx + 1, total);
                    use std::io::Write;
                    std::io::stderr().flush()?;
                }
            }
            eprintln!("\n✓ Imported {} versions", total);
        }
        "users" => {
            let data: Vec<models::User> = serde_json::from_str(&json)?;
            eprintln!("Found {} users to import", data.len());

            if !merge {
                eprintln!("WARNING: This will replace existing users!");
                eprintln!("Press Ctrl+C within 5 seconds to cancel...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }

            let total = data.len();
            for (idx, item) in data.into_iter().enumerate() {
                if merge && db.get_user(item.id)?.is_some() {
                    continue;
                }
                db.insert_user(item)?;

                if (idx + 1) % 100 == 0 || idx + 1 == total {
                    eprint!("\rImporting users: {}/{}", idx + 1, total);
                    use std::io::Write;
                    std::io::stderr().flush()?;
                }
            }
            eprintln!("\n✓ Imported {} users", total);
        }
        "vulnerabilities" => {
            let data: Vec<models::Vulnerability> = serde_json::from_str(&json)?;
            eprintln!("Found {} vulnerabilities to import", data.len());

            if !merge {
                eprintln!("WARNING: This will replace existing vulnerabilities!");
                eprintln!("Press Ctrl+C within 5 seconds to cancel...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }

            let total = data.len();
            for (idx, item) in data.into_iter().enumerate() {
                if merge && db.get_vulnerability(item.id)?.is_some() {
                    continue;
                }
                db.insert_vulnerability(item)?;

                if (idx + 1) % 100 == 0 || idx + 1 == total {
                    eprint!("\rImporting vulnerabilities: {}/{}", idx + 1, total);
                    use std::io::Write;
                    std::io::stderr().flush()?;
                }
            }
            eprintln!("\n✓ Imported {} vulnerabilities", total);
        }
        "timeline_events" => {
            let data: Vec<models::TimelineEvent> = serde_json::from_str(&json)?;
            eprintln!("Found {} timeline events to import", data.len());

            if !merge {
                eprintln!("WARNING: This will replace existing timeline events!");
                eprintln!("Press Ctrl+C within 5 seconds to cancel...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }

            let total = data.len();
            for (idx, item) in data.into_iter().enumerate() {
                if merge && db.get_timeline_event(item.id)?.is_some() {
                    continue;
                }
                db.insert_timeline_event(item)?;

                if (idx + 1) % 100 == 0 || idx + 1 == total {
                    eprint!("\rImporting timeline events: {}/{}", idx + 1, total);
                    use std::io::Write;
                    std::io::stderr().flush()?;
                }
            }
            eprintln!("\n✓ Imported {} timeline events", total);
        }
        _ => {
            eprintln!("Error: Unknown table '{}'. Valid tables: packages, versions, users, vulnerabilities, timeline_events", table_name);
            return Err(anyhow::anyhow!("Unknown table: {}", table_name));
        }
    }

    eprintln!("\nImport completed successfully!");

    Ok(())
}
