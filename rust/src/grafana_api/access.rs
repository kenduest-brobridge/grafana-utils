use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};
use crate::http::JsonHttpClient;

pub(crate) struct AccessResourceClient<'a> {
    http: &'a JsonHttpClient,
}

impl<'a> AccessResourceClient<'a> {
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

    pub(crate) fn fetch_current_org(&self) -> Result<Map<String, Value>> {
        match self.request_json(Method::GET, "/api/org", &[], None)? {
            Some(value) => {
                let object = value
                    .as_object()
                    .cloned()
                    .ok_or_else(|| message("Unexpected current-org payload from Grafana."))?;
                Ok(object)
            }
            None => Err(message("Grafana did not return current-org metadata.")),
        }
    }

    pub(crate) fn list_orgs(&self) -> Result<Vec<Map<String, Value>>> {
        match self.request_json(Method::GET, "/api/orgs", &[], None)? {
            Some(Value::Array(items)) => items
                .into_iter()
                .map(|item| {
                    item.as_object()
                        .cloned()
                        .ok_or_else(|| message("Unexpected org entry in /api/orgs response."))
                })
                .collect(),
            Some(_) => Err(message("Unexpected /api/orgs payload from Grafana.")),
            None => Ok(Vec::new()),
        }
    }
}
