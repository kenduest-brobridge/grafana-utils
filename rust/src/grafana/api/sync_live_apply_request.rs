use reqwest::Method;
use serde_json::{Map, Value};

use crate::alert::{
    build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_rule_import_payload, build_template_import_payload,
};
use crate::common::Result;
use crate::review_contract::{
    REVIEW_ACTION_WOULD_CREATE, REVIEW_ACTION_WOULD_DELETE, REVIEW_ACTION_WOULD_UPDATE,
};
use crate::sync::live::SyncApplyOperation;

use super::super::sync_live_apply_datasource::{
    resolve_live_datasource_id, resolve_live_datasource_target,
};
use super::super::sync_live_apply_error::{
    alert_sync_delete_requires_uid, alert_sync_live_apply_requires_uid,
    datasource_sync_target_not_resolved, refuse_live_folder_delete, unsupported_alert_sync_action,
    unsupported_alert_sync_kind, unsupported_datasource_sync_action,
    unsupported_folder_sync_action, unsupported_sync_resource_kind,
};
use super::super::sync_live_apply_phase::execute_live_apply_phase;

pub(crate) fn execute_live_apply_with_request<F>(
    mut request_json: F,
    operations: &[SyncApplyOperation],
    allow_folder_delete: bool,
    allow_policy_reset: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    execute_live_apply_phase(operations, allow_policy_reset, |operation| {
        apply_live_operation_with_request(&mut request_json, operation, allow_folder_delete)
    })
}

fn apply_live_operation_with_request<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
    allow_folder_delete: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let kind = operation.kind.as_str();
    match kind {
        "folder" => {
            apply_folder_operation_with_request(request_json, operation, allow_folder_delete)
        }
        "dashboard" => apply_dashboard_operation_with_request(request_json, operation),
        "datasource" => apply_datasource_operation_with_request(request_json, operation),
        "alert"
        | "alert-contact-point"
        | "alert-mute-timing"
        | "alert-policy"
        | "alert-template" => apply_alert_operation_with_request(request_json, operation),
        _ => Err(unsupported_sync_resource_kind(kind)),
    }
}

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
        REVIEW_ACTION_WOULD_CREATE => {
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
        REVIEW_ACTION_WOULD_UPDATE => Ok(request_json(
            Method::PUT,
            &format!("/api/folders/{identity}"),
            &[],
            Some(&Value::Object(desired.clone())),
        )?
        .unwrap_or(Value::Null)),
        REVIEW_ACTION_WOULD_DELETE => {
            if !allow_folder_delete {
                return Err(refuse_live_folder_delete(identity));
            }
            Ok(request_json(
                Method::DELETE,
                &format!("/api/folders/{identity}"),
                &[("forceDeleteRules".to_string(), "false".to_string())],
                None,
            )?
            .unwrap_or(Value::Null))
        }
        _ => Err(unsupported_folder_sync_action(action)),
    }
}

fn apply_dashboard_operation_with_request<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    if action == REVIEW_ACTION_WOULD_DELETE {
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
        Value::Bool(action == REVIEW_ACTION_WOULD_UPDATE),
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
        REVIEW_ACTION_WOULD_CREATE => Ok(request_json(
            Method::POST,
            "/api/datasources",
            &[],
            Some(&Value::Object(body)),
        )?
        .unwrap_or(Value::Null)),
        REVIEW_ACTION_WOULD_UPDATE => {
            let datasources = match request_json(Method::GET, "/api/datasources", &[], None)? {
                Some(Value::Array(items)) => items,
                Some(_) => {
                    return Err(crate::common::message(
                        "Unexpected datasource list response from Grafana.",
                    ))
                }
                None => Vec::new(),
            };
            let datasources = datasources
                .iter()
                .map(|item| {
                    crate::sync::require_json_object(item, "Grafana datasource payload").cloned()
                })
                .collect::<Result<Vec<_>>>()?;
            let target = resolve_live_datasource_target(&datasources, identity)?
                .ok_or_else(|| datasource_sync_target_not_resolved(identity))?;
            let datasource_id = resolve_live_datasource_id(&target, "update")?;
            Ok(request_json(
                Method::PUT,
                &format!("/api/datasources/{datasource_id}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        REVIEW_ACTION_WOULD_DELETE => {
            let datasources = match request_json(Method::GET, "/api/datasources", &[], None)? {
                Some(Value::Array(items)) => items,
                Some(_) => {
                    return Err(crate::common::message(
                        "Unexpected datasource list response from Grafana.",
                    ))
                }
                None => Vec::new(),
            };
            let datasources = datasources
                .iter()
                .map(|item| {
                    crate::sync::require_json_object(item, "Grafana datasource payload").cloned()
                })
                .collect::<Result<Vec<_>>>()?;
            let target = resolve_live_datasource_target(&datasources, identity)?
                .ok_or_else(|| datasource_sync_target_not_resolved(identity))?;
            let datasource_id = resolve_live_datasource_id(&target, "delete")?;
            Ok(request_json(
                Method::DELETE,
                &format!("/api/datasources/{datasource_id}"),
                &[],
                None,
            )?
            .unwrap_or(Value::Null))
        }
        _ => Err(unsupported_datasource_sync_action(action)),
    }
}

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
        REVIEW_ACTION_WOULD_DELETE => match kind {
            "alert" => {
                if identity.is_empty() {
                    return Err(alert_sync_delete_requires_uid());
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
            _ => Err(unsupported_alert_sync_kind(kind)),
        },
        REVIEW_ACTION_WOULD_CREATE | REVIEW_ACTION_WOULD_UPDATE => match kind {
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
                    .ok_or_else(alert_sync_live_apply_requires_uid)?;
                let method = if action == REVIEW_ACTION_WOULD_CREATE {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == REVIEW_ACTION_WOULD_CREATE {
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
                let method = if action == REVIEW_ACTION_WOULD_CREATE {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == REVIEW_ACTION_WOULD_CREATE {
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
                let method = if action == REVIEW_ACTION_WOULD_CREATE {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == REVIEW_ACTION_WOULD_CREATE {
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
            _ => Err(unsupported_alert_sync_kind(kind)),
        },
        _ => Err(unsupported_alert_sync_action(action)),
    }
}
