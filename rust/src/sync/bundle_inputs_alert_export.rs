//! Alert export directory loading for source-bundle inputs.

use super::bundle_inputs_alert_registry::{
    alert_export_section_for_path, ALERT_EXPORT_SECTION_SPECS,
};
use super::json::{discover_json_files, load_json_value};
use crate::common::Result;
use serde_json::{Map, Value};
use std::iter::FromIterator;
use std::path::Path;

pub(crate) fn load_alerting_bundle_section(output_dir: &Path) -> Result<Value> {
    let mut alerting = Map::from_iter(ALERT_EXPORT_SECTION_SPECS.iter().map(|spec| {
        (
            spec.section_key.to_string(),
            Value::Array(Vec::<Value>::new()),
        )
    }));
    for path in discover_json_files(output_dir, &["index.json", "export-metadata.json"])? {
        let relative_path = path
            .strip_prefix(output_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let Some(section_spec) = alert_export_section_for_path(&relative_path) else {
            continue;
        };
        let item = serde_json::json!({
            "sourcePath": relative_path,
            "document": load_json_value(&path, "Alert export document")?,
        });
        alerting
            .entry(section_spec.section_key.to_string())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .expect("alerting section array")
            .push(item);
    }
    alerting.insert(
        "summary".to_string(),
        Value::Object(alerting_summary(&alerting)),
    );
    let export_metadata_path = output_dir.join("export-metadata.json");
    if export_metadata_path.is_file() {
        alerting.insert(
            "exportMetadata".to_string(),
            load_json_value(&export_metadata_path, "Alert export metadata")?,
        );
    }
    alerting.insert(
        "exportDir".to_string(),
        Value::String(output_dir.display().to_string()),
    );
    Ok(Value::Object(alerting))
}

fn alerting_summary(alerting: &Map<String, Value>) -> Map<String, Value> {
    Map::from_iter(ALERT_EXPORT_SECTION_SPECS.iter().map(|spec| {
        (
            spec.summary_key.to_string(),
            Value::Number(
                alerting
                    .get(spec.section_key)
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0)
                    .into(),
            ),
        )
    }))
}
