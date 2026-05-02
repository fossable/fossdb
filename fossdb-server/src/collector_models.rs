use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export types for consistency
pub use crate::Dependency;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedPackage {
    pub name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub versions: Vec<CollectedVersion>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
    pub dependents_count: Option<u32>,
    pub rank: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedVersion {
    pub version: String,
    pub release_date: DateTime<Utc>,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub changelog: Option<String>,
}

#[async_trait::async_trait]
pub trait Collector: Send + Sync {
    fn name(&self) -> &str;
    async fn collect(&self, db: std::sync::Arc<crate::db::Database>) -> anyhow::Result<()>;
}
