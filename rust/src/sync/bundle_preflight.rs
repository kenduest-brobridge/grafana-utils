//! Staged bundle-level sync preflight helpers.
//!
//! Purpose:
//! - Aggregate staged sync preflight checks and datasource provider assessments
//!   into one reviewable bundle document.
//! - Keep Rust-side bundle planning pure and import-safe before any CLI wiring.

use super::preflight::build_sync_preflight_document;
use crate::common::{message, string_field, Result};
use crate::datasource_provider::{
    build_provider_plan, iter_provider_names, summarize_provider_plan,
};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// Constant for sync bundle preflight kind.
pub const SYNC_BUNDLE_PREFLIGHT_KIND: &str = "grafana-utils-sync-bundle-preflight";
/// Constant for sync bundle preflight schema version.
pub const SYNC_BUNDLE_PREFLIGHT_SCHEMA_VERSION: i64 = 1;

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn require_object(value: Option<&Value>, label: &str) -> Result<Map<String, Value>> {
    match value {
        None => Ok(Map::new()),
        Some(Value::Object(object)) => Ok(object.clone()),
        Some(_) => Err(message(format!("{label} must be a JSON object."))),
    }
}

fn require_array<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a Vec<Value>> {
    match value {
        None => Err(message(format!("{label} must be a JSON array."))),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(message(format!("{label} must be a JSON array."))),
    }
}

fn require_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| message(format!("{label} must be a list.")))?;
    let mut result = Vec::new();
    for item in items {
        let text = normalize_text(Some(item));
        if !text.is_empty() {
            result.push(text);
        }
    }
    Ok(result)
}

fn build_provider_assessment(
    datasource_specs: &[Value],
    availability: &Map<String, Value>,
) -> Result<Value> {
    let provider_names = require_string_list(availability.get("providerNames"), "providerNames")?
        .into_iter()
        .collect::<BTreeSet<String>>();
    let mut plans = Vec::new();
    let mut checks = Vec::new();
    for datasource in datasource_specs {
        let Some(object) = datasource.as_object() else {
            continue;
        };
        let Some(Value::Object(_)) = object.get("secureJsonDataProviders") else {
            continue;
        };
        let mut datasource_spec = object.clone();
        let body = object
            .get("body")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if !datasource_spec.contains_key("uid") {
            if let Some(value) = body.get("uid") {
                datasource_spec.insert("uid".to_string(), value.clone());
            }
        }
        if !datasource_spec.contains_key("name") {
            if let Some(value) = body.get("name") {
                datasource_spec.insert("name".to_string(), value.clone());
            }
        }
        if !datasource_spec.contains_key("type") {
            if let Some(value) = body.get("type") {
                datasource_spec.insert("type".to_string(), value.clone());
            }
        }
        let plan = build_provider_plan(&datasource_spec)?;
        let plan_summary = summarize_provider_plan(&plan);
        for provider_name in iter_provider_names(&plan.references) {
            let missing = !provider_names.contains(provider_name);
            checks.push(Value::Object(Map::from_iter(vec![
                (
                    "kind".to_string(),
                    Value::String("secret-provider".to_string()),
                ),
                (
                    "datasourceName".to_string(),
                    Value::String(plan.datasource_name.clone()),
                ),
                (
                    "identity".to_string(),
                    Value::String(format!(
                        "{}->{}",
                        plan.datasource_uid
                            .clone()
                            .unwrap_or_else(|| plan.datasource_name.clone()),
                        provider_name
                    )),
                ),
                (
                    "providerName".to_string(),
                    Value::String(provider_name.to_string()),
                ),
                (
                    "status".to_string(),
                    Value::String(if missing { "missing" } else { "ok" }.to_string()),
                ),
                ("blocking".to_string(), Value::Bool(missing)),
            ])));
        }
        plans.push(plan_summary);
    }
    let reference_count = plans
        .iter()
        .map(|plan| {
            plan.get("providers")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0)
        })
        .sum::<usize>();
    let blocking_count = checks
        .iter()
        .filter(|item| item.get("blocking").and_then(Value::as_bool) == Some(true))
        .count();
    Ok(Value::Object(Map::from_iter(vec![
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "datasourceCount".to_string(),
                    Value::Number((plans.len() as i64).into()),
                ),
                (
                    "referenceCount".to_string(),
                    Value::Number((reference_count as i64).into()),
                ),
                (
                    "blockingCount".to_string(),
                    Value::Number((blocking_count as i64).into()),
                ),
            ])),
        ),
        ("plans".to_string(), Value::Array(plans)),
        ("checks".to_string(), Value::Array(checks)),
    ])))
}

