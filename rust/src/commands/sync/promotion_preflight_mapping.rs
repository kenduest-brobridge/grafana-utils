//! Promotion-preflight mapping extraction and classification helpers.

use super::promotion_preflight_checks::{normalize_text, PromotionCheck};
use crate::common::{message, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(crate) const FOLDER_REMAP_KIND: &str = "folder-remap";
pub(crate) const DATASOURCE_UID_REMAP_KIND: &str = "datasource-uid-remap";
pub(crate) const ALERT_DATASOURCE_UID_REMAP_KIND: &str = "alert-datasource-uid-remap";
pub(crate) const DATASOURCE_NAME_REMAP_KIND: &str = "datasource-name-remap";
pub(crate) const ALERT_DATASOURCE_NAME_REMAP_KIND: &str = "alert-datasource-name-remap";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetInventoryField {
    Uid,
    NameOrTitle,
}

#[derive(Debug, Clone, Copy)]
struct RemapSpec {
    kind: &'static str,
    mapping_source: &'static str,
    target_collection: &'static str,
    target_field: TargetInventoryField,
    missing_detail: &'static str,
    ignored_source_values: &'static [&'static str],
}

const FOLDER_REMAP_SPEC: RemapSpec = RemapSpec {
    kind: FOLDER_REMAP_KIND,
    mapping_source: "folders",
    target_collection: "folders",
    target_field: TargetInventoryField::Uid,
    missing_detail:
        "Dashboard folder UID is missing from the target inventory and has no valid promotion mapping.",
    ignored_source_values: &[],
};

const DASHBOARD_DATASOURCE_UID_REMAP_SPEC: RemapSpec = RemapSpec {
    kind: DATASOURCE_UID_REMAP_KIND,
    mapping_source: "datasources.uids",
    target_collection: "datasources",
    target_field: TargetInventoryField::Uid,
    missing_detail:
        "Datasource UID is missing from the target inventory and has no valid promotion mapping.",
    ignored_source_values: &[],
};

const DASHBOARD_DATASOURCE_NAME_REMAP_SPEC: RemapSpec = RemapSpec {
    kind: DATASOURCE_NAME_REMAP_KIND,
    mapping_source: "datasources.names",
    target_collection: "datasources",
    target_field: TargetInventoryField::NameOrTitle,
    missing_detail:
        "Datasource name is missing from the target inventory and has no valid promotion mapping.",
    ignored_source_values: &[],
};

const ALERT_DATASOURCE_UID_REMAP_SPEC: RemapSpec = RemapSpec {
    kind: ALERT_DATASOURCE_UID_REMAP_KIND,
    mapping_source: "datasources.uids",
    target_collection: "datasources",
    target_field: TargetInventoryField::Uid,
    missing_detail:
        "Alert datasource UID is missing from the target inventory and has no valid promotion mapping.",
    ignored_source_values: &["__expr__", "__dashboard__"],
};

