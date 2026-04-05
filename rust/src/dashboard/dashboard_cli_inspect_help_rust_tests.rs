//! Dashboard CLI parser/help regressions kept separate from runtime-heavy tests.
use super::super::test_support;
use super::super::{
    parse_cli_from, DashboardCliArgs, DashboardCommand, DashboardImportInputFormat,
    InspectExportReportFormat, InspectOutputFormat, ValidationOutputFormat,
};
use super::dashboard_cli_parser_help_rust_tests::render_dashboard_subcommand_help;
use clap::{CommandFactory, Parser};
use std::path::{Path, PathBuf};

#[test]
fn parse_cli_supports_inspect_live_report_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--report",
        "json",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert_eq!(inspect_args.common.url, "https://grafana.example.com");
            assert_eq!(inspect_args.report, Some(InspectExportReportFormat::Json));
            assert_eq!(inspect_args.output_format, None);
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_output_format_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "governance-json",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert_eq!(inspect_args.common.url, "https://grafana.example.com");
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::GovernanceJson)
            );
            assert_eq!(inspect_args.report, None);
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_output_format_dependency_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "report-dependency-json",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert_eq!(inspect_args.common.url, "https://grafana.example.com");
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::ReportDependencyJson)
            );
            assert_eq!(inspect_args.report, None);
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_output_file() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "report-tree",
        "--output-file",
        "/tmp/inspect-live.txt",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert_eq!(
                inspect_args.output_file,
                Some(PathBuf::from("/tmp/inspect-live.txt"))
            );
            assert!(!inspect_args.also_stdout);
            assert_eq!(
                inspect_args.output_format,
                Some(InspectOutputFormat::ReportTree)
            );
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_also_stdout_with_output_file() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--output-file",
        "/tmp/inspect-live.txt",
        "--also-stdout",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert_eq!(
                inspect_args.output_file,
                Some(PathBuf::from("/tmp/inspect-live.txt"))
            );
            assert!(inspect_args.also_stdout);
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_report_tree_table_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--report",
        "tree-table",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert_eq!(inspect_args.common.url, "https://grafana.example.com");
            assert_eq!(
                inspect_args.report,
                Some(InspectExportReportFormat::TreeTable)
            );
            assert_eq!(inspect_args.output_format, None);
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_report_dependency_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--report",
        "dependency",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert_eq!(inspect_args.common.url, "https://grafana.example.com");
            assert_eq!(
                inspect_args.report,
                Some(InspectExportReportFormat::Dependency)
            );
            assert_eq!(inspect_args.output_format, None);
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_report_governance_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--report",
        "governance-json",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert_eq!(inspect_args.common.url, "https://grafana.example.com");
            assert_eq!(
                inspect_args.report,
                Some(InspectExportReportFormat::GovernanceJson)
            );
            assert_eq!(inspect_args.output_format, None);
            assert!(!inspect_args.json);
            assert!(!inspect_args.table);
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_help_full_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-live",
        "--url",
        "https://grafana.example.com",
        "--help-full",
    ]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert!(inspect_args.help_full);
            assert_eq!(inspect_args.common.url, "https://grafana.example.com");
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn parse_cli_supports_inspect_live_all_orgs_flag() {
    let args = parse_cli_from(["grafana-util", "inspect-live", "--all-orgs", "--table"]);

    match args.command {
        DashboardCommand::InspectLive(inspect_args) => {
            assert!(inspect_args.all_orgs);
            assert!(inspect_args.table);
            assert!(inspect_args.org_id.is_none());
        }
        _ => panic!("expected inspect-live command"),
    }
}

#[test]
fn inspect_live_help_mentions_interactive_browser() {
    let help = render_dashboard_subcommand_help("inspect-live");
    assert!(help.contains("--interactive"));
}

