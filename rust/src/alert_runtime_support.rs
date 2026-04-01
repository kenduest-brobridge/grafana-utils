//! Build alert compare/import/delete plan documents from live Grafana payloads.
//!
//! Responsibilities:
//! - Gather and normalize alert resources through shared request helpers.
//! - Produce plan and delete-preview documents used by diff/import execution flows.
//! - Preserve request semantics so CLI/runtime callers receive a stable sync-ready
//!   shape across execution paths.

use crate::common::{message, tool_version, value_as_object, Result};
use reqwest::Method;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::alert_support::{
    build_contact_point_scaffold_document, build_managed_policy_route_preview,
    normalize_compare_payload, remove_managed_policy_subtree, upsert_managed_policy_subtree,
};
use super::{
    build_compare_document, build_contact_point_import_payload, build_import_operation,
    build_mute_timing_import_payload, build_new_contact_point_scaffold_document,
    build_new_rule_scaffold_document, build_new_template_scaffold_document,
    build_policies_import_payload, build_resource_identity, build_rule_import_payload,
    build_template_import_payload, discover_alert_resource_files, init_alert_managed_dir,
    load_alert_resource_file, parse_template_list_response, resource_subdir_by_kind,
    strip_server_managed_fields, write_alert_resource_file, CONTACT_POINT_KIND, MUTE_TIMING_KIND,
    POLICIES_KIND, RULE_KIND, TEMPLATE_KIND,
};

pub const ALERT_PLAN_KIND: &str = "grafana-util-alert-plan";
pub const ALERT_PLAN_SCHEMA_VERSION: i64 = 1;
pub const ALERT_DELETE_PREVIEW_KIND: &str = "grafana-util-alert-delete-preview";
pub const ALERT_DELETE_PREVIEW_SCHEMA_VERSION: i64 = 1;

fn row_object<'a>(row: &'a Value, label: &str) -> Result<&'a Map<String, Value>> {
    value_as_object(row, label)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[allow(dead_code)]
#[cfg(test)]
pub(crate) fn request_object_with_request<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    payload: Option<&Value>,
    error_message: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let value = request_json(method, path, &[], payload)?
        .ok_or_else(|| message(error_message.to_string()))?;
    Ok(value_as_object(&value, error_message)?.clone())
}

pub(crate) fn request_array_with_request<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    payload: Option<&Value>,
    error_message: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    super::expect_object_list(request_json(method, path, &[], payload)?, error_message)
}

pub(crate) fn request_optional_object_with_request<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    payload: Option<&Value>,
) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let value = match request_json(method, path, &[], payload) {
        Ok(value) => value,
        Err(error) if error.status_code() == Some(404) => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(value) = value else {
        return Ok(None);
    };
    Ok(Some(
        value_as_object(&value, "Unexpected alert request object response.")?.clone(),
    ))
}

fn request_template_list_with_request<F>(request_json: &mut F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    parse_template_list_response(request_json(
        Method::GET,
        "/api/v1/provisioning/templates",
        &[],
        None,
    )?)
}

pub(crate) fn fetch_live_compare_document_with_request<F>(
    mut request_json: F,
    kind: &str,
    payload: &Map<String, Value>,
) -> Result<Option<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if uid.is_empty() {
                return Ok(None);
            }
            Ok(request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                None,
            )?
            .map(|remote| build_compare_document(kind, &normalize_compare_payload(kind, &remote))))
        }
        CONTACT_POINT_KIND => {
            let uid = super::string_field(payload, "uid", "");
            let remote = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/contact-points",
                None,
                "Unexpected contact-point list response from Grafana.",
            )?
            .into_iter()
            .find(|item| super::string_field(item, "uid", "") == uid);
            Ok(remote
                .map(|item| build_compare_document(kind, &normalize_compare_payload(kind, &item))))
        }
        MUTE_TIMING_KIND => {
            let name = super::string_field(payload, "name", "");
            let remote = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/mute-timings",
                None,
                "Unexpected mute-timing list response from Grafana.",
            )?
            .into_iter()
            .find(|item| super::string_field(item, "name", "") == name);
            Ok(remote
                .map(|item| build_compare_document(kind, &normalize_compare_payload(kind, &item))))
        }
        TEMPLATE_KIND => {
            let name = super::string_field(payload, "name", "");
            Ok(request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?
            .map(|remote| build_compare_document(kind, &normalize_compare_payload(kind, &remote))))
        }
        POLICIES_KIND => Ok(request_optional_object_with_request(
            &mut request_json,
            Method::GET,
            "/api/v1/provisioning/policies",
            None,
        )?
        .map(|remote| build_compare_document(kind, &normalize_compare_payload(kind, &remote)))),
        _ => unreachable!(),
    }
}

