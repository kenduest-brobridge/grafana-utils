//! Dashboard inspection pipeline for live systems and export directories.
//!
//! Input contract: callers hand this module either a raw export tree or a live-inspect
//! staging directory that has already been normalized into the same export shape
//! (`export-metadata.json`, `index.json`, folder inventory, datasource inventory, and
//! dashboard JSON files).
//! Normalized internal model: `ExportInspectionSummary`, `ExportInspectionQueryReport`,
//! `QueryAnalysis`, and the governance document rows are the shared shapes that every
//! renderer and parity test should consume.
//! Output contract: summary, query-report, dependency, and governance outputs are all
//! projections from that normalized model, so text/table/CSV/JSON parity comes from the
//! same row builders instead of separate code paths.
//!
//! Shortest modification path for maintainers:
//! - adjust query extraction in `inspect.rs`, `inspect_query.rs`, and
//!   `inspect_query_report.rs`
//! - adjust live staging in `inspect_live.rs`
//! - adjust summary/report/governance shape in `inspect_summary.rs`, `inspect_report.rs`,
//!   and `inspect_governance.rs`
//! - adjust rendering in `inspect_render.rs`
//! - adjust inspect regressions in `inspect_live_rust_tests.rs` first, then
//!   `dashboard_rust_tests.rs` when a behavior spans multiple paths
#[path = "inspect_extract.rs"]
mod inspect_extract;
#[path = "inspect_output.rs"]
mod inspect_output;
#[path = "inspect_query_report.rs"]
mod inspect_query_report;

use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::cli_defs::{InspectExportArgs, InspectExportReportFormat, InspectOutputFormat};
use super::files::{
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file,
};
#[cfg(feature = "tui")]
use super::inspect_governance::build_export_inspection_governance_document;
use super::inspect_live::load_variant_index_entries;
#[cfg(test)]
pub(crate) use super::inspect_query::QueryAnalysis;
#[allow(unused_imports)]
pub(crate) use super::inspect_query::{
    dispatch_query_analysis, ordered_unique_push,
    resolve_query_analyzer_family_from_datasource_type,
    resolve_query_analyzer_family_from_query_signature, QueryExtractionContext,
    DATASOURCE_FAMILY_UNKNOWN,
};
use super::inspect_render::render_simple_table;
use super::inspect_report::{
    refresh_filtered_query_report_summary, report_format_supports_columns,
    resolve_report_column_ids_for_format, ExportInspectionQueryReport,
};
use super::inspect_summary::{
    build_export_inspection_summary_document, build_export_inspection_summary_rows,
    DatasourceInventorySummary, ExportDatasourceUsage, ExportFolderUsage, ExportInspectionSummary,
    MixedDashboardSummary,
};
#[cfg(feature = "tui")]
use super::inspect_workbench::run_inspect_workbench;
#[cfg(feature = "tui")]
use super::inspect_workbench_support::build_inspect_workbench_document;
use super::models::{
    DatasourceInventoryItem, ExportMetadata, FolderInventoryItem, VariantIndexEntry,
};
use super::prompt::collect_datasource_refs;
use super::{
    DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID, DEFAULT_UNKNOWN_UID,
    RAW_EXPORT_SUBDIR,
};
use crate::common::{message, object_field, string_field, value_as_object, Result};
pub(crate) use inspect_extract::{
    extract_flux_pipeline_functions, extract_influxql_select_functions,
    extract_influxql_select_metrics, extract_influxql_time_windows, extract_metric_names,
    extract_prometheus_functions, extract_prometheus_metric_names,
    extract_prometheus_range_windows, extract_query_buckets, extract_query_field_and_text,
    extract_query_measurements, extract_sql_query_shape_hints, extract_sql_source_references,
    resolve_datasource_inventory_item, resolve_query_analyzer_family, summarize_datasource_name,
    summarize_datasource_ref, summarize_datasource_type, summarize_datasource_uid,
    summarize_panel_datasource_key,
};
use inspect_output::render_export_inspection_report_output;
pub(crate) use inspect_query_report::build_export_inspection_query_report;

pub(crate) use super::inspect_live::{prepare_inspect_export_import_dir, TempInspectDir};

const INSPECT_SOURCE_ROOT_FILENAME: &str = ".inspect-source-root";

