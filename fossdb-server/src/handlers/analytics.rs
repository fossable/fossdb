use crate::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct DatabaseStats {
    pub total_packages: u64,
    pub total_versions: u64,
    pub total_users: u64,
    pub total_vulnerabilities: u64,
    pub total_timeline_events: u64,
    pub collectors_running: Vec<String>,
}

#[derive(Serialize)]
pub struct AnalyticsResponse {
    pub total_packages: u64,
    pub programming_languages: u64,
    pub weekly_updates: u64,
    pub language_distribution: Vec<LanguageStats>,
    pub license_distribution: Vec<LicenseStats>,
    pub trending_packages: Vec<TrendingPackage>,
    pub security_overview: SecurityStats,
    pub growth_data: Vec<GrowthPoint>,
}

#[derive(Serialize)]
pub struct LanguageStats {
    pub language: String,
    pub percentage: f32,
    pub count: u64,
}

#[derive(Serialize)]
pub struct LicenseStats {
    pub license: String,
    pub percentage: f32,
    pub count: u64,
}

#[derive(Serialize)]
pub struct TrendingPackage {
    pub name: String,
    pub description: String,
    pub growth_percentage: f32,
    pub category: String,
}

#[derive(Serialize)]
pub struct SecurityStats {
    pub clean_packages: u64,
    pub minor_issues: u64,
    pub critical_vulnerabilities: u64,
    pub scan_coverage: f32,
}

#[derive(Serialize)]
pub struct GrowthPoint {
    pub date: String,
    pub packages_added: u64,
    pub cumulative_total: u64,
}

pub async fn get_analytics(
    State(state): State<AppState>,
) -> Result<Json<AnalyticsResponse>, StatusCode> {
    // Fetch real data from database
    let packages = state
        .db
        .get_all_packages()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let vulnerabilities = state
        .db
        .get_all_vulnerabilities()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total = packages.len() as u64;

    // Calculate language distribution from actual packages
    let mut language_counts = std::collections::HashMap::new();
    let mut license_counts = std::collections::HashMap::new();

    for pkg in &packages {
        if let Some(lang) = &pkg.language {
            *language_counts.entry(lang.clone()).or_insert(0) += 1;
        }
        if let Some(license) = &pkg.license {
            *license_counts.entry(license.clone()).or_insert(0) += 1;
        }
    }

    // Build language distribution
    let mut language_distribution: Vec<LanguageStats> = language_counts
        .into_iter()
        .map(|(lang, count)| LanguageStats {
            language: lang,
            percentage: if total > 0 {
                (count as f32 / total as f32) * 100.0
            } else {
                0.0
            },
            count,
        })
        .collect();
    language_distribution.sort_by(|a, b| b.count.cmp(&a.count));

    // Build license distribution
    let mut license_distribution: Vec<LicenseStats> = license_counts
        .into_iter()
        .map(|(license, count)| LicenseStats {
            license,
            percentage: if total > 0 {
                (count as f32 / total as f32) * 100.0
            } else {
                0.0
            },
            count,
        })
        .collect();
    license_distribution.sort_by(|a, b| b.count.cmp(&a.count));

    // Calculate security stats from real vulnerabilities
    let critical_vulns = vulnerabilities
        .iter()
        .filter(|v| matches!(v.severity, crate::VulnerabilitySeverity::Critical))
        .count() as u64;
    let minor_issues = vulnerabilities
        .iter()
        .filter(|v| {
            matches!(
                v.severity,
                crate::VulnerabilitySeverity::Low
                    | crate::VulnerabilitySeverity::Medium
            )
        })
        .count() as u64;

    let security_overview = SecurityStats {
        clean_packages: total.saturating_sub(vulnerabilities.len() as u64),
        minor_issues,
        critical_vulnerabilities: critical_vulns,
        scan_coverage: if total > 0 { 100.0 } else { 0.0 },
    };

    // Trending packages - just get most recent packages for now
    let trending_packages: Vec<TrendingPackage> = packages
        .iter()
        .rev()
        .take(3)
        .map(|pkg| TrendingPackage {
            name: pkg.name.clone(),
            description: pkg.description.clone().unwrap_or_default(),
            growth_percentage: 0.0, // No historical data yet
            category: pkg.platform.clone().unwrap_or_else(|| "other".to_string()),
        })
        .collect();

    let analytics = AnalyticsResponse {
        total_packages: total,
        programming_languages: language_distribution.len() as u64,
        weekly_updates: 0, // Would need historical tracking
        language_distribution,
        license_distribution,
        trending_packages,
        security_overview,
        growth_data: vec![], // Would need historical tracking
    };

    Ok(Json(analytics))
}

pub async fn get_language_trends(
    State(state): State<AppState>,
) -> Result<Json<Vec<LanguageStats>>, StatusCode> {
    let packages = state
        .db
        .get_all_packages()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total = packages.len() as u64;
    let mut language_counts = std::collections::HashMap::new();

    for pkg in &packages {
        if let Some(lang) = &pkg.language {
            *language_counts.entry(lang.clone()).or_insert(0) += 1;
        }
    }

    let mut trends: Vec<LanguageStats> = language_counts
        .into_iter()
        .map(|(lang, count)| LanguageStats {
            language: lang,
            percentage: if total > 0 {
                (count as f32 / total as f32) * 100.0
            } else {
                0.0
            },
            count,
        })
        .collect();
    trends.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(Json(trends))
}

pub async fn get_security_report(
    State(state): State<AppState>,
) -> Result<Json<SecurityStats>, StatusCode> {
    let packages = state
        .db
        .get_all_packages()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let vulnerabilities = state
        .db
        .get_all_vulnerabilities()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total = packages.len() as u64;
    let critical_vulns = vulnerabilities
        .iter()
        .filter(|v| matches!(v.severity, crate::VulnerabilitySeverity::Critical))
        .count() as u64;
    let minor_issues = vulnerabilities
        .iter()
        .filter(|v| {
            matches!(
                v.severity,
                crate::VulnerabilitySeverity::Low
                    | crate::VulnerabilitySeverity::Medium
            )
        })
        .count() as u64;

    let security_stats = SecurityStats {
        clean_packages: total.saturating_sub(vulnerabilities.len() as u64),
        minor_issues,
        critical_vulnerabilities: critical_vulns,
        scan_coverage: if total > 0 { 100.0 } else { 0.0 },
    };

    Ok(Json(security_stats))
}

pub async fn get_db_stats(
    State(state): State<AppState>,
) -> Result<Json<DatabaseStats>, StatusCode> {
    let packages = state
        .db
        .get_all_packages()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let versions = state
        .db
        .get_all_versions()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let users = state
        .db
        .get_all_users()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let vulnerabilities = state
        .db
        .get_all_vulnerabilities()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let timeline_events = state
        .db
        .get_all_timeline_events()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats = DatabaseStats {
        total_packages: packages.len() as u64,
        total_versions: versions.len() as u64,
        total_users: users.len() as u64,
        total_vulnerabilities: vulnerabilities.len() as u64,
        total_timeline_events: timeline_events.len() as u64,
        collectors_running: vec!["crates.io".to_string()],
    };

    Ok(Json(stats))
}
