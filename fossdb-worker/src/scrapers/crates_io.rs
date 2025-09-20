use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::models::{Scraper, ScrapedPackage, ScrapedVersion};

pub struct CratesIoScraper {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct CratesResponse {
    crates: Vec<CrateInfo>,
    meta: CratesMeta,
}

#[derive(Debug, Deserialize)]
struct CratesMeta {
    total: u32,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    id: String,
    name: String,
    description: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    max_version: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CrateVersionsResponse {
    versions: Vec<CrateVersion>,
}

#[derive(Debug, Deserialize)]
struct CrateVersion {
    num: String,
    created_at: DateTime<Utc>,
    downloads: u64,
    features: serde_json::Value,
    yanked: bool,
}

impl CratesIoScraper {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    async fn get_crate_versions(&self, crate_name: &str) -> Result<Vec<ScrapedVersion>> {
        let url = format!("https://crates.io/api/v1/crates/{}/versions", crate_name);
        let response: CrateVersionsResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let versions = response.versions
            .into_iter()
            .filter(|v| !v.yanked)
            .take(10) // Limit to latest 10 versions
            .map(|v| {
                let version_num = v.num.clone();
                ScrapedVersion {
                    version: v.num,
                    release_date: v.created_at,
                    download_url: Some(format!("https://crates.io/api/v1/crates/{}/{}/download", crate_name, version_num)),
                    checksum: None,
                    dependencies: Vec::new(), // Would need additional API call
                    changelog: None,
                }
            })
            .collect();

        Ok(versions)
    }
}

#[async_trait]
impl Scraper for CratesIoScraper {
    fn name(&self) -> &str {
        "crates.io"
    }

    async fn scrape(&self) -> Result<Vec<ScrapedPackage>> {
        let mut packages = Vec::new();
        let mut page = 1;
        let per_page = 100;

        // Only scrape first few pages to avoid overwhelming the API
        while page <= 3 {
            let url = format!(
                "https://crates.io/api/v1/crates?page={}&per_page={}&sort=recent-updates",
                page, per_page
            );

            let response: CratesResponse = self.client
                .get(&url)
                .send()
                .await?
                .json()
                .await?;

            let crates_len = response.crates.len();
            for crate_info in response.crates {
                let versions = self.get_crate_versions(&crate_info.name).await.unwrap_or_default();
                
                let package = ScrapedPackage {
                    name: crate_info.name,
                    description: crate_info.description,
                    homepage: crate_info.homepage,
                    repository: crate_info.repository,
                    license: None, // Would need additional API call
                    maintainers: Vec::new(), // Would need additional API call
                    tags: vec!["rust".to_string(), "crate".to_string()],
                    versions,
                    platform: Some("crates.io".to_string()),
                    language: Some("rust".to_string()),
                    status: None,
                    dependents_count: None,
                    rank: None,
                };

                packages.push(package);
            }

            if crates_len < per_page as usize {
                break;
            }

            page += 1;
        }

        Ok(packages)
    }
}