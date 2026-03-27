use serde_json::{Map, Value};
use std::collections::BTreeMap;

use super::datasource_catalog_data::{DatasourceCatalogEntry, DatasourcePresetProfile};
use super::datasource_catalog_lookup::find_supported_datasource_entry;

fn insert_json_data_default(json_data: &mut Map<String, Value>, key_path: &str, value: Value) {
    if let Some((prefix, suffix)) = key_path.split_once('.') {
        let child = json_data
            .entry(prefix.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(map) = child {
            insert_json_data_default(map, suffix, value);
        }
        return;
    }
    json_data.insert(key_path.to_string(), value);
}

fn build_full_json_data_scaffold(entry: &DatasourceCatalogEntry) -> Map<String, Value> {
    let mut json_data = Map::new();
    match entry.type_id {
        "loki" => {
            json_data.insert(
                "derivedFields".to_string(),
                serde_json::json!([
                    {
                        "name": "TraceID",
                        "matcherRegex": "traceID=(\\w+)",
                        "datasourceUid": "tempo",
                        "url": "$${__value.raw}",
                        "urlDisplayLabel": "View Trace"
                    }
                ]),
            );
        }
        "tempo" => {
            json_data.insert(
                "serviceMap".to_string(),
                serde_json::json!({
                    "datasourceUid": "prometheus"
                }),
            );
            json_data.insert(
                "tracesToLogsV2".to_string(),
                serde_json::json!({
                    "datasourceUid": "loki",
                    "spanStartTimeShift": "-1h",
                    "spanEndTimeShift": "1h"
                }),
            );
            json_data.insert(
                "tracesToMetrics".to_string(),
                serde_json::json!({
                    "datasourceUid": "prometheus",
                    "spanStartTimeShift": "-1h",
                    "spanEndTimeShift": "1h"
                }),
            );
        }
        "mysql" => {
            json_data.insert("tlsAuth".to_string(), Value::Bool(true));
            json_data.insert("tlsSkipVerify".to_string(), Value::Bool(true));
        }
        "postgresql" => {
            json_data.insert("postgresVersion".to_string(), Value::Number(903.into()));
            json_data.insert("timescaledb".to_string(), Value::Bool(false));
        }
        _ => {}
    }
    json_data
}

fn build_json_data_defaults(
    entry: &DatasourceCatalogEntry,
    preset_profile: DatasourcePresetProfile,
) -> Map<String, Value> {
    let mut json_data = Map::new();
    if let Some(http_method) = entry.add_defaults_http_method {
        json_data.insert(
            "httpMethod".to_string(),
            Value::String(http_method.to_string()),
        );
    }
    if let Some(time_field) = entry.add_defaults_time_field {
        json_data.insert(
            "timeField".to_string(),
            Value::String(time_field.to_string()),
        );
    }
    for (key, value) in entry.add_defaults_json_data {
        insert_json_data_default(&mut json_data, key, value.to_json_value());
    }
    if matches!(preset_profile, DatasourcePresetProfile::Full) {
        for (key, value) in build_full_json_data_scaffold(entry) {
            json_data.insert(key, value);
        }
    }
    json_data
}

pub(crate) fn build_add_defaults_document(entry: &DatasourceCatalogEntry) -> Value {
    let mut document = Map::new();
    if let Some(access) = entry.add_defaults_access {
        document.insert("access".to_string(), Value::String(access.to_string()));
    }
    let json_data = build_json_data_defaults(entry, DatasourcePresetProfile::Starter);
    if !json_data.is_empty() {
        document.insert("jsonData".to_string(), Value::Object(json_data));
    }
    Value::Object(document)
}

pub(crate) fn build_full_add_defaults_document(entry: &DatasourceCatalogEntry) -> Value {
    let mut document = Map::new();
    if let Some(access) = entry.add_defaults_access {
        document.insert("access".to_string(), Value::String(access.to_string()));
    }
    let mut json_data = build_json_data_defaults(entry, DatasourcePresetProfile::Starter);
    match entry.type_id {
        "loki" => {
            json_data.insert(
                "derivedFields".to_string(),
                serde_json::json!([
                    {
                        "name": "TraceID",
                        "matcherRegex": "traceID=(\\w+)",
                        "datasourceUid": "tempo",
                        "url": "$${__value.raw}",
                        "urlDisplayLabel": "View Trace"
                    }
                ]),
            );
        }
        "tempo" => {
            json_data.insert(
                "serviceMap".to_string(),
                serde_json::json!({
                    "datasourceUid": "prometheus"
                }),
            );
            json_data.insert(
                "tracesToLogsV2".to_string(),
                serde_json::json!({
                    "datasourceUid": "loki",
                    "spanStartTimeShift": "-1h",
                    "spanEndTimeShift": "1h"
                }),
            );
            json_data.insert(
                "tracesToMetrics".to_string(),
                serde_json::json!({
                    "datasourceUid": "prometheus",
                    "spanStartTimeShift": "-1h",
                    "spanEndTimeShift": "1h"
                }),
            );
        }
        "mysql" => {
            json_data.insert("tlsAuth".to_string(), Value::Bool(true));
            json_data.insert("tlsSkipVerify".to_string(), Value::Bool(true));
        }
        "postgresql" => {
            json_data.insert("postgresVersion".to_string(), Value::Number(903.into()));
            json_data.insert("timescaledb".to_string(), Value::Bool(false));
        }
        _ => {}
    }
    if !json_data.is_empty() {
        document.insert("jsonData".to_string(), Value::Object(json_data));
    }
    Value::Object(document)
}

pub fn build_add_defaults_for_supported_type(
    type_or_alias: &str,
    preset_profile: DatasourcePresetProfile,
) -> BTreeMap<String, Value> {
    let Some(entry) = find_supported_datasource_entry(type_or_alias) else {
        return BTreeMap::new();
    };
    let mut defaults = BTreeMap::new();
    if let Some(access) = entry.add_defaults_access {
        defaults.insert("access".to_string(), Value::String(access.to_string()));
    }
    if matches!(preset_profile, DatasourcePresetProfile::Full) {
        if let Some(http_method) = entry.add_defaults_http_method {
            defaults.insert(
                "httpMethod".to_string(),
                Value::String(http_method.to_string()),
            );
        }
        if let Some(time_field) = entry.add_defaults_time_field {
            defaults.insert(
                "timeField".to_string(),
                Value::String(time_field.to_string()),
            );
        }
    }
    let json_data = build_json_data_defaults(entry, preset_profile);
    if !json_data.is_empty() {
        defaults.insert("jsonData".to_string(), Value::Object(json_data));
    }
    defaults
}
