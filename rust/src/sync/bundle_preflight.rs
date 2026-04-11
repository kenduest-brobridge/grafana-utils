//! Staged bundle-level sync preflight helpers.
//!
//! Purpose:
//! - Aggregate staged sync preflight checks and datasource provider assessments
//!   into one reviewable bundle document.
//! - Keep Rust-side bundle planning pure and import-safe before any CLI wiring.

use super::json::{require_json_array, require_json_object, require_json_object_field};
use super::preflight::{build_sync_preflight_document, SyncPreflightSummary};
use crate::common::{message, string_field, Result};
use crate::datasource_provider::{
    build_provider_plan, iter_provider_names, summarize_provider_plan,
};
use crate::datasource_secret::{
    build_secret_placeholder_plan, iter_secret_placeholder_names, summarize_secret_placeholder_plan,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// Constant for sync bundle preflight kind.
pub const SYNC_BUNDLE_PREFLIGHT_KIND: &str = "grafana-utils-sync-bundle-preflight";
/// Constant for sync bundle preflight schema version.
pub const SYNC_BUNDLE_PREFLIGHT_SCHEMA_VERSION: i64 = 1;

/// Struct definition for SyncBundlePreflightSummary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncBundlePreflightSummary {
    pub resource_count: i64,
    pub sync_blocking_count: i64,
    pub provider_blocking_count: i64,
    pub secret_placeholder_blocking_count: i64,
    pub alert_artifact_count: i64,
    pub alert_artifact_blocked_count: i64,
    pub alert_artifact_plan_only_count: i64,
}

/// Struct definition for ProviderAssessmentSummary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ProviderAssessmentSummary {
    blocking_count: i64,
}

/// Struct definition for SecretPlaceholderAssessmentSummary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct SecretPlaceholderAssessmentSummary {
    blocking_count: i64,
}

/// Struct definition for AlertArtifactAssessmentSummary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct AlertArtifactAssessmentSummary {
    pub(crate) resource_count: i64,
    pub(crate) blocked_count: i64,
    pub(crate) plan_only_count: i64,
}

impl SyncBundlePreflightSummary {
    pub(crate) fn from_document(document: &Value) -> Result<Self> {
        let object = require_json_object(document, "Sync bundle preflight document")?;
        let summary =
            require_json_object_field(object, "summary", "Sync bundle preflight document")?;
        serde_json::from_value(Value::Object(summary.clone()))
            .map_err(|error| message(format!("Sync bundle preflight summary is invalid: {error}")))
    }
}

pub(crate) fn require_sync_bundle_preflight_summary(
    document: &Value,
) -> Result<SyncBundlePreflightSummary> {
    let document = require_json_object(document, "Sync bundle preflight document")?;
    let summary = require_json_object_field(document, "summary", "Sync bundle preflight document")?;
    let resource_count = summary
        .get("resourceCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync bundle preflight summary is missing resourceCount."))?;
    let sync_blocking_count = summary
        .get("syncBlockingCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync bundle preflight summary is missing syncBlockingCount."))?;
    let provider_blocking_count = summary
        .get("providerBlockingCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            message("Sync bundle preflight summary is missing providerBlockingCount.")
        })?;
    let secret_placeholder_blocking_count = summary
        .get("secretPlaceholderBlockingCount")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Ok(SyncBundlePreflightSummary {
        resource_count,
        sync_blocking_count,
        provider_blocking_count,
        secret_placeholder_blocking_count,
        alert_artifact_count: 0,
        alert_artifact_blocked_count: 0,
        alert_artifact_plan_only_count: 0,
    })
}

fn provider_assessment_summary(document: &Value) -> Result<ProviderAssessmentSummary> {
    let document = require_json_object(document, "Sync provider assessment document")?;
    let summary =
        require_json_object_field(document, "summary", "Sync provider assessment document")?;
    serde_json::from_value(Value::Object(summary.clone())).map_err(|error| {
        message(format!(
            "Sync provider assessment summary is invalid: {error}"
        ))
    })
}

