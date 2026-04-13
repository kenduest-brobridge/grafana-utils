//! Alert resource contract extensions for future sync bundle wiring.
//!
//! This module extracts typed alert-plane artifacts (rules/contact points/mute
//! timings/policies/templates) from a source bundle and classifies which ones are
//! ready for top-level sync contracts.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Constant for alert rule kind.
pub const ALERT_RULE_KIND: &str = "grafana-alert-rule";
/// Constant for alert contact point kind.
pub const ALERT_CONTACT_POINT_KIND: &str = "grafana-contact-point";
/// Constant for alert mute timing kind.
pub const ALERT_MUTE_TIMING_KIND: &str = "grafana-mute-timing";
/// Constant for alert policy kind.
pub const ALERT_POLICY_KIND: &str = "grafana-notification-policies";
/// Constant for alert template kind.
pub const ALERT_TEMPLATE_KIND: &str = "grafana-notification-template";

/// Struct definition for AlertResourceContract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResourceContract {
    pub kind: String,
    pub identity: String,
    pub title: String,
    pub source_path: String,
    pub safe_for_top_level_sync: bool,
    pub references: BTreeSet<String>,
    pub managed_fields: Vec<String>,
}

impl AlertResourceContract {
    fn from_kind_and_identity(
        kind: &str,
        identity: String,
        title: String,
        source_path: String,
    ) -> Self {
        let managed_fields = match kind {
            ALERT_RULE_KIND => vec![
                "condition".to_string(),
                "labels".to_string(),
                "annotations".to_string(),
            ],
            ALERT_CONTACT_POINT_KIND => vec!["receivers".to_string(), "routes".to_string()],
            ALERT_MUTE_TIMING_KIND => vec!["mute_timings".to_string()],
            ALERT_POLICY_KIND => vec!["policy".to_string()],
            ALERT_TEMPLATE_KIND => vec!["template".to_string()],
            _ => Vec::new(),
        };
        Self {
            kind: kind.to_string(),
            identity,
            title,
            source_path,
            safe_for_top_level_sync: is_kind_considered_sync_safe(kind),
            references: BTreeSet::new(),
            managed_fields,
        }
    }
}

/// Struct definition for AlertBundleContractReport.
#[derive(Debug, Clone)]
pub struct AlertBundleContractReport {
    pub resources: Vec<AlertResourceContract>,
}

