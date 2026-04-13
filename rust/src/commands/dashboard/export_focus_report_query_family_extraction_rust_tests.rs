//! Dashboard domain test suite.
//! Covers parser surfaces, formatter/output contracts, and export/import/inspect/list/diff
//! behavior with in-memory/mocked request fixtures.
#![allow(unused_imports)]

use super::test_support;
use super::test_support::{
    attach_dashboard_folder_paths_with_request, build_export_metadata, build_export_variant_dirs,
    build_external_export_document, build_folder_inventory_status, build_folder_path,
    build_governance_gate_tui_groups, build_governance_gate_tui_items, build_impact_browser_items,
    build_impact_document, build_impact_tui_groups, build_import_auth_context,
    build_import_payload, build_output_path, build_preserved_web_import_document,
    build_root_export_index, build_topology_document, build_topology_tui_groups,
    diff_dashboards_with_request, discover_dashboard_files, export_dashboards_with_request,
    extract_dashboard_variables, filter_impact_tui_items, filter_topology_tui_items,
    format_dashboard_summary_line, format_export_progress_line, format_export_verbose_line,
    format_folder_inventory_status_line, format_import_progress_line, format_import_verbose_line,
    import_dashboards_with_org_clients, import_dashboards_with_request,
    list_dashboards_with_request, parse_cli_from, render_dashboard_governance_gate_result,
    render_dashboard_summary_csv, render_dashboard_summary_json, render_dashboard_summary_table,
    render_impact_text, render_import_dry_run_json, render_import_dry_run_table,
    render_topology_dot, render_topology_mermaid, CommonCliArgs, DashboardCliArgs,
    DashboardCommand, DashboardGovernanceGateFinding, DashboardGovernanceGateResult,
    DashboardGovernanceGateSummary, DiffArgs, ExportArgs, FolderInventoryStatusKind,
    GovernanceGateArgs, GovernanceGateOutputFormat, ImpactAlertResource, ImpactDashboard,
    ImpactDocument, ImpactOutputFormat, ImpactSummary, ImportArgs, InspectExportArgs,
    InspectExportReportFormat, InspectLiveArgs, InspectOutputFormat, ListArgs, SimpleOutputFormat,
    TopologyDocument, TopologyOutputFormat, ValidationOutputFormat,
    DASHBOARD_PERMISSION_BUNDLE_FILENAME, DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME,
    FOLDER_INVENTORY_FILENAME, TOOL_SCHEMA_VERSION,
};
use super::{
    assert_all_orgs_export_live_documents_match, assert_governance_documents_match,
    export_query_row, load_inspection_analyzer_cases, load_prompt_export_cases,
    make_basic_common_args, make_common_args, make_import_args, sample_topology_tui_document,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use crate::common::api_response;
use crate::dashboard::inspect::{
    dispatch_query_analysis, extract_query_field_and_text, resolve_query_analyzer_family,
    QueryAnalysis, QueryExtractionContext,
};
use crate::dashboard::inspect_governance::governance_risk_spec;
use clap::{CommandFactory, Parser};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn build_export_inspection_query_report_extracts_loki_query_details() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("Logs")).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("Logs").join("loki.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "logs-main",
                "title": "Logs Main",
                "panels": [
                    {
                        "id": 11,
                        "title": "Errors",
                        "type": "logs",
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"uid": "loki-main", "type": "loki"},
                                "expr": "sum by (level) (count_over_time({job=\"grafana\",level=~\"error|warn\"} |= \"timeout\" |~ \"panic|fatal\" | json | line_format \"{{.msg}}\" [5m]))"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();

    assert_eq!(report.queries.len(), 1);
    assert_eq!(report.queries[0].metrics, Vec::<String>::new());
    assert_eq!(
        report.queries[0].functions,
        vec![
            "sum".to_string(),
            "count_over_time".to_string(),
            "json".to_string(),
            "line_format".to_string(),
            "line_filter_contains".to_string(),
            "line_filter_contains:timeout".to_string(),
            "line_filter_regex".to_string(),
            "line_filter_regex:panic|fatal".to_string(),
        ]
    );
    assert_eq!(
        report.queries[0].measurements,
        vec![
            "{job=\"grafana\",level=~\"error|warn\"}".to_string(),
            "job=\"grafana\"".to_string(),
            "level=~\"error|warn\"".to_string(),
        ]
    );
    assert_eq!(report.queries[0].buckets, vec!["5m".to_string()]);
}

#[test]
fn build_export_inspection_query_report_ignores_loki_line_format_templates_when_extracting_filters()
{
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "loki-main",
        "Logs Main",
        "loki-main",
        "loki",
        "logs",
        "folder-1",
        "Logs",
        "expr",
        "sum by (namespace) (count_over_time({job=\"grafana\",namespace!~\"kube-system\"} | line_format \"{{.msg}} |= {{.status}} |~ {{.level}}\" |= \"timeout\" |~ \"panic|fatal\" [10m]))",
    );

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();

    assert_eq!(report.queries.len(), 1);
    assert_eq!(
        report.queries[0].functions,
        vec![
            "sum".to_string(),
            "count_over_time".to_string(),
            "line_format".to_string(),
            "line_filter_contains".to_string(),
            "line_filter_contains:timeout".to_string(),
            "line_filter_regex".to_string(),
            "line_filter_regex:panic|fatal".to_string(),
        ]
    );
    assert_eq!(
        report.queries[0].measurements,
        vec![
            "{job=\"grafana\",namespace!~\"kube-system\"}".to_string(),
            "job=\"grafana\"".to_string(),
            "namespace!~\"kube-system\"".to_string(),
        ]
    );
    assert_eq!(report.queries[0].buckets, vec!["10m".to_string()]);
}

