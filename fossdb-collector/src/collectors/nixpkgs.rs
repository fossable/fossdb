use anyhow::{Context, Result};
use async_trait::async_trait;
use fossdb::{CollectedPackage, CollectedVersion};
use serde::Deserialize;
use std::sync::Arc;
use tokio::process::Command;

use crate::client::CollectorClient;
use crate::collectors::{Collector, helpers};

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
        let output = self.run_nix_command(&["search", "nixpkgs", "^", "--json"]).await?;
        let search_result: NixSearchResult =
            serde_json::from_str(&output).context("Failed to parse nix search output")?;
        let mut packages: Vec<(String, NixPackageInfo)> =
            search_result.packages.into_iter().collect();
        packages.sort_by(|a, b| a.0.cmp(&b.0));
        tracing::info!("Found {} packages from nixpkgs", packages.len());
        Ok(packages)
    }

    async fn get_package_details(&self, attr_path: &str) -> Result<NixPackageMeta> {
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
        let output = self.run_nix_command(&["eval", "--impure", "--expr", &expr, "--json"]).await?;
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

    async fn collect(&self, client: Arc<CollectorClient>) -> Result<()> {
        use chrono::Utc;

        tracing::info!("Starting nixpkgs collection...");

        let mut packages_processed = 0;
        let max_packages = if cfg!(debug_assertions) { 5 } else { usize::MAX };

        let packages = self.search_packages().await?;

        for (attr_path, search_info) in packages {
            let package_name = search_info.pname.clone().unwrap_or_else(|| {
                attr_path.rsplit('.').next().unwrap_or(&attr_path).to_string()
            });

            match client.get_package(&package_name).await? {
                Some(_) => {
                    tracing::debug!("Package {} already exists, skipping", package_name);
                    continue;
                }
                None => {
                    let package_meta = match self.get_package_details(&attr_path).await {
                        Ok(meta) => Some(meta),
                        Err(e) => {
                            tracing::warn!("Failed to fetch details for {}: {}", package_name, e);
                            None
                        }
                    };

                    let now = Utc::now();

                    let license = if let Some(ref meta) = package_meta {
                        meta.meta.license.as_ref().and_then(|lic| match lic {
                            NixLicense::Single(l) => l.spdx_id.clone()
                                .or_else(|| l.short_name.clone())
                                .or_else(|| l.full_name.clone()),
                            NixLicense::Multiple(licenses) => {
                                let s = licenses
                                    .iter()
                                    .filter_map(|l| l.spdx_id.clone()
                                        .or_else(|| l.short_name.clone())
                                        .or_else(|| l.full_name.clone()))
                                    .collect::<Vec<_>>()
                                    .join(" OR ");
                                if s.is_empty() { None } else { Some(s) }
                            }
                        })
                    } else {
                        None
                    };

                    if let Some(ref lic) = license {
                        if !helpers::is_free_license(lic) {
                            tracing::info!("Skipping {} with non-free license: {}", package_name, lic);
                            continue;
                        }
                    } else {
                        tracing::info!("Skipping {} with no license information", package_name);
                        continue;
                    }

                    let description = package_meta.as_ref()
                        .and_then(|m| m.meta.description.clone())
                        .or_else(|| search_info.description.clone());
                    let homepage = package_meta.as_ref().and_then(|m| m.meta.homepage.clone());

                    let version_string = package_meta.as_ref()
                        .and_then(|m| m.version.clone())
                        .or_else(|| search_info.version.clone());

                    let versions = if let Some(ver) = version_string {
                        vec![CollectedVersion {
                            version: ver,
                            release_date: now,
                            download_url: None,
                            checksum: None,
                            dependencies: Vec::new(),
                            changelog: package_meta.as_ref().and_then(|m| m.meta.changelog.clone()),
                        }]
                    } else {
                        Vec::new()
                    };

                    let pkg = CollectedPackage {
                        name: package_name.clone(),
                        description,
                        homepage,
                        repository: None,
                        license,
                        tags: vec!["nix".to_string(), "nixpkgs".to_string()],
                        versions,
                        platform: Some("nixpkgs".to_string()),
                        language: None,
                        status: None,
                        dependents_count: None,
                        rank: None,
                    };

                    match client.submit_package(pkg).await {
                        Ok(_) => tracing::info!("Submitted package: {}", package_name),
                        Err(e) => tracing::error!("Failed to submit {}: {}", package_name, e),
                    }
                }
            }

            packages_processed += 1;
            if packages_processed >= max_packages {
                if cfg!(debug_assertions) {
                    tracing::info!("Debug mode: reached limit of {} packages", max_packages);
                }
                break;
            }
        }

        tracing::info!("Nixpkgs collection completed");
        Ok(())
    }
}
