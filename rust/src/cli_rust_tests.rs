use super::{
    dispatch_with_handlers, maybe_render_unified_help_from_os_args, parse_cli_from,
    render_unified_help_text, CliArgs, UnifiedCommand,
};
use crate::alert::AlertGroupCommand;
use crate::cli_help_examples::{paint_section, paint_support};
use crate::dashboard::SimpleOutputFormat;
use crate::dashboard::{
    parse_cli_from as parse_dashboard_cli_from, DashboardCliArgs, DashboardCommand,
};
use crate::help_styles::CLI_HELP_STYLES;
use crate::profile_cli::ProfileCommand;
use crate::resource::{ResourceCommand, ResourceKind, ResourceOutputFormat};
use clap::builder::styling::AnsiColor;
use clap::{CommandFactory, Parser};
use std::cell::RefCell;
use std::path::Path;

fn render_cli_help_path(path: &[&str]) -> String {
    let mut command = CliArgs::command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing cli subcommand {segment}"));
    }
    current.render_help().to_string()
}

#[test]
fn unified_help_mentions_common_surfaces_without_legacy_dashboard_paths() {
    let help = render_unified_help_text(false);
    assert!(help.contains("config profile add"));
    assert!(help.contains("status overview"));
    assert!(help.contains("export dashboard"));
    assert!(help.contains("export alert"));
    assert!(help.contains("workspace preview"));
    assert!(help.contains("dashboard export"));
    assert!(help.contains("dashboard summary"));
    assert!(!help.contains("advanced dashboard"));
    assert!(!help.contains("observe"));
    assert!(!help.contains("change"));
    assert!(!help.contains("dashboard live"));
    assert!(!help.contains("dashboard draft"));
    assert!(!help.contains("dashboard sync"));
    assert!(!help.contains("dashboard analyze"));
    assert!(!help.contains("dashboard capture"));
    assert!(!help.contains("alert migrate export"));
}

#[test]
fn dashboard_help_uses_flat_paths_and_short_help() {
    let help =
        maybe_render_unified_help_from_os_args(["grafana-util", "dashboard", "--help"], false)
            .unwrap();
    assert!(help.contains("browse"));
    assert!(help.contains("variables"));
    assert!(help.contains("get"));
    assert!(help.contains("clone"));
    assert!(help.contains("edit-live"));
    assert!(help.contains("export"));
    assert!(help.contains("import"));
    assert!(help.contains("review"));
    assert!(help.contains("patch"));
    assert!(help.contains("serve"));
    assert!(help.contains("publish"));
    assert!(help.contains("summary"));
    assert!(help.contains("dependencies"));
    assert!(help.contains("impact"));
    assert!(help.contains("policy"));
    assert!(help.contains("screenshot"));
    assert!(!help.contains("live         browse, list, vars, fetch, clone, edit, delete, history"));
    assert!(!help.contains("draft        review, patch, serve, publish"));
    assert!(!help.contains("sync         export, import, diff, convert"));
    assert!(!help.contains("analyze      summary, topology, impact, governance"));
    assert!(!help.contains("capture      screenshot"));
    assert!(!help.contains("advanced dashboard live"));
    assert!(!help.contains("migrate dashboard"));
}

#[test]
fn export_dashboard_help_shows_examples_and_grouped_headings() {
    let help = render_cli_help_path(&["export", "dashboard"]);
    assert!(help.contains("Connection Options"));
    assert!(help.contains("Selection Options"));
    assert!(help.contains("Export Variant Options"));
    assert!(help.contains("Notes:"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Preview the files and indexes without writing to disk."));
    assert!(help.contains("grafana-util export dashboard"));
    assert!(help.contains("--without-raw"));
    assert!(help.contains("--provider-name <NAME>"));
    assert!(help.contains("--provider-update-interval-seconds <SECONDS>"));
    assert!(!help.contains("--without-dashboard-raw"));
    assert!(!help.contains("--provisioning-provider-name"));
}

#[test]
fn export_dashboard_help_colorizes_notes_and_example_commands() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "export", "dashboard", "--help"],
        true,
    )
    .expect("expected export dashboard help");
    assert!(help.contains(&paint_section("Notes:")));
    assert!(help.contains(&paint_section("Examples:")));
    assert!(help.contains(&paint_support("Export dashboards from the current org:")));
    assert!(help.contains("grafana-util export dashboard"));
}

