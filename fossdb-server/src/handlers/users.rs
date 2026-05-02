use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppState, auth::Claims, PackageSubscription};

#[derive(Debug, Deserialize)]
pub struct SubscriptionRequest {
    pub package_name: String,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub subscriptions: Vec<PackageSubscription>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePackageNotificationRequest {
    pub notifications_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct NotificationSettingsRequest {
    pub notifications_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationSettingsResponse {
    pub notifications_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

pub async fn get_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineQuery>,
    claims: Option<Extension<Claims>>,
) -> Result<Json<Value>, StatusCode> {
    // If user is logged in, return their personal timeline (paginated)
    // Otherwise, return the global timeline (generated dynamically from recent versions)
    let is_authenticated = claims.is_some();
    let mut db_events = if let Some(Extension(claims)) = claims {
        // User is logged in - get their personal timeline
        let user_id: u64 = claims.sub.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

        state
            .db
            .get_timeline_events_by_user(user_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        // No user logged in - generate global timeline dynamically from recent package versions
        use crate::{TimelineEvent, EventType};

        let mut versions = state
            .db
            .get_all_versions()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Sort by release date (most recent first)
        versions.sort_by(|a, b| b.release_date.cmp(&a.release_date));

        // Take the 50 most recent versions and convert to timeline events
        versions
            .into_iter()
            .take(50)
            .filter_map(|version| {
                // Get package name
                state.db.get_package(version.package_id).ok()?.map(|package| {
                    let version_str = version.version.clone();
                    TimelineEvent {
                        id: 0,
                        package_id: package.id,
                        user_id: None,
                        event_type: EventType::NewRelease,
                        package_name: package.name,
                        version: Some(version.version),
                        message: format!("New version {} released", version_str),
                        metadata: None,
                        created_at: version.release_date,
                        notified_at: None,
                    }
                })
            })
            .collect()
    };

    // For personal timelines, apply pagination
    let total = db_events.len();
    if is_authenticated {
        let limit = params.limit.unwrap_or(20).min(100);
        let offset = params.offset.unwrap_or(0);

        db_events = db_events.into_iter().skip(offset).take(limit).collect();

        Ok(Json(serde_json::json!({
            "events": db_events,
            "total": total,
            "limit": limit,
            "offset": offset
        })))
    } else {
        // Global timeline - no pagination metadata
        Ok(Json(serde_json::json!({
            "events": db_events
        })))
    }
}

pub async fn get_subscriptions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<SubscriptionResponse>, StatusCode> {
    let user_id: u64 = claims.sub.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let user = state
        .db
        .get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(SubscriptionResponse {
        subscriptions: user.subscriptions,
    }))
}

pub async fn add_subscription(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<SubscriptionRequest>,
) -> Result<Json<SubscriptionResponse>, StatusCode> {
    let user_id: u64 = claims.sub.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut user = state
        .db
        .get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify package exists
    if state
        .db
        .get_package_by_name(&payload.package_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_none()
    {
        return Err(StatusCode::NOT_FOUND);
    }

    // Add subscription if not already subscribed
    if !user
        .subscriptions
        .iter()
        .any(|s| s.package_name == payload.package_name)
    {
        user.subscriptions.push(PackageSubscription {
            package_name: payload.package_name,
            notifications_enabled: true, // Default to enabled
        });
        state
            .db
            .update_user(user.clone())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(SubscriptionResponse {
        subscriptions: user.subscriptions,
    }))
}

pub async fn remove_subscription(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(package_name): Path<String>,
) -> Result<Json<SubscriptionResponse>, StatusCode> {
    let user_id: u64 = claims.sub.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut user = state
        .db
        .get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    user.subscriptions
        .retain(|s| s.package_name != package_name);

    state
        .db
        .update_user(user.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SubscriptionResponse {
        subscriptions: user.subscriptions,
    }))
}

pub async fn get_notification_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<NotificationSettingsResponse>, StatusCode> {
    let user_id: u64 = claims.sub.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let user = state
        .db
        .get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(NotificationSettingsResponse {
        notifications_enabled: user.notifications_enabled,
    }))
}

pub async fn update_notification_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<NotificationSettingsRequest>,
) -> Result<Json<NotificationSettingsResponse>, StatusCode> {
    let user_id: u64 = claims.sub.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut user = state
        .db
        .get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    user.notifications_enabled = payload.notifications_enabled;

    state
        .db
        .update_user(user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(NotificationSettingsResponse {
        notifications_enabled: payload.notifications_enabled,
    }))
}

pub async fn update_package_notification(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(package_name): Path<String>,
    Json(payload): Json<UpdatePackageNotificationRequest>,
) -> Result<Json<SubscriptionResponse>, StatusCode> {
    let user_id: u64 = claims.sub.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut user = state
        .db
        .get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Find and update the subscription
    if let Some(subscription) = user
        .subscriptions
        .iter_mut()
        .find(|s| s.package_name == package_name)
    {
        subscription.notifications_enabled = payload.notifications_enabled;

        state
            .db
            .update_user(user.clone())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(SubscriptionResponse {
            subscriptions: user.subscriptions,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