impl AlertBundleContractReport {
    /// as json.
    pub fn as_json(&self) -> Value {
        let mut summary = BTreeMap::<String, usize>::new();
        for resource in &self.resources {
            summary.insert(
                resource.kind.clone(),
                summary.get(&resource.kind).cloned().unwrap_or(0) + 1,
            );
        }
        let mut counts = BTreeMap::new();
        for (kind, count) in summary {
            counts.insert(kind, count);
        }
        let payload = self
            .resources
            .iter()
            .map(|resource| {
                let managed_fields = resource.managed_fields.to_vec();
                json!({
                    "kind": resource.kind,
                    "identity": resource.identity,
                    "title": resource.title,
                    "sourcePath": resource.source_path,
                    "safeForSync": resource.safe_for_top_level_sync,
                    "managedFields": managed_fields,
                    "references": resource.references.iter().cloned().collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();

        let mut count_array = Vec::new();
        for (kind, count) in counts {
            count_array.push(json!({ "kind": kind, "count": count }));
        }
        json!({
            "kind": "grafana-utils-sync-alert-contract",
            "summary": {
                "total": self.resources.len(),
                "safeForSync": self
                    .resources
                    .iter()
                    .filter(|resource| resource.safe_for_top_level_sync)
                    .count(),
            },
            "countsByKind": count_array,
            "resources": payload,
        })
    }
}

fn is_kind_considered_sync_safe(kind: &str) -> bool {
    matches!(kind, ALERT_RULE_KIND | ALERT_CONTACT_POINT_KIND)
}

fn resolve_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn extract_identity(document: &Value, kind: &str) -> String {
    match kind {
        ALERT_RULE_KIND => {
            let identity = resolve_string(
                document
                    .get("uid")
                    .or_else(|| document.get("name"))
                    .or_else(|| document.get("identity")),
            );
            if !identity.is_empty() {
                return identity;
            }
            resolve_string(document.get("spec").and_then(|value| value.get("uid")))
        }
        _ => resolve_string(document.get("uid").or_else(|| document.get("name")))
            .trim()
            .to_string(),
    }
}

fn extract_title(document: &Value, kind: &str) -> String {
    let fallback = resolve_string(document.get("title"));
    let candidate = resolve_string(document.get("name"));
    if !fallback.is_empty() {
        return fallback;
    }
    if !candidate.is_empty() {
        return candidate;
    }
    let source = match kind {
        ALERT_RULE_KIND => "unknown-alert-rule",
        ALERT_CONTACT_POINT_KIND => "unknown-contact-point",
        ALERT_MUTE_TIMING_KIND => "unknown-mute-timing",
        ALERT_POLICY_KIND => "unknown-policy",
        ALERT_TEMPLATE_KIND => "unknown-template",
        _ => "unknown-alert-resource",
    };
    source.to_string()
}

fn append_references(resource: &mut AlertResourceContract, document: &Value) {
    let body = document
        .as_object()
        .and_then(|value| value.get("spec").or(Some(document)))
        .and_then(Value::as_object);
    let Some(body) = body else {
        return;
    };
    for key in [
        "datasourceUid",
        "datasourceName",
        "contactPointUid",
        "notificationPolicy",
        "templateUid",
    ] {
        if let Some(Value::String(text)) = body.get(key) {
            let normalized = text.trim().to_string();
            if !normalized.is_empty() {
                resource.references.insert(normalized);
            }
        }
    }
}

fn collect_alert_section(items: &[Value], kind: &str, target: &mut Vec<AlertResourceContract>) {
    for item in items {
        let Some(object) = item.as_object() else {
            continue;
        };
        let source_path = resolve_string(object.get("sourcePath"));
        let Some(document) = object
            .get("document")
            .or_else(|| object.get("body"))
            .or(Some(item))
        else {
            continue;
        };
        let identity = extract_identity(document, kind);
        let title = extract_title(document, kind);
        let mut contract = AlertResourceContract::from_kind_and_identity(
            kind,
            if identity.is_empty() {
                "unknown".to_string()
            } else {
                identity
            },
            title,
            source_path,
        );
        append_references(&mut contract, document);
        target.push(contract);
    }
}

/// collect alert bundle contracts.
pub fn collect_alert_bundle_contracts(source_bundle: &Value) -> Vec<AlertResourceContract> {
    let mut contracts = Vec::new();
    let alerting = source_bundle.get("alerting").and_then(Value::as_object);
    let alerting = match alerting {
        Some(value) => value,
        None => return contracts,
    };

    let rules = alerting
        .get("rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    collect_alert_section(rules.as_slice(), ALERT_RULE_KIND, &mut contracts);

    let contact_points = alerting
        .get("contactPoints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    collect_alert_section(
        contact_points.as_slice(),
        ALERT_CONTACT_POINT_KIND,
        &mut contracts,
    );

    let mute_timings = alerting
        .get("muteTimings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    collect_alert_section(
        mute_timings.as_slice(),
        ALERT_MUTE_TIMING_KIND,
        &mut contracts,
    );

    let policies = alerting
        .get("policies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    collect_alert_section(policies.as_slice(), ALERT_POLICY_KIND, &mut contracts);

    let templates = alerting
        .get("templates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    collect_alert_section(templates.as_slice(), ALERT_TEMPLATE_KIND, &mut contracts);
    contracts.sort_by(|left, right| left.identity.cmp(&right.identity));
    contracts
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn build_alert_bundle_contract_document(source_bundle: &Value) -> Value {
    let contracts = collect_alert_bundle_contracts(source_bundle);
    AlertBundleContractReport {
        resources: contracts,
    }
    .as_json()
}