fn normalize_index_entry_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if let Some((prefix, remainder)) = normalized.split_once('/') {
        if prefix.starts_with("org_") {
            if let Some((_raw_prefix, raw_remainder)) = remainder.rsplit_once("/raw/") {
                return format!("{prefix}/{raw_remainder}");
            }
            return format!("{prefix}/{}", remainder.trim_start_matches('/'));
        }
    }
    normalized
        .rsplit_once("/raw/")
        .map(|(_, remainder)| remainder.to_string())
        .unwrap_or(normalized)
}

fn write_inspect_output(output: &str, output_file: Option<&PathBuf>) -> Result<()> {
    let normalized = output.trim_end_matches('\n');
    if normalized.is_empty() {
        return Ok(());
    }
    if let Some(output_path) = output_file {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, format!("{normalized}\n"))?;
    }
    print!("{normalized}");
    println!();
    Ok(())
}

fn load_export_identity_values(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
    field_name: &str,
) -> Result<std::collections::BTreeSet<String>> {
    let mut values = std::collections::BTreeSet::new();
    if let Some(metadata) = metadata {
        let metadata_value = match field_name {
            "org" => metadata.org.clone().unwrap_or_default(),
            "orgId" => metadata.org_id.clone().unwrap_or_default(),
            _ => String::new(),
        };
        if !metadata_value.trim().is_empty() {
            values.insert(metadata_value.trim().to_string());
        }
    }
    let index_file = metadata
        .map(|item| item.index_file.clone())
        .unwrap_or_else(|| "index.json".to_string());
    let index_path = import_dir.join(&index_file);
    if index_path.is_file() {
        let raw = fs::read_to_string(&index_path)?;
        let entries: Vec<VariantIndexEntry> = serde_json::from_str(&raw).map_err(|error| {
            message(format!(
                "Invalid dashboard export index in {}: {error}",
                index_path.display()
            ))
        })?;
        for entry in entries {
            let value = match field_name {
                "org" => entry.org.trim(),
                "orgId" => entry.org_id.trim(),
                _ => "",
            };
            if !value.is_empty() {
                values.insert(value.to_string());
            }
        }
    }
    for folder in load_folder_inventory(import_dir, metadata)? {
        let value = match field_name {
            "org" => folder.org.trim(),
            "orgId" => folder.org_id.trim(),
            _ => "",
        };
        if !value.is_empty() {
            values.insert(value.to_string());
        }
    }
    for datasource in load_datasource_inventory(import_dir, metadata)? {
        let value = match field_name {
            "org" => datasource.org.trim(),
            "orgId" => datasource.org_id.trim(),
            _ => "",
        };
        if !value.is_empty() {
            values.insert(value.to_string());
        }
    }
    Ok(values)
}

fn resolve_export_identity_field(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
    field_name: &str,
) -> Result<Option<String>> {
    let values = load_export_identity_values(import_dir, metadata, field_name)?;
    if values.is_empty() {
        return Ok(None);
    }
    if values.len() > 1 {
        return Ok(None);
    }
    Ok(values.into_iter().next())
}

fn load_dashboard_org_scope_by_file(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<std::collections::BTreeMap<String, (String, String)>> {
    let mut scope_by_file = std::collections::BTreeMap::new();
    for entry in load_variant_index_entries(import_dir, metadata)? {
        scope_by_file.insert(
            normalize_index_entry_path(&entry.path),
            (entry.org, entry.org_id),
        );
    }
    Ok(scope_by_file)
}

fn load_inspect_source_root(import_dir: &Path) -> Option<PathBuf> {
    let source_root_path = import_dir.join(INSPECT_SOURCE_ROOT_FILENAME);
    let raw = fs::read_to_string(source_root_path).ok()?;
    let text = raw.trim();
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    }
}

fn resolve_dashboard_source_file_path(
    import_dir: &Path,
    dashboard_file: &Path,
    source_root: Option<&Path>,
) -> String {
    let Some(source_root) = source_root else {
        return dashboard_file.display().to_string();
    };
    let Ok(relative_path) = dashboard_file.strip_prefix(import_dir) else {
        return dashboard_file.display().to_string();
    };
    let mut parts = relative_path.components();
    let Some(first) = parts.next() else {
        return dashboard_file.display().to_string();
    };
    let first = first.as_os_str();
    if first.to_string_lossy().starts_with("org_") {
        return source_root
            .join(first)
            .join(RAW_EXPORT_SUBDIR)
            .join(parts.as_path())
            .display()
            .to_string();
    }
    source_root.join(relative_path).display().to_string()
}

