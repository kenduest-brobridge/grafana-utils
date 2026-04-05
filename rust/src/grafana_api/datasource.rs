use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};
use crate::http::JsonHttpClient;

pub(crate) struct DatasourceResourceClient<'a> {
    http: &'a JsonHttpClient,
}

impl<'a> DatasourceResourceClient<'a> {
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

    pub(crate) fn list_datasources(&self) -> Result<Vec<Map<String, Value>>> {
        match self.request_json(Method::GET, "/api/datasources", &[], None)? {
            Some(Value::Array(items)) => items
                .into_iter()
                .map(|item| match item {
                    Value::Object(object) => Ok(object),
                    _ => Err(message("Unexpected datasource payload from Grafana.")),
                })
                .collect(),
            Some(_) => Err(message("Unexpected datasource list response from Grafana.")),
            None => Ok(Vec::new()),
        }
    }

    pub(crate) fn fetch_current_org(&self) -> Result<Map<String, Value>> {
        match self.request_json(Method::GET, "/api/org", &[], None)? {
            Some(Value::Object(object)) => Ok(object),
            Some(_) => Err(message("Unexpected current-org payload from Grafana.")),
            None => Err(message("Grafana did not return current-org metadata.")),
        }
    }

    pub(crate) fn list_orgs(&self) -> Result<Vec<Map<String, Value>>> {
        match self.request_json(Method::GET, "/api/orgs", &[], None)? {
            Some(Value::Array(items)) => items
                .into_iter()
                .map(|item| match item {
                    Value::Object(object) => Ok(object),
                    _ => Err(message("Unexpected org entry in /api/orgs response.")),
                })
                .collect(),
            Some(_) => Err(message("Unexpected /api/orgs payload from Grafana.")),
            None => Ok(Vec::new()),
        }
    }

    pub(crate) fn create_org(&self, org_name: &str) -> Result<Map<String, Value>> {
        let payload = Value::Object(Map::from_iter(vec![(
            "name".to_string(),
            Value::String(org_name.to_string()),
        )]));
        match self.request_json(Method::POST, "/api/orgs", &[], Some(&payload))? {
            Some(Value::Object(object)) => Ok(object),
            Some(_) => Err(message("Unexpected create-org payload from Grafana.")),
            None => Err(message("Grafana did not return create-org metadata.")),
        }
    }

    pub(crate) fn create_datasource(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        match self.request_json(
            Method::POST,
            "/api/datasources",
            &[],
            Some(&Value::Object(payload.clone())),
        )? {
            Some(Value::Object(object)) => Ok(object),
            _ => Err(message("Unexpected datasource create response from Grafana.")),
        }
    }

    pub(crate) fn update_datasource(
        &self,
        datasource_id: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        match self.request_json(
            Method::PUT,
            &format!("/api/datasources/{datasource_id}"),
            &[],
            Some(&Value::Object(payload.clone())),
        )? {
            Some(Value::Object(object)) => Ok(object),
            _ => Err(message("Unexpected datasource update response from Grafana.")),
        }
    }

    pub(crate) fn delete_datasource(&self, datasource_id: &str) -> Result<Value> {
        Ok(self
            .request_json(Method::DELETE, &format!("/api/datasources/{datasource_id}"), &[], None)?
            .unwrap_or(Value::Null))
    }
}
