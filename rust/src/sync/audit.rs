//! Sync audit and checksum lock helpers.
//!
//! Purpose:
//! - Build deterministic checksum snapshots for GitOps-managed Grafana resources.
//! - Compare live state against a staged lock file and report drift for CI/CD.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::common::{message, Result};

use super::workbench::{normalize_resource_specs, SyncResourceSpec};

pub const SYNC_LOCK_KIND: &str = "grafana-utils-sync-lock";
pub const SYNC_LOCK_SCHEMA_VERSION: i64 = 1;
pub const SYNC_AUDIT_KIND: &str = "grafana-utils-sync-audit";
pub const SYNC_AUDIT_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone)]
struct ManagedLockSpec {
    kind: String,
    identity: String,
    title: String,
    managed_fields: Vec<String>,
    source_path: String,
}

fn fnv1a64_hex(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_value).collect()),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, item)| (key.clone(), canonicalize_value(item)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn checksum_value(value: &Value) -> Result<String> {
    let serialized = serde_json::to_string(&canonicalize_value(value))?;
    Ok(fnv1a64_hex(&serialized))
}

fn build_live_index(
    raw_live_specs: &[Value],
) -> Result<BTreeMap<(String, String), SyncResourceSpec>> {
    let live_specs = normalize_resource_specs(raw_live_specs)?;
    let mut index = BTreeMap::new();
    for spec in live_specs {
        index.insert((spec.kind.clone(), spec.identity.clone()), spec);
    }
    Ok(index)
}

fn build_lock_specs_from_managed_specs(
    raw_managed_specs: &[Value],
) -> Result<Vec<ManagedLockSpec>> {
    let managed_specs = normalize_resource_specs(raw_managed_specs)?;
    Ok(managed_specs
        .into_iter()
        .map(|spec| ManagedLockSpec {
            kind: spec.kind,
            identity: spec.identity,
            title: spec.title,
            managed_fields: spec.managed_fields,
            source_path: spec.source_path,
        })
        .collect())
}

fn parse_lock_resources<'a>(lock_document: &'a Value) -> Result<&'a Vec<Value>> {
    if lock_document.get("kind").and_then(Value::as_str) != Some(SYNC_LOCK_KIND) {
        return Err(message("Sync lock document kind is not supported."));
    }
    lock_document
        .get("resources")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync lock document is missing resources."))
}

fn build_lock_specs_from_lock_document(lock_document: &Value) -> Result<Vec<ManagedLockSpec>> {
    let resources = parse_lock_resources(lock_document)?;
    let mut specs = Vec::with_capacity(resources.len());
    for resource in resources {
        let object = resource
            .as_object()
            .ok_or_else(|| message("Sync lock resource must be a JSON object."))?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        let identity = object
            .get("identity")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        if kind.is_empty() || identity.is_empty() {
            return Err(message(
                "Sync lock resources require non-empty kind and identity.",
            ));
        }
        let title = object
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(identity.as_str())
            .to_string();
        let managed_fields = object
            .get("managedFields")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect::<Vec<_>>();
        let source_path = object
            .get("sourcePath")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        specs.push(ManagedLockSpec {
            kind,
            identity,
            title,
            managed_fields,
            source_path,
        });
    }
    Ok(specs)
}

fn build_snapshot(body: &Map<String, Value>, managed_fields: &[String]) -> Value {
    let fields = if managed_fields.is_empty() {
        body.keys().cloned().collect::<BTreeSet<_>>()
    } else {
        managed_fields.iter().cloned().collect::<BTreeSet<_>>()
    };
    let mut snapshot = Map::new();
    for field in fields {
        snapshot.insert(
            field.clone(),
            body.get(&field).cloned().unwrap_or(Value::Null),
        );
    }
    Value::Object(snapshot)
}

