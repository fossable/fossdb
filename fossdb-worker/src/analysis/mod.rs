pub mod binary;
pub mod source;
#[cfg(feature = "analysis-symbolic")]
pub mod symbolic;

use anyhow::Result;
use chrono::Utc;
use fossdb::{AnalysisFinding, FindingKind, FindingSeverity, WorkerAnalysis, WorkerTask};
use sha2::{Digest, Sha256};
use std::io::Write;

const MAX_DOWNLOAD_BYTES: u64 = 100 * 1024 * 1024; // 100 MB

pub async fn analyze(task: &WorkerTask, http: &reqwest::Client) -> Result<WorkerAnalysis> {
    let download_url = task
        .version
        .download_url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("No download URL for version"))?;

    tracing::info!(
        "Analyzing {}@{} from {}",
        task.package.name,
        task.version.version,
        download_url
    );

    let tmp = tempfile::NamedTempFile::new()?;
    let (actual_checksum, bytes_downloaded) = download(http, download_url, tmp.path()).await?;
    tracing::debug!("Downloaded {} bytes, sha256={}", bytes_downloaded, actual_checksum);

    let mut findings = Vec::new();

    // Verify checksum if available
    let checksum_verified = if let Some(expected) = &task.version.checksum {
        let expected_norm = expected.trim().to_lowercase();
        let actual_norm = actual_checksum.to_lowercase();
        let matches = actual_norm == expected_norm
            || actual_norm == expected_norm.trim_start_matches("sha256:");
        if !matches {
            findings.push(AnalysisFinding {
                kind: FindingKind::ChecksumMismatch,
                severity: FindingSeverity::High,
                description: format!("Expected checksum {}, got {}", expected, actual_checksum),
                location: Some(download_url.to_string()),
            });
        }
        Some(matches)
    } else {
        None
    };

    // Detect format and dispatch to appropriate analyzer
    let url_lower = download_url.to_lowercase();
    let is_archive = url_lower.ends_with(".tar.gz")
        || url_lower.ends_with(".tgz")
        || url_lower.ends_with(".tar.bz2")
        || url_lower.ends_with(".tbz2")
        || url_lower.ends_with(".zip");

    if is_archive {
        match source::analyze_archive(tmp.path()) {
            Ok(mut source_findings) => findings.append(&mut source_findings),
            Err(e) => tracing::warn!("Source analysis failed: {}", e),
        }
    } else {
        match binary::analyze(tmp.path()) {
            Ok(mut binary_findings) => findings.append(&mut binary_findings),
            Err(e) => tracing::warn!("Binary analysis failed: {}", e),
        }
    }

    // Check if detected license matches declared license
    if let Some(declared) = &task.package.license {
        if let Some(detected_finding) = findings
            .iter()
            .find(|f| f.kind == FindingKind::LicenseDetected)
        {
            let detected = detected_finding
                .description
                .trim_start_matches("License file detected: ");
            if !declared.to_uppercase().contains(&detected.to_uppercase())
                && !detected.to_uppercase().contains(&declared.to_uppercase())
            {
                findings.push(AnalysisFinding {
                    kind: FindingKind::LicenseMismatch,
                    severity: FindingSeverity::Medium,
                    description: format!(
                        "Declared license '{}' does not match detected '{}'",
                        declared, detected
                    ),
                    location: None,
                });
            }
        }
    }

    let detected_license = findings
        .iter()
        .find(|f| f.kind == FindingKind::LicenseDetected)
        .map(|f| {
            f.description
                .trim_start_matches("License file detected: ")
                .to_string()
        });

    Ok(WorkerAnalysis {
        id: 0,
        package_id: task.package.id,
        version_id: task.version.id,
        analyzed_at: Utc::now(),
        findings,
        checksum_verified,
        detected_license,
    })
}

async fn download(
    http: &reqwest::Client,
    url: &str,
    dest: &std::path::Path,
) -> Result<(String, u64)> {
    let mut response = http.get(url).send().await?.error_for_status()?;

    let mut file = std::fs::File::create(dest)?;
    let mut hasher = Sha256::new();
    let mut total = 0u64;

    while let Some(chunk) = response.chunk().await? {
        if total + chunk.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err(anyhow::anyhow!(
                "Download exceeds {} byte limit",
                MAX_DOWNLOAD_BYTES
            ));
        }
        hasher.update(&chunk);
        file.write_all(&chunk)?;
        total += chunk.len() as u64;
    }

    let checksum = hex::encode(hasher.finalize());
    Ok((checksum, total))
}
