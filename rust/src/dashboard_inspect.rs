use regex::Regex;
use reqwest::Method;
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{message, object_field, string_field, value_as_object, Result};

use super::*;

pub(crate) fn render_csv(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    fn escape_csv(value: &str) -> String {
        if value.contains(',') || value.contains('"') || value.contains('\n') {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }

    let mut lines = Vec::new();
    lines.push(
        headers
            .iter()
            .map(|value| escape_csv(value))
            .collect::<Vec<String>>()
            .join(","),
    );
    for row in rows {
        lines.push(
            row.iter()
                .map(|value| escape_csv(value))
                .collect::<Vec<String>>()
                .join(","),
        );
    }
    lines
}

fn render_simple_table(
    headers: &[&str],
    rows: &[Vec<String>],
    include_header: bool,
) -> Vec<String> {
    let mut widths = headers
        .iter()
        .map(|value| value.len())
        .collect::<Vec<usize>>();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if index >= widths.len() {
                widths.push(value.len());
            } else {
                widths[index] = widths[index].max(value.len());
            }
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        let header_row = headers
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>();
        let divider_row = widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<String>>();
        lines.push(format_row(&header_row));
        lines.push(format_row(&divider_row));
    }
    for row in rows {
        lines.push(format_row(row));
    }
    lines
}

fn resolve_export_folder_path(
    document: &Map<String, Value>,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
) -> String {
    let folder_uid = object_field(document, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if !folder_uid.is_empty() {
        if let Some(folder) = folders_by_uid.get(&folder_uid) {
            return folder.path.clone();
        }
    }
    let relative_parent = dashboard_file
        .strip_prefix(import_dir)
        .ok()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| Path::new(""));
    let folder_name = relative_parent.display().to_string();
    if !folder_name.is_empty() && folder_name != "." && folder_name != DEFAULT_FOLDER_TITLE {
        let matches = folders_by_uid
            .values()
            .filter(|item| item.title == folder_name)
            .collect::<Vec<&FolderInventoryItem>>();
        if matches.len() == 1 {
            return matches[0].path.clone();
        }
    }
    if folder_name.is_empty() || folder_name == "." || folder_name == DEFAULT_FOLDER_TITLE {
        DEFAULT_FOLDER_TITLE.to_string()
    } else {
        folder_name
    }
}

fn collect_panel_stats(panel: &Map<String, Value>) -> (usize, usize) {
    let mut panel_count = 1usize;
    let mut query_count = panel
        .get("targets")
        .and_then(Value::as_array)
        .map(|targets| targets.len())
        .unwrap_or(0);
    if let Some(children) = panel.get("panels").and_then(Value::as_array) {
        for child in children {
            if let Some(child_object) = child.as_object() {
                let (child_panels, child_queries) = collect_panel_stats(child_object);
                panel_count += child_panels;
                query_count += child_queries;
            }
        }
    }
    (panel_count, query_count)
}

fn count_dashboard_panels_and_queries(dashboard: &Map<String, Value>) -> (usize, usize) {
    let mut panel_count = 0usize;
    let mut query_count = 0usize;
    if let Some(panels) = dashboard.get("panels").and_then(Value::as_array) {
        for panel in panels {
            if let Some(panel_object) = panel.as_object() {
                let (child_panels, child_queries) = collect_panel_stats(panel_object);
                panel_count += child_panels;
                query_count += child_queries;
            }
        }
    }
    (panel_count, query_count)
}