#[test]
fn export_dashboard_help_ends_with_blank_line() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "export", "dashboard", "--help"],
        false,
    )
    .expect("expected export dashboard help");
    assert!(help.ends_with("\n\n"));
}

#[test]
fn export_dashboard_help_colorizes_default_context_bright_green() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "export", "dashboard", "--help"],
        true,
    )
    .expect("expected export dashboard help");
    assert!(help.contains("default:"));
    assert!(help.contains("500"));
    assert!(help.contains("grafana-utils-dashboards"));
    assert!(help.contains("possible values:"));
}

#[test]
fn cli_help_styles_use_bright_green_bold_context() {
    let rendered = format!("{}", CLI_HELP_STYLES.get_context());
    let expected = format!("{}", AnsiColor::BrightGreen.on_default().bold());
    assert_eq!(rendered, expected);
}

#[test]
fn export_dashboard_help_colorizes_option_descriptions_as_secondary_text() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "export", "dashboard", "--help"],
        true,
    )
    .expect("expected export dashboard help");
    assert!(help.contains(&format!(
        "          {}",
        paint_support("Set the generated provisioning provider name.")
    )));
}

#[test]
fn status_overview_help_uses_canonical_examples() {
    assert!(crate::overview::OVERVIEW_HELP_TEXT
        .contains("grafana-util status overview --dashboard-export-dir ./dashboards/raw"));
    assert!(crate::overview::OVERVIEW_LIVE_HELP_TEXT
        .contains("grafana-util status overview live --url http://localhost:3000 --token"));
    let help = render_cli_help_path(&["status", "overview"]);
    assert!(!help.contains("grafana-util observe overview"));
}

#[test]
fn config_profile_help_uses_canonical_examples() {
    let help = render_cli_help_path(&["config", "profile"]);
    assert!(help.contains("grafana-util config profile list"));
    assert!(help.contains("grafana-util config profile show --profile prod --output-format yaml"));
    assert!(help.contains("grafana-util config profile validate --profile prod"));
    assert!(help.contains("grafana-util config profile add prod --url https://grafana.example.com"));
    assert!(help.contains("grafana-util config profile init --overwrite"));
    let current_help = render_cli_help_path(&["config", "profile", "current"]);
    assert!(current_help.contains("status live, status overview"));
    assert!(!help.contains("grafana-util profile"));
}

#[test]
fn dashboard_convert_help_shows_flat_subcommand() {
    let help = render_cli_help_path(&["dashboard", "convert"]);
    assert!(help.contains("Run dashboard format conversion workflows."));
    assert!(help.contains("raw-to-prompt"));
    assert!(!help.contains("advanced dashboard"));
    assert!(!help.contains("sync"));
}

#[test]
fn dashboard_export_help_uses_short_export_flags() {
    let help = render_cli_help_path(&["dashboard", "export"]);
    assert!(help.contains("--without-raw"));
    assert!(help.contains("--provider-name <NAME>"));
    assert!(help.contains("--provider-update-interval-seconds <SECONDS>"));
    assert!(!help.contains("--without-dashboard-raw"));
    assert!(!help.contains("--provisioning-provider-name"));
}

#[test]
fn dashboard_short_help_uses_flat_paths_only() {
    let help =
        maybe_render_unified_help_from_os_args(["grafana-util", "dashboard", "--help"], false)
            .unwrap();
    assert!(help.contains("browse"));
    assert!(help.contains("list"));
    assert!(help.contains("variables"));
    assert!(help.contains("get"));
    assert!(help.contains("clone"));
    assert!(help.contains("edit-live"));
    assert!(help.contains("delete"));
    assert!(help.contains("history"));
    assert!(help.contains("review"));
    assert!(help.contains("patch"));
    assert!(help.contains("serve"));
    assert!(help.contains("publish"));
    assert!(help.contains("export"));
    assert!(help.contains("import"));
    assert!(help.contains("diff"));
    assert!(help.contains("summary"));
    assert!(help.contains("dependencies"));
    assert!(help.contains("impact"));
    assert!(help.contains("policy"));
    assert!(help.contains("screenshot"));
    assert!(!help.contains("live         browse, list, vars, fetch, clone, edit, delete, history"));
    assert!(!help.contains("draft        review, patch, serve, publish"));
    assert!(!help.contains("sync         export, import, diff, convert"));
    assert!(!help.contains("analyze      summary, topology, impact, governance"));
    assert!(!help.contains("capture      screenshot"));
}

