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

#[cfg(test)]
#[path = "export_focus_report_query_family_rust_tests.rs"]
mod export_focus_report_query_family_rust_tests;

#[cfg(test)]
#[path = "export_focus_report_query_presentation_rust_tests.rs"]
mod export_focus_report_query_presentation_rust_tests;

#[allow(clippy::too_many_arguments)]
pub(crate) fn make_core_family_report_row(
    dashboard_uid: &str,
    panel_id: &str,
    ref_id: &str,
    datasource_uid: &str,
    datasource_name: &str,
    datasource_type: &str,
    datasource_family: &str,
    query_text: &str,
    measurements: &[&str],
) -> test_support::ExportInspectionQueryRow {
    test_support::ExportInspectionQueryRow {
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
        dashboard_uid: dashboard_uid.to_string(),
        dashboard_title: format!("{dashboard_uid} Dashboard"),
        dashboard_tags: Vec::new(),
        folder_path: "General".to_string(),
        folder_full_path: "/".to_string(),
        folder_level: "1".to_string(),
        folder_uid: "general".to_string(),
        parent_folder_uid: String::new(),
        panel_id: panel_id.to_string(),
        panel_title: "Query".to_string(),
        panel_type: "table".to_string(),
        panel_target_count: 1,
        panel_query_count: 1,
        panel_datasource_count: 0,
        panel_variables: Vec::new(),
        ref_id: ref_id.to_string(),
        datasource: datasource_name.to_string(),
        datasource_name: datasource_name.to_string(),
        datasource_uid: datasource_uid.to_string(),
        datasource_org: String::new(),
        datasource_org_id: String::new(),
        datasource_database: String::new(),
        datasource_bucket: String::new(),
        datasource_organization: String::new(),
        datasource_index_pattern: String::new(),
        datasource_type: datasource_type.to_string(),
        datasource_family: datasource_family.to_string(),
        query_field: "query".to_string(),
        target_hidden: "false".to_string(),
        target_disabled: "false".to_string(),
        query_text: query_text.to_string(),
        query_variables: Vec::new(),
        metrics: Vec::new(),
        functions: Vec::new(),
        measurements: measurements.iter().map(|value| value.to_string()).collect(),
        buckets: Vec::new(),
        file_path: format!("/tmp/raw/{dashboard_uid}.json"),
    }
}

#[test]
fn apply_query_report_filters_matches_core_family_aliases() {
    let make_row = |dashboard_uid: &str,
                    panel_id: &str,
                    ref_id: &str,
                    datasource_uid: &str,
                    datasource_name: &str,
                    datasource_type: &str,
                    datasource_family: &str| {
        make_core_family_report_row(
            dashboard_uid,
            panel_id,
            ref_id,
            datasource_uid,
            datasource_name,
            datasource_type,
            datasource_family,
            "placeholder",
            &[],
        )
    };
    let report = test_support::ExportInspectionQueryReport {
        input_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 6,
            panel_count: 6,
            query_count: 6,
            report_row_count: 6,
        },
        queries: vec![
            make_row(
                "prom-main",
                "1",
                "A",
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "prometheus",
            ),
            make_row(
                "logs-main",
                "2",
                "A",
                "logs-main",
                "Logs Main",
                "loki",
                "loki",
            ),
            make_row(
                "flux-main",
                "3",
                "A",
                "flux-main",
                "Influx Main",
                "influxdb",
                "flux",
            ),
            make_row(
                "sql-main",
                "4",
                "A",
                "sql-main",
                "Postgres Main",
                "postgres",
                "postgres",
            ),
            make_row(
                "search-main",
                "5",
                "A",
                "search-main",
                "Elastic Main",
                "elasticsearch",
                "search",
            ),
            make_row(
                "trace-main",
                "6",
                "A",
                "trace-main",
                "Tempo Main",
                "tempo",
                "tracing",
            ),
        ],
    };
    let cases = [
        ("prometheus", "prom-main"),
        ("loki", "logs-main"),
        ("flux", "flux-main"),
        ("postgres", "sql-main"),
        ("search", "search-main"),
        ("tracing", "trace-main"),
    ];

    for (filter_value, expected_dashboard_uid) in cases {
        let filtered =
            test_support::apply_query_report_filters(report.clone(), Some(filter_value), None);
        assert_eq!(filtered.queries.len(), 1);
        assert_eq!(filtered.queries[0].dashboard_uid, expected_dashboard_uid);
    }

    let rendered = test_support::render_grouped_query_report(&report).join("\n");
    assert!(rendered.contains("datasourceFamily=search"));
    assert!(rendered.contains("datasourceFamily=tracing"));
}