fn summarize_datasource_ref(reference: &Value) -> Option<String> {
    if reference.is_null() || is_builtin_datasource_ref(reference) {
        return None;
    }
    match reference {
        Value::String(text) => {
            if is_placeholder_string(text) {
                None
            } else {
                Some(text.to_string())
            }
        }
        Value::Object(object) => {
            for key in ["name", "uid", "type"] {
                if let Some(value) = object.get(key).and_then(Value::as_str) {
                    if !value.is_empty() && !is_placeholder_string(value) {
                        return Some(value.to_string());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn summarize_datasource_uid(reference: &Value) -> Option<String> {
    if reference.is_null() || is_builtin_datasource_ref(reference) {
        return None;
    }
    match reference {
        Value::String(text) => {
            if is_placeholder_string(text) {
                None
            } else {
                Some(text.to_string())
            }
        }
        Value::Object(object) => object
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value))
            .map(ToString::to_string),
        _ => None,
    }
}

fn summarize_datasource_inventory_usage(
    datasource: &DatasourceInventoryItem,
    usage_by_label: &std::collections::BTreeMap<
        String,
        (usize, std::collections::BTreeSet<String>),
    >,
) -> (usize, usize) {
    let mut labels = Vec::new();
    if !datasource.uid.is_empty() {
        labels.push(datasource.uid.as_str());
    }
    if !datasource.name.is_empty() && datasource.name != datasource.uid {
        labels.push(datasource.name.as_str());
    }
    let mut reference_count = 0usize;
    let mut dashboards = std::collections::BTreeSet::new();
    for label in labels {
        if let Some((count, dashboard_uids)) = usage_by_label.get(label) {
            reference_count += *count;
            dashboards.extend(dashboard_uids.iter().cloned());
        }
    }
    (reference_count, dashboards.len())
}

fn string_list_field(target: &Map<String, Value>, key: &str) -> Vec<String> {
    target
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn quoted_captures(text: &str, pattern: &str) -> Vec<String> {
    let regex = Regex::new(pattern).expect("invalid hard-coded query report regex");
    let mut values = std::collections::BTreeSet::new();
    for captures in regex.captures_iter(text) {
        if let Some(value) = captures.get(1).map(|item| item.as_str().trim()) {
            if !value.is_empty() {
                values.insert(value.to_string());
            }
        }
    }
    values.into_iter().collect()
}

fn extract_query_field_and_text(target: &Map<String, Value>) -> (String, String) {
    for key in ["expr", "expression", "query", "rawSql", "sql", "rawQuery"] {
        if let Some(value) = target.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return (key.to_string(), trimmed.to_string());
            }
        }
    }
    (String::new(), String::new())
}

fn extract_metric_names(query_text: &str) -> Vec<String> {
    if query_text.trim().is_empty() {
        return Vec::new();
    }
    let token_regex =
        Regex::new(r"[A-Za-z_:][A-Za-z0-9_:]*").expect("invalid hard-coded metric regex");
    let mut values = std::collections::BTreeSet::new();
    let reserved_words = [
        "and",
        "bool",
        "by",
        "group_left",
        "group_right",
        "ignoring",
        "offset",
        "on",
        "or",
        "unless",
        "without",
    ];
    for capture in quoted_captures(query_text, r#"__name__\s*=\s*"([A-Za-z_:][A-Za-z0-9_:]*)""#) {
        values.insert(capture);
    }
    for matched in token_regex.find_iter(query_text) {
        let start = matched.start();
        let end = matched.end();
        let previous = query_text[..start].chars().next_back();
        if previous
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let next = query_text[end..].chars().next();
        if next
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let token = matched.as_str();
        if reserved_words.contains(&token) {
            continue;
        }
        if query_text[end..].trim_start().starts_with('(') {
            continue;
        }
        values.insert(token.to_string());
    }
    values.into_iter().collect()
}

fn extract_query_measurements(target: &Map<String, Value>, query_text: &str) -> Vec<String> {
    let mut values = std::collections::BTreeSet::new();
    if let Some(measurement) = target.get("measurement").and_then(Value::as_str) {
        let trimmed = measurement.trim();
        if !trimmed.is_empty() {
            values.insert(trimmed.to_string());
        }
    }
    for value in string_list_field(target, "measurements") {
        values.insert(value);
    }
    for value in quoted_captures(query_text, r#"(?i)\bFROM\s+"?([A-Za-z0-9_.:-]+)"?"#) {
        values.insert(value);
    }
    for value in quoted_captures(query_text, r#"_measurement\s*==\s*"([^"]+)""#) {
        values.insert(value);
    }
    values.into_iter().collect()
}

fn extract_query_buckets(target: &Map<String, Value>, query_text: &str) -> Vec<String> {
    let mut values = std::collections::BTreeSet::new();
    if let Some(bucket) = target.get("bucket").and_then(Value::as_str) {
        let trimmed = bucket.trim();
        if !trimmed.is_empty() {
            values.insert(trimmed.to_string());
        }
    }
    for value in string_list_field(target, "buckets") {
        values.insert(value);
    }
    for value in quoted_captures(query_text, r#"from\s*\(\s*bucket\s*:\s*"([^"]+)""#) {
        values.insert(value);
    }
    values.into_iter().collect()
}

fn collect_query_report_rows(
    panels: &[Value],
    dashboard_uid: &str,
    dashboard_title: &str,
    folder_path: &str,
    rows: &mut Vec<ExportInspectionQueryRow>,
) {
    for panel in panels {
        let Some(panel_object) = panel.as_object() else {
            continue;
        };
        let panel_id = panel_object
            .get("id")
            .map(|value| match value {
                Value::Number(number) => number.to_string(),
                Value::String(text) => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();
        let panel_title = string_field(panel_object, "title", "");
        let panel_type = string_field(panel_object, "type", "");
        let panel_datasource = panel_object.get("datasource");
        if let Some(targets) = panel_object.get("targets").and_then(Value::as_array) {
            for target in targets {
                let Some(target_object) = target.as_object() else {
                    continue;
                };
                let datasource = target_object
                    .get("datasource")
                    .and_then(summarize_datasource_ref)
                    .or_else(|| panel_datasource.and_then(summarize_datasource_ref))
                    .unwrap_or_default();
                let datasource_uid = target_object
                    .get("datasource")
                    .and_then(summarize_datasource_uid)
                    .or_else(|| panel_datasource.and_then(summarize_datasource_uid))
                    .unwrap_or_default();
                let (query_field, query_text) = extract_query_field_and_text(target_object);
                let metrics = extract_metric_names(&query_text);
                let measurements = extract_query_measurements(target_object, &query_text);
                let buckets = extract_query_buckets(target_object, &query_text);
                rows.push(ExportInspectionQueryRow {
                    dashboard_uid: dashboard_uid.to_string(),
                    dashboard_title: dashboard_title.to_string(),
                    folder_path: folder_path.to_string(),
                    panel_id: panel_id.clone(),
                    panel_title: panel_title.clone(),
                    panel_type: panel_type.clone(),
                    ref_id: string_field(target_object, "refId", ""),
                    datasource,
                    datasource_uid,
                    query_field,
                    query_text,
                    metrics,
                    measurements,
                    buckets,
                });
            }
        }
        if let Some(children) = panel_object.get("panels").and_then(Value::as_array) {
            collect_query_report_rows(children, dashboard_uid, dashboard_title, folder_path, rows);
        }
    }
}

pub(crate) fn build_export_inspection_query_report(
    import_dir: &Path,
) -> Result<ExportInspectionQueryReport> {
    let summary = build_export_inspection_summary(import_dir)?;
    let metadata = load_export_metadata(import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let dashboard_files = discover_dashboard_files(import_dir)?;
    let folder_inventory = load_folder_inventory(import_dir, metadata.as_ref())?;
    let folders_by_uid = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect::<std::collections::BTreeMap<String, FolderInventoryItem>>();
    let mut rows = Vec::new();

    for dashboard_file in &dashboard_files {
        let document = load_json_file(dashboard_file)?;
        let document_object =
            value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let folder_path = resolve_export_folder_path(
            document_object,
            dashboard_file,
            import_dir,
            &folders_by_uid,
        );
        let dashboard_uid = string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
        let dashboard_title = string_field(dashboard, "title", DEFAULT_DASHBOARD_TITLE);
        if let Some(panels) = dashboard.get("panels").and_then(Value::as_array) {
            collect_query_report_rows(
                panels,
                &dashboard_uid,
                &dashboard_title,
                &folder_path,
                &mut rows,
            );
        }
    }

    Ok(ExportInspectionQueryReport {
        import_dir: summary.import_dir.clone(),
        summary: QueryReportSummary {
            dashboard_count: summary.dashboard_count,
            panel_count: summary.panel_count,
            query_count: summary.query_count,
            report_row_count: rows.len(),
        },
        queries: rows,
    })
}

pub(crate) fn resolve_report_column_ids(selected: &[String]) -> Result<Vec<String>> {
    if selected.is_empty() {
        return Ok(DEFAULT_REPORT_COLUMN_IDS
            .iter()
            .map(|value| value.to_string())
            .collect());
    }
    let mut result = Vec::new();
    for value in selected {
        let normalized = value.trim();
        if normalized.is_empty() {
            continue;
        }
        if !SUPPORTED_REPORT_COLUMN_IDS.contains(&normalized) {
            return Err(message(format!(
                "Unsupported --report-columns value {:?}. Supported columns: {}",
                normalized,
                SUPPORTED_REPORT_COLUMN_IDS.join(",")
            )));
        }
        if !result.iter().any(|item| item == normalized) {
            result.push(normalized.to_string());
        }
    }
    if result.is_empty() {
        return Err(message(format!(
            "--report-columns did not include any supported columns. Supported columns: {}",
            SUPPORTED_REPORT_COLUMN_IDS.join(",")
        )));
    }
    Ok(result)
}

fn report_column_header(column_id: &str) -> &'static str {
    match column_id {
        "dashboard_uid" => "DASHBOARD_UID",
        "dashboard_title" => "DASHBOARD_TITLE",
        "folder_path" => "FOLDER_PATH",
        "panel_id" => "PANEL_ID",
        "panel_title" => "PANEL_TITLE",
        "panel_type" => "PANEL_TYPE",
        "ref_id" => "REF_ID",
        "datasource" => "DATASOURCE",
        "datasource_uid" => "DATASOURCE_UID",
        "query_field" => "QUERY_FIELD",
        "metrics" => "METRICS",
        "measurements" => "MEASUREMENTS",
        "buckets" => "BUCKETS",
        "query" => "QUERY",
        _ => unreachable!("unsupported report column header"),
    }
}

fn render_query_report_column(row: &ExportInspectionQueryRow, column_id: &str) -> String {
    match column_id {
        "dashboard_uid" => row.dashboard_uid.clone(),
        "dashboard_title" => row.dashboard_title.clone(),
        "folder_path" => row.folder_path.clone(),
        "panel_id" => row.panel_id.clone(),
        "panel_title" => row.panel_title.clone(),
        "panel_type" => row.panel_type.clone(),
        "ref_id" => row.ref_id.clone(),
        "datasource" => row.datasource.clone(),
        "datasource_uid" => row.datasource_uid.clone(),
        "query_field" => row.query_field.clone(),
        "metrics" => row.metrics.join(","),
        "measurements" => row.measurements.join(","),
        "buckets" => row.buckets.join(","),
        "query" => row.query_text.clone(),
        _ => unreachable!("unsupported report column value"),
    }
}

#[derive(Clone, Debug, PartialEq)]
struct GroupedPanelReport {
    panel_id: String,
    panel_title: String,
    panel_type: String,
    queries: Vec<ExportInspectionQueryRow>,
}

#[derive(Clone, Debug, PartialEq)]
struct GroupedDashboardReport {
    dashboard_uid: String,
    dashboard_title: String,
    folder_path: String,
    panels: Vec<GroupedPanelReport>,
}

fn build_grouped_query_report(
    report: &ExportInspectionQueryReport,
) -> Vec<GroupedDashboardReport> {
    let mut dashboards = Vec::new();
    for row in &report.queries {
        let dashboard_index = dashboards
            .iter()
            .position(|item: &GroupedDashboardReport| item.dashboard_uid == row.dashboard_uid)
            .unwrap_or_else(|| {
                dashboards.push(GroupedDashboardReport {
                    dashboard_uid: row.dashboard_uid.clone(),
                    dashboard_title: row.dashboard_title.clone(),
                    folder_path: row.folder_path.clone(),
                    panels: Vec::new(),
                });
                dashboards.len() - 1
            });
        let panels = &mut dashboards[dashboard_index].panels;
        let panel_index = panels
            .iter()
            .position(|item| {
                item.panel_id == row.panel_id
                    && item.panel_title == row.panel_title
                    && item.panel_type == row.panel_type
            })
            .unwrap_or_else(|| {
                panels.push(GroupedPanelReport {
                    panel_id: row.panel_id.clone(),
                    panel_title: row.panel_title.clone(),
                    panel_type: row.panel_type.clone(),
                    queries: Vec::new(),
                });
                panels.len() - 1
            });
        panels[panel_index].queries.push(row.clone());
    }
    dashboards
}

pub(crate) fn render_grouped_query_report(
    report: &ExportInspectionQueryReport,
) -> Vec<String> {
    let grouped = build_grouped_query_report(report);
    let mut lines = Vec::new();
    lines.push(format!("Export inspection report: {}", report.import_dir));
    lines.push(String::new());
    lines.push("# Summary".to_string());
    lines.push(format!(
        "dashboards={} panels={} queries={} rows={}",
        report.summary.dashboard_count,
        report.summary.panel_count,
        report.summary.query_count,
        report.summary.report_row_count
    ));
    lines.push(String::new());
    lines.push("# Dashboard tree".to_string());
    for dashboard in grouped {
        let panel_count = dashboard.panels.len();
        let query_count = dashboard
            .panels
            .iter()
            .map(|panel| panel.queries.len())
            .sum::<usize>();
        lines.push(format!(
            "Dashboard: {} (uid={}, folder={}, panels={}, queries={})",
            dashboard.dashboard_title,
            dashboard.dashboard_uid,
            dashboard.folder_path,
            panel_count,
            query_count
        ));
        for panel in dashboard.panels {
            lines.push(format!(
                "  Panel: {} (id={}, type={}, queries={})",
                panel.panel_title,
                panel.panel_id,
                panel.panel_type,
                panel.queries.len()
            ));
            for query in panel.queries {
                let mut details = vec![format!("refId={}", query.ref_id)];
                if !query.datasource.is_empty() {
                    details.push(format!("datasource={}", query.datasource));
                }
                if !query.datasource_uid.is_empty() {
                    details.push(format!("datasourceUid={}", query.datasource_uid));
                }
                if !query.query_field.is_empty() {
                    details.push(format!("field={}", query.query_field));
                }
                if !query.metrics.is_empty() {
                    details.push(format!("metrics={}", query.metrics.join(",")));
                }
                if !query.measurements.is_empty() {
                    details.push(format!(
                        "measurements={}",
                        query.measurements.join(",")
                    ));
                }
                if !query.buckets.is_empty() {
                    details.push(format!("buckets={}", query.buckets.join(",")));
                }
                lines.push(format!("    Query: {}", details.join(" ")));
                if !query.query_text.is_empty() {
                    lines.push(format!("      {}", query.query_text));
                }
            }
        }
    }
    lines
}

pub(crate) fn apply_query_report_filters(
    mut report: ExportInspectionQueryReport,
    datasource_filter: Option<&str>,
    panel_id_filter: Option<&str>,
) -> ExportInspectionQueryReport {
    let datasource_filter = datasource_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let panel_id_filter = panel_id_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if datasource_filter.is_none() && panel_id_filter.is_none() {
        return report;
    }
    report.queries.retain(|row| {
        let datasource_match = datasource_filter
            .map(|value| row.datasource == value)
            .unwrap_or(true);
        let panel_match = panel_id_filter
            .map(|value| row.panel_id == value)
            .unwrap_or(true);
        datasource_match && panel_match
    });
    report.summary.dashboard_count = report
        .queries
        .iter()
        .map(|row| row.dashboard_uid.clone())
        .collect::<std::collections::BTreeSet<String>>()
        .len();
    report.summary.panel_count = report
        .queries
        .iter()
        .map(|row| {
            (
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                row.panel_title.clone(),
            )
        })
        .collect::<std::collections::BTreeSet<(String, String, String)>>()
        .len();
    report.summary.query_count = report.queries.len();
    report.summary.report_row_count = report.queries.len();
    report
}

pub(crate) fn validate_inspect_export_report_args(args: &InspectExportArgs) -> Result<()> {
    if args.report.is_none() {
        if !args.report_columns.is_empty() {
            return Err(message(
                "--report-columns is only supported together with --report.",
            ));
        }
        if args.report_filter_datasource.is_some() {
            return Err(message(
                "--report-filter-datasource is only supported together with --report.",
            ));
        }
        if args.report_filter_panel_id.is_some() {
            return Err(message(
                "--report-filter-panel-id is only supported together with --report.",
            ));
        }
        return Ok(());
    }
    if matches!(
        args.report,
        Some(InspectExportReportFormat::Json | InspectExportReportFormat::Tree)
    ) && !args.report_columns.is_empty()
    {
        return Err(message(
            "--report-columns is only supported with table or csv --report output.",
        ));
    }
    let _ = resolve_report_column_ids(&args.report_columns)?;
    Ok(())
}

pub(crate) fn build_export_inspection_summary(
    import_dir: &Path,
) -> Result<ExportInspectionSummary> {
    let metadata = load_export_metadata(import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let dashboard_files = discover_dashboard_files(import_dir)?;
    let folder_inventory = load_folder_inventory(import_dir, metadata.as_ref())?;
    let datasource_inventory = load_datasource_inventory(import_dir, metadata.as_ref())?;
    let folders_by_uid = folder_inventory
        .clone()
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect::<std::collections::BTreeMap<String, FolderInventoryItem>>();

    let mut folder_order = Vec::new();
    let mut folder_counts = std::collections::BTreeMap::new();
    let mut datasource_counts =
        std::collections::BTreeMap::<String, (usize, std::collections::BTreeSet<String>)>::new();
    let mut mixed_dashboards = Vec::new();
    let mut total_panels = 0usize;
    let mut total_queries = 0usize;

    let mut inventory_paths = folder_inventory
        .iter()
        .filter_map(|item| {
            let path = item.path.trim();
            if path.is_empty() {
                None
            } else {
                Some(path.to_string())
            }
        })
        .collect::<Vec<String>>();
    inventory_paths.sort();
    inventory_paths.dedup();
    for path in inventory_paths {
        folder_order.push(path.clone());
        folder_counts.insert(path, 0usize);
    }

    for dashboard_file in &dashboard_files {
        let document = load_json_file(dashboard_file)?;
        let document_object =
            value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let uid = string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
        let title = string_field(dashboard, "title", DEFAULT_DASHBOARD_TITLE);
        let folder_path = resolve_export_folder_path(
            document_object,
            dashboard_file,
            import_dir,
            &folders_by_uid,
        );
        if !folder_counts.contains_key(&folder_path) {
            folder_order.push(folder_path.clone());
            folder_counts.insert(folder_path.clone(), 0usize);
        }
        *folder_counts.entry(folder_path.clone()).or_insert(0usize) += 1;

        let (panel_count, query_count) = count_dashboard_panels_and_queries(dashboard);
        total_panels += panel_count;
        total_queries += query_count;

        let mut refs = Vec::new();
        collect_datasource_refs(&Value::Object(dashboard.clone()), &mut refs);
        let mut unique_datasources = std::collections::BTreeSet::new();
        for reference in refs {
            if let Some(label) = summarize_datasource_ref(&reference) {
                let usage = datasource_counts
                    .entry(label.clone())
                    .or_insert_with(|| (0usize, std::collections::BTreeSet::new()));
                usage.0 += 1;
                usage.1.insert(uid.clone());
                unique_datasources.insert(label);
            }
        }
        if unique_datasources.len() > 1 {
            mixed_dashboards.push(MixedDashboardSummary {
                uid,
                title,
                folder_path,
                datasource_count: unique_datasources.len(),
                datasources: unique_datasources.into_iter().collect(),
            });
        }
    }

    let folder_paths = folder_order
        .into_iter()
        .map(|path| ExportFolderUsage {
            dashboards: *folder_counts.get(&path).unwrap_or(&0usize),
            path,
        })
        .collect::<Vec<ExportFolderUsage>>();
    let mut datasource_usage = datasource_counts
        .iter()
        .map(
            |(datasource, (reference_count, dashboards))| ExportDatasourceUsage {
                datasource: datasource.clone(),
                reference_count: *reference_count,
                dashboard_count: dashboards.len(),
            },
        )
        .collect::<Vec<ExportDatasourceUsage>>();
    datasource_usage.sort_by(|left, right| left.datasource.cmp(&right.datasource));
    let mut datasource_inventory_summary = datasource_inventory
        .iter()
        .map(|datasource| {
            let (reference_count, dashboard_count) =
                summarize_datasource_inventory_usage(datasource, &datasource_counts);
            DatasourceInventorySummary {
                uid: datasource.uid.clone(),
                name: datasource.name.clone(),
                datasource_type: datasource.datasource_type.clone(),
                access: datasource.access.clone(),
                url: datasource.url.clone(),
                is_default: datasource.is_default.clone(),
                org: datasource.org.clone(),
                org_id: datasource.org_id.clone(),
                reference_count,
                dashboard_count,
            }
        })
        .collect::<Vec<DatasourceInventorySummary>>();
    datasource_inventory_summary.sort_by(|left, right| {
        left.org_id
            .cmp(&right.org_id)
            .then(left.name.cmp(&right.name))
            .then(left.uid.cmp(&right.uid))
    });
    mixed_dashboards.sort_by(|left, right| {
        left.folder_path
            .cmp(&right.folder_path)
            .then(left.title.cmp(&right.title))
            .then(left.uid.cmp(&right.uid))
    });

    Ok(ExportInspectionSummary {
        import_dir: import_dir.display().to_string(),
        dashboard_count: dashboard_files.len(),
        folder_count: folder_paths.len(),
        panel_count: total_panels,
        query_count: total_queries,
        datasource_inventory_count: datasource_inventory_summary.len(),
        mixed_dashboard_count: mixed_dashboards.len(),
        folder_paths,
        datasource_usage,
        datasource_inventory: datasource_inventory_summary,
        mixed_dashboards,
    })
}

pub(crate) fn analyze_export_dir(args: &InspectExportArgs) -> Result<usize> {
    validate_inspect_export_report_args(args)?;
    if let Some(report_format) = args.report {
        let report = apply_query_report_filters(
            build_export_inspection_query_report(&args.import_dir)?,
            args.report_filter_datasource.as_deref(),
            args.report_filter_panel_id.as_deref(),
        );
        if report_format == InspectExportReportFormat::Json {
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(report.summary.dashboard_count);
        }
        if report_format == InspectExportReportFormat::Tree {
            for line in render_grouped_query_report(&report) {
                println!("{line}");
            }
            return Ok(report.summary.dashboard_count);
        }

        let column_ids = resolve_report_column_ids(&args.report_columns)?;
        let rows = report
            .queries
            .iter()
            .map(|item| {
                column_ids
                    .iter()
                    .map(|column_id| render_query_report_column(item, column_id))
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<Vec<String>>>();
        let headers = column_ids
            .iter()
            .map(|column_id| report_column_header(column_id))
            .collect::<Vec<&str>>();

        if report_format == InspectExportReportFormat::Csv {
            for line in render_csv(&headers, &rows) {
                println!("{line}");
            }
            return Ok(report.summary.dashboard_count);
        }

        println!("Export inspection report: {}", report.import_dir);
        println!();
        println!("# Query report");
        for line in render_simple_table(&headers, &rows, !args.no_header) {
            println!("{line}");
        }
        return Ok(report.summary.dashboard_count);
    }

    let summary = build_export_inspection_summary(&args.import_dir)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(summary.dashboard_count);
    }

    println!("Export inspection: {}", summary.import_dir);
    if args.table {
        println!();
        println!("# Summary");
        let summary_rows = vec![
            vec![
                "dashboard_count".to_string(),
                summary.dashboard_count.to_string(),
            ],
            vec!["folder_count".to_string(), summary.folder_count.to_string()],
            vec!["panel_count".to_string(), summary.panel_count.to_string()],
            vec!["query_count".to_string(), summary.query_count.to_string()],
            vec![
                "datasource_inventory_count".to_string(),
                summary.datasource_inventory_count.to_string(),
            ],
            vec![
                "mixed_datasource_dashboard_count".to_string(),
                summary.mixed_dashboard_count.to_string(),
            ],
        ];
        for line in render_simple_table(&["METRIC", "VALUE"], &summary_rows, !args.no_header) {
            println!("{line}");
        }
    } else {
        println!("Dashboards: {}", summary.dashboard_count);
        println!("Folders: {}", summary.folder_count);
        println!("Panels: {}", summary.panel_count);
        println!("Queries: {}", summary.query_count);
        println!(
            "Datasource inventory: {}",
            summary.datasource_inventory_count
        );
        println!(
            "Mixed datasource dashboards: {}",
            summary.mixed_dashboard_count
        );
    }

    println!();
    println!("# Folder paths");
    let folder_rows = summary
        .folder_paths
        .iter()
        .map(|item| vec![item.path.clone(), item.dashboards.to_string()])
        .collect::<Vec<Vec<String>>>();
    for line in render_simple_table(
        &["FOLDER_PATH", "DASHBOARDS"],
        &folder_rows,
        !args.no_header,
    ) {
        println!("{line}");
    }

    println!();
    println!("# Datasource usage");
    let datasource_rows = summary
        .datasource_usage
        .iter()
        .map(|item| {
            vec![
                item.datasource.clone(),
                item.reference_count.to_string(),
                item.dashboard_count.to_string(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    for line in render_simple_table(
        &["DATASOURCE", "REFS", "DASHBOARDS"],
        &datasource_rows,
        !args.no_header,
    ) {
        println!("{line}");
    }

    if !summary.datasource_inventory.is_empty() {
        println!();
        println!("# Datasource inventory");
        let datasource_inventory_rows = summary
            .datasource_inventory
            .iter()
            .map(|item| {
                vec![
                    item.org_id.clone(),
                    item.uid.clone(),
                    item.name.clone(),
                    item.datasource_type.clone(),
                    item.access.clone(),
                    item.url.clone(),
                    item.is_default.clone(),
                    item.reference_count.to_string(),
                    item.dashboard_count.to_string(),
                ]
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(
            &[
                "ORG_ID",
                "UID",
                "NAME",
                "TYPE",
                "ACCESS",
                "URL",
                "IS_DEFAULT",
                "REFS",
                "DASHBOARDS",
            ],
            &datasource_inventory_rows,
            !args.no_header,
        ) {
            println!("{line}");
        }
    }

    if !summary.mixed_dashboards.is_empty() {
        println!();
        println!("# Mixed datasource dashboards");
        let mixed_rows = summary
            .mixed_dashboards
            .iter()
            .map(|item| {
                vec![
                    item.uid.clone(),
                    item.title.clone(),
                    item.folder_path.clone(),
                    item.datasources.join(","),
                ]
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(
            &["UID", "TITLE", "FOLDER_PATH", "DATASOURCES"],
            &mixed_rows,
            !args.no_header,
        ) {
            println!("{line}");
        }
    }
    Ok(summary.dashboard_count)
}

struct TempInspectLiveDir {
    path: PathBuf,
}

impl TempInspectLiveDir {
    fn new() -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| message(format!("Failed to build inspect-live temp path: {error}")))?
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "grafana-utils-inspect-live-{}-{timestamp}",
            process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempInspectLiveDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn build_live_export_args(args: &InspectLiveArgs, export_dir: PathBuf) -> ExportArgs {
    ExportArgs {
        common: args.common.clone(),
        export_dir,
        page_size: args.page_size,
        org_id: args.org_id,
        all_orgs: args.all_orgs,
        flat: false,
        overwrite: false,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        dry_run: false,
        progress: false,
        verbose: false,
    }
}

fn build_export_inspect_args_from_live(
    args: &InspectLiveArgs,
    import_dir: PathBuf,
) -> InspectExportArgs {
    InspectExportArgs {
        import_dir,
        json: args.json,
        table: args.table,
        report: args.report,
        report_columns: args.report_columns.clone(),
        report_filter_datasource: args.report_filter_datasource.clone(),
        report_filter_panel_id: args.report_filter_panel_id.clone(),
        no_header: args.no_header,
    }
}

pub(crate) fn inspect_live_dashboards_with_request<F>(
    mut request_json: F,
    args: &InspectLiveArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.all_orgs {
        return Err(message(
            "inspect-live does not yet support --all-orgs. Export dashboards first or inspect one org at a time.",
        ));
    }
    let temp_dir = TempInspectLiveDir::new()?;
    let export_args = build_live_export_args(args, temp_dir.path.clone());
    let _ = dashboard_export::export_dashboards_with_request(&mut request_json, &export_args)?;
    let inspect_args =
        build_export_inspect_args_from_live(args, temp_dir.path.join(RAW_EXPORT_SUBDIR));
    analyze_export_dir(&inspect_args)
}
