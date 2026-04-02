//! Query-report row collection and assembly helpers for dashboard inspection.
//!
//! Keeps the normalized row builder behind the `inspect.rs` facade so the rest of the
//! dashboard module can consume a stable report API.
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

use crate::common::{string_field, value_as_object, Result};

use super::super::files::{
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file,
};
use super::super::inspect_family::normalize_family_name;
use super::super::inspect_query::{
    dispatch_query_analysis, ordered_unique_push, QueryExtractionContext,
};
use super::super::inspect_report::{
    build_query_report, ExportInspectionQueryReport, ExportInspectionQueryRow,
};
use super::super::models::{DatasourceInventoryItem, FolderInventoryItem};
use super::extract_query_field_and_text;
use super::resolve_datasource_inventory_item;
use super::summarize_datasource_name;
use super::summarize_datasource_ref;
use super::summarize_datasource_type;
use super::summarize_datasource_uid;
use super::summarize_panel_datasource_key;
use super::{
    build_export_inspection_summary_for_variant, load_dashboard_org_scope_by_file,
    load_inspect_source_root, resolve_dashboard_source_file_path,
    resolve_export_folder_inventory_item, resolve_export_folder_path, DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE, DEFAULT_UNKNOWN_UID, RAW_EXPORT_SUBDIR,
};

struct QueryReportContext<'a> {
    export_org: &'a str,
    export_org_id: &'a str,
    dashboard: &'a Map<String, Value>,
    dashboard_uid: &'a str,
    dashboard_title: &'a str,
    folder_path: &'a str,
    folder_uid: &'a str,
    parent_folder_uid: &'a str,
    dashboard_file_display: &'a str,
    datasource_inventory: &'a [DatasourceInventoryItem],
}

static QUERY_VARIABLE_BRACED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?::[^}]*)?\}")
        .expect("invalid hard-coded variable regex")
});
static QUERY_VARIABLE_PLAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").expect("invalid hard-coded variable regex")
});
static QUERY_VARIABLE_DOUBLE_BRACKET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[([A-Za-z_][A-Za-z0-9_]*)(?::[^\]]*)?\]\]")
        .expect("invalid hard-coded variable regex")
});
static TEMPLATE_VARIABLE_NAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?m)(?:^|[^A-Za-z0-9_])\$(?:\{([A-Za-z_][A-Za-z0-9_]*)\}|([A-Za-z_][A-Za-z0-9_]*))"#,
    )
    .expect("invalid hard-coded dashboard template variable regex")
});

fn calculate_folder_level(folder_path: &str) -> String {
    let level = folder_path
        .split(" / ")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .count();
    if level == 0 {
        String::new()
    } else {
        level.to_string()
    }
}

fn calculate_folder_full_path(folder_path: &str) -> String {
    let segments = folder_path
        .split(" / ")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<&str>>();
    if segments.is_empty() || (segments.len() == 1 && segments[0] == DEFAULT_FOLDER_TITLE) {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn normalize_relative_dashboard_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn extract_dashboard_tags(dashboard: &Map<String, Value>) -> Vec<String> {
    dashboard
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            let mut values = Vec::new();
            for tag in tags {
                if let Some(value) = tag
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    ordered_unique_push(&mut values, value);
                }
            }
            values
        })
        .unwrap_or_default()
}

fn extract_query_variables(query_text: &str) -> Vec<String> {
    let regexes = [
        &*QUERY_VARIABLE_BRACED_REGEX,
        &*QUERY_VARIABLE_PLAIN_REGEX,
        &*QUERY_VARIABLE_DOUBLE_BRACKET_REGEX,
    ];
    let mut values = Vec::new();
    for regex in regexes {
        for capture in regex.captures_iter(query_text) {
            let Some(value) = capture.get(1).map(|item| item.as_str().trim()) else {
                continue;
            };
            if value.is_empty() {
                continue;
            }
            ordered_unique_push(&mut values, value);
        }
    }
    values
}

