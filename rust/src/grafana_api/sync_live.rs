use reqwest::Method;
use serde_json::{Map, Value};

use super::GrafanaApiClient;
#[cfg(test)]
use crate::alert::{
    build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_rule_import_payload, build_template_import_payload,
};
use crate::common::{message, Result};
#[cfg(test)]
use crate::sync::live::SyncApplyOperation;
use crate::sync::{append_unique_strings, require_json_object};
#[cfg(test)]
use crate::sync::{normalize_alert_managed_fields, normalize_alert_resource_identity_and_title};

#[cfg(test)]
fn build_live_alert_resource_spec(sync_kind: &str, body: Map<String, Value>) -> Result<Value> {
    let (identity, title) = normalize_alert_resource_identity_and_title(sync_kind, &body)?;
    Ok(serde_json::json!({
        "kind": sync_kind,
        "uid": if sync_kind == "alert-contact-point" { identity.clone() } else { String::new() },
        "name": if matches!(sync_kind, "alert-mute-timing" | "alert-template") { identity.clone() } else { String::new() },
        "title": title,
        "managedFields": normalize_alert_managed_fields(&body),
        "body": body,
    }))
}

pub(crate) fn merge_availability(base: Option<Value>, extra: &Value) -> Result<Value> {
    let mut merged = match base {
        Some(Value::Object(object)) => object,
        Some(_) => {
            return Err(message(
                "Sync availability input file must contain a JSON object.",
            ))
        }
        None => Map::new(),
    };
    let extra_object = require_json_object(extra, "Live availability document")?;
    for (key, value) in extra_object {
        if matches!(
            key.as_str(),
            "datasourceUids" | "datasourceNames" | "pluginIds" | "contactPoints"
        ) {
            let existing = merged
                .remove(key)
                .and_then(|item| item.as_array().cloned())
                .unwrap_or_default();
            let mut combined = existing;
            let extra_items = value
                .as_array()
                .ok_or_else(|| message(format!("Live availability field {key} must be a list.")))?;
            let strings = extra_items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            append_unique_strings(&mut combined, &strings);
            merged.insert(key.clone(), Value::Array(combined));
        } else {
            merged.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(merged))
}

pub(crate) struct SyncLiveClient<'a> {
    api: &'a GrafanaApiClient,
}

impl<'a> SyncLiveClient<'a> {
    pub(crate) fn new(api: &'a GrafanaApiClient) -> Self {
        Self { api }
    }

    fn request_json(
        &self,
        method: Method,
        path: &str,
        params: &[(String, String)],
        payload: Option<&Value>,
    ) -> Result<Option<Value>> {
        self.api
            .http_client()
            .request_json(method, path, params, payload)
    }

    pub(crate) fn list_folders(&self) -> Result<Vec<Map<String, Value>>> {
        self.api.dashboard().list_folders()
    }

    pub(crate) fn list_dashboard_summaries(
        &self,
        page_size: usize,
    ) -> Result<Vec<Map<String, Value>>> {
        self.api.dashboard().list_dashboard_summaries(page_size)
    }

    pub(crate) fn fetch_dashboard(&self, uid: &str) -> Result<Value> {
        self.api.dashboard().fetch_dashboard(uid)
    }

    pub(crate) fn list_datasources(&self) -> Result<Vec<Map<String, Value>>> {
        self.api.datasource().list_datasources()
    }

    pub(crate) fn list_plugins(&self) -> Result<Vec<Map<String, Value>>> {
        match self.request_json(Method::GET, "/api/plugins", &[], None)? {
            Some(Value::Array(items)) => items
                .into_iter()
                .map(|item| match item {
                    Value::Object(object) => Ok(object),
                    _ => Err(message("Unexpected plugin list response from Grafana.")),
                })
                .collect(),
            Some(_) => Err(message("Unexpected plugin list response from Grafana.")),
            None => Ok(Vec::new()),
        }
    }

