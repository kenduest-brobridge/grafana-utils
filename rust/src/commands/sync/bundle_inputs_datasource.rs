//! Datasource bundle input normalization helpers.

use super::json::require_json_object;
use crate::common::{message, Result};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::fs;
use std::iter::FromIterator;
use std::path::Path;

pub(crate) fn normalize_datasource_bundle_item(document: &Value) -> Result<Value> {
    let object = require_json_object(document, "Datasource inventory record")?;
    let uid = object
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if uid.is_empty() && name.is_empty() {
        return Err(message("Datasource inventory record requires uid or name."));
    }
    let title = if name.is_empty() { uid } else { name };
    Ok(serde_json::json!({
        "kind": "datasource",
        "uid": uid,
        "name": title,
        "title": title,
        "body": {
            "uid": uid,
            "name": title,
            "type": object.get("type").cloned().unwrap_or(Value::String(String::new())),
            "access": object.get("access").cloned().unwrap_or(Value::String(String::new())),
            "url": object.get("url").cloned().unwrap_or(Value::String(String::new())),
            "isDefault": object.get("isDefault").cloned().unwrap_or(Value::Bool(false)),
        },
        "secureJsonDataProviders": object.get("secureJsonDataProviders").cloned().unwrap_or(Value::Object(Map::new())),
        "secureJsonDataPlaceholders": object.get("secureJsonDataPlaceholders").cloned().unwrap_or(Value::Object(Map::new())),
        "sourcePath": object.get("sourcePath").cloned().unwrap_or(Value::String(String::new())),
    }))
}

#[derive(Debug, Deserialize)]
struct DatasourceProvisioningDocument {
    #[serde(default)]
    datasources: Vec<DatasourceProvisioningRecord>,
}

#[derive(Debug, Deserialize)]
struct DatasourceProvisioningRecord {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    datasource_type: String,
    #[serde(default)]
    access: String,
    #[serde(default)]
    url: String,
    #[serde(default, rename = "isDefault")]
    is_default: bool,
    #[serde(default, rename = "orgId")]
    org_id: Option<i64>,
}

pub(crate) fn load_datasource_provisioning_records(provisioning_file: &Path) -> Result<Vec<Value>> {
    if !provisioning_file.is_file() {
        return Err(message(format!(
            "Datasource provisioning file does not exist: {}",
            provisioning_file.display()
        )));
    }
    let extension = provisioning_file
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !matches!(extension, "yaml" | "yml") {
        return Err(message(format!(
            "Datasource provisioning file must be YAML (.yaml or .yml): {}",
            provisioning_file.display()
        )));
    }
    let raw = fs::read_to_string(provisioning_file)?;
    let document: DatasourceProvisioningDocument = serde_yaml::from_str(&raw).map_err(|error| {
        message(format!(
            "Failed to parse datasource provisioning YAML {}: {error}",
            provisioning_file.display()
        ))
    })?;
    document
        .datasources
        .into_iter()
        .map(|datasource| {
            let uid = datasource.uid.trim().to_string();
            let name = datasource.name.trim().to_string();
            if uid.is_empty() && name.is_empty() {
                return Err(message(
                    "Datasource provisioning record requires uid or name.",
                ));
            }
            let title = if name.is_empty() {
                uid.clone()
            } else {
                name.clone()
            };
            let mut body = Map::new();
            body.insert("uid".to_string(), Value::String(uid.clone()));
            body.insert("name".to_string(), Value::String(title.clone()));
            body.insert(
                "type".to_string(),
                Value::String(datasource.datasource_type.trim().to_string()),
            );
            body.insert(
                "access".to_string(),
                Value::String(datasource.access.trim().to_string()),
            );
            body.insert(
                "url".to_string(),
                Value::String(datasource.url.trim().to_string()),
            );
            body.insert(
                "isDefault".to_string(),
                Value::String(datasource.is_default.to_string()),
            );
            let mut record = Map::from_iter([
                ("uid".to_string(), Value::String(uid)),
                ("name".to_string(), Value::String(title)),
                (
                    "type".to_string(),
                    Value::String(datasource.datasource_type.trim().to_string()),
                ),
                (
                    "access".to_string(),
                    Value::String(datasource.access.trim().to_string()),
                ),
                (
                    "url".to_string(),
                    Value::String(datasource.url.trim().to_string()),
                ),
                (
                    "isDefault".to_string(),
                    Value::String(datasource.is_default.to_string()),
                ),
                ("body".to_string(), Value::Object(body)),
                (
                    "sourcePath".to_string(),
                    Value::String(provisioning_file.display().to_string()),
                ),
            ]);
            if let Some(org_id) = datasource.org_id {
                record.insert("orgId".to_string(), Value::String(org_id.to_string()));
            }
            Ok(Value::Object(record))
        })
        .collect()
}