#[allow(dead_code)]
#[cfg(test)]
pub(crate) fn determine_import_action_with_request<F>(
    mut request_json: F,
    kind: &str,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if uid.is_empty() {
                return Ok("would-create");
            }
            if request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                None,
            )?
            .is_some()
            {
                if replace_existing {
                    Ok("would-update")
                } else {
                    Ok("would-fail-existing")
                }
            } else {
                Ok("would-create")
            }
        }
        CONTACT_POINT_KIND => {
            let uid = super::string_field(payload, "uid", "");
            let exists = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/contact-points",
                None,
                "Unexpected contact-point list response from Grafana.",
            )?
            .into_iter()
            .any(|item| super::string_field(&item, "uid", "") == uid);
            if exists {
                if replace_existing {
                    Ok("would-update")
                } else {
                    Ok("would-fail-existing")
                }
            } else {
                Ok("would-create")
            }
        }
        MUTE_TIMING_KIND => {
            let name = super::string_field(payload, "name", "");
            let exists = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/mute-timings",
                None,
                "Unexpected mute-timing list response from Grafana.",
            )?
            .into_iter()
            .any(|item| super::string_field(&item, "name", "") == name);
            if exists {
                if replace_existing {
                    Ok("would-update")
                } else {
                    Ok("would-fail-existing")
                }
            } else {
                Ok("would-create")
            }
        }
        TEMPLATE_KIND => {
            let name = super::string_field(payload, "name", "");
            let exists = request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?
            .is_some();
            if exists {
                if replace_existing {
                    Ok("would-update")
                } else {
                    Ok("would-fail-existing")
                }
            } else {
                Ok("would-create")
            }
        }
        POLICIES_KIND => Ok("would-update"),
        _ => unreachable!(),
    }
}

