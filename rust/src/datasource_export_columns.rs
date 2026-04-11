//! Datasource list column discovery and rendering helpers.

use serde_json::{Map, Value};
use std::collections::BTreeSet;

use crate::common::{requested_columns_include_all, string_field};

const DATASOURCE_LIST_DEFAULT_COLUMNS: [&str; 5] = ["uid", "name", "type", "url", "is_default"];
const DATASOURCE_LIST_ORG_COLUMNS: [&str; 2] = ["org", "org_id"];
const DATASOURCE_LIST_DISCOVERABLE_COLUMNS: [&str; 14] = [
    "uid",
    "name",
    "type",
    "access",
    "url",
    "is_default",
    "basicAuth",
    "basicAuthUser",
    "database",
    "user",
    "withCredentials",
    "org",
    "org_id",
    "jsonData.<key>",
];

pub(crate) fn datasource_list_column_ids() -> &'static [&'static str] {
    &DATASOURCE_LIST_DISCOVERABLE_COLUMNS
}

fn normalize_datasource_column_id(id: &str) -> String {
    match id {
        "isDefault" => "is_default".to_string(),
        "orgId" => "org_id".to_string(),
        other => other.to_string(),
    }
}

fn datasource_record_path_segments(column: &str) -> Vec<String> {
    column
        .split('.')
        .map(|segment| match segment {
            "is_default" => "isDefault".to_string(),
            "org_id" => "orgId".to_string(),
            other => other.to_string(),
        })
        .collect()
}

fn datasource_column_header(column: &str) -> String {
    column.to_ascii_uppercase()
}

fn datasource_json_scalar_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn lookup_datasource_column_value<'a>(
    datasource: &'a Map<String, Value>,
    column: &str,
) -> Option<&'a Value> {
    let path = datasource_record_path_segments(column);
    let mut current = datasource.get(path.first()?)?;
    for segment in path.iter().skip(1) {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

fn lookup_datasource_column_text(datasource: &Map<String, Value>, column: &str) -> String {
    lookup_datasource_column_value(datasource, column)
        .map(datasource_json_scalar_text)
        .unwrap_or_default()
}

fn insert_projected_datasource_value(
    target: &mut Map<String, Value>,
    path: &[String],
    value: Value,
) {
    if path.is_empty() {
        return;
    }
    if path.len() == 1 {
        target.insert(path[0].clone(), value);
        return;
    }
    let entry = target
        .entry(path[0].clone())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    if let Some(object) = entry.as_object_mut() {
        insert_projected_datasource_value(object, &path[1..], value);
    }
}

fn project_datasource_record(
    datasource: &Map<String, Value>,
    selected_columns: &[String],
) -> Map<String, Value> {
    let mut projected = Map::new();
    for column in selected_columns {
        let path = datasource_record_path_segments(column);
        if let Some(value) = lookup_datasource_column_value(datasource, column).cloned() {
            insert_projected_datasource_value(&mut projected, &path, value);
        }
    }
    projected
}

fn collect_datasource_leaf_columns_from_value(
    prefix: &str,
    value: &Value,
    columns: &mut BTreeSet<String>,
) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let normalized_key = normalize_datasource_column_id(key);
                let child_prefix = if prefix.is_empty() {
                    normalized_key
                } else {
                    format!("{prefix}.{normalized_key}")
                };
                collect_datasource_leaf_columns_from_value(&child_prefix, child, columns);
            }
        }
        _ => {
            if !prefix.is_empty() {
                columns.insert(prefix.to_string());
            }
        }
    }
}

fn discover_all_datasource_columns(
    datasources: &[Map<String, Value>],
    include_org_scope: bool,
) -> Vec<String> {
    let mut discovered = BTreeSet::new();
    for datasource in datasources {
        for (key, value) in datasource {
            let normalized_key = normalize_datasource_column_id(key);
            collect_datasource_leaf_columns_from_value(&normalized_key, value, &mut discovered);
        }
    }
    let mut columns = DATASOURCE_LIST_DEFAULT_COLUMNS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<String>>();
    if include_org_scope {
        columns.extend(
            DATASOURCE_LIST_ORG_COLUMNS
                .iter()
                .map(|value| value.to_string()),
        );
    }
    for column in discovered {
        if !columns.iter().any(|item| item == &column) {
            columns.push(column);
        }
    }
    columns
}

