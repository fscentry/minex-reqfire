use std::time::Instant;
use chrono::Utc;
use reqwest::{Client, Method};
use tracing::{debug, info, warn};

use crate::config::{TestConfig, HttpMethod};
use crate::models::RequestRecord;

pub struct ApiClient {
    client: Client,
    config: TestConfig,
}

impl ApiClient {
    pub fn new(config: &TestConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let request_timeout = std::time::Duration::from_millis(config.timeout_ms);

        let connect_timeout = std::time::Duration::from_millis(
            (config.timeout_ms as f64 * 0.3).max(5_000.0) as u64
        );

        if config.danger_accept_invalid_certs {
            warn!("⚠️ TLS verification disabled");
        }

        let client = Client::builder()
            .timeout(request_timeout)
            .connect_timeout(connect_timeout)
            // .danger_accept_invalid_certs(config.danger_accept_invalid_certs)
            .danger_accept_invalid_certs(config.danger_accept_invalid_certs)

            .redirect(if config.follow_redirects {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .pool_max_idle_per_host(config.parallel_limit * 2)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .tcp_nodelay(true)
            .user_agent("api-perf-test/1.0")
            .build()
            .map_err(|e| format!("Failed building HTTP client: {}", e))?;

        info!(
            "HTTP Client Ready — timeout: {}ms",
            config.timeout_ms
        );

        Ok(ApiClient {
            client,
            config: config.clone(),
        })
    }

    pub async fn execute_request(
        &self,
        request_id: usize,
        worker_id: usize,
        body_override: Option<serde_json::Value>,
    ) -> RequestRecord {
        let started_at = Utc::now();
        let timer = Instant::now();

        let result = self.send_request(body_override).await;

        let elapsed_ms = timer.elapsed().as_secs_f64() * 1000.0;
        let finished_at = Utc::now();

        match result {
            Ok((status_code, response_size)) => {
                let success = status_code >= 200 && status_code < 300;

                RequestRecord {
                    request_id,
                    worker_id,
                    started_at,
                    finished_at,
                    response_time_ms: elapsed_ms,
                    status_code,
                    success,
                    error_message: None,
                    response_size_bytes: response_size,
                }
            }
            Err(e) => {
                RequestRecord {
                    request_id,
                    worker_id,
                    started_at,
                    finished_at,
                    response_time_ms: elapsed_ms,
                    status_code: 0,
                    success: false,
                    error_message: Some(e.to_string()),
                    response_size_bytes: 0,
                }
            }
        }
    }

    async fn send_request(
        &self,
        body_override: Option<serde_json::Value>,
    ) -> Result<(u16, usize), Box<dyn std::error::Error + Send + Sync>> {
        let method = to_reqwest_method(&self.config.method);

        let mut req_builder = self.client.request(method, &self.config.url);

        for (key, value) in &self.config.headers {
            req_builder = req_builder.header(key, value);
        }

        let effective_body = body_override.or_else(|| self.config.body.clone());

        if let Some(body) = effective_body {
            req_builder = req_builder.json(&body);
        }

        let response = req_builder.send().await?;

        let status = response.status().as_u16();
        let body_size = response.bytes().await?.len();

        Ok((status, body_size))
    }
}

fn to_reqwest_method(method: &HttpMethod) -> Method {
    match method {
        HttpMethod::GET => Method::GET,
        HttpMethod::POST => Method::POST,
        HttpMethod::PUT => Method::PUT,
        HttpMethod::PATCH => Method::PATCH,
        HttpMethod::DELETE => Method::DELETE,
    }
}