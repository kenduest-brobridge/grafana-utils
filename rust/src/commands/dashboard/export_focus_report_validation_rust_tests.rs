//! Dashboard export report validation tests.
#![allow(unused_imports)]

use super::test_support;
use super::{InspectExportArgs, InspectOutputFormat};
use std::path::PathBuf;

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_without_report() {
    let args = InspectExportArgs {
        input_dir: PathBuf::from("./dashboards/raw"),
        input_type: None,
        input_format: test_support::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: None,
        report_columns: vec!["dashboard_uid".to_string()],
        list_columns: false,
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error.to_string().contains(
        "--report-columns is only supported together with table, csv, tree-table, or queries-json output."
    ));
}

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_for_json_report() {
    let args = InspectExportArgs {
        input_dir: PathBuf::from("./dashboards/raw"),
        input_type: None,
        input_format: test_support::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::QueriesJson),
        report_columns: vec!["dashboard_uid".to_string()],
        list_columns: false,
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error
        .to_string()
        .contains("--report-columns is only supported with table, csv, or tree-table output."));
}

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_for_dependency_report() {
    let args = InspectExportArgs {
        input_dir: PathBuf::from("./dashboards/raw"),
        input_type: None,
        input_format: test_support::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::Dependency),
        report_columns: vec!["dashboard_uid".to_string()],
        list_columns: false,
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error
        .to_string()
        .contains("--report-columns is only supported with table, csv, or tree-table output."));
}

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_for_tree_report() {
    let args = InspectExportArgs {
        input_dir: PathBuf::from("./dashboards/raw"),
        input_type: None,
        input_format: test_support::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::Tree),
        report_columns: vec!["dashboard_uid".to_string()],
        list_columns: false,
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error
        .to_string()
        .contains("--report-columns is only supported with table, csv, or tree-table output."));
}

#[test]
fn validate_inspect_export_report_args_rejects_report_columns_for_governance_report() {
    let args = InspectExportArgs {
        input_dir: PathBuf::from("./dashboards/raw"),
        input_type: None,
        input_format: test_support::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::Governance),
        report_columns: vec!["dashboard_uid".to_string()],
        list_columns: false,
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error.to_string().contains("--report-columns"));
}

#[test]
fn validate_inspect_export_report_args_allows_report_columns_for_tree_table_report() {
    let args = InspectExportArgs {
        input_dir: PathBuf::from("./dashboards/raw"),
        input_type: None,
        input_format: test_support::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::TreeTable),
        report_columns: vec!["panel_id".to_string(), "query".to_string()],
        list_columns: false,
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    test_support::validate_inspect_export_report_args(&args).unwrap();
}

#[test]
fn validate_inspect_export_report_args_rejects_panel_filter_without_report() {
    let args = InspectExportArgs {
        input_dir: PathBuf::from("./dashboards/raw"),
        input_type: None,
        input_format: test_support::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: None,
        report_columns: Vec::new(),
        list_columns: false,
        report_filter_datasource: None,
        report_filter_panel_id: Some("7".to_string()),
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error
        .to_string()
        .contains("--report-filter-panel-id is only supported together with table, csv, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output."));
}
