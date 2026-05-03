use anyhow::Result;
use std::sync::Arc;

mod client;
mod collectors;

use client::CollectorClient;
use collectors::Collector;

struct Config {
    server_url: String,
    api_key: String,
    interval_hours: u64,
    #[allow(dead_code)]
    libraries_io_api_key: Option<String>,
}

impl Config {
    fn from_env() -> Self {
        Self {
            server_url: std::env::var("SERVER_INTERNAL_URL")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            api_key: std::env::var("COLLECTOR_API_KEY")
                .expect("COLLECTOR_API_KEY environment variable must be set"),
            interval_hours: std::env::var("COLLECTOR_INTERVAL_HOURS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .unwrap_or(1),
            libraries_io_api_key: std::env::var("LIBRARIES_IO_API_KEY").ok(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    let client = Arc::new(CollectorClient::new(config.server_url.clone(), config.api_key.clone())?);

    let mut collector_list: Vec<Arc<dyn Collector>> = vec![];

    #[cfg(feature = "collector-rust")]
    {
        let http = reqwest::Client::builder().user_agent("fossdb-collector").build()?;
        collector_list.push(Arc::new(collectors::crates_io::CratesIoCollector::new(http)));
    }

    #[cfg(feature = "collector-nixpkgs")]
    collector_list.push(Arc::new(collectors::nixpkgs::NixpkgsCollector {}));

    #[cfg(feature = "collector-libraries-io")]
    if let Some(api_key) = config.libraries_io_api_key {
        let http = reqwest::Client::builder().user_agent("fossdb-collector").build()?;
        collector_list.push(Arc::new(collectors::libraries_io::LibrariesIoCollector::new(http, api_key)));
    }

    if collector_list.is_empty() {
        tracing::warn!("No collectors configured, exiting");
        return Ok(());
    }

    let handles: Vec<_> = collector_list
        .into_iter()
        .map(|collector| {
            let client = client.clone();
            let interval_hours = config.interval_hours;
            tokio::spawn(async move {
                run_collector_loop(collector, client, interval_hours).await;
            })
        })
        .collect();

    futures::future::join_all(handles).await;
    Ok(())
}

async fn run_collector_loop(
    collector: Arc<dyn Collector>,
    client: Arc<CollectorClient>,
    interval_hours: u64,
) {
    let name = collector.name().to_string();
    loop {
        tracing::info!("Starting collector: {}", name);
        match collector.collect(client.clone()).await {
            Ok(()) => tracing::info!("Collector {} completed", name),
            Err(e) => tracing::error!("Collector {} failed: {}", name, e),
        }
        let duration = tokio::time::Duration::from_secs(interval_hours * 3600);
        tracing::info!("Collector {} sleeping for {} hours", name, interval_hours);
        tokio::time::sleep(duration).await;
    }
}
