//! Feature-oriented inspect-export regressions.
//! Keeps the export-inspection parser coverage separate from the large dashboard test file.

use super::test_support::{
    parse_cli_from, DashboardCommand, DashboardImportInputFormat, InspectOutputFormat,
};
use crate::dashboard::cli_defs::InspectExportInputType;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn parse_cli_supports_inspect_export_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--json",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert!(inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_output_format_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "tree-table",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::TreeTable)
            );
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_baseline_output_formats() {
    for (output_format, expected) in [
        ("text", InspectOutputFormat::Text),
        ("table", InspectOutputFormat::Table),
        ("csv", InspectOutputFormat::Csv),
        ("json", InspectOutputFormat::Json),
        ("yaml", InspectOutputFormat::Yaml),
    ] {
        let args = parse_cli_from([
            "grafana-util",
            "inspect-export",
            "--input-dir",
            "./dashboards/raw",
            "--output-format",
            output_format,
        ]);

        match args.command {
            DashboardCommand::InspectExport(inspect_args) => {
                assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
                assert_eq!(inspect_args.output_format, Some(expected));
                assert!(!inspect_args.json);
                assert!(!inspect_args.table);
            }
            _ => panic!("expected inspect-export command"),
        }
    }
}

#[test]
fn parse_cli_supports_inspect_export_output_format_dependency_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "dependency",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::Dependency)
            );
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_output_file() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "queries-json",
        "--output-file",
        "/tmp/inspect-export.txt",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(
                inspect_args.output_file,
                Some(PathBuf::from("/tmp/inspect-export.txt"))
            );
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::QueriesJson)
            );
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_interactive_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--interactive",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert!(inspect_args.interactive);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_input_type_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards",
        "--input-type",
        "source",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards"));
            assert_eq!(
                inspect_args.input_type,
                Some(InspectExportInputType::Source)
            );
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "queries-json",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::QueriesJson)
            );
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_csv_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "csv",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(inspect_args.output_format, Some(InspectOutputFormat::Csv));
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_tree_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "tree",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(inspect_args.output_format, Some(InspectOutputFormat::Tree));
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_tree_table_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "tree-table",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::TreeTable)
            );
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_dependency_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "dependency",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::Dependency)
            );
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_dependency_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "dependency-json",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::DependencyJson)
            );
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_governance_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "governance",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::Governance)
            );
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn analyze_export_dir_supports_explicit_provisioning_input_format() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    let dashboards_dir = provisioning_root.join("dashboards").join("team");
    fs::create_dir_all(&dashboards_dir).unwrap();
    fs::write(
        dashboards_dir.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "panels": []
        }))
        .unwrap(),
    )
    .unwrap();

    let args = super::test_support::InspectExportArgs {
        input_dir: provisioning_root,
        input_type: None,
        input_format: DashboardImportInputFormat::Provisioning,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let dashboard_count = super::test_support::analyze_export_dir(&args).unwrap();
    assert_eq!(dashboard_count, 1);
}

#[test]
fn analyze_export_dir_accepts_workspace_wrapper_root_when_dashboards_metadata_exists() {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("workspace");
    let dashboard_root = workspace_root.join("dashboards");
    let datasource_root = workspace_root.join("datasources");
    let raw_dir = dashboard_root.join("org_1_Main_Org").join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::create_dir_all(&datasource_root).unwrap();
    fs::write(
        dashboard_root.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "root",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "orgCount": 1,
            "orgs": [{"org": "Main Org.", "orgId": "1", "dashboardCount": 1}]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "panels": []
        }))
        .unwrap(),
    )
    .unwrap();

    let args = super::test_support::InspectExportArgs {
        input_dir: workspace_root.clone(),
        input_type: None,
        input_format: DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let dashboard_count = super::test_support::analyze_export_dir(&args).unwrap();
    assert_eq!(dashboard_count, 1);
}

#[test]
fn analyze_export_dir_requires_input_type_for_dashboard_root_with_raw_and_prompt_variants() {
    let temp = tempdir().unwrap();
    let dashboard_root = temp.path().join("dashboards");
    let raw_dir = dashboard_root.join("org_1_Main_Org").join("raw");
    let prompt_dir = dashboard_root.join("org_1_Main_Org").join("prompt");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::create_dir_all(&prompt_dir).unwrap();
    fs::write(
        dashboard_root.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "root",
            "dashboardCount": 2,
            "indexFile": "index.json",
            "orgCount": 1,
            "orgs": [{"org": "Main Org.", "orgId": "1", "dashboardCount": 2}]
        }))
        .unwrap(),
    )
    .unwrap();

    let args = super::test_support::InspectExportArgs {
        input_dir: dashboard_root,
        input_type: None,
        input_format: DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let error = super::test_support::analyze_export_dir(&args).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("contains both raw/ and prompt/ dashboard variants"));
    assert!(text.contains("--input-type raw"));
    assert!(text.contains("--input-type source"));
}

#[test]
fn parse_cli_supports_inspect_export_help_full_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--help-full",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert!(inspect_args.help_full);
            assert_eq!(inspect_args.input_dir, Path::new("./dashboards/raw"));
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_columns_and_filter() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "tree-table",
        "--report-columns",
        "org,orgId,dashboard_uid,datasource,query",
        "--report-filter-datasource",
        "prom-main",
        "--report-filter-panel-id",
        "7",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::TreeTable)
            );
            assert_eq!(
                inspect_args.report_columns,
                vec![
                    "org".to_string(),
                    "org_id".to_string(),
                    "dashboard_uid".to_string(),
                    "datasource".to_string(),
                    "query".to_string()
                ]
            );
            assert_eq!(
                inspect_args.report_filter_datasource,
                Some("prom-main".to_string())
            );
            assert_eq!(inspect_args.report_filter_panel_id, Some("7".to_string()));
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_columns_all() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--input-dir",
        "./dashboards/raw",
        "--output-format",
        "tree-table",
        "--report-columns",
        "all",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::TreeTable)
            );
            assert_eq!(inspect_args.report_columns, vec!["all".to_string()]);
        }
        _ => panic!("expected inspect-export command"),
    }
}
