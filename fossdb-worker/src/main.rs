use anyhow::Result;
use futures::future;
use reqwest::Client;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{error, info, warn};

mod scrapers;
mod db;
mod models;
mod package_coordinator;
mod config;

use db::Database;
use models::Scraper;
use package_coordinator::PackageCoordinator;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("Starting FossDB scraper...");

    // Wait for CouchDB to be ready
    let db = loop {
        match Database::new().await {
            Ok(db) => break Arc::new(db),
            Err(e) => {
                info!("Waiting for CouchDB to be ready: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    };
    
    // Initialize package coordinator
    let coordinator = Arc::new(PackageCoordinator::new(db.clone()));
    let client = Client::new();

    // Initialize scrapers
    let crates_scraper = scrapers::crates_io::CratesIoScraper::new(client.clone());
    
    // Load configuration
    let config = config::Config::from_env();
    let libraries_io_api_key = config.libraries_io_api_key.clone();
    
    let libraries_scraper = if let Some(api_key) = libraries_io_api_key {
        Some(scrapers::libraries_io::LibrariesIoScraper::new(client.clone(), api_key))
    } else {
        warn!("LIBRARIES_IO_API_KEY not set, libraries.io scraper will be skipped");
        None
    };

    let scrapers: Vec<Arc<dyn Scraper + Send + Sync>> = {
        let mut scrapers: Vec<Arc<dyn Scraper + Send + Sync>> = vec![
            Arc::new(crates_scraper),
        ];
        
        if let Some(libraries_scraper) = libraries_scraper {
            scrapers.push(Arc::new(libraries_scraper));
        }
        
        scrapers
    };

    loop {
        info!("Starting parallel scraping cycle with {} scrapers", scrapers.len());
        
        // Run all scrapers in parallel using futures
        let mut tasks = Vec::new();
        
        for scraper in &scrapers {
            let scraper_name = scraper.name().to_string();
            let coordinator = coordinator.clone();
            let scraper = scraper.clone();
            
            // Create future for each scraper
            let task = async move {
                run_scraper(scraper.as_ref(), coordinator, scraper_name).await
            };
            tasks.push(task);
        }
        
        // Wait for all scrapers to complete
        let results = future::join_all(tasks).await;
        
        for (index, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                error!("Scraper {} failed: {}", scrapers[index].name(), e);
            }
        }
        
        // Cleanup unused package locks
        coordinator.cleanup_unused_locks();
        
        let sleep_duration = Duration::from_secs(config.scraper_interval_hours * 3600);
        info!("All scrapers complete, sleeping for {} hours...", config.scraper_interval_hours);
        sleep(sleep_duration).await;
    }
}

/// Run a single scraper and save packages using the coordinator
async fn run_scraper(
    scraper: &dyn Scraper,
    coordinator: Arc<PackageCoordinator>,
    scraper_name: String,
) -> Result<()> {
    info!("Starting scraper: {}", scraper_name);
    
    match scraper.scrape().await {
        Ok(packages) => {
            info!("Found {} packages from {}", packages.len(), scraper_name);
            
            // Process packages concurrently but with locking
            let save_handles: Vec<_> = packages
                .into_iter()
                .map(|package| {
                    let coordinator = coordinator.clone();
                    let scraper_name = scraper_name.clone();
                    
                    tokio::spawn(async move {
                        match coordinator.save_package(package.clone()).await {
                            Ok(()) => {
                                tracing::debug!("Successfully saved package {} from {}", package.name, scraper_name);
                            }
                            Err(e) => {
                                error!("Failed to save package {} from {}: {}", package.name, scraper_name, e);
                            }
                        }
                    })
                })
                .collect();
            
            // Wait for all package saves to complete
            for handle in save_handles {
                if let Err(e) = handle.await {
                    error!("Package save task failed: {}", e);
                }
            }
            
            info!("Completed processing packages from {}", scraper_name);
        }
        Err(e) => {
            error!("Scraper {} failed during scraping: {}", scraper_name, e);
            return Err(e);
        }
    }
    
    Ok(())
}
