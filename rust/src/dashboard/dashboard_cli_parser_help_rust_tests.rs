//! Dashboard CLI parser/help regressions kept separate from runtime-heavy tests.
use super::super::{
    parse_cli_from, DashboardCliArgs, DashboardCommand, DashboardHistorySubcommand,
    RawToPromptLogFormat, RawToPromptOutputFormat, RawToPromptResolution, SimpleOutputFormat,
};
use crate::common::CliColorChoice;
use crate::dashboard::DashboardImportInputFormat;
use clap::{CommandFactory, Parser};
use std::path::PathBuf;

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

pub(super) fn render_dashboard_history_subcommand_help(name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    let history = command
        .find_subcommand_mut("history")
        .unwrap_or_else(|| panic!("missing history subcommand"));
    history
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing history {name} subcommand"))
        .render_help()
        .to_string()
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
fn parse_cli_supports_raw_to_prompt_command_defaults() {
    let args = parse_cli_from([
        "grafana-util",
        "raw-to-prompt",
        "--input-file",
        "./dashboards/raw/cpu-main.json",
    ]);

    match args.command {
        DashboardCommand::RawToPrompt(raw_args) => {
            assert_eq!(
                raw_args.input_file,
                vec![PathBuf::from("./dashboards/raw/cpu-main.json")]
            );
            assert_eq!(raw_args.input_dir, None);
            assert_eq!(raw_args.output_file, None);
            assert_eq!(raw_args.output_dir, None);
            assert!(!raw_args.overwrite);
            assert_eq!(raw_args.output_format, RawToPromptOutputFormat::Text);
            assert!(!raw_args.no_header);
            assert_eq!(raw_args.color, CliColorChoice::Auto);
            assert!(!raw_args.progress);
            assert!(!raw_args.verbose);
            assert!(!raw_args.dry_run);
            assert_eq!(raw_args.log_file, None);
            assert_eq!(raw_args.log_format, RawToPromptLogFormat::Text);
            assert_eq!(raw_args.resolution, RawToPromptResolution::InferFamily);
            assert_eq!(raw_args.datasource_map, None);
            assert_eq!(raw_args.profile, None);
            assert_eq!(raw_args.url, None);
            assert_eq!(raw_args.api_token, None);
            assert_eq!(raw_args.username, None);
            assert_eq!(raw_args.password, None);
            assert!(!raw_args.prompt_password);
            assert!(!raw_args.prompt_token);
            assert_eq!(raw_args.org_id, None);
            assert_eq!(raw_args.timeout, None);
            assert!(!raw_args.verify_ssl);
        }
        _ => panic!("expected raw-to-prompt command"),
    }
}

#[test]
fn parse_cli_supports_raw_to_prompt_command_flags() {
    let args = parse_cli_from([
        "grafana-util",
        "raw-to-prompt",
        "--input-dir",
        "./dashboards/raw",
        "--output-dir",
        "./dashboards/prompt",
        "--output-format",
        "yaml",
        "--no-header",
        "--color",
        "always",
        "--progress",
        "--verbose",
        "--dry-run",
        "--log-file",
        "./raw-to-prompt.log",
        "--log-format",
        "json",
        "--resolution",
        "exact",
        "--datasource-map",
        "./datasource-map.json",
        "--profile",
        "prod",
        "--url",
        "https://grafana.example.com",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--org-id",
        "2",
        "--timeout",
        "45",
        "--verify-ssl",
        "--overwrite",
    ]);

    match args.command {
        DashboardCommand::RawToPrompt(raw_args) => {
            assert_eq!(raw_args.input_file, Vec::<PathBuf>::new());
            assert_eq!(raw_args.input_dir, Some(PathBuf::from("./dashboards/raw")));
            assert_eq!(
                raw_args.output_dir,
                Some(PathBuf::from("./dashboards/prompt"))
            );
            assert_eq!(raw_args.output_file, None);
            assert!(raw_args.overwrite);
            assert_eq!(raw_args.output_format, RawToPromptOutputFormat::Yaml);
            assert!(raw_args.no_header);
            assert_eq!(raw_args.color, CliColorChoice::Always);
            assert!(raw_args.progress);
            assert!(raw_args.verbose);
            assert!(raw_args.dry_run);
            assert_eq!(
                raw_args.log_file,
                Some(PathBuf::from("./raw-to-prompt.log"))
            );
            assert_eq!(raw_args.log_format, RawToPromptLogFormat::Json);
            assert_eq!(raw_args.resolution, RawToPromptResolution::Exact);
            assert_eq!(
                raw_args.datasource_map,
                Some(PathBuf::from("./datasource-map.json"))
            );
            assert_eq!(raw_args.profile.as_deref(), Some("prod"));
            assert_eq!(raw_args.url.as_deref(), Some("https://grafana.example.com"));
            assert_eq!(raw_args.username.as_deref(), Some("admin"));
            assert_eq!(raw_args.password.as_deref(), Some("admin"));
            assert_eq!(raw_args.org_id, Some(2));
            assert_eq!(raw_args.timeout, Some(45));
            assert!(raw_args.verify_ssl);
        }
        _ => panic!("expected raw-to-prompt command"),
    }
}

#[test]
fn parse_cli_supports_raw_to_prompt_multiple_input_files() {
    let args = parse_cli_from([
        "grafana-util",
        "raw-to-prompt",
        "--input-file",
        "./dashboards/raw/cpu-main.json",
        "--input-file",
        "./dashboards/raw/network-main.json",
        "--output-file",
        "./dashboards/prompt/cpu-main.prompt.json",
    ]);

    match args.command {
        DashboardCommand::RawToPrompt(raw_args) => {
            assert_eq!(
                raw_args.input_file,
                vec![
                    PathBuf::from("./dashboards/raw/cpu-main.json"),
                    PathBuf::from("./dashboards/raw/network-main.json"),
                ]
            );
            assert_eq!(
                raw_args.output_file,
                Some(PathBuf::from("./dashboards/prompt/cpu-main.prompt.json"))
            );
        }
        _ => panic!("expected raw-to-prompt command"),
    }
}

#[test]
fn parse_cli_rejects_raw_to_prompt_input_file_with_input_dir() {
    let error = DashboardCliArgs::try_parse_from([
        "grafana-util",
        "raw-to-prompt",
        "--input-file",
        "./dashboards/raw/cpu-main.json",
        "--input-dir",
        "./dashboards/raw",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--input-file"));
    assert!(error.to_string().contains("--input-dir"));
}

#[test]
fn parse_cli_rejects_raw_to_prompt_output_file_with_output_dir() {
    let error = DashboardCliArgs::try_parse_from([
        "grafana-util",
        "raw-to-prompt",
        "--input-file",
        "./dashboards/raw/cpu-main.json",
        "--output-file",
        "./dashboards/prompt/cpu-main.prompt.json",
        "--output-dir",
        "./dashboards/prompt",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--output-file"));
    assert!(error.to_string().contains("--output-dir"));
}

#[test]
fn parse_cli_supports_export_provisioning_provider_customization() {
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
    assert!(help.contains("Prefer Basic auth when you need cross-org export"));
    assert!(help.contains("Export dashboards across all visible orgs with Basic auth"));
    assert!(help.contains("Write dashboard files directly into each export variant directory"));
    assert!(help.contains("folder-based subdirectories on disk"));
}

#[test]
fn export_help_mentions_history_artifacts() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("history/"));
    assert!(help.contains("revision history"));
    assert!(help.contains("per-dashboard revision history"));
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
fn export_help_mentions_provisioning_provider_customization() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("provisioning/provisioning/dashboards.yaml"));
    assert!(help.contains("--provisioning-provider-name"));
    assert!(help.contains("--provisioning-provider-org-id"));
    assert!(help.contains("--provisioning-provider-path"));
    assert!(help.contains("--provisioning-provider-disable-deletion"));
    assert!(help.contains("--provisioning-provider-allow-ui-updates"));
    assert!(help.contains("--provisioning-provider-update-interval-seconds"));
    assert!(help.contains("current export tree path"));
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
        "--input-dir",
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
fn parse_cli_supports_import_interactive_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./dashboards/raw",
        "--interactive",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.interactive);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_provisioning_import_format() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
        "./dashboards/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert_eq!(
                import_args.input_format,
                DashboardImportInputFormat::Provisioning
            );
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn import_help_mentions_interactive_review_picker() {
    use clap::CommandFactory;

    let mut help = Vec::new();
    let mut command = DashboardCliArgs::command();
    command
        .find_subcommand_mut("import")
        .unwrap()
        .write_long_help(&mut help)
        .unwrap();
    let rendered = String::from_utf8(help).unwrap();

    assert!(rendered.contains("interactive review picker"));
    assert!(rendered.contains("create/update/skip action"));
    assert!(rendered.contains("With --dry-run"));
    assert!(rendered.contains("--input-format"));
    assert!(rendered.contains("raw export files or Grafana file-provisioning artifacts"));
}

#[test]
fn parse_cli_supports_provisioning_diff_input_format() {
    let args = parse_cli_from([
        "grafana-util",
        "diff",
        "--input-dir",
        "./dashboards/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DashboardCommand::Diff(diff_args) => {
            assert_eq!(
                diff_args.input_format,
                DashboardImportInputFormat::Provisioning
            );
            assert_eq!(
                diff_args.input_dir,
                std::path::PathBuf::from("./dashboards/provisioning")
            );
        }
        _ => panic!("expected diff command"),
    }
}

#[test]
fn parse_cli_supports_patch_file_command() {
    let args = parse_cli_from([
        "grafana-util",
        "patch-file",
        "--input",
        "./drafts/cpu-main.json",
        "--output",
        "./drafts/cpu-main-patched.json",
        "--name",
        "CPU Overview",
        "--uid",
        "cpu-main",
        "--folder-uid",
        "infra",
        "--message",
        "Promote CPU dashboard",
        "--tag",
        "prod",
        "--tag",
        "sre",
    ]);

    match args.command {
        DashboardCommand::PatchFile(patch_args) => {
            assert_eq!(patch_args.input, PathBuf::from("./drafts/cpu-main.json"));
            assert_eq!(
                patch_args.output,
                Some(PathBuf::from("./drafts/cpu-main-patched.json"))
            );
            assert_eq!(patch_args.name.as_deref(), Some("CPU Overview"));
            assert_eq!(patch_args.uid.as_deref(), Some("cpu-main"));
            assert_eq!(patch_args.folder_uid.as_deref(), Some("infra"));
            assert_eq!(patch_args.message.as_deref(), Some("Promote CPU dashboard"));
            assert_eq!(patch_args.tags, vec!["prod", "sre"]);
        }
        _ => panic!("expected patch-file command"),
    }
}

#[test]
fn parse_cli_supports_patch_file_stdin_input() {
    let args = parse_cli_from([
        "grafana-util",
        "patch-file",
        "--input",
        "-",
        "--output",
        "./drafts/cpu-main.json",
    ]);

    match args.command {
        DashboardCommand::PatchFile(patch_args) => {
            assert_eq!(patch_args.input, PathBuf::from("-"));
            assert_eq!(
                patch_args.output,
                Some(PathBuf::from("./drafts/cpu-main.json"))
            );
        }
        _ => panic!("expected patch-file command"),
    }
}

#[test]
fn parse_cli_supports_publish_command() {
    let args = parse_cli_from([
        "grafana-util",
        "publish",
        "--url",
        "https://grafana.example.com",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--input",
        "./drafts/cpu-main.json",
        "--folder-uid",
        "infra",
        "--message",
        "Promote CPU dashboard",
        "--dry-run",
        "--table",
    ]);

    match args.command {
        DashboardCommand::Publish(publish_args) => {
            assert_eq!(publish_args.common.url, "https://grafana.example.com");
            assert_eq!(publish_args.common.username.as_deref(), Some("admin"));
            assert_eq!(publish_args.common.password.as_deref(), Some("admin"));
            assert_eq!(publish_args.input, PathBuf::from("./drafts/cpu-main.json"));
            assert_eq!(publish_args.folder_uid.as_deref(), Some("infra"));
            assert_eq!(publish_args.message, "Promote CPU dashboard");
            assert!(publish_args.dry_run);
            assert!(!publish_args.watch);
            assert!(publish_args.table);
            assert!(!publish_args.json);
        }
        _ => panic!("expected publish command"),
    }
}

#[test]
fn parse_cli_supports_publish_watch_and_stdin_input() {
    let watch_args = parse_cli_from([
        "grafana-util",
        "publish",
        "--url",
        "https://grafana.example.com",
        "--input",
        "./drafts/cpu-main.json",
        "--watch",
    ]);
    let stdin_args = parse_cli_from([
        "grafana-util",
        "publish",
        "--url",
        "https://grafana.example.com",
        "--input",
        "-",
    ]);

    match watch_args.command {
        DashboardCommand::Publish(publish_args) => {
            assert_eq!(publish_args.input, PathBuf::from("./drafts/cpu-main.json"));
            assert!(publish_args.watch);
        }
        _ => panic!("expected publish command"),
    }

    match stdin_args.command {
        DashboardCommand::Publish(publish_args) => {
            assert_eq!(publish_args.input, PathBuf::from("-"));
            assert!(!publish_args.watch);
        }
        _ => panic!("expected publish command"),
    }
}

#[test]
fn diff_help_mentions_provisioning_input_format() {
    let help = render_dashboard_subcommand_help("diff");
    assert!(help.contains("--input-format"));
    assert!(help.contains("Grafana file-provisioning artifacts"));
    assert!(help.contains("provisioning/ root or its dashboards/ subdirectory"));
    assert!(help.contains("Compare a provisioning export root against the current org"));
}

#[test]
fn patch_file_help_mentions_in_place_and_output_paths() {
    let help = render_dashboard_subcommand_help("patch-file");
    assert!(help.contains("--input"));
    assert!(help.contains("--output"));
    assert!(help.contains("--name"));
    assert!(help.contains("--uid"));
    assert!(help.contains("--folder-uid"));
    assert!(help.contains("--message"));
    assert!(help.contains("--tag"));
    assert!(help.contains("Patch a raw export file in place"));
    assert!(help.contains("Patch one draft file into a new output path"));
    assert!(help.contains("Patch one dashboard from standard input into an explicit output file"));
}

#[test]
fn raw_to_prompt_help_mentions_defaults_and_output_controls() {
    let help = render_dashboard_subcommand_help("raw-to-prompt");
    assert!(help.contains("--input-file"));
    assert!(help.contains("--input-dir"));
    assert!(help.contains("--output-file"));
    assert!(help.contains("--output-dir"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("--no-header"));
    assert!(help.contains("--color"));
    assert!(help.contains("--progress"));
    assert!(help.contains("--verbose"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("--log-file"));
    assert!(help.contains("--log-format"));
    assert!(help.contains("--resolution"));
    assert!(help.contains("--datasource-map"));
    assert!(help.contains("sibling .prompt.json"));
    assert!(help.contains("prompt/ lane"));
    assert!(help.contains("Convert raw dashboard exports into prompt lane artifacts."));
    assert!(help.contains("Convert one raw export root into a sibling prompt/ lane"));
}

#[test]
fn parse_cli_supports_review_command() {
    let args = parse_cli_from([
        "grafana-util",
        "review",
        "--input",
        "./drafts/cpu-main.json",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        DashboardCommand::Review(review_args) => {
            assert_eq!(review_args.input, PathBuf::from("./drafts/cpu-main.json"));
            assert_eq!(review_args.output_format, Some(SimpleOutputFormat::Yaml));
        }
        _ => panic!("expected review command"),
    }
}

#[test]
fn parse_cli_supports_review_output_format_variants() {
    for (flag, expected) in [
        ("text", SimpleOutputFormat::Text),
        ("table", SimpleOutputFormat::Table),
        ("csv", SimpleOutputFormat::Csv),
        ("json", SimpleOutputFormat::Json),
        ("yaml", SimpleOutputFormat::Yaml),
    ] {
        let args = parse_cli_from([
            "grafana-util",
            "review",
            "--input",
            "./drafts/cpu-main.json",
            "--output-format",
            flag,
        ]);

        match args.command {
            DashboardCommand::Review(review_args) => {
                assert_eq!(review_args.output_format, Some(expected));
            }
            _ => panic!("expected review command"),
        }
    }
}

#[test]
fn review_help_mentions_local_file_only_output_modes() {
    let help = render_dashboard_subcommand_help("review");
    assert!(help.contains("--input"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("text"));
    assert!(help.contains("table"));
    assert!(help.contains("csv"));
    assert!(help.contains("json"));
    assert!(help.contains("yaml"));
    assert!(help.contains("Review one local dashboard JSON file without touching Grafana."));
    assert!(help.contains("grafana-util dashboard review"));
    assert!(help.contains("standard input"));
    assert!(help.contains("Review one generated dashboard from standard input"));
}

#[test]
fn publish_help_mentions_dry_run_preview() {
    let help = render_dashboard_subcommand_help("publish");
    assert!(help.contains("--input"));
    assert!(help.contains("--folder-uid"));
    assert!(help.contains("--message"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("--watch"));
    assert!(help.contains("--table"));
    assert!(help.contains("Publish one draft file to the current Grafana org"));
    assert!(help.contains("Preview the same publish without writing to Grafana"));
    assert!(help.contains("Publish one generated dashboard from standard input"));
    assert!(help.contains("Watch one local draft file and rerun dry-run after each save"));
}

#[test]
fn parse_cli_supports_dashboard_serve_command() {
    let args = parse_cli_from([
        "grafana-util",
        "serve",
        "--script",
        "jsonnet dashboards/cpu.jsonnet",
        "--watch",
        "./dashboards",
        "--port",
        "18080",
        "--open-browser",
    ]);

    match args.command {
        DashboardCommand::Serve(serve_args) => {
            assert_eq!(
                serve_args.script.as_deref(),
                Some("jsonnet dashboards/cpu.jsonnet")
            );
            assert_eq!(serve_args.port, 18080);
            assert_eq!(serve_args.watch, vec![PathBuf::from("./dashboards")]);
            assert!(serve_args.input.is_none());
            assert!(serve_args.open_browser);
        }
        _ => panic!("expected serve command"),
    }
}

#[test]
fn serve_help_mentions_local_preview_server() {
    let help = render_dashboard_subcommand_help("serve");
    assert!(help.contains("--input"));
    assert!(help.contains("--script"));
    assert!(help.contains("--watch"));
    assert!(help.contains("--open-browser"));
    assert!(help.contains("--port"));
    assert!(help.contains("local preview server"));
}

#[test]
fn parse_cli_supports_dashboard_edit_live_command() {
    let args = parse_cli_from([
        "grafana-util",
        "edit-live",
        "--url",
        "https://grafana.example.com",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./drafts/cpu-main.edited.json",
    ]);

    match args.command {
        DashboardCommand::EditLive(edit_args) => {
            assert_eq!(edit_args.common.url, "https://grafana.example.com");
            assert_eq!(edit_args.dashboard_uid, "cpu-main");
            assert_eq!(
                edit_args.output,
                Some(PathBuf::from("./drafts/cpu-main.edited.json"))
            );
            assert!(!edit_args.apply_live);
        }
        _ => panic!("expected edit-live command"),
    }
}

#[test]
fn edit_live_help_mentions_safe_local_draft_default() {
    let help = render_dashboard_subcommand_help("edit-live");
    assert!(help.contains("--dashboard-uid"));
    assert!(help.contains("--output"));
    assert!(help.contains("--apply-live"));
    assert!(help.contains("local draft"));
    assert!(help.contains("review output"));
}

#[test]
fn parse_cli_supports_import_dry_run_json_flag() {
    let args = parse_cli_from([
        "grafana-util",
        "import",
        "--input-dir",
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
        "--input-dir",
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
            assert!(!browse_args.all_orgs);
            assert_eq!(browse_args.path.as_deref(), Some("Platform / Infra"));
            assert_eq!(browse_args.page_size, 500);
        }
        _ => panic!("expected browse command"),
    }
}

#[test]
fn parse_cli_supports_browse_local_export_tree() {
    let args = parse_cli_from([
        "grafana-util",
        "browse",
        "--input-dir",
        "./dashboards/raw",
        "--input-format",
        "raw",
        "--path",
        "Platform / Infra",
    ]);

    match args.command {
        DashboardCommand::Browse(browse_args) => {
            assert_eq!(
                browse_args.input_dir,
                Some(PathBuf::from("./dashboards/raw"))
            );
            assert_eq!(browse_args.input_format, DashboardImportInputFormat::Raw);
            assert_eq!(browse_args.path.as_deref(), Some("Platform / Infra"));
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
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("Open the browser at one folder subtree"));
    assert!(help.contains("Browse all visible orgs with Basic auth"));
    assert!(help.contains("local export tree"));
}

#[test]
fn browse_help_mentions_local_export_tree_examples() {
    let help = render_dashboard_subcommand_help("browse");
    assert!(help.contains("--input-dir ./dashboards/raw"));
    assert!(help.contains("Browse a raw export tree from disk"));
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
        "fetch-live",
        "--profile",
        "prod",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.json",
    ]);
    let clone_args = parse_cli_from([
        "grafana-util",
        "clone-live",
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
        _ => panic!("expected fetch-live command"),
    }

    match clone_args.command {
        DashboardCommand::CloneLive(args) => {
            assert_eq!(args.source_uid, "cpu-main");
            assert_eq!(args.name.as_deref(), Some("CPU Clone"));
            assert_eq!(args.uid.as_deref(), Some("cpu-main-clone"));
            assert_eq!(args.folder_uid.as_deref(), Some("infra"));
            assert_eq!(args.output, PathBuf::from("./cpu-main-clone.json"));
        }
        _ => panic!("expected clone-live command"),
    }
}

#[test]
fn dashboard_fetch_live_help_mentions_local_draft_and_output_path() {
    let help = render_dashboard_subcommand_help("fetch-live");
    assert!(help.contains("API-safe local JSON draft"));
    assert!(help.contains("What it does:"));
    assert!(help.contains("When to use:"));
    assert!(help.contains("--dashboard-uid"));
    assert!(help.contains("--output"));
    assert!(help.contains("grafana-util dashboard fetch-live"));
}

#[test]
fn dashboard_clone_live_help_mentions_override_flags() {
    let help = render_dashboard_subcommand_help("clone-live");
    assert!(help.contains("optional overrides"));
    assert!(help.contains("What it does:"));
    assert!(help.contains("Related commands:"));
    assert!(help.contains("--source-uid"));
    assert!(help.contains("--name"));
    assert!(help.contains("--uid"));
    assert!(help.contains("--folder-uid"));
    assert!(help.contains("grafana-util dashboard clone-live"));
}

#[test]
fn dashboard_fetch_live_help_colorizes_section_headings_and_example_commands() {
    let help = crate::dashboard::maybe_render_dashboard_subcommand_help_from_os_args(
        ["grafana-util", "dashboard", "fetch-live", "--help"],
        true,
    )
    .expect("expected dashboard subcommand help");
    assert!(help.contains("\u{1b}[1;36mWhat it does:\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;36mExamples:\u{1b}[0m"));
    assert!(help.contains("    \u{1b}[1;97mgrafana-util dashboard fetch-live"));
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