fn normalize_merged_dashboard_folder_path(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let mut parts = normalized.split('/').collect::<Vec<&str>>();
    if parts.len() >= 2 && parts[0].starts_with("org_") {
        parts.drain(0..1);
        return parts.join("/");
    }
    normalized
        .rsplit_once("/raw/")
        .map(|(_, remainder)| remainder.to_string())
        .unwrap_or(normalized)
}

fn resolve_export_folder_inventory_item(
    document: &Map<String, Value>,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
) -> Option<FolderInventoryItem> {
    let folder_uid = object_field(document, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if !folder_uid.is_empty() {
        if let Some(folder) = folders_by_uid.get(&folder_uid) {
            return Some(folder.clone());
        }
        if folder_uid == DEFAULT_FOLDER_UID {
            return Some(FolderInventoryItem {
                uid: DEFAULT_FOLDER_UID.to_string(),
                title: DEFAULT_FOLDER_TITLE.to_string(),
                path: DEFAULT_FOLDER_TITLE.to_string(),
                parent_uid: None,
                org: String::new(),
                org_id: String::new(),
            });
        }
    }
    let relative_parent = dashboard_file
        .strip_prefix(import_dir)
        .ok()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| Path::new(""));
    let folder_name = normalize_merged_dashboard_folder_path(relative_parent);
    if !folder_name.is_empty() && folder_name != "." && folder_name != DEFAULT_FOLDER_TITLE {
        let matches = folders_by_uid
            .values()
            .filter(|item| item.title == folder_name)
            .collect::<Vec<&FolderInventoryItem>>();
        if matches.len() == 1 {
            return Some((*matches[0]).clone());
        }
    }
    if folder_name.is_empty() || folder_name == "." || folder_name == DEFAULT_FOLDER_TITLE {
        return Some(FolderInventoryItem {
            uid: DEFAULT_FOLDER_UID.to_string(),
            title: DEFAULT_FOLDER_TITLE.to_string(),
            path: DEFAULT_FOLDER_TITLE.to_string(),
            parent_uid: None,
            org: String::new(),
            org_id: String::new(),
        });
    }
    None
}

fn resolve_export_folder_path(
    document: &Map<String, Value>,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
) -> String {
    if let Some(folder) =
        resolve_export_folder_inventory_item(document, dashboard_file, import_dir, folders_by_uid)
    {
        if !folder.path.trim().is_empty() {
            return folder.path;
        }
    }
    let relative_parent = dashboard_file
        .strip_prefix(import_dir)
        .ok()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| Path::new(""));
    let folder_name = normalize_merged_dashboard_folder_path(relative_parent);
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
            .map(|value| {
                row.datasource == value
                    || row.datasource_uid == value
                    || row.datasource_type == value
                    || row.datasource_family == value
            })
            .unwrap_or(true);
        let panel_match = panel_id_filter
            .map(|value| row.panel_id == value)
            .unwrap_or(true);
        datasource_match && panel_match
    });
    refresh_filtered_query_report_summary(&mut report);
    report
}

pub(crate) fn validate_inspect_export_report_args(args: &InspectExportArgs) -> Result<()> {
    let report_format = effective_inspect_report_format(args);
    if report_format.is_none() {
        if !args.report_columns.is_empty() {
            return Err(message(
                "--report-columns is only supported together with --report or report-like --output-format.",
            ));
        }
        if args.report_filter_datasource.is_some() {
            return Err(message(
                "--report-filter-datasource is only supported together with --report or report-like --output-format.",
            ));
        }
        if args.report_filter_panel_id.is_some() {
            return Err(message(
                "--report-filter-panel-id is only supported together with --report or report-like --output-format.",
            ));
        }
        return Ok(());
    }
    if report_format
        .map(|format| {
            matches!(
                format,
                InspectExportReportFormat::Governance | InspectExportReportFormat::GovernanceJson
            )
        })
        .unwrap_or(false)
        && !args.report_columns.is_empty()
    {
        return Err(message(
            "--report-columns is not supported with governance output.",
        ));
    }
    if report_format
        .map(|format| !report_format_supports_columns(format))
        .unwrap_or(false)
        && !args.report_columns.is_empty()
    {
        return Err(message(
            "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes.",
        ));
    }
    let _ = resolve_report_column_ids_for_format(report_format, &args.report_columns)?;
    Ok(())
}

