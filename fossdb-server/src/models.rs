use native_db::*;
use native_model::{Model, native_model};
use serde::{Deserialize, Serialize};

macro_rules! impl_deref {
    ($wrapper:ty, $inner:ty) => {
        impl std::ops::Deref for $wrapper {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }
        impl std::ops::DerefMut for $wrapper {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }
    };
}

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
        self.id
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

impl_deref!(Package, fossdb::Package);

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
        self.id
    }
    fn package_id(&self) -> u64 {
        self.package_id
    }
}

impl_deref!(PackageVersion, fossdb::PackageVersion);

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
        self.id
    }
    fn email(&self) -> String {
        self.email.clone()
    }
    fn username(&self) -> String {
        self.username.clone()
    }
}

impl_deref!(User, fossdb::User);

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
        self.id
    }
}

impl_deref!(Vulnerability, fossdb::Vulnerability);

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
        self.id
    }
    fn package_id(&self) -> u64 {
        self.package_id
    }
    fn user_id(&self) -> Option<u64> {
        self.user_id
    }
}

impl_deref!(TimelineEvent, fossdb::TimelineEvent);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[native_model(id = 6, version = 1)]
#[native_db(
    primary_key(id -> u64),
    secondary_key(package_id -> u64),
    secondary_key(version_id -> u64),
)]
pub struct WorkerAnalysis {
    pub inner: fossdb::WorkerAnalysis,
}

impl WorkerAnalysis {
    fn id(&self) -> u64 {
        self.id
    }
    fn package_id(&self) -> u64 {
        self.package_id
    }
    fn version_id(&self) -> u64 {
        self.version_id
    }
}

impl_deref!(WorkerAnalysis, fossdb::WorkerAnalysis);
