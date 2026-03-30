//! Shared overview JSON/loading/support helpers.

use super::OverviewInputField;
use crate::access::{ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION};
use crate::common::{message, Result};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

pub(super) fn load_json_value(path: &Path, label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Invalid JSON in {label} {}: {error}",
            path.display()
        ))
    })
}

pub(super) fn load_json_object_value(path: &Path, label: &str) -> Result<Value> {
    let value = load_json_value(path, label)?;
    if !value.is_object() {
        return Err(message(format!(
            "{label} file must contain a JSON object: {}",
            path.display()
        )));
    }
    Ok(value)
}

pub(super) fn load_json_array_value(path: &Path, label: &str) -> Result<Vec<Value>> {
    let value = load_json_value(path, label)?;
    value.as_array().cloned().ok_or_else(|| {
        message(format!(
            "{label} file must contain a JSON array: {}",
            path.display()
        ))
    })
}

pub(super) fn load_object_from_value(path: &Path, label: &str) -> Result<Map<String, Value>> {
    let value = load_json_object_value(path, label)?;
    value.as_object().cloned().ok_or_else(|| {
        message(format!(
            "{label} must contain a JSON object: {}",
            path.display()
        ))
    })
}

pub(super) fn load_access_export_records(
    export_dir: &Path,
    payload_filename: &str,
    expected_kind: &str,
    label: &str,
) -> Result<Vec<Map<String, Value>>> {
    let payload_path = export_dir.join(payload_filename);
    let metadata_path = export_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    let payload = load_object_from_value(&payload_path, label)?;
    let metadata = load_object_from_value(&metadata_path, label)?;

    let payload_kind = payload.get("kind").and_then(Value::as_str).ok_or_else(|| {
        message(format!(
            "{label} is missing kind: {}",
            payload_path.display()
        ))
    })?;
    if payload_kind != expected_kind {
        return Err(message(format!(
            "{label} kind mismatch in {}: expected {}, got {}",
            payload_path.display(),
            expected_kind,
            payload_kind
        )));
    }

    let metadata_kind = metadata
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            message(format!(
                "{label} metadata is missing kind: {}",
                metadata_path.display()
            ))
        })?;
    if metadata_kind != expected_kind {
        return Err(message(format!(
            "{label} metadata kind mismatch in {}: expected {}, got {}",
            metadata_path.display(),
            expected_kind,
            metadata_kind
        )));
    }

    let payload_version = payload
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            message(format!(
                "{label} is missing version: {}",
                payload_path.display()
            ))
        })?;
    let metadata_version = metadata
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            message(format!(
                "{label} metadata is missing version: {}",
                metadata_path.display()
            ))
        })?;
    if payload_version != metadata_version {
        return Err(message(format!(
            "{label} version mismatch between {} and {}: {} vs {}",
            payload_path.display(),
            metadata_path.display(),
            payload_version,
            metadata_version
        )));
    }
    if payload_version > ACCESS_EXPORT_VERSION {
        return Err(message(format!(
            "Unsupported access export version {} in {}. Supported <= {}.",
            payload_version,
            payload_path.display(),
            ACCESS_EXPORT_VERSION
        )));
    }

    let records = payload
        .get("records")
        .ok_or_else(|| {
            message(format!(
                "{label} is missing records list: {}",
                payload_path.display()
            ))
        })?
        .as_array()
        .ok_or_else(|| {
            message(format!(
                "{label} records must be a list in {}",
                payload_path.display()
            ))
        })?;
    let metadata_record_count = metadata
        .get("recordCount")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            message(format!(
                "{label} metadata is missing recordCount: {}",
                metadata_path.display()
            ))
        })? as usize;
    if metadata_record_count != records.len() {
        return Err(message(format!(
            "{label} recordCount mismatch between {} and {}: {} vs {}",
            payload_path.display(),
            metadata_path.display(),
            records.len(),
            metadata_record_count
        )));
    }

    records
        .iter()
        .map(|value| {
            value.as_object().cloned().ok_or_else(|| {
                message(format!(
                    "{label} entry must be a JSON object: {}",
                    payload_path.display()
                ))
            })
        })
        .collect()
}

pub(super) fn object_field_count(object: &Map<String, Value>, key: &str) -> usize {
    object
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

pub(super) fn value_is_truthy(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(true)) => true,
        Some(Value::String(text)) => text.trim().eq_ignore_ascii_case("true"),
        Some(Value::Number(number)) => number.as_i64() == Some(1),
        _ => false,
    }
}

pub(super) fn overview_inputs(pairs: &[(&str, String)]) -> Vec<OverviewInputField> {
    pairs
        .iter()
        .map(|(name, value)| OverviewInputField {
            name: (*name).to_string(),
            value: value.clone(),
        })
        .collect()
}
