pub use fossdb::{
    AffectedPackage, AuthResponse, CreatePackageRequest, DatabaseStats, Dependency, EventType,
    LoginRequest, PackageSubscription, PackagesResponse, RegisterRequest, SubscriptionRequest,
    SubscriptionResponse, TimelineEventType, TimelineResponse, UserResponse, VulnerabilitySeverity,
    WebSocketMessage,
};

#[cfg(feature = "db")]
pub mod db;
#[cfg(feature = "db")]
pub mod id_generator;
#[cfg(feature = "db")]
pub mod models;
#[cfg(feature = "db")]
pub use models::{Package, PackageVersion, TimelineEvent, User, Vulnerability};

#[cfg(feature = "api-server")]
pub mod auth;
#[cfg(feature = "api-server")]
pub mod client;
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
}

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "email")]
pub mod notifications;

#[cfg(feature = "collector")]
pub mod collector_models;
#[cfg(feature = "collector")]
pub mod collectors;
