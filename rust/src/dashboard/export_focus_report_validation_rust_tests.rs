//! Dashboard export report validation tests.
#![allow(unused_imports)]

use super::test_support;
use super::{InspectExportArgs, InspectExportReportFormat};
use std::path::PathBuf;

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_without_report() {
    let args = InspectExportArgs {
        import_dir: PathBuf::from("./dashboards/raw"),
        json: false,
        table: false,
        report: None,
        output_format: None,
        report_columns: vec!["dashboard_uid".to_string()],
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error.to_string().contains(
        "--report-columns is only supported together with --report or report-like --output-format"
    ));
}

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_for_json_report() {
    let args = InspectExportArgs {
        import_dir: PathBuf::from("./dashboards/raw"),
        json: false,
        table: false,
        report: Some(InspectExportReportFormat::Json),
        output_format: None,
        report_columns: vec!["dashboard_uid".to_string()],
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error.to_string().contains(
        "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes"
    ));
}

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_for_dependency_report() {
    let args = InspectExportArgs {
        import_dir: PathBuf::from("./dashboards/raw"),
        json: false,
        table: false,
        report: Some(InspectExportReportFormat::Dependency),
        output_format: None,
        report_columns: vec!["dashboard_uid".to_string()],
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error.to_string().contains(
        "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes"
    ));
}

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_for_tree_report() {
    let args = InspectExportArgs {
        import_dir: PathBuf::from("./dashboards/raw"),
        json: false,
        table: false,
        report: Some(InspectExportReportFormat::Tree),
        output_format: None,
        report_columns: vec!["dashboard_uid".to_string()],
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error.to_string().contains(
        "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes"
    ));
}

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_for_governance_report() {
    let args = InspectExportArgs {
        import_dir: PathBuf::from("./dashboards/raw"),
        json: false,
        table: false,
        report: Some(InspectExportReportFormat::Governance),
        output_format: None,
        report_columns: vec!["dashboard_uid".to_string()],
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error
        .to_string()
        .contains("--report-columns is not supported with governance output"));
}

#[test]
fn validate_inspect_export_report_args_allows_report_columns_for_tree_table_report() {
    let args = InspectExportArgs {
        import_dir: PathBuf::from("./dashboards/raw"),
        json: false,
        table: false,
        report: Some(InspectExportReportFormat::TreeTable),
        output_format: None,
        report_columns: vec!["panel_id".to_string(), "query".to_string()],
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        interactive: false,
    };

    test_support::validate_inspect_export_report_args(&args).unwrap();
}

#[test]
fn validate_inspect_export_report_args_rejects_panel_filter_without_report() {
    let args = InspectExportArgs {
        import_dir: PathBuf::from("./dashboards/raw"),
        json: false,
        table: false,
        report: None,
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: Some("7".to_string()),
        help_full: false,
        no_header: false,
        output_file: None,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error
        .to_string()
        .contains("--report-filter-panel-id is only supported together with --report or report-like --output-format"));
}
