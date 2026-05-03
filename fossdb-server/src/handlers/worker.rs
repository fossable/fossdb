use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use fossdb::{WorkerAnalysis as FossdbWorkerAnalysis, WorkerTask};

use crate::{AppState, WorkerAnalysis};

pub async fn get_task(
    State(state): State<AppState>,
) -> Result<Json<WorkerTask>, StatusCode> {
    // Packages are assigned IDs in insertion order; iterate highest-ID first so
    // we always hand out the most recently added unanalyzed package.
    let mut packages = state.db.get_all_packages().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    packages.sort_unstable_by(|a, b| b.id.cmp(&a.id));

    let mut claimed = state
        .claimed_versions
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for pkg in &packages {
        let versions = state
            .db
            .get_versions_by_package(pkg.id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Pick the latest version that has a download URL
        let Some(version) = versions.into_iter().filter(|v| v.download_url.is_some()).max_by_key(|v| v.id) else {
            continue;
        };

        // Skip if already claimed by another worker
        if claimed.contains(&version.id) {
            continue;
        }

        // Skip if a completed analysis already exists for this version
        let existing = state
            .db
            .get_analyses_by_version(version.id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !existing.is_empty() {
            continue;
        }

        claimed.insert(version.id);
        return Ok(Json(WorkerTask {
            package: pkg.inner.clone(),
            version: version.inner,
        }));
    }

    Err(StatusCode::NOT_FOUND)
}

pub async fn submit_analysis(
    State(state): State<AppState>,
    Json(analysis): Json<FossdbWorkerAnalysis>,
) -> Result<Json<FossdbWorkerAnalysis>, StatusCode> {
    // Validate the referenced package and version exist
    state
        .db
        .get_package(analysis.package_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    state
        .db
        .get_version(analysis.version_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Release the claim so another worker can pick up this version if needed
    if let Ok(mut claimed) = state.claimed_versions.lock() {
        claimed.remove(&analysis.version_id);
    }

    let model = WorkerAnalysis { inner: analysis };
    let saved = state
        .db
        .insert_worker_analysis(model)
        .map_err(|e| {
            tracing::error!("Failed to insert worker analysis: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "Stored analysis for package_id={} version_id={} findings={}",
        saved.package_id,
        saved.version_id,
        saved.findings.len()
    );

    Ok(Json(saved.inner))
}

pub async fn get_package_analyses(
    State(state): State<AppState>,
    Path(package_id): Path<u64>,
) -> Result<Json<Vec<FossdbWorkerAnalysis>>, StatusCode> {
    let analyses = state
        .db
        .get_analyses_by_package(package_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(analyses.into_iter().map(|a| a.inner).collect()))
}
