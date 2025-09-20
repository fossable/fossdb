use couch_rs::{Client, database::Database as CouchDatabase};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
        let couchdb_url = std::env::var("COUCHDB_URL").unwrap_or_else(|_| "http://localhost:5984".to_string());
        let couchdb_username = std::env::var("COUCHDB_USERNAME").unwrap_or_else(|_| "admin".to_string());
        let couchdb_password = std::env::var("COUCHDB_PASSWORD").unwrap_or_else(|_| "password".to_string());
        
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub maintainers: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitted_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub package_id: String,
    pub version: String,
    pub release_date: DateTime<Utc>,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub vulnerabilities: Vec<String>,
    pub changelog: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_constraint: String,
    pub optional: bool,
}