fn map_output_format_to_report(
    output_format: InspectOutputFormat,
) -> Option<InspectExportReportFormat> {
    match output_format {
        InspectOutputFormat::Text | InspectOutputFormat::Table | InspectOutputFormat::Json => None,
        InspectOutputFormat::ReportTable => Some(InspectExportReportFormat::Table),
        InspectOutputFormat::ReportCsv => Some(InspectExportReportFormat::Csv),
        InspectOutputFormat::ReportJson => Some(InspectExportReportFormat::Json),
        InspectOutputFormat::ReportTree => Some(InspectExportReportFormat::Tree),
        InspectOutputFormat::ReportTreeTable => Some(InspectExportReportFormat::TreeTable),
        InspectOutputFormat::ReportDependency => Some(InspectExportReportFormat::Dependency),
        InspectOutputFormat::ReportDependencyJson => {
            Some(InspectExportReportFormat::DependencyJson)
        }
        InspectOutputFormat::Governance => Some(InspectExportReportFormat::Governance),
        InspectOutputFormat::GovernanceJson => Some(InspectExportReportFormat::GovernanceJson),
    }
}

fn effective_inspect_report_format(args: &InspectExportArgs) -> Option<InspectExportReportFormat> {
    args.report
        .or_else(|| args.output_format.and_then(map_output_format_to_report))
}

fn effective_inspect_json(args: &InspectExportArgs) -> bool {
    args.json || matches!(args.output_format, Some(InspectOutputFormat::Json))
}

fn effective_inspect_table(args: &InspectExportArgs) -> bool {
    args.table || matches!(args.output_format, Some(InspectOutputFormat::Table))
}

pub(crate) fn build_export_inspection_summary(
    import_dir: &Path,
) -> Result<ExportInspectionSummary> {
    // Summary aggregation must stay file-system driven and deterministic so export and
    // live inspection can converge on the same coverage counts after staging.
    let metadata = load_export_metadata(import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let source_root = load_inspect_source_root(import_dir);
    let export_org = resolve_export_identity_field(import_dir, metadata.as_ref(), "org")?;
    let export_org_id = resolve_export_identity_field(import_dir, metadata.as_ref(), "orgId")?;
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
    let orphaned_datasource_summary = datasource_inventory_summary
        .iter()
        .filter(|item| item.reference_count == 0 && item.dashboard_count == 0)
        .cloned()
        .collect::<Vec<DatasourceInventorySummary>>();
    mixed_dashboards.sort_by(|left, right| {
        left.folder_path
            .cmp(&right.folder_path)
            .then(left.title.cmp(&right.title))
            .then(left.uid.cmp(&right.uid))
    });

    Ok(ExportInspectionSummary {
        import_dir: source_root
            .as_ref()
            .map_or(import_dir, |value| value.as_path())
            .display()
            .to_string(),
        export_org,
        export_org_id,
        dashboard_count: dashboard_files.len(),
        folder_count: folder_paths.len(),
        panel_count: total_panels,
        query_count: total_queries,
        datasource_inventory_count: datasource_inventory_summary.len(),
        orphaned_datasource_count: orphaned_datasource_summary.len(),
        mixed_dashboard_count: mixed_dashboards.len(),
        folder_paths,
        datasource_usage,
        datasource_inventory: datasource_inventory_summary,
        orphaned_datasources: orphaned_datasource_summary,
        mixed_dashboards,
    })
}

fn render_export_inspection_summary_output(
    args: &InspectExportArgs,
    summary: &ExportInspectionSummary,
) -> Result<String> {
    if effective_inspect_json(args) {
        let document = build_export_inspection_summary_document(summary);
        Ok(format!("{}\n", serde_json::to_string_pretty(&document)?))
    } else {
        let mut output = String::new();
        output.push_str(&format!("Export inspection: {}\n", summary.import_dir));
        if effective_inspect_table(args) {
            output.push('\n');
            output.push_str("# Summary\n");
            let summary_rows = build_export_inspection_summary_rows(summary);
            for line in render_simple_table(&["METRIC", "VALUE"], &summary_rows, !args.no_header) {
                output.push_str(&line);
                output.push('\n');
            }
        } else {
            if let Some(export_org) = &summary.export_org {
                output.push_str(&format!("Export org: {}\n", export_org));
            }
            if let Some(export_org_id) = &summary.export_org_id {
                output.push_str(&format!("Export orgId: {}\n", export_org_id));
            }
            output.push_str(&format!("Dashboards: {}\n", summary.dashboard_count));
            output.push_str(&format!("Folders: {}\n", summary.folder_count));
            output.push_str(&format!("Panels: {}\n", summary.panel_count));
            output.push_str(&format!("Queries: {}\n", summary.query_count));
            output.push_str(&format!(
                "Datasource inventory: {}\n",
                summary.datasource_inventory_count
            ));
            output.push_str(&format!(
                "Orphaned datasources: {}\n",
                summary.orphaned_datasource_count
            ));
            output.push_str(&format!(
                "Mixed datasource dashboards: {}\n",
                summary.mixed_dashboard_count
            ));
        }

        output.push('\n');
        output.push_str("# Folder paths\n");
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
            output.push_str(&line);
            output.push('\n');
        }

        output.push('\n');
        output.push_str("# Datasource usage\n");
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
            output.push_str(&line);
            output.push('\n');
        }

        if !summary.datasource_inventory.is_empty() {
            output.push('\n');
            output.push_str("# Datasource inventory\n");
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
                output.push_str(&line);
                output.push('\n');
            }
        }

        if !summary.orphaned_datasources.is_empty() {
            output.push('\n');
            output.push_str("# Orphaned datasources\n");
            let orphaned_rows = summary
                .orphaned_datasources
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
                ],
                &orphaned_rows,
                !args.no_header,
            ) {
                output.push_str(&line);
                output.push('\n');
            }
        }

        if !summary.mixed_dashboards.is_empty() {
            output.push('\n');
            output.push_str("# Mixed datasource dashboards\n");
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
                output.push_str(&line);
                output.push('\n');
            }
        }
        Ok(output)
    }
}