    pub(crate) fn list_alert_rules(&self) -> Result<Vec<Map<String, Value>>> {
        self.api.alerting().list_alert_rules()
    }

    pub(crate) fn list_contact_points(&self) -> Result<Vec<Map<String, Value>>> {
        self.api.alerting().list_contact_points()
    }

    pub(crate) fn list_mute_timings(&self) -> Result<Vec<Map<String, Value>>> {
        self.api.alerting().list_mute_timings()
    }

    pub(crate) fn get_notification_policies(&self) -> Result<Map<String, Value>> {
        self.api.alerting().get_notification_policies()
    }

    pub(crate) fn list_templates(&self) -> Result<Vec<Map<String, Value>>> {
        self.api.alerting().list_templates()
    }

    pub(crate) fn get_template(&self, name: &str) -> Result<Map<String, Value>> {
        self.api.alerting().get_template(name)
    }

    pub(crate) fn create_folder(
        &self,
        title: &str,
        uid: &str,
        parent_uid: Option<&str>,
    ) -> Result<Map<String, Value>> {
        self.api
            .dashboard()
            .create_folder_entry(title, uid, parent_uid)
    }

    pub(crate) fn update_folder(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.dashboard().update_folder_request(uid, payload)
    }

    pub(crate) fn delete_folder(&self, uid: &str) -> Result<Value> {
        Ok(Value::Object(
            self.api.dashboard().delete_folder_request(uid)?.into_iter().collect(),
        ))
    }

    pub(crate) fn upsert_dashboard(
        &self,
        payload: &Map<String, Value>,
        overwrite: bool,
        folder_uid: Option<&str>,
    ) -> Result<Value> {
        let mut body = Map::new();
        body.insert("dashboard".to_string(), Value::Object(payload.clone()));
        body.insert("overwrite".to_string(), Value::Bool(overwrite));
        if let Some(folder_uid) = folder_uid.filter(|value| !value.is_empty()) {
            body.insert(
                "folderUid".to_string(),
                Value::String(folder_uid.to_string()),
            );
        }
        self.api
            .dashboard()
            .import_dashboard_request(&Value::Object(body))
    }

    pub(crate) fn delete_dashboard(&self, uid: &str) -> Result<Value> {
        Ok(Value::Object(
            self.api
                .dashboard()
                .delete_dashboard_request(uid)?
                .into_iter()
                .collect(),
        ))
    }

    pub(crate) fn resolve_datasource_target(
        &self,
        identity: &str,
    ) -> Result<Option<Map<String, Value>>> {
        let datasources = self.list_datasources()?;
        for datasource in &datasources {
            if datasource.get("uid").and_then(Value::as_str).map(str::trim) == Some(identity) {
                return Ok(Some(datasource.clone()));
            }
        }
        for datasource in &datasources {
            if datasource
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                == Some(identity)
            {
                return Ok(Some(datasource.clone()));
            }
        }
        Ok(None)
    }

    pub(crate) fn create_datasource(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.datasource().create_datasource(payload)
    }

    pub(crate) fn update_datasource(
        &self,
        datasource_id: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.datasource().update_datasource(datasource_id, payload)
    }

    pub(crate) fn delete_datasource(&self, datasource_id: &str) -> Result<Value> {
        self.api.datasource().delete_datasource(datasource_id)
    }

    pub(crate) fn create_alert_rule(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().create_alert_rule(payload)
    }

    pub(crate) fn update_alert_rule(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_alert_rule(uid, payload)
    }

    pub(crate) fn delete_alert_rule(&self, uid: &str) -> Result<Value> {
        self.api.alerting().delete_alert_rule(uid)
    }

    pub(crate) fn create_contact_point(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().create_contact_point(payload)
    }

    pub(crate) fn update_contact_point(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_contact_point(uid, payload)
    }