const ALERT_DATASOURCE_NAME_REMAP_SPEC: RemapSpec = RemapSpec {
    kind: ALERT_DATASOURCE_NAME_REMAP_KIND,
    mapping_source: "datasources.names",
    target_collection: "datasources",
    target_field: TargetInventoryField::NameOrTitle,
    missing_detail:
        "Alert datasource name is missing from the target inventory and has no valid promotion mapping.",
    ignored_source_values: &[],
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PromotionMappingSummaryDocument {
    pub(crate) mapping_kind: Value,
    pub(crate) mapping_schema_version: Value,
    pub(crate) source_environment: Value,
    pub(crate) target_environment: Value,
    pub(crate) folder_mapping_count: usize,
    pub(crate) datasource_uid_mapping_count: usize,
    pub(crate) datasource_name_mapping_count: usize,
}

pub(crate) fn nested_mapping(
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

pub(crate) fn parse_promotion_mapping_document(
    value: Option<&Value>,
) -> Result<(Map<String, Value>, Value)> {
    let Some(value) = value else {
        return Ok((Map::new(), Value::Object(Map::new())));
    };
    let object = super::super::json::require_json_object(value, "Sync promotion mapping input")?;
    let kind = normalize_text(object.get("kind"));
    if !kind.is_empty() && kind != super::promotion_preflight_checks::SYNC_PROMOTION_MAPPING_KIND {
        return Err(message(
            "Sync promotion mapping input kind is not supported.",
        ));
    }
    if let Some(schema_version) = object.get("schemaVersion").and_then(Value::as_i64) {
        if schema_version
            != super::promotion_preflight_checks::SYNC_PROMOTION_MAPPING_SCHEMA_VERSION
        {
            return Err(message(format!(
                "Sync promotion mapping schemaVersion must be {}.",
                super::promotion_preflight_checks::SYNC_PROMOTION_MAPPING_SCHEMA_VERSION
            )));
        }
    }
    let metadata = object
        .get("metadata")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let source_environment = metadata
        .get("sourceEnvironment")
        .cloned()
        .unwrap_or(Value::Null);
    let target_environment = metadata
        .get("targetEnvironment")
        .cloned()
        .unwrap_or(Value::Null);
    let mut mapping = object.clone();
    mapping.remove("kind");
    mapping.remove("schemaVersion");
    mapping.remove("metadata");
    Ok((
        mapping,
        serde_json::json!({
            "kind": if kind.is_empty() { Value::Null } else { Value::String(kind) },
            "schemaVersion": object.get("schemaVersion").cloned().unwrap_or(Value::Null),
            "sourceEnvironment": source_environment,
            "targetEnvironment": target_environment,
        }),
    ))
}

fn target_values(document: &Map<String, Value>, spec: RemapSpec) -> BTreeSet<String> {
    document
        .get(spec.target_collection)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|object| match spec.target_field {
            TargetInventoryField::Uid => normalize_text(object.get("uid")),
            TargetInventoryField::NameOrTitle => {
                let name = normalize_text(object.get("name"));
                if name.is_empty() {
                    normalize_text(object.get("title"))
                } else {
                    name
                }
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
    spec: RemapSpec,
    identity: String,
    source_value: String,
    mapped_value: String,
    target_values: &BTreeSet<String>,
) -> Option<PromotionCheck> {
    if source_value.is_empty()
        || spec
            .ignored_source_values
            .iter()
            .any(|ignored| *ignored == source_value)
    {
        return None;
    }
    if target_values.contains(&source_value) {
        return Some(PromotionCheck {
            kind: spec.kind.to_string(),
            identity,
            source_value: source_value.clone(),
            target_value: source_value,
            resolution: "direct-match".to_string(),
            mapping_source: "inventory".to_string(),
            status: "direct".to_string(),
            detail: "Target inventory already contains the same identifier.".to_string(),
            blocking: false,
        });
    }
    if !mapped_value.is_empty() && target_values.contains(&mapped_value) {
        return Some(PromotionCheck {
            kind: spec.kind.to_string(),
            identity,
            source_value,
            target_value: mapped_value,
            resolution: "explicit-map".to_string(),
            mapping_source: spec.mapping_source.to_string(),
            status: "mapped".to_string(),
            detail: "Promotion mapping resolves this source identifier onto the target inventory."
                .to_string(),
            blocking: false,
        });
    }
    Some(PromotionCheck {
        kind: spec.kind.to_string(),
        identity,
        source_value,
        target_value: mapped_value,
        resolution: "missing-map".to_string(),
        mapping_source: spec.mapping_source.to_string(),
        status: "missing-target".to_string(),
        detail: spec.missing_detail.to_string(),
        blocking: true,
    })
}

fn resource_body_reference_checks(
    source_bundle: &Map<String, Value>,
    target_inventory: &Map<String, Value>,
    mapping: &Map<String, Value>,
    resource_collection: &str,
    identity_field: &str,
    body_reference_field: &str,
    spec: RemapSpec,
) -> Vec<PromotionCheck> {
    let target_values = target_values(target_inventory, spec);
    let mut checks = Vec::new();
    for resource in source_bundle
        .get(resource_collection)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(object) = resource.as_object() else {
            continue;
        };
        let identity = normalize_text(object.get(identity_field));
        let body = object.get("body").and_then(Value::as_object);
        for reference in body
            .and_then(|body| body.get(body_reference_field))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_value = normalize_text(Some(reference));
            if let Some(check) = classify_mapping_check(
                spec,
                identity.clone(),
                source_value.clone(),
                mapped_target(mapping, &source_value),
                &target_values,
            ) {
                checks.push(check);
            }
        }
    }
    checks
}

pub(crate) fn dashboard_folder_checks(
    source_bundle: &Map<String, Value>,
    target_inventory: &Map<String, Value>,
    mapping: &Map<String, Value>,
) -> Vec<PromotionCheck> {
    let target_folder_uids = target_values(target_inventory, FOLDER_REMAP_SPEC);
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
                FOLDER_REMAP_SPEC,
                if dashboard_uid.is_empty() {
                    "dashboard".to_string()
                } else {
                    dashboard_uid
                },
                folder_uid.clone(),
                mapped_target(mapping, &folder_uid),
                &target_folder_uids,
            )
        })
        .collect()
}

pub(crate) fn datasource_reference_checks(
    source_bundle: &Map<String, Value>,
    target_inventory: &Map<String, Value>,
    uid_mapping: &Map<String, Value>,
    name_mapping: &Map<String, Value>,
) -> Vec<PromotionCheck> {
    let mut checks = Vec::new();
    checks.extend(resource_body_reference_checks(
        source_bundle,
        target_inventory,
        uid_mapping,
        "dashboards",
        "uid",
        "datasourceUids",
        DASHBOARD_DATASOURCE_UID_REMAP_SPEC,
    ));
    checks.extend(resource_body_reference_checks(
        source_bundle,
        target_inventory,
        name_mapping,
        "dashboards",
        "uid",
        "datasourceNames",
        DASHBOARD_DATASOURCE_NAME_REMAP_SPEC,
    ));
    checks.extend(resource_body_reference_checks(
        source_bundle,
        target_inventory,
        uid_mapping,
        "alerts",
        "uid",
        "datasourceUids",
        ALERT_DATASOURCE_UID_REMAP_SPEC,
    ));
    checks.extend(resource_body_reference_checks(
        source_bundle,
        target_inventory,
        name_mapping,
        "alerts",
        "uid",
        "datasourceNames",
        ALERT_DATASOURCE_NAME_REMAP_SPEC,
    ));
    checks
}
