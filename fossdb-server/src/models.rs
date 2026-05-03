use native_db::*;
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[native_model(id = 1, version = 1)]
#[native_db(
    primary_key(id -> u64),
    secondary_key(name -> String, unique),
)]
pub struct Package {
    pub inner: fossdb::Package,
}

impl Package {
    fn id(&self) -> u64 {
        self.inner.id
    }
    fn name(&self) -> String {
        self.inner.name.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[native_model(id = 2, version = 1)]
#[native_db(
    primary_key(id -> u64),
    secondary_key(package_id -> u64),
)]
pub struct PackageVersion {
    pub inner: fossdb::PackageVersion,
}

impl PackageVersion {
    fn id(&self) -> u64 {
        self.inner.id
    }
    fn package_id(&self) -> u64 {
        self.inner.package_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[native_model(id = 3, version = 1)]
#[native_db(
    primary_key(id -> u64),
    secondary_key(email -> String, unique),
    secondary_key(username -> String, unique),
)]
pub struct User {
    pub inner: fossdb::User,
}

impl User {
    fn id(&self) -> u64 {
        self.inner.id
    }
    fn email(&self) -> String {
        self.inner.email.clone()
    }
    fn username(&self) -> String {
        self.inner.username.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[native_model(id = 4, version = 1)]
#[native_db(
    primary_key(id -> u64),
)]
pub struct Vulnerability {
    pub inner: fossdb::Vulnerability,
}

impl Vulnerability {
    fn id(&self) -> u64 {
        self.inner.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[native_model(id = 5, version = 1)]
#[native_db(
    primary_key(id -> u64),
    secondary_key(package_id -> u64),
    secondary_key(user_id -> Option<u64>, optional),
)]
pub struct TimelineEvent {
    pub inner: fossdb::TimelineEvent,
}

impl TimelineEvent {
    fn id(&self) -> u64 {
        self.inner.id
    }
    fn package_id(&self) -> u64 {
        self.inner.package_id
    }
    fn user_id(&self) -> Option<u64> {
        self.inner.user_id
    }
}
