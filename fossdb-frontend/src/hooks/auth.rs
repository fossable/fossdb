use crate::api::{types::*, ApiClient};
use crate::hooks::storage::{LocalStorage, StorageKey};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct AuthState {
    pub token: Option<String>,
    pub user: Option<UserResponse>,
}

impl Default for AuthState {
    fn default() -> Self {
        let token: Option<String> = LocalStorage::get(StorageKey::AuthToken);
        let user: Option<UserResponse> = LocalStorage::get(StorageKey::UserData);

        Self { token, user }
    }
}

#[derive(Copy, Clone)]
pub struct AuthContext {
    auth_state: Signal<AuthState>,
}

impl AuthContext {
    pub fn is_authenticated(&self) -> bool {
        self.auth_state.read().token.is_some()
    }

    pub fn token(&self) -> Option<String> {
        self.auth_state.read().token.clone()
    }

    pub fn user(&self) -> Option<UserResponse> {
        self.auth_state.read().user.clone()
    }

    pub fn login(&mut self, token: String, user: User) {
        let user_response = UserResponse::from(user);
        let _ = LocalStorage::set(StorageKey::AuthToken, &token);
        let _ = LocalStorage::set(StorageKey::UserData, &user_response);
        self.auth_state.write().token = Some(token.clone());
        self.auth_state.write().user = Some(user_response.clone());
    }

    pub fn logout(&mut self) {
        LocalStorage::remove(StorageKey::AuthToken);
        LocalStorage::remove(StorageKey::UserData);
        LocalStorage::remove(StorageKey::Subscriptions);
        self.auth_state.write().token = None;
        self.auth_state.write().user = None;
    }
}

pub fn use_auth() -> AuthContext {
    let auth_state = use_context::<Signal<AuthState>>();

    AuthContext { auth_state }
}

pub fn use_api_client() -> ApiClient {
    let auth = use_auth();
    ApiClient::new().with_token(auth.token())
}
