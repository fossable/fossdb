use governor::{Quota, RateLimiter};
use reqwest::{Client, RequestBuilder, Response};
use std::num::NonZeroU32;
use std::sync::Arc;

/// A wrapper around reqwest::Client that applies rate limiting to all requests.
#[derive(Clone)]
#[allow(dead_code)]
pub struct RateLimitedClient {
    client: Client,
    limiter: Arc<
        RateLimiter<
            governor::state::direct::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
        >,
    >,
}

#[allow(dead_code)]
impl RateLimitedClient {
    /// Create a new rate-limited client.
    ///
    /// # Arguments
    /// * `client` - The underlying HTTP client
    /// * `requests_per_second` - Maximum requests per second
    ///
    /// # Examples
    /// ```
    /// let client = reqwest::Client::new();
    /// let limited = RateLimitedClient::new(client, 10); // 10 req/s
    /// ```
    pub fn new(client: Client, requests_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap());
        let limiter = Arc::new(RateLimiter::direct(quota));
        Self { client, limiter }
    }

    /// Create a rate-limited client with requests per minute.
    ///
    /// # Arguments
    /// * `client` - The underlying HTTP client
    /// * `requests_per_minute` - Maximum requests per minute
    pub fn per_minute(client: Client, requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        let limiter = Arc::new(RateLimiter::direct(quota));
        Self { client, limiter }
    }

    /// Create a rate-limited client with custom burst capacity.
    ///
    /// # Arguments
    /// * `client` - The underlying HTTP client
    /// * `requests_per_second` - Maximum requests per second
    /// * `burst_size` - Maximum burst size (number of requests that can be made immediately)
    pub fn with_burst(client: Client, requests_per_second: u32, burst_size: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap())
            .allow_burst(NonZeroU32::new(burst_size).unwrap());
        let limiter = Arc::new(RateLimiter::direct(quota));
        Self { client, limiter }
    }

    /// Get a reference to the underlying client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Create a GET request (rate limited).
    pub async fn get(&self, url: &str) -> RequestBuilder {
        self.limiter.until_ready().await;
        self.client.get(url)
    }

    /// Create a POST request (rate limited).
    pub async fn post(&self, url: &str) -> RequestBuilder {
        self.limiter.until_ready().await;
        self.client.post(url)
    }

    /// Create a PUT request (rate limited).
    pub async fn put(&self, url: &str) -> RequestBuilder {
        self.limiter.until_ready().await;
        self.client.put(url)
    }

    /// Create a DELETE request (rate limited).
    pub async fn delete(&self, url: &str) -> RequestBuilder {
        self.limiter.until_ready().await;
        self.client.delete(url)
    }

    /// Create a PATCH request (rate limited).
    pub async fn patch(&self, url: &str) -> RequestBuilder {
        self.limiter.until_ready().await;
        self.client.patch(url)
    }

    /// Create a HEAD request (rate limited).
    pub async fn head(&self, url: &str) -> RequestBuilder {
        self.limiter.until_ready().await;
        self.client.head(url)
    }
}

/// Configuration for adaptive rate limiting based on HTTP response codes.
pub struct AdaptiveConfig {
    /// Initial requests per second
    pub initial_rate: u32,
    /// Minimum requests per second (safety floor)
    pub min_rate: u32,
    /// Maximum requests per second (ceiling)
    pub max_rate: u32,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            initial_rate: 10,
            min_rate: 1,
            max_rate: 100,
        }
    }
}

/// A wrapper around reqwest::Client with adaptive rate limiting.
///
/// This client automatically adjusts its rate limit based on HTTP responses,
/// slowing down when receiving 429 responses and gradually speeding up on success.
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

impl AdaptiveRateLimitedClient {
    /// Create a new adaptive rate-limited client.
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

    /// Create an adaptive client with default configuration.
    ///
    /// Starts at 10 req/s and adjusts between 1-100 req/s.
    #[allow(dead_code)]
    pub fn with_defaults(client: Client) -> Self {
        Self::new(client, AdaptiveConfig::default())
    }

    /// Get a reference to the underlying client.
    #[allow(dead_code)]
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Execute a GET request with adaptive rate limiting.
    pub async fn get(&self, url: &str) -> Result<Response, reqwest::Error> {
        {
            let limiter = self.limiter.read().await;
            limiter.until_ready().await;
        }
        let response = self.client.get(url).send().await?;
        self.report_result(response.status().as_u16()).await;
        Ok(response)
    }

    /// Execute a POST request with adaptive rate limiting.
    #[allow(dead_code)]
    pub async fn post(&self, url: &str) -> Result<Response, reqwest::Error> {
        {
            let limiter = self.limiter.read().await;
            limiter.until_ready().await;
        }
        let response = self.client.post(url).send().await?;
        self.report_result(response.status().as_u16()).await;
        Ok(response)
    }

    /// Create a GET RequestBuilder (requires manual response reporting).
    ///
    /// Use this when you need to configure the request before sending.
    /// Don't forget to call `report_response` after receiving the response.
    #[allow(dead_code)]
    pub async fn get_builder(&self, url: &str) -> RequestBuilder {
        {
            let limiter = self.limiter.read().await;
            limiter.until_ready().await;
        }
        self.client.get(url)
    }

    /// Create a POST RequestBuilder (requires manual response reporting).
    #[allow(dead_code)]
    pub async fn post_builder(&self, url: &str) -> RequestBuilder {
        {
            let limiter = self.limiter.read().await;
            limiter.until_ready().await;
        }
        self.client.post(url)
    }

    /// Report a response to the adaptive rate limiter.
    ///
    /// Use this after manually sending a request created with `*_builder` methods.
    #[allow(dead_code)]
    pub async fn report_response(&self, response: &Response) {
        self.report_result(response.status().as_u16()).await;
    }

    /// Report the result of an API request to adjust the rate limit.
    async fn report_result(&self, status_code: u16) {
        let mut current_rate = self.current_rate.write().await;
        let old_rate = *current_rate;

        let new_rate = match status_code {
            // Rate limited - decrease to half
            429 => (old_rate / 2).max(self.config.min_rate),
            // Server errors - slight decrease
            500..=599 => ((old_rate * 9) / 10).max(self.config.min_rate),
            // Success - gradually increase
            200..=299 => ((old_rate * 11) / 10).min(self.config.max_rate),
            // Other - no change
            _ => old_rate,
        };

        if new_rate != old_rate {
            *current_rate = new_rate;

            // Update the limiter with new rate
            let quota = Quota::per_second(NonZeroU32::new(new_rate).unwrap());
            let mut limiter = self.limiter.write().await;
            *limiter = RateLimiter::direct(quota);

            match status_code {
                429 => tracing::warn!(
                    "Rate limit hit (429), decreasing rate to {} req/s",
                    new_rate
                ),
                500..=599 => tracing::debug!(
                    "Server error, slightly decreasing rate to {} req/s",
                    new_rate
                ),
                200..=299 => {
                    tracing::debug!("Successful response, increasing rate to {} req/s", new_rate)
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limited_client_creation() {
        let client = Client::new();
        let _limited = RateLimitedClient::new(client, 5);
    }

    #[tokio::test]
    async fn test_adaptive_client_creation() {
        let client = Client::new();
        let _adaptive = AdaptiveRateLimitedClient::with_defaults(client);
    }
}