fn value_is_truthy(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(boolean)) => *boolean,
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0) != 0,
        Some(Value::String(text)) => {
            matches!(
                text.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes"
            )
        }
        _ => false,
    }
}

fn target_is_hidden(target: &Map<String, Value>) -> bool {
    value_is_truthy(target.get("hide"))
}

fn target_is_disabled(target: &Map<String, Value>) -> bool {
    value_is_truthy(target.get("disabled"))
}

fn extract_template_variable_names_from_text(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for captures in TEMPLATE_VARIABLE_NAME_REGEX.captures_iter(text) {
        let value = captures
            .get(1)
            .or_else(|| captures.get(2))
            .map(|item| item.as_str())
            .unwrap_or_default();
        ordered_unique_push(&mut values, value);
    }
    values
}

fn extract_template_variable_names_from_value(value: &Value, skip_keys: &[&str]) -> Vec<String> {
    fn visit(value: &Value, skip_keys: &[&str], values: &mut Vec<String>) {
        match value {
            Value::String(text) => {
                for variable in extract_template_variable_names_from_text(text) {
                    ordered_unique_push(values, &variable);
                }
            }
            Value::Array(items) => {
                for item in items {
                    visit(item, skip_keys, values);
                }
            }
            Value::Object(object) => {
                for (key, item) in object {
                    if skip_keys.iter().any(|skip_key| skip_key == &key.as_str()) {
                        continue;
                    }
                    visit(item, skip_keys, values);
                }
            }
            _ => {}
        }
    }

    let mut values = Vec::new();
    visit(value, skip_keys, &mut values);
    values
}

