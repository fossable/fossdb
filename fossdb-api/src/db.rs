use couch_rs::{Client, database::Database as CouchDatabase};
use anyhow::Result;
use serde_json::Value;

pub struct Database {
    client: Client,
    packages_db: CouchDatabase,
    versions_db: CouchDatabase,
    users_db: CouchDatabase,
    vulnerabilities_db: CouchDatabase,
    timeline_db: CouchDatabase,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let config = crate::config::Config::from_env();
        let couchdb_url = config.couchdb_url;
        let couchdb_username = config.couchdb_username;
        let couchdb_password = config.couchdb_password;
        
        let client = Client::new(&couchdb_url, &couchdb_username, &couchdb_password)?;
        
        let packages_db = client.db("packages").await?;
        let versions_db = client.db("versions").await?;
        let users_db = client.db("users").await?;
        let vulnerabilities_db = client.db("vulnerabilities").await?;
        let timeline_db = client.db("timeline").await?;

        Ok(Self {
            client,
            packages_db,
            versions_db,
            users_db,
            vulnerabilities_db,
            timeline_db,
        })
    }

    pub fn packages(&self) -> &CouchDatabase {
        &self.packages_db
    }

    pub fn versions(&self) -> &CouchDatabase {
        &self.versions_db
    }

    pub fn users(&self) -> &CouchDatabase {
        &self.users_db
    }

    pub fn vulnerabilities(&self) -> &CouchDatabase {
        &self.vulnerabilities_db
    }

    pub fn timeline(&self) -> &CouchDatabase {
        &self.timeline_db
    }
}