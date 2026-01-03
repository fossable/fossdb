use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;

use crate::{AppState, Package, PackageVersion, CreatePackageRequest};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
        Ok(mut packages) => {
            // Filter by search term if provided
            if let Some(search) = &params.search {
                let search_lower = search.to_lowercase();
                packages.retain(|pkg| {
                    pkg.name.to_lowercase().contains(&search_lower)
                        || pkg
                            .description
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&search_lower))
                            .unwrap_or(false)
                });
            }

            // Filter by tag if provided
            if let Some(tag) = &params.tag {
                packages.retain(|pkg| pkg.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)));
            }

            // Apply pagination
            let total = packages.len();
            let limit = params.limit.unwrap_or(50).min(100) as usize;
            let page = params.page.unwrap_or(1).max(1);
            let offset = ((page - 1) * limit as u32) as usize;

            let paginated_packages: Vec<Package> =
                packages.into_iter().skip(offset).take(limit).collect();

            Ok(Json(serde_json::json!({
                "packages": paginated_packages,
                "total": total,
                "page": page,
                "limit": limit
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_package(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Package>, StatusCode> {
    let id = id.parse::<u64>().map_err(|_| StatusCode::BAD_REQUEST)?;

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
        id: 0, // Will be auto-generated
        name: payload.name,
        description: payload.description,
        homepage: payload.homepage,
        repository: payload.repository,
        license: payload.license,
        maintainers: payload.maintainers,
        tags: payload.tags,
        created_at: now,
        updated_at: now,
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

pub async fn get_package_versions(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<PackageVersion>>, StatusCode> {
    let id = id.parse::<u64>().map_err(|_| StatusCode::BAD_REQUEST)?;

    match state.db.get_versions_by_package(id) {
        Ok(versions) => Ok(Json(versions)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_package_subscriber_count(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let id = id.parse::<u64>().map_err(|_| StatusCode::BAD_REQUEST)?;

    // First get the package to get its name
    let package = match state.db.get_package(id) {
        Ok(Some(pkg)) => pkg,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Get subscriber count
    match state.db.get_users_subscribed_to(&package.name) {
        Ok(subscribers) => Ok(Json(serde_json::json!({
            "package_id": id,
            "package_name": package.name,
            "subscriber_count": subscribers.len()
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
