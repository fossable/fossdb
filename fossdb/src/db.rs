use anyhow::Result;
use native_db::*;
use once_cell::sync::Lazy;
use std::sync::Arc;

use crate::id_generator::IdGenerator;
use crate::models::*;

// Macro for generating insert methods
macro_rules! impl_insert {
    ($method:ident, $type:ty, $id_gen:ident) => {
        pub fn $method(&self, mut entity: $type) -> Result<$type> {
            if entity.id == 0 {
                entity.id = self.$id_gen.next();
            }
            let rw = self.db.rw_transaction()?;
            rw.insert(entity.clone())?;
            rw.commit()?;
            Ok(entity)
        }
    };
    (#[allow(dead_code)] $method:ident, $type:ty, $id_gen:ident) => {
        #[allow(dead_code)]
        pub fn $method(&self, mut entity: $type) -> Result<$type> {
            if entity.id == 0 {
                entity.id = self.$id_gen.next();
            }
            let rw = self.db.rw_transaction()?;
            rw.insert(entity.clone())?;
            rw.commit()?;
            Ok(entity)
        }
    };
}

// Macro for generating get by ID methods
macro_rules! impl_get {
    ($method:ident, $type:ty) => {
        pub fn $method(&self, id: u64) -> Result<Option<$type>> {
            let r = self.db.r_transaction()?;
            Ok(r.get().primary(id)?)
        }
    };
    (#[allow(dead_code)] $method:ident, $type:ty) => {
        #[allow(dead_code)]
        pub fn $method(&self, id: u64) -> Result<Option<$type>> {
            let r = self.db.r_transaction()?;
            Ok(r.get().primary(id)?)
        }
    };
}

// Macro for generating get all methods
macro_rules! impl_get_all {
    ($method:ident, $type:ty) => {
        pub fn $method(&self) -> Result<Vec<$type>> {
            let r = self.db.r_transaction()?;
            let all: Vec<$type> = r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
            Ok(all)
        }
    };
}

// Macro for generating update methods
macro_rules! impl_update {
    ($method:ident, $type:ty) => {
        pub fn $method(&self, entity: $type) -> Result<()> {
            let rw = self.db.rw_transaction()?;
            if let Some(old) = rw.get().primary::<$type>(entity.id)? {
                rw.remove(old)?;
            }
            rw.insert(entity)?;
            rw.commit()?;
            Ok(())
        }
    };
}

// Macro for finding max ID
macro_rules! find_max_id {
    ($tx:expr, $type:ty) => {
        $tx.scan()
            .primary::<$type>()?
            .all()?
            .map(|e| e.map(|item| item.id).unwrap_or(0))
            .max()
            .unwrap_or(0)
    };
}

static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<Package>().unwrap();
    models.define::<PackageVersion>().unwrap();
    models.define::<User>().unwrap();
    models.define::<Vulnerability>().unwrap();
    models.define::<TimelineEvent>().unwrap();
    models
});

pub struct Database {
    pub db: native_db::Database<'static>,
    package_ids: Arc<IdGenerator>,
    version_ids: Arc<IdGenerator>,
    user_ids: Arc<IdGenerator>,
    #[allow(dead_code)]
    vulnerability_ids: Arc<IdGenerator>,
    timeline_ids: Arc<IdGenerator>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        // Open or create database using static MODELS
        let db = Builder::new().create(&MODELS, path)?;

        // Scan database to find highest IDs and initialize generators
        let r = db.r_transaction()?;

        let max_package_id = find_max_id!(r, Package);
        let max_version_id = find_max_id!(r, PackageVersion);
        let max_user_id = find_max_id!(r, User);
        let max_vulnerability_id = find_max_id!(r, Vulnerability);
        let max_timeline_id = find_max_id!(r, TimelineEvent);

        drop(r);