#[test]
fn alert_short_help_uses_flat_task_groups_only() {
    let help =
        maybe_render_unified_help_from_os_args(["grafana-util", "alert", "-h"], false).unwrap();
    assert!(help.contains("inventory"));
    assert!(help.contains("backup"));
    assert!(help.contains("authoring"));
    assert!(help.contains("review"));
    assert!(help.contains("list-rules"));
    assert!(help.contains("export"));
    assert!(help.contains("add-rule"));
    assert!(help.contains("plan"));
    assert!(!help.contains("live         list-rules"));
    assert!(!help.contains("migrate      export, import, diff"));
    assert!(!help.contains("author       init, rule add|clone"));
    assert!(!help.contains("scaffold     rule, contact-point, template"));
    assert!(!help.contains("change       plan, apply"));
}

#[test]
fn parse_cli_supports_status_surface() {
    let live_args: CliArgs = parse_cli_from(["grafana-util", "status", "live", "--all-orgs"]);
    let overview_args: CliArgs = parse_cli_from([
        "grafana-util",
        "status",
        "overview",
        "--dashboard-export-dir",
        "./dashboards/raw",
    ]);
    let resource_args: CliArgs = parse_cli_from([
        "grafana-util",
        "status",
        "resource",
        "describe",
        "dashboards",
        "--output-format",
        "json",
    ]);

    match live_args.command {
        UnifiedCommand::Status { command } => match command {
            super::StatusCommand::Live(inner) => assert!(inner.all_orgs),
            _ => panic!("expected status live"),
        },
        _ => panic!("expected status command"),
    }

    match overview_args.command {
        UnifiedCommand::Status { command } => match command {
            super::StatusCommand::Overview { staged, command } => {
                assert!(command.is_none());
                assert_eq!(
                    staged.dashboard_export_dir.as_deref(),
                    Some(Path::new("./dashboards/raw"))
                );
            }
            _ => panic!("expected status overview"),
        },
        _ => panic!("expected status command"),
    }

    match resource_args.command {
        UnifiedCommand::Status { command } => match command {
            super::StatusCommand::Resource { command } => match command {
                ResourceCommand::Describe(inner) => {
                    assert_eq!(inner.kind, Some(ResourceKind::Dashboards));
                    assert_eq!(inner.output_format, ResourceOutputFormat::Json);
                }
                _ => panic!("expected status resource describe"),
            },
            _ => panic!("expected status resource"),
        },
        _ => panic!("expected status command"),
    }
}

#[test]
fn parse_cli_rejects_legacy_status_roots() {
    for args in [
        vec!["grafana-util", "observe", "live"],
        vec!["grafana-util", "change", "inspect"],
    ] {
        let _error = CliArgs::try_parse_from(args).unwrap_err();
    }
}

#[test]
fn parse_cli_supports_workspace_surface() {
    let args: CliArgs =
        parse_cli_from(["grafana-util", "workspace", "preview", "./grafana-oac-repo"]);

    match args.command {
        UnifiedCommand::Workspace { command } => match command {
            super::SyncGroupCommand::Preview(inner) => {
                assert_eq!(inner.inputs.workspace, Path::new("./grafana-oac-repo"));
            }
            _ => panic!("expected workspace preview"),
        },
        _ => panic!("expected workspace command"),
    }
}

#[test]
fn parse_cli_supports_config_profile_surface() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "config",
        "profile",
        "show",
        "--profile",
        "prod",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        UnifiedCommand::Config { command } => match command {
            super::ConfigCommand::Profile(profile_args) => match profile_args.command {
                ProfileCommand::Show(show_args) => {
                    assert_eq!(show_args.profile.as_deref(), Some("prod"));
                    assert_eq!(show_args.output_format, SimpleOutputFormat::Yaml);
                }
                _ => panic!("expected config profile show"),
            },
        },
        _ => panic!("expected config command"),
    }
}

