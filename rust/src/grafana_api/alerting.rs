use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};
use crate::http::JsonHttpClient;

pub(crate) struct AlertingResourceClient<'a> {
    http: &'a JsonHttpClient,
}

impl<'a> AlertingResourceClient<'a> {
    pub(crate) fn new(http: &'a JsonHttpClient) -> Self {
        Self { http }
    }

    pub(crate) fn request_json(
        &self,
        method: Method,
        path: &str,
        params: &[(String, String)],
        payload: Option<&Value>,
    ) -> Result<Option<Value>> {
        self.http.request_json(method, path, params, payload)
    }

    pub(crate) fn list_alert_rules(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(Method::GET, "/api/v1/provisioning/alert-rules", &[], None)?,
            "Unexpected alert-rule list response from Grafana.",
        )
    }

    pub(crate) fn list_orgs(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(Method::GET, "/api/orgs", &[], None)?,
            "Unexpected /api/orgs payload from Grafana.",
        )
    }

    pub(crate) fn search_dashboards(&self, query: &str) -> Result<Vec<Map<String, Value>>> {
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

    pub(crate) fn get_dashboard(&self, uid: &str) -> Result<Map<String, Value>> {
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

    pub(crate) fn get_alert_rule(&self, uid: &str) -> Result<Map<String, Value>> {
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

    pub(crate) fn create_alert_rule(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
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

    pub(crate) fn update_alert_rule(
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

    pub(crate) fn list_contact_points(&self) -> Result<Vec<Map<String, Value>>> {
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

    pub(crate) fn create_contact_point(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
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

    pub(crate) fn update_contact_point(
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

    pub(crate) fn list_mute_timings(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(Method::GET, "/api/v1/provisioning/mute-timings", &[], None)?,
            "Unexpected mute-timing list response from Grafana.",
        )
    }

    pub(crate) fn create_mute_timing(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
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

    pub(crate) fn update_mute_timing(
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

    pub(crate) fn get_notification_policies(&self) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(Method::GET, "/api/v1/provisioning/policies", &[], None)?,
            "Unexpected notification policy response from Grafana.",
        )
    }

    pub(crate) fn update_notification_policies(
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

    pub(crate) fn delete_notification_policies(&self) -> Result<Value> {
        Ok(self
            .request_json(Method::DELETE, "/api/v1/provisioning/policies", &[], None)?
            .unwrap_or(Value::Null))
    }

    pub(crate) fn list_templates(&self) -> Result<Vec<Map<String, Value>>> {
        parse_template_list_response(self.request_json(
            Method::GET,
            "/api/v1/provisioning/templates",
            &[],
            None,
        )?)
    }

    pub(crate) fn get_template(&self, name: &str) -> Result<Map<String, Value>> {
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

    pub(crate) fn update_template(
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

    pub(crate) fn delete_alert_rule(&self, uid: &str) -> Result<Value> {
        Ok(self
            .request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                &[],
                None,
            )?
            .unwrap_or(Value::Null))
    }

    pub(crate) fn delete_contact_point(&self, uid: &str) -> Result<Value> {
        Ok(self
            .request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/contact-points/{uid}"),
                &[],
                None,
            )?
            .unwrap_or(Value::Null))
    }

    pub(crate) fn delete_mute_timing(&self, name: &str) -> Result<Value> {
        Ok(self
            .request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/mute-timings/{name}"),
                &[("version".to_string(), String::new())],
                None,
            )?
            .unwrap_or(Value::Null))
    }

    pub(crate) fn delete_template(&self, name: &str) -> Result<Value> {
        Ok(self
            .request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[("version".to_string(), String::new())],
                None,
            )?
            .unwrap_or(Value::Null))
    }
}

pub(crate) fn expect_object(
    value: Option<Value>,
    error_message: &str,
) -> Result<Map<String, Value>> {
    match value {
        Some(Value::Object(object)) => Ok(object),
        _ => Err(message(error_message)),
    }
}

pub(crate) fn expect_object_list(
    value: Option<Value>,
    error_message: &str,
) -> Result<Vec<Map<String, Value>>> {
    match value {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| match item {
                Value::Object(object) => Ok(object),
                _ => Err(message(error_message)),
            })
            .collect(),
        None => Ok(Vec::new()),
        _ => Err(message(error_message)),
    }
}

pub(crate) fn parse_template_list_response(
    value: Option<Value>,
) -> Result<Vec<Map<String, Value>>> {
    match value {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| match item {
                Value::Object(object) => Ok(object),
                _ => Err(message(
                    "Unexpected notification-template list response from Grafana.",
                )),
            })
            .collect(),
        Some(Value::Null) | None => Ok(Vec::new()),
        _ => Err(message(
            "Unexpected notification-template list response from Grafana.",
        )),
    }
}
