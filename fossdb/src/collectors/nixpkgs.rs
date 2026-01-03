use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use tokio::process::Command;

use crate::collector_models::Collector;

#[derive(Debug, Deserialize)]
struct NixSearchResult {
    #[serde(flatten)]
    packages: std::collections::HashMap<String, NixPackageInfo>,
}

#[derive(Debug, Deserialize)]
struct NixPackageInfo {
    pname: Option<String>,
    version: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NixPackageMeta {
    version: Option<String>,
    meta: NixMetaInfo,
}

#[derive(Debug, Deserialize)]
struct NixMetaInfo {
    description: Option<String>,
    homepage: Option<String>,
    license: Option<NixLicense>,
    maintainers: Option<Vec<NixMaintainer>>,
    changelog: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NixLicense {
    Single(NixLicenseInfo),
    Multiple(Vec<NixLicenseInfo>),
}

#[derive(Debug, Deserialize)]
struct NixLicenseInfo {
    #[serde(rename = "shortName")]
    short_name: Option<String>,
    #[serde(rename = "fullName")]
    full_name: Option<String>,
    #[serde(rename = "spdxId")]
    spdx_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NixMaintainer {
    email: Option<String>,
    github: Option<String>,
    name: Option<String>,
}

pub struct NixpkgsCollector {}

impl NixpkgsCollector {
    async fn run_nix_command(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("nix")
            .args(args)
            .output()
            .await
            .context("Failed to execute nix command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("nix command failed: {}", stderr);
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    async fn search_packages(&self) -> Result<Vec<(String, NixPackageInfo)>> {
        tracing::info!("Searching nixpkgs for packages...");

        // Search for all packages using nix search with JSON output
        let output = self
            .run_nix_command(&["search", "nixpkgs", "^", "--json"])
            .await?;

        let search_result: NixSearchResult =
            serde_json::from_str(&output).context("Failed to parse nix search output")?;

        let mut packages: Vec<(String, NixPackageInfo)> =
            search_result.packages.into_iter().collect();

        // Sort by attribute path for consistent ordering
        packages.sort_by(|a, b| a.0.cmp(&b.0));

        tracing::info!("Found {} packages from nixpkgs", packages.len());
        Ok(packages)
    }

    async fn get_package_details(&self, attr_path: &str) -> Result<NixPackageMeta> {
        tracing::debug!("Fetching details for {}", attr_path);

        // Build a Nix expression to get package metadata
        let expr = format!(
            r#"with import <nixpkgs> {{}}; let pkg = {}; in {{
                name = pkg.pname or pkg.name;
                version = pkg.version or null;
                meta = pkg.meta or {{}};
            }}"#,
            attr_path
                .strip_prefix("legacyPackages.x86_64-linux.")
                .or_else(|| attr_path.strip_prefix("packages.x86_64-linux."))
                .unwrap_or(attr_path)
        );

        let output = self
            .run_nix_command(&["eval", "--impure", "--expr", &expr, "--json"])
            .await?;

        let package_meta: NixPackageMeta =
            serde_json::from_str(&output).context("Failed to parse package details")?;

        Ok(package_meta)
    }
}

#[async_trait]
impl Collector for NixpkgsCollector {
    fn name(&self) -> &str {
        "nixpkgs"
    }

