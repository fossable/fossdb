use anyhow::Result;
use async_trait::async_trait;
use crates_io_api::{AsyncClient, Sort};
use std::sync::Arc;

use crate::scraper_models::Scraper;

pub struct CratesIoScraper {
    client: Arc<AsyncClient>,
}

impl CratesIoScraper {
    pub fn new(_client: reqwest::Client) -> Self {
        // crates_io_api handles rate limiting internally (1 req/s)
        // We don't need our custom rate limiting for this scraper
        Self {
            client: Arc::new(AsyncClient::new(
                "fossdb (https://github.com/fossable/fossdb)",
                std::time::Duration::from_millis(1000),
            ).expect("Failed to create crates.io client")),
        }
    }
}

#[async_trait]
impl Scraper for CratesIoScraper {
    fn name(&self) -> &str {
        "crates.io"
    }

    async fn scrape(&self, db: Arc<crate::db::Database>) -> Result<()> {
        use chrono::Utc;
        use crate::models::{Package, PackageVersion};

        // Scrape first 3 pages of recently updated crates
        for page in 1..=3 {
            let mut query = crates_io_api::CratesQuery::builder()
                .sort(Sort::RecentUpdates)
                .page(page)
                .build();
            query.set_page_size(100);

            // Use async client directly
            let crates_page = self.client.crates(query).await?;

            tracing::info!("Fetched {} crates from page {}", crates_page.crates.len(), page);

            // For each crate, fetch full details including versions
            for krate in &crates_page.crates {
                let crate_name = krate.name.clone();

                // Check if package already exists
                match db.get_package_by_name(&crate_name) {
                    Ok(Some(_)) => {
                        tracing::debug!("Package {} already exists, skipping", crate_name);
                        continue;
                    }
                    Ok(None) => {
                        // Package doesn't exist, fetch and save it
                        let crate_name_for_log = crate_name.clone();
                        match self.client.full_crate(&crate_name, false).await {
                            Ok(full_crate) => {
                                let now = Utc::now();

                                // Create and save the package
                                let package = Package {
                                    id: 0, // Will be auto-generated
                                    name: full_crate.name.clone(),
                                    description: full_crate.description.clone(),
                                    homepage: full_crate.homepage.clone(),
                                    repository: full_crate.repository.clone(),
                                    license: full_crate.versions.first().and_then(|v| v.license.clone()),
                                    maintainers: Vec::new(), // crates_io_api doesn't expose maintainers easily
                                    tags: vec!["rust".to_string(), "crate".to_string()],
                                    created_at: now,
                                    updated_at: now,
                                    submitted_by: Some("scraper".to_string()),
                                    platform: Some("crates.io".to_string()),
                                    language: Some("rust".to_string()),
                                    status: None,
                                    dependents_count: None,
                                    rank: None,
                                };

                                match db.insert_package(package) {
                                    Ok(saved_package) => {
                                        tracing::info!("Saved package: {}", saved_package.name);

                                        // Save versions (up to 10 non-yanked versions)
                                        for v in full_crate.versions.iter()
                                            .filter(|v| !v.yanked)
                                            .take(10)
                                        {
                                            let version = PackageVersion {
                                                id: 0, // Will be auto-generated
                                                package_id: saved_package.id,
                                                version: v.num.clone(),
                                                release_date: v.created_at,
                                                download_url: Some(format!("https://crates.io{}", v.dl_path)),
                                                checksum: None,
                                                dependencies: Vec::new(), // Could fetch dependencies if needed
                                                vulnerabilities: Vec::new(),
                                                changelog: None,
                                                created_at: now,
                                            };

                                            if let Err(e) = db.insert_version(version) {
                                                tracing::error!(
                                                    "Failed to save version {} for package {}: {}",
                                                    v.num,
                                                    saved_package.name,
                                                    e
                                                );
                                            } else {
                                                tracing::debug!(
                                                    "Saved version {} for package {}",
                                                    v.num,
                                                    saved_package.name
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to save package {}: {}",
                                            full_crate.name,
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to fetch details for crate {}: {}", crate_name_for_log, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to check if package {} exists: {}",
                            crate_name,
                            e
                        );
                    }
                }
            }

            if crates_page.crates.len() < 100 {
                break;
            }
        }

        Ok(())
    }
}
