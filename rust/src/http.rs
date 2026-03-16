//! Shared HTTP transport for all Rust domains.
//! Wraps reqwest blocking client creation, URL building, query encoding, and request/response error mapping.
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, ACCEPT_ENCODING, CONTENT_TYPE};
use reqwest::{Method, StatusCode, Url};
use serde_json::Value;

use crate::common::{api_response, message, Result};

#[derive(Debug, Clone)]
pub struct JsonHttpClientConfig {
    pub base_url: String,
    pub headers: Vec<(String, String)>,
    pub timeout_secs: u64,
    pub verify_ssl: bool,
}

pub struct JsonHttpClient {
    base_url: String,
    client: Client,
}

impl JsonHttpClient {
    pub fn new(config: JsonHttpClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
        for (name, value) in config.headers {
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| message(format!("Invalid header name: {name}")))?;
            let header_value = HeaderValue::from_str(&value)
                .map_err(|_| message(format!("Invalid header value for {name}")))?;
            headers.insert(header_name, header_value);
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .http1_only()
            .build()?;

        Ok(Self {
            base_url: config.base_url.trim_end_matches('/').to_string(),
            client,
        })
    }

    // Low-level HTTP execution hook used by all domain clients.
    // Returns decoded JSON on success and maps non-2xx responses through domain Result errors.
    pub fn request_json(
        &self,
        method: Method,
        path: &str,
        params: &[(String, String)],
        payload: Option<&Value>,
    ) -> Result<Option<Value>> {
        let url = self.build_url(path, params)?;
        let mut request = self.client.request(method, url.clone());
        if payload.is_some() {
            request = request.header(CONTENT_TYPE, "application/json");
        }
        if let Some(payload) = payload {
            request = request.json(payload);
        }

        let response = request.send()?;
        let status = response.status();
        let body = response.bytes()?;
        let body_text = String::from_utf8_lossy(&body).to_string();

        if status.is_client_error() || status.is_server_error() {
            return Err(api_response(status.as_u16(), url.to_string(), body_text));
        }

        if body_text.trim().is_empty() || status == StatusCode::NO_CONTENT {
            return Ok(None);
        }

        Ok(Some(serde_json::from_slice(&body)?))
    }

    // Centralized URL constructor for path+query assembly.
    // Accepts already-resolved base_url and enforces consistent param encoding.
    fn build_url(&self, path: &str, params: &[(String, String)]) -> Result<Url> {
        let mut url = Url::parse(&format!("{}{}", self.base_url, path))
            .map_err(|error| message(format!("Invalid request URL {path}: {error}")))?;
        if !params.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in params {
                pairs.append_pair(key, value);
            }
        }
        Ok(url)
    }
}

#[cfg(test)]
#[path = "http_rust_tests.rs"]
mod http_rust_tests;
