use anyhow::Result;
use chrono::{DateTime, Utc};
use fossdb::{CollectedPackage, Package, PackageLookup};
use reqwest::Client;
use serde::Serialize;

#[cfg(feature = "collector-libraries-io")]
use governor::{Quota, RateLimiter};
#[cfg(feature = "collector-libraries-io")]
use reqwest::Response;
#[cfg(feature = "collector-libraries-io")]
use std::num::NonZeroU32;
#[cfg(feature = "collector-libraries-io")]
use std::sync::Arc;

pub struct CollectorClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl CollectorClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self> {
        let client = Client::builder().user_agent("fossdb-collector").build()?;
        Ok(Self { client, base_url, api_key })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    pub async fn get_package(&self, name: &str) -> Result<Option<PackageLookup>> {
        let encoded_name: String = name.chars().map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        }).collect();

        let resp = self.client
            .get(format!("{}/packages?name={}", self.base_url, encoded_name))
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }

        let lookup: PackageLookup = resp.error_for_status()?.json().await?;
        Ok(Some(lookup))
    }

    pub async fn submit_package(&self, pkg: CollectedPackage) -> Result<Package> {
        let resp = self.client
            .post(format!("{}/packages", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&pkg)
            .send()
            .await?;

        let package: Package = resp.error_for_status()?.json().await?;
        Ok(package)
    }

    pub async fn update_package_timestamp(&self, name: &str, updated_at: DateTime<Utc>) -> Result<()> {
        #[derive(Serialize)]
        struct Body {
            updated_at: DateTime<Utc>,
        }

        self.client
            .put(format!("{}/packages/{}/timestamp", self.base_url, name))
            .header("Authorization", self.auth_header())
            .json(&Body { updated_at })
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(feature = "collector-libraries-io")]
pub struct AdaptiveConfig {
    pub initial_rate: u32,
    pub min_rate: u32,
    pub max_rate: u32,
}

#[cfg(feature = "collector-libraries-io")]
#[derive(Clone)]
pub struct AdaptiveRateLimitedClient {
    client: Client,
    limiter: Arc<
        tokio::sync::RwLock<
            RateLimiter<
                governor::state::direct::NotKeyed,
                governor::state::InMemoryState,
                governor::clock::DefaultClock,
            >,
        >,
    >,
    config: Arc<AdaptiveConfig>,
    current_rate: Arc<tokio::sync::RwLock<u32>>,
}

#[cfg(feature = "collector-libraries-io")]
impl AdaptiveRateLimitedClient {
    pub fn new(client: Client, config: AdaptiveConfig) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(config.initial_rate).unwrap());
        let limiter = Arc::new(tokio::sync::RwLock::new(RateLimiter::direct(quota)));
        let current_rate = Arc::new(tokio::sync::RwLock::new(config.initial_rate));
        Self {
            client,
            limiter,
            config: Arc::new(config),
            current_rate,
        }
    }

    pub async fn get(&self, url: &str) -> Result<Response, reqwest::Error> {
        {
            let limiter = self.limiter.read().await;
            limiter.until_ready().await;
        }
        let response = self.client.get(url).send().await?;
        self.report_result(response.status().as_u16()).await;
        Ok(response)
    }

    async fn report_result(&self, status_code: u16) {
        let mut current_rate = self.current_rate.write().await;
        let old_rate = *current_rate;

        let new_rate = match status_code {
            429 => (old_rate / 2).max(self.config.min_rate),
            500..=599 => ((old_rate * 9) / 10).max(self.config.min_rate),
            200..=299 => ((old_rate * 11) / 10).min(self.config.max_rate),
            _ => old_rate,
        };

        if new_rate != old_rate {
            *current_rate = new_rate;
            let quota = Quota::per_second(NonZeroU32::new(new_rate).unwrap());
            let mut limiter = self.limiter.write().await;
            *limiter = RateLimiter::direct(quota);
        }
    }
}
