pub use fossdb::{
    AffectedPackage, AnalysisFinding, AuthResponse, CreatePackageRequest, DatabaseStats,
    Dependency, EventType, FindingKind, FindingSeverity, LoginRequest, PackageSubscription,
    PackagesResponse, RegisterRequest, SubscriptionRequest, SubscriptionResponse,
    TimelineEventType, TimelineResponse, UserResponse, VulnerabilitySeverity, WebSocketMessage,
    WorkerTask,
};

#[cfg(feature = "db")]
pub mod db;
#[cfg(feature = "db")]
pub mod id_generator;
#[cfg(feature = "db")]
pub mod models;
#[cfg(feature = "db")]
pub use models::{Package, PackageVersion, TimelineEvent, User, Vulnerability, WorkerAnalysis};

#[cfg(feature = "api-server")]
pub mod auth;
#[cfg(feature = "api-server")]
pub mod config;
#[cfg(feature = "api-server")]
pub mod db_listener;
#[cfg(feature = "api-server")]
pub mod handlers;
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
    pub collector_api_key: String,
    pub worker_api_key: String,
    /// Version IDs currently claimed by a worker, preventing double-assignment.
    pub claimed_versions: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<u64>>>,
}

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "email")]
pub mod notifications;

