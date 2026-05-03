use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use fossdb::{CollectedPackage, PackageLookup};
use serde::Deserialize;
use std::collections::HashSet;

use crate::AppState;
use crate::{Package, PackageVersion};

#[derive(Deserialize)]
pub struct PackageQuery {
    pub name: String,
}

pub async fn get_package(
    State(state): State<AppState>,
    Query(q): Query<PackageQuery>,
) -> Result<Json<PackageLookup>, StatusCode> {
    match state.db.get_package_by_name(&q.name) {
        Ok(Some(pkg)) => {
            let version_strings = state.db
                .get_versions_by_package(pkg.id)
                .unwrap_or_default()
                .into_iter()
                .map(|v| v.version.clone())
                .collect();
            Ok(Json(PackageLookup {
                package: pkg.inner,
                version_strings,
            }))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn upsert_package(
    State(state): State<AppState>,
    Json(collected): Json<CollectedPackage>,
) -> Result<Json<fossdb::Package>, StatusCode> {
    let now = Utc::now();

    match state.db.get_package_by_name(&collected.name) {
        Ok(Some(existing)) => {
            let existing_versions: HashSet<String> = state.db
                .get_versions_by_package(existing.id)
                .unwrap_or_default()
                .into_iter()
                .map(|v| v.version.clone())
                .collect();

            for v in collected.versions {
                if !existing_versions.contains(&v.version) {
                    tracing::info!("New version detected: {} {}", existing.name, v.version);
                    let version = PackageVersion {
                        inner: fossdb::PackageVersion {
                            id: 0,
                            package_id: existing.id,
                            version: v.version,
                            release_date: v.release_date,
                            download_url: v.download_url,
                            checksum: v.checksum,
                            dependencies: v.dependencies,
                            vulnerabilities: Vec::new(),
                            changelog: v.changelog,
                            created_at: now,
                        },
                    };
                    if let Err(e) = state.db.insert_version(version) {
                        tracing::error!("Failed to insert version: {}", e);
                    }
                }
            }

            Ok(Json(existing.inner))
        }
        Ok(None) => {
            let package = Package {
                inner: fossdb::Package {
                    id: 0,
                    name: collected.name,
                    description: collected.description,
                    homepage: collected.homepage,
                    repository: collected.repository,
                    license: collected.license,
                    tags: collected.tags,
                    created_at: now,
                    updated_at: now,
                    platform: collected.platform,
                    language: collected.language,
                    status: collected.status,
                    dependents_count: collected.dependents_count,
                    rank: collected.rank,
                },
            };

            match state.db.insert_package(package) {
                Ok(saved) => {
                    tracing::info!("Saved new package: {}", saved.name);
                    for v in collected.versions {
                        let version = PackageVersion {
                            inner: fossdb::PackageVersion {
                                id: 0,
                                package_id: saved.id,
                                version: v.version,
                                release_date: v.release_date,
                                download_url: v.download_url,
                                checksum: v.checksum,
                                dependencies: v.dependencies,
                                vulnerabilities: Vec::new(),
                                changelog: v.changelog,
                                created_at: now,
                            },
                        };
                        if let Err(e) = state.db.insert_version(version) {
                            tracing::error!("Failed to insert version: {}", e);
                        }
                    }
                    Ok(Json(saved.inner))
                }
                Err(e) => {
                    tracing::error!("Failed to insert package: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            tracing::error!("DB error checking package: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
pub struct TimestampUpdate {
    pub updated_at: DateTime<Utc>,
}

pub async fn update_package_timestamp(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<TimestampUpdate>,
) -> StatusCode {
    match state.db.get_package_by_name(&name) {
        Ok(Some(mut pkg)) => {
            pkg.updated_at = body.updated_at;
            match state.db.update_package(pkg) {
                Ok(_) => StatusCode::OK,
                Err(e) => {
                    tracing::error!("Failed to update package timestamp: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            }
        }
        Ok(None) => StatusCode::NOT_FOUND,
        Err(e) => {
            tracing::error!("DB error looking up package: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
