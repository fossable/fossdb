use native_db::*;
use anyhow::Result;
use std::sync::Arc;
use once_cell::sync::Lazy;

use crate::models::*;
use crate::id_generator::IdGenerator;

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
    db: native_db::Database<'static>,
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

        let max_package_id = r.scan().primary::<Package>()?.all()?
            .map(|p| p.map(|pkg| pkg.id).unwrap_or(0))
            .max()
            .unwrap_or(0);

        let max_version_id = r.scan().primary::<PackageVersion>()?.all()?
            .map(|v| v.map(|ver| ver.id).unwrap_or(0))
            .max()
            .unwrap_or(0);

        let max_user_id = r.scan().primary::<User>()?.all()?
            .map(|u| u.map(|user| user.id).unwrap_or(0))
            .max()
            .unwrap_or(0);

        let max_vulnerability_id = r.scan().primary::<Vulnerability>()?.all()?
            .map(|v| v.map(|vuln| vuln.id).unwrap_or(0))
            .max()
            .unwrap_or(0);

        let max_timeline_id = r.scan().primary::<TimelineEvent>()?.all()?
            .map(|e| e.map(|event| event.id).unwrap_or(0))
            .max()
            .unwrap_or(0);

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
    pub fn insert_package(&self, mut package: Package) -> Result<Package> {
        if package.id == 0 {
            package.id = self.package_ids.next();
        }
        let rw = self.db.rw_transaction()?;
        rw.insert(package.clone())?;
        rw.commit()?;
        Ok(package)
    }

    pub fn get_package(&self, id: u64) -> Result<Option<Package>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id)?)
    }

    pub fn get_package_by_name(&self, name: &str) -> Result<Option<Package>> {
        let r = self.db.r_transaction()?;
        let results: Vec<Package> = r.scan().secondary(PackageKey::name)?
            .start_with(name)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results.into_iter().next())
    }

    pub fn get_all_packages(&self) -> Result<Vec<Package>> {
        let r = self.db.r_transaction()?;
        let all: Vec<Package> = r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
        Ok(all)
    }

    pub fn update_package(&self, package: Package) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        // native_db doesn't have direct update, must remove and insert
        if let Some(old) = rw.get().primary::<Package>(package.id)? {
            rw.remove(old)?;
        }
        rw.insert(package)?;
        rw.commit()?;
        Ok(())
    }

    // PackageVersion operations
    pub fn insert_version(&self, mut version: PackageVersion) -> Result<PackageVersion> {
        if version.id == 0 {
            version.id = self.version_ids.next();
        }
        let rw = self.db.rw_transaction()?;
        rw.insert(version.clone())?;
        rw.commit()?;
        Ok(version)
    }

    #[allow(dead_code)]
    pub fn get_version(&self, id: u64) -> Result<Option<PackageVersion>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id)?)
    }

    pub fn get_versions_by_package(&self, package_id: u64) -> Result<Vec<PackageVersion>> {
        let r = self.db.r_transaction()?;
        let versions: Vec<PackageVersion> = r.scan().secondary(PackageVersionKey::package_id)?
            .start_with(package_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(versions)
    }

    pub fn get_all_versions(&self) -> Result<Vec<PackageVersion>> {
        let r = self.db.r_transaction()?;
        let all: Vec<PackageVersion> = r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
        Ok(all)
    }

    // User operations
    pub fn insert_user(&self, mut user: User) -> Result<User> {
        if user.id == 0 {
            user.id = self.user_ids.next();
        }
        let rw = self.db.rw_transaction()?;
        rw.insert(user.clone())?;
        rw.commit()?;
        Ok(user)
    }

    pub fn get_user(&self, id: u64) -> Result<Option<User>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id)?)
    }

    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let r = self.db.r_transaction()?;
        let results: Vec<User> = r.scan().secondary(UserKey::email)?
            .start_with(email)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results.into_iter().next())
    }

    #[allow(dead_code)]
    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let r = self.db.r_transaction()?;
        let results: Vec<User> = r.scan().secondary(UserKey::username)?
            .start_with(username)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results.into_iter().next())
    }

    pub fn get_all_users(&self) -> Result<Vec<User>> {
        let r = self.db.r_transaction()?;
        let all: Vec<User> = r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
        Ok(all)
    }

    pub fn update_user(&self, user: User) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        if let Some(old) = rw.get().primary::<User>(user.id)? {
            rw.remove(old)?;
        }
        rw.insert(user)?;
        rw.commit()?;
        Ok(())
    }

    // Vulnerability operations
    #[allow(dead_code)]
    pub fn insert_vulnerability(&self, mut vuln: Vulnerability) -> Result<Vulnerability> {
        if vuln.id == 0 {
            vuln.id = self.vulnerability_ids.next();
        }
        let rw = self.db.rw_transaction()?;
        rw.insert(vuln.clone())?;
        rw.commit()?;
        Ok(vuln)
    }

    #[allow(dead_code)]
    pub fn get_vulnerability(&self, id: u64) -> Result<Option<Vulnerability>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id)?)
    }

    pub fn get_all_vulnerabilities(&self) -> Result<Vec<Vulnerability>> {
        let r = self.db.r_transaction()?;
        let all: Vec<Vulnerability> = r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
        Ok(all)
    }

    // TimelineEvent operations
    pub fn insert_timeline_event(&self, mut event: TimelineEvent) -> Result<TimelineEvent> {
        if event.id == 0 {
            event.id = self.timeline_ids.next();
        }
        let rw = self.db.rw_transaction()?;
        rw.insert(event.clone())?;
        rw.commit()?;
        Ok(event)
    }

    #[allow(dead_code)]
    pub fn get_timeline_event(&self, id: u64) -> Result<Option<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id)?)
    }

    pub fn get_all_timeline_events(&self) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let all: Vec<TimelineEvent> = r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
        Ok(all)
    }

    #[allow(dead_code)]
    pub fn get_timeline_by_package(&self, package_id: u64) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let events: Vec<TimelineEvent> = r.scan().secondary(TimelineEventKey::package_id)?
            .start_with(package_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    pub fn update_timeline_event(&self, event: TimelineEvent) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        if let Some(old) = rw.get().primary::<TimelineEvent>(event.id)? {
            rw.remove(old)?;
        }
        rw.insert(event)?;
        rw.commit()?;
        Ok(())
    }

    pub fn get_timeline_events_by_user(&self, user_id: u64) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let events: Vec<TimelineEvent> = r.scan().secondary(TimelineEventKey::user_id)?
            .start_with(Some(user_id))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }

    pub fn get_pending_notifications(&self) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let all_events: Vec<TimelineEvent> = r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
        // Filter in memory - native_db doesn't support complex queries
        // For better scalability, consider adding a compound index or separate pending_notifications table
        Ok(all_events
            .into_iter()
            .filter(|e| e.user_id.is_some() && e.notified_at.is_none() && e.event_type == crate::models::EventType::NewRelease)
            .collect())
    }

    pub fn get_users_subscribed_to(&self, package_name: &str) -> Result<Vec<u64>> {
        let all_users = self.get_all_users()?;
        Ok(all_users
            .into_iter()
            .filter(|u| u.subscriptions.contains(&package_name.to_string()))
            .map(|u| u.id)
            .collect())
    }
}