    pub(crate) fn delete_contact_point(&self, uid: &str) -> Result<Value> {
        self.api.alerting().delete_contact_point(uid)
    }

    pub(crate) fn create_mute_timing(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().create_mute_timing(payload)
    }

    pub(crate) fn update_mute_timing(
        &self,
        name: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_mute_timing(name, payload)
    }

    pub(crate) fn delete_mute_timing(&self, name: &str) -> Result<Value> {
        self.api.alerting().delete_mute_timing(name)
    }

    pub(crate) fn update_notification_policies(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_notification_policies(payload)
    }

    pub(crate) fn delete_notification_policies(&self) -> Result<Value> {
        self.api.alerting().delete_notification_policies()
    }

    pub(crate) fn update_template(
        &self,
        name: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_template(name, payload)
    }

    pub(crate) fn delete_template(&self, name: &str) -> Result<Value> {
        self.api.alerting().delete_template(name)
    }
}

#[cfg(test)]
pub(crate) fn fetch_live_resource_specs_with_request<F>(
    mut request_json: F,
    page_size: usize,
) -> Result<Vec<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut specs = Vec::new();
    match request_json(Method::GET, "/api/folders", &[], None)? {
        Some(Value::Array(folders)) => {
            for folder in folders {
                let object = require_json_object(&folder, "Grafana folder payload")?;
                let uid = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                if uid.is_empty() {
                    continue;
                }
                let title = object
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(uid);
                let mut body = Map::new();
                body.insert("title".to_string(), Value::String(title.to_string()));
                if let Some(parent_uid) = object
                    .get("parentUid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                {
                    body.insert(
                        "parentUid".to_string(),
                        Value::String(parent_uid.to_string()),
                    );
                }
                specs.push(serde_json::json!({
                    "kind": "folder",
                    "uid": uid,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected folder list response from Grafana.")),
        None => {}
    }

    let mut page = 1usize;
    loop {
        let params = vec![
            ("type".to_string(), "dash-db".to_string()),
            ("limit".to_string(), page_size.to_string()),
            ("page".to_string(), page.to_string()),
        ];
        let batch = match request_json(Method::GET, "/api/search", &params, None)? {
            Some(Value::Array(items)) => items,
            Some(_) => return Err(message("Unexpected search response from Grafana.")),
            None => Vec::new(),
        };
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        for item in batch {
            let summary = require_json_object(&item, "Grafana dashboard summary")?;
            let uid = summary
                .get("uid")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if uid.is_empty() {
                continue;
            }
            let dashboard_wrapper = match request_json(
                Method::GET,
                &format!("/api/dashboards/uid/{uid}"),
                &[],
                None,
            )? {
                Some(value) => value,
                None => continue,
            };
            let wrapper = require_json_object(&dashboard_wrapper, "Grafana dashboard payload")?;
            let dashboard = wrapper
                .get("dashboard")
                .ok_or_else(|| message(format!("Unexpected dashboard payload for UID {uid}.")))?;
            let body = require_json_object(dashboard, "Grafana dashboard body")?;
            let mut normalized = body.clone();
            normalized.remove("id");
            let title = normalized
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value: &&str| !value.is_empty())
                .unwrap_or(uid);
            specs.push(serde_json::json!({
                "kind": "dashboard",
                "uid": uid,
                "title": title,
                "body": normalized,
            }));
        }
        if batch_len < page_size {
            break;
        }
        page += 1;
    }

    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(datasources)) => {
            for datasource in datasources {
                let object = require_json_object(&datasource, "Grafana datasource payload")?;
                let uid = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                let name = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                if uid.is_empty() && name.is_empty() {
                    continue;
                }
                let title = if name.is_empty() { uid } else { name };
                let mut body = Map::new();
                body.insert("uid".to_string(), Value::String(uid.to_string()));
                body.insert("name".to_string(), Value::String(title.to_string()));
                body.insert(
                    "type".to_string(),
                    object
                        .get("type")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "access".to_string(),
                    object
                        .get("access")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "url".to_string(),
                    object
                        .get("url")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "isDefault".to_string(),
                    object
                        .get("isDefault")
                        .cloned()
                        .unwrap_or(Value::Bool(false)),
                );
                if let Some(json_data) = object.get("jsonData").and_then(Value::as_object) {
                    if !json_data.is_empty() {
                        body.insert("jsonData".to_string(), Value::Object(json_data.clone()));
                    }
                }
                specs.push(serde_json::json!({
                    "kind": "datasource",
                    "uid": uid,
                    "name": title,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/alert-rules", &[], None)? {
        Some(Value::Array(rules)) => {
            for rule in rules {
                let object = require_json_object(&rule, "Grafana alert-rule payload")?;
                let body = crate::alert::build_rule_import_payload(object)?;
                let uid = body
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .ok_or_else(|| message("Live alert rule payload is missing uid."))?;
                let title = body
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(uid);
                specs.push(serde_json::json!({
                    "kind": "alert",
                    "uid": uid,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected alert-rule list response from Grafana.")),
        None => {}
    }

    match request_json(
        Method::GET,
        "/api/v1/provisioning/contact-points",
        &[],
        None,
    )? {
        Some(Value::Array(contact_points)) => {
            for contact_point in contact_points {
                let object = require_json_object(&contact_point, "Grafana contact-point payload")?;
                specs.push(build_live_alert_resource_spec(
                    "alert-contact-point",
                    object.clone(),
                )?);
            }
        }
        Some(_) => {
            return Err(message(
                "Unexpected contact-point list response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/mute-timings", &[], None)? {
        Some(Value::Array(mute_timings)) => {
            for mute_timing in mute_timings {
                let object = require_json_object(&mute_timing, "Grafana mute-timing payload")?;
                specs.push(build_live_alert_resource_spec(
                    "alert-mute-timing",
                    object.clone(),
                )?);
            }
        }
        Some(_) => {
            return Err(message(
                "Unexpected mute-timing list response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/policies", &[], None)? {
        Some(Value::Object(policies)) => {
            specs.push(build_live_alert_resource_spec(
                "alert-policy",
                policies.clone(),
            )?);
        }
        Some(_) => {
            return Err(message(
                "Unexpected notification policy response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/templates", &[], None)? {
        Some(Value::Array(templates)) => {
            for template in templates {
                let object = require_json_object(&template, "Grafana template summary payload")?;
                let name = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .ok_or_else(|| message("Live template payload is missing name."))?;
                let template_payload = match request_json(
                    Method::GET,
                    &format!("/api/v1/provisioning/templates/{name}"),
                    &[],
                    None,
                )? {
                    Some(Value::Object(template_object)) => template_object,
                    Some(_) => return Err(message("Unexpected template payload from Grafana.")),
                    None => continue,
                };
                specs.push(build_live_alert_resource_spec(
                    "alert-template",
                    template_payload,
                )?);
            }
        }
        Some(Value::Null) => {}
        Some(_) => return Err(message("Unexpected template list response from Grafana.")),
        None => {}
    }

    Ok(specs)
}

#[cfg(test)]
pub(crate) fn fetch_live_availability_with_request<F>(mut request_json: F) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut availability = Map::from_iter(vec![
        ("datasourceUids".to_string(), Value::Array(Vec::new())),
        ("datasourceNames".to_string(), Value::Array(Vec::new())),
        ("pluginIds".to_string(), Value::Array(Vec::new())),
        ("contactPoints".to_string(), Value::Array(Vec::new())),
    ]);

    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(datasources)) => {
            let mut uids = Vec::new();
            let mut names = Vec::new();
            for datasource in datasources {
                let object = require_json_object(&datasource, "Grafana datasource payload")?;
                if let Some(uid) = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                {
                    uids.push(uid.to_string());
                }
                if let Some(name) = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                {
                    names.push(name.to_string());
                }
            }
            append_unique_strings(
                availability
                    .get_mut("datasourceUids")
                    .and_then(Value::as_array_mut)
                    .expect("datasourceUids should be array"),
                &uids,
            );
            append_unique_strings(
                availability
                    .get_mut("datasourceNames")
                    .and_then(Value::as_array_mut)
                    .expect("datasourceNames should be array"),
                &names,
            );
        }
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => {}
    }

    match request_json(Method::GET, "/api/plugins", &[], None)? {
        Some(Value::Array(plugins)) => {
            let ids = plugins
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|plugin| plugin.get("id").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value: &&str| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            append_unique_strings(
                availability
                    .get_mut("pluginIds")
                    .and_then(Value::as_array_mut)
                    .expect("pluginIds should be array"),
                &ids,
            );
        }
        Some(_) => return Err(message("Unexpected plugin list response from Grafana.")),
        None => {}
    }

    match request_json(
        Method::GET,
        "/api/v1/provisioning/contact-points",
        &[],
        None,
    )? {
        Some(Value::Array(contact_points)) => {
            let mut names = Vec::new();
            for item in contact_points {
                let object = require_json_object(&item, "Grafana contact-point payload")?;
                if let Some(name) = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                {
                    names.push(name.to_string());
                }
                if let Some(uid) = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                {
                    names.push(uid.to_string());
                }
            }
            append_unique_strings(
                availability
                    .get_mut("contactPoints")
                    .and_then(Value::as_array_mut)
                    .expect("contactPoints should be array"),
                &names,
            );
        }
        Some(_) => {
            return Err(message(
                "Unexpected contact-point list response from Grafana.",
            ))
        }
        None => {}
    }

    Ok(Value::Object(availability))
}

#[cfg(test)]
fn resolve_live_datasource_target_with_request<F>(
    request_json: &mut F,
    identity: &str,
) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let datasources = match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(items)) => items,
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => Vec::new(),
    };
    for datasource in &datasources {
        let object = crate::sync::require_json_object(datasource, "Grafana datasource payload")?;
        if object.get("uid").and_then(Value::as_str).map(str::trim) == Some(identity) {
            return Ok(Some(object.clone()));
        }
    }
    for datasource in &datasources {
        let object = crate::sync::require_json_object(datasource, "Grafana datasource payload")?;
        if object.get("name").and_then(Value::as_str).map(str::trim) == Some(identity) {
            return Ok(Some(object.clone()));
        }
    }
    Ok(None)
}

#[cfg(test)]
fn apply_folder_operation_with_request<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
    allow_folder_delete: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    let desired = &operation.desired;
    match action {
        "would-create" => {
            let title = desired
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value: &&str| !value.is_empty())
                .unwrap_or(identity);
            let mut payload = Map::new();
            payload.insert("uid".to_string(), Value::String(identity.to_string()));
            payload.insert("title".to_string(), Value::String(title.to_string()));
            if let Some(parent_uid) = desired
                .get("parentUid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value: &&str| !value.is_empty())
            {
                payload.insert(
                    "parentUid".to_string(),
                    Value::String((*parent_uid).to_string()),
                );
            }
            Ok(request_json(
                Method::POST,
                "/api/folders",
                &[],
                Some(&Value::Object(payload)),
            )?
            .unwrap_or(Value::Null))
        }
        "would-update" => Ok(request_json(
            Method::PUT,
            &format!("/api/folders/{identity}"),
            &[],
            Some(&Value::Object(desired.clone())),
        )?
        .unwrap_or(Value::Null)),
        "would-delete" => {
            if !allow_folder_delete {
                return Err(message(format!(
                    "Refusing live folder delete for {identity} without --allow-folder-delete."
                )));
            }
            Ok(request_json(
                Method::DELETE,
                &format!("/api/folders/{identity}"),
                &[("forceDeleteRules".to_string(), "false".to_string())],
                None,
            )?
            .unwrap_or(Value::Null))
        }
        _ => Err(message(format!("Unsupported folder sync action {action}."))),
    }
}

#[cfg(test)]
fn apply_dashboard_operation_with_request<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    if action == "would-delete" {
        return Ok(request_json(
            Method::DELETE,
            &format!("/api/dashboards/uid/{identity}"),
            &[],
            None,
        )?
        .unwrap_or(Value::Null));
    }
    let mut body = operation.desired.clone();
    body.insert("uid".to_string(), Value::String(identity.to_string()));
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .unwrap_or(identity);
    body.insert("title".to_string(), Value::String(title.to_string()));
    body.remove("id");
    let mut payload = Map::new();
    payload.insert("dashboard".to_string(), Value::Object(body.clone()));
    payload.insert(
        "overwrite".to_string(),
        Value::Bool(action == "would-update"),
    );
    if let Some(folder_uid) = body
        .get("folderUid")
        .or_else(|| body.get("folderUID"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
    {
        payload.insert(
            "folderUid".to_string(),
            Value::String(folder_uid.to_string()),
        );
    }
    Ok(request_json(
        Method::POST,
        "/api/dashboards/db",
        &[],
        Some(&Value::Object(payload)),
    )?
    .unwrap_or(Value::Null))
}

#[cfg(test)]
fn apply_datasource_operation_with_request<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    let mut body = operation.desired.clone();
    if !identity.is_empty() {
        body.entry("uid".to_string())
            .or_insert_with(|| Value::String(identity.to_string()));
    }
    let title = body
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .unwrap_or(identity);
    body.insert("name".to_string(), Value::String(title.to_string()));
    match action {
        "would-create" => Ok(request_json(
            Method::POST,
            "/api/datasources",
            &[],
            Some(&Value::Object(body)),
        )?
        .unwrap_or(Value::Null)),
        "would-update" => {
            let target = resolve_live_datasource_target_with_request(request_json, identity)?
                .ok_or_else(|| {
                    message(format!(
                        "Could not resolve live datasource target {identity} during sync apply."
                    ))
                })?;
            let datasource_id = target
                .get("id")
                .map(|value| match value {
                    Value::String(text) => text.clone(),
                    _ => value.to_string(),
                })
                .filter(|value: &String| !value.is_empty())
                .ok_or_else(|| message("Datasource sync update requires a live datasource id."))?;
            Ok(request_json(
                Method::PUT,
                &format!("/api/datasources/{datasource_id}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        "would-delete" => {
            let target = resolve_live_datasource_target_with_request(request_json, identity)?
                .ok_or_else(|| {
                    message(format!(
                        "Could not resolve live datasource target {identity} during sync apply."
                    ))
                })?;
            let datasource_id = target
                .get("id")
                .map(|value| match value {
                    Value::String(text) => text.clone(),
                    _ => value.to_string(),
                })
                .filter(|value: &String| !value.is_empty())
                .ok_or_else(|| message("Datasource sync delete requires a live datasource id."))?;
            Ok(request_json(
                Method::DELETE,
                &format!("/api/datasources/{datasource_id}"),
                &[],
                None,
            )?
            .unwrap_or(Value::Null))
        }
        _ => Err(message(format!(
            "Unsupported datasource sync action {action}."
        ))),
    }
}

#[cfg(test)]
fn apply_alert_operation_with_request<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let kind = operation.kind.as_str();
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    let desired = &operation.desired;
    match action {
        "would-delete" => match kind {
            "alert" => {
                if identity.is_empty() {
                    return Err(message(
                        "Alert sync delete requires a stable uid identity for live apply.",
                    ));
                }
                Ok(request_json(
                    Method::DELETE,
                    &format!("/api/v1/provisioning/alert-rules/{identity}"),
                    &[],
                    None,
                )?
                .unwrap_or(Value::Null))
            }
            "alert-contact-point" => Ok(request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/contact-points/{identity}"),
                &[],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-mute-timing" => Ok(request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/mute-timings/{identity}"),
                &[("version".to_string(), String::new())],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-template" => Ok(request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/templates/{identity}"),
                &[("version".to_string(), String::new())],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-policy" => {
                Ok(
                    request_json(Method::DELETE, "/api/v1/provisioning/policies", &[], None)?
                        .unwrap_or(Value::Null),
                )
            }
            _ => Err(message(format!("Unsupported alert sync kind {kind}."))),
        },
        "would-create" | "would-update" => match kind {
            "alert" => {
                let mut payload = build_rule_import_payload(desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let uid = payload
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .ok_or_else(|| {
                        message("Alert sync live apply requires alert rule payloads with a uid.")
                    })?;
                let method = if action == "would-create" {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == "would-create" {
                    "/api/v1/provisioning/alert-rules".to_string()
                } else {
                    format!("/api/v1/provisioning/alert-rules/{uid}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-contact-point" => {
                let mut payload = build_contact_point_import_payload(desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let method = if action == "would-create" {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == "would-create" {
                    "/api/v1/provisioning/contact-points".to_string()
                } else {
                    format!("/api/v1/provisioning/contact-points/{identity}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-mute-timing" => {
                let payload = build_mute_timing_import_payload(desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(identity);
                let method = if action == "would-create" {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == "would-create" {
                    "/api/v1/provisioning/mute-timings".to_string()
                } else {
                    format!("/api/v1/provisioning/mute-timings/{name}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-policy" => {
                let payload = build_policies_import_payload(desired)?;
                Ok(request_json(
                    Method::PUT,
                    "/api/v1/provisioning/policies",
                    &[],
                    Some(&Value::Object(payload)),
                )?
                .unwrap_or(Value::Null))
            }
            "alert-template" => {
                let mut payload = build_template_import_payload(desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(identity)
                    .to_string();
                payload.remove("name");
                Ok(request_json(
                    Method::PUT,
                    &format!("/api/v1/provisioning/templates/{name}"),
                    &[],
                    Some(&Value::Object(payload)),
                )?
                .unwrap_or(Value::Null))
            }
            _ => Err(message(format!("Unsupported alert sync kind {kind}."))),
        },
        _ => Err(message(format!("Unsupported alert sync action {action}."))),
    }
}

#[cfg(test)]
pub(crate) fn execute_live_apply_with_request<F>(
    mut request_json: F,
    operations: &[SyncApplyOperation],
    allow_folder_delete: bool,
    allow_policy_reset: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut results = Vec::new();
    for operation in operations {
        let kind = operation.kind.as_str();
        let identity = operation.identity.as_str();
        let action = operation.action.as_str();
        let response = match kind {
            "folder" => apply_folder_operation_with_request(
                &mut request_json,
                operation,
                allow_folder_delete,
            )?,
            "dashboard" => apply_dashboard_operation_with_request(&mut request_json, operation)?,
            "datasource" => apply_datasource_operation_with_request(&mut request_json, operation)?,
            "alert"
            | "alert-contact-point"
            | "alert-mute-timing"
            | "alert-policy"
            | "alert-template" => {
                if operation.kind == "alert-policy"
                    && operation.action == "would-delete"
                    && !allow_policy_reset
                {
                    return Err(message(
                        "Refusing live notification policy reset without --allow-policy-reset.",
                    ));
                }
                apply_alert_operation_with_request(&mut request_json, operation)?
            }
            _ => return Err(message(format!("Unsupported sync resource kind {kind}."))),
        };
        results.push(serde_json::json!({
            "kind": kind,
            "identity": identity,
            "action": action,
            "response": response,
        }));
    }
    Ok(serde_json::json!({
        "mode": "live-apply",
        "appliedCount": results.len(),
        "results": results,
    }))
}
