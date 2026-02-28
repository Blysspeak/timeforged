use reqwest::Client;
use serde::de::DeserializeOwned;

use timeforged_core::api::ErrorResponse;
use timeforged_core::config::CliConfig;

pub struct TfClient {
    http: Client,
    base_url: String,
    api_key: Option<String>,
}

impl TfClient {
    pub fn new(config: &CliConfig) -> Self {
        Self {
            http: Client::new(),
            base_url: config.server_url.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let mut req = self.http.get(self.url(path));
        if let Some(ref key) = self.api_key {
            req = req.header("X-Api-Key", key);
        }
        let resp = req.send().await.map_err(|e| format!("request failed: {e}"))?;
        handle_response(resp).await
    }

    pub async fn get_with_query<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, String> {
        let mut req = self.http.get(self.url(path)).query(query);
        if let Some(ref key) = self.api_key {
            req = req.header("X-Api-Key", key);
        }
        let resp = req.send().await.map_err(|e| format!("request failed: {e}"))?;
        handle_response(resp).await
    }

    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let mut req = self.http.post(self.url(path)).json(body);
        if let Some(ref key) = self.api_key {
            req = req.header("X-Api-Key", key);
        }
        let resp = req.send().await.map_err(|e| format!("request failed: {e}"))?;
        handle_response(resp).await
    }

    pub async fn delete_with_body<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let mut req = self.http.delete(self.url(path)).json(body);
        if let Some(ref key) = self.api_key {
            req = req.header("X-Api-Key", key);
        }
        let resp = req.send().await.map_err(|e| format!("request failed: {e}"))?;
        handle_response(resp).await
    }

    pub async fn health(&self) -> Result<timeforged_core::api::HealthResponse, String> {
        self.get("/health").await
    }
}

async fn handle_response<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, String> {
    let status = resp.status();
    if status.is_success() {
        resp.json::<T>().await.map_err(|e| format!("parse error: {e}"))
    } else {
        let body = resp
            .json::<ErrorResponse>()
            .await
            .map(|e| e.error)
            .unwrap_or_else(|_| format!("HTTP {status}"));
        Err(body)
    }
}
