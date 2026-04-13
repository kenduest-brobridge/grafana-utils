//! Datasource import payload and secret-value helpers.

use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

use crate::common::{message, Result};
use crate::datasource_secret::{
    build_secret_placeholder_plan, describe_secret_placeholder_plan, resolve_secret_placeholders,
};

use super::datasource_import_export_support::DatasourceImportRecord;

fn parse_secret_values_json(raw: &str, label: &str) -> Result<Map<String, Value>> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| message(format!("Invalid JSON for {label}: {error}")))?;
    let object = value
        .as_object()
        .cloned()
        .ok_or_else(|| message(format!("{label} must decode to a JSON object.")))?;
    Ok(object)
}

pub(crate) fn parse_secret_values_inputs(
    value: Option<&str>,
    file_path: Option<&Path>,
) -> Result<Option<Map<String, Value>>> {
    if value.is_some() && file_path.is_some() {
        return Err(message(
            "Choose either --secret-values or --secret-values-file, not both.",
        ));
    }
    if let Some(raw) = value {
        return Ok(Some(parse_secret_values_json(raw, "--secret-values")?));
    }
    let Some(path) = file_path else {
        return Ok(None);
    };
    let raw = fs::read_to_string(path).map_err(|error| {
        message(format!(
            "Failed to read datasource secret values file {}: {error}",
            path.display()
        ))
    })?;
    Ok(Some(parse_secret_values_json(
        &raw,
        "--secret-values-file",
    )?))
}

#[cfg(test)]
pub(crate) fn build_import_payload(record: &DatasourceImportRecord) -> Value {
    build_import_payload_with_secret_values(record, None)
        .expect("import payload without secret values should remain valid")
}

pub(crate) fn build_import_payload_with_secret_values(
    record: &DatasourceImportRecord,
    secret_values: Option<&Map<String, Value>>,
) -> Result<Value> {
    build_import_payload_with_secret_values_impl(record, secret_values)
}

fn build_import_payload_with_secret_values_impl(
    record: &DatasourceImportRecord,
    secret_values: Option<&Map<String, Value>>,
) -> Result<Value> {
    let mut payload = Map::from_iter(vec![
        ("name".to_string(), Value::String(record.name.clone())),
        (
            "type".to_string(),
            Value::String(record.datasource_type.clone()),
        ),
        ("url".to_string(), Value::String(record.url.clone())),
        ("access".to_string(), Value::String(record.access.clone())),
        ("uid".to_string(), Value::String(record.uid.clone())),
        ("isDefault".to_string(), Value::Bool(record.is_default)),
    ]);
    if let Some(value) = record.basic_auth {
        payload.insert("basicAuth".to_string(), Value::Bool(value));
    }
    if !record.basic_auth_user.is_empty() {
        payload.insert(
            "basicAuthUser".to_string(),
            Value::String(record.basic_auth_user.clone()),
        );
    }
    if !record.user.is_empty() {
        payload.insert("user".to_string(), Value::String(record.user.clone()));
    }
    if let Some(value) = record.with_credentials {
        payload.insert("withCredentials".to_string(), Value::Bool(value));
    }
    if !record.database.is_empty() {
        payload.insert(
            "database".to_string(),
            Value::String(record.database.clone()),
        );
    }
    if let Some(json_data) = &record.json_data {
        payload.insert("jsonData".to_string(), Value::Object(json_data.clone()));
    }
    if let Some(placeholders) = &record.secure_json_data_placeholders {
        let datasource_spec = Map::from_iter(vec![
            ("uid".to_string(), Value::String(record.uid.clone())),
            ("name".to_string(), Value::String(record.name.clone())),
            (
                "type".to_string(),
                Value::String(record.datasource_type.clone()),
            ),
            (
                "secureJsonDataPlaceholders".to_string(),
                Value::Object(placeholders.clone()),
            ),
        ]);
        let plan = build_secret_placeholder_plan(&datasource_spec)?;
        let secret_values = secret_values.ok_or_else(|| {
            message(format!(
                "Datasource import for '{}' requires --secret-values because secureJsonDataPlaceholders are present. {}",
                if record.uid.is_empty() { &record.name } else { &record.uid },
                describe_secret_placeholder_plan(&plan)
            ))
        })?;
        let resolved = resolve_secret_placeholders(&plan.placeholders, secret_values)?;
        if !resolved.is_empty() {
            payload.insert("secureJsonData".to_string(), Value::Object(resolved));
        }
    }
    Ok(Value::Object(payload))
}
