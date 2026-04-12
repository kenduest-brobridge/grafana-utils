use super::*;

#[test]
fn browse_help_mentions_live_tree_controls() {
    let help = render_dashboard_subcommand_help("browse");
    assert!(help.contains("interactive terminal UI"));
    assert!(help.contains("--path"));
    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("--workspace"));
    assert!(help.contains("Open the browser at one folder subtree"));
    assert!(help.contains("Browse all visible orgs with Basic auth"));
    assert!(help.contains("local export tree"));
}

#[test]
fn browse_help_mentions_local_export_tree_examples() {
    let help = render_dashboard_subcommand_help("browse");
    assert!(help.contains("--input-dir ./dashboards/raw"));
    assert!(help.contains("Browse a raw export tree from disk"));
    assert!(help.contains("--workspace ./grafana-oac-repo"));
    assert!(help.contains("Browse one repo-backed workspace root from disk"));
}

#[test]
fn parse_cli_supports_browse_all_orgs() {
    let args = parse_cli_from(["grafana-util", "browse", "--all-orgs"]);

    match args.command {
        DashboardCommand::Browse(browse_args) => {
            assert!(browse_args.all_orgs);
            assert_eq!(browse_args.org_id, None);
        }
        _ => panic!("expected browse command"),
    }
}

#[test]
fn parse_cli_supports_screenshot_command() {
    let args = parse_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
    ]);

    match args.command {
        DashboardCommand::Screenshot(screenshot_args) => {
            assert_eq!(screenshot_args.dashboard_uid.as_deref(), Some("cpu-main"));
            assert_eq!(
                screenshot_args.output,
                std::path::Path::new("./cpu-main.png")
            );
            assert_eq!(screenshot_args.dashboard_url, None);
            assert!(!screenshot_args.full_page);
        }
        _ => panic!("expected screenshot command"),
    }
}

#[test]
fn screenshot_help_mentions_capture_options() {
    let help = render_dashboard_subcommand_help("screenshot");
    assert!(help.contains("--dashboard-uid"));
    assert!(help.contains("--dashboard-url"));
    assert!(help.contains("--output"));
    assert!(help.contains("--full-page"));
    assert!(help.contains("--browser-path"));
    assert!(help.contains("Capture a full dashboard from a browser URL"));
    assert!(help.contains("Capture a solo panel with a vars-query fragment"));
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
            assert!(!delete_args.prompt);
        }
        _ => panic!("expected delete command"),
    }
}

