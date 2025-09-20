use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::AppState;

#[derive(Serialize)]
pub struct AnalyticsResponse {
    pub total_packages: u64,
    pub active_maintainers: u64,
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
    // In a real implementation, these would be calculated from the database
    // For now, we'll return mock data that matches our frontend
    
    let analytics = AnalyticsResponse {
        total_packages: 1_234_567,
        active_maintainers: 89_123,
        programming_languages: 156,
        weekly_updates: 45_678,
        
        language_distribution: vec![
            LanguageStats {
                language: "JavaScript".to_string(),
                percentage: 32.4,
                count: 400_000,
            },
            LanguageStats {
                language: "Python".to_string(),
                percentage: 24.1,
                count: 297_500,
            },
            LanguageStats {
                language: "Rust".to_string(),
                percentage: 18.7,
                count: 230_865,
            },
            LanguageStats {
                language: "Go".to_string(),
                percentage: 12.3,
                count: 151_852,
            },
            LanguageStats {
                language: "Java".to_string(),
                percentage: 8.9,
                count: 109_876,
            },
            LanguageStats {
                language: "Others".to_string(),
                percentage: 3.6,
                count: 44_446,
            },
        ],
        
        license_distribution: vec![
            LicenseStats {
                license: "MIT".to_string(),
                percentage: 42.3,
                count: 522_222,
            },
            LicenseStats {
                license: "Apache-2.0".to_string(),
                percentage: 28.1,
                count: 347_013,
            },
            LicenseStats {
                license: "GPL".to_string(),
                percentage: 15.7,
                count: 193_827,
            },
            LicenseStats {
                license: "Other".to_string(),
                percentage: 13.9,
                count: 171_645,
            },
        ],
        
        trending_packages: vec![
            TrendingPackage {
                name: "next-auth".to_string(),
                description: "Authentication library for Next.js".to_string(),
                growth_percentage: 247.0,
                category: "web".to_string(),
            },
            TrendingPackage {
                name: "serde".to_string(),
                description: "Rust serialization framework".to_string(),
                growth_percentage: 189.0,
                category: "rust".to_string(),
            },
            TrendingPackage {
                name: "tailwindcss".to_string(),
                description: "Utility-first CSS framework".to_string(),
                growth_percentage: 156.0,
                category: "css".to_string(),
            },
        ],
        
        security_overview: SecurityStats {
            clean_packages: 876_432,
            minor_issues: 12_345,
            critical_vulnerabilities: 1_789,
            scan_coverage: 98.7,
        },
        
        growth_data: vec![
            GrowthPoint {
                date: "2024-01".to_string(),
                packages_added: 45_000,
                cumulative_total: 1_100_000,
            },
            GrowthPoint {
                date: "2024-02".to_string(),
                packages_added: 52_000,
                cumulative_total: 1_152_000,
            },
            GrowthPoint {
                date: "2024-03".to_string(),
                packages_added: 38_000,
                cumulative_total: 1_190_000,
            },
            GrowthPoint {
                date: "2024-04".to_string(),
                packages_added: 67_000,
                cumulative_total: 1_257_000,
            },
            // Add more growth points as needed
        ],
    };
    
    Ok(Json(analytics))
}

pub async fn get_language_trends(
    State(state): State<AppState>,
) -> Result<Json<Vec<LanguageStats>>, StatusCode> {
    // This could return more detailed language trend data
    let trends = vec![
        LanguageStats {
            language: "Rust".to_string(),
            percentage: 15.2, // Growth over time
            count: 230_865,
        },
        LanguageStats {
            language: "TypeScript".to_string(),
            percentage: 12.8,
            count: 158_000,
        },
        LanguageStats {
            language: "Go".to_string(),
            percentage: 8.4,
            count: 103_675,
        },
    ];
    
    Ok(Json(trends))
}

pub async fn get_security_report(
    State(state): State<AppState>,
) -> Result<Json<SecurityStats>, StatusCode> {
    let security_stats = SecurityStats {
        clean_packages: 876_432,
        minor_issues: 12_345,
        critical_vulnerabilities: 1_789,
        scan_coverage: 98.7,
    };
    
    Ok(Json(security_stats))
}