fn build_sync_lock_document_from_specs(
    lock_specs: &[ManagedLockSpec],
    raw_live_specs: &[Value],
) -> Result<Value> {
    let live_index = build_live_index(raw_live_specs)?;
    let mut resources = Vec::with_capacity(lock_specs.len());
    let mut present_count = 0i64;
    let mut missing_live_count = 0i64;
    for spec in lock_specs {
        let key = (spec.kind.clone(), spec.identity.clone());
        if let Some(live_spec) = live_index.get(&key) {
            let snapshot = build_snapshot(&live_spec.body, &spec.managed_fields);
            let checksum = checksum_value(&snapshot)?;
            present_count += 1;
            resources.push(serde_json::json!({
                "kind": spec.kind,
                "identity": spec.identity,
                "title": live_spec.title,
                "status": "present",
                "managedFields": spec.managed_fields,
                "checksum": checksum,
                "snapshot": snapshot,
                "sourcePath": spec.source_path,
            }));
        } else {
            missing_live_count += 1;
            resources.push(serde_json::json!({
                "kind": spec.kind,
                "identity": spec.identity,
                "title": spec.title,
                "status": "missing-live",
                "managedFields": spec.managed_fields,
                "checksum": Value::Null,
                "snapshot": Value::Null,
                "sourcePath": spec.source_path,
            }));
        }
    }
    Ok(serde_json::json!({
        "kind": SYNC_LOCK_KIND,
        "schemaVersion": SYNC_LOCK_SCHEMA_VERSION,
        "summary": {
            "resourceCount": resources.len(),
            "presentCount": present_count,
            "missingLiveCount": missing_live_count,
        },
        "resources": resources,
    }))
}

pub(crate) fn build_sync_lock_document(
    raw_managed_specs: &[Value],
    raw_live_specs: &[Value],
) -> Result<Value> {
    let lock_specs = build_lock_specs_from_managed_specs(raw_managed_specs)?;
    build_sync_lock_document_from_specs(&lock_specs, raw_live_specs)
}

pub(crate) fn build_sync_lock_document_from_lock(
    lock_document: &Value,
    raw_live_specs: &[Value],
) -> Result<Value> {
    let lock_specs = build_lock_specs_from_lock_document(lock_document)?;
    build_sync_lock_document_from_specs(&lock_specs, raw_live_specs)
}

fn diff_snapshot_fields(baseline: Option<&Value>, current: Option<&Value>) -> Vec<String> {
    let baseline_object = baseline.and_then(Value::as_object);
    let current_object = current.and_then(Value::as_object);
    let mut fields = BTreeSet::new();
    if let Some(object) = baseline_object {
        fields.extend(object.keys().cloned());
    }
    if let Some(object) = current_object {
        fields.extend(object.keys().cloned());
    }
    fields
        .into_iter()
        .filter(|field| {
            baseline_object.and_then(|object| object.get(field))
                != current_object.and_then(|object| object.get(field))
        })
        .collect()
}