        // Initialize ID generators starting from max_id + 1
        let package_ids = Arc::new(IdGenerator::new(max_package_id + 1));
        let version_ids = Arc::new(IdGenerator::new(max_version_id + 1));
        let user_ids = Arc::new(IdGenerator::new(max_user_id + 1));
        let vulnerability_ids = Arc::new(IdGenerator::new(max_vulnerability_id + 1));
        let timeline_ids = Arc::new(IdGenerator::new(max_timeline_id + 1));

        Ok(Self {
            db,
            package_ids,
            version_ids,
            user_ids,
            vulnerability_ids,
            timeline_ids,
        })
    }

    // Package operations
    impl_insert!(insert_package, Package, package_ids);
    impl_get!(get_package, Package);

    pub fn get_package_by_name(&self, name: &str) -> Result<Option<Package>> {
        let r = self.db.r_transaction()?;
        let results: Vec<Package> = r
            .scan()
            .secondary(PackageKey::name)?
            .start_with(name)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results.into_iter().next())
    }

    impl_get_all!(get_all_packages, Package);
    impl_update!(update_package, Package);

    // PackageVersion operations
    impl_insert!(insert_version, PackageVersion, version_ids);
    impl_get!(
        #[allow(dead_code)]
        get_version,
        PackageVersion
    );

    pub fn get_versions_by_package(&self, package_id: u64) -> Result<Vec<PackageVersion>> {
        let r = self.db.r_transaction()?;
        let versions: Vec<PackageVersion> = r
            .scan()
            .secondary(PackageVersionKey::package_id)?
            .start_with(package_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(versions)
    }

    impl_get_all!(get_all_versions, PackageVersion);

    // User operations
    impl_insert!(insert_user, User, user_ids);
    impl_get!(get_user, User);

    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let r = self.db.r_transaction()?;
        let results: Vec<User> = r
            .scan()
            .secondary(UserKey::email)?
            .start_with(email)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results.into_iter().next())
    }

    #[allow(dead_code)]
    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let r = self.db.r_transaction()?;
        let results: Vec<User> = r
            .scan()
            .secondary(UserKey::username)?
            .start_with(username)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results.into_iter().next())
    }

    impl_get_all!(get_all_users, User);
    impl_update!(update_user, User);

    // Vulnerability operations
    impl_insert!(
        #[allow(dead_code)]
        insert_vulnerability,
        Vulnerability,
        vulnerability_ids
    );
    impl_get!(
        #[allow(dead_code)]
        get_vulnerability,
        Vulnerability
    );
    impl_get_all!(get_all_vulnerabilities, Vulnerability);

    // TimelineEvent operations
    impl_insert!(insert_timeline_event, TimelineEvent, timeline_ids);
    impl_get!(
        #[allow(dead_code)]
        get_timeline_event,
        TimelineEvent
    );
    impl_get_all!(get_all_timeline_events, TimelineEvent);

    #[allow(dead_code)]
    pub fn get_timeline_by_package(&self, package_id: u64) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let events: Vec<TimelineEvent> = r
            .scan()
            .secondary(TimelineEventKey::package_id)?
            .start_with(package_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    impl_update!(update_timeline_event, TimelineEvent);

    pub fn get_timeline_events_by_user(&self, user_id: u64) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let events: Vec<TimelineEvent> = r
            .scan()
            .secondary(TimelineEventKey::user_id)?
            .start_with(Some(user_id))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    pub fn get_pending_notifications(&self) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let all_events: Vec<TimelineEvent> =
            r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
        // Filter in memory - native_db doesn't support complex queries
        // For better scalability, consider adding a compound index or separate pending_notifications table
        Ok(all_events
            .into_iter()
            .filter(|e| {
                e.user_id.is_some()
                    && e.notified_at.is_none()
                    && e.event_type == crate::models::EventType::NewRelease
            })
            .collect())
    }

    pub fn get_users_subscribed_to(&self, package_name: &str) -> Result<Vec<u64>> {
        let all_users = self.get_all_users()?;
        Ok(all_users
            .into_iter()
            .filter(|u| {
                u.subscriptions
                    .iter()
                    .any(|s| s.package_name == package_name && s.notifications_enabled)
            })
            .map(|u| u.id)
            .collect())
    }
}
