use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::common::string_field;

use super::{BUILTIN_DATASOURCE_NAMES, BUILTIN_DATASOURCE_TYPES};

const DEFAULT_GENERATED_DATASOURCE_INPUT: &str = "DATASOURCE";

fn known_datasource_type(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "prom" | "prometheus" => Some("prometheus"),
        "loki" => Some("loki"),
        "elastic" | "elasticsearch" => Some("elasticsearch"),
        "opensearch" => Some("grafana-opensearch-datasource"),
        "mysql" => Some("mysql"),
        "postgres" | "postgresql" => Some("postgres"),
        "mssql" => Some("mssql"),
        "influxdb" => Some("influxdb"),
        "tempo" => Some("tempo"),
        "jaeger" => Some("jaeger"),
        "zipkin" => Some("zipkin"),
        "cloudwatch" => Some("cloudwatch"),
        _ => None,
    }
}

/// datasource type alias.
pub(crate) fn datasource_type_alias(value: &str) -> &str {
    known_datasource_type(value).unwrap_or(value)
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedDatasource {
    pub(crate) key: String,
    pub(crate) input_label: String,
    pub(crate) ds_type: String,
    pub(crate) plugin_name: String,
    pub(crate) plugin_version: String,
}

#[derive(Clone, Debug)]
pub(crate) struct InputMapping {
    pub(crate) input_name: String,
    pub(crate) label: String,
    pub(crate) plugin_name: String,
    pub(crate) ds_type: String,
    pub(crate) plugin_version: String,
}

/// Struct definition for DatasourceCatalog.
pub struct DatasourceCatalog {
    pub(crate) by_uid: BTreeMap<String, Map<String, Value>>,
    pub(crate) by_name: BTreeMap<String, Map<String, Value>>,
}

/// Purpose: implementation note.
pub(crate) fn build_datasource_catalog(datasources: &[Map<String, Value>]) -> DatasourceCatalog {
    let mut by_uid = BTreeMap::new();
    let mut by_name = BTreeMap::new();
    for datasource in datasources {
        let uid = string_field(datasource, "uid", "");
        if !uid.is_empty() {
            by_uid.insert(uid, datasource.clone());
        }
        let name = string_field(datasource, "name", "");
        if !name.is_empty() {
            by_name.insert(name, datasource.clone());
        }
    }
    DatasourceCatalog { by_uid, by_name }
}

/// Purpose: implementation note.
pub(crate) fn is_placeholder_string(value: &str) -> bool {
    value.starts_with('$')
}

pub(crate) fn extract_placeholder_name(value: &str) -> String {
    if value.starts_with("${") && value.ends_with('}') && value.len() > 3 {
        return value[2..value.len() - 1].to_string();
    }
    if value.starts_with('$') && value.len() > 1 {
        return value[1..].to_string();
    }
    value.to_string()
}

fn is_generated_input_placeholder(value: &str) -> bool {
    extract_placeholder_name(value).starts_with("DS_")
}

/// Purpose: implementation note.
pub(crate) fn is_builtin_datasource_ref(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            BUILTIN_DATASOURCE_NAMES.contains(&text.as_str())
                || is_generated_input_placeholder(text)
        }
        Value::Object(object) => {
            let uid = object
                .get("uid")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let name = object
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let ds_type = object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            is_generated_input_placeholder(uid)
                || is_generated_input_placeholder(name)
                || BUILTIN_DATASOURCE_NAMES.contains(&uid)
                || BUILTIN_DATASOURCE_NAMES.contains(&name)
                || BUILTIN_DATASOURCE_TYPES.contains(&uid)
                || BUILTIN_DATASOURCE_TYPES.contains(&ds_type)
        }
        _ => false,
    }
}

/// collect datasource refs.
pub(crate) fn collect_datasource_refs(node: &Value, refs: &mut Vec<Value>) {
    match node {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "datasource" {
                    refs.push(value.clone());
                }
                collect_datasource_refs(value, refs);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_datasource_refs(item, refs);
            }
        }
        _ => {}
    }
}

