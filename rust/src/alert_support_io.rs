use crate::common::{load_json_object_file, message, value_as_object, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::super::RAW_EXPORT_SUBDIR;

pub fn discover_alert_resource_files(input_dir: &Path) -> Result<Vec<PathBuf>> {
    if !input_dir.exists() {
        return Err(message(format!(
            "Import directory does not exist: {}",
            input_dir.display()
        )));
    }
    if !input_dir.is_dir() {
        return Err(message(format!(
            "Import path is not a directory: {}",
            input_dir.display()
        )));
    }
    if input_dir.join(RAW_EXPORT_SUBDIR).is_dir() {
        return Err(message(format!(
            "Import path {} looks like the export root. Point --input-dir at {}.",
            input_dir.display(),
            input_dir.join(RAW_EXPORT_SUBDIR).display()
        )));
    }

    let mut files = Vec::new();
    collect_resource_files(input_dir, &mut files)?;
    files.retain(|path| {
        !matches!(
            path.file_name().and_then(|value| value.to_str()),
            Some("index.json" | "index.yaml" | "index.yml")
        )
    });
    files.sort();
    if files.is_empty() {
        return Err(message(format!(
            "No alerting resource JSON or YAML files found in {}",
            input_dir.display()
        )));
    }
    Ok(files)
}

fn collect_resource_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_resource_files(&path, files)?;
            continue;
        }
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("json" | "yaml" | "yml")
        ) {
            files.push(path);
        }
    }
    Ok(())
}

pub fn load_alert_resource_file(path: &Path, object_label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let value = match extension {
        "yaml" | "yml" => serde_yaml::from_str::<Value>(&raw).map_err(|error| {
            message(format!("Failed to parse YAML {}: {error}", path.display()))
        })?,
        _ => serde_json::from_str::<Value>(&raw)?,
    };
    if !value.is_object() {
        return Err(message(format!(
            "{object_label} file must contain a top-level object: {}",
            path.display()
        )));
    }
    Ok(value)
}

pub fn write_alert_resource_file(path: &Path, payload: &Value, overwrite: bool) -> Result<()> {
    if path.exists() && !overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = match path.extension().and_then(|value| value.to_str()) {
        Some("yaml" | "yml") => serde_yaml::to_string(payload).map_err(|error| {
            message(format!("Failed to write YAML {}: {error}", path.display()))
        })?,
        _ => serde_json::to_string_pretty(payload)?,
    };
    fs::write(path, format!("{serialized}\n"))?;
    Ok(())
}

pub fn derive_dashboard_slug(value: &Value) -> String {
    let mut text = value.as_str().unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return String::new();
    }
    if let Some(index) = text.find("/d/") {
        let tail = &text[index + 3..];
        let mut segments = tail.split('/');
        let _uid = segments.next();
        if let Some(slug) = segments.next() {
            return slug
                .split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_string();
        }
    }
    if text.starts_with('/') {
        text = text
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
    }
    text
}

pub fn load_string_map(path: Option<&Path>, label: &str) -> Result<BTreeMap<String, String>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let payload = load_json_object_file(path, label)?;
    let object = value_as_object(&payload, &format!("{label} must be a JSON object."))?;
    Ok(object
        .iter()
        .map(|(key, value)| (key.clone(), value_to_string(value)))
        .collect())
}

pub fn load_panel_id_map(
    path: Option<&Path>,
) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let payload = load_json_object_file(path, "Panel ID map")?;
    let object = value_as_object(&payload, "Panel ID map must be a JSON object.")?;
    let mut normalized = BTreeMap::new();
    for (dashboard_uid, mapping_value) in object {
        let mapping_object = value_as_object(
            mapping_value,
            "Panel ID map values must be JSON objects keyed by source panel ID.",
        )?;
        normalized.insert(
            dashboard_uid.clone(),
            mapping_object
                .iter()
                .map(|(panel_id, target_panel_id)| {
                    (panel_id.clone(), value_to_string(target_panel_id))
                })
                .collect(),
        );
    }
    Ok(normalized)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AlertLinkageMappings {
    dashboard_uid_map: BTreeMap<String, String>,
    panel_id_map: BTreeMap<String, BTreeMap<String, String>>,
}

impl AlertLinkageMappings {
    pub(crate) fn load(
        dashboard_uid_path: Option<&Path>,
        panel_id_path: Option<&Path>,
    ) -> Result<AlertLinkageMappings> {
        Ok(AlertLinkageMappings {
            dashboard_uid_map: load_string_map(dashboard_uid_path, "Dashboard UID map")?,
            panel_id_map: load_panel_id_map(panel_id_path)?,
        })
    }

    pub(crate) fn resolve_dashboard_uid(&self, source_dashboard_uid: &str) -> String {
        self.dashboard_uid_map
            .get(source_dashboard_uid)
            .cloned()
            .unwrap_or_else(|| source_dashboard_uid.to_string())
    }

    pub(crate) fn resolve_panel_id(
        &self,
        source_dashboard_uid: &str,
        source_panel_id: &str,
    ) -> Option<String> {
        self.panel_id_map
            .get(source_dashboard_uid)
            .and_then(|mapping| mapping.get(source_panel_id))
            .cloned()
    }
}

pub(crate) fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}
