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
#[path = "inspect_orchestration.rs"]
mod inspect_orchestration;
#[path = "inspect_output.rs"]
mod inspect_output;
#[path = "inspect_paths.rs"]
mod inspect_paths;
#[path = "inspect_query_report.rs"]
mod inspect_query_report;

use serde_json::{Map, Value};
use std::path::Path;

use super::files::{
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file,
};
#[cfg(test)]
pub(crate) use super::inspect_live::prepare_inspect_export_import_dir;
pub(crate) use super::inspect_live::TempInspectDir;
#[cfg(test)]
pub(crate) use super::inspect_query::QueryAnalysis;
#[allow(unused_imports)]
pub(crate) use super::inspect_query::{
    dispatch_query_analysis, ordered_unique_push,
    resolve_query_analyzer_family_from_datasource_type,
    resolve_query_analyzer_family_from_query_signature, QueryExtractionContext,
    DATASOURCE_FAMILY_UNKNOWN,
};
use super::inspect_summary::{
    DatasourceInventorySummary, ExportDatasourceUsage, ExportFolderUsage, ExportInspectionSummary,
    MixedDashboardSummary,
};
use super::models::{DatasourceInventoryItem, FolderInventoryItem};
use super::prompt::collect_datasource_refs;
use super::{
    DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_TITLE, DEFAULT_UNKNOWN_UID, RAW_EXPORT_SUBDIR,
};
use crate::common::{string_field, value_as_object, Result};
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
pub(crate) use inspect_orchestration::analyze_export_dir;
#[allow(unused_imports)]
pub(crate) use inspect_orchestration::{
    apply_query_report_filters, effective_inspect_report_format, resolve_inspect_export_import_dir,
    validate_inspect_export_report_args,
};
pub(crate) use inspect_output::{
    render_export_inspection_report_output, render_export_inspection_summary_output,
};
pub(crate) use inspect_paths::{
    load_dashboard_org_scope_by_file, load_inspect_source_root, resolve_dashboard_source_file_path,
    resolve_export_folder_inventory_item, resolve_export_folder_path,
    resolve_export_identity_field, write_inspect_output,
};
pub(crate) use inspect_query_report::build_export_inspection_query_report;
pub(crate) use inspect_query_report::build_export_inspection_query_report_for_variant;

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

fn build_export_inspection_summary_with_variant(
    import_dir: &Path,
    expected_variant: Option<&str>,
) -> Result<ExportInspectionSummary> {
    // Summary aggregation must stay file-system driven and deterministic so export and
    // live inspection can converge on the same coverage counts after staging.
    let metadata = load_export_metadata(import_dir, expected_variant)?;
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

pub(crate) fn build_export_inspection_summary(
    import_dir: &Path,
) -> Result<ExportInspectionSummary> {
    build_export_inspection_summary_with_variant(import_dir, Some(RAW_EXPORT_SUBDIR))
}

pub(crate) fn build_export_inspection_summary_for_variant(
    import_dir: &Path,
    expected_variant: &str,
) -> Result<ExportInspectionSummary> {
    build_export_inspection_summary_with_variant(import_dir, Some(expected_variant))
}