pub(crate) fn make_input_name(label: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_underscore = false;
    for character in label.chars().flat_map(|character| character.to_uppercase()) {
        if character.is_ascii_alphanumeric() {
            normalized.push(character);
            last_was_underscore = false;
        } else if !last_was_underscore {
            normalized.push('_');
            last_was_underscore = true;
        }
    }
    let normalized = normalized.trim_matches('_').to_string();
    format!(
        "DS_{}",
        if normalized.is_empty() {
            DEFAULT_GENERATED_DATASOURCE_INPUT
        } else {
            &normalized
        }
    )
}

pub(crate) fn format_plugin_name(datasource_type: &str) -> String {
    match datasource_type_alias(datasource_type) {
        "cloudwatch" => return "CloudWatch".to_string(),
        "grafana-opensearch-datasource" => return "OpenSearch".to_string(),
        "influxdb" => return "InfluxDB".to_string(),
        "mssql" => return "Microsoft SQL Server".to_string(),
        "mysql" => return "MySQL".to_string(),
        "postgres" => return "PostgreSQL".to_string(),
        _ => {}
    }
    datasource_type_alias(datasource_type)
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

pub(crate) fn format_panel_plugin_name(panel_type: &str) -> String {
    match panel_type.to_ascii_lowercase().as_str() {
        "bargauge" => "Bar gauge".to_string(),
        "dashlist" => "Dash list".to_string(),
        "gauge" => "Gauge".to_string(),
        "heatmap" => "Heatmap".to_string(),
        "histogram" => "Histogram".to_string(),
        "logs" => "Logs".to_string(),
        "news" => "News".to_string(),
        "piechart" => "Pie chart".to_string(),
        "row" => "Row".to_string(),
        "state-timeline" => "State timeline".to_string(),
        "stat" => "Stat".to_string(),
        "status-history" => "Status history".to_string(),
        "table" => "Table".to_string(),
        "text" => "Text".to_string(),
        "timeseries" => "Time series".to_string(),
        _ => panel_type
            .replace(['-', '_'], " ")
            .split_whitespace()
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => format!(
                        "{}{}",
                        first.to_ascii_uppercase(),
                        chars.as_str().to_ascii_lowercase()
                    ),
                    None => String::new(),
                }
            })
            .collect::<Vec<String>>()
            .join(" "),
    }
}

pub(crate) fn make_input_label(datasource_type: &str, index: usize) -> String {
    let title = format_plugin_name(datasource_type);
    if index == 1 {
        format!("{title} datasource")
    } else {
        format!("{title} datasource {index}")
    }
}

pub(crate) fn build_resolved_datasource(
    key: String,
    ds_type: String,
    input_label: String,
) -> ResolvedDatasource {
    let plugin_name = format_plugin_name(&ds_type);
    ResolvedDatasource {
        key,
        input_label,
        ds_type,
        plugin_name,
        plugin_version: String::new(),
    }
}

pub(crate) fn datasource_plugin_version(datasource: &Map<String, Value>) -> String {
    if let Some(version) = datasource
        .get("pluginVersion")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return version.to_string();
    }
    if let Some(version) = datasource
        .get("version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return version.to_string();
    }
    datasource
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("info"))
        .and_then(Value::as_object)
        .and_then(|info| info.get("version"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// lookup datasource.
pub(crate) fn lookup_datasource(
    datasource_catalog: &DatasourceCatalog,
    uid: Option<&str>,
    name: Option<&str>,
) -> Option<Map<String, Value>> {
    if let Some(uid) = uid.filter(|value| !value.is_empty()) {
        if let Some(datasource) = datasource_catalog.by_uid.get(uid) {
            return Some(datasource.clone());
        }
    }
    if let Some(name) = name.filter(|value| !value.is_empty()) {
        return datasource_catalog.by_name.get(name).cloned();
    }
    None
}

/// Purpose: implementation note.
pub(crate) fn resolve_datasource_type_alias(
    reference: &str,
    datasource_catalog: &DatasourceCatalog,
) -> Option<String> {
    if let Some(alias) = known_datasource_type(reference) {
        return Some(alias.to_string());
    }
    let lower = reference.to_ascii_lowercase();
    for candidate in datasource_catalog.by_uid.values() {
        let candidate_type = string_field(candidate, "type", "");
        if !candidate_type.is_empty() && candidate_type.eq_ignore_ascii_case(&lower) {
            return Some(candidate_type);
        }
    }
    None
}
