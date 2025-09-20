use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde_json::Value;

use crate::{models::*, AppState};

pub async fn get_timeline(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let timeline_db = state.db.timeline();
    
    match timeline_db.get_all().await {
        Ok(events) => {
            let events: Vec<TimelineEvent> = events
                .get_data()
                .iter()
                .filter_map(|doc: &Value| serde_json::from_value(doc.clone()).ok())
                .collect();
            
            Ok(Json(serde_json::json!({
                "events": events
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_subscriptions(
    State(_state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    // In a real app, you'd get the user ID from JWT token
    Ok(Json(serde_json::json!({
        "subscriptions": []
    })))
}

pub async fn add_subscription(
    State(_state): State<AppState>,
    Json(_payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // In a real app, you'd validate the JWT and update user subscriptions
    Ok(Json(serde_json::json!({
        "success": true
    })))
}