#[test]
fn build_export_inspection_query_report_extracts_negative_loki_line_filters() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "loki-main",
        "Logs Main",
        "loki-main",
        "loki",
        "logs",
        "folder-1",
        "Logs",
        "expr",
        "sum by (namespace) (count_over_time({job=\"grafana\",namespace!=\"kube-system\"} != \"debug\" !~ \"health|metrics\" | json [15m]))",
    );

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();

    assert_eq!(report.queries.len(), 1);
    assert_eq!(
        report.queries[0].functions,
        vec![
            "sum".to_string(),
            "count_over_time".to_string(),
            "json".to_string(),
            "line_filter_not_contains".to_string(),
            "line_filter_not_contains:debug".to_string(),
            "line_filter_not_regex".to_string(),
            "line_filter_not_regex:health|metrics".to_string(),
        ]
    );
    assert_eq!(
        report.queries[0].measurements,
        vec![
            "{job=\"grafana\",namespace!=\"kube-system\"}".to_string(),
            "job=\"grafana\"".to_string(),
            "namespace!=\"kube-system\"".to_string(),
        ]
    );
    assert_eq!(report.queries[0].buckets, vec!["15m".to_string()]);
}

#[test]
fn build_export_inspection_query_report_extracts_loki_pipeline_field_hints() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "loki-main",
        "Logs Main",
        "loki-main",
        "loki",
        "logs",
        "folder-1",
        "Logs",
        "expr",
        "sum by (level) (count_over_time({job=\"grafana\"} |= \"timeout\" | json status >= 500 | logfmt level = \"error\" | unwrap duration_ms [5m]))",
    );

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();

    assert_eq!(report.queries.len(), 1);
    assert_eq!(
        report.queries[0].functions,
        vec![
            "sum".to_string(),
            "count_over_time".to_string(),
            "json".to_string(),
            "logfmt".to_string(),
            "unwrap".to_string(),
            "line_filter_contains".to_string(),
            "line_filter_contains:timeout".to_string(),
        ]
    );
    assert_eq!(
        report.queries[0].measurements,
        vec![
            "{job=\"grafana\"}".to_string(),
            "job=\"grafana\"".to_string(),
            "status".to_string(),
            "level".to_string(),
            "duration_ms".to_string(),
        ]
    );
    assert_eq!(report.queries[0].buckets, vec!["5m".to_string()]);
}

#[test]
fn build_export_inspection_query_report_ignores_loki_regex_character_classes_when_extracting_buckets(
) {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "loki-main",
        "Logs Main",
        "loki-main",
        "loki",
        "logs",
        "folder-1",
        "Logs",
        "expr",
        "sum(count_over_time({job=\"grafana\"} |~ \"panic[0-9]+\" [5m]))",
    );

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();

    assert_eq!(report.queries.len(), 1);
    assert_eq!(
        report.queries[0].functions,
        vec![
            "sum".to_string(),
            "count_over_time".to_string(),
            "line_filter_regex".to_string(),
            "line_filter_regex:panic[0-9]+".to_string(),
        ]
    );
    assert_eq!(
        report.queries[0].measurements,
        vec![
            "{job=\"grafana\"}".to_string(),
            "job=\"grafana\"".to_string(),
        ]
    );
    assert_eq!(report.queries[0].buckets, vec!["5m".to_string()]);
}

#[test]
fn build_export_inspection_query_report_keeps_prometheus_metrics_and_skips_label_tokens() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("General")).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("General").join("prometheus.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "prom-main",
                "title": "Prom Main",
                "panels": [
                    {
                        "id": 7,
                        "title": "HTTP Requests",
                        "type": "timeseries",
                        "datasource": {"uid": "prom-main", "type": "prometheus"},
                        "targets": [
                            {
                                "refId": "A",
                                "expr": "sum by(instance) (rate(http_requests_total{job=\"api\", instance=~\"web-.+\", __name__=\"http_requests_total\"}[5m])) / ignoring(pod) group_left(namespace) kube_pod_info{namespace=\"prod\", pod=~\"api-.+\"}"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();

    assert_eq!(report.queries.len(), 1);
    assert_eq!(
        report.queries[0].metrics,
        vec![
            "http_requests_total".to_string(),
            "kube_pod_info".to_string(),
        ]
    );
    assert_eq!(report.queries[0].functions, vec!["rate".to_string()]);
}