fn extract_alert_rule_spec(document: &Map<String, Value>) -> Result<Option<Map<String, Value>>> {
    if document.get("kind").and_then(Value::as_str) == Some("grafana-alert-rule") {
        let Some(spec) = document.get("spec").and_then(Value::as_object) else {
            return Err(message(
                "grafana-alert-rule bundle document is missing a valid spec object.",
            ));
        };
        return Ok(Some(spec.clone()));
    }
    if document.contains_key("condition") && document.contains_key("data") {
        return Ok(Some(document.clone()));
    }
    Ok(None)
}

fn normalize_alert_bundle_item(
    rule_spec: &Map<String, Value>,
    source_path: Option<&str>,
) -> Result<Value> {
    let uid = normalize_text(rule_spec.get("uid"));
    if uid.is_empty() {
        return Err(message(format!(
            "Alert bundle rule document is missing uid{}.",
            source_path
                .map(|value| format!(": {value}"))
                .unwrap_or_default()
        )));
    }
    let title = {
        let title = normalize_text(rule_spec.get("title"));
        if title.is_empty() {
            uid.clone()
        } else {
            title
        }
    };
    Ok(serde_json::json!({
        "kind": "alert",
        "uid": uid,
        "title": title,
        "managedFields": ["condition"],
        "body": rule_spec,
        "sourcePath": source_path.unwrap_or(""),
    }))
}

fn collect_alert_specs(source_bundle: &Map<String, Value>) -> Result<Vec<Value>> {
    let mut alerts = Vec::new();
    if let Some(items) = source_bundle.get("alerts") {
        for item in require_array(Some(items), "alerts")? {
            alerts.push(item.clone());
        }
    }
    if !alerts.is_empty() {
        return Ok(alerts);
    }
    let Some(alerting) = source_bundle.get("alerting").and_then(Value::as_object) else {
        return Ok(alerts);
    };
    let Some(rule_documents) = alerting.get("rules") else {
        return Ok(alerts);
    };
    for item in require_array(Some(rule_documents), "alerting.rules")? {
        let Some(object) = item.as_object() else {
            continue;
        };
        let source_path = object.get("sourcePath").and_then(Value::as_str);
        let Some(document) = object.get("document").and_then(Value::as_object) else {
            continue;
        };
        let Some(rule_spec) = extract_alert_rule_spec(document)? else {
            continue;
        };
        alerts.push(normalize_alert_bundle_item(&rule_spec, source_path)?);
    }
    Ok(alerts)
}

