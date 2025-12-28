use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;

use crate::{models::*, AppState};

#[derive(Debug, Deserialize)]
pub struct ListPackagesQuery {
    page: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
    tag: Option<String>,
}

pub async fn list_packages(
    Query(params): Query<ListPackagesQuery>,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    match state.db.get_all_packages() {
        Ok(packages) => {
            Ok(Json(serde_json::json!({
                "packages": packages,
                "total": packages.len()
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_package(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Package>, StatusCode> {
    let id = id.parse::<u64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match state.db.get_package(id) {
        Ok(Some(package)) => Ok(Json(package)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn create_package(
    State(state): State<AppState>,
    Json(payload): Json<CreatePackageRequest>,
) -> Result<Json<Package>, StatusCode> {
    let now = Utc::now();

    let package = Package {
        id: 0,  // Will be auto-generated
        name: payload.name,
        description: payload.description,
        homepage: payload.homepage,
        repository: payload.repository,
        license: payload.license,
        maintainers: payload.maintainers,
        tags: payload.tags,
        created_at: now,
        updated_at: now,
        submitted_by: None,
        platform: None,
        language: None,
        status: None,
        dependents_count: None,
        rank: None,
    };

    match state.db.insert_package(package) {
        Ok(package) => Ok(Json(package)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}