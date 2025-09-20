use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

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
    let packages_db = state.db.packages();
    
    match packages_db.get_all().await {
        Ok(packages) => {
            let packages: Vec<Package> = packages
                .get_data()
                .iter()
                .filter_map(|doc: &Value| serde_json::from_value(doc.clone()).ok())
                .collect();
            
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
    let packages_db = state.db.packages();
    
    match packages_db.get(&id).await {
        Ok(doc) => {
            let package: Package = serde_json::from_value(doc)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(package))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn create_package(
    State(state): State<AppState>,
    Json(payload): Json<CreatePackageRequest>,
) -> Result<Json<Package>, StatusCode> {
    let packages_db = state.db.packages();
    
    let package = Package {
        id: Uuid::new_v4().to_string(),
        rev: None,
        name: payload.name,
        description: payload.description,
        homepage: payload.homepage,
        repository: payload.repository,
        license: payload.license,
        maintainers: payload.maintainers,
        tags: payload.tags,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        submitted_by: None,
        platform: None,
        language: None,
        status: None,
        dependents_count: None,
        rank: None,
    };

    let mut package_value = serde_json::to_value(&package)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match packages_db.save(&mut package_value).await {
        Ok(_) => Ok(Json(package)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}