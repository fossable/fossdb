use anyhow::Result;
use fossdb::{WorkerAnalysis, WorkerTask};

pub struct WorkerClient {
    http: reqwest::Client,
    server_url: String,
    api_key: String,
}

impl WorkerClient {
    pub fn new(server_url: String, api_key: String) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent("fossdb-worker")
            .build()?;
        Ok(Self { http, server_url, api_key })
    }

    pub async fn get_task(&self) -> Result<WorkerTask> {
        let resp = self
            .http
            .get(format!("{}/packages/next", self.server_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn submit_analysis(&self, analysis: &WorkerAnalysis) -> Result<WorkerAnalysis> {
        let resp = self
            .http
            .post(format!("{}/analysis", self.server_url))
            .bearer_auth(&self.api_key)
            .json(analysis)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.json().await?)
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }
}
