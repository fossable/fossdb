use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Form,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::*, models::*, AppState};

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    register_user(state, payload.username, payload.email, payload.password).await
}

pub async fn register_form(
    State(state): State<AppState>,
    Form(payload): Form<RegisterForm>,
) -> Result<Json<AuthResponse>, StatusCode> {
    register_user(state, payload.username, payload.email, payload.password).await
}

async fn register_user(
    state: AppState,
    username: String,
    email: String,
    password: String,
) -> Result<Json<AuthResponse>, StatusCode> {
    let users_db = state.db.users();
    
    let password_hash = hash_password(&password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = User {
        id: Uuid::new_v4().to_string(),
        rev: None,
        username: username.clone(),
        email,
        password_hash,
        subscriptions: Vec::new(),
        created_at: Utc::now(),
        is_verified: false,
    };

    let mut user_value = serde_json::to_value(&user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    users_db.save(&mut user_value).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let token = create_jwt(&user.id, &user.username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    login_user(state, payload.email, payload.password).await
}

pub async fn login_form(
    State(state): State<AppState>,
    Form(payload): Form<LoginForm>,
) -> Result<Json<AuthResponse>, StatusCode> {
    login_user(state, payload.email, payload.password).await
}

async fn login_user(
    state: AppState,
    email: String,
    password: String,
) -> Result<Json<AuthResponse>, StatusCode> {
    let users_db = state.db.users();
    
    // Find user by email - in a real implementation, you'd want an index
    let all_users = users_db.get_all().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let user = all_users
        .get_data()
        .iter()
        .find_map(|doc: &serde_json::Value| {
            let user: Result<User, _> = serde_json::from_value(doc.clone());
            match user {
                Ok(u) if u.email == email => Some(u),
                _ => None,
            }
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let is_valid = verify_password(&password, &user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = create_jwt(&user.id, &user.username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
    }))
}