#[allow(dead_code)]
#[cfg(test)]
pub(crate) fn import_resource_document_with_request<F>(
    mut request_json: F,
    kind: &str,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if replace_existing
                && !uid.is_empty()
                && request_optional_object_with_request(
                    &mut request_json,
                    Method::GET,
                    &format!("/api/v1/provisioning/alert-rules/{uid}"),
                    None,
                )?
                .is_some()
            {
                let result = request_object_with_request(
                    &mut request_json,
                    Method::PUT,
                    &format!("/api/v1/provisioning/alert-rules/{uid}"),
                    Some(&Value::Object(payload.clone())),
                    "Unexpected alert-rule update response from Grafana.",
                )?;
                return Ok((
                    "updated".to_string(),
                    super::string_field(&result, "uid", &uid),
                ));
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/alert-rules",
                Some(&Value::Object(payload.clone())),
                "Unexpected alert-rule create response from Grafana.",
            )?;
            Ok((
                "created".to_string(),
                super::string_field(&result, "uid", &uid),
            ))
        }
        CONTACT_POINT_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if replace_existing && !uid.is_empty() {
                let existing: Vec<String> = request_array_with_request(
                    &mut request_json,
                    Method::GET,
                    "/api/v1/provisioning/contact-points",
                    None,
                    "Unexpected contact-point list response from Grafana.",
                )?
                .into_iter()
                .map(|item| super::string_field(&item, "uid", ""))
                .collect();
                if existing.iter().any(|item| item == &uid) {
                    let result = request_object_with_request(
                        &mut request_json,
                        Method::PUT,
                        &format!("/api/v1/provisioning/contact-points/{uid}"),
                        Some(&Value::Object(payload.clone())),
                        "Unexpected contact-point update response from Grafana.",
                    )?;
                    return Ok((
                        "updated".to_string(),
                        super::string_field(&result, "uid", &uid),
                    ));
                }
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/contact-points",
                Some(&Value::Object(payload.clone())),
                "Unexpected contact-point create response from Grafana.",
            )?;
            Ok((
                "created".to_string(),
                super::string_field(&result, "uid", &uid),
            ))
        }
        MUTE_TIMING_KIND => {
            let name = super::string_field(payload, "name", "");
            if replace_existing && !name.is_empty() {
                let existing: Vec<String> = request_array_with_request(
                    &mut request_json,
                    Method::GET,
                    "/api/v1/provisioning/mute-timings",
                    None,
                    "Unexpected mute-timing list response from Grafana.",
                )?
                .into_iter()
                .map(|item| super::string_field(&item, "name", ""))
                .collect();
                if existing.iter().any(|item| item == &name) {
                    let result = request_object_with_request(
                        &mut request_json,
                        Method::PUT,
                        &format!("/api/v1/provisioning/mute-timings/{name}"),
                        Some(&Value::Object(payload.clone())),
                        "Unexpected mute-timing update response from Grafana.",
                    )?;
                    return Ok((
                        "updated".to_string(),
                        super::string_field(&result, "name", &name),
                    ));
                }
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/mute-timings",
                Some(&Value::Object(payload.clone())),
                "Unexpected mute-timing create response from Grafana.",
            )?;
            Ok((
                "created".to_string(),
                super::string_field(&result, "name", &name),
            ))
        }
        TEMPLATE_KIND => {
            let name = super::string_field(payload, "name", "");
            let existing = request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?;
            if existing.is_some() && !replace_existing {
                return Err(message(format!(
                    "Template {name:?} already exists. Use --replace-existing."
                )));
            }
            let mut template_payload = payload.clone();
            if let Some(current) = existing {
                template_payload.insert(
                    "version".to_string(),
                    Value::String(super::string_field(&current, "version", "")),
                );
            } else {
                template_payload.insert("version".to_string(), Value::String(String::new()));
            }
            let mut body = template_payload.clone();
            body.remove("name");
            let result = request_object_with_request(
                &mut request_json,
                Method::PUT,
                &format!("/api/v1/provisioning/templates/{name}"),
                Some(&Value::Object(body)),
                "Unexpected template update response from Grafana.",
            )?;
            Ok((
                (if template_payload
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .is_empty()
                {
                    "created"
                } else {
                    "updated"
                })
                .to_string(),
                super::string_field(&result, "name", &name),
            ))
        }
        POLICIES_KIND => {
            let result = request_object_with_request(
                &mut request_json,
                Method::PUT,
                "/api/v1/provisioning/policies",
                Some(&Value::Object(payload.clone())),
                "Unexpected notification policy update response from Grafana.",
            )?;
            Ok((
                "updated".to_string(),
                super::string_field(&result, "receiver", "root"),
            ))
        }
        _ => unreachable!(),
    }
}

fn request_live_resources_by_kind_with_request<F>(
    request_json: &mut F,
    kind: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => request_array_with_request(
            request_json,
            Method::GET,
            "/api/v1/provisioning/alert-rules",
            None,
            "Unexpected alert-rule list response from Grafana.",
        ),
        CONTACT_POINT_KIND => request_array_with_request(
            request_json,
            Method::GET,
            "/api/v1/provisioning/contact-points",
            None,
            "Unexpected contact-point list response from Grafana.",
        ),
        MUTE_TIMING_KIND => request_array_with_request(
            request_json,
            Method::GET,
            "/api/v1/provisioning/mute-timings",
            None,
            "Unexpected mute-timing list response from Grafana.",
        ),
        TEMPLATE_KIND => request_template_list_with_request(request_json),
        POLICIES_KIND => Ok(request_optional_object_with_request(
            request_json,
            Method::GET,
            "/api/v1/provisioning/policies",
            None,
        )?
        .into_iter()
        .collect()),
        _ => unreachable!(),
    }
}

