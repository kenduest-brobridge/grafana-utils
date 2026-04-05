#[cfg(test)]
use reqwest::Method;
use serde_json::{Map, Value};

use crate::alert::{
    build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_rule_import_payload, build_template_import_payload,
};
use crate::common::{message, Result};
use crate::sync::live::SyncApplyOperation;

use super::SyncLiveClient;

impl<'a> SyncLiveClient<'a> {
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
        if let Some(folder_uid) = folder_uid.filter(|value: &&str| !value.is_empty()) {
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

    pub(crate) fn execute_live_apply(
        &self,
        operations: &[SyncApplyOperation],
        allow_folder_delete: bool,
        allow_policy_reset: bool,
    ) -> Result<Value> {
        let mut results = Vec::new();
        for operation in operations {
            let kind = operation.kind.as_str();
            let identity = operation.identity.as_str();
            let action = operation.action.as_str();
            let response = match kind {
                "folder" => apply_folder_operation_with_client(
                    self,
                    operation,
                    allow_folder_delete,
                )?,
                "dashboard" => apply_dashboard_operation_with_client(self, operation)?,
                "datasource" => apply_datasource_operation_with_client(self, operation)?,
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
                    apply_alert_operation_with_client(self, operation)?
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
}

pub(crate) fn execute_live_apply_with_client(
    client: &SyncLiveClient<'_>,
    operations: &[SyncApplyOperation],
    allow_folder_delete: bool,
    allow_policy_reset: bool,
) -> Result<Value> {
    client.execute_live_apply(operations, allow_folder_delete, allow_policy_reset)
}

fn apply_folder_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
    allow_folder_delete: bool,
) -> Result<Value> {
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
            let parent_uid = desired
                .get("parentUid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value: &&str| !value.is_empty());
            Ok(Value::Object(
                client
                    .create_folder(title, identity, parent_uid)?
                    .into_iter()
                    .collect(),
            ))
        }
        "would-update" => Ok(Value::Object(
            client
                .update_folder(identity, desired)?
                .into_iter()
                .collect(),
        )),
        "would-delete" => {
            if !allow_folder_delete {
                return Err(message(format!(
                    "Refusing live folder delete for {identity} without --allow-folder-delete."
                )));
            }
            Ok(client.delete_folder(identity)?)
        }
        _ => Err(message(format!("Unsupported folder sync action {action}."))),
    }
}

fn apply_dashboard_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
) -> Result<Value> {
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    if action == "would-delete" {
        return Ok(client.delete_dashboard(identity)?);
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
    let folder_uid = body
        .get("folderUid")
        .or_else(|| body.get("folderUID"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty());
    client.upsert_dashboard(&body, action == "would-update", folder_uid)
}

fn apply_datasource_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
) -> Result<Value> {
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
        "would-create" => Ok(Value::Object(
            client.create_datasource(&body)?.into_iter().collect(),
        )),
        "would-update" => {
            let target = client.resolve_datasource_target(identity)?.ok_or_else(|| {
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
            Ok(Value::Object(
                client
                    .update_datasource(&datasource_id, &body)?
                    .into_iter()
                    .collect(),
            ))
        }
        "would-delete" => {
            let target = client.resolve_datasource_target(identity)?.ok_or_else(|| {
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
            Ok(client.delete_datasource(&datasource_id)?)
        }
        _ => Err(message(format!(
            "Unsupported datasource sync action {action}."
        ))),
    }
}

fn apply_alert_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
) -> Result<Value> {
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
                Ok(client.delete_alert_rule(identity)?)
            }
            "alert-contact-point" => Ok(client.delete_contact_point(identity)?),
            "alert-mute-timing" => Ok(client.delete_mute_timing(identity)?),
            "alert-template" => Ok(client.delete_template(identity)?),
            "alert-policy" => Ok(client.delete_notification_policies()?),
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
                let response = if action == "would-create" {
                    client.create_alert_rule(&payload)?
                } else {
                    client.update_alert_rule(uid, &payload)?
                };
                Ok(Value::Object(response.into_iter().collect()))
            }
            "alert-contact-point" => {
                let mut payload = build_contact_point_import_payload(desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let response = if action == "would-create" {
                    client.create_contact_point(&payload)?
                } else {
                    client.update_contact_point(identity, &payload)?
                };
                Ok(Value::Object(response.into_iter().collect()))
            }
            "alert-mute-timing" => {
                let payload = build_mute_timing_import_payload(desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(identity);
                let response = if action == "would-create" {
                    client.create_mute_timing(&payload)?
                } else {
                    client.update_mute_timing(name, &payload)?
                };
                Ok(Value::Object(response.into_iter().collect()))
            }
            "alert-policy" => {
                let payload = build_policies_import_payload(desired)?;
                Ok(Value::Object(
                    client
                        .update_notification_policies(&payload)?
                        .into_iter()
                        .collect(),
                ))
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
                Ok(Value::Object(
                    client
                        .update_template(&name, &payload)?
                        .into_iter()
                        .collect(),
                ))
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
