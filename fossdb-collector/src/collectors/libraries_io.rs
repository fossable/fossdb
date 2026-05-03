use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fossdb::{CollectedPackage, CollectedVersion, Dependency};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

use crate::client::{AdaptiveConfig, AdaptiveRateLimitedClient, CollectorClient};
use crate::collectors::{Collector, helpers};

pub struct LibrariesIoCollector {
    client: AdaptiveRateLimitedClient,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct LibrariesIoProject {
    name: String,
    platform: String,
    description: Option<String>,
    homepage: Option<String>,
    repository_url: Option<String>,
    licenses: Option<String>,
    latest_release_number: Option<String>,
    latest_release_published_at: Option<DateTime<Utc>>,
    language: Option<String>,
    status: Option<String>,
    dependents_count: Option<u32>,
    #[allow(dead_code)]
    dependent_repositories_count: Option<u32>,
    rank: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LibrariesIoVersion {
    number: String,
    published_at: Option<DateTime<Utc>>,
    spdx_expression: Option<String>,
    original_license: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LibrariesIoDependency {
    project_name: String,
    name: String,
    platform: String,
    requirements: String,
    latest_stable: Option<String>,
    latest: Option<String>,
    deprecated: Option<bool>,
    outdated: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LibrariesIoPlatform {
    name: String,
    project_count: u32,
    homepage: Option<String>,
    color: Option<String>,
    default_language: Option<String>,
}

impl LibrariesIoCollector {
    pub fn new(client: Client, api_key: String) -> Self {
        let config = AdaptiveConfig {
            initial_rate: 30,
            min_rate: 6,
            max_rate: 60,
        };
        Self {
            client: AdaptiveRateLimitedClient::new(client, config),
            api_key,
        }
    }

    async fn get_platforms(&self) -> Result<Vec<LibrariesIoPlatform>> {
        let url = format!("https://libraries.io/api/platforms?api_key={}", self.api_key);
        let response = self.client.get(&url).await?;
        Ok(response.json().await?)
    }

    async fn get_project_dependencies(
        &self,
        platform: &str,
        name: &str,
        version: Option<&str>,
    ) -> Result<Vec<Dependency>> {
        let version_param = version.unwrap_or("latest");
        let url = format!(
            "https://libraries.io/api/{}/{}/{}/dependencies?api_key={}",
            platform, name, version_param, self.api_key
        );
        let response = self.client.get(&url).await?;
        let deps: Vec<LibrariesIoDependency> = response.json().await.unwrap_or_default();
        Ok(deps.into_iter().map(|dep| Dependency {
            name: dep.name,
            version_requirement: dep.requirements,
            dependency_type: "runtime".to_string(),
            optional: false,
        }).collect())
    }

    async fn get_project_details(&self, platform: &str, name: &str) -> Result<Option<LibrariesIoProject>> {
        let url = format!("https://libraries.io/api/{}/{}?api_key={}", platform, name, self.api_key);
        let response = self.client.get(&url).await?;
        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        Ok(Some(response.json().await?))
    }

    async fn scrape_platform(&self, platform: &LibrariesIoPlatform) -> Result<Vec<CollectedPackage>> {
        let mut packages = Vec::new();
        let search_url = format!(
            "https://libraries.io/api/search?platforms={}&sort=rank&per_page=50&api_key={}",
            platform.name.to_lowercase(),
            self.api_key
        );

        let response = self.client.get(&search_url).await?;
        let search_results: Vec<LibrariesIoProject> = response.json().await.unwrap_or_default();

        for project in search_results.into_iter().take(20) {
            if let Some(details) = self.get_project_details(&project.platform, &project.name).await.unwrap_or(None) {
                if let Some(ref lic) = details.licenses {
                    if !helpers::is_free_license(lic) {
                        tracing::info!("Skipping {} with non-free license: {}", details.name, lic);
                        continue;
                    }
                } else {
                    tracing::info!("Skipping {} with no license information", details.name);
                    continue;
                }

                let mut versions = Vec::new();
                if let (Some(ver), Some(date)) = (&details.latest_release_number, &details.latest_release_published_at) {
                    let deps = self.get_project_dependencies(&project.platform, &project.name, Some(ver))
                        .await
                        .unwrap_or_default();
                    versions.push(CollectedVersion {
                        version: ver.clone(),
                        release_date: *date,
                        download_url: None,
                        checksum: None,
                        dependencies: deps,
                        changelog: None,
                    });
                }

                let mut tags = vec![details.platform.to_lowercase(), "libraries.io".to_string()];
                if let Some(lang) = &details.language {
                    tags.push(lang.to_lowercase());
                }

                packages.push(CollectedPackage {
                    name: details.name,
                    description: details.description,
                    homepage: details.homepage,
                    repository: details.repository_url,
                    license: details.licenses,
                    tags,
                    versions,
                    platform: Some(details.platform),
                    language: details.language,
                    status: details.status,
                    dependents_count: details.dependents_count,
                    rank: details.rank,
                });
            }
        }

        Ok(packages)
    }
}

#[async_trait]
impl Collector for LibrariesIoCollector {
    fn name(&self) -> &str {
        "libraries.io"
    }

    async fn collect(&self, client: Arc<CollectorClient>) -> Result<()> {
        let mut packages_processed = 0;
        let max_packages = if cfg!(debug_assertions) { 5 } else { usize::MAX };

        let platforms = self.get_platforms().await?;
        let priority_platforms = ["NPM", "Maven", "PyPI", "Packagist", "Go", "NuGet", "RubyGems"];

        'platform_loop: for platform in platforms {
            if !priority_platforms.contains(&platform.name.as_str()) {
                continue;
            }

            tracing::info!("Scraping libraries.io platform: {}", platform.name);

            match self.scrape_platform(&platform).await {
                Ok(packages) => {
                    for pkg in packages {
                        match client.submit_package(pkg).await {
                            Ok(_) => {}
                            Err(e) => tracing::error!("Failed to submit package: {}", e),
                        }

                        packages_processed += 1;
                        if packages_processed >= max_packages {
                            if cfg!(debug_assertions) {
                                tracing::info!("Debug mode: reached limit of {} packages", max_packages);
                            }
                            break 'platform_loop;
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to scrape platform {}: {}", platform.name, e),
            }
        }

        Ok(())
    }
}
