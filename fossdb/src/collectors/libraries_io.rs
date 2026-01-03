use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

use crate::client::{AdaptiveConfig, AdaptiveRateLimitedClient};
use crate::collector_models::{CollectedPackage, CollectedVersion, Collector, Dependency};
use crate::collectors::helpers;

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
        // libraries.io has a 60 req/min rate limit for authenticated requests
        // Start conservative and let it adapt
        let config = AdaptiveConfig {
            initial_rate: 30, // 30 req/min
            min_rate: 6,      // 6 req/min minimum
            max_rate: 60,     // 60 req/min maximum
        };
        let adaptive_client = AdaptiveRateLimitedClient::new(client, config);
        Self {
            client: adaptive_client,
            api_key,
        }
    }

    async fn get_platforms(&self) -> Result<Vec<LibrariesIoPlatform>> {
        let url = format!(
            "https://libraries.io/api/platforms?api_key={}",
            self.api_key
        );

        let response = self.client.get(&url).await?;
        let platforms: Vec<LibrariesIoPlatform> = response.json().await?;
        Ok(platforms)
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
        let dependencies: Vec<LibrariesIoDependency> = response.json().await.unwrap_or_default();

        let deps = dependencies
            .into_iter()
            .map(|dep| Dependency {
                name: dep.name,
                version_requirement: dep.requirements,
                dependency_type: "runtime".to_string(), // Libraries.io doesn't distinguish types clearly
                optional: false,
            })
            .collect();

        Ok(deps)
    }

    async fn get_project_details(
        &self,
        platform: &str,
        name: &str,
    ) -> Result<Option<LibrariesIoProject>> {
        let url = format!(
            "https://libraries.io/api/{}/{}?api_key={}",
            platform, name, self.api_key
        );

        let response = self.client.get(&url).await?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        let project: LibrariesIoProject = response.json().await?;
        Ok(Some(project))
    }

    async fn scrape_platform(
        &self,
        platform: &LibrariesIoPlatform,
    ) -> Result<Vec<CollectedPackage>> {
        let mut packages = Vec::new();

        // Search for popular packages on this platform
        let search_url = format!(
            "https://libraries.io/api/search?platforms={}&sort=rank&per_page=50&api_key={}",
            platform.name.to_lowercase(),
            self.api_key
        );

        let response = self.client.get(&search_url).await?;
        let search_results: Vec<LibrariesIoProject> = response.json().await.unwrap_or_default();

        for project in search_results.into_iter().take(20) {
            // Limit to 20 packages per platform
            if let Some(project_details) = self
                .get_project_details(&project.platform, &project.name)
                .await
                .unwrap_or(None)
            {
                let mut versions = Vec::new();

                // Create a version from the latest release info if available
                if let (Some(version_num), Some(release_date)) = (
                    &project_details.latest_release_number,
                    &project_details.latest_release_published_at,
                ) {
                    let dependencies = self
                        .get_project_dependencies(
                            &project.platform,
                            &project.name,
                            Some(version_num),
                        )
                        .await
                        .unwrap_or_default();

                    versions.push(CollectedVersion {
                        version: version_num.clone(),
                        release_date: *release_date,
                        download_url: None, // Libraries.io doesn't provide direct download URLs
                        checksum: None,
                        dependencies,
                        changelog: None,
                    });
                }

                let mut tags = vec![
                    project_details.platform.to_lowercase(),
                    "libraries.io".to_string(),
                ];

                if let Some(lang) = &project_details.language {
                    tags.push(lang.to_lowercase());
                }

                if let Some(status) = &project_details.status {
                    tags.push(format!("status:{}", status.to_lowercase()));
                }

                // Skip packages with non-free licenses
                if let Some(ref lic) = project_details.licenses {
                    if !helpers::is_free_license(lic) {
                        tracing::info!(
                            "Skipping package {} with non-free license: {}",
                            project_details.name,
                            lic
                        );
                        continue;
                    }
                } else {
                    tracing::info!(
                        "Skipping package {} with no license information",
                        project_details.name
                    );
                    continue;
                }

                let package = CollectedPackage {
                    name: project_details.name,
                    description: project_details.description,
                    homepage: project_details.homepage,
                    repository: project_details.repository_url,
                    license: project_details.licenses,
                    tags,
                    versions,
                    platform: Some(project_details.platform),
                    language: project_details.language,
                    status: project_details.status,
                    dependents_count: project_details.dependents_count,
                    rank: project_details.rank,
                };

                packages.push(package);
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

    async fn collect(&self, db: Arc<crate::db::Database>) -> Result<()> {
        use crate::{Package, PackageVersion};
        use std::collections::HashSet;

        // In debug mode, limit to 5 packages total
        let mut packages_processed = 0;
        let max_packages = if cfg!(debug_assertions) { 5 } else { usize::MAX };

        // Get list of supported platforms
        let platforms = self.get_platforms().await?;

        // Focus on the most popular platforms to avoid overwhelming the API
        let priority_platforms = [
            "NPM",
            "Maven",
            "PyPI",
            "Packagist",
            "Go",
            "NuGet",
            "RubyGems",
        ];

        'platform_loop: for platform in platforms {
            if priority_platforms.contains(&platform.name.as_str()) {
                tracing::info!("Scraping libraries.io platform: {}", platform.name);

                match self.scrape_platform(&platform).await {
                    Ok(packages) => {
                        tracing::info!(
                            "Found {} packages from platform {}",
                            packages.len(),
                            platform.name
                        );

                        // Save each package to the database
                        for package_data in packages {
                            // Check if package already exists
                            match db.get_package_by_name(&package_data.name) {
                                Ok(Some(existing_package)) => {
                                    // Package exists - check for new versions
                                    tracing::debug!(
                                        "Package {} exists, checking for new versions",
                                        package_data.name
                                    );

                                    let existing_versions =
                                        match db.get_versions_by_package(existing_package.id) {
                                            Ok(v) => v,
                                            Err(e) => {
                                                tracing::error!(
                                                    "Failed to get existing versions for {}: {}",
                                                    package_data.name,
                                                    e
                                                );
                                                continue;
                                            }
                                        };

                                    let existing_version_nums: HashSet<String> = existing_versions
                                        .iter()
                                        .map(|v| v.version.clone())
                                        .collect();

                                    let now = chrono::Utc::now();

                                    for version_data in package_data.versions {
                                        if !existing_version_nums.contains(&version_data.version) {
                                            // New version found
                                            tracing::info!(
                                                "New version detected: {} {}",
                                                package_data.name,
                                                version_data.version
                                            );

                                            let version = PackageVersion {
                                                id: 0,
                                                package_id: existing_package.id,
                                                version: version_data.version.clone(),
                                                release_date: version_data.release_date,
                                                download_url: version_data.download_url,
                                                checksum: version_data.checksum,
                                                dependencies: version_data.dependencies,
                                                vulnerabilities: Vec::new(),
                                                changelog: version_data.changelog,
                                                created_at: now,
                                            };

                                            // Timeline events will be created automatically by the database listener
                                            if db.insert_version(version).is_ok() {
                                                tracing::info!(
                                                    "Saved new version {} for {}",
                                                    version_data.version,
                                                    package_data.name
                                                );
                                            }
                                        }
                                    }
                                    continue;
                                }
                                Ok(None) => {
                                    // Package doesn't exist, save it
                                    let now = Utc::now();

                                    let package = Package {
                                        id: 0, // Will be auto-generated
                                        name: package_data.name.clone(),
                                        description: package_data.description,
                                        homepage: package_data.homepage,
                                        repository: package_data.repository,
                                        license: package_data.license,
                                        tags: package_data.tags,
                                        created_at: now,
                                        updated_at: now,
                                        platform: package_data.platform,
                                        language: package_data.language,
                                        status: package_data.status,
                                        dependents_count: package_data.dependents_count,
                                        rank: package_data.rank,
                                    };

                                    match db.insert_package(package) {
                                        Ok(saved_package) => {
                                            tracing::info!("Saved package: {}", saved_package.name);

                                            // Save versions
                                            for version_data in package_data.versions {
                                                let version = PackageVersion {
                                                    id: 0, // Will be auto-generated
                                                    package_id: saved_package.id,
                                                    version: version_data.version.clone(),
                                                    release_date: version_data.release_date,
                                                    download_url: version_data.download_url,
                                                    checksum: version_data.checksum,
                                                    dependencies: version_data.dependencies,
                                                    vulnerabilities: Vec::new(),
                                                    changelog: version_data.changelog,
                                                    created_at: now,
                                                };

                                                if let Err(e) = db.insert_version(version) {
                                                    tracing::error!(
                                                        "Failed to save version {} for package {}: {}",
                                                        version_data.version,
                                                        saved_package.name,
                                                        e
                                                    );
                                                } else {
                                                    tracing::debug!(
                                                        "Saved version {} for package {}",
                                                        version_data.version,
                                                        saved_package.name
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to save package {} from libraries.io: {}",
                                                package_data.name,
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to check if package {} exists: {}",
                                        package_data.name,
                                        e
                                    );
                                }
                            }

                            // Increment counter and check limit
                            packages_processed += 1;
                            if packages_processed >= max_packages {
                                if cfg!(debug_assertions) {
                                    tracing::info!("Debug mode: Reached limit of {} packages, stopping collection", max_packages);
                                }
                                break 'platform_loop;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to scrape platform {}: {}", platform.name, e);
                    }
                }
            }
        }

        Ok(())
    }
}
