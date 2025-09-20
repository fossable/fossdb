use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// Re-export types from backend for consistency
pub use crate::db::Package;
pub use crate::db::PackageVersion;
pub use crate::db::Dependency;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedPackage {
    pub name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub maintainers: Vec<String>,
    pub tags: Vec<String>,
    pub versions: Vec<ScrapedVersion>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
    pub dependents_count: Option<u32>,
    pub rank: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedVersion {
    pub version: String,
    pub release_date: DateTime<Utc>,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub changelog: Option<String>,
}

#[async_trait::async_trait]
pub trait Scraper: Send + Sync {
    fn name(&self) -> &str;
    async fn scrape(&self) -> anyhow::Result<Vec<ScrapedPackage>>;
}