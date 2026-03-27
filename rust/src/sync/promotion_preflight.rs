//! Staged promotion-preflight helpers.
//!
//! Purpose:
//! - Assess cross-environment folder and datasource remap needs before a
//!   promotion workflow moves from a source bundle toward a target inventory.
//! - Keep the first promotion workflow pure and reviewable by building on top
//!   of the existing staged bundle-preflight document.

use super::bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
    require_sync_bundle_preflight_summary,
};
use super::json::{require_json_object, require_json_object_field};
use crate::common::{message, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub const SYNC_PROMOTION_PREFLIGHT_KIND: &str = "grafana-utils-sync-promotion-preflight";
pub const SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncPromotionPreflightSummary {
    pub resource_count: i64,
    pub direct_match_count: i64,
    pub mapped_count: i64,
    pub missing_mapping_count: i64,
    pub bundle_blocking_count: i64,
    pub blocking_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromotionCheck {
    kind: String,
    identity: String,
    source_value: String,
    target_value: String,
    status: String,
    detail: String,
    blocking: bool,
}

impl SyncPromotionPreflightSummary {
    pub(crate) fn from_document(document: &Value) -> Result<Self> {
        let object = require_json_object(document, "Sync promotion preflight document")?;
        let summary =
            require_json_object_field(object, "summary", "Sync promotion preflight document")?;
        serde_json::from_value(Value::Object(summary.clone())).map_err(|error| {
            message(format!(
                "Sync promotion preflight summary is invalid: {error}"
            ))
        })
    }
}

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn nested_mapping(
    root: &Map<String, Value>,
    first: &str,
    second: Option<&str>,
) -> Map<String, Value> {
    match second {
        Some(second) => root
            .get(first)
            .and_then(Value::as_object)
            .and_then(|object| object.get(second))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
        None => root
            .get(first)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
    }
}

fn target_uids(document: &Map<String, Value>, key: &str) -> BTreeSet<String> {
    document
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|object| normalize_text(object.get("uid")))
        .filter(|value| !value.is_empty())
        .collect()
}

fn target_names(document: &Map<String, Value>, key: &str) -> BTreeSet<String> {
    document
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|object| {
            let name = normalize_text(object.get("name"));
            if name.is_empty() {
                normalize_text(object.get("title"))
            } else {
                name
            }
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn mapped_target(mapping: &Map<String, Value>, source_value: &str) -> String {
    mapping
        .get(source_value)
        .map(|value| normalize_text(Some(value)))
        .unwrap_or_default()
}

fn classify_mapping_check(
    kind: &str,
    identity: String,
    source_value: String,
    mapped_value: String,
    target_values: &BTreeSet<String>,
    missing_detail: String,
) -> Option<PromotionCheck> {
    if source_value.is_empty() {
        return None;
    }
    if target_values.contains(&source_value) {
        return Some(PromotionCheck {
            kind: kind.to_string(),
            identity,
            source_value: source_value.clone(),
            target_value: source_value,
            status: "direct".to_string(),
            detail: "Target inventory already contains the same identifier.".to_string(),
            blocking: false,
        });
    }
    if !mapped_value.is_empty() && target_values.contains(&mapped_value) {
        return Some(PromotionCheck {
            kind: kind.to_string(),
            identity,
            source_value,
            target_value: mapped_value,
            status: "mapped".to_string(),
            detail: "Promotion mapping resolves this source identifier onto the target inventory."
                .to_string(),
            blocking: false,
        });
    }
    Some(PromotionCheck {
        kind: kind.to_string(),
        identity,
        source_value,
        target_value: mapped_value,
        status: "missing-target".to_string(),
        detail: missing_detail,
        blocking: true,
    })
}

fn dashboard_folder_checks(
    source_bundle: &Map<String, Value>,
    target_inventory: &Map<String, Value>,
    mapping: &Map<String, Value>,
) -> Vec<PromotionCheck> {
    let target_folder_uids = target_uids(target_inventory, "folders");
    source_bundle
        .get("dashboards")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|dashboard| {
            let dashboard_uid = normalize_text(dashboard.get("uid"));
            let folder_uid = normalize_text(dashboard.get("folderUid"));
            classify_mapping_check(
                "folder-remap",
                if dashboard_uid.is_empty() {
                    "dashboard".to_string()
                } else {
                    dashboard_uid
                },
                folder_uid.clone(),
                mapped_target(mapping, &folder_uid),
                &target_folder_uids,
                "Dashboard folder UID is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            )
        })
        .collect()
}

fn datasource_reference_checks(
    source_bundle: &Map<String, Value>,
    target_inventory: &Map<String, Value>,
    uid_mapping: &Map<String, Value>,
    name_mapping: &Map<String, Value>,
) -> Vec<PromotionCheck> {
    let target_datasource_uids = target_uids(target_inventory, "datasources");
    let target_datasource_names = target_names(target_inventory, "datasources");
    let mut checks = Vec::new();
    for dashboard in source_bundle
        .get("dashboards")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(object) = dashboard.as_object() else {
            continue;
        };
        let dashboard_uid = normalize_text(object.get("uid"));
        let body = object.get("body").and_then(Value::as_object);
        for datasource_uid in body
            .and_then(|body| body.get("datasourceUids"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_uid = normalize_text(Some(datasource_uid));
            if let Some(check) = classify_mapping_check(
                "datasource-uid-remap",
                dashboard_uid.clone(),
                source_uid.clone(),
                mapped_target(uid_mapping, &source_uid),
                &target_datasource_uids,
                "Datasource UID is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            ) {
                checks.push(check);
            }
        }
        for datasource_name in body
            .and_then(|body| body.get("datasourceNames"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_name = normalize_text(Some(datasource_name));
            if let Some(check) = classify_mapping_check(
                "datasource-name-remap",
                dashboard_uid.clone(),
                source_name.clone(),
                mapped_target(name_mapping, &source_name),
                &target_datasource_names,
                "Datasource name is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            ) {
                checks.push(check);
            }
        }
    }
    for alert in source_bundle
        .get("alerts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(object) = alert.as_object() else {
            continue;
        };
        let alert_uid = normalize_text(object.get("uid"));
        let body = object.get("body").and_then(Value::as_object);
        for datasource_uid in body
            .and_then(|body| body.get("datasourceUids"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_uid = normalize_text(Some(datasource_uid));
            if source_uid.is_empty() || source_uid == "__expr__" || source_uid == "__dashboard__" {
                continue;
            }
            if let Some(check) = classify_mapping_check(
                "alert-datasource-uid-remap",
                alert_uid.clone(),
                source_uid.clone(),
                mapped_target(uid_mapping, &source_uid),
                &target_datasource_uids,
                "Alert datasource UID is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            ) {
                checks.push(check);
            }
        }
        for datasource_name in body
            .and_then(|body| body.get("datasourceNames"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_name = normalize_text(Some(datasource_name));
            if let Some(check) = classify_mapping_check(
                "alert-datasource-name-remap",
                alert_uid.clone(),
                source_name.clone(),
                mapped_target(name_mapping, &source_name),
                &target_datasource_names,
                "Alert datasource name is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            ) {
                checks.push(check);
            }
        }
    }
    checks
}

pub fn build_sync_promotion_preflight_document(
    source_bundle: &Value,
    target_inventory: &Value,
    availability: Option<&Value>,
    mapping: Option<&Value>,
) -> Result<Value> {
    let source_bundle = require_json_object(source_bundle, "Sync source bundle input")?;
    let target_inventory = require_json_object(target_inventory, "Sync target inventory input")?;
    let bundle_preflight = build_sync_bundle_preflight_document(
        &Value::Object(source_bundle.clone()),
        &Value::Object(target_inventory.clone()),
        availability,
    )?;
    let mapping = match mapping {
        Some(value) => require_json_object(value, "Sync promotion mapping input")?.clone(),
        None => Map::new(),
    };
    let folder_mapping = nested_mapping(&mapping, "folders", None);
    let datasource_uid_mapping = nested_mapping(&mapping, "datasources", Some("uids"));
    let datasource_name_mapping = nested_mapping(&mapping, "datasources", Some("names"));

    let mut checks = dashboard_folder_checks(source_bundle, target_inventory, &folder_mapping);
    checks.extend(datasource_reference_checks(
        source_bundle,
        target_inventory,
        &datasource_uid_mapping,
        &datasource_name_mapping,
    ));

    let direct_match_count = checks.iter().filter(|item| item.status == "direct").count() as i64;
    let mapped_count = checks.iter().filter(|item| item.status == "mapped").count() as i64;
    let missing_mapping_count = checks.iter().filter(|item| item.blocking).count() as i64;
    let bundle_summary = require_sync_bundle_preflight_summary(&bundle_preflight)?;
    let bundle_blocking_count = bundle_summary.sync_blocking_count
        + bundle_summary.provider_blocking_count
        + bundle_summary.alert_artifact_blocked_count;
    let resource_count = source_bundle
        .get("summary")
        .and_then(Value::as_object)
        .map(|summary| summary.values().filter_map(Value::as_i64).sum::<i64>())
        .unwrap_or(0);

    Ok(serde_json::json!({
        "kind": SYNC_PROMOTION_PREFLIGHT_KIND,
        "schemaVersion": SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION,
        "summary": SyncPromotionPreflightSummary {
            resource_count,
            direct_match_count,
            mapped_count,
            missing_mapping_count,
            bundle_blocking_count,
            blocking_count: bundle_blocking_count + missing_mapping_count,
        },
        "bundlePreflight": bundle_preflight,
        "mappingSummary": {
            "folderMappingCount": folder_mapping.len(),
            "datasourceUidMappingCount": datasource_uid_mapping.len(),
            "datasourceNameMappingCount": datasource_name_mapping.len(),
        },
        "checks": checks.iter().map(|item| serde_json::json!({
            "kind": item.kind,
            "identity": item.identity,
            "sourceValue": item.source_value,
            "targetValue": item.target_value,
            "status": item.status,
            "detail": item.detail,
            "blocking": item.blocking,
        })).collect::<Vec<_>>(),
    }))
}

pub fn render_sync_promotion_preflight_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(SYNC_PROMOTION_PREFLIGHT_KIND) {
        return Err(message(
            "Sync promotion preflight document kind is not supported.",
        ));
    }
    let summary = SyncPromotionPreflightSummary::from_document(document)?;
    let mapping_summary = require_json_object_field(
        require_json_object(document, "Sync promotion preflight document")?,
        "mappingSummary",
        "Sync promotion preflight document",
    )?;
    let bundle_preflight = document
        .get("bundlePreflight")
        .ok_or_else(|| message("Sync promotion preflight document is missing bundlePreflight."))?;
    let mut lines = vec![
        "Sync promotion preflight".to_string(),
        format!(
            "Summary: resources={} direct={} mapped={} missing-mappings={} bundle-blocking={} blocking={}",
            summary.resource_count,
            summary.direct_match_count,
            summary.mapped_count,
            summary.missing_mapping_count,
            summary.bundle_blocking_count,
            summary.blocking_count,
        ),
        format!(
            "Mappings: folders={} datasource-uids={} datasource-names={}",
            mapping_summary
                .get("folderMappingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            mapping_summary
                .get("datasourceUidMappingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            mapping_summary
                .get("datasourceNameMappingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        "Reason: promotion stays blocked until bundle-preflight blockers are cleared and cross-environment mappings resolve to real target identifiers.".to_string(),
        String::new(),
        "# Promotion checks".to_string(),
    ];
    if let Some(checks) = document.get("checks").and_then(Value::as_array) {
        if checks.is_empty() {
            lines.push(
                "- none status=ok detail=No cross-environment remaps are required.".to_string(),
            );
        } else {
            for check in checks {
                if let Some(object) = check.as_object() {
                    lines.push(format!(
                        "- {} identity={} source={} target={} status={} detail={}",
                        normalize_text(object.get("kind")),
                        normalize_text(object.get("identity")),
                        normalize_text(object.get("sourceValue")),
                        normalize_text(object.get("targetValue")),
                        normalize_text(object.get("status")),
                        normalize_text(object.get("detail")),
                    ));
                }
            }
        }
    }
    lines.push(String::new());
    lines.extend(render_sync_bundle_preflight_text(bundle_preflight)?);
    Ok(lines)
}
