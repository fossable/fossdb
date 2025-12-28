use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Scraper, ScrapedPackage, ScrapedVersion, Dependency};

pub struct NixpkgsScraper {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct NixPackage {
    #[serde(rename = "pname")]
    package_name: Option<String>,
    name: String,
    version: Option<String>,
    description: Option<String>,
    homepage: Option<serde_json::Value>,
    license: Option<serde_json::Value>,
    maintainers: Option<Vec<serde_json::Value>>,
    #[serde(rename = "meta")]
    metadata: Option<NixMeta>,
}

#[derive(Debug, Deserialize)]
struct NixMeta {
    description: Option<String>,
    homepage: Option<serde_json::Value>,
    license: Option<serde_json::Value>,
    maintainers: Option<Vec<serde_json::Value>>,
    platforms: Option<Vec<String>>,
}

impl NixpkgsScraper {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn extract_homepage(homepage: &Option<serde_json::Value>) -> Option<String> {
        match homepage {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Array(arr)) => {
                arr.first()
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            }
            _ => None,
        }
    }

    fn extract_license(license: &Option<serde_json::Value>) -> Option<String> {
        match license {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Object(obj)) => {
                obj.get("spdxId")
                    .or_else(|| obj.get("shortName"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            }
            Some(serde_json::Value::Array(arr)) => {
                arr.first()
                    .and_then(|v| Self::extract_license(&Some(v.clone())))
            }
            _ => None,
        }
    }

    fn extract_maintainers(maintainers: &Option<Vec<serde_json::Value>>) -> Vec<String> {
        maintainers
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|m| {
                match m {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Object(obj) => {
                        obj.get("name")
                            .or_else(|| obj.get("github"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    }
                    _ => None,
                }
            })
            .collect()
    }
}

#[async_trait]
impl Scraper for NixpkgsScraper {
    fn name(&self) -> &str {
        "nixpkgs"
    }

    async fn scrape(&self) -> Result<Vec<ScrapedPackage>> {
        // Note: This is a simplified implementation. In reality, you'd want to:
        // 1. Clone the nixpkgs repository or use the GitHub API
        // 2. Parse Nix expressions to extract package information
        // 3. Use the NixOS/nixpkgs API if available
        
        // For now, we'll use a mock approach that demonstrates the structure
        let url = "https://raw.githubusercontent.com/NixOS/nixpkgs/master/pkgs/top-level/all-packages.nix";
        
        // This is a simplified approach - in reality you'd need to parse Nix expressions
        let response = self.client
            .get(url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        // For demonstration, return some mock packages
        // In a real implementation, you'd parse the Nix expressions
        let mock_packages = vec![
            ScrapedPackage {
                name: "firefox".to_string(),
                description: Some("A web browser built from Firefox source tree".to_string()),
                homepage: Some("https://www.mozilla.org/firefox/".to_string()),
                repository: Some("https://github.com/mozilla/gecko-dev".to_string()),
                license: Some("MPL-2.0".to_string()),
                maintainers: vec!["nixos-maintainers".to_string()],
                tags: vec!["browser".to_string(), "nixpkgs".to_string(), "application".to_string()],
                versions: vec![
                    ScrapedVersion {
                        version: "120.0".to_string(),
                        release_date: Utc::now(),
                        download_url: None,
                        checksum: None,
                        dependencies: Vec::new(),
                        changelog: None,
                    }
                ],
            },
            ScrapedPackage {
                name: "vim".to_string(),
                description: Some("The most popular clone of the VI editor".to_string()),
                homepage: Some("https://www.vim.org/".to_string()),
                repository: Some("https://github.com/vim/vim".to_string()),
                license: Some("Vim".to_string()),
                maintainers: vec!["nixos-maintainers".to_string()],
                tags: vec!["editor".to_string(), "nixpkgs".to_string(), "terminal".to_string()],
                versions: vec![
                    ScrapedVersion {
                        version: "9.0".to_string(),
                        release_date: Utc::now(),
                        download_url: None,
                        checksum: None,
                        dependencies: Vec::new(),
                        changelog: None,
                    }
                ],
            },
        ];

        Ok(mock_packages)
    }
}