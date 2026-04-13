//! Shared JSON loading and validation helpers for sync workflows.

use crate::common::{message, Result};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn load_json_value(path: &Path, label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Invalid JSON in {} {}: {error}",
            label,
            path.display()
        ))
    })
}

pub(crate) fn load_json_array_file(path: &Path, label: &str) -> Result<Vec<Value>> {
    let value = load_json_value(path, label)?;
    value.as_array().cloned().ok_or_else(|| {
        message(format!(
            "{label} file must contain a JSON array: {}",
            path.display()
        ))
    })
}

pub(crate) fn load_optional_json_object_file(
    path: Option<&PathBuf>,
    label: &str,
) -> Result<Option<Value>> {
    match path {
        None => Ok(None),
        Some(path) => {
            let value = load_json_value(path, label)?;
            if !value.is_object() {
                return Err(message(format!(
                    "{label} file must contain a JSON object: {}",
                    path.display()
                )));
            }
            Ok(Some(value))
        }
    }
}

pub(crate) fn append_unique_strings(target: &mut Vec<Value>, values: &[String]) {
    let mut seen = target
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    for value in values {
        if !value.trim().is_empty() && seen.insert(value.clone()) {
            target.push(Value::String(value.clone()));
        }
    }
}

pub(crate) fn discover_json_files(root: &Path, ignored_names: &[&str]) -> Result<Vec<PathBuf>> {
    fn visit(current: &Path, ignored_names: &[&str], files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit(&path, ignored_names, files)?;
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if ignored_names.contains(&file_name) {
                continue;
            }
            files.push(path);
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(root, ignored_names, &mut files)?;
    files.sort();
    Ok(files)
}

pub(crate) fn require_json_object<'a>(
    document: &'a Value,
    label: &str,
) -> Result<&'a Map<String, Value>> {
    document
        .as_object()
        .ok_or_else(|| message(format!("{label} must be a JSON object.")))
}

pub(crate) fn require_json_array<'a>(document: &'a Value, label: &str) -> Result<&'a Vec<Value>> {
    document
        .as_array()
        .ok_or_else(|| message(format!("{label} must be a JSON array.")))
}

pub(crate) fn require_json_object_field<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a Map<String, Value>> {
    object
        .get(key)
        .ok_or_else(|| message(format!("{label} is missing {key}.")))?
        .as_object()
        .ok_or_else(|| message(format!("{label} {key} must be a JSON object.")))
}

pub(crate) fn require_json_array_field<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a Vec<Value>> {
    object
        .get(key)
        .ok_or_else(|| message(format!("{label} is missing {key}.")))?
        .as_array()
        .ok_or_else(|| message(format!("{label} {key} must be a JSON array.")))
}
