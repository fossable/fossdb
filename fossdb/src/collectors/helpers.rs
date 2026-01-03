use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;

use crate::db::Database;
use crate::{Package, PackageVersion};

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
            tracing::info!(
                "New version detected: {} {}",
                package_name,
                version_data.version
            );

            let version = create_version(&version_data, package_id, now);

            if let Ok(_saved_version) = db.insert_version(version) {
                tracing::info!(
                    "Saved new version {} for {}",
                    version_data.version,
                    package_name
                );
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

/// Check if a license string represents a free/open source license
/// Returns true if the license is free/open source, false if proprietary or unknown
pub fn is_free_license(license: &str) -> bool {
    // Normalize license string: lowercase and remove common separators
    let normalized = license.to_lowercase();

    // List of known free/open source licenses and their variations
    // Based on OSI-approved licenses and FSF free software licenses
    let free_licenses = [
        // Permissive licenses
        "mit", "apache", "apache-2.0", "apache 2.0", "bsd", "isc", "cc0",
        "unlicense", "wtfpl", "0bsd", "bsl-1.0", "ncsa", "zlib", "x11",

        // Copyleft licenses
        "gpl", "lgpl", "agpl", "mpl", "epl", "cpl", "cddl", "cecill",
        "eupl", "osl", "afl", "artistic",

        // Creative Commons free licenses
        "cc-by", "cc-by-sa",

        // Public domain
        "public domain", "publicdomain", "unlicensed",
    ];

    // Known non-free keywords
    let non_free_keywords = [
        "proprietary", "commercial", "private", "closed",
        "all rights reserved", "copyright only",
        // Non-free Creative Commons licenses
        "cc-by-nd", "cc-by-nc",
    ];

    // Check for non-free keywords first
    for keyword in &non_free_keywords {
        if normalized.contains(keyword) {
            return false;
        }
    }

    // Check if it matches any known free license
    for free_license in &free_licenses {
        if normalized.contains(free_license) {
            return true;
        }
    }

    // Handle SPDX-style "OR" expressions - if any part is free, consider it free
    if normalized.contains(" or ") || normalized.contains("/") {
        let parts: Vec<&str> = normalized.split(&[' ', '/', '|'][..]).collect();
        for part in parts {
            let part = part.trim();
            if part.is_empty() || part == "or" {
                continue;
            }
            for free_license in &free_licenses {
                if part.contains(free_license) {
                    return true;
                }
            }
        }
    }

    // If unknown, log it and reject for safety
    tracing::warn!("Unknown license, treating as non-free: {}", license);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_free_license() {
        // Common permissive licenses
        assert!(is_free_license("MIT"));
        assert!(is_free_license("mit"));
        assert!(is_free_license("Apache-2.0"));
        assert!(is_free_license("apache 2.0"));
        assert!(is_free_license("BSD-3-Clause"));
        assert!(is_free_license("ISC"));

        // Copyleft licenses
        assert!(is_free_license("GPL-3.0"));
        assert!(is_free_license("LGPL-2.1"));
        assert!(is_free_license("AGPL-3.0"));
        assert!(is_free_license("MPL-2.0"));

        // SPDX OR expressions
        assert!(is_free_license("MIT OR Apache-2.0"));
        assert!(is_free_license("GPL-2.0/GPL-3.0"));

        // Non-free licenses
        assert!(!is_free_license("proprietary"));
        assert!(!is_free_license("Commercial"));
        assert!(!is_free_license("All Rights Reserved"));
        assert!(!is_free_license("CC-BY-NC"));
        assert!(!is_free_license("CC-BY-ND"));

        // Unknown licenses (should be rejected)
        assert!(!is_free_license("CustomLicense"));
        assert!(!is_free_license(""));
    }
}
