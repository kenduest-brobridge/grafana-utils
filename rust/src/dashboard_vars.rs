//! Live dashboard templating variable inspection helpers.
//! Fetches one dashboard JSON document and summarizes `dashboard.templating.list`
//! so operators know which `--var name=value` assignments to pass to screenshot
//! or other URL-based workflows.
use reqwest::Url;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::common::{message, object_field, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;

use super::dashboard_inspect_render::{render_csv, render_simple_table};
use super::dashboard_screenshot::parse_vars_query;
use super::{
    build_http_client, build_http_client_for_org, fetch_dashboard, InspectVarsArgs,
    SimpleOutputFormat,
};

/// Struct definition for DashboardVariableRow.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DashboardVariableRow {
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: String,
    pub label: String,
    pub current: String,
    pub datasource: String,
    pub query: String,
    pub multi: bool,
    pub include_all: bool,
    pub option_count: usize,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct DashboardVariableDocument {
    pub dashboard_uid: String,
    pub dashboard_title: String,
    pub variable_count: usize,
    pub variables: Vec<DashboardVariableRow>,
}

/// inspect dashboard variables.
pub(crate) fn inspect_dashboard_variables(args: &InspectVarsArgs) -> Result<()> {
    let dashboard_uid = resolve_dashboard_uid(args)?;
    let client = build_inspect_vars_client(args)?;
    let mut document = build_dashboard_variable_document(&client, &dashboard_uid)?;
    if let Some(vars_query) = args.vars_query.as_deref() {
        apply_vars_query_overrides(&mut document.variables, vars_query)?;
    }
    match args.output_format.unwrap_or(SimpleOutputFormat::Table) {
        SimpleOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&document)?),
        SimpleOutputFormat::Csv => {
            for line in render_csv(
                &[
                    "name",
                    "type",
                    "label",
                    "current",
                    "datasource",
                    "multi",
                    "include_all",
                    "option_count",
                    "options",
                ],
                &build_variable_table_rows(&document.variables),
            ) {
                println!("{line}");
            }
        }
        SimpleOutputFormat::Table => {
            for line in render_simple_table(
                &["NAME", "TYPE", "LABEL", "CURRENT", "DATASOURCE", "OPTIONS"],
                &document
                    .variables
                    .iter()
                    .map(|row| {
                        vec![
                            row.name.clone(),
                            row.variable_type.clone(),
                            row.label.clone(),
                            row.current.clone(),
                            row.datasource.clone(),
                            summarize_options(row),
                        ]
                    })
                    .collect::<Vec<Vec<String>>>(),
                !args.no_header,
            ) {
                println!("{line}");
            }
        }
    }
    Ok(())
}

fn build_inspect_vars_client(args: &InspectVarsArgs) -> Result<JsonHttpClient> {
    let mut common = args.common.clone();
    if let Some(dashboard_url) = args
        .dashboard_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let url = Url::parse(dashboard_url)
            .map_err(|error| message(format!("Invalid --dashboard-url: {error}")))?;
        if let Some(host) = url.host_str() {
            let mut base = format!("{}://{host}", url.scheme());
            if let Some(port) = url.port() {
                base.push(':');
                base.push_str(&port.to_string());
            }
            common.url = base;
        }
    }
    match args.org_id {
        Some(org_id) => build_http_client_for_org(&common, org_id),
        None => build_http_client(&common),
    }
}

fn resolve_dashboard_uid(args: &InspectVarsArgs) -> Result<String> {
    if let Some(dashboard_uid) = args
        .dashboard_uid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(dashboard_uid.to_string());
    }
    let dashboard_url = args
        .dashboard_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| message("Set --dashboard-uid or pass --dashboard-url."))?;
    let url = Url::parse(dashboard_url)
        .map_err(|error| message(format!("Invalid --dashboard-url: {error}")))?;
    let segments = url
        .path_segments()
        .map(|values| values.map(str::to_string).collect::<Vec<String>>())
        .unwrap_or_default();
    if segments.len() >= 3 && (segments[0] == "d" || segments[0] == "d-solo") {
        return Ok(segments[1].clone());
    }
    Err(message(
        "Unable to derive dashboard UID from --dashboard-url. Use a /d/... or /d-solo/... Grafana URL, or pass --dashboard-uid explicitly.",
    ))
}

