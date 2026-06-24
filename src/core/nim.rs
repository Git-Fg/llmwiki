use crate::error::WikiError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct NimClient {
    base_url: String,
    api_key: String,
    max_attempts: u32,
    backoff_ms: u64,
    timeout_secs: u64,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    input: Vec<&'a str>,
    model: &'a str,
    input_type: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedItem>,
}

#[derive(Deserialize)]
struct EmbedItem {
    embedding: Vec<f32>,
}

impl NimClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self::with_timeout(base_url, api_key, 30)
    }

    /// Construct a client with a custom request timeout (in seconds).
    /// Wired from `cfg.nim.request_timeout_secs` by call sites that have
    /// the resolved config (currently `embed.rs`, `search.rs`, `query.rs`,
    /// `doctor.rs`). The default constructor keeps the legacy 30s default.
    pub fn with_timeout(base_url: String, api_key: String, timeout_secs: u64) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs.max(1)))
            .build()
            .unwrap();
        NimClient {
            base_url,
            api_key,
            max_attempts: 3,
            backoff_ms: 500,
            timeout_secs,
            http,
        }
    }

    /// Current request timeout in seconds. Used by `doctor` to display
    /// the actual value used by the rest of the CLI.
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }

    pub fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n;
        self
    }
    pub fn with_backoff_ms(mut self, ms: u64) -> Self {
        self.backoff_ms = ms;
        self
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub async fn embed(
        &self,
        texts: &[&str],
        model: &str,
        input_type: &str,
    ) -> Result<Vec<Vec<f32>>, WikiError> {
        if self.api_key.is_empty() {
            return Err(WikiError::NimApiKeyMissing);
        }
        let url = format!("{}/v1/embeddings", self.base_url);
        let body = EmbedRequest {
            input: texts.to_vec(),
            model,
            input_type,
        };

        let mut attempt = 0;
        loop {
            attempt += 1;
            let res_result = self
                .http
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await;

            let resp = match res_result {
                Ok(r) => r,
                Err(e) => {
                    if attempt >= self.max_attempts {
                        return Err(WikiError::NimUnreachable(e.to_string()));
                    }
                    tokio::time::sleep(Duration::from_millis(self.backoff_ms * attempt as u64))
                        .await;
                    continue;
                }
            };

            if resp.status().is_success() {
                let parsed: EmbedResponse =
                    resp.json().await.map_err(|e| WikiError::Other(e.into()))?;
                return Ok(parsed.data.into_iter().map(|i| i.embedding).collect());
            }
            if resp.status().as_u16() == 401 {
                return Err(WikiError::NimUnreachable(
                    "401 Unauthorized — check NVIDIA_NIM_API_KEY".into(),
                ));
            }
            if attempt >= self.max_attempts {
                return Err(WikiError::NimUnreachable(format!(
                    "status {}",
                    resp.status()
                )));
            }
            tokio::time::sleep(Duration::from_millis(self.backoff_ms * attempt as u64)).await;
        }
    }
}