pub(crate) fn build_sync_audit_document(
    current_lock_document: &Value,
    baseline_lock_document: Option<&Value>,
) -> Result<Value> {
    let current_resources = parse_lock_resources(current_lock_document)?;
    let baseline_resources = match baseline_lock_document {
        Some(document) => Some(parse_lock_resources(document)?),
        None => None,
    };
    let mut baseline_index = BTreeMap::new();
    if let Some(resources) = baseline_resources {
        for resource in resources {
            let object = resource
                .as_object()
                .ok_or_else(|| message("Sync lock resource must be a JSON object."))?;
            let kind = object
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let identity = object
                .get("identity")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            baseline_index.insert((kind, identity), object.clone());
        }
    }

    let mut drifts = Vec::new();
    let mut in_sync_count = 0i64;
    let mut missing_lock_count = 0i64;
    let mut missing_live_count = 0i64;
    let mut current_present_count = 0i64;
    let mut current_missing_count = 0i64;

    for resource in current_resources {
        let object = resource
            .as_object()
            .ok_or_else(|| message("Sync lock resource must be a JSON object."))?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let identity = object
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let title = object
            .get("title")
            .cloned()
            .unwrap_or(Value::String(identity.clone()));
        let current_status = object
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("missing-live");
        if current_status == "present" {
            current_present_count += 1;
        } else {
            current_missing_count += 1;
            missing_live_count += 1;
        }
        let current_checksum = object.get("checksum").cloned().unwrap_or(Value::Null);
        let current_snapshot = object.get("snapshot");
        let key = (kind.clone(), identity.clone());
        let baseline = baseline_index.get(&key);
        let baseline_status = baseline
            .and_then(|row| row.get("status"))
            .and_then(Value::as_str);
        let baseline_checksum = baseline.and_then(|row| row.get("checksum")).cloned();
        let baseline_snapshot = baseline.and_then(|row| row.get("snapshot"));
        let drift_status = if current_status != "present" {
            Some("missing-live")
        } else if baseline_lock_document.is_none() {
            None
        } else if baseline.is_none() && baseline_lock_document.is_some() {
            Some("missing-lock")
        } else if baseline_checksum.as_ref() != Some(&current_checksum)
            || baseline_status != Some(current_status)
        {
            Some("drift-detected")
        } else {
            None
        };
        if let Some(status) = drift_status {
            if status == "missing-lock" {
                missing_lock_count += 1;
            }
            drifts.push(serde_json::json!({
                "kind": kind,
                "identity": identity,
                "title": title,
                "status": status,
                "baselineStatus": baseline_status.unwrap_or("missing-lock"),
                "currentStatus": current_status,
                "baselineChecksum": baseline_checksum.unwrap_or(Value::Null),
                "currentChecksum": current_checksum,
                "driftedFields": diff_snapshot_fields(baseline_snapshot, current_snapshot),
                "sourcePath": object.get("sourcePath").cloned().unwrap_or(Value::String(String::new())),
            }));
        } else {
            in_sync_count += 1;
        }
    }

    Ok(serde_json::json!({
        "kind": SYNC_AUDIT_KIND,
        "schemaVersion": SYNC_AUDIT_SCHEMA_VERSION,
        "summary": {
            "managedCount": current_resources.len(),
            "baselineCount": baseline_index.len(),
            "currentPresentCount": current_present_count,
            "currentMissingCount": current_missing_count,
            "inSyncCount": in_sync_count,
            "driftCount": drifts.len(),
            "missingLockCount": missing_lock_count,
            "missingLiveCount": missing_live_count,
        },
        "currentLock": current_lock_document,
        "baselineLock": baseline_lock_document.cloned().unwrap_or(Value::Null),
        "drifts": drifts,
    }))
}

pub(crate) fn render_sync_audit_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(SYNC_AUDIT_KIND) {
        return Err(message("Sync audit document kind is not supported."));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Sync audit document is missing summary."))?;
    let drifts = document
        .get("drifts")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync audit document is missing drifts."))?;
    let mut lines = vec![
        "Sync audit".to_string(),
        format!(
            "Managed: {} baseline={} current-present={} current-missing={}",
            summary
                .get("managedCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("baselineCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("currentPresentCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("currentMissingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        format!(
            "Drift: count={} in-sync={} missing-lock={} missing-live={}",
            summary
                .get("driftCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("inSyncCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("missingLockCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("missingLiveCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
    ];
    for drift in drifts {
        let object = drift
            .as_object()
            .ok_or_else(|| message("Sync audit drift row must be a JSON object."))?;
        let fields = object
            .get("driftedFields")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect::<Vec<_>>();
        lines.push(format!(
            "- [{}] {} {} fields={}",
            object
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            object
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            object
                .get("identity")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            if fields.is_empty() {
                "-".to_string()
            } else {
                fields.join(",")
            }
        ));
    }
    Ok(lines)
}
