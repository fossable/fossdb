use gloo_storage::{LocalStorage as GlooLocalStorage, Storage};
use serde::{Deserialize, Serialize};

pub enum StorageKey {
    AuthToken,
    UserData,
    Subscriptions,
    ViewMode,
}

impl StorageKey {
    fn as_str(&self) -> &str {
        match self {
            StorageKey::AuthToken => "auth_token",
            StorageKey::UserData => "user_data",
            StorageKey::Subscriptions => "subscriptions",
            StorageKey::ViewMode => "view_mode",
        }
    }
}

pub struct LocalStorage;

impl LocalStorage {
    pub fn get<T: for<'de> Deserialize<'de>>(key: StorageKey) -> Option<T> {
        GlooLocalStorage::get(key.as_str()).ok()
    }

    pub fn set<T: Serialize>(
        key: StorageKey,
        value: &T,
    ) -> Result<(), gloo_storage::errors::StorageError> {
        GlooLocalStorage::set(key.as_str(), value)
    }

    pub fn remove(key: StorageKey) {
        GlooLocalStorage::delete(key.as_str());
    }

    pub fn clear() {
        GlooLocalStorage::clear();
    }
}
