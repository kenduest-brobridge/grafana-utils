//! Dashboard CLI parser/help regressions kept separate from runtime-heavy tests.
use super::super::test_support;
use super::super::{
    parse_cli_from, DashboardCliArgs, DashboardCommand, DashboardImportInputFormat,
    InspectOutputFormat, SimpleOutputFormat, ValidationOutputFormat,
};
use super::dashboard_cli_parser_help_rust_tests::render_dashboard_subcommand_help;
use clap::{CommandFactory, Parser};
use std::path::{Path, PathBuf};

#[test]
fn parse_cli_supports_summary_through_canonical_summary_command() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "governance-json",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
            assert!(analyze_args.input_dir.is_none());
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::GovernanceJson)
            );
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_summary_export_tree_through_canonical_summary_command() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--input-dir",
        "./dashboards/raw",
        "--input-format",
        "raw",
        "--output-format",
        "tree-table",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(
                analyze_args.input_dir,
                Some(PathBuf::from("./dashboards/raw"))
            );
            assert_eq!(analyze_args.input_format, DashboardImportInputFormat::Raw);
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::TreeTable)
            );
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_summary_export_git_sync_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--input-dir",
        "./dashboards/provisioning",
        "--input-format",
        "git-sync",
        "--output-format",
        "governance",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.input_format, DashboardImportInputFormat::Raw);
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::Governance)
            );
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_policy_git_sync_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "policy",
        "--input-dir",
        "./grafana-oac-repo",
        "--input-format",
        "git-sync",
        "--policy-source",
        "builtin",
        "--builtin-policy",
        "default",
    ]);

    match args.command {
        DashboardCommand::GovernanceGate(governance_args) => {
            assert_eq!(
                governance_args.input_format,
                DashboardImportInputFormat::Raw
            );
            assert_eq!(
                governance_args.input_dir.as_deref(),
                Some(Path::new("./grafana-oac-repo"))
            );
        }
        _ => panic!("expected policy command"),
    }
}

#[test]
fn parse_cli_supports_dependencies_git_sync_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "dependencies",
        "--input-dir",
        "./grafana-oac-repo",
        "--input-format",
        "git-sync",
    ]);

    match args.command {
        DashboardCommand::Topology(topology_args) => {
            assert_eq!(topology_args.input_format, DashboardImportInputFormat::Raw);
            assert_eq!(
                topology_args.input_dir.as_deref(),
                Some(Path::new("./grafana-oac-repo"))
            );
        }
        _ => panic!("expected dependencies command"),
    }
}

#[test]
fn parse_cli_supports_impact_git_sync_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "impact",
        "--input-dir",
        "./grafana-oac-repo",
        "--input-format",
        "git-sync",
        "--datasource-uid",
        "prom-main",
    ]);

    match args.command {
        DashboardCommand::Impact(impact_args) => {
            assert_eq!(impact_args.input_format, DashboardImportInputFormat::Raw);
            assert_eq!(
                impact_args.input_dir.as_deref(),
                Some(Path::new("./grafana-oac-repo"))
            );
        }
        _ => panic!("expected impact command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_queries_json_output_format() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "queries-json",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::QueriesJson)
            );
            assert!(!analyze_args.json);
            assert!(!analyze_args.table);
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_output_format_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "governance-json",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::GovernanceJson)
            );
            assert!(!analyze_args.json);
            assert!(!analyze_args.table);
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_output_format_dependency_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "dependency-json",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::DependencyJson)
            );
            assert!(!analyze_args.json);
            assert!(!analyze_args.table);
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_output_file() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "tree",
        "--output-file",
        "/tmp/analyze-live.txt",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(
                analyze_args.output_file,
                Some(PathBuf::from("/tmp/analyze-live.txt"))
            );
            assert!(!analyze_args.also_stdout);
            assert_eq!(analyze_args.output_format, Some(InspectOutputFormat::Tree));
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_also_stdout_with_output_file() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-file",
        "/tmp/analyze-live.txt",
        "--also-stdout",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(
                analyze_args.output_file,
                Some(PathBuf::from("/tmp/analyze-live.txt"))
            );
            assert!(analyze_args.also_stdout);
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_tree_table_output_format() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "tree-table",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::TreeTable)
            );
            assert!(!analyze_args.json);
            assert!(!analyze_args.table);
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_dependency_output_format() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "dependency",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::Dependency)
            );
            assert!(!analyze_args.json);
            assert!(!analyze_args.table);
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_governance_json_output_format() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "governance-json",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::GovernanceJson)
            );
            assert!(!analyze_args.json);
            assert!(!analyze_args.table);
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_help_full_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--help-full",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert!(analyze_args.help_full);
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_report_columns_all_and_list_columns() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "tree-table",
        "--report-columns",
        "all",
        "--list-columns",
    ]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert_eq!(analyze_args.common.url, "https://grafana.example.com");
            assert_eq!(
                analyze_args.output_format,
                Some(InspectOutputFormat::TreeTable)
            );
            assert_eq!(analyze_args.report_columns, vec!["all".to_string()]);
            assert!(analyze_args.list_columns);
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn parse_cli_supports_analyze_live_all_orgs_flag() {
    let args = parse_cli_from(["grafana-util", "summary", "--all-orgs", "--table"]);

    match args.command {
        DashboardCommand::Analyze(analyze_args) => {
            assert!(analyze_args.all_orgs);
            assert!(analyze_args.table);
            assert!(analyze_args.org_id.is_none());
        }
        _ => panic!("expected summary command"),
    }
}