fn plan_summary(rows: &[Value]) -> Value {
    let count = |action: &str| {
        rows.iter()
            .filter(|row| row.get("action").and_then(Value::as_str) == Some(action))
            .count()
    };
    json!({
        "processed": rows.len(),
        "create": count("create"),
        "update": count("update"),
        "noop": count("noop"),
        "delete": count("delete"),
        "blocked": count("blocked"),
    })
}

pub fn build_alert_plan_document(rows: &[Value], allow_prune: bool) -> Value {
    json!({
        "kind": ALERT_PLAN_KIND,
        "schemaVersion": ALERT_PLAN_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "reviewRequired": true,
        "reviewed": false,
        "allowPrune": allow_prune,
        "summary": plan_summary(rows),
        "rows": rows,
    })
}

pub fn build_alert_delete_preview_document(rows: &[Value], allow_policy_reset: bool) -> Value {
    let count = |action: &str| {
        rows.iter()
            .filter(|row| row.get("action").and_then(Value::as_str) == Some(action))
            .count()
    };
    json!({
        "kind": ALERT_DELETE_PREVIEW_KIND,
        "schemaVersion": ALERT_DELETE_PREVIEW_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "allowPolicyReset": allow_policy_reset,
        "summary": {
            "processed": rows.len(),
            "delete": count("delete"),
            "blocked": count("blocked"),
        },
        "rows": rows,
    })
}

pub fn load_alert_desired_operations(
    dir: &Path,
) -> Result<Vec<(PathBuf, String, Map<String, Value>)>> {
    let resource_files = discover_alert_resource_files(dir)?;
    let mut operations = Vec::new();
    for path in resource_files {
        let document = load_alert_resource_file(&path, "Alerting resource")?;
        let (kind, payload) = build_import_operation(&document)?;
        operations.push((path, kind, payload));
    }
    Ok(operations)
}

pub fn build_alert_plan_with_request<F>(
    mut request_json: F,
    desired_dir: &Path,
    allow_prune: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let desired_operations = load_alert_desired_operations(desired_dir)?;
    let mut rows = Vec::new();
    let mut desired_keys = BTreeSet::new();

    for (path, kind, payload) in desired_operations {
        let identity = build_resource_identity(&kind, &payload);
        let key = (kind.clone(), identity.clone());
        if !desired_keys.insert(key.clone()) {
            return Err(message(format!(
                "Duplicate alert desired identity detected for kind={} id={}.",
                kind, identity
            )));
        }
        let desired_compare =
            build_compare_document(&kind, &normalize_compare_payload(&kind, &payload));
        let live_compare =
            fetch_live_compare_document_with_request(&mut request_json, &kind, &payload)?;
        let action = match live_compare.as_ref() {
            None => "create",
            Some(live) if live == &desired_compare => "noop",
            Some(_) => "update",
        };
        rows.push(json!({
            "path": path_string(&path),
            "kind": kind,
            "identity": identity,
            "action": action,
            "reason": match action {
                "create" => "missing-live",
                "noop" => "in-sync",
                "update" => "drift-detected",
                _ => unreachable!(),
            },
            "desired": Value::Object(payload),
            "live": live_compare.unwrap_or(Value::Null),
        }));
    }

    for kind in resource_subdir_by_kind().keys() {
        let mut live_items = request_live_resources_by_kind_with_request(&mut request_json, kind)?
            .into_iter()
            .map(|item| {
                let payload = strip_server_managed_fields(kind, &item);
                let identity = build_resource_identity(kind, &payload);
                (identity, payload)
            })
            .collect::<Vec<(String, Map<String, Value>)>>();
        live_items.sort_by(|left, right| left.0.cmp(&right.0));
        for (identity, payload) in live_items {
            if desired_keys.contains(&(kind.to_string(), identity.clone())) {
                continue;
            }
            let action = if allow_prune { "delete" } else { "blocked" };
            rows.push(json!({
                "path": Value::Null,
                "kind": *kind,
                "identity": identity,
                "action": action,
                "reason": if allow_prune {
                    "missing-from-desired-state"
                } else {
                    "prune-required"
                },
                "desired": Value::Null,
                "live": Value::Object(payload),
            }));
        }
    }

    Ok(build_alert_plan_document(&rows, allow_prune))
}