#[test]
fn parse_cli_supports_dashboard_validate_export_command() {
    let args = parse_cli_from([
        "grafana-util",
        "validate-export",
        "--import-dir",
        "./dashboards/raw",
        "--reject-custom-plugins",
        "--reject-legacy-properties",
        "--target-schema-version",
        "39",
        "--output-format",
        "json",
        "--output-file",
        "./dashboard-validation.json",
    ]);

    match args.command {
        DashboardCommand::ValidateExport(validate_args) => {
            assert_eq!(validate_args.import_dir, Path::new("./dashboards/raw"));
            assert_eq!(validate_args.input_format, DashboardImportInputFormat::Raw);
            assert!(validate_args.reject_custom_plugins);
            assert!(validate_args.reject_legacy_properties);
            assert_eq!(validate_args.target_schema_version, Some(39));
            assert_eq!(validate_args.output_format, ValidationOutputFormat::Json);
            assert_eq!(
                validate_args.output_file,
                Some(PathBuf::from("./dashboard-validation.json"))
            );
            assert!(!validate_args.also_stdout);
        }
        _ => panic!("expected validate-export command"),
    }
}

#[test]
fn parse_cli_supports_dashboard_validate_export_provisioning_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "validate-export",
        "--import-dir",
        "./dashboards/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DashboardCommand::ValidateExport(validate_args) => {
            assert_eq!(
                validate_args.import_dir,
                Path::new("./dashboards/provisioning")
            );
            assert_eq!(
                validate_args.input_format,
                DashboardImportInputFormat::Provisioning
            );
        }
        _ => panic!("expected validate-export command"),
    }
}

#[test]
fn validate_export_help_mentions_provisioning_input_format() {
    let help = render_dashboard_subcommand_help("validate-export");

    assert!(help.contains("--input-format"));
    assert!(help.contains("Grafana file-provisioning artifacts"));
    assert!(help.contains("provisioning/ root or its dashboards/ subdirectory"));
    assert!(help.contains("Validate a provisioning export root explicitly"));
}

