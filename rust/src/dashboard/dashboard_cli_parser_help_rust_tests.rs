//! Dashboard CLI parser/help regressions kept separate from runtime-heavy tests.
use super::super::{parse_cli_from, DashboardCliArgs, DashboardCommand};
use clap::{CommandFactory, Parser};

pub(super) fn render_dashboard_subcommand_help(name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    command
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing {name} subcommand"))
        .render_help()
        .to_string()
}

pub(super) fn render_dashboard_help() -> String {
    let mut command = DashboardCliArgs::command();
    command.render_help().to_string()
}

#[test]
fn parse_cli_supports_list_mode() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--page-size",
        "25",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.common.url, "https://grafana.example.com");
            assert_eq!(list_args.page_size, 25);
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(!list_args.with_sources);
            assert!(list_args.output_columns.is_empty());
            assert!(!list_args.table);
            assert!(!list_args.csv);
            assert!(!list_args.json);
            assert!(!list_args.no_header);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_list_with_sources() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--with-sources",
        "--json",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(list_args.with_sources);
            assert!(list_args.output_columns.is_empty());
            assert!(list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_list_output_columns_with_aliases() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--output-columns",
        "uid,folderUid,orgId,sources,sourceUids",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(
                list_args.output_columns,
                vec!["uid", "folder_uid", "org_id", "sources", "source_uids"]
            );
            assert!(!list_args.with_sources);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_list_output_format_csv() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--output-format",
        "csv",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(!list_args.table);
            assert!(list_args.csv);
            assert!(!list_args.json);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_preferred_auth_aliases() {
    let args = parse_cli_from([
        "grafana-util",
        "export",
        "--token",
        "abc123",
        "--basic-user",
        "user",
        "--basic-password",
        "pass",
    ]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.common.api_token.as_deref(), Some("abc123"));
            assert_eq!(export_args.common.username.as_deref(), Some("user"));
            assert_eq!(export_args.common.password.as_deref(), Some("pass"));
            assert!(!export_args.common.prompt_password);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_prompt_password() {
    let args = parse_cli_from([
        "grafana-util",
        "export",
        "--basic-user",
        "user",
        "--prompt-password",
    ]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.common.username.as_deref(), Some("user"));
            assert_eq!(export_args.common.password.as_deref(), None);
            assert!(export_args.common.prompt_password);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_prompt_token() {
    let args = parse_cli_from(["grafana-util", "export", "--prompt-token"]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.common.api_token.as_deref(), None);
            assert!(export_args.common.prompt_token);
            assert!(!export_args.common.prompt_password);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_export_org_scope_flags() {
    let org_args = parse_cli_from(["grafana-util", "export", "--org-id", "7"]);
    let all_orgs_args = parse_cli_from(["grafana-util", "export", "--all-orgs"]);

    match org_args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.org_id, Some(7));
            assert!(!export_args.all_orgs);
        }
        _ => panic!("expected export command"),
    }

    match all_orgs_args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.org_id, None);
            assert!(export_args.all_orgs);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_rejects_conflicting_export_org_scope_flags() {
    let error =
        DashboardCliArgs::try_parse_from(["grafana-util", "export", "--org-id", "7", "--all-orgs"])
            .unwrap_err();

    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--all-orgs"));
}

#[test]
fn export_help_explains_flat_layout() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("Prefer Basic auth when you need cross-org export"));
    assert!(help.contains("Export dashboards across all visible orgs with Basic auth"));
    assert!(help.contains("Write dashboard files directly into each export variant directory"));
    assert!(help.contains("folder-based subdirectories on disk"));
}

#[test]
fn export_help_describes_progress_and_verbose_modes() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("--progress"));
    assert!(help.contains("<current>/<total>"));
    assert!(help.contains("-v, --verbose"));
    assert!(help.contains("Overrides --progress output"));
    assert!(!help.contains("--username"));
    assert!(!help.contains("--password "));
}

#[test]
fn import_help_explains_common_operator_flags() {
    let help = render_dashboard_subcommand_help("import");
    assert!(help.contains("Use the raw/ export directory for single-org import"));
    assert!(help.contains("folder missing/match/mismatch state"));
    assert!(help.contains("skipped/blocked"));
    assert!(help.contains("folder check is also shown in table form"));
    assert!(help.contains("source raw folder path matches"));
    assert!(help.contains("--org-id"));
    assert!(help.contains("--use-export-org"));
    assert!(help.contains("--only-org-id"));
    assert!(help.contains("--create-missing-orgs"));
    assert!(help.contains("requires Basic auth"));
    assert!(help.contains("--require-matching-export-org"));
    assert!(help.contains("--output-columns"));
}