#[test]
fn dispatch_query_analysis_matches_shared_analyzer_fixture_cases() {
    for case in load_inspection_analyzer_cases() {
        let case_name = case["name"].as_str().unwrap();
        let expected_family = case["expectedFamily"].as_str().unwrap();
        let expected_analysis = &case["expectedAnalysis"];
        let panel = case["panel"].as_object().unwrap().clone();
        let target = case["target"].as_object().unwrap().clone();
        let query_field = case["queryField"].as_str().unwrap();
        let query_text = case["queryText"].as_str().unwrap();
        let context = QueryExtractionContext {
            panel: &panel,
            target: &target,
            query_field,
            query_text,
            resolved_datasource_type: "",
        };

        assert_eq!(
            resolve_query_analyzer_family(&context),
            expected_family,
            "case={case_name}"
        );
        assert_eq!(
            dispatch_query_analysis(&context),
            QueryAnalysis {
                metrics: expected_analysis["metrics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<String>>(),
                functions: expected_analysis["functions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<String>>(),
                measurements: expected_analysis["measurements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<String>>(),
                buckets: expected_analysis["buckets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<String>>(),
            },
            "case={case_name}"
        );
    }
}

#[test]
fn resolve_report_column_ids_include_file_by_default_and_allow_datasource_uid() {
    let default_columns = test_support::resolve_report_column_ids(&[]).unwrap();
    assert!(default_columns.iter().any(|value| value == "file"));
    assert!(!default_columns
        .iter()
        .any(|value| value == "datasource_uid"));
    assert!(default_columns
        .iter()
        .any(|value| value == "datasource_type"));
    assert!(default_columns
        .iter()
        .any(|value| value == "datasource_family"));
    assert!(default_columns
        .iter()
        .any(|value| value == "dashboard_tags"));
    assert!(default_columns
        .iter()
        .any(|value| value == "panel_query_count"));
    assert!(default_columns
        .iter()
        .any(|value| value == "panel_datasource_count"));
    assert!(default_columns
        .iter()
        .any(|value| value == "panel_variables"));
    assert!(default_columns
        .iter()
        .any(|value| value == "query_variables"));

    let selected = test_support::resolve_report_column_ids(&[
        "dashboard_uid".to_string(),
        "datasource_uid".to_string(),
        "datasource_type".to_string(),
        "datasource_family".to_string(),
        "file".to_string(),
        "query".to_string(),
    ])
    .unwrap();
    assert_eq!(
        selected,
        vec![
            "dashboard_uid".to_string(),
            "datasource_uid".to_string(),
            "datasource_type".to_string(),
            "datasource_family".to_string(),
            "file".to_string(),
            "query".to_string(),
        ]
    );
}

#[test]
fn resolve_report_column_ids_for_format_defaults_csv_to_supported_columns() {
    let csv_columns = test_support::resolve_report_column_ids_for_format(
        Some(InspectExportReportFormat::Csv),
        &[],
    )
    .unwrap();
    assert!(csv_columns.iter().any(|value| value == "datasource_uid"));
    assert!(csv_columns
        .iter()
        .any(|value| value == "panel_target_count"));
    assert!(csv_columns.iter().any(|value| value == "target_hidden"));
    assert!(csv_columns.iter().any(|value| value == "target_disabled"));
    assert_eq!(
        csv_columns.len(),
        test_support::SUPPORTED_REPORT_COLUMN_IDS.len()
    );

    let table_columns = test_support::resolve_report_column_ids_for_format(
        Some(InspectExportReportFormat::Table),
        &[],
    )
    .unwrap();
    assert_eq!(
        table_columns,
        test_support::DEFAULT_REPORT_COLUMN_IDS
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>()
    );
}

#[test]
fn resolve_report_column_ids_accepts_json_style_aliases() {
    let selected = test_support::resolve_report_column_ids(&[
        "dashboardUid".to_string(),
        "dashboardTags".to_string(),
        "datasourceUid".to_string(),
        "datasourceType".to_string(),
        "datasourceFamily".to_string(),
        "panelQueryCount".to_string(),
        "panelDatasourceCount".to_string(),
        "panelVariables".to_string(),
        "queryField".to_string(),
        "queryVariables".to_string(),
        "file".to_string(),
    ])
    .unwrap();
    assert_eq!(
        selected,
        vec![
            "dashboard_uid".to_string(),
            "dashboard_tags".to_string(),
            "datasource_uid".to_string(),
            "datasource_type".to_string(),
            "datasource_family".to_string(),
            "panel_query_count".to_string(),
            "panel_datasource_count".to_string(),
            "panel_variables".to_string(),
            "query_field".to_string(),
            "query_variables".to_string(),
            "file".to_string(),
        ]
    );
}

#[test]
fn export_inspection_query_row_json_keeps_datasource_uid_and_file_fields() {
    let row = test_support::ExportInspectionQueryRow {
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
        dashboard_uid: "main".to_string(),
        dashboard_title: "Main".to_string(),
        dashboard_tags: Vec::new(),
        folder_path: "General".to_string(),
        folder_full_path: "/".to_string(),
        folder_level: "1".to_string(),
        folder_uid: "general".to_string(),
        parent_folder_uid: String::new(),
        panel_id: "1".to_string(),
        panel_title: "CPU".to_string(),
        panel_type: "timeseries".to_string(),
        panel_target_count: 0,
        panel_query_count: 0,
        panel_datasource_count: 0,
        panel_variables: Vec::new(),
        ref_id: "A".to_string(),
        datasource: "prom-main".to_string(),
        datasource_name: "prom-main".to_string(),
        datasource_uid: String::new(),
        datasource_org: String::new(),
        datasource_org_id: String::new(),
        datasource_database: String::new(),
        datasource_bucket: String::new(),
        datasource_organization: String::new(),
        datasource_index_pattern: String::new(),
        datasource_type: "prometheus".to_string(),
        datasource_family: "prometheus".to_string(),
        query_field: "expr".to_string(),
        target_hidden: "false".to_string(),
        target_disabled: "false".to_string(),
        query_text: "up".to_string(),
        query_variables: Vec::new(),
        metrics: vec!["up".to_string()],
        functions: Vec::new(),
        measurements: Vec::new(),
        buckets: Vec::new(),
        file_path: "/tmp/raw/main.json".to_string(),
    };

    let value = serde_json::to_value(&row).unwrap();

    assert_eq!(value["org"], Value::String("Main Org.".to_string()));
    assert_eq!(value["orgId"], Value::String("1".to_string()));
    assert_eq!(value["folderFullPath"], Value::String("/".to_string()));
    assert_eq!(value["folderLevel"], Value::String("1".to_string()));
    assert_eq!(value["datasourceUid"], Value::String(String::new()));
    assert_eq!(
        value["datasourceType"],
        Value::String("prometheus".to_string())
    );
    assert_eq!(
        value["datasourceFamily"],
        Value::String("prometheus".to_string())
    );
    assert_eq!(
        value["file"],
        Value::String("/tmp/raw/main.json".to_string())
    );
}

#[test]
fn resolve_report_column_ids_rejects_unknown_columns() {
    let error = test_support::resolve_report_column_ids(&["unknown".to_string()]).unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported --report-columns value"));
}

#[test]
fn resolve_report_column_ids_supports_all() {
    let columns = test_support::resolve_report_column_ids(&["all".to_string()]).unwrap();
    assert!(columns.contains(&"folder_full_path".to_string()));
    assert!(columns.contains(&"folder_level".to_string()));
    assert!(columns.contains(&"datasource_uid".to_string()));
    assert!(columns.contains(&"dashboard_tags".to_string()));
    assert!(columns.contains(&"panel_query_count".to_string()));
    assert!(columns.contains(&"panel_datasource_count".to_string()));
    assert!(columns.contains(&"panel_variables".to_string()));
    assert!(columns.contains(&"query_variables".to_string()));
    assert!(columns.contains(&"file".to_string()));
}

#[test]
fn report_format_supports_columns_matches_inspection_contract() {
    assert!(test_support::report_format_supports_columns(
        InspectExportReportFormat::Table
    ));
    assert!(test_support::report_format_supports_columns(
        InspectExportReportFormat::Csv
    ));
    assert!(test_support::report_format_supports_columns(
        InspectExportReportFormat::TreeTable
    ));
    assert!(!test_support::report_format_supports_columns(
        InspectExportReportFormat::QueriesJson
    ));
    assert!(!test_support::report_format_supports_columns(
        InspectExportReportFormat::Tree
    ));
}