#[test]
fn inspect_live_help_mentions_report_and_panel_filter_flags() {
    let help = render_dashboard_subcommand_help("inspect-live");

    assert!(help.contains("--report"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("text, table, csv, json, yaml"));
    assert!(help.contains("--report-filter-panel-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("--concurrency"));
    assert!(help.contains("--progress"));
    assert!(help.contains("--help-full"));
    assert!(help.contains("--also-stdout"));
    assert!(help.contains("tree"));
    assert!(help.contains("tree-table"));
    assert!(!help.contains("Extended Examples:"));
}

#[test]
fn inspect_export_help_lists_datasource_uid_report_column() {
    let mut command = DashboardCliArgs::command();
    let help = command
        .find_subcommand_mut("inspect-export")
        .expect("inspect-export subcommand")
        .render_help()
        .to_string();

    assert!(help.contains("datasource_uid"));
    assert!(help.contains("folder_level"));
    assert!(help.contains("folder_full_path"));
    assert!(help.contains("Use all to expand every supported column."));
    assert!(help.contains("datasource_type"));
    assert!(help.contains("datasource_family"));
    assert!(help.contains("dashboard_tags"));
    assert!(help.contains("panel_query_count"));
    assert!(help.contains("panel_datasource_count"));
    assert!(help.contains("panel_variables"));
    assert!(help.contains("query_variables"));
    assert!(help.contains("dashboardTags"));
    assert!(help.contains("panelQueryCount"));
    assert!(help.contains("panelDatasourceCount"));
    assert!(help.contains("panelVariables"));
    assert!(help.contains("queryVariables"));
    assert!(help.contains("file"));
    assert!(help.contains("dashboardUid"));
    assert!(help.contains("datasource label, uid, type, or family"));
    assert!(help.contains("--input-format"));
    assert!(help.contains("provisioning"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("text, table, csv, json, yaml"));
    assert!(help.contains("--interactive"));
}

#[test]
fn inspect_export_help_mentions_operator_summary_and_machine_readable_paths() {
    let help = render_dashboard_subcommand_help("inspect-export");

    assert!(help.contains("operator-summary table"));
    assert!(help.contains("operator-summary CSV"));
    assert!(help.contains("full machine-readable summary contract in JSON"));
    assert!(help.contains("full machine-readable summary contract in YAML"));
    assert!(help.contains("machine-readable governance contract"));
    assert!(help.contains("operator-summary views"));
    assert!(help.contains("report and contract views"));
    assert!(help.contains(
        "Analyze dashboard export directories with operator-summary and report-contract views."
    ));
}

#[test]
fn inspect_vars_help_mentions_all_baseline_output_formats() {
    let help = render_dashboard_subcommand_help("inspect-vars");

    assert!(help.contains("Render dashboard variables as table, csv, text, json, or yaml."));
    assert!(help.contains("output-format yaml"));
}

#[test]
fn parse_cli_supports_inspect_export_provisioning_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "inspect-export",
        "--import-dir",
        "./dashboards/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DashboardCommand::InspectExport(inspect_args) => {
            assert_eq!(
                inspect_args.input_format,
                DashboardImportInputFormat::Provisioning
            );
            assert_eq!(
                inspect_args.import_dir,
                Path::new("./dashboards/provisioning")
            );
        }
        _ => panic!("expected inspect-export command"),
    }
}

#[test]
fn inspect_export_help_full_includes_extended_examples() {
    let help = test_support::render_inspect_export_help_full();

    assert!(help.contains("--help-full"));
    assert!(help.contains("Extended Examples:"));
    assert!(help.contains("--interactive"));
    assert!(help.contains("--input-format raw"));
    assert!(help.contains("--input-format provisioning"));
    assert!(help.contains("provisioning root"));
    assert!(help.contains("--report tree-table"));
    assert!(help.contains("--report-filter-datasource"));
    assert!(help.contains("--report-filter-panel-id 7"));
    assert!(help.contains("--report-columns"));
    assert!(help.contains(
        "--report-columns dashboard_tags,panel_id,panel_query_count,panel_datasource_count,query_variables,panel_variables"
    ));
    assert!(help.contains(
        "--report-columns dashboard_uid,folder_path,folder_full_path,folder_level,folder_uid,parent_folder_uid,file"
    ));
    assert!(help.contains(
        "--report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query"
    ));
}

#[test]
fn inspect_live_help_full_includes_extended_examples() {
    let help = test_support::render_inspect_live_help_full();

    assert!(help.contains("--help-full"));
    assert!(help.contains("Extended Examples:"));
    assert!(help.contains("--interactive"));
    assert!(help.contains("--token \"$GRAFANA_API_TOKEN\""));
    assert!(help.contains("--report tree-table"));
    assert!(help.contains("--report-filter-panel-id"));
    assert!(help.contains("--report-columns"));
    assert!(help.contains(
        "--report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query"
    ));
    assert!(help.contains(
        "--report-columns dashboard_tags,panel_id,panel_query_count,panel_datasource_count,query_variables,panel_variables"
    ));
    assert!(help.contains(
        "--report-columns dashboard_uid,folder_path,folder_full_path,folder_level,folder_uid,parent_folder_uid,file"
    ));
    assert!(help.contains(
        "--report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query"
    ));
}

#[test]
fn maybe_render_dashboard_help_full_from_os_args_handles_missing_required_args() {
    let help = test_support::maybe_render_dashboard_help_full_from_os_args([
        "grafana-util",
        "dashboard",
        "inspect-export",
        "--help-full",
    ])
    .expect("expected inspect-export full help");

    assert!(help.contains("inspect-export"));
    assert!(help.contains("Extended Examples:"));
    assert!(help.contains("--report tree-table"));
    assert!(help.contains("--report-filter-panel-id 7"));
}

#[test]
fn maybe_render_dashboard_help_full_from_os_args_ignores_other_commands() {
    let help = test_support::maybe_render_dashboard_help_full_from_os_args([
        "grafana-util",
        "export",
        "--help-full",
    ]);

    assert!(help.is_none());
}

#[test]
fn parse_cli_supports_list_json_mode() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--json",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(!list_args.table);
            assert!(!list_args.csv);
            assert!(list_args.json);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_rejects_conflicting_list_output_modes() {
    let error = DashboardCliArgs::try_parse_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--table",
        "--json",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--table"));
    assert!(error.to_string().contains("--json"));
}

#[test]
fn parse_cli_supports_list_org_scope_flags() {
    let org_args = parse_cli_from(["grafana-util", "list", "--org-id", "7"]);
    let all_orgs_args = parse_cli_from(["grafana-util", "list", "--all-orgs"]);

    match org_args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, Some(7));
            assert!(!list_args.all_orgs);
        }
        _ => panic!("expected list command"),
    }

    match all_orgs_args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(list_args.all_orgs);
        }
        _ => panic!("expected list command"),
    }
}
