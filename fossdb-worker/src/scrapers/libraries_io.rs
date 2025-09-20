use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::models::{Scraper, ScrapedPackage, ScrapedVersion, Dependency};

pub struct LibrariesIoScraper {
    client: Client,
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
    dependent_repositories_count: Option<u32>,
    rank: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LibrariesIoVersion {
    number: String,
    published_at: Option<DateTime<Utc>>,
    spdx_expression: Option<String>,
    original_license: Option<String>,
}

#[derive(Debug, Deserialize)]
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
struct LibrariesIoPlatform {
    name: String,
    project_count: u32,
    homepage: Option<String>,
    color: Option<String>,
    default_language: Option<String>,
}

impl LibrariesIoScraper {
    pub fn new(client: Client, api_key: String) -> Self {
        Self { client, api_key }
    }

    async fn get_platforms(&self) -> Result<Vec<LibrariesIoPlatform>> {
        let url = format!("https://libraries.io/api/platforms?api_key={}", self.api_key);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status().as_u16() == 429 {
            // Rate limited, wait a bit
            tokio::time::sleep(Duration::from_secs(60)).await;
            return Err(anyhow::anyhow!("Rate limited by libraries.io API"));
        }

        let platforms: Vec<LibrariesIoPlatform> = response.json().await?;
        Ok(platforms)
    }

    async fn get_project_dependencies(&self, platform: &str, name: &str, version: Option<&str>) -> Result<Vec<Dependency>> {
        let version_param = version.unwrap_or("latest");
        let url = format!(
            "https://libraries.io/api/{}/{}/{}/dependencies?api_key={}", 
            platform, name, version_param, self.api_key
        );
        
        // Add delay to respect rate limiting
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status().as_u16() == 429 {
            tokio::time::sleep(Duration::from_secs(60)).await;
            return Ok(Vec::new()); // Return empty on rate limit
        }

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

    async fn get_project_details(&self, platform: &str, name: &str) -> Result<Option<LibrariesIoProject>> {
        let url = format!(
            "https://libraries.io/api/{}/{}?api_key={}", 
            platform, name, self.api_key
        );
        
        // Add delay to respect rate limiting
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if response.status().as_u16() == 429 {
            tokio::time::sleep(Duration::from_secs(60)).await;
            return Ok(None);
        }

        let project: LibrariesIoProject = response.json().await?;
        Ok(Some(project))
    }

    async fn scrape_platform(&self, platform: &LibrariesIoPlatform) -> Result<Vec<ScrapedPackage>> {
        let mut packages = Vec::new();
        
        // Search for popular packages on this platform
        let search_url = format!(
            "https://libraries.io/api/search?platforms={}&sort=rank&per_page=50&api_key={}", 
            platform.name.to_lowercase(), self.api_key
        );

        // Add delay to respect rate limiting
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let response = self.client
            .get(&search_url)
            .send()
            .await?;

        if response.status().as_u16() == 429 {
            tokio::time::sleep(Duration::from_secs(60)).await;
            return Ok(packages);
        }

        let search_results: Vec<LibrariesIoProject> = response.json().await.unwrap_or_default();

        for project in search_results.into_iter().take(20) { // Limit to 20 packages per platform
            if let Some(project_details) = self.get_project_details(&project.platform, &project.name).await.unwrap_or(None) {
                let mut versions = Vec::new();
                
                // Create a version from the latest release info if available
                if let (Some(version_num), Some(release_date)) = (&project_details.latest_release_number, &project_details.latest_release_published_at) {
                    let dependencies = self.get_project_dependencies(&project.platform, &project.name, Some(version_num))
                        .await
                        .unwrap_or_default();

                    versions.push(ScrapedVersion {
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

                let package = ScrapedPackage {
                    name: project_details.name,
                    description: project_details.description,
                    homepage: project_details.homepage,
                    repository: project_details.repository_url,
                    license: project_details.licenses,
                    maintainers: Vec::new(), // Libraries.io doesn't provide maintainer info in basic API
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
impl Scraper for LibrariesIoScraper {
    fn name(&self) -> &str {
        "libraries.io"
    }

    async fn scrape(&self) -> Result<Vec<ScrapedPackage>> {
        let mut all_packages = Vec::new();

        // Get list of supported platforms
        let platforms = self.get_platforms().await?;
        
        // Focus on the most popular platforms to avoid overwhelming the API
        let priority_platforms = ["NPM", "Maven", "PyPI", "Packagist", "Go", "NuGet", "RubyGems"];
        
        for platform in platforms {
            if priority_platforms.contains(&platform.name.as_str()) {
                tracing::info!("Scraping libraries.io platform: {}", platform.name);
                
                match self.scrape_platform(&platform).await {
                    Ok(mut packages) => {
                        tracing::info!("Found {} packages from platform {}", packages.len(), platform.name);
                        all_packages.append(&mut packages);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to scrape platform {}: {}", platform.name, e);
                    }
                }

                // Add delay between platforms to respect rate limits
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }

        tracing::info!("Total packages scraped from libraries.io: {}", all_packages.len());
        Ok(all_packages)
    }
}