pub fn build_alert_delete_preview_from_files(
    resource_files: &[PathBuf],
    allow_policy_reset: bool,
) -> Result<Value> {
    let mut rows = Vec::new();
    for path in resource_files {
        let document = load_alert_resource_file(path, "Alerting delete target")?;
        let (kind, payload) = build_import_operation(&document)?;
        let identity = build_resource_identity(&kind, &payload);
        let blocked = kind == POLICIES_KIND && !allow_policy_reset;
        rows.push(json!({
            "path": path_string(path),
            "kind": kind,
            "identity": identity,
            "action": if blocked { "blocked" } else { "delete" },
            "reason": if blocked {
                "policy-reset-requires-allow-policy-reset"
            } else {
                "explicit-delete-request"
            },
            "desired": Value::Object(payload),
        }));
    }
    Ok(build_alert_delete_preview_document(
        &rows,
        allow_policy_reset,
    ))
}

pub fn build_alert_delete_preview_from_dir(
    desired_dir: &Path,
    allow_policy_reset: bool,
) -> Result<Value> {
    build_alert_delete_preview_from_files(
        &discover_alert_resource_files(desired_dir)?,
        allow_policy_reset,
    )
}

fn payload_object_from_row<'a>(
    row: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a Map<String, Value>> {
    row.get(field)
        .ok_or_else(|| message(format!("Alert plan row is missing {field}.")))
        .and_then(|value| value_as_object(value, &format!("Alert plan row field {field}")))
}