#[test]
fn parse_cli_supports_delete_prompt_and_interactive_alias() {
    for flag in ["--prompt", "--interactive"] {
        let args = parse_cli_from(["grafana-util", "delete", flag]);
        match args.command {
            DashboardCommand::Delete(delete_args) => assert!(delete_args.prompt),
            _ => panic!("expected delete command"),
        }
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
fn delete_help_mentions_prompt_and_delete_folders() {
    let help = render_dashboard_subcommand_help("delete");
    assert!(help.contains("--prompt"));
    assert!(!help.contains("--interactive"));
    assert!(help.contains("--delete-folders"));
    assert!(help.contains("--yes"));
}

#[test]
fn parse_cli_supports_history_restore_prompt_without_version() {
    let args = parse_cli_from([
        "grafana-util",
        "history",
        "restore",
        "--dashboard-uid",
        "cpu-main",
        "--prompt",
    ]);

    match args.command {
        DashboardCommand::History(history_args) => match history_args.command {
            DashboardHistorySubcommand::Restore(restore_args) => {
                assert_eq!(restore_args.dashboard_uid, "cpu-main");
                assert!(restore_args.prompt);
                assert_eq!(restore_args.version, None);
            }
            _ => panic!("expected history restore command"),
        },
        _ => panic!("expected history command"),
    }
}

#[test]
fn history_restore_help_mentions_prompt() {
    let help = render_dashboard_history_subcommand_help("restore");
    assert!(help.contains("--prompt"));
    assert!(help.contains("--version"));
    assert!(help.contains("Required unless --prompt is used"));
}

#[test]
fn parse_cli_supports_import_dry_run_output_columns() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
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
fn parse_cli_supports_import_dry_run_output_columns_all_and_list_columns() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./dashboards/raw",
        "--dry-run",
        "--output-format",
        "table",
        "--output-columns",
        "all",
        "--list-columns",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.table);
            assert_eq!(import_args.output_columns, vec!["all"]);
            assert!(import_args.list_columns);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_update_existing_only_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
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
        "--input-dir",
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
        "--input-dir",
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
        "--input-dir",
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
        "--input-dir",
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
        "--input-dir",
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
        "--input-dir",
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

#[test]
fn parse_cli_supports_dashboard_get_and_clone_live_commands() {
    let get_args = parse_cli_from([
        "grafana-util",
        "get",
        "--profile",
        "prod",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.json",
    ]);
    let clone_args = parse_cli_from([
        "grafana-util",
        "clone",
        "--source-uid",
        "cpu-main",
        "--name",
        "CPU Clone",
        "--uid",
        "cpu-main-clone",
        "--folder-uid",
        "infra",
        "--output",
        "./cpu-main-clone.json",
    ]);

    match get_args.command {
        DashboardCommand::Get(args) => {
            assert_eq!(args.common.profile.as_deref(), Some("prod"));
            assert_eq!(args.dashboard_uid, "cpu-main");
            assert_eq!(args.output, PathBuf::from("./cpu-main.json"));
        }
        _ => panic!("expected get command"),
    }

    match clone_args.command {
        DashboardCommand::CloneLive(args) => {
            assert_eq!(args.source_uid, "cpu-main");
            assert_eq!(args.name.as_deref(), Some("CPU Clone"));
            assert_eq!(args.uid.as_deref(), Some("cpu-main-clone"));
            assert_eq!(args.folder_uid.as_deref(), Some("infra"));
            assert_eq!(args.output, PathBuf::from("./cpu-main-clone.json"));
        }
        _ => panic!("expected clone command"),
    }
}

#[test]
fn dashboard_fetch_live_help_mentions_local_draft_and_output_path() {
    let help = render_dashboard_subcommand_help("get");
    assert!(help.contains("API-safe local JSON draft"));
    assert!(help.contains("What it does:"));
    assert!(help.contains("When to use:"));
    assert!(help.contains("--dashboard-uid"));
    assert!(help.contains("--output"));
}

#[test]
fn dashboard_clone_live_help_mentions_override_flags() {
    let help = render_dashboard_subcommand_help("clone");
    assert!(help.contains("optional overrides"));
    assert!(help.contains("What it does:"));
    assert!(help.contains("Related commands:"));
    assert!(help.contains("--source-uid"));
    assert!(help.contains("--name"));
    assert!(help.contains("--uid"));
    assert!(help.contains("--folder-uid"));
}

#[test]
fn dashboard_fetch_live_help_colorizes_section_headings_and_example_commands() {
    let help = crate::dashboard::maybe_render_dashboard_subcommand_help_from_os_args(
        ["grafana-util", "dashboard", "get", "--help"],
        true,
    )
    .expect("expected dashboard subcommand help");
    assert!(help.contains(&paint_section("What it does:")));
    assert!(help.contains(&paint_section("Examples:")));
}

#[test]
fn parse_cli_supports_dashboard_history_list_input_sources() {
    let input_args = parse_cli_from([
        "grafana-util",
        "history",
        "list",
        "--input",
        "./cpu-main.history.json",
        "--output-format",
        "json",
    ]);
    let import_dir_args = parse_cli_from([
        "grafana-util",
        "history",
        "list",
        "--input-dir",
        "./dashboards",
        "--dashboard-uid",
        "cpu-main",
        "--output-format",
        "yaml",
    ]);

    match input_args.command {
        DashboardCommand::History(history_args) => match history_args.command {
            DashboardHistorySubcommand::List(args) => {
                assert_eq!(args.dashboard_uid, None);
                assert_eq!(args.input, Some(PathBuf::from("./cpu-main.history.json")));
                assert_eq!(args.input_dir, None);
            }
            _ => panic!("expected history list subcommand"),
        },
        _ => panic!("expected history command"),
    }

    match import_dir_args.command {
        DashboardCommand::History(history_args) => match history_args.command {
            DashboardHistorySubcommand::List(args) => {
                assert_eq!(args.dashboard_uid.as_deref(), Some("cpu-main"));
                assert_eq!(args.input, None);
                assert_eq!(args.input_dir, Some(PathBuf::from("./dashboards")));
            }
            _ => panic!("expected history list subcommand"),
        },
        _ => panic!("expected history command"),
    }
}

#[test]
fn dashboard_history_list_help_mentions_local_inputs() {
    let help = render_dashboard_history_subcommand_help("list");
    assert!(help.contains("--input"));
    assert!(help.contains("--input-dir"));
    assert!(help.contains("dashboard history export"));
    assert!(help.contains("dashboard export --include-history"));
}

#[test]
fn dashboard_history_diff_help_mentions_dual_sources() {
    let help = render_dashboard_history_subcommand_help("diff");
    assert!(help.contains("--base-dashboard-uid"));
    assert!(help.contains("--new-dashboard-uid"));
    assert!(help.contains("--base-input"));
    assert!(help.contains("--new-input"));
    assert!(help.contains("--base-input-dir"));
    assert!(help.contains("--new-input-dir"));
    assert!(help.contains("--base-version"));
    assert!(help.contains("--new-version"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("Compare two historical dashboard revisions"));
}

#[test]
fn parse_cli_supports_dashboard_history_diff_with_local_artifacts() {
    let args = parse_cli_from([
        "grafana-util",
        "history",
        "diff",
        "--base-input",
        "./exports-2026-04-01/history/cpu-main.history.json",
        "--base-version",
        "17",
        "--new-input",
        "./exports-2026-04-07/history/cpu-main.history.json",
        "--new-version",
        "21",
        "--output-format",
        "json",
    ]);

    match args.command {
        DashboardCommand::History(history_args) => match history_args.command {
            DashboardHistorySubcommand::Diff(args) => {
                assert_eq!(
                    args.base_input,
                    Some(PathBuf::from(
                        "./exports-2026-04-01/history/cpu-main.history.json"
                    ))
                );
                assert_eq!(args.base_input_dir, None);
                assert_eq!(args.base_dashboard_uid, None);
                assert_eq!(args.base_version, 17);
                assert_eq!(
                    args.new_input,
                    Some(PathBuf::from(
                        "./exports-2026-04-07/history/cpu-main.history.json"
                    ))
                );
                assert_eq!(args.new_input_dir, None);
                assert_eq!(args.new_dashboard_uid, None);
                assert_eq!(args.new_version, 21);
                assert_eq!(args.output_format, crate::common::DiffOutputFormat::Json);
            }
            _ => panic!("expected history diff subcommand"),
        },
        _ => panic!("expected history command"),
    }
}