#[test]
fn inspect_live_help_matches_fixture() {
    let help = render_dashboard_subcommand_help("summary");
    assert!(help.contains("--output-format governance-json"));
    assert!(help.contains("--list-columns"));
}

#[test]
fn parse_cli_supports_dashboard_validate_export_command() {
    let args = parse_cli_from([
        "grafana-util",
        "validate-export",
        "--input-dir",
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
            assert_eq!(validate_args.input_dir, Path::new("./dashboards/raw"));
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
        "--input-dir",
        "./dashboards/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DashboardCommand::ValidateExport(validate_args) => {
            assert_eq!(
                validate_args.input_dir,
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
fn parse_cli_supports_dashboard_validate_export_git_sync_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "validate-export",
        "--input-dir",
        "./grafana-oac-repo",
        "--input-format",
        "git-sync",
    ]);

    match args.command {
        DashboardCommand::ValidateExport(validate_args) => {
            assert_eq!(validate_args.input_dir, Path::new("./grafana-oac-repo"));
            assert_eq!(validate_args.input_format, DashboardImportInputFormat::Raw);
        }
        _ => panic!("expected validate-export command"),
    }
}

#[test]
fn validate_export_help_mentions_git_sync_input_format() {
    let help = render_dashboard_subcommand_help("validate-export");

    assert!(help.contains("--input-format"));
    assert!(help.contains("Grafana file-provisioning artifacts"));
    assert!(help.contains("provisioning/ root or its dashboards/ subdirectory"));
    assert!(help.contains("git-sync"));
    assert!(help.contains("Grafana OaC repo root"));
    assert!(help.contains("Validate a provisioning export root explicitly"));
}

#[test]
fn governance_gate_help_mentions_git_sync_input_format() {
    let help = render_dashboard_subcommand_help("policy");

    assert!(help.contains("git-sync"));
    assert!(help.contains("repo-backed Git Sync dashboard tree"));
}

#[test]
fn topology_help_mentions_git_sync_input_format() {
    let help = render_dashboard_subcommand_help("dependencies");

    assert!(help.contains("git-sync"));
    assert!(help.contains("repo-backed Git Sync dashboard tree"));
}

#[test]
fn impact_help_mentions_git_sync_input_format() {
    let help = render_dashboard_subcommand_help("impact");

    assert!(help.contains("git-sync"));
    assert!(help.contains("repo-backed Git Sync dashboard tree"));
}

#[test]
fn inspect_live_help_mentions_report_and_panel_filter_flags() {
    let help = render_dashboard_subcommand_help("summary");

    assert!(help.contains("--output-format"));
    assert!(help.contains("text, table, csv, json, yaml"));
    assert!(help.contains("queries-json"));
    assert!(help.contains("--report-filter-panel-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("--concurrency"));
    assert!(help.contains("--progress"));
    assert!(help.contains("--help-full"));
    assert!(help.contains("--also-stdout"));
    assert!(help.contains("tree"));
    assert!(help.contains("tree-table"));
}

#[test]
fn analyze_help_mentions_git_sync_input_format_alias() {
    let help = render_dashboard_subcommand_help("summary");

    assert!(help.contains("git-sync"));
    assert!(help.contains("repo-backed Git Sync dashboard tree"));
}

#[test]
fn inspect_export_help_lists_datasource_uid_report_column() {
    let mut command = DashboardCliArgs::command();
    let help = command
        .find_subcommand_mut("summary")
        .expect("summary subcommand")
        .render_help()
        .to_string();

    assert!(help.contains("--input-format"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("--interactive"));
}

#[test]
fn inspect_export_help_mentions_operator_summary_and_machine_readable_paths() {
    let help = render_dashboard_subcommand_help("summary");

    assert!(help.contains("--interactive"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("queries-json"));
    assert!(help.contains("--report-columns"));
}

#[test]
fn inspect_vars_help_mentions_all_baseline_output_formats() {
    let help = render_dashboard_subcommand_help("variables");

    assert!(help.contains("Render dashboard variables as table, csv, text, json, or yaml."));
    assert!(help.contains("output-format yaml"));
    assert!(help.contains("local dashboard file"));
    assert!(help.contains("local export tree"));
}

#[test]
fn parse_cli_supports_list_vars_local_input_file() {
    let args = parse_cli_from([
        "grafana-util",
        "variables",
        "--input",
        "./dashboards/raw/cpu-main.json",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        DashboardCommand::InspectVars(inspect_args) => {
            assert_eq!(
                inspect_args.input,
                Some(PathBuf::from("./dashboards/raw/cpu-main.json"))
            );
            assert!(inspect_args.input_dir.is_none());
            assert_eq!(inspect_args.output_format, Some(SimpleOutputFormat::Yaml));
        }
        other => panic!("expected variables command, got {other:?}"),
    }
}

#[test]
fn parse_cli_supports_list_vars_local_import_dir() {
    let args = parse_cli_from([
        "grafana-util",
        "variables",
        "--input-dir",
        "./dashboards/raw",
        "--dashboard-uid",
        "cpu-main",
        "--input-format",
        "raw",
        "--output-format",
        "table",
    ]);

    match args.command {
        DashboardCommand::InspectVars(inspect_args) => {
            assert_eq!(
                inspect_args.input_dir,
                Some(PathBuf::from("./dashboards/raw"))
            );
            assert_eq!(inspect_args.input_format, DashboardImportInputFormat::Raw);
            assert_eq!(inspect_args.dashboard_uid.as_deref(), Some("cpu-main"));
            assert_eq!(inspect_args.output_format, Some(SimpleOutputFormat::Table));
        }
        other => panic!("expected variables command, got {other:?}"),
    }
}

#[test]
fn parse_cli_supports_analyze_export_provisioning_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "summary",
        "--input-dir",
        "./dashboards/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DashboardCommand::Analyze(inspect_args) => {
            assert_eq!(
                inspect_args.input_format,
                DashboardImportInputFormat::Provisioning
            );
            assert_eq!(
                inspect_args.input_dir,
                Some(PathBuf::from("./dashboards/provisioning"))
            );
        }
        _ => panic!("expected summary command"),
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
    assert!(help.contains("--output-format tree-table"));
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
    assert!(help.contains("--output-format tree-table"));
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
        "summary",
        "--help-full",
    ])
    .expect("expected summary full help");

    assert!(help.contains("summary"));
    assert!(help.contains("Extended Examples:"));
    assert!(help.contains("--output-format tree-table"));
    assert!(help.contains("Render a live Grafana dashboard summary as governance JSON"));
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
