use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Package {
    pub id: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub id: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: u64,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub subscriptions: Vec<PackageSubscription>,
    pub created_at: DateTime<Utc>,
    pub is_verified: bool,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: u64,
    pub cve_id: Option<String>,
    pub title: String,
    pub description: String,
    pub severity: VulnerabilitySeverity,
    pub affected_packages: Vec<AffectedPackage>,
    pub discovered_at: DateTime<Utc>,
    pub fixed_in: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineEvent {
    pub id: u64,
    pub package_id: u64,
    pub user_id: Option<u64>,
    pub event_type: EventType,
    pub package_name: String,
    pub version: Option<String>,
    pub message: String,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
    pub notified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    NewRelease,
    SecurityAlert,
    PackageAdded,
    PackageUpdated,
}

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
pub struct CollectedPackage {
    pub name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub versions: Vec<CollectedVersion>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
    pub dependents_count: Option<u32>,
    pub rank: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedVersion {
    pub version: String,
    pub release_date: DateTime<Utc>,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageLookup {
    pub package: Package,
    pub version_strings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTask {
    pub package: Package,
    pub version: PackageVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkerAnalysis {
    pub id: u64,
    pub package_id: u64,
    pub version_id: u64,
    pub analyzed_at: DateTime<Utc>,
    pub findings: Vec<AnalysisFinding>,
    pub checksum_verified: Option<bool>,
    pub detected_license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisFinding {
    pub kind: FindingKind,
    pub severity: FindingSeverity,
    pub description: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FindingKind {
    LicenseDetected,
    LicenseMismatch,
    SecretDetected,
    ChecksumMismatch,
    BinaryMetadata,
    MaliciousPattern,
    SyscallReachable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    Auth { token: String },
    Ping,
    Pong,
    TimelineEvent { event: TimelineEvent },
}