fn build_alert_artifact_assessment(source_bundle: &Map<String, Value>) -> Value {
    let Some(alerting) = source_bundle.get("alerting").and_then(Value::as_object) else {
        return serde_json::json!({
            "summary": {
                "resourceCount": 0,
                "contactPointCount": 0,
                "muteTimingCount": 0,
                "policyCount": 0,
                "templateCount": 0,
                "planOnlyCount": 0,
                "blockedCount": 0,
            },
            "checks": [],
        });
    };

    fn append_alert_artifact_section(
        alerting: &Map<String, Value>,
        section: &str,
        kind: &str,
        status: &str,
        blocking: bool,
        count_slot: &mut usize,
        checks: &mut Vec<Value>,
    ) {
        let items = alerting
            .get(section)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        *count_slot = items.len();
        for item in items {
            let Some(object) = item.as_object() else {
                continue;
            };
            let source_path = string_field(object, "sourcePath", "");
            let document = object
                .get("document")
                .or_else(|| object.get("body"))
                .unwrap_or(&item);
            let Some(document_object) = document.as_object() else {
                continue;
            };
            let spec = document_object
                .get("spec")
                .and_then(Value::as_object)
                .unwrap_or(document_object);
            let identity = match section {
                "contactPoints" => {
                    let uid = string_field(spec, "uid", "");
                    if uid.is_empty() {
                        string_field(spec, "name", "")
                    } else {
                        uid
                    }
                }
                "muteTimings" => string_field(spec, "name", ""),
                "policies" => {
                    let receiver = string_field(spec, "receiver", "");
                    if receiver.is_empty() {
                        string_field(spec, "name", "")
                    } else {
                        receiver
                    }
                }
                "templates" => string_field(spec, "name", ""),
                _ => String::new(),
            };
            let title = match section {
                "contactPoints" => {
                    let name = string_field(spec, "name", "");
                    if name.is_empty() {
                        identity.clone()
                    } else {
                        name
                    }
                }
                "muteTimings" => {
                    let name = string_field(spec, "name", "");
                    if name.is_empty() {
                        identity.clone()
                    } else {
                        name
                    }
                }
                "policies" => {
                    let receiver = string_field(spec, "receiver", "");
                    if receiver.is_empty() {
                        let name = string_field(spec, "name", "");
                        if name.is_empty() {
                            identity.clone()
                        } else {
                            name
                        }
                    } else {
                        receiver
                    }
                }
                "templates" => {
                    let name = string_field(spec, "name", "");
                    if name.is_empty() {
                        identity.clone()
                    } else {
                        name
                    }
                }
                _ => identity.clone(),
            };
            let detail = match section {
                "contactPoints" => {
                    "Contact points are staged in the source bundle but are not live-wired yet."
                }
                "muteTimings" => {
                    "Mute timings are staged in the source bundle but are not live-wired yet."
                }
                "policies" => {
                    "Notification policies are staged in the source bundle but are not live-wired yet."
                }
                "templates" => {
                    "Templates are staged in the source bundle but are not live-wired yet."
                }
                _ => "",
            };
            checks.push(serde_json::json!({
                "kind": kind,
                "identity": if identity.is_empty() { source_path.clone() } else { identity },
                "title": title,
                "sourcePath": source_path,
                "status": status,
                "blocking": blocking,
                "detail": detail,
            }));
        }
    }

    let mut checks = Vec::new();
    let mut contact_point_count = 0usize;
    let mut mute_timing_count = 0usize;
    let mut policy_count = 0usize;
    let mut template_count = 0usize;

    append_alert_artifact_section(
        alerting,
        "contactPoints",
        "alert-contact-point",
        "plan-only",
        false,
        &mut contact_point_count,
        &mut checks,
    );
    append_alert_artifact_section(
        alerting,
        "muteTimings",
        "alert-mute-timing",
        "blocked",
        true,
        &mut mute_timing_count,
        &mut checks,
    );
    append_alert_artifact_section(
        alerting,
        "policies",
        "alert-policy",
        "blocked",
        true,
        &mut policy_count,
        &mut checks,
    );
    append_alert_artifact_section(
        alerting,
        "templates",
        "alert-template",
        "blocked",
        true,
        &mut template_count,
        &mut checks,
    );

    let plan_only_count = checks
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("plan-only"))
        .count();
    let blocked_count = checks
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("blocked"))
        .count();

    serde_json::json!({
        "summary": {
            "resourceCount": checks.len(),
            "contactPointCount": contact_point_count,
            "muteTimingCount": mute_timing_count,
            "policyCount": policy_count,
            "templateCount": template_count,
            "planOnlyCount": plan_only_count,
            "blockedCount": blocked_count,
        },
        "checks": checks,
    })
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn build_sync_bundle_preflight_document(
    source_bundle: &Value,
    target_inventory: &Value,
    availability: Option<&Value>,
) -> Result<Value> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: sync.rs:run_sync_cli, sync_bundle_rust_tests.rs:build_sync_bundle_preflight_document_aggregates_sync_and_provider_checks, sync_bundle_rust_tests.rs:build_sync_bundle_preflight_document_counts_top_level_alert_specs_from_source_bundle, sync_bundle_rust_tests.rs:build_sync_bundle_preflight_document_falls_back_to_alerting_rule_documents, sync_bundle_rust_tests.rs:build_sync_bundle_preflight_document_reads_provider_metadata_from_source_bundle_document, sync_bundle_rust_tests.rs:render_sync_bundle_preflight_text_renders_summary
    // Downstream callees: sync_bundle_preflight.rs:build_provider_assessment, sync_bundle_preflight.rs:collect_alert_specs, sync_bundle_preflight.rs:require_array, sync_bundle_preflight.rs:require_object, sync_preflight.rs:build_sync_preflight_document

    let source_bundle = require_object(Some(source_bundle), "source bundle")?;
    let _target_inventory = require_object(Some(target_inventory), "target inventory")?;
    let availability = require_object(availability, "availability")?;
    let mut desired_specs = Vec::new();
    for key in ["dashboards", "datasources", "folders"] {
        let Some(items) = source_bundle.get(key) else {
            continue;
        };
        for item in require_array(Some(items), key)? {
            desired_specs.push(item.clone());
        }
    }
    desired_specs.extend(collect_alert_specs(&source_bundle)?);
    let sync_preflight =
        build_sync_preflight_document(&desired_specs, Some(&Value::Object(availability.clone())))?;
    let alert_artifact_assessment = build_alert_artifact_assessment(&source_bundle);
    let provider_assessment = build_provider_assessment(
        source_bundle
            .get("datasources")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        &availability,
    )?;
    let sync_blocking_count = sync_preflight
        .get("summary")
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("blockingCount"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let provider_blocking_count = provider_assessment
        .get("summary")
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("blockingCount"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Ok(Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(SYNC_BUNDLE_PREFLIGHT_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(SYNC_BUNDLE_PREFLIGHT_SCHEMA_VERSION.into()),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "resourceCount".to_string(),
                    Value::Number((desired_specs.len() as i64).into()),
                ),
                (
                    "syncBlockingCount".to_string(),
                    Value::Number(sync_blocking_count.into()),
                ),
                (
                    "providerBlockingCount".to_string(),
                    Value::Number(provider_blocking_count.into()),
                ),
                (
                    "alertArtifactCount".to_string(),
                    Value::Number(
                        alert_artifact_assessment["summary"]["resourceCount"]
                            .as_i64()
                            .unwrap_or(0)
                            .into(),
                    ),
                ),
                (
                    "alertArtifactBlockedCount".to_string(),
                    Value::Number(
                        alert_artifact_assessment["summary"]["blockedCount"]
                            .as_i64()
                            .unwrap_or(0)
                            .into(),
                    ),
                ),
                (
                    "alertArtifactPlanOnlyCount".to_string(),
                    Value::Number(
                        alert_artifact_assessment["summary"]["planOnlyCount"]
                            .as_i64()
                            .unwrap_or(0)
                            .into(),
                    ),
                ),
            ])),
        ),
        ("syncPreflight".to_string(), sync_preflight),
        (
            "alertArtifactAssessment".to_string(),
            alert_artifact_assessment.clone(),
        ),
        ("providerAssessment".to_string(), provider_assessment),
    ])))
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_sync_bundle_preflight_text(document: &Value) -> Result<Vec<String>> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: sync.rs:run_sync_cli, sync_bundle_rust_tests.rs:render_sync_bundle_preflight_rejects_wrong_kind, sync_bundle_rust_tests.rs:render_sync_bundle_preflight_text_renders_summary
    // Downstream callees: common.rs:message, sync_bundle_preflight.rs:normalize_text, sync_bundle_preflight.rs:require_object

    let kind = normalize_text(document.get("kind"));
    if kind != SYNC_BUNDLE_PREFLIGHT_KIND {
        return Err(message(
            "Sync bundle preflight document kind is not supported.",
        ));
    }
    let summary = require_object(document.get("summary"), "summary")?;
    Ok(vec![
        "Sync bundle preflight summary".to_string(),
        format!(
            "Resources: {} total",
            summary
                .get("resourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Sync blocking: {}",
            summary
                .get("syncBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Provider blocking: {}",
            summary
                .get("providerBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Alert artifacts: {} total",
            summary
                .get("alertArtifactCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
    ])
}

#[cfg(test)]
#[path = "bundle_rust_tests.rs"]
mod sync_bundle_rust_tests;
