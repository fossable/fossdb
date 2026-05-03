use anyhow::Result;
use async_trait::async_trait;
use crates_io_api::{AsyncClient, Sort};
use fossdb::{CollectedPackage, CollectedVersion};
use std::sync::Arc;

use crate::client::CollectorClient;
use crate::collectors::{Collector, helpers};

pub struct CratesIoCollector {
    client: Arc<AsyncClient>,
}

impl CratesIoCollector {
    pub fn new(_client: reqwest::Client) -> Self {
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

    async fn collect(&self, client: Arc<CollectorClient>) -> Result<()> {
        let mut packages_processed = 0;
        let max_packages = if cfg!(debug_assertions) { 5 } else { usize::MAX };

        for page in 1..=3 {
            let mut query = crates_io_api::CratesQuery::builder()
                .sort(Sort::RecentUpdates)
                .page(page)
                .build();
            query.set_page_size(100);

            let crates_page = self.client.crates(query).await?;
            tracing::info!("Fetched {} crates from page {}", crates_page.crates.len(), page);

            for krate in &crates_page.crates {
                let crate_name = krate.name.clone();

                match client.get_package(&crate_name).await? {
                    Some(lookup) => {
                        if krate.updated_at <= lookup.package.updated_at {
                            tracing::debug!("Package {} hasn't been updated, skipping", crate_name);
                            continue;
                        }

                        match self.client.full_crate(&crate_name, false).await {
                            Ok(full_crate) => {
                                let new_versions: Vec<CollectedVersion> = full_crate
                                    .versions
                                    .iter()
                                    .filter(|v| !v.yanked && !lookup.version_strings.contains(&v.num))
                                    .take(10)
                                    .map(|v| CollectedVersion {
                                        version: v.num.clone(),
                                        release_date: v.created_at,
                                        download_url: Some(format!("https://crates.io{}", v.dl_path)),
                                        checksum: None,
                                        dependencies: Vec::new(),
                                        changelog: None,
                                    })
                                    .collect();

                                if !new_versions.is_empty() {
                                    let pkg = CollectedPackage {
                                        name: crate_name.clone(),
                                        description: None,
                                        homepage: None,
                                        repository: None,
                                        license: None,
                                        tags: Vec::new(),
                                        versions: new_versions,
                                        platform: None,
                                        language: None,
                                        status: None,
                                        dependents_count: None,
                                        rank: None,
                                    };
                                    if let Err(e) = client.submit_package(pkg).await {
                                        tracing::error!("Failed to submit new versions for {}: {}", crate_name, e);
                                    }
                                }

                                if let Err(e) = client.update_package_timestamp(&crate_name, krate.updated_at).await {
                                    tracing::error!("Failed to update timestamp for {}: {}", crate_name, e);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to fetch crate details for {}: {}", crate_name, e);
                            }
                        }
                    }
                    None => {
                        match self.client.full_crate(&crate_name, false).await {
                            Ok(full_crate) => {
                                let license = full_crate.versions.first().and_then(|v| v.license.clone());

                                if let Some(ref lic) = license {
                                    if !helpers::is_free_license(lic) {
                                        tracing::info!("Skipping {} with non-free license: {}", crate_name, lic);
                                        continue;
                                    }
                                } else {
                                    tracing::info!("Skipping {} with no license information", crate_name);
                                    continue;
                                }

                                let versions: Vec<CollectedVersion> = full_crate
                                    .versions
                                    .iter()
                                    .filter(|v| !v.yanked)
                                    .take(10)
                                    .map(|v| CollectedVersion {
                                        version: v.num.clone(),
                                        release_date: v.created_at,
                                        download_url: Some(format!("https://crates.io{}", v.dl_path)),
                                        checksum: None,
                                        dependencies: Vec::new(),
                                        changelog: None,
                                    })
                                    .collect();

                                let pkg = CollectedPackage {
                                    name: full_crate.name.clone(),
                                    description: full_crate.description.clone(),
                                    homepage: full_crate.homepage.clone(),
                                    repository: full_crate.repository.clone(),
                                    license,
                                    tags: vec!["rust".to_string(), "crate".to_string()],
                                    versions,
                                    platform: Some("crates.io".to_string()),
                                    language: Some("rust".to_string()),
                                    status: None,
                                    dependents_count: None,
                                    rank: None,
                                };

                                match client.submit_package(pkg).await {
                                    Ok(_) => tracing::info!("Submitted package: {}", crate_name),
                                    Err(e) => tracing::error!("Failed to submit {}: {}", crate_name, e),
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to fetch crate {}: {}", crate_name, e);
                            }
                        }
                    }
                }

                packages_processed += 1;
                if packages_processed >= max_packages {
                    if cfg!(debug_assertions) {
                        tracing::info!("Debug mode: reached limit of {} packages", max_packages);
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
