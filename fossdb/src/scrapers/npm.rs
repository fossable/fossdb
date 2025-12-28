use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Scraper, ScrapedPackage, ScrapedVersion, Dependency};

pub struct NpmScraper {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct NpmSearchResponse {
    objects: Vec<NpmPackageObject>,
    total: u32,
}

#[derive(Debug, Deserialize)]
struct NpmPackageObject {
    package: NpmPackageInfo,
    score: NpmScore,
}

#[derive(Debug, Deserialize)]
struct NpmPackageInfo {
    name: String,
    version: String,
    description: Option<String>,
    keywords: Option<Vec<String>>,
    date: DateTime<Utc>,
    links: Option<NpmLinks>,
    maintainers: Option<Vec<NpmMaintainer>>,
}

#[derive(Debug, Deserialize)]
struct NpmLinks {
    npm: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    bugs: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmMaintainer {
    username: String,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmScore {
    #[serde(rename = "final")]
    final_score: f64,
    detail: NpmScoreDetail,
}

#[derive(Debug, Deserialize)]
struct NpmScoreDetail {
    quality: f64,
    popularity: f64,
    maintenance: f64,
}

#[derive(Debug, Deserialize)]
struct NpmPackageDetails {
    name: String,
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: HashMap<String, String>,
    versions: HashMap<String, NpmVersionInfo>,
    time: HashMap<String, DateTime<Utc>>,
    license: Option<serde_json::Value>,
    homepage: Option<String>,
    repository: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct NpmVersionInfo {
    name: String,
    version: String,
    description: Option<String>,
    main: Option<String>,
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
    dist: Option<NpmDist>,
}

#[derive(Debug, Deserialize)]
struct NpmDist {
    tarball: Option<String>,
    shasum: Option<String>,
    integrity: Option<String>,
}

impl NpmScraper {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    async fn get_package_details(&self, package_name: &str) -> Result<Option<NpmPackageDetails>> {
        let url = format!("https://registry.npmjs.org/{}", package_name);
        
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let details: NpmPackageDetails = response.json().await?;
                Ok(Some(details))
            }
            _ => Ok(None),
        }
    }

    fn extract_license(license: &Option<serde_json::Value>) -> Option<String> {
        match license {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Object(obj)) => {
                obj.get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            }
            _ => None,
        }
    }

    fn extract_repository(repository: &Option<serde_json::Value>) -> Option<String> {
        match repository {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Object(obj)) => {
                obj.get("url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            }
            _ => None,
        }
    }
}

#[async_trait]
impl Scraper for NpmScraper {
    fn name(&self) -> &str {
        "npm"
    }

    async fn scrape(&self) -> Result<Vec<ScrapedPackage>> {
        let mut packages = Vec::new();
        let mut from = 0;
        let size = 50;

        // Only scrape first few pages
        while from < 200 {
            let url = format!(
                "https://registry.npmjs.org/-/v1/search?text=*&size={}&from={}&quality=0.65&popularity=0.98&maintenance=0.5",
                size, from
            );

            let response: NpmSearchResponse = self.client
                .get(&url)
                .send()
                .await?
                .json()
                .await?;

            for obj in response.objects {
                let pkg_info = obj.package;
                
                // Get detailed package information
                if let Ok(Some(details)) = self.get_package_details(&pkg_info.name).await {
                    let versions: Vec<ScrapedVersion> = details.versions
                        .iter()
                        .take(5) // Limit to latest 5 versions
                        .map(|(version, version_info)| {
                            let release_date = details.time
                                .get(version)
                                .copied()
                                .unwrap_or_else(|| pkg_info.date);

                            let dependencies = version_info.dependencies
                                .as_ref()
                                .map(|deps| {
                                    deps.iter()
                                        .map(|(name, version_req)| Dependency {
                                            name: name.clone(),
                                            version_constraint: version_req.clone(),
                                            optional: false,
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            ScrapedVersion {
                                version: version.clone(),
                                release_date,
                                download_url: version_info.dist.as_ref()
                                    .and_then(|d| d.tarball.clone()),
                                checksum: version_info.dist.as_ref()
                                    .and_then(|d| d.shasum.clone()),
                                dependencies,
                                changelog: None,
                            }
                        })
                        .collect();

                    let maintainers = pkg_info.maintainers
                        .unwrap_or_default()
                        .into_iter()
                        .map(|m| m.username)
                        .collect();

                    let mut tags = vec!["javascript".to_string(), "npm".to_string()];
                    if let Some(keywords) = pkg_info.keywords {
                        tags.extend(keywords);
                    }

                    let package = ScrapedPackage {
                        name: pkg_info.name,
                        description: pkg_info.description,
                        homepage: details.homepage.or(pkg_info.links.and_then(|l| l.homepage)),
                        repository: Self::extract_repository(&details.repository),
                        license: Self::extract_license(&details.license),
                        maintainers,
                        tags,
                        versions,
                    };

                    packages.push(package);
                }
            }

            if response.objects.len() < size as usize {
                break;
            }

            from += size;
        }

        Ok(packages)
    }
}