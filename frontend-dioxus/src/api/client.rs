use super::types::*;
use gloo_console::log;
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

type Result<T> = std::result::Result<T, JsValue>;

pub struct ApiClient {
    base_url: String,
    token: Option<String>,
}

impl ApiClient {
    pub fn new() -> Self {
        let base_url = web_sys::window()
            .and_then(|w| w.location().hostname().ok())
            .map(|hostname| {
                if hostname == "localhost" || hostname == "127.0.0.1" {
                    "http://localhost:3000/api".to_string()
                } else {
                    "/api".to_string()
                }
            })
            .unwrap_or_else(|| "/api".to_string());

        Self {
            base_url,
            token: None,
        }
    }

    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<String>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);

        let mut opts = RequestInit::new();
        opts.method(method);
        opts.mode(RequestMode::Cors);

        if let Some(body_str) = body {
            opts.body(Some(&JsValue::from_str(&body_str)));
        }

        let request = Request::new_with_str_and_init(&url, &opts)?;

        request.headers().set("Content-Type", "application/json")?;

        if let Some(token) = &self.token {
            request.headers().set("Authorization", &format!("Bearer {}", token))?;
        }

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        if !resp.ok() {
            return Err(JsValue::from_str(&format!("HTTP error: {}", resp.status())));
        }

        let json = JsFuture::from(resp.json().map_err(|_| JsValue::from_str("Failed to parse response"))?).await?;
        serde_wasm_bindgen::from_value(json).map_err(|e| JsValue::from_str(&format!("Deserialization error: {:?}", e)))
    }

    pub async fn login(&self, email: String, password: String) -> Result<AuthResponse> {
        let body = serde_json::to_string(&LoginRequest { email, password })
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
        self.request("POST", "/auth/login", Some(body)).await
    }

    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<AuthResponse> {
        let body = serde_json::to_string(&RegisterRequest {
            username,
            email,
            password,
        }).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
        self.request("POST", "/auth/register", Some(body)).await
    }

    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        self.request("GET", "/stats", None).await
    }

    pub async fn get_packages(&self, search: Option<String>, page: u32, limit: u32) -> Result<PackagesResponse> {
        let mut path = format!("/packages?page={}&limit={}", page, limit);
        if let Some(query) = search {
            path.push_str(&format!("&search={}", query));
        }
        self.request("GET", &path, None).await
    }

    pub async fn get_package(&self, id: &str) -> Result<Package> {
        self.request("GET", &format!("/packages/{}", id), None).await
    }

    pub async fn get_package_versions(&self, id: &str) -> Result<Vec<PackageVersion>> {
        self.request("GET", &format!("/packages/{}/versions", id), None).await
    }

    pub async fn get_package_subscribers(&self, id: &str) -> Result<usize> {
        #[derive(serde::Deserialize)]
        struct SubscriberCount {
            count: usize,
        }
        let result: SubscriberCount = self
            .request("GET", &format!("/packages/{}/subscribers", id), None)
            .await?;
        Ok(result.count)
    }

    pub async fn get_subscriptions(&self) -> Result<Vec<SubscriptionResponse>> {
        self.request("GET", "/users/subscriptions", None).await
    }

    pub async fn subscribe(&self, package_name: String) -> Result<()> {
        let body = serde_json::to_string(&SubscriptionRequest { package_name })
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
        self.request("POST", "/users/subscriptions", Some(body)).await
    }

    pub async fn unsubscribe(&self, package_name: &str) -> Result<()> {
        self.request("DELETE", &format!("/users/subscriptions/{}", package_name), None).await
    }

    pub async fn toggle_notifications(&self, package_name: &str, enabled: bool) -> Result<()> {
        #[derive(serde::Serialize)]
        struct NotificationToggle {
            notifications_enabled: bool,
        }
        let body = serde_json::to_string(&NotificationToggle {
            notifications_enabled: enabled,
        }).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
        self.request(
            "PUT",
            &format!("/users/subscriptions/{}/notifications", package_name),
            Some(body),
        )
        .await
    }

    pub async fn get_timeline(&self, offset: usize, limit: usize) -> Result<TimelineResponse> {
        self.request(
            "GET",
            &format!("/users/timeline?offset={}&limit={}", offset, limit),
            None,
        )
        .await
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}
