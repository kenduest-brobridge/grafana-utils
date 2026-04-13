use crate::alert::AlertGroupCommand;
use crate::cli::{
    parse_cli_from, CliArgs, ConfigCommand, DashboardConvertCommand, DashboardRootCommand,
    ExportAccessCommand, ExportCommand, StatusCommand, UnifiedCommand,
};
use crate::cli_completion::CompletionShell;
use crate::dashboard::{
    parse_cli_from as parse_dashboard_cli_from, DashboardCliArgs, DashboardCommand,
    SimpleOutputFormat,
};
use crate::profile_cli::ProfileCommand;
use crate::resource::{ResourceCommand, ResourceKind, ResourceOutputFormat};
use crate::sync::SyncGroupCommand;
use clap::Parser;
use std::path::Path;

#[test]
fn ambiguous_root_subcommand_prefix_stays_on_clap_error_path() {
    let args = ["grafana-util", "da", "--help"];
    assert!(super::maybe_render_unified_help_from_os_args(args, false).is_none());
    assert!(
        crate::dashboard::maybe_render_dashboard_subcommand_help_from_os_args(args, false)
            .is_none()
    );

    let error = CliArgs::try_parse_from(args).expect_err("da should be ambiguous");
    let rendered = error.to_string();
    assert!(rendered.contains("da"));
    assert!(rendered.contains("dashboard"));
    assert!(rendered.contains("datasource"));
}

#[test]
fn inferred_root_subcommand_dispatches_through_clap_parser() {
    let args = parse_cli_from(["grafana-util", "dashb", "list"]);
    match args.command {
        UnifiedCommand::Dashboard {
            command: DashboardRootCommand::List(_),
        } => {}
        other => panic!("expected dashboard list command, got {other:?}"),
    }
}

#[test]
fn inferred_unique_long_option_dispatches_through_unified_parser() {
    let args = parse_cli_from(["grafana-util", "access", "user", "list", "--all-o", "--tab"]);
    match args.command {
        UnifiedCommand::Access(access_args) => match access_args.command {
            crate::access::AccessCommand::User {
                command: crate::access::UserCommand::List(list_args),
            } => {
                assert!(list_args.all_orgs);
                assert!(list_args.table);
            }
            other => panic!("expected access user list command, got {other:?}"),
        },
        other => panic!("expected access command, got {other:?}"),
    }
}

#[test]
fn ambiguous_long_option_prefix_stays_on_clap_error_path() {
    let error =
        CliArgs::try_parse_from(["grafana-util", "access", "user", "list", "--output", "json"])
            .expect_err("--output should be ambiguous for access user list");
    let rendered = error.to_string();
    assert!(rendered.contains("--output"));
    assert!(
        rendered.contains("--output-format")
            || rendered.contains("--output-columns")
            || rendered.contains("possible values")
    );
}

#[test]
fn parse_cli_supports_completion_surface() {
    let bash_args: CliArgs = parse_cli_from(["grafana-util", "completion", "bash"]);
    let zsh_args: CliArgs = parse_cli_from(["grafana-util", "completion", "zsh"]);

    match bash_args.command {
        UnifiedCommand::Completion(args) => assert_eq!(args.shell, CompletionShell::Bash),
        other => panic!("expected completion bash command, got {other:?}"),
    }
    match zsh_args.command {
        UnifiedCommand::Completion(args) => assert_eq!(args.shell, CompletionShell::Zsh),
        other => panic!("expected completion zsh command, got {other:?}"),
    }
}

#[test]
fn parse_cli_rejects_unsupported_completion_shells() {
    let error = CliArgs::try_parse_from(["grafana-util", "completion", "fish"])
        .expect_err("fish completion should not be supported");

    assert!(error.to_string().contains("invalid value"));
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
            StatusCommand::Live(inner) => assert!(inner.all_orgs),
            _ => panic!("expected status live"),
        },
        _ => panic!("expected status command"),
    }

    match overview_args.command {
        UnifiedCommand::Status { command } => match command {
            StatusCommand::Overview { staged, command } => {
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
            StatusCommand::Resource { command } => match command {
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
fn parse_cli_rejects_removed_legacy_roots_without_compatibility() {
    for args in [
        vec!["grafana-util", "observe", "live"],
        vec!["grafana-util", "change", "inspect"],
        vec!["grafana-util", "overview", "live"],
        vec!["grafana-util", "advanced", "dashboard", "browse"],
        vec!["grafana-util", "dashboard", "live", "list"],
        vec!["grafana-util", "alert", "live", "list-rules"],
        vec!["grafana-util", "alert", "migrate", "export"],
        vec!["grafana-util", "alert", "author", "rule", "add"],
        vec!["grafana-util", "alert", "scaffold", "template"],
        vec!["grafana-util", "alert", "change", "apply"],
    ] {
        let error = CliArgs::try_parse_from(args.clone()).unwrap_err();
        assert!(
            error.to_string().contains("unrecognized subcommand"),
            "expected a normal Clap rejection for {}",
            args.join(" ")
        );
    }
}

#[test]
fn parse_cli_supports_workspace_surface() {
    let args: CliArgs =
        parse_cli_from(["grafana-util", "workspace", "preview", "./grafana-oac-repo"]);

    match args.command {
        UnifiedCommand::Workspace { command } => match command {
            SyncGroupCommand::Preview(inner) => {
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
            ConfigCommand::Profile(profile_args) => match profile_args.command {
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
            ExportCommand::Dashboard(inner) => {
                assert_eq!(inner.output_dir, Path::new("./dashboards"));
                assert!(inner.overwrite);
            }
            _ => panic!("expected export dashboard"),
        },
        _ => panic!("expected export command"),
    }

    match access_args.command {
        UnifiedCommand::Export { command } => match command {
            ExportCommand::Access { command } => match command {
                ExportAccessCommand::ServiceAccount(inner) => {
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
            DashboardRootCommand::Convert { command } => match command {
                DashboardConvertCommand::RawToPrompt(inner) => {
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
