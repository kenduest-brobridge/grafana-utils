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

    #[allow(dead_code)]
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

    pub(crate) fn list_org_users(&self) -> Result<Vec<Map<String, Value>>> {
        match self.request_json(Method::GET, "/api/org/users", &[], None)? {
            Some(Value::Array(items)) => items
                .into_iter()
                .map(|item| {
                    item.as_object()
                        .cloned()
                        .ok_or_else(|| message("Unexpected org user entry in /api/org/users."))
                })
                .collect(),
            Some(_) => Err(message("Unexpected org user list response from Grafana.")),
            None => Ok(Vec::new()),
        }
    }

    pub(crate) fn iter_global_users(&self, page_size: usize) -> Result<Vec<Map<String, Value>>> {
        let mut users = Vec::new();
        let mut page = 1usize;
        loop {
            let params = vec![
                ("page".to_string(), page.to_string()),
                ("perpage".to_string(), page_size.to_string()),
            ];
            let batch = match self.request_json(Method::GET, "/api/users", &params, None)? {
                Some(Value::Array(items)) => items
                    .into_iter()
                    .map(|item| {
                        item.as_object().cloned().ok_or_else(|| {
                            message("Unexpected global user entry in /api/users response.")
                        })
                    })
                    .collect::<Result<Vec<Map<String, Value>>>>()?,
                Some(_) => return Err(message("Unexpected global user list response from Grafana.")),
                None => Vec::new(),
            };
            if batch.is_empty() {
                break;
            }
            let batch_len = batch.len();
            users.extend(batch);
            if batch_len < page_size {
                break;
            }
            page += 1;
        }
        Ok(users)
    }

    pub(crate) fn iter_teams(
        &self,
        query: Option<&str>,
        page_size: usize,
    ) -> Result<Vec<Map<String, Value>>> {
        let mut teams = Vec::new();
        let mut page = 1usize;
        loop {
            let params = vec![
                ("query".to_string(), query.unwrap_or("").to_string()),
                ("page".to_string(), page.to_string()),
                ("perpage".to_string(), page_size.to_string()),
            ];
            let batch = match self.request_json(Method::GET, "/api/teams/search", &params, None)? {
                Some(Value::Object(object)) => object
                    .get("teams")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|item| {
                        item.as_object().cloned().ok_or_else(|| {
                            message("Unexpected team entry in /api/teams/search response.")
                        })
                    })
                    .collect::<Result<Vec<Map<String, Value>>>>()?,
                Some(_) => return Err(message("Unexpected team list response from Grafana.")),
                None => Vec::new(),
            };
            if batch.is_empty() {
                break;
            }
            let batch_len = batch.len();
            teams.extend(batch);
            if batch_len < page_size {
                break;
            }
            page += 1;
        }
        Ok(teams)
    }

    pub(crate) fn list_service_accounts(
        &self,
        page_size: usize,
    ) -> Result<Vec<Map<String, Value>>> {
        let mut rows = Vec::new();
        let mut page = 1usize;
        loop {
            let params = vec![
                ("query".to_string(), String::new()),
                ("page".to_string(), page.to_string()),
                ("perpage".to_string(), page_size.to_string()),
            ];
            let batch = match self.request_json(
                Method::GET,
                "/api/serviceaccounts/search",
                &params,
                None,
            )? {
                Some(Value::Object(object)) => object
                    .get("serviceAccounts")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|item| {
                        item.as_object().cloned().ok_or_else(|| {
                            message(
                                "Unexpected service-account entry in /api/serviceaccounts/search response.",
                            )
                        })
                    })
                    .collect::<Result<Vec<Map<String, Value>>>>()?,
                Some(_) => {
                    return Err(message("Unexpected service-account list response from Grafana."))
                }
                None => Vec::new(),
            };
            if batch.is_empty() {
                break;
            }
            let batch_len = batch.len();
            rows.extend(batch);
            if batch_len < page_size {
                break;
            }
            page += 1;
        }
        Ok(rows)
    }
}
