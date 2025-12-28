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
    vulnerability_ids: Arc<IdGenerator>,
    timeline_ids: Arc<IdGenerator>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        // Open or create database using static MODELS
        let db = Builder::new().create(&MODELS, path)?;

        // Initialize ID generators (start at 1)
        let package_ids = Arc::new(IdGenerator::new(1));
        let version_ids = Arc::new(IdGenerator::new(1));
        let user_ids = Arc::new(IdGenerator::new(1));
        let vulnerability_ids = Arc::new(IdGenerator::new(1));
        let timeline_ids = Arc::new(IdGenerator::new(1));

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

    // Vulnerability operations
    pub fn insert_vulnerability(&self, mut vuln: Vulnerability) -> Result<Vulnerability> {
        if vuln.id == 0 {
            vuln.id = self.vulnerability_ids.next();
        }
        let rw = self.db.rw_transaction()?;
        rw.insert(vuln.clone())?;
        rw.commit()?;
        Ok(vuln)
    }

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

    pub fn get_timeline_event(&self, id: u64) -> Result<Option<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id)?)
    }

    pub fn get_all_timeline_events(&self) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let all: Vec<TimelineEvent> = r.scan().primary()?.all()?.collect::<Result<Vec<_>, _>>()?;
        Ok(all)
    }

    pub fn get_timeline_by_package(&self, package_id: u64) -> Result<Vec<TimelineEvent>> {
        let r = self.db.r_transaction()?;
        let events: Vec<TimelineEvent> = r.scan().secondary(TimelineEventKey::package_id)?
            .start_with(package_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }
}