fn analyze_export_dir_at_path(args: &InspectExportArgs, import_dir: &Path) -> Result<usize> {
    if args.interactive {
        return run_interactive_export_workbench(import_dir);
    }
    let write_output =
        |output: &str| -> Result<()> { write_inspect_output(output, args.output_file.as_ref()) };

    if let Some(report_format) = effective_inspect_report_format(args) {
        let report = apply_query_report_filters(
            build_export_inspection_query_report(import_dir)?,
            args.report_filter_datasource.as_deref(),
            args.report_filter_panel_id.as_deref(),
        );
        let rendered =
            render_export_inspection_report_output(args, import_dir, report_format, &report)?;
        write_output(&rendered.output)?;
        return Ok(rendered.dashboard_count);
    }

    let summary = build_export_inspection_summary(import_dir)?;
    let output = render_export_inspection_summary_output(args, &summary)?;
    write_output(&output)?;
    Ok(summary.dashboard_count)
}

#[cfg(feature = "tui")]
fn run_interactive_export_workbench(import_dir: &Path) -> Result<usize> {
    let summary = build_export_inspection_summary(import_dir)?;
    let report = build_export_inspection_query_report(import_dir)?;
    let governance = build_export_inspection_governance_document(&summary, &report);
    let document =
        build_inspect_workbench_document("export artifacts", &summary, &governance, &report);
    run_inspect_workbench(document)?;
    Ok(summary.dashboard_count)
}

#[cfg(not(feature = "tui"))]
fn run_interactive_export_workbench(_import_dir: &Path) -> Result<usize> {
    super::tui_not_built("inspect-export --interactive")
}

pub(crate) fn analyze_export_dir(args: &InspectExportArgs) -> Result<usize> {
    // Keep validation and root expansion here so all report/render branches start from
    // the same normalized import tree.
    validate_inspect_export_report_args(args)?;
    let temp_dir = TempInspectDir::new("inspect-export")?;
    let import_dir = prepare_inspect_export_import_dir(&temp_dir.path, &args.import_dir)?;
    analyze_export_dir_at_path(args, &import_dir)
}
