use axum::{Form, extract::State, http::StatusCode, response::Json};
use chrono::Utc;
use serde::Deserialize;

use crate::{AppState, auth::*, User, RegisterRequest, LoginRequest, AuthResponse};

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
    let password_hash = hash_password(&password).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = User {
        id: 0, // Will be auto-generated
        username: username.clone(),
        email,
        password_hash,
        subscriptions: Vec::new(),
        created_at: Utc::now(),
        is_verified: false,
        notifications_enabled: true, // Enable notifications by default
    };

    let user = state
        .db
        .insert_user(user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token = create_jwt(&user.id.to_string(), &user.username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse { token, user }))
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
    // Use indexed email lookup
    let user = state
        .db
        .get_user_by_email(&email)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let is_valid = verify_password(&password, &user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = create_jwt(&user.id.to_string(), &user.username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse { token, user }))
}
