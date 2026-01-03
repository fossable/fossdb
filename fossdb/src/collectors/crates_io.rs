use anyhow::Result;
use async_trait::async_trait;
use crates_io_api::{AsyncClient, Sort};
use std::sync::Arc;

use crate::collector_models::Collector;
use crate::collectors::helpers;

pub struct CratesIoCollector {
    client: Arc<AsyncClient>,
}

impl CratesIoCollector {
    pub fn new(_client: reqwest::Client) -> Self {
        // crates_io_api handles rate limiting internally (1 req/s)
        // We don't need our custom rate limiting for this collector
        Self {
            client: Arc::new(
                AsyncClient::new(
                    "fossdb (https://github.com/fossable/fossdb)",
                    std::time::Duration::from_millis(1000),
                )
                .expect("Failed to create crates.io client"),
            ),
        }
    }
}

#[async_trait]
impl Collector for CratesIoCollector {
    fn name(&self) -> &str {
        "crates.io"
    }

    async fn collect(&self, db: Arc<crate::db::Database>) -> Result<()> {
        use crate::{Package, PackageVersion};
        use chrono::Utc;
        use std::collections::HashSet;

        // In debug mode, limit to 5 packages total
        let mut packages_processed = 0;
        let max_packages = if cfg!(debug_assertions) { 5 } else { usize::MAX };

        // Scrape first 3 pages of recently updated crates
        for page in 1..=3 {
            let mut query = crates_io_api::CratesQuery::builder()
                .sort(Sort::RecentUpdates)
                .page(page)
                .build();
            query.set_page_size(100);

            // Use async client directly
            let crates_page = self.client.crates(query).await?;

            tracing::info!(
                "Fetched {} crates from page {}",
                crates_page.crates.len(),
                page
            );

            // For each crate, check if we need to update it
            for krate in &crates_page.crates {
                let crate_name = krate.name.clone();

                // Check if package already exists
                match db.get_package_by_name(&crate_name) {
                    Ok(Some(existing_package)) => {
                        // Package exists - check if it has been updated since we last scraped
                        // Use the updated_at field from the search result to avoid unnecessary API calls
                        if krate.updated_at <= existing_package.updated_at {
                            // Package hasn't been updated since we last scraped it, skip
                            tracing::debug!(
                                "Package {} hasn't been updated (crates.io: {}, local: {}), skipping",
                                crate_name,
                                krate.updated_at,
                                existing_package.updated_at
                            );
                            continue;
                        }

                        // Package has been updated - fetch full details to check for new versions
                        tracing::info!(
                            "Package {} has been updated (crates.io: {}, local: {}), fetching details",
                            crate_name,
                            krate.updated_at,
                            existing_package.updated_at
                        );

                        match self.client.full_crate(&crate_name, false).await {
                            Ok(full_crate) => {
                                let existing_versions =
                                    db.get_versions_by_package(existing_package.id)?;
                                let existing_version_nums: HashSet<String> = existing_versions
                                    .iter()
                                    .map(|v| v.version.clone())
                                    .collect();

                                let now = Utc::now();

                                // Check for new versions
                                for v in full_crate.versions.iter().filter(|v| !v.yanked).take(10) {
                                    if !existing_version_nums.contains(&v.num) {
                                        // NEW VERSION FOUND!
                                        tracing::info!(
                                            "New version detected: {} {}",
                                            crate_name,
                                            v.num
                                        );

                                        let version = PackageVersion {
                                            id: 0,
                                            package_id: existing_package.id,
                                            version: v.num.clone(),
                                            release_date: v.created_at,
                                            download_url: Some(format!(
                                                "https://crates.io{}",
                                                v.dl_path
                                            )),
                                            checksum: None,
                                            dependencies: Vec::new(),
                                            vulnerabilities: Vec::new(),
                                            changelog: None,
                                            created_at: now,
                                        };

                                        // Save version - timeline events will be created automatically by the database listener
                                        if let Ok(_saved_version) = db.insert_version(version) {
                                            tracing::info!(
                                                "Saved new version {} for {}",
                                                v.num,
                                                crate_name
                                            );
                                        }
                                    }
                                }

                                // Update the package's updated_at timestamp
                                let mut updated_package = existing_package.clone();
                                updated_package.updated_at = krate.updated_at;
                                if let Err(e) = db.update_package(updated_package) {
                                    tracing::error!(
                                        "Failed to update package {} timestamp: {}",
                                        crate_name,
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to fetch crate details for {}: {}",
                                    crate_name,
                                    e
                                );
                            }
                        }
                        continue;
                    }
                    Ok(None) => {
                        // Package doesn't exist, fetch and save it
                        tracing::info!("New package discovered: {}", crate_name);
                        let crate_name_for_log = crate_name.clone();
                        match self.client.full_crate(&crate_name, false).await {
                            Ok(full_crate) => {
                                let now = Utc::now();

                                // Get the license from the latest version
                                let license = full_crate
                                    .versions
                                    .first()
                                    .and_then(|v| v.license.clone());

                                // Skip packages with non-free licenses
                                if let Some(ref lic) = license {
                                    if !helpers::is_free_license(lic) {
                                        tracing::info!(
                                            "Skipping package {} with non-free license: {}",
                                            crate_name_for_log,
                                            lic
                                        );
                                        continue;
                                    }
                                } else {
                                    tracing::info!(
                                        "Skipping package {} with no license information",
                                        crate_name_for_log
                                    );
                                    continue;
                                }

                                // Create and save the package using data from both search result and full details
                                let package = Package {
                                    id: 0, // Will be auto-generated
                                    name: full_crate.name.clone(),
                                    description: full_crate.description.clone(),
                                    homepage: full_crate.homepage.clone(),
                                    repository: full_crate.repository.clone(),
                                    license,
                                    tags: vec!["rust".to_string(), "crate".to_string()],
                                    created_at: now,
                                    updated_at: krate.updated_at, // Use timestamp from search result
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
                                        for v in full_crate
                                            .versions
                                            .iter()
                                            .filter(|v| !v.yanked)
                                            .take(10)
                                        {
                                            let version = PackageVersion {
                                                id: 0, // Will be auto-generated
                                                package_id: saved_package.id,
                                                version: v.num.clone(),
                                                release_date: v.created_at,
                                                download_url: Some(format!(
                                                    "https://crates.io{}",
                                                    v.dl_path
                                                )),
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
                                tracing::warn!(
                                    "Failed to fetch details for crate {}: {}",
                                    crate_name_for_log,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to check if package {} exists: {}", crate_name, e);
                    }
                }

                // Increment counter and check limit
                packages_processed += 1;
                if packages_processed >= max_packages {
                    if cfg!(debug_assertions) {
                        tracing::info!("Debug mode: Reached limit of {} packages, stopping collection", max_packages);
                    }
                    return Ok(());
                }
            }

            if crates_page.crates.len() < 100 {
                break;
            }
        }

        Ok(())
    }
}
