use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::common::Result;
use crate::dashboard::{EXPORT_METADATA_FILENAME, ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION};

use super::snapshot_support::export_scope_kind_from_metadata_value;
use super::{
    SNAPSHOT_DATASOURCE_EXPORT_FILENAME, SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME,
    SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND, SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION,
};

fn load_json_value_file(path: &Path, label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|error| {
        crate::common::message(format!(
            "{label} must contain valid JSON in {}: {}",
            path.display(),
            error
        ))
    })
}

pub(super) fn load_snapshot_dashboard_metadata(dashboard_dir: &Path) -> Result<Value> {
    let metadata_path = dashboard_dir.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Err(crate::common::message(format!(
            "Snapshot dashboard export is missing metadata: {}",
            metadata_path.display()
        )));
    }
    let metadata = load_json_value_file(&metadata_path, "Snapshot dashboard export metadata")?;
    let kind = metadata
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let schema_version = metadata
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let variant = metadata
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if kind != ROOT_INDEX_KIND || schema_version != TOOL_SCHEMA_VERSION || variant != "root" {
        return Err(crate::common::message(format!(
            "Snapshot dashboard export metadata is not a supported root export: {}",
            metadata_path.display()
        )));
    }
    Ok(metadata)
}

pub(super) fn load_snapshot_dashboard_index(dashboard_dir: &Path) -> Result<Value> {
    let index_path = dashboard_dir.join("index.json");
    if index_path.is_file() {
        return load_json_value_file(&index_path, "Snapshot dashboard export index");
    }
    Ok(json!({
        "kind": ROOT_INDEX_KIND,
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "items": [],
        "variants": {
            "raw": null,
            "prompt": null,
            "provisioning": null
        },
        "folders": []
    }))
}

pub(super) fn build_dashboard_lane_summary(scope_dirs: &[PathBuf]) -> Value {
    let scope_count = scope_dirs.len() as u64;
    let raw_count = scope_dirs
        .iter()
        .filter(|scope| scope.join("raw").join("index.json").is_file())
        .count() as u64;
    let prompt_count = scope_dirs
        .iter()
        .filter(|scope| scope.join("prompt").join("index.json").is_file())
        .count() as u64;
    let provisioning_count = scope_dirs
        .iter()
        .filter(|scope| {
            scope.join("provisioning").join("index.json").is_file()
                && scope
                    .join("provisioning")
                    .join("provisioning")
                    .join("dashboards.yaml")
                    .is_file()
        })
        .count() as u64;
    json!({
        "scopeCount": scope_count,
        "rawScopeCount": raw_count,
        "promptScopeCount": prompt_count,
        "provisioningScopeCount": provisioning_count,
    })
}

pub(super) fn build_datasource_lane_summary(
    datasource_lane_dir: &Path,
    scope_dirs: &[PathBuf],
) -> Value {
    let scope_count = scope_dirs.len() as u64;
    let metadata_path = datasource_lane_dir.join(SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME);
    let metadata = fs::read_to_string(&metadata_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let has_non_root_scopes = scope_dirs.iter().any(|scope| scope != datasource_lane_dir);
    let scope_kind = export_scope_kind_from_metadata_value(&metadata);
    let inventory_scope_dirs: Vec<&PathBuf> =
        if matches!(scope_kind, "all-orgs-root" | "workspace-root") && has_non_root_scopes {
            scope_dirs
                .iter()
                .filter(|scope| scope.as_path() != datasource_lane_dir)
                .collect()
        } else {
            scope_dirs.iter().collect()
        };
    let inventory_count = inventory_scope_dirs
        .iter()
        .filter(|scope| scope.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME).is_file())
        .count() as u64;
    let provisioning_count = scope_dirs
        .iter()
        .filter(|scope| {
            scope
                .join("provisioning")
                .join("datasources.yaml")
                .is_file()
        })
        .count() as u64;
    json!({
        "scopeCount": scope_count,
        "inventoryExpectedScopeCount": inventory_scope_dirs.len() as u64,
        "inventoryScopeCount": inventory_count,
        "provisioningExpectedScopeCount": scope_count,
        "provisioningScopeCount": provisioning_count,
    })
}

pub(super) fn load_snapshot_datasource_rows(datasource_dir: &Path) -> Result<Vec<Value>> {
    let metadata_path = datasource_dir.join(SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME);
    let metadata = load_json_value_file(&metadata_path, "Snapshot datasource export metadata")?;
    let kind = metadata
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let schema_version = metadata
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let resource = metadata
        .get("resource")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if kind != SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND
        || schema_version != SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION
        || resource != "datasource"
        || !matches!(
            export_scope_kind_from_metadata_value(&metadata),
            "org-root" | "all-orgs-root" | "workspace-root"
        )
    {
        return Err(crate::common::message(format!(
            "Snapshot datasource export metadata is not a supported root export: {}",
            metadata_path.display()
        )));
    }

    let datasources_path = datasource_dir.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME);
    if !datasources_path.is_file() {
        return Err(crate::common::message(format!(
            "Snapshot datasource export is missing inventory: {}",
            datasources_path.display()
        )));
    }
    let raw = fs::read_to_string(&datasources_path)?;
    serde_json::from_str(&raw).map_err(|error| {
        crate::common::message(format!(
            "Snapshot datasource inventory must contain valid JSON in {}: {}",
            datasources_path.display(),
            error
        ))
    })
}