fn apply_create_with_request<F>(
    request_json: &mut F,
    kind: &str,
    payload: &Map<String, Value>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => Ok(request_json(
            Method::POST,
            "/api/v1/provisioning/alert-rules",
            &[],
            Some(&Value::Object(build_rule_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        CONTACT_POINT_KIND => Ok(request_json(
            Method::POST,
            "/api/v1/provisioning/contact-points",
            &[],
            Some(&Value::Object(build_contact_point_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        MUTE_TIMING_KIND => Ok(request_json(
            Method::POST,
            "/api/v1/provisioning/mute-timings",
            &[],
            Some(&Value::Object(build_mute_timing_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        TEMPLATE_KIND => {
            let mut template_payload = build_template_import_payload(payload)?;
            let name = super::string_field(&template_payload, "name", "");
            template_payload.insert("version".to_string(), Value::String(String::new()));
            template_payload.remove("name");
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[],
                Some(&Value::Object(template_payload)),
            )?
            .unwrap_or(Value::Null))
        }
        POLICIES_KIND => Ok(request_json(
            Method::PUT,
            "/api/v1/provisioning/policies",
            &[],
            Some(&Value::Object(build_policies_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        _ => Err(message(format!("Unsupported alert create kind {kind}."))),
    }
}

fn apply_update_with_request<F>(
    request_json: &mut F,
    kind: &str,
    identity: &str,
    payload: &Map<String, Value>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => {
            let mut body = build_rule_import_payload(payload)?;
            if !body.contains_key("uid") && !identity.is_empty() {
                body.insert("uid".to_string(), Value::String(identity.to_string()));
            }
            let uid = super::string_field(&body, "uid", identity);
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        CONTACT_POINT_KIND => {
            let mut body = build_contact_point_import_payload(payload)?;
            if !body.contains_key("uid") && !identity.is_empty() {
                body.insert("uid".to_string(), Value::String(identity.to_string()));
            }
            let uid = super::string_field(&body, "uid", identity);
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/contact-points/{uid}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        MUTE_TIMING_KIND => {
            let body = build_mute_timing_import_payload(payload)?;
            let name = super::string_field(&body, "name", identity);
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/mute-timings/{name}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        TEMPLATE_KIND => {
            let mut body = build_template_import_payload(payload)?;
            let name = super::string_field(&body, "name", identity);
            let existing = request_optional_object_with_request(
                &mut *request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?;
            body.insert(
                "version".to_string(),
                Value::String(
                    existing
                        .as_ref()
                        .map(|item| super::string_field(item, "version", ""))
                        .unwrap_or_default(),
                ),
            );
            body.remove("name");
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        POLICIES_KIND => Ok(request_json(
            Method::PUT,
            "/api/v1/provisioning/policies",
            &[],
            Some(&Value::Object(build_policies_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        _ => Err(message(format!("Unsupported alert update kind {kind}."))),
    }
}

fn apply_delete_with_request<F>(
    request_json: &mut F,
    kind: &str,
    identity: &str,
    allow_policy_reset: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => Ok(request_json(
            Method::DELETE,
            &format!("/api/v1/provisioning/alert-rules/{identity}"),
            &[],
            None,
        )?
        .unwrap_or(Value::Null)),
        CONTACT_POINT_KIND => Ok(request_json(
            Method::DELETE,
            &format!("/api/v1/provisioning/contact-points/{identity}"),
            &[],
            None,
        )?
        .unwrap_or(Value::Null)),
        MUTE_TIMING_KIND => Ok(request_json(
            Method::DELETE,
            &format!("/api/v1/provisioning/mute-timings/{identity}"),
            &[("version".to_string(), String::new())],
            None,
        )?
        .unwrap_or(Value::Null)),
        TEMPLATE_KIND => Ok(request_json(
            Method::DELETE,
            &format!("/api/v1/provisioning/templates/{identity}"),
            &[("version".to_string(), String::new())],
            None,
        )?
        .unwrap_or(Value::Null)),
        POLICIES_KIND => {
            if !allow_policy_reset {
                return Err(message(
                    "Refusing live notification policy reset without --allow-policy-reset.",
                ));
            }
            Ok(
                request_json(Method::DELETE, "/api/v1/provisioning/policies", &[], None)?
                    .unwrap_or(Value::Null),
            )
        }
        _ => Err(message(format!("Unsupported alert delete kind {kind}."))),
    }
}

pub fn execute_alert_plan_with_request<F>(
    mut request_json: F,
    plan_document: &Value,
    allow_policy_reset: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let plan = value_as_object(plan_document, "Alert plan document")?;
    if plan.get("kind").and_then(Value::as_str) != Some(ALERT_PLAN_KIND) {
        return Err(message("Alert plan document kind is not supported."));
    }
    let rows = plan
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Alert plan document is missing rows."))?;

    let mut results = Vec::new();
    let mut applied_count = 0usize;
    for row in rows {
        let row = row_object(row, "Alert plan row")?;
        let action = row.get("action").and_then(Value::as_str).unwrap_or("");
        if !matches!(action, "create" | "update" | "delete") {
            continue;
        }
        let kind = row
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| message("Alert plan row is missing kind."))?;
        let identity = row
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let response = match action {
            "create" => {
                let desired = payload_object_from_row(row, "desired")?;
                apply_create_with_request(&mut request_json, kind, desired)?
            }
            "update" => {
                let desired = payload_object_from_row(row, "desired")?;
                apply_update_with_request(&mut request_json, kind, identity, desired)?
            }
            "delete" => {
                apply_delete_with_request(&mut request_json, kind, identity, allow_policy_reset)?
            }
            _ => unreachable!(),
        };
        applied_count += 1;
        results.push(json!({
            "kind": kind,
            "identity": identity,
            "action": action,
            "response": response,
        }));
    }

    Ok(json!({
        "kind": "grafana-util-alert-apply-result",
        "mode": "apply",
        "allowPolicyReset": allow_policy_reset,
        "appliedCount": applied_count,
        "results": results,
    }))
}

pub fn init_alert_runtime_layout(root: &Path) -> Result<Value> {
    let created = init_alert_managed_dir(root)?
        .into_iter()
        .map(|path| Value::String(path_string(&path)))
        .collect::<Vec<Value>>();
    Ok(json!({
        "kind": "grafana-util-alert-init",
        "root": path_string(root),
        "created": created,
    }))
}

pub fn write_new_rule_scaffold(path: &Path, name: &str, overwrite: bool) -> Result<Value> {
    let document = build_new_rule_scaffold_document(name);
    write_alert_resource_file(path, &document, overwrite)?;
    Ok(document)
}

pub fn write_new_contact_point_scaffold(path: &Path, name: &str, overwrite: bool) -> Result<Value> {
    let document = build_new_contact_point_scaffold_document(name);
    write_alert_resource_file(path, &document, overwrite)?;
    Ok(document)
}

#[allow(dead_code)]
pub fn write_contact_point_scaffold(
    path: &Path,
    name: &str,
    channel_type: &str,
    overwrite: bool,
) -> Result<Value> {
    let document = build_contact_point_scaffold_document(name, channel_type);
    write_alert_resource_file(path, &document, overwrite)?;
    Ok(document)
}

pub fn write_new_template_scaffold(path: &Path, name: &str, overwrite: bool) -> Result<Value> {
    let document = build_new_template_scaffold_document(name);
    write_alert_resource_file(path, &document, overwrite)?;
    Ok(document)
}

#[allow(dead_code)]
pub fn build_managed_policy_edit_preview_document(
    current_policy_document: &Value,
    route_name: &str,
    desired_route_document: Option<&Value>,
) -> Result<Value> {
    let current_policy = value_as_object(current_policy_document, "Current notification policies")?;
    let desired_route = match desired_route_document {
        Some(value) => Some(value_as_object(value, "Desired managed route")?),
        None => None,
    };
    Ok(json!({
        "kind": "grafana-util-alert-managed-policy-preview",
        "routeName": route_name,
        "preview": build_managed_policy_route_preview(current_policy, route_name, desired_route)?,
    }))
}

#[allow(dead_code)]
pub fn apply_managed_policy_subtree_edit_document(
    current_policy_document: &Value,
    route_name: &str,
    desired_route_document: Option<&Value>,
) -> Result<Value> {
    let current_policy = value_as_object(current_policy_document, "Current notification policies")?;
    let (next_policy, action) = match desired_route_document {
        Some(value) => upsert_managed_policy_subtree(
            current_policy,
            route_name,
            value_as_object(value, "Desired managed route")?,
        )?,
        None => remove_managed_policy_subtree(current_policy, route_name)?,
    };
    Ok(json!({
        "kind": POLICIES_KIND,
        "action": action,
        "spec": Value::Object(next_policy),
    }))
}

pub fn build_alert_import_dry_run_document(rows: &[Value]) -> Value {
    let processed = rows.len();
    let would_create = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-create"))
        .count();
    let would_update = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-update"))
        .count();
    let would_fail_existing = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-fail-existing"))
        .count();

    json!({
        "summary": {
            "processed": processed,
            "wouldCreate": would_create,
            "wouldUpdate": would_update,
            "wouldFailExisting": would_fail_existing,
        },
        "rows": rows,
    })
}

pub fn build_alert_diff_document(rows: &[Value]) -> Value {
    let checked = rows.len();
    let same = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("same"))
        .count();
    let different = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("different"))
        .count();
    let missing_remote = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("missing-remote"))
        .count();

    json!({
        "summary": {
            "checked": checked,
            "same": same,
            "different": different,
            "missingRemote": missing_remote,
        },
        "rows": rows,
    })
}