fn build_dashboard_variable_document(
    client: &JsonHttpClient,
    dashboard_uid: &str,
) -> Result<DashboardVariableDocument> {
    let payload = fetch_dashboard(client, dashboard_uid)?;
    let object = value_as_object(&payload, "Unexpected dashboard payload from Grafana.")?;
    let dashboard = object_field(object, "dashboard").ok_or_else(|| {
        message(format!(
            "Dashboard UID {dashboard_uid} did not include a dashboard object."
        ))
    })?;
    let title = string_field(dashboard, "title", dashboard_uid);
    let variables = extract_dashboard_variables(dashboard)?;
    Ok(DashboardVariableDocument {
        dashboard_uid: dashboard_uid.to_string(),
        dashboard_title: title,
        variable_count: variables.len(),
        variables,
    })
}

fn apply_vars_query_overrides(rows: &mut [DashboardVariableRow], vars_query: &str) -> Result<()> {
    let overrides = parse_vars_query(vars_query)?;
    if overrides.is_empty() {
        return Ok(());
    }
    for row in rows.iter_mut() {
        if let Some((_, value)) = overrides.iter().find(|(name, _)| name == &row.name) {
            row.current = value.clone();
        }
    }
    Ok(())
}

/// extract dashboard variables.
pub(crate) fn extract_dashboard_variables(
    dashboard: &Map<String, Value>,
) -> Result<Vec<DashboardVariableRow>> {
    let templating = match object_field(dashboard, "templating") {
        Some(value) => value,
        None => return Ok(Vec::new()),
    };
    let entries = match templating.get("list").and_then(Value::as_array) {
        Some(value) => value,
        None => return Ok(Vec::new()),
    };
    let mut rows = Vec::new();
    for entry in entries {
        let object = value_as_object(entry, "Dashboard templating entry must be a JSON object.")?;
        let name = string_field(object, "name", "");
        if name.trim().is_empty() {
            continue;
        }
        let variable_type = string_field(object, "type", "");
        let label = string_field(object, "label", "");
        let current = object
            .get("current")
            .map(format_current_value)
            .unwrap_or_default();
        let datasource = object
            .get("datasource")
            .map(format_compact_value)
            .unwrap_or_default();
        let query = object
            .get("query")
            .map(format_compact_value)
            .unwrap_or_default();
        let multi = object
            .get("multi")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let include_all = object
            .get("includeAll")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let options = object
            .get("options")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(format_option_value)
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        rows.push(DashboardVariableRow {
            name,
            variable_type,
            label,
            current,
            datasource,
            query,
            multi,
            include_all,
            option_count: options.len(),
            options,
        });
    }
    Ok(rows)
}

fn build_variable_table_rows(rows: &[DashboardVariableRow]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                row.name.clone(),
                row.variable_type.clone(),
                row.label.clone(),
                row.current.clone(),
                row.datasource.clone(),
                row.multi.to_string(),
                row.include_all.to_string(),
                row.option_count.to_string(),
                summarize_options(row),
            ]
        })
        .collect()
}

fn summarize_options(row: &DashboardVariableRow) -> String {
    const LIMIT: usize = 6;
    if row.options.is_empty() {
        return String::new();
    }
    let mut preview = row
        .options
        .iter()
        .take(LIMIT)
        .cloned()
        .collect::<Vec<String>>();
    if row.options.len() > LIMIT {
        preview.push(format!("(+{} more)", row.options.len() - LIMIT));
    }
    preview.join(", ")
}

fn format_current_value(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let text = object
                .get("text")
                .map(format_compact_value)
                .unwrap_or_default();
            let raw = object
                .get("value")
                .map(format_compact_value)
                .unwrap_or_default();
            match (text.is_empty(), raw.is_empty()) {
                (false, false) if text != raw => format!("{text} ({raw})"),
                (false, _) => text,
                (_, false) => raw,
                _ => String::new(),
            }
        }
        _ => format_compact_value(value),
    }
}

fn format_option_value(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let text = object
                .get("text")
                .map(format_compact_value)
                .unwrap_or_default();
            let raw = object
                .get("value")
                .map(format_compact_value)
                .unwrap_or_default();
            if text.is_empty() {
                raw
            } else if raw.is_empty() || text == raw {
                text
            } else {
                format!("{text} ({raw})")
            }
        }
        _ => format_compact_value(value),
    }
}

fn format_compact_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(item) => item.to_string(),
        Value::Number(item) => item.to_string(),
        Value::String(item) => item.clone(),
        Value::Array(items) => items
            .iter()
            .map(format_compact_value)
            .filter(|value| !value.is_empty())
            .collect::<Vec<String>>()
            .join("|"),
        Value::Object(object) => {
            for key in ["uid", "name", "label", "text", "value", "type"] {
                let candidate = object
                    .get(key)
                    .map(format_compact_value)
                    .unwrap_or_default();
                if !candidate.is_empty() {
                    return candidate;
                }
            }
            serde_json::to_string(object).unwrap_or_default()
        }
    }
}
