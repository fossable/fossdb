use anyhow::Result;

mod analysis;
mod client;

use client::WorkerClient;

struct Config {
    server_url: String,
    api_key: String,
    interval_secs: u64,
}

impl Config {
    fn from_env() -> Self {
        Self {
            server_url: std::env::var("WORKER_SERVER_URL")
                .unwrap_or_else(|_| "http://localhost:3002".to_string()),
            api_key: std::env::var("WORKER_API_KEY")
                .expect("WORKER_API_KEY environment variable must be set"),
            interval_secs: std::env::var("WORKER_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    let client = WorkerClient::new(config.server_url.clone(), config.api_key.clone())?;

    tracing::info!("Worker started, connecting to {}", config.server_url);

    loop {
        match run_once(&client).await {
            Ok(()) => {}
            Err(e) => tracing::error!("Worker iteration failed: {}", e),
        }

        tracing::debug!("Sleeping {} seconds before next task", config.interval_secs);
        tokio::time::sleep(tokio::time::Duration::from_secs(config.interval_secs)).await;
    }
}

async fn run_once(client: &WorkerClient) -> Result<()> {
    let task = match client.get_task().await {
        Ok(t) => t,
        Err(e) if e.to_string().contains("404") => {
            tracing::info!("No unanalyzed packages available, skipping");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    tracing::info!(
        "Received task: {}@{}",
        task.package.name,
        task.version.version
    );

    let analysis = analysis::analyze(&task, client.http()).await?;

    tracing::info!(
        "Analysis complete: {} findings (license={:?}, checksum_ok={:?})",
        analysis.findings.len(),
        analysis.detected_license,
        analysis.checksum_verified
    );

    client.submit_analysis(&analysis).await?;
    tracing::info!("Analysis submitted successfully");

    Ok(())
}
