//! Alert compare helpers for deterministic diff and export-index documents.
//!
//! Responsibilities:
//! - Normalize compare payloads into stable object ordering.
//! - Build and serialize compare payload envelopes consumed by import snapshots.
//! - Maintain ordered index and summary rendering for predictable diff output.

use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::common::Result;

pub(crate) fn build_compare_document(kind: &str, payload: &Map<String, Value>) -> Value {
    Value::Object(Map::from_iter([
        ("kind".to_string(), Value::String(kind.to_string())),
        ("spec".to_string(), Value::Object(payload.clone())),
    ]))
}

pub(crate) fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_value).collect()),
        Value::Object(object) => {
            let sorted = object
                .iter()
                .map(|(key, item)| (key.clone(), canonicalize_value(item)))
                .collect::<BTreeMap<_, _>>();
            Value::Object(Map::from_iter(sorted))
        }
        _ => value.clone(),
    }
}

pub(crate) fn serialize_compare_document(document: &Value) -> Result<String> {
    Ok(serde_json::to_string(&canonicalize_value(document))?)
}

pub(crate) fn build_compare_diff_text(
    remote_compare: &Value,
    local_compare: &Value,
    identity: &str,
    resource_file: &Path,
) -> Result<String> {
    let remote_pretty = serde_json::to_string_pretty(&canonicalize_value(remote_compare))?;
    let local_pretty = serde_json::to_string_pretty(&canonicalize_value(local_compare))?;
    let mut text = String::new();
    let _ = writeln!(&mut text, "--- remote:{identity}");
    let _ = writeln!(&mut text, "+++ {}", resource_file.display());
    for line in remote_pretty.lines() {
        let _ = writeln!(&mut text, "-{line}");
    }
    for line in local_pretty.lines() {
        let _ = writeln!(&mut text, "+{line}");
    }
    Ok(text)
}

pub(crate) fn build_resource_identity(kind: &str, payload: &Map<String, Value>) -> String {
    match kind {
        super::RULE_KIND => super::string_field(payload, "uid", "unknown"),
        super::CONTACT_POINT_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if uid.is_empty() {
                super::string_field(payload, "name", "unknown")
            } else {
                uid
            }
        }
        super::MUTE_TIMING_KIND | super::TEMPLATE_KIND => {
            super::string_field(payload, "name", "unknown")
        }
        super::POLICIES_KIND => super::string_field(payload, "receiver", "root"),
        _ => "unknown".to_string(),
    }
}

pub(crate) fn append_root_index_item(
    root_index: &mut Map<String, Value>,
    subdir: &str,
    item: Value,
) {
    let items = root_index
        .entry(subdir.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(entries) = items {
        entries.push(item);
    }
}

pub(crate) fn write_resource_indexes(
    resource_dirs: &BTreeMap<&'static str, std::path::PathBuf>,
    root_index: &Map<String, Value>,
) -> Result<()> {
    for (kind, subdir) in super::resource_subdir_by_kind() {
        let Some(Value::Array(items)) = root_index.get(subdir) else {
            continue;
        };
        super::write_json_file(
            &resource_dirs[kind].join("index.json"),
            &Value::Array(items.clone()),
            true,
        )?;
    }
    Ok(())
}

pub(crate) fn format_export_summary(root_index: &Map<String, Value>, index_path: &Path) -> String {
    format!(
        "Exported {} alert rules, {} contact points, {} mute timings, {} notification policy documents, {} templates. Root index: {}",
        root_index.get(super::RULES_SUBDIR).and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        root_index
            .get(super::CONTACT_POINTS_SUBDIR)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        root_index
            .get(super::MUTE_TIMINGS_SUBDIR)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        root_index
            .get(super::POLICIES_SUBDIR)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        root_index
            .get(super::TEMPLATES_SUBDIR)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        index_path.display(),
    )
}
