//! HTTP transport unit tests.
//! Checks client construction behavior and can be extended for request/URL building
//! contract coverage.
use super::{JsonHttpClient, JsonHttpClientConfig};

#[test]
fn client_builder_accepts_basic_config() {
    let client = JsonHttpClient::new(JsonHttpClientConfig {
        base_url: "http://127.0.0.1:3000".to_string(),
        headers: vec![("Authorization".to_string(), "Bearer token".to_string())],
        timeout_secs: 30,
        verify_ssl: false,
    });
    assert!(client.is_ok());
}
