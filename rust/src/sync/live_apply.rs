//! Reviewed sync apply execution against live Grafana APIs.
//!
//! Maintainer note:
//! - This module is the last staged/live boundary: it consumes reviewed apply
//!   intent operations and translates them into Grafana requests without
//!   mutating the staged document itself.
//! - `apply_contract.rs` owns parsing the staged contract shape; this file owns
//!   per-kind request mapping, identity fallback, and destructive-operation
//!   guards.
//! - Returned JSON is treated as a separate live execution audit record, not as
//!   new staged contract input.

use crate::alert::{
    build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_rule_import_payload, build_template_import_payload,
};
use crate::common::{message, Result};
use reqwest::Method;
use serde_json::{Map, Value};

use crate::sync::apply_contract::SyncApplyOperation;

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
            // Folder create stays narrow on purpose: the staged operation may
            // carry extra normalized fields, but the live create contract only
            // depends on identity, title, and optional parent linkage.
            let title = desired
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(identity);
            let mut payload = Map::new();
            payload.insert("uid".to_string(), Value::String(identity.to_string()));
            payload.insert("title".to_string(), Value::String(title.to_string()));
            if let Some(parent_uid) = desired
                .get("parentUid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                payload.insert(
                    "parentUid".to_string(),
                    Value::String(parent_uid.to_string()),
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
            // Folder delete is the broadest destructive edge in sync apply, so
            // require an explicit CLI guard instead of inferring intent from
            // prune or review state alone.
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
    // Dashboard apply treats the reviewed identity as the stable UID contract
    // and drops live-only numeric ids so staged artifacts remain portable.
    body.insert("uid".to_string(), Value::String(identity.to_string()));
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
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
        .filter(|value| !value.is_empty())
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
    // Datasource plans may have been keyed by UID or by human name depending on
    // the export path that produced them. Prefer UID resolution, then fall back
    // to name to keep reviewed intents compatible with older staged inputs.
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
        // Preserve the reviewed identity as `uid` when the desired payload did
        // not include it explicitly so the live request still targets the same
        // logical datasource contract.
        body.entry("uid".to_string())
            .or_insert_with(|| Value::String(identity.to_string()));
    }
    let title = body
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
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
            // Grafana datasource mutation APIs are keyed by numeric live id, so
            // the staged identity must first resolve back onto the current live
            // object before the reviewed intent can execute.
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
                .filter(|value| !value.is_empty())
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
                .filter(|value| !value.is_empty())
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
                // Alert rule delete is only safe with a stable UID-backed
                // identity; name-based fallback would be ambiguous once rules
                // drift in live Grafana.
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
                // Policy delete resets the singleton notification policy tree.
                // The outer executor adds a second guard before this path runs.
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
                    // Carry the staged identity forward when the reviewed body
                    // omitted it; live update semantics require a UID-stable
                    // alert rule contract.
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let uid = payload
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
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
                    .filter(|value| !value.is_empty())
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
                    .filter(|value| !value.is_empty())
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

pub(crate) fn execute_live_apply_with_request<F>(
    mut request_json: F,
    operations: &[SyncApplyOperation],
    allow_folder_delete: bool,
    allow_policy_reset: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    // Translate reviewed operations into Grafana requests without mutating the
    // staged plan payload; the returned response stays a separate audit record.
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
                    // Deleting alert-policy is effectively a full policy reset,
                    // so require an explicit live-only escape hatch even after
                    // review and approval.
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
