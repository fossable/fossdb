// Core model types and macros
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "db")]
use native_db::*;
#[cfg(feature = "db")]
use native_model::{native_model, Model};

// Macro to conditionally apply native_db attributes based on the "db" feature
#[cfg(feature = "db")]
macro_rules! db_model {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $(
                $(#[$field_meta])*
                $field_vis $field_name: $field_type
            ),*
        }
    };
}

#[cfg(not(feature = "db"))]
macro_rules! db_model {
    (
        // Match and filter out native_model attribute
        #[derive($($derive:ident),*)]
        #[native_model(id = $id:expr, version = $version:expr)]
        #[native_db]
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        // Emit struct without native_db attributes
        #[derive($($derive),*)]
        $vis struct $name {
            $(
                $field_vis $field_name: $field_type
            ),*
        }
    };
}

db_model! {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[native_model(id = 1, version = 1)]
    #[native_db]
    pub struct Package {
        #[primary_key]
        pub id: u64,
        #[secondary_key(unique)]
        pub name: String,
        pub description: Option<String>,
        pub homepage: Option<String>,
        pub repository: Option<String>,
        pub license: Option<String>,
        pub tags: Vec<String>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub platform: Option<String>,
        pub language: Option<String>,
        pub status: Option<String>,
        pub dependents_count: Option<u32>,
        pub rank: Option<u32>,
    }
}

db_model! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[native_model(id = 2, version = 1)]
    #[native_db]
    pub struct PackageVersion {
        #[primary_key]
        pub id: u64,
        #[secondary_key]
        pub package_id: u64,
        pub version: String,
        pub release_date: DateTime<Utc>,
        pub download_url: Option<String>,
        pub checksum: Option<String>,
        pub dependencies: Vec<Dependency>,
        pub vulnerabilities: Vec<String>,
        pub changelog: Option<String>,
        pub created_at: DateTime<Utc>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_requirement: String,
    pub dependency_type: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageSubscription {
    pub package_name: String,
    pub notifications_enabled: bool,
}

db_model! {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[native_model(id = 3, version = 1)]
    #[native_db]
    pub struct User {
        #[primary_key]
        pub id: u64,
        #[secondary_key(unique)]
        pub email: String,
        #[secondary_key(unique)]
        pub username: String,
        pub password_hash: String,
        pub subscriptions: Vec<PackageSubscription>,
        pub created_at: DateTime<Utc>,
        pub is_verified: bool,
        pub notifications_enabled: bool,
    }
}

db_model! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[native_model(id = 4, version = 1)]
    #[native_db]
    pub struct Vulnerability {
        #[primary_key]
        pub id: u64,
        pub cve_id: Option<String>,
        pub title: String,
        pub description: String,
        pub severity: VulnerabilitySeverity,
        pub affected_packages: Vec<AffectedPackage>,
        pub discovered_at: DateTime<Utc>,
        pub fixed_in: Option<String>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VulnerabilitySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedPackage {
    pub package_id: u64,
    pub version_range: String,
}

db_model! {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[native_model(id = 5, version = 1)]
    #[native_db]
    pub struct TimelineEvent {
        #[primary_key]
        pub id: u64,
        #[secondary_key]
        pub package_id: u64,
        #[secondary_key]
        pub user_id: Option<u64>,
        pub event_type: EventType,
        pub package_name: String,
        pub version: Option<String>,
        pub message: String,
        pub metadata: Option<String>,
        pub created_at: DateTime<Utc>,
        pub notified_at: Option<DateTime<Utc>>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    NewRelease,
    SecurityAlert,
    PackageAdded,
    PackageUpdated,
}

// Alias for API compatibility
pub type TimelineEventType = EventType;

#[derive(Debug, Deserialize)]
pub struct CreatePackageRequest {
    pub name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_packages: u64,
    pub total_versions: u64,
    pub total_users: u64,
    pub total_vulnerabilities: u64,
    pub total_timeline_events: u64,
    pub collectors_running: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub package_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    pub package_name: String,
    pub notifications_enabled: bool,
    pub package: Option<Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineResponse {
    pub events: Vec<TimelineEvent>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserResponse {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagesResponse {
    pub packages: Vec<Package>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    Auth { token: String },
    Ping,
    Pong,
    TimelineEvent { event: TimelineEvent },
}

// Conditionally compile modules based on features
#[cfg(feature = "api-server")]
pub mod auth;
#[cfg(feature = "api-server")]
pub mod client;
#[cfg(feature = "api-server")]
pub mod config;
#[cfg(feature = "api-server")]
pub mod db;
#[cfg(feature = "api-server")]
pub mod db_listener;
#[cfg(feature = "api-server")]
pub mod handlers;
#[cfg(feature = "api-server")]
pub mod id_generator;
#[cfg(feature = "api-server")]
pub mod middleware;
#[cfg(feature = "api-server")]
pub mod websocket;

// Application state for API server
#[cfg(feature = "api-server")]
#[derive(Clone)]
pub struct AppState {
    pub db: std::sync::Arc<db::Database>,
    pub broadcaster: std::sync::Arc<websocket::TimelineBroadcaster>,
}

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "email")]
pub mod notifications;

#[cfg(feature = "collector")]
pub mod collector_models;
#[cfg(feature = "collector")]
pub mod collectors;