#[test]
fn parse_cli_supports_task_first_export_surface() {
    let dashboard_args: CliArgs = parse_cli_from([
        "grafana-util",
        "export",
        "dashboard",
        "--output-dir",
        "./dashboards",
        "--overwrite",
    ]);
    let access_args: CliArgs = parse_cli_from([
        "grafana-util",
        "export",
        "access",
        "service-account",
        "--output-dir",
        "./access-service-accounts",
        "--overwrite",
    ]);

    match dashboard_args.command {
        UnifiedCommand::Export { command } => match command {
            super::ExportCommand::Dashboard(inner) => {
                assert_eq!(inner.output_dir, Path::new("./dashboards"));
                assert!(inner.overwrite);
            }
            _ => panic!("expected export dashboard"),
        },
        _ => panic!("expected export command"),
    }

    match access_args.command {
        UnifiedCommand::Export { command } => match command {
            super::ExportCommand::Access { command } => match command {
                super::ExportAccessCommand::ServiceAccount(inner) => {
                    assert_eq!(inner.output_dir, Path::new("./access-service-accounts"));
                    assert!(inner.overwrite);
                }
                _ => panic!("expected export access service-account"),
            },
            _ => panic!("expected export access"),
        },
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_dashboard_cli_supports_flat_dashboard_surface() {
    let browse_args = parse_dashboard_cli_from([
        "grafana-util",
        "browse",
        "--url",
        "https://grafana.example.com",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
    ]);
    let review_args = parse_dashboard_cli_from([
        "grafana-util",
        "review",
        "--input",
        "./drafts/cpu-main.json",
    ]);
    let analyze_args =
        parse_dashboard_cli_from(["grafana-util", "summary", "--input-dir", "./dashboards/raw"]);
    let screenshot_args = parse_dashboard_cli_from([
        "grafana-util",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
    ]);

    match browse_args.command {
        DashboardCommand::Browse(inner) => {
            assert_eq!(inner.common.url, "https://grafana.example.com");
        }
        _ => panic!("expected dashboard browse"),
    }

    match review_args.command {
        DashboardCommand::Review(inner) => {
            assert_eq!(inner.input, Path::new("./drafts/cpu-main.json"));
        }
        _ => panic!("expected dashboard review"),
    }

    match analyze_args.command {
        DashboardCommand::Analyze(inner) => {
            assert_eq!(
                inner.input_dir.as_deref(),
                Some(Path::new("./dashboards/raw"))
            );
        }
        _ => panic!("expected dashboard summary"),
    }

    match screenshot_args.command {
        DashboardCommand::Screenshot(inner) => {
            assert_eq!(inner.dashboard_uid.as_deref(), Some("cpu-main"));
            assert_eq!(inner.output, Path::new("./cpu-main.png"));
        }
        _ => panic!("expected dashboard screenshot"),
    }
}

#[test]
fn parse_dashboard_cli_rejects_legacy_grouped_and_compatibility_paths() {
    for args in [
        vec!["grafana-util", "advanced", "dashboard", "live", "browse"],
        vec!["grafana-util", "migrate", "dashboard", "raw-to-prompt"],
        vec!["grafana-util", "live", "browse"],
        vec!["grafana-util", "draft", "review"],
        vec!["grafana-util", "sync", "export"],
        vec!["grafana-util", "analyze", "summary"],
        vec!["grafana-util", "capture", "screenshot"],
    ] {
        let _error = DashboardCliArgs::try_parse_from(args).unwrap_err();
    }
}

#[test]
fn parse_cli_supports_dashboard_convert_surface() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "convert",
        "raw-to-prompt",
        "--input-file",
        "./dashboards/raw/cpu-main.json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardRootCommand::Convert { command } => match command {
                super::DashboardConvertCommand::RawToPrompt(inner) => {
                    assert_eq!(
                        inner.input_file,
                        vec![Path::new("./dashboards/raw/cpu-main.json").to_path_buf()]
                    );
                }
            },
            _ => panic!("expected dashboard convert"),
        },
        _ => panic!("expected dashboard command"),
    }
}

