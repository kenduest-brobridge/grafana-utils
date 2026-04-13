use std::collections::BTreeSet;

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;

pub(crate) struct DashboardResourceClient<'a> {
    http: &'a JsonHttpClient,
}

impl<'a> DashboardResourceClient<'a> {
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
    pub(crate) fn search_dashboards(&self, query: &str) -> Result<Vec<Map<String, Value>>> {
        self.expect_object_list(
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

    pub(crate) fn list_folders(&self) -> Result<Vec<Map<String, Value>>> {
        self.expect_object_list(
            self.request_json(Method::GET, "/api/folders", &[], None)?,
            "Unexpected folder list response from Grafana.",
        )
    }

    pub(crate) fn list_dashboard_summaries(
        &self,
        page_size: usize,
    ) -> Result<Vec<Map<String, Value>>> {
        let mut dashboards = Vec::new();
        let mut seen_uids = BTreeSet::new();
        let mut page = 1;

        loop {
            let params = vec![
                ("type".to_string(), "dash-db".to_string()),
                ("limit".to_string(), page_size.to_string()),
                ("page".to_string(), page.to_string()),
            ];
            let response = self.request_json(Method::GET, "/api/search", &params, None)?;
            let batch = match response {
                Some(Value::Array(batch)) => batch,
                Some(_) => return Err(message("Unexpected search response from Grafana.")),
                None => Vec::new(),
            };
            if batch.is_empty() {
                break;
            }

            let batch_len = batch.len();
            for item in batch {
                let object =
                    value_as_object(&item, "Unexpected dashboard summary payload from Grafana.")?;
                let uid = string_field(object, "uid", "");
                if uid.is_empty() || seen_uids.contains(&uid) {
                    continue;
                }
                seen_uids.insert(uid);
                dashboards.push(object.clone());
            }

            if batch_len < page_size {
                break;
            }
            page += 1;
        }

        Ok(dashboards)
    }

    pub(crate) fn fetch_current_org(&self) -> Result<Map<String, Value>> {
        match self.request_json(Method::GET, "/api/org", &[], None)? {
            Some(value) => {
                let object =
                    value_as_object(&value, "Unexpected current-org payload from Grafana.")?;
                Ok(object.clone())
            }
            None => Err(message("Grafana did not return current-org metadata.")),
        }
    }

    pub(crate) fn list_orgs(&self) -> Result<Vec<Map<String, Value>>> {
        self.expect_object_list(
            self.request_json(Method::GET, "/api/orgs", &[], None)?,
            "Unexpected org list payload from Grafana.",
        )
    }

    pub(crate) fn fetch_folder_if_exists(&self, uid: &str) -> Result<Option<Map<String, Value>>> {
        match self.request_json(Method::GET, &format!("/api/folders/{uid}"), &[], None) {
            Ok(Some(value)) => {
                let object =
                    value_as_object(&value, &format!("Unexpected folder payload for UID {uid}."))?;
                Ok(Some(object.clone()))
            }
            Ok(None) => Ok(None),
            Err(error) if error.status_code() == Some(404) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn fetch_dashboard(&self, uid: &str) -> Result<Value> {
        match self.request_json(
            Method::GET,
            &format!("/api/dashboards/uid/{uid}"),
            &[],
            None,
        )? {
            Some(value) => {
                let object = value_as_object(
                    &value,
                    &format!("Unexpected dashboard payload for UID {uid}."),
                )?;
                if !object.contains_key("dashboard") {
                    return Err(message(format!(
                        "Unexpected dashboard payload for UID {uid}."
                    )));
                }
                Ok(value)
            }
            None => Err(message(format!(
                "Unexpected empty dashboard payload for UID {uid}."
            ))),
        }
    }

    pub(crate) fn fetch_dashboard_if_exists(&self, uid: &str) -> Result<Option<Value>> {
        match self.fetch_dashboard(uid) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.status_code() == Some(404) => Ok(None),
            Err(error) => Err(error),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn fetch_dashboard_permissions(&self, uid: &str) -> Result<Vec<Map<String, Value>>> {
        let path = format!("/api/dashboards/uid/{uid}/permissions");
        self.expect_object_list(
            self.request_json(Method::GET, &path, &[], None)?,
            &format!("Unexpected dashboard permissions payload for UID {uid}."),
        )
    }

    #[allow(dead_code)]
    pub(crate) fn fetch_folder_permissions(&self, uid: &str) -> Result<Vec<Map<String, Value>>> {
        let path = format!("/api/folders/{uid}/permissions");
        self.expect_object_list(
            self.request_json(Method::GET, &path, &[], None)?,
            &format!("Unexpected folder permissions payload for UID {uid}."),
        )
    }

    pub(crate) fn create_folder_entry(
        &self,
        title: &str,
        uid: &str,
        parent_uid: Option<&str>,
    ) -> Result<Map<String, Value>> {
        let mut payload = Map::new();
        payload.insert("uid".to_string(), Value::String(uid.to_string()));
        payload.insert("title".to_string(), Value::String(title.to_string()));
        if let Some(parent_uid) = parent_uid.filter(|value| !value.is_empty()) {
            payload.insert(
                "parentUid".to_string(),
                Value::String(parent_uid.to_string()),
            );
        }

        match self.request_json(
            Method::POST,
            "/api/folders",
            &[],
            Some(&Value::Object(payload)),
        )? {
            Some(value) => {
                let object = value_as_object(
                    &value,
                    &format!("Unexpected folder create response for UID {uid}."),
                )?;
                Ok(object.clone())
            }
            None => Err(message(format!(
                "Unexpected empty folder create response for UID {uid}."
            ))),
        }
    }

    pub(crate) fn update_folder_request(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        let path = format!("/api/folders/{uid}");
        match self.request_json(
            Method::PUT,
            &path,
            &[],
            Some(&Value::Object(payload.clone())),
        )? {
            Some(value) => {
                let object = value_as_object(
                    &value,
                    &format!("Unexpected folder update response for UID {uid}."),
                )?;
                Ok(object.clone())
            }
            None => Err(message(format!(
                "Unexpected empty folder update response for UID {uid}."
            ))),
        }
    }

    pub(crate) fn import_dashboard_request(&self, payload: &Value) -> Result<Value> {
        match self.request_json(Method::POST, "/api/dashboards/db", &[], Some(payload))? {
            Some(value) => {
                value_as_object(&value, "Unexpected dashboard import response from Grafana.")?;
                Ok(value)
            }
            None => Err(message(
                "Unexpected empty dashboard import response from Grafana.",
            )),
        }
    }

    pub(crate) fn delete_dashboard_request(&self, uid: &str) -> Result<Map<String, Value>> {
        let path = format!("/api/dashboards/uid/{uid}");
        match self.request_json(Method::DELETE, &path, &[], None)? {
            Some(value) => {
                let object = value_as_object(
                    &value,
                    &format!("Unexpected dashboard delete response for UID {uid}."),
                )?;
                Ok(object.clone())
            }
            None => Err(message(format!(
                "Unexpected empty dashboard delete response for UID {uid}."
            ))),
        }
    }

    pub(crate) fn delete_folder_request(&self, uid: &str) -> Result<Map<String, Value>> {
        let path = format!("/api/folders/{uid}");
        match self.request_json(Method::DELETE, &path, &[], None)? {
            Some(value) => {
                let object = value_as_object(
                    &value,
                    &format!("Unexpected folder delete response for UID {uid}."),
                )?;
                Ok(object.clone())
            }
            None => Err(message(format!(
                "Unexpected empty folder delete response for UID {uid}."
            ))),
        }
    }

    pub(crate) fn latest_dashboard_version_timestamp(
        &self,
        dashboard_summaries: &[Map<String, Value>],
    ) -> Option<String> {
        let uid = dashboard_summaries.iter().find_map(|summary| {
            summary
                .get("uid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })?;
        let path = format!("/api/dashboards/uid/{uid}/versions");
        let params = vec![("limit".to_string(), "1".to_string())];
        let response = self
            .request_json(Method::GET, &path, &params, None)
            .ok()
            .flatten()?;
        let versions = match response {
            Value::Array(items) => items,
            Value::Object(object) => object
                .get("versions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        versions
            .first()
            .and_then(Value::as_object)
            .and_then(|object| {
                ["updated", "updatedAt", "modified", "createdAt", "created"]
                    .into_iter()
                    .find_map(|key| object.get(key).and_then(Value::as_str).map(str::trim))
                    .filter(|value| !value.is_empty())
            })
            .map(str::to_string)
    }

    pub(crate) fn list_datasources(&self) -> Result<Vec<Map<String, Value>>> {
        self.expect_object_list(
            self.request_json(Method::GET, "/api/datasources", &[], None)?,
            "Unexpected datasource list response from Grafana.",
        )
    }

    fn expect_object_list(
        &self,
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
}
