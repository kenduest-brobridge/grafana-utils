//! Minimal Grafana alerting API client.
//! Wraps typed request wrappers for list/import/export flows and hides raw HTTP plumbing from CLI handlers.
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::AlertAuthContext;

/// Struct definition for GrafanaAlertClient.
pub struct GrafanaAlertClient {
    http: JsonHttpClient,
}

impl GrafanaAlertClient {
    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn new(context: &AlertAuthContext) -> Result<Self> {
        Ok(Self {
            http: JsonHttpClient::new(JsonHttpClientConfig {
                base_url: context.url.clone(),
                headers: context.headers.clone(),
                timeout_secs: context.timeout,
                verify_ssl: context.verify_ssl,
            })?,
        })
    }

    fn request_json(
        &self,
        method: Method,
        path: &str,
        params: &[(String, String)],
        payload: Option<&Value>,
    ) -> Result<Option<Value>> {
        self.http.request_json(method, path, params, payload)
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn list_alert_rules(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(Method::GET, "/api/v1/provisioning/alert-rules", &[], None)?,
            "Unexpected alert-rule list response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn list_orgs(&self) -> Result<Vec<Map<String, Value>>> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: alert_client.rs:expect_object_list, alert_client.rs:request_json

        expect_object_list(
            self.request_json(Method::GET, "/api/orgs", &[], None)?,
            "Unexpected /api/orgs payload from Grafana.",
        )
    }

    /// search dashboards.
    pub fn search_dashboards(&self, query: &str) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(
                Method::GET,
                "/api/search",
                &[
                    ("type".to_string(), "dash-db".to_string()),
                    ("query".to_string(), query.to_string()),
                    ("limit".to_string(), "500".to_string()),
                ],
                None,
            )?,
            "Unexpected dashboard search response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn get_dashboard(&self, uid: &str) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::GET,
                &format!("/api/dashboards/uid/{uid}"),
                &[],
                None,
            )?,
            &format!("Unexpected dashboard payload for UID {uid}."),
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn get_alert_rule(&self, uid: &str) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::GET,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                &[],
                None,
            )?,
            &format!("Unexpected alert-rule payload for UID {uid}."),
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn create_alert_rule(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::POST,
                "/api/v1/provisioning/alert-rules",
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected alert-rule create response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn update_alert_rule(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected alert-rule update response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn list_contact_points(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(
                Method::GET,
                "/api/v1/provisioning/contact-points",
                &[],
                None,
            )?,
            "Unexpected contact-point list response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn create_contact_point(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::POST,
                "/api/v1/provisioning/contact-points",
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected contact-point create response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn update_contact_point(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/contact-points/{uid}"),
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected contact-point update response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn list_mute_timings(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(Method::GET, "/api/v1/provisioning/mute-timings", &[], None)?,
            "Unexpected mute-timing list response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn create_mute_timing(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::POST,
                "/api/v1/provisioning/mute-timings",
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected mute-timing create response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn update_mute_timing(
        &self,
        name: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/mute-timings/{name}"),
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected mute-timing update response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn get_notification_policies(&self) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(Method::GET, "/api/v1/provisioning/policies", &[], None)?,
            "Unexpected notification policy response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn update_notification_policies(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::PUT,
                "/api/v1/provisioning/policies",
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected notification policy update response from Grafana.",
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn list_templates(&self) -> Result<Vec<Map<String, Value>>> {
        parse_template_list_response(self.request_json(
            Method::GET,
            "/api/v1/provisioning/templates",
            &[],
            None,
        )?)
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn get_template(&self, name: &str) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[],
                None,
            )?,
            &format!("Unexpected template payload for name {name}."),
        )
    }

    /// Purpose: implementation note.
    ///
    /// Args: see function signature.
    /// Returns: see implementation.
    pub fn update_template(
        &self,
        name: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        let mut body = payload.clone();
        body.remove("name");
        expect_object(
            self.request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[],
                Some(&Value::Object(body)),
            )?,
            "Unexpected template update response from Grafana.",
        )
    }
}

fn expect_object(value: Option<Value>, error_message: &str) -> Result<Map<String, Value>> {
    match value {
        Some(Value::Object(object)) => Ok(object),
        _ => Err(message(error_message)),
    }
}

/// expect object list.
pub fn expect_object_list(
    value: Option<Value>,
    error_message: &str,
) -> Result<Vec<Map<String, Value>>> {
    let array = match value {
        Some(Value::Array(items)) => items,
        _ => return Err(message(error_message)),
    };
    Ok(array
        .into_iter()
        .filter_map(|item| match item {
            Value::Object(object) => Some(object),
            _ => None,
        })
        .collect())
}

/// Interpret Grafana template-list responses in the one supported shape for
/// alerting imports: either empty/null or a JSON array of template objects.
pub fn parse_template_list_response(value: Option<Value>) -> Result<Vec<Map<String, Value>>> {
// Call graph (hierarchy): this function is used in related modules.
// Upstream callers: alert_client.rs:list_templates, alert_rust_tests.rs:template_list_null_is_treated_as_empty_in_live_client_path
// Downstream callees: alert_client.rs:expect_object_list, alert_client.rs:new

    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(value) => expect_object_list(
            Some(value),
            "Unexpected template list response from Grafana.",
        ),
    }
}
