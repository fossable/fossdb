use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

mod scrapers;
mod db;
mod models;

use db::Database;
use models::*;
use models::Scraper;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("Starting FossDB scraper...");

    // Wait for CouchDB to be ready
    let db = loop {
        match Database::new().await {
            Ok(db) => break db,
            Err(e) => {
                info!("Waiting for CouchDB to be ready: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    };
    let client = Client::new();

    let scraper = scrapers::crates_io::CratesIoScraper::new(client.clone());

    loop {
        info!("Running scraper: {}", scraper.name());
        
        match scraper.scrape().await {
            Ok(packages) => {
                info!("Found {} packages from {}", packages.len(), scraper.name());
                
                for package in packages {
                    if let Err(e) = save_package(&db, package).await {
                        error!("Failed to save package: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Scraper {} failed: {}", scraper.name(), e);
            }
        }
        
        info!("Scraping cycle complete, sleeping for 1 hour...");
        sleep(Duration::from_secs(3600)).await;
    }
}

async fn save_package(db: &Database, package_data: ScrapedPackage) -> Result<()> {
    let package = Package {
        id: Uuid::new_v4().to_string(),
        rev: None,
        name: package_data.name,
        description: package_data.description,
        homepage: package_data.homepage,
        repository: package_data.repository,
        license: package_data.license,
        maintainers: package_data.maintainers,
        tags: package_data.tags,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        submitted_by: Some("scraper".to_string()),
    };

    let mut package_value = serde_json::to_value(&package)?;
    db.packages().save(&mut package_value).await?;
    
    // Save versions
    for version_data in package_data.versions {
        let version = PackageVersion {
            id: Uuid::new_v4().to_string(),
            rev: None,
            package_id: package.id.clone(),
            version: version_data.version,
            release_date: version_data.release_date,
            download_url: version_data.download_url,
            checksum: version_data.checksum,
            dependencies: version_data.dependencies,
            vulnerabilities: Vec::new(),
            changelog: version_data.changelog,
            created_at: Utc::now(),
        };
        
        let mut version_value = serde_json::to_value(&version)?;
        db.versions().save(&mut version_value).await?;
    }

    Ok(())
}
