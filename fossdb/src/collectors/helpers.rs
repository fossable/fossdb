use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;

use crate::db::Database;
use crate::models::{Package, PackageVersion};

/// Helper for checking and inserting new versions for an existing package
pub async fn check_and_insert_new_versions<F>(
    db: &Arc<Database>,
    package_id: u64,
    package_name: &str,
    new_versions: Vec<VersionData>,
    create_version: F,
) -> Result<usize>
where
    F: Fn(&VersionData, u64, DateTime<Utc>) -> PackageVersion,
{
    let existing_versions = db.get_versions_by_package(package_id)?;
    let existing_version_nums: HashSet<String> = existing_versions
        .iter()
        .map(|v| v.version.clone())
        .collect();

    let now = Utc::now();
    let mut new_count = 0;

    for version_data in new_versions {
        if !existing_version_nums.contains(&version_data.version) {
            tracing::info!("New version detected: {} {}", package_name, version_data.version);

            let version = create_version(&version_data, package_id, now);

            if let Ok(_saved_version) = db.insert_version(version) {
                tracing::info!("Saved new version {} for {}", version_data.version, package_name);
                new_count += 1;
            }
        }
    }

    Ok(new_count)
}

/// Helper for creating a new package and its versions
pub async fn insert_package_with_versions<F>(
    db: &Arc<Database>,
    package: Package,
    versions: Vec<VersionData>,
    create_version: F,
) -> Result<Package>
where
    F: Fn(&VersionData, u64, DateTime<Utc>) -> PackageVersion,
{
    let saved_package = db.insert_package(package)?;
    tracing::info!("Saved package: {}", saved_package.name);

    let now = Utc::now();

    for version_data in versions {
        let version = create_version(&version_data, saved_package.id, now);

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

    Ok(saved_package)
}

/// Generic version data structure that collectors can convert to
#[derive(Debug, Clone)]
pub struct VersionData {
    pub version: String,
    pub release_date: DateTime<Utc>,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub changelog: Option<String>,
}

impl VersionData {
    pub fn new(version: String, release_date: DateTime<Utc>) -> Self {
        Self {
            version,
            release_date,
            download_url: None,
            checksum: None,
            changelog: None,
        }
    }

    pub fn with_download_url(mut self, url: Option<String>) -> Self {
        self.download_url = url;
        self
    }

    pub fn with_checksum(mut self, checksum: Option<String>) -> Self {
        self.checksum = checksum;
        self
    }

    pub fn with_changelog(mut self, changelog: Option<String>) -> Self {
        self.changelog = changelog;
        self
    }
}
