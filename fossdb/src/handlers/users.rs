use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppState, auth::Claims, models::PackageSubscription};

/// Convert database TimelineEvent to API TimelineEvent
fn convert_timeline_event(db_event: &crate::models::TimelineEvent) -> crate::TimelineEvent {
    use crate::TimelineEventType;
    use crate::models::EventType;

    let event_type = match db_event.event_type {
        EventType::NewRelease => TimelineEventType::NewRelease,
        EventType::SecurityAlert => TimelineEventType::SecurityAlert,
        EventType::PackageAdded => TimelineEventType::PackageAdded,
        EventType::PackageUpdated => TimelineEventType::PackageUpdated,
    };

    crate::TimelineEvent {
        id: db_event.id,
        event_type,
        package_name: db_event.package_name.clone(),
        version: db_event.version.clone(),
        message: db_event.description.clone(),
        metadata: None,
        created_at: db_event.created_at,
    }
}

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
    // Otherwise, return the global timeline (limited to 50)
    let is_authenticated = claims.is_some();
    let mut db_events = if let Some(Extension(claims)) = claims {
        // User is logged in - get their personal timeline
        let user_id: u64 = claims.sub.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

        state
            .db
            .get_timeline_events_by_user(user_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        // No user logged in - get global timeline (events with user_id = None)
        let all_events = state
            .db
            .get_all_timeline_events()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Filter to only global events (user_id is None) and limit to 50
        all_events
            .into_iter()
            .filter(|event| event.user_id.is_none())
            .take(50)
            .collect()
    };

    // For personal timelines, apply pagination
    let total = db_events.len();
    if is_authenticated {
        let limit = params.limit.unwrap_or(20).min(100);
        let offset = params.offset.unwrap_or(0);

        db_events = db_events.into_iter().skip(offset).take(limit).collect();

        // Convert database events to API events
        let api_events: Vec<crate::TimelineEvent> =
            db_events.iter().map(convert_timeline_event).collect();

        Ok(Json(serde_json::json!({
            "events": api_events,
            "total": total,
            "limit": limit,
            "offset": offset
        })))
    } else {
        // Convert database events to API events
        let api_events: Vec<crate::TimelineEvent> =
            db_events.iter().map(convert_timeline_event).collect();

        // Global timeline - no pagination metadata
        Ok(Json(serde_json::json!({
            "events": api_events
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
