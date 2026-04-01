//! Feature-oriented inspect-export regressions.
//! Keeps the export-inspection parser coverage separate from the large dashboard test file.

use super::test_support::{
    parse_cli_from, DashboardCommand, DashboardImportInputFormat, InspectExportReportFormat,
    InspectOutputFormat,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn parse_cli_supports_inspect_export_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--import-dir",
        "./dashboards/raw",
        "--json",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
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
        "--import-dir",
        "./dashboards/raw",
        "--output-format",
        "report-tree-table",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::ReportTreeTable)
            );
            assert_eq!(inspect_args.report, None);
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
            "--import-dir",
            "./dashboards/raw",
            "--output-format",
            output_format,
        ]);

        match args.command {
            DashboardCommand::InspectExport(inspect_args) => {
                assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
                assert_eq!(inspect_args.output_format, Some(expected));
                assert_eq!(inspect_args.report, None);
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
        "--import-dir",
        "./dashboards/raw",
        "--output-format",
        "report-dependency",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::ReportDependency)
            );
            assert_eq!(inspect_args.report, None);
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
        "--import-dir",
        "./dashboards/raw",
        "--output-format",
        "report-json",
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
                Some(InspectOutputFormat::ReportJson)
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
        "--import-dir",
        "./dashboards/raw",
        "--interactive",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert!(inspect_args.interactive);
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "json",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(inspect_args.report, Some(InspectExportReportFormat::Json));
            assert_eq!(inspect_args.output_format, None);
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
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "csv",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(inspect_args.report, Some(InspectExportReportFormat::Csv));
            assert_eq!(inspect_args.output_format, None);
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
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "tree",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(inspect_args.report, Some(InspectExportReportFormat::Tree));
            assert_eq!(inspect_args.output_format, None);
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
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "tree-table",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.report,
                Some(InspectExportReportFormat::TreeTable)
            );
            assert_eq!(inspect_args.output_format, None);
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
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "dependency",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.report,
                Some(InspectExportReportFormat::Dependency)
            );
            assert_eq!(inspect_args.output_format, None);
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
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "dependency-json",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.report,
                Some(InspectExportReportFormat::DependencyJson)
            );
            assert_eq!(inspect_args.output_format, None);
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
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "governance",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(
                inspect_args.report,
                Some(InspectExportReportFormat::Governance)
            );
            assert_eq!(inspect_args.output_format, None);
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
        import_dir: provisioning_root,
        input_format: DashboardImportInputFormat::Provisioning,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        report: None,
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: None,
        interactive: false,
    };

    let dashboard_count = super::test_support::analyze_export_dir(&args).unwrap();
    assert_eq!(dashboard_count, 1);
}

#[test]
fn parse_cli_supports_inspect_export_help_full_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--import-dir",
        "./dashboards/raw",
        "--help-full",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert!(inspect_args.help_full);
            assert_eq!(inspect_args.import_dir, Path::new("./dashboards/raw"));
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn parse_cli_supports_inspect_export_report_columns_and_filter() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "--report-columns",
        "org,orgId,dashboard_uid,datasource,query",
        "--report-filter-datasource",
        "prom-main",
        "--report-filter-panel-id",
        "7",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.report, Some(InspectExportReportFormat::Table));
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
        "--import-dir",
        "./dashboards/raw",
        "--report",
        "csv",
        "--report-columns",
        "all",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(inspect_args.report, Some(InspectExportReportFormat::Csv));
            assert_eq!(inspect_args.report_columns, vec!["all".to_string()]);
        }
        _ => panic!("expected inspect-export command"),
    }
}