    async fn collect(&self, db: Arc<crate::db::Database>) -> Result<()> {
        use crate::{Package, PackageVersion};
        use chrono::Utc;

        tracing::info!("Starting nixpkgs collection...");

        // Search for packages
        let packages = self.search_packages().await?;

        for (attr_path, search_info) in packages {
            let package_name = search_info.pname.clone().unwrap_or_else(|| {
                // Extract package name from attribute path if pname is not available
                attr_path
                    .rsplit('.')
                    .next()
                    .unwrap_or(&attr_path)
                    .to_string()
            });

            // Check if package already exists
            match db.get_package_by_name(&package_name) {
                Ok(Some(_existing_package)) => {
                    tracing::debug!("Package {} already exists, skipping for now", package_name);
                    // For now, skip existing packages
                    // In the future, we could check for version updates
                    continue;
                }
                Ok(None) => {
                    // Package doesn't exist, fetch details and save it
                    tracing::info!("New package discovered: {}", package_name);

                    // Try to get detailed metadata
                    let package_meta = match self.get_package_details(&attr_path).await {
                        Ok(meta) => Some(meta),
                        Err(e) => {
                            tracing::warn!("Failed to fetch details for {}: {}", package_name, e);
                            None
                        }
                    };

                    let now = Utc::now();

                    // Extract maintainers
                    let maintainers = if let Some(ref meta) = package_meta {
                        meta.meta
                            .maintainers
                            .as_ref()
                            .map(|m| {
                                m.iter()
                                    .filter_map(|maintainer| {
                                        maintainer
                                            .name
                                            .clone()
                                            .or_else(|| maintainer.github.clone())
                                            .or_else(|| maintainer.email.clone())
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    // Extract license
                    let license = if let Some(ref meta) = package_meta {
                        meta.meta.license.as_ref().and_then(|lic| match lic {
                            NixLicense::Single(l) => l
                                .spdx_id
                                .clone()
                                .or_else(|| l.short_name.clone())
                                .or_else(|| l.full_name.clone()),
                            NixLicense::Multiple(licenses) => {
                                // Join multiple licenses with " OR "
                                let license_str = licenses
                                    .iter()
                                    .filter_map(|l| {
                                        l.spdx_id
                                            .clone()
                                            .or_else(|| l.short_name.clone())
                                            .or_else(|| l.full_name.clone())
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" OR ");
                                if license_str.is_empty() {
                                    None
                                } else {
                                    Some(license_str)
                                }
                            }
                        })
                    } else {
                        None
                    };

                    // Extract description - prefer meta description over search description
                    let description = package_meta
                        .as_ref()
                        .and_then(|m| m.meta.description.clone())
                        .or_else(|| search_info.description.clone());

                    // Extract homepage
                    let homepage = package_meta.as_ref().and_then(|m| m.meta.homepage.clone());

                    // Create the package
                    let package = Package {
                        id: 0,
                        name: package_name.clone(),
                        description,
                        homepage,
                        repository: None, // Nixpkgs doesn't directly expose repository URLs
                        license,
                        maintainers,
                        tags: vec!["nix".to_string(), "nixpkgs".to_string()],
                        created_at: now,
                        updated_at: now,
                        platform: Some("nixpkgs".to_string()),
                        language: None,
                        status: None,
                        dependents_count: None,
                        rank: None,
                    };

                    match db.insert_package(package) {
                        Ok(saved_package) => {
                            tracing::info!("Saved package: {}", saved_package.name);

                            // Save the current version if available
                            let version_string = package_meta
                                .as_ref()
                                .and_then(|m| m.version.clone())
                                .or_else(|| search_info.version.clone());

                            if let Some(version_str) = version_string {
                                let version = PackageVersion {
                                    id: 0,
                                    package_id: saved_package.id,
                                    version: version_str.clone(),
                                    release_date: now, // We don't have exact release dates from nix
                                    download_url: None,
                                    checksum: None,
                                    dependencies: Vec::new(),
                                    vulnerabilities: Vec::new(),
                                    changelog: package_meta
                                        .as_ref()
                                        .and_then(|m| m.meta.changelog.clone()),
                                    created_at: now,
                                };

                                if let Err(e) = db.insert_version(version) {
                                    tracing::error!(
                                        "Failed to save version {} for package {}: {}",
                                        version_str,
                                        saved_package.name,
                                        e
                                    );
                                } else {
                                    tracing::debug!(
                                        "Saved version {} for package {}",
                                        version_str,
                                        saved_package.name
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to save package {}: {}", package_name, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to check if package {} exists: {}", package_name, e);
                }
            }
        }

        tracing::info!("Nixpkgs collection completed");
        Ok(())
    }
}
