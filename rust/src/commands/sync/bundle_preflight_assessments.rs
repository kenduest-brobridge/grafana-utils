use super::json::{require_json_array, require_json_object, require_json_object_field};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ProviderAssessmentSummary {
    pub(crate) blocking_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct SecretPlaceholderAssessmentSummary {
    pub(crate) blocking_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct AlertArtifactAssessmentSummary {
    pub(crate) resource_count: i64,
    pub(crate) blocked_count: i64,
    pub(crate) plan_only_count: i64,
}

pub(crate) fn provider_assessment_summary(document: &Value) -> Result<ProviderAssessmentSummary> {
    let document = require_json_object(document, "Sync provider assessment document")?;
    let summary =
        require_json_object_field(document, "summary", "Sync provider assessment document")?;
    serde_json::from_value(Value::Object(summary.clone())).map_err(|error| {
        message(format!(
            "Sync provider assessment summary is invalid: {error}"
        ))
    })
}

pub(crate) fn alert_artifact_assessment_summary(
    document: &Value,
) -> Result<AlertArtifactAssessmentSummary> {
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

pub(crate) fn secret_placeholder_assessment_summary(
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

fn datasource_spec_with_body_fields(object: &Map<String, Value>) -> Map<String, Value> {
    let mut datasource_spec = object.clone();
    let body = object
        .get("body")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for key in ["uid", "name", "type"] {
        if !datasource_spec.contains_key(key) {
            if let Some(value) = body.get(key) {
                datasource_spec.insert(key.to_string(), value.clone());
            }
        }
    }
    datasource_spec
}

pub(crate) fn build_provider_assessment(
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
        let datasource_spec = datasource_spec_with_body_fields(object);
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

pub(crate) fn build_secret_placeholder_assessment(
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
        let datasource_spec = datasource_spec_with_body_fields(object);
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

pub(crate) fn collect_alert_specs(source_bundle: &Map<String, Value>) -> Result<Vec<Value>> {
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

#[derive(Debug, Clone, Copy)]
struct AlertArtifactSectionSpec {
    section: &'static str,
    kind: &'static str,
    status: &'static str,
    blocking: bool,
    detail: &'static str,
}

const ALERT_ARTIFACT_SECTION_SPECS: &[AlertArtifactSectionSpec] = &[
    AlertArtifactSectionSpec {
        section: "contactPoints",
        kind: "alert-contact-point",
        status: "plan-only",
        blocking: false,
        detail: "Contact points are staged in the workspace package but are not live-wired yet.",
    },
    AlertArtifactSectionSpec {
        section: "muteTimings",
        kind: "alert-mute-timing",
        status: "blocked",
        blocking: true,
        detail: "Mute timings are staged in the workspace package but are not live-wired yet.",
    },
    AlertArtifactSectionSpec {
        section: "policies",
        kind: "alert-policy",
        status: "blocked",
        blocking: true,
        detail:
            "Notification policies are staged in the workspace package but are not live-wired yet.",
    },
    AlertArtifactSectionSpec {
        section: "templates",
        kind: "alert-template",
        status: "blocked",
        blocking: true,
        detail: "Templates are staged in the workspace package but are not live-wired yet.",
    },
];

#[derive(Debug, Default)]
struct AlertArtifactSectionCounts {
    contact_points: usize,
    mute_timings: usize,
    policies: usize,
    templates: usize,
}

impl AlertArtifactSectionCounts {
    fn set(&mut self, section: &str, count: usize) {
        match section {
            "contactPoints" => self.contact_points = count,
            "muteTimings" => self.mute_timings = count,
            "policies" => self.policies = count,
            "templates" => self.templates = count,
            _ => {}
        }
    }
}

fn alert_artifact_identity(section: &str, spec: &Map<String, Value>) -> String {
    match section {
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
    }
}

fn alert_artifact_title(section: &str, spec: &Map<String, Value>, identity: &str) -> String {
    match section {
        "contactPoints" | "muteTimings" | "templates" => {
            let name = string_field(spec, "name", "");
            if name.is_empty() {
                identity.to_string()
            } else {
                name
            }
        }
        "policies" => {
            let receiver = string_field(spec, "receiver", "");
            if receiver.is_empty() {
                let name = string_field(spec, "name", "");
                if name.is_empty() {
                    identity.to_string()
                } else {
                    name
                }
            } else {
                receiver
            }
        }
        _ => identity.to_string(),
    }
}

fn append_alert_artifact_section(
    alerting: &Map<String, Value>,
    section_spec: AlertArtifactSectionSpec,
    counts: &mut AlertArtifactSectionCounts,
    checks: &mut Vec<Value>,
) {
    let items = alerting
        .get(section_spec.section)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    counts.set(section_spec.section, items.len());
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
        let identity = alert_artifact_identity(section_spec.section, spec);
        let title = alert_artifact_title(section_spec.section, spec, &identity);
        checks.push(serde_json::json!({
            "kind": section_spec.kind,
            "identity": if identity.is_empty() { source_path.clone() } else { identity },
            "title": title,
            "sourcePath": source_path,
            "status": section_spec.status,
            "blocking": section_spec.blocking,
            "detail": section_spec.detail,
        }));
    }
}

pub(crate) fn build_alert_artifact_assessment(source_bundle: &Map<String, Value>) -> Value {
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

    let mut checks = Vec::new();
    let mut counts = AlertArtifactSectionCounts::default();
    for section_spec in ALERT_ARTIFACT_SECTION_SPECS {
        append_alert_artifact_section(alerting, *section_spec, &mut counts, &mut checks);
    }

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
                "contactPointCount": counts.contact_points,
                "muteTimingCount": counts.mute_timings,
                "policyCount": counts.policies,
                "templateCount": counts.templates,
            "planOnlyCount": plan_only_count,
            "blockedCount": blocked_count,
        },
        "checks": checks,
    })
}
