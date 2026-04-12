use super::*;

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
            assert!(!list_args.show_sources);
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
fn parse_cli_supports_list_show_sources() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--show-sources",
        "--json",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(list_args.show_sources);
            assert!(list_args.output_columns.is_empty());
            assert!(list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_list_with_sources_alias() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--with-sources",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert!(list_args.show_sources);
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
            assert!(!list_args.show_sources);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_list_output_columns_all_and_list_columns() {
    let args = parse_cli_from([
        "grafana-util",
        "list",
        "--url",
        "https://grafana.example.com",
        "--output-columns",
        "all",
        "--list-columns",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.output_columns, vec!["all"]);
            assert!(list_args.list_columns);
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
    let history_args = parse_cli_from(["grafana-util", "export", "--include-history"]);

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

    match history_args.command {
        DashboardCommand::Export(export_args) => {
            assert!(export_args.include_history);
            assert!(!export_args.all_orgs);
            assert_eq!(export_args.org_id, None);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_export_provisioning_provider_customization() {
    let args = parse_cli_from([
        "grafana-util",
        "export",
        "--provider-name",
        "grafana-utils-prod",
        "--provider-org-id",
        "9",
        "--provider-path",
        "/srv/grafana/dashboards",
        "--provider-disable-deletion",
        "--provider-allow-ui-updates",
        "--provider-update-interval-seconds",
        "45",
    ]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.provisioning_provider_name, "grafana-utils-prod");
            assert_eq!(export_args.provisioning_provider_org_id, Some(9));
            assert_eq!(
                export_args.provisioning_provider_path,
                Some(PathBuf::from("/srv/grafana/dashboards"))
            );
            assert!(export_args.provisioning_provider_disable_deletion);
            assert!(export_args.provisioning_provider_allow_ui_updates);
            assert_eq!(
                export_args.provisioning_provider_update_interval_seconds,
                45
            );
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_keeps_export_provisioning_provider_aliases() {
    let args = parse_cli_from([
        "grafana-util",
        "export",
        "--provisioning-provider-name",
        "grafana-utils-prod",
        "--provisioning-provider-org-id",
        "9",
        "--provisioning-provider-path",
        "/srv/grafana/dashboards",
        "--provisioning-provider-disable-deletion",
        "--provisioning-provider-allow-ui-updates",
        "--provisioning-provider-update-interval-seconds",
        "45",
        "--without-dashboard-raw",
        "--without-dashboard-prompt",
        "--without-dashboard-provisioning",
    ]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.provisioning_provider_name, "grafana-utils-prod");
            assert_eq!(export_args.provisioning_provider_org_id, Some(9));
            assert_eq!(
                export_args.provisioning_provider_path,
                Some(PathBuf::from("/srv/grafana/dashboards"))
            );
            assert!(export_args.provisioning_provider_disable_deletion);
            assert!(export_args.provisioning_provider_allow_ui_updates);
            assert_eq!(
                export_args.provisioning_provider_update_interval_seconds,
                45
            );
            assert!(export_args.without_dashboard_raw);
            assert!(export_args.without_dashboard_prompt);
            assert!(export_args.without_dashboard_provisioning);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_export_provisioning_provider_defaults() {
    let args = parse_cli_from(["grafana-util", "export"]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(
                export_args.provisioning_provider_name,
                "grafana-utils-dashboards"
            );
            assert_eq!(export_args.provisioning_provider_org_id, None);
            assert_eq!(export_args.provisioning_provider_path, None);
            assert!(!export_args.provisioning_provider_disable_deletion);
            assert!(!export_args.provisioning_provider_allow_ui_updates);
            assert_eq!(
                export_args.provisioning_provider_update_interval_seconds,
                30
            );
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
    assert!(help.contains("--include-history"));
    assert!(help.contains("Use Basic auth with --all-orgs."));
    assert!(help.contains("Export dashboards across all visible orgs with Basic auth"));
    assert!(help.contains("Write files directly into each export variant directory"));
    assert!(help.contains("files directly under each variant directory"));
}

#[test]
fn export_help_mentions_history_artifacts() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("history/"));
    assert!(help.contains("Use --include-history to add history/"));
    assert!(!help.contains("per-dashboard revision history"));
}

#[test]
fn export_help_keeps_option_summaries_short() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("Export dashboards from every visible Grafana org."));
    assert!(help.contains("Skip the raw/ export variant."));
    assert!(help.contains("Skip the prompt/ export variant."));
    assert!(help.contains("Skip the provisioning/ export variant."));
    assert!(help.contains("Also write history/ artifacts for each exported org scope."));
    assert!(
        !help.contains("Use this only when you do not need later API import or diff workflows.")
    );
    assert!(!help
        .contains("Use this only when you do not need Grafana UI import with datasource prompts."));
    assert!(
        !help.contains("Use this only when you do not need Grafana file provisioning artifacts.")
    );
}

#[test]
fn history_restore_help_uses_operator_facing_summary() {
    let help = render_dashboard_history_subcommand_help("restore");
    assert!(help.contains("Restore a previous live dashboard revision from Grafana history."));
    assert!(!help.contains("new latest revision entry on the same dashboard"));
}

#[test]
fn export_help_describes_progress_and_verbose_modes() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("--progress"));
    assert!(help.contains("<current>/<total>"));
    assert!(help.contains("-v, --verbose"));
    assert!(help.contains("Overrides --progress"));
    assert!(!help.contains("--username"));
    assert!(!help.contains("--password "));
}

#[test]
fn export_help_mentions_provisioning_provider_customization() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("provisioning/provisioning/dashboards.yaml"));
    assert!(help.contains("--provider-name"));
    assert!(help.contains("--provider-org-id"));
    assert!(help.contains("--provider-path"));
    assert!(help.contains("--provider-disable-deletion"));
    assert!(help.contains("--provider-allow-ui-updates"));
    assert!(help.contains("--provider-update-interval-seconds"));
    assert!(help.contains("The provider file is"));
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
    assert!(help.contains("--interactive"));
    assert!(help.contains("choose which exported dashboards to import"));
}

#[test]
fn top_level_help_includes_examples() {
    let help = render_dashboard_help();
    assert!(help.contains("Export dashboards from local Grafana with Basic auth"));
    assert!(help.contains("Export dashboards across all visible orgs with Basic auth"));
    assert!(help.contains("List dashboards across all visible orgs with Basic auth"));
    assert!(help.contains("Export dashboards with an API token from the current org"));
    assert!(help.contains("grafana-util dashboard export"));
    assert!(help.contains("grafana-util dashboard list"));
    assert!(help.contains("grafana-util dashboard diff"));
    assert!(help.contains("grafana-util dashboard publish"));
    assert!(help.contains("grafana-util dashboard screenshot"));
    assert!(help.contains("--all-orgs"));
    assert!(!help.contains("grafana-util export"));
    assert!(!help.contains("grafana-util list"));
    assert!(!help.contains("grafana-util diff"));
    assert!(!help.contains("grafana-util publish"));
}