fn resolve_datasource_list_columns(
    datasources: &[Map<String, Value>],
    include_org_scope: bool,
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    match selected_columns {
        None => {
            let mut columns = DATASOURCE_LIST_DEFAULT_COLUMNS
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<String>>();
            if include_org_scope {
                columns.extend(
                    DATASOURCE_LIST_ORG_COLUMNS
                        .iter()
                        .map(|value| value.to_string()),
                );
            }
            columns
        }
        Some(selected) if requested_columns_include_all(selected) => {
            discover_all_datasource_columns(datasources, include_org_scope)
        }
        Some(selected) => selected.to_vec(),
    }
}

fn data_source_rows_include_org_scope(datasources: &[Map<String, Value>]) -> bool {
    datasources.iter().any(|datasource| {
        !string_field(datasource, "org", "").is_empty()
            || !string_field(datasource, "orgId", "").is_empty()
    })
}

pub(crate) fn render_data_source_summary_line(
    datasource: &Map<String, Value>,
    selected_columns: Option<&[String]>,
) -> String {
    let include_org_scope = !string_field(datasource, "org", "").is_empty()
        || !string_field(datasource, "orgId", "").is_empty();
    let columns = resolve_datasource_list_columns(
        std::slice::from_ref(datasource),
        include_org_scope,
        selected_columns,
    );
    let values = columns
        .iter()
        .filter_map(|column| {
            let value = lookup_datasource_column_text(datasource, column);
            if value.is_empty() {
                None
            } else {
                Some(format!("{column}={value}"))
            }
        })
        .collect::<Vec<String>>();
    format!("- {}", values.join(" "))
}

pub(crate) fn render_data_source_table(
    datasources: &[Map<String, Value>],
    include_header: bool,
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    let include_org_scope = data_source_rows_include_org_scope(datasources);
    let columns = resolve_datasource_list_columns(datasources, include_org_scope, selected_columns);
    let headers = columns
        .iter()
        .map(|column| datasource_column_header(column))
        .collect::<Vec<String>>();
    let rows: Vec<Vec<String>> = datasources
        .iter()
        .map(|datasource| {
            columns
                .iter()
                .map(|column| lookup_datasource_column_text(datasource, column))
                .collect::<Vec<String>>()
        })
        .collect();
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let separator: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    let mut lines = Vec::new();
    if include_header {
        lines.extend([format_row(&headers), format_row(&separator)]);
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

pub(crate) fn render_data_source_csv(
    datasources: &[Map<String, Value>],
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    let include_org_scope = data_source_rows_include_org_scope(datasources);
    let columns = resolve_datasource_list_columns(datasources, include_org_scope, selected_columns);
    let mut lines = vec![columns
        .iter()
        .map(|column| match column.as_str() {
            "is_default" => "isDefault".to_string(),
            "org_id" => "orgId".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<String>>()
        .join(",")];
    lines.extend(datasources.iter().map(|datasource| {
        columns
            .iter()
            .map(|column| lookup_datasource_column_text(datasource, column))
            .map(|value| {
                if value.contains(',') || value.contains('"') || value.contains('\n') {
                    format!("\"{}\"", value.replace('"', "\"\""))
                } else {
                    value
                }
            })
            .collect::<Vec<String>>()
            .join(",")
    }));
    lines
}

pub(crate) fn render_data_source_json(
    datasources: &[Map<String, Value>],
    selected_columns: Option<&[String]>,
) -> Value {
    if selected_columns.is_none() || requested_columns_include_all(selected_columns.unwrap_or(&[]))
    {
        return Value::Array(
            datasources
                .iter()
                .cloned()
                .map(Value::Object)
                .collect::<Vec<Value>>(),
        );
    }
    let selected_columns = selected_columns.unwrap_or(&[]);
    Value::Array(
        datasources
            .iter()
            .map(|datasource| {
                Value::Object(project_datasource_record(datasource, selected_columns))
            })
            .collect(),
    )
}