fn collect_query_report_rows(
    panels: &[Value],
    context: &QueryReportContext<'_>,
    rows: &mut Vec<ExportInspectionQueryRow>,
) {
    // Keep one output row per target so the query report, dependency contract, and
    // governance document all see the same normalized target-level facts.
    for panel in panels {
        let Some(panel_object) = panel.as_object() else {
            continue;
        };
        let dashboard_tags = extract_dashboard_tags(context.dashboard);
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
        let panel_datasource_value = panel_object.get("datasource");
        let panel_variables = extract_template_variable_names_from_value(
            &Value::Object(panel_object.clone()),
            &["targets", "panels"],
        );
        if let Some(targets) = panel_object.get("targets").and_then(Value::as_array) {
            let panel_target_count = targets
                .iter()
                .filter(|target| target.as_object().is_some())
                .count();
            let panel_query_count = targets
                .iter()
                .filter_map(Value::as_object)
                .filter(|target_object| {
                    !target_is_disabled(target_object)
                        && !extract_query_field_and_text(target_object)
                            .1
                            .trim()
                            .is_empty()
                })
                .count();
            let mut panel_datasource_keys = BTreeSet::new();
            for target in targets {
                let Some(target_object) = target.as_object() else {
                    continue;
                };
                if target_is_disabled(target_object) {
                    continue;
                }
                let panel_datasource_label = target_object
                    .get("datasource")
                    .and_then(summarize_panel_datasource_key)
                    .or_else(|| panel_datasource_value.and_then(summarize_panel_datasource_key))
                    .unwrap_or_default();
                if !panel_datasource_label.is_empty() {
                    panel_datasource_keys.insert(panel_datasource_label);
                }
            }
            let panel_datasource_count = panel_datasource_keys.len();
            for target in targets {
                let Some(target_object) = target.as_object() else {
                    continue;
                };
                let datasource = target_object
                    .get("datasource")
                    .and_then(summarize_datasource_ref)
                    .or_else(|| panel_datasource_value.and_then(summarize_datasource_ref))
                    .unwrap_or_default();
                let datasource_name = target_object
                    .get("datasource")
                    .and_then(|value| {
                        summarize_datasource_name(value, context.datasource_inventory)
                    })
                    .or_else(|| {
                        panel_datasource_value.and_then(|value| {
                            summarize_datasource_name(value, context.datasource_inventory)
                        })
                    })
                    .unwrap_or_default();
                let datasource_uid = target_object
                    .get("datasource")
                    .and_then(summarize_datasource_uid)
                    .or_else(|| panel_datasource_value.and_then(summarize_datasource_uid))
                    .unwrap_or_default();
                let datasource_type = target_object
                    .get("datasource")
                    .and_then(|value| {
                        summarize_datasource_type(value, context.datasource_inventory)
                    })
                    .or_else(|| {
                        panel_datasource_value.and_then(|value| {
                            summarize_datasource_type(value, context.datasource_inventory)
                        })
                    })
                    .unwrap_or_default();
                let datasource_inventory_item = target_object
                    .get("datasource")
                    .and_then(|value| {
                        resolve_datasource_inventory_item(value, context.datasource_inventory)
                    })
                    .or_else(|| {
                        panel_datasource_value.and_then(|value| {
                            resolve_datasource_inventory_item(value, context.datasource_inventory)
                        })
                    });
                let (query_field, query_text) = extract_query_field_and_text(target_object);
                let mut query_variables = extract_query_variables(&query_text);
                if query_variables.is_empty() {
                    query_variables = extract_template_variable_names_from_value(
                        &Value::Object(target_object.clone()),
                        &[],
                    );
                }
                let analysis = dispatch_query_analysis(&QueryExtractionContext {
                    panel: panel_object,
                    target: target_object,
                    query_field: &query_field,
                    query_text: &query_text,
                    resolved_datasource_type: &datasource_type,
                });
                rows.push(ExportInspectionQueryRow {
                    org: context.export_org.to_string(),
                    org_id: context.export_org_id.to_string(),
                    dashboard_uid: context.dashboard_uid.to_string(),
                    dashboard_title: context.dashboard_title.to_string(),
                    dashboard_tags: dashboard_tags.clone(),
                    folder_path: context.folder_path.to_string(),
                    folder_full_path: calculate_folder_full_path(context.folder_path),
                    folder_level: calculate_folder_level(context.folder_path),
                    folder_uid: context.folder_uid.to_string(),
                    parent_folder_uid: context.parent_folder_uid.to_string(),
                    panel_id: panel_id.clone(),
                    panel_title: panel_title.clone(),
                    panel_type: panel_type.clone(),
                    panel_target_count,
                    panel_query_count,
                    panel_datasource_count,
                    panel_variables: panel_variables.clone(),
                    ref_id: string_field(target_object, "refId", ""),
                    datasource,
                    datasource_name,
                    datasource_uid,
                    datasource_org: datasource_inventory_item
                        .map(|item| item.org.clone())
                        .unwrap_or_default(),
                    datasource_org_id: datasource_inventory_item
                        .map(|item| item.org_id.clone())
                        .unwrap_or_default(),
                    datasource_database: datasource_inventory_item
                        .map(|item| item.database.clone())
                        .unwrap_or_default(),
                    datasource_bucket: datasource_inventory_item
                        .map(|item| item.default_bucket.clone())
                        .unwrap_or_default(),
                    datasource_organization: datasource_inventory_item
                        .map(|item| item.organization.clone())
                        .unwrap_or_default(),
                    datasource_index_pattern: datasource_inventory_item
                        .map(|item| item.index_pattern.clone())
                        .unwrap_or_default(),
                    datasource_family: normalize_family_name(&datasource_type),
                    datasource_type,
                    query_field,
                    target_hidden: target_is_hidden(target_object).to_string(),
                    target_disabled: target_is_disabled(target_object).to_string(),
                    query_text,
                    query_variables,
                    metrics: analysis.metrics,
                    functions: analysis.functions,
                    measurements: analysis.measurements,
                    buckets: analysis.buckets,
                    file_path: context.dashboard_file_display.to_string(),
                });
            }
        }
        if let Some(children) = panel_object.get("panels").and_then(Value::as_array) {
            collect_query_report_rows(children, context, rows);
        }
    }
}