#[test]
fn top_level_help_includes_examples() {
    let help = render_dashboard_help();
    assert!(help.contains("Export dashboards from local Grafana with Basic auth"));
    assert!(help.contains("Export dashboards across all visible orgs with Basic auth"));
    assert!(help.contains("List dashboards across all visible orgs with Basic auth"));
    assert!(help.contains("Export dashboards with an API token from the current org"));
    assert!(help.contains("grafana-util export"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util diff"));
}

#[test]
fn list_help_mentions_cross_org_basic_auth_examples() {
    let help = render_dashboard_subcommand_help("list");
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("Prefer Basic auth when you need cross-org listing"));
    assert!(help.contains("List dashboards across all visible orgs with Basic auth"));
    assert!(help.contains("List dashboards from one explicit org ID"));
    assert!(help.contains("List dashboards from the current org with an API token"));
}

#[test]
fn parse_cli_supports_list_csv_mode() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--csv",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(!list_args.table);
            assert!(list_args.csv);
            assert!(!list_args.json);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_export_progress_and_verbose_flags() {
    let args = parse_cli_from(["grafana-util", "export", "--progress", "--verbose"]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert!(export_args.progress);
            assert!(export_args.verbose);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_import_progress_and_verbose_flags() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--progress",
        "-v",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.progress);
            assert!(import_args.verbose);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_dry_run_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--dry-run",
        "--json",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.dry_run);
            assert!(import_args.json);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_dry_run_output_format_table() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.dry_run);
            assert!(import_args.table);
            assert!(!import_args.json);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_browse_with_path() {
    let args = parse_cli_from([
        "grafana-util",
        "browse",
        "--url",
        "https://grafana.example.com",
        "--org-id",
        "2",
        "--path",
        "Platform / Infra",
    ]);

    match args.command {
        DashboardCommand::Browse(browse_args) => {
            assert_eq!(browse_args.common.url, "https://grafana.example.com");
            assert_eq!(browse_args.org_id, Some(2));
            assert_eq!(browse_args.path.as_deref(), Some("Platform / Infra"));
            assert_eq!(browse_args.page_size, 500);
        }
        _ => panic!("expected browse command"),
    }
}

#[test]
fn browse_help_mentions_live_tree_controls() {
    let help = render_dashboard_subcommand_help("browse");
    assert!(help.contains("interactive terminal UI"));
    assert!(help.contains("--path"));
    assert!(help.contains("--org-id"));
    assert!(help.contains("Open the browser at one folder subtree"));
}

#[test]
fn parse_cli_supports_delete_by_uid() {
    let args = parse_cli_from([
        "grafana-util",
        "delete",
        "--url",
        "https://grafana.example.com",
        "--uid",
        "cpu-main",
    ]);

    match args.command {
        DashboardCommand::Delete(delete_args) => {
            assert_eq!(delete_args.uid.as_deref(), Some("cpu-main"));
            assert_eq!(delete_args.path, None);
            assert!(!delete_args.delete_folders);
            assert!(!delete_args.interactive);
        }
        _ => panic!("expected delete command"),
    }
}

#[test]
fn parse_cli_supports_delete_output_format_json() {
    let args = parse_cli_from([
        "grafana-util",
        "delete",
        "--uid",
        "cpu-main",
        "--output-format",
        "json",
    ]);

    match args.command {
        DashboardCommand::Delete(delete_args) => {
            assert!(delete_args.json);
            assert!(!delete_args.table);
        }
        _ => panic!("expected delete command"),
    }
}

#[test]
fn delete_help_mentions_interactive_and_delete_folders() {
    let help = render_dashboard_subcommand_help("delete");
    assert!(help.contains("--interactive"));
    assert!(help.contains("--delete-folders"));
    assert!(help.contains("--yes"));
}

#[test]
fn parse_cli_supports_import_dry_run_output_columns() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--dry-run",
        "--output-format",
        "table",
        "--output-columns",
        "uid,action,source_folder_path,destinationFolderPath,reason,file",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.table);
            assert_eq!(
                import_args.output_columns,
                vec![
                    "uid",
                    "action",
                    "source_folder_path",
                    "destination_folder_path",
                    "reason",
                    "file",
                ]
            );
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_update_existing_only_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--update-existing-only",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.update_existing_only);
            assert!(!import_args.replace_existing);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_require_matching_folder_path_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--require-matching-folder-path",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.require_matching_folder_path);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_org_scope_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--org-id",
        "7",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert_eq!(import_args.org_id, Some(7));
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_by_export_org_flags() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards",
        "--use-export-org",
        "--only-org-id",
        "2",
        "--only-org-id",
        "5",
        "--create-missing-orgs",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.use_export_org);
            assert_eq!(import_args.only_org_id, vec![2, 5]);
            assert!(import_args.create_missing_orgs);
            assert_eq!(import_args.org_id, None);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_require_matching_export_org_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--require-matching-export-org",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.require_matching_export_org);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_rejects_import_org_id_with_use_export_org() {
    let error = DashboardCliArgs::try_parse_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards",
        "--org-id",
        "7",
        "--use-export-org",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--use-export-org"));
}

#[test]
fn parse_cli_supports_import_use_export_org_flags() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./dashboards",
        "--use-export-org",
        "--only-org-id",
        "2",
        "--only-org-id",
        "5",
        "--create-missing-orgs",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.use_export_org);
            assert_eq!(import_args.only_org_id, vec![2, 5]);
            assert!(import_args.create_missing_orgs);
        }
        _ => panic!("expected import command"),
    }
}
