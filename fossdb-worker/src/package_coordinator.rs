use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use anyhow::Result;

use crate::db::Database;
use crate::models::ScrapedPackage;

/// Coordinates package updates to prevent concurrent modifications
pub struct PackageCoordinator {
    /// Maps package names to semaphores (1 permit = exclusive access)
    package_locks: DashMap<String, Arc<Semaphore>>,
    database: Arc<Database>,
}

impl PackageCoordinator {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            package_locks: DashMap::new(),
            database,
        }
    }

    /// Get or create a semaphore for a package name
    fn get_package_lock(&self, package_name: &str) -> Arc<Semaphore> {
        self.package_locks
            .entry(package_name.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(1)))
            .clone()
    }

    /// Save a package with exclusive locking to prevent concurrent updates
    pub async fn save_package(&self, package_data: ScrapedPackage) -> Result<()> {
        let package_name = package_data.name.clone();
        let lock = self.get_package_lock(&package_name);
        
        // Acquire exclusive access to this package
        let _permit = lock.acquire().await.unwrap();
        
        tracing::debug!("Acquired lock for package: {}", package_name);
        
        // Check if package already exists in database
        let existing_package = self.check_existing_package(&package_name).await?;
        
        if existing_package {
            tracing::info!("Package {} already exists, skipping", package_name);
            return Ok(());
        }
        
        // Save the package
        let result = self.save_package_to_database(package_data).await;
        
        tracing::debug!("Released lock for package: {}", package_name);
        
        result
    }

    /// Check if a package already exists in the database
    async fn check_existing_package(&self, package_name: &str) -> Result<bool> {
        // Create a simple query to check if package exists
        let query = format!(r#"
        {{
            "selector": {{
                "name": "{}"
            }},
            "limit": 1
        }}"#, package_name);

        match self.database.packages().find::<serde_json::Value>(&serde_json::from_str(&query)?).await {
            Ok(results) => {
                let docs = results.get_data();
                Ok(!docs.is_empty())
            }
            Err(_) => Ok(false), // Assume doesn't exist if query fails
        }
    }

    /// Save package to database (the actual database operation)
    async fn save_package_to_database(&self, package_data: ScrapedPackage) -> Result<()> {
        use chrono::Utc;
        use uuid::Uuid;
        use crate::db::{Package, PackageVersion};

        let package = Package {
            id: Uuid::new_v4().to_string(),
            rev: None,
            name: package_data.name.clone(),
            description: package_data.description,
            homepage: package_data.homepage,
            repository: package_data.repository,
            license: package_data.license,
            maintainers: package_data.maintainers,
            tags: package_data.tags,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            submitted_by: Some("scraper".to_string()),
            platform: package_data.platform,
            language: package_data.language,
            status: package_data.status,
            dependents_count: package_data.dependents_count,
            rank: package_data.rank,
        };

        let mut package_value = serde_json::to_value(&package)?;
        self.database.packages().save(&mut package_value).await?;
        
        tracing::info!("Saved package: {}", package.name);
        
        // Save versions
        for version_data in package_data.versions {
            let version = PackageVersion {
                id: Uuid::new_v4().to_string(),
                rev: None,
                package_id: package.id.clone(),
                version: version_data.version.clone(),
                release_date: version_data.release_date,
                download_url: version_data.download_url,
                checksum: version_data.checksum,
                dependencies: version_data.dependencies,
                vulnerabilities: Vec::new(),
                changelog: version_data.changelog,
                created_at: Utc::now(),
            };
            
            let mut version_value = serde_json::to_value(&version)?;
            self.database.versions().save(&mut version_value).await?;
            
            tracing::debug!("Saved version {} for package {}", version.version, package.name);
        }

        Ok(())
    }

    /// Cleanup old locks that are no longer needed
    pub fn cleanup_unused_locks(&self) {
        self.package_locks.retain(|_, semaphore| {
            // Keep locks that have permits available (someone might be waiting)
            semaphore.available_permits() < 1
        });
    }
}