#[test]
fn parse_cli_supports_flat_alert_surface() {
    let export_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "export",
        "--output-dir",
        "./alerts",
        "--overwrite",
    ]);
    let author_rule_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "add-rule",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "cpu-high",
        "--folder",
        "platform-alerts",
        "--rule-group",
        "cpu",
        "--receiver",
        "pagerduty-primary",
        "--expr",
        "A",
        "--threshold",
        "80",
        "--above",
    ]);
    let scaffold_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "new-rule",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "cpu-main",
    ]);
    let change_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "plan",
        "--desired-dir",
        "./alerts/desired",
        "--output-format",
        "json",
    ]);

    match export_args.command {
        UnifiedCommand::Alert { command, .. } => match command {
            AlertGroupCommand::Export(inner) => {
                assert_eq!(inner.output_dir, Path::new("./alerts"));
                assert!(inner.overwrite);
            }
            _ => panic!("expected alert export"),
        },
        _ => panic!("expected alert command"),
    }

    match author_rule_args.command {
        UnifiedCommand::Alert { command, .. } => match command {
            AlertGroupCommand::AddRule(inner) => {
                assert_eq!(inner.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(inner.name, "cpu-high");
                assert_eq!(inner.folder, "platform-alerts");
            }
            _ => panic!("expected alert add-rule"),
        },
        _ => panic!("expected alert command"),
    }

    match scaffold_args.command {
        UnifiedCommand::Alert { command, .. } => match command {
            AlertGroupCommand::NewRule(inner) => {
                assert_eq!(inner.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(inner.name, "cpu-main");
            }
            _ => panic!("expected alert new-rule"),
        },
        _ => panic!("expected alert command"),
    }

    match change_args.command {
        UnifiedCommand::Alert { command, .. } => match command {
            AlertGroupCommand::Plan(inner) => {
                assert_eq!(inner.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(format!("{:?}", inner.output_format), "Json");
            }
            _ => panic!("expected alert plan"),
        },
        _ => panic!("expected alert command"),
    }
}

#[test]
fn docs_describe_dashboard_and_legacy_compatibility_surfaces() {
    let en_index = include_str!("../../docs/commands/en/index.md");
    assert!(en_index.contains("Start Here"));
    assert!(en_index.contains("Common Tasks"));
    assert!(en_index.contains("dashboard convert raw-to-prompt"));
    assert!(!en_index.contains("advanced dashboard"));
    assert!(!en_index.contains("migrate dashboard"));
    assert!(en_index.contains("dashboard"));
    assert!(en_index.contains("status"));
    assert!(en_index.contains("export"));
    assert!(en_index.contains("config profile"));

    let zh_index = include_str!("../../docs/commands/zh-TW/index.md");
    assert!(zh_index.contains("先從這裡開始"));
    assert!(zh_index.contains("常用工作"));
    assert!(zh_index.contains("dashboard convert raw-to-prompt"));
    assert!(!zh_index.contains("advanced dashboard"));
    assert!(!zh_index.contains("migrate dashboard"));
    assert!(zh_index.contains("status"));
    assert!(zh_index.contains("export"));
    assert!(zh_index.contains("config profile"));
}

#[test]
fn dispatch_routes_status_live_to_project_status_handler() {
    let routed = RefCell::new(Vec::<String>::new());
    let args: CliArgs = parse_cli_from(["grafana-util", "status", "live", "--all-orgs"]);

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| Ok(()),
        |_datasource_args| Ok(()),
        |_sync_args| Ok(()),
        |_alert_args| Ok(()),
        |_access_args| Ok(()),
        |_profile_args| Ok(()),
        |_snapshot_args| Ok(()),
        |_overview_args| Ok(()),
        |_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["status".to_string()]);
}

#[test]
fn dispatch_routes_export_dashboard_to_dashboard_handler() {
    let routed = RefCell::new(Vec::<String>::new());
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "export",
        "dashboard",
        "--output-dir",
        "./dashboards",
    ]);

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| Ok(()),
        |_sync_args| Ok(()),
        |_alert_args| Ok(()),
        |_access_args| Ok(()),
        |_profile_args| Ok(()),
        |_snapshot_args| Ok(()),
        |_overview_args| Ok(()),
        |_status_args| Ok(()),
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["dashboard".to_string()]);
}