fn alert_artifact_assessment_summary(document: &Value) -> Result<AlertArtifactAssessmentSummary> {
    let document = require_json_object(document, "Sync alert artifact assessment document")?;
    let summary = require_json_object_field(
        document,
        "summary",
        "Sync alert artifact assessment document",
    )?;
    serde_json::from_value(Value::Object(summary.clone())).map_err(|error| {
        message(format!(
            "Sync alert artifact assessment summary is invalid: {error}"
        ))
    })
}

fn secret_placeholder_assessment_summary(
    document: &Value,
) -> Result<SecretPlaceholderAssessmentSummary> {
    let document = require_json_object(document, "Sync secret placeholder assessment document")?;
    let summary = require_json_object_field(
        document,
        "summary",
        "Sync secret placeholder assessment document",
    )?;
    serde_json::from_value(Value::Object(summary.clone())).map_err(|error| {
        message(format!(
            "Sync secret placeholder assessment summary is invalid: {error}"
        ))
    })
}

pub(crate) fn alert_artifact_assessment_summary_or_default(
    document: &Value,
) -> AlertArtifactAssessmentSummary {
    alert_artifact_assessment_summary(document).unwrap_or_default()
}

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn require_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = require_json_array(value, label)?;
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

fn build_secret_placeholder_assessment(
    datasource_specs: &[Value],
    availability: &Map<String, Value>,
) -> Result<Value> {
    let placeholder_names = require_string_list(
        availability
            .get("secretPlaceholderNames")
            .or_else(|| availability.get("secretNames")),
        "secretPlaceholderNames",
    )?
    .into_iter()
    .collect::<BTreeSet<String>>();
    let mut plans = Vec::new();
    let mut checks = Vec::new();
    for datasource in datasource_specs {
        let Some(object) = datasource.as_object() else {
            continue;
        };
        let Some(Value::Object(_)) = object.get("secureJsonDataPlaceholders") else {
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
        let plan = build_secret_placeholder_plan(&datasource_spec)?;
        let plan_summary = summarize_secret_placeholder_plan(&plan);
        for placeholder_name in iter_secret_placeholder_names(&plan.placeholders) {
            let missing = !placeholder_names.contains(placeholder_name);
            checks.push(Value::Object(Map::from_iter(vec![
                (
                    "kind".to_string(),
                    Value::String("secret-placeholder".to_string()),
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
                        placeholder_name
                    )),
                ),
                (
                    "placeholderName".to_string(),
                    Value::String(placeholder_name.to_string()),
                ),
                (
                    "status".to_string(),
                    Value::String(if missing { "missing" } else { "ok" }.to_string()),
                ),
                (
                    "detail".to_string(),
                    Value::String(if missing {
                        "Datasource secret placeholder is not available in secretPlaceholderNames availability input."
                            .to_string()
                    } else {
                        "Datasource secret placeholder is available for staged review via secretPlaceholderNames availability input."
                            .to_string()
                    }),
                ),
                ("blocking".to_string(), Value::Bool(missing)),
            ])));
        }
        plans.push(plan_summary);
    }
    let reference_count = plans
        .iter()
        .map(|plan| {
            plan.get("placeholderNames")
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
        for item in require_json_array(items, "alerts")? {
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
    for item in require_json_array(rule_documents, "alerting.rules")? {
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
                    "Contact points are staged in the workspace package but are not live-wired yet."
                }
                "muteTimings" => {
                    "Mute timings are staged in the workspace package but are not live-wired yet."
                }
                "policies" => {
                    "Notification policies are staged in the workspace package but are not live-wired yet."
                }
                "templates" => {
                    "Templates are staged in the workspace package but are not live-wired yet."
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
    // Build the bundle preflight from staged bundle JSON so the sync-level and
    // provider-level gates can share one normalized availability snapshot.

    let source_bundle = require_json_object(source_bundle, "source bundle")?.clone();
    let _target_inventory = require_json_object(target_inventory, "target inventory")?.clone();
    let availability = match availability {
        None => Map::new(),
        Some(value) => require_json_object(value, "availability")?.clone(),
    };
    let mut desired_specs = Vec::new();
    for key in ["dashboards", "datasources", "folders"] {
        let Some(items) = source_bundle.get(key) else {
            continue;
        };
        for item in require_json_array(items, key)? {
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
    let secret_placeholder_assessment = build_secret_placeholder_assessment(
        source_bundle
            .get("datasources")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        &availability,
    )?;
    let sync_preflight_summary = SyncPreflightSummary::from_document(&sync_preflight)?;
    let provider_summary = provider_assessment_summary(&provider_assessment)?;
    let secret_placeholder_summary =
        secret_placeholder_assessment_summary(&secret_placeholder_assessment)?;
    let alert_artifact_summary = alert_artifact_assessment_summary(&alert_artifact_assessment)?;
    let summary = SyncBundlePreflightSummary {
        resource_count: desired_specs.len() as i64,
        sync_blocking_count: sync_preflight_summary.blocking_count,
        provider_blocking_count: provider_summary.blocking_count,
        secret_placeholder_blocking_count: secret_placeholder_summary.blocking_count,
        alert_artifact_count: alert_artifact_summary.resource_count,
        alert_artifact_blocked_count: alert_artifact_summary.blocked_count,
        alert_artifact_plan_only_count: alert_artifact_summary.plan_only_count,
    };
    Ok(Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(SYNC_BUNDLE_PREFLIGHT_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(SYNC_BUNDLE_PREFLIGHT_SCHEMA_VERSION.into()),
        ),
        ("summary".to_string(), serde_json::to_value(&summary)?),
        ("syncPreflight".to_string(), sync_preflight),
        (
            "alertArtifactAssessment".to_string(),
            alert_artifact_assessment.clone(),
        ),
        (
            "secretPlaceholderAssessment".to_string(),
            secret_placeholder_assessment,
        ),
        ("providerAssessment".to_string(), provider_assessment),
    ])))
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_sync_bundle_preflight_text(document: &Value) -> Result<Vec<String>> {
    // Keep the renderer strict about kind/summary so output formatting cannot
    // drift away from the staged bundle-preflight document contract.

    let kind = normalize_text(document.get("kind"));
    if kind != SYNC_BUNDLE_PREFLIGHT_KIND {
        return Err(message(
            "Sync bundle preflight document kind is not supported.",
        ));
    }
    let summary = SyncBundlePreflightSummary::from_document(document)?;
    let (
        secret_placeholder_datasource_count,
        secret_placeholder_reference_count,
        secret_placeholder_blocking_count,
    ) = if let Some(secret_assessment) = document
        .get("secretPlaceholderAssessment")
        .and_then(Value::as_object)
    {
        let summary = secret_assessment.get("summary").and_then(Value::as_object);
        (
            summary
                .and_then(|item| item.get("datasourceCount"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .and_then(|item| item.get("referenceCount"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .and_then(|item| item.get("blockingCount"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
        )
    } else {
        (0, 0, summary.secret_placeholder_blocking_count)
    };
    Ok(vec![
        "Sync bundle preflight summary".to_string(),
        format!("Resources: {} total", summary.resource_count),
        format!("Sync blocking: {}", summary.sync_blocking_count),
        format!("Provider blocking: {}", summary.provider_blocking_count),
        format!(
            "Secret placeholders: {} datasources, {} references, {} blocking",
            secret_placeholder_datasource_count,
            secret_placeholder_reference_count,
            secret_placeholder_blocking_count
        ),
        format!(
            "Secret placeholders blocking: {}",
            summary.secret_placeholder_blocking_count
        ),
        format!("Alert artifacts: {} total", summary.alert_artifact_count),
        "Reason: missing provider or secret placeholder availability blocks apply; plan-only alert artifacts stay staged; blocked artifacts prevent apply."
            .to_string(),
    ])
}

#[cfg(test)]
#[path = "bundle_rust_tests.rs"]
mod sync_bundle_rust_tests;