pub(crate) fn build_export_inspection_query_report(
    import_dir: &Path,
) -> Result<ExportInspectionQueryReport> {
    build_export_inspection_query_report_for_variant(import_dir, RAW_EXPORT_SUBDIR)
}

pub(crate) fn build_export_inspection_query_report_for_variant(
    import_dir: &Path,
    expected_variant: &str,
) -> Result<ExportInspectionQueryReport> {
    // Build the normalized row set once; every downstream output format should derive
    // from this report instead of re-reading dashboard files or re-running analysis.
    let summary = build_export_inspection_summary_for_variant(import_dir, expected_variant)?;
    let metadata = load_export_metadata(import_dir, Some(expected_variant))?;
    let dashboard_org_scope = load_dashboard_org_scope_by_file(import_dir, metadata.as_ref())?;
    let source_root = load_inspect_source_root(import_dir);
    let dashboard_files = discover_dashboard_files(import_dir)?;
    let datasource_inventory = load_datasource_inventory(import_dir, metadata.as_ref())?;
    let folder_inventory = load_folder_inventory(import_dir, metadata.as_ref())?;
    let folders_by_uid = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect::<BTreeMap<String, FolderInventoryItem>>();
    let mut rows = Vec::new();
    let export_org = summary.export_org.clone().unwrap_or_default();
    let export_org_id = summary.export_org_id.clone().unwrap_or_default();

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
        let relative_dashboard_path = dashboard_file
            .strip_prefix(import_dir)
            .map(normalize_relative_dashboard_path)
            .unwrap_or_else(|_| normalize_relative_dashboard_path(dashboard_file));
        let (dashboard_org, dashboard_org_id) = dashboard_org_scope
            .get(&relative_dashboard_path)
            .cloned()
            .unwrap_or_else(|| (export_org.clone(), export_org_id.clone()));
        let mut folder_record = resolve_export_folder_inventory_item(
            document_object,
            dashboard_file,
            import_dir,
            &folders_by_uid,
        );
        if folder_record
            .as_ref()
            .map(|item| item.uid.is_empty())
            .unwrap_or(true)
        {
            let scoped_matches = folders_by_uid
                .values()
                .filter(|item| {
                    item.path == folder_path
                        && ((!dashboard_org_id.is_empty() && item.org_id == dashboard_org_id)
                            || (!dashboard_org.is_empty() && item.org == dashboard_org))
                })
                .collect::<Vec<&FolderInventoryItem>>();
            if scoped_matches.len() == 1 {
                folder_record = Some((*scoped_matches[0]).clone());
            }
        }
        let folder_uid = folder_record
            .as_ref()
            .map(|item| item.uid.trim().to_string())
            .unwrap_or_default();
        let parent_folder_uid = folder_record
            .as_ref()
            .and_then(|item| item.parent_uid.clone())
            .unwrap_or_default();
        let dashboard_file_display =
            resolve_dashboard_source_file_path(import_dir, dashboard_file, source_root.as_deref());
        let context = QueryReportContext {
            export_org: &dashboard_org,
            export_org_id: &dashboard_org_id,
            dashboard,
            dashboard_uid: &dashboard_uid,
            dashboard_title: &dashboard_title,
            folder_path: &folder_path,
            folder_uid: &folder_uid,
            parent_folder_uid: &parent_folder_uid,
            dashboard_file_display: &dashboard_file_display,
            datasource_inventory: &datasource_inventory,
        };
        if let Some(panels) = dashboard.get("panels").and_then(Value::as_array) {
            collect_query_report_rows(panels, &context, &mut rows);
        }
    }

    Ok(build_query_report(
        source_root
            .as_ref()
            .map_or(import_dir, |value| value.as_path())
            .display()
            .to_string(),
        summary.dashboard_count,
        summary.panel_count,
        summary.query_count,
        rows,
    ))
}
