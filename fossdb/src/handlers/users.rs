use axum::{
    extract::{State, Extension, Path},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppState, auth::Claims};

#[derive(Debug, Deserialize)]
pub struct SubscriptionRequest {
    pub package_name: String,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub subscriptions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct NotificationSettingsRequest {
    pub notifications_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationSettingsResponse {
    pub notifications_enabled: bool,
}

pub async fn get_timeline(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    match state.db.get_all_timeline_events() {
        Ok(events) => {
            Ok(Json(serde_json::json!({
                "events": events
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_subscriptions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<SubscriptionResponse>, StatusCode> {
    let user_id: u64 = claims.sub.parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let user = state.db.get_user(user_id)
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
    let user_id: u64 = claims.sub.parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut user = state.db.get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify package exists
    if state.db.get_package_by_name(&payload.package_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_none()
    {
        return Err(StatusCode::NOT_FOUND);
    }

    // Add subscription if not already subscribed
    if !user.subscriptions.contains(&payload.package_name) {
        user.subscriptions.push(payload.package_name);
        state.db.update_user(user.clone())
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
    let user_id: u64 = claims.sub.parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut user = state.db.get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    user.subscriptions.retain(|s| s != &package_name);

    state.db.update_user(user.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SubscriptionResponse {
        subscriptions: user.subscriptions,
    }))
}

pub async fn get_notification_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<NotificationSettingsResponse>, StatusCode> {
    let user_id: u64 = claims.sub.parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let user = state.db.get_user(user_id)
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
    let user_id: u64 = claims.sub.parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut user = state.db.get_user(user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    user.notifications_enabled = payload.notifications_enabled;

    state.db.update_user(user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(NotificationSettingsResponse {
        notifications_enabled: payload.notifications_enabled,
    }))
}