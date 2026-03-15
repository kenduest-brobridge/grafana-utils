// Unified CLI test suite.
// Focuses on command routing for aliases/groups and ensures handlers receive the expected domain payload shapes.
use super::{dispatch_with_handlers, parse_cli_from, CliArgs, UnifiedCommand};
use crate::dashboard::DashboardCommand;
use crate::datasource::DatasourceGroupCommand;
use crate::sync::{SyncGroupCommand, SyncOutputFormat, DEFAULT_REVIEW_TOKEN};
use clap::CommandFactory;
use std::cell::RefCell;
use std::path::Path;

fn render_unified_help() -> String {
    let mut command = CliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_unified_subcommand_help(path: &[&str]) -> String {
    let mut command = CliArgs::command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing unified subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn parse_cli_supports_dashboard_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "export",
        "--export-dir",
        "./dashboards",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Export(inner) => {
                assert_eq!(inner.export_dir, Path::new("./dashboards"));
            }
            _ => panic!("expected dashboard export"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_shortcut_alias_db() {
    let args: CliArgs = parse_cli_from(["grafana-util", "db", "list", "--table"]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::List(inner) => {
                assert!(inner.table);
            }
            _ => panic!("expected dashboard list"),
        },
        _ => panic!("expected dashboard alias db"),
    }
}

#[test]
fn parse_cli_supports_datasource_shortcut_alias_ds() {
    let args: CliArgs = parse_cli_from(["grafana-util", "ds", "list", "--table"]);

    match args.command {
        UnifiedCommand::Datasource { command } => match command {
            DatasourceGroupCommand::List(inner) => {
                assert!(inner.table);
            }
            _ => panic!("expected datasource list"),
        },
        _ => panic!("expected datasource alias ds"),
    }
}

#[test]
fn parse_cli_supports_alert_shortcut_alias_al() {
    let args: CliArgs = parse_cli_from(["grafana-util", "al", "list-rules", "--json"]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::ListRules(inner)) => {
                assert!(inner.json);
            }
            _ => panic!("expected alert list-rules"),
        },
        _ => panic!("expected alert alias al"),
    }
}

#[test]
fn parse_cli_supports_access_shortcut_alias_ac() {
    let args: CliArgs = parse_cli_from(["grafana-util", "ac", "user", "list", "--json"]);

    match args.command {
        UnifiedCommand::Access(inner) => match inner.command {
            crate::access::AccessCommand::User { command } => match command {
                crate::access::UserCommand::List(inner) => {
                    assert!(inner.json);
                }
                _ => panic!("expected access user list"),
            },
            _ => panic!("expected access user"),
        },
        _ => panic!("expected access alias ac"),
    }
}

#[test]
fn parse_cli_supports_datasource_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "datasource",
        "import",
        "--import-dir",
        "./datasources",
        "--dry-run",
    ]);

    match args.command {
        UnifiedCommand::Datasource { command } => match command {
            DatasourceGroupCommand::Import(inner) => {
                assert_eq!(inner.import_dir, Path::new("./datasources"));
                assert!(inner.dry_run);
            }
            _ => panic!("expected datasource import"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_diff_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "datasource",
        "diff",
        "--diff-dir",
        "./datasources",
    ]);

    match args.command {
        UnifiedCommand::Datasource { command } => match command {
            DatasourceGroupCommand::Diff(inner) => {
                assert_eq!(inner.diff_dir, Path::new("./datasources"));
            }
            _ => panic!("expected datasource diff"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_inspect_export_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "inspect-export",
        "--import-dir",
        "./dashboards/raw",
        "--json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::InspectExport(inner) => {
                assert_eq!(inner.import_dir, Path::new("./dashboards/raw"));
                assert!(inner.json);
            }
            _ => panic!("expected dashboard inspect-export"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_inspect_live_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "inspect-live",
        "--url",
        "http://127.0.0.1:3000",
        "--report",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::InspectLive(inner) => {
                assert_eq!(inner.common.url, "http://127.0.0.1:3000");
                assert_eq!(
                    inner.report,
                    Some(crate::dashboard::InspectExportReportFormat::Json)
                );
            }
            _ => panic!("expected dashboard inspect-live"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_namespace_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "dashboard", "list", "--json"]);

    match args.command {
        UnifiedCommand::Dashboard { command: super::DashboardGroupCommand::List(inner) } => {
            assert!(inner.json);
        }
        _ => panic!("expected dashboard list"),
    }
}

#[test]
fn parse_cli_supports_alert_group() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "export",
        "--output-dir",
        "./alerts",
        "--overwrite",
    ]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Export(export_args)) => {
                assert_eq!(export_args.output_dir, Path::new("./alerts"));
                assert!(export_args.overwrite);
            }
            _ => panic!("expected alert export"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_list_rules_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "alert", "list-rules", "--json"]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::ListRules(alert_inner)) => {
                assert!(alert_inner.json);
            }
            _ => panic!("expected alert list-rules"),
        },
        _ => panic!("expected alert list-rules"),
    }
}

#[test]
fn parse_cli_supports_access_group() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "access",
        "user",
        "list",
        "--json",
        "--token",
        "abc",
    ]);

    match args.command {
        UnifiedCommand::Access(inner) => match inner.command {
            crate::access::AccessCommand::User { command } => match command {
                crate::access::UserCommand::List(list_args) => {
                    assert!(list_args.json);
                    assert_eq!(list_args.common.api_token.as_deref(), Some("abc"));
                }
                _ => panic!("expected user list"),
            },
            _ => panic!("expected access user"),
        },
        _ => panic!("expected access group"),
    }
}

#[test]
fn unified_help_mentions_alert_access_and_all_org_examples() {
    let help = render_unified_help();
    assert!(help.contains("grafana-util access org list"));
    assert!(help.contains("grafana-util access team list"));
    assert!(help.contains("grafana-util sync preflight"));
    assert!(help.contains("--basic-user admin --basic-password admin --all-orgs"));
    assert!(help.contains("grafana-util dashboard inspect-export"));
    assert!(help.contains("Datasource [list|add|modify|delete|export|import|diff]."));
    assert!(help.contains("Sync [summary|preflight|plan|apply|review|assess]."));
    assert!(help.contains("Dashboard [list|export|import|diff|inspect-export|inspect-live]."));
    assert!(help.contains("Alert [export|import|diff|list-rules|list-contact-points|list-mute-timings|list-templates]."));
    assert!(!help.contains("Compatibility shim remains available"));
    assert!(!help.contains("grafana-access-utils"));
}

#[test]
fn dashboard_namespace_help_includes_examples() {
    let export_help = render_unified_subcommand_help(&["dashboard", "export"]);
    assert!(export_help.contains("Examples:"));
    assert!(export_help.contains("grafana-util dashboard export"));

    let inspect_help = render_unified_subcommand_help(&["dashboard", "inspect-live"]);
    assert!(inspect_help.contains("Examples:"));
    assert!(inspect_help.contains("grafana-util dashboard inspect-live"));
}

#[test]
fn parse_cli_supports_sync_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "summary",
        "--desired-file",
        "./desired.json",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Summary(inner) => {
                assert_eq!(inner.desired_file, Path::new("./desired.json"));
                assert_eq!(inner.output, SyncOutputFormat::Json);
            }
            _ => panic!("expected sync summary"),
        },
        _ => panic!("expected sync group"),
    }
}

#[test]
fn parse_cli_supports_sync_shortcut_alias_sy() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sy",
        "summary",
        "--desired-file",
        "./desired.json",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Summary(inner) => {
                assert_eq!(inner.desired_file, Path::new("./desired.json"));
                assert_eq!(inner.output, SyncOutputFormat::Json);
            }
            _ => panic!("expected sync summary"),
        },
        _ => panic!("expected sync alias sy"),
    }
}

#[test]
fn parse_cli_supports_sync_plan_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "plan",
        "--desired-file",
        "./desired.json",
        "--live-file",
        "./live.json",
        "--trace-id",
        "trace-explicit",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Plan(inner) => {
                assert_eq!(inner.desired_file, Path::new("./desired.json"));
                assert_eq!(
                    inner.live_file,
                    Some(Path::new("./live.json").to_path_buf())
                );
                assert_eq!(inner.trace_id, Some("trace-explicit".to_string()));
                assert_eq!(inner.output, SyncOutputFormat::Json);
            }
            _ => panic!("expected sync plan"),
        },
        _ => panic!("expected sync group"),
    }
}

#[test]
fn parse_cli_supports_sync_apply_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "apply",
        "--plan-file",
        "./plan.json",
        "--preflight-file",
        "./preflight.json",
        "--bundle-preflight-file",
        "./bundle-preflight.json",
        "--approve",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Apply(inner) => {
                assert_eq!(inner.plan_file, Path::new("./plan.json"));
                assert_eq!(
                    inner.preflight_file,
                    Some(Path::new("./preflight.json").to_path_buf())
                );
                assert_eq!(
                    inner.bundle_preflight_file,
                    Some(Path::new("./bundle-preflight.json").to_path_buf())
                );
                assert!(inner.approve);
                assert_eq!(inner.output, SyncOutputFormat::Json);
                assert_eq!(inner.applied_by, None);
                assert_eq!(inner.applied_at, None);
                assert_eq!(inner.approval_reason, None);
                assert_eq!(inner.apply_note, None);
            }
            _ => panic!("expected sync apply"),
        },
        _ => panic!("expected sync group"),
    }
}

#[test]
fn parse_cli_supports_sync_apply_group_command_with_reason_and_note() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
        "--approval-reason",
        "change-approved",
        "--apply-note",
        "local apply intent only",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Apply(inner) => {
                assert_eq!(inner.approval_reason, Some("change-approved".to_string()));
                assert_eq!(
                    inner.apply_note,
                    Some("local apply intent only".to_string())
                );
            }
            _ => panic!("expected sync apply"),
        },
        _ => panic!("expected sync group"),
    }
}

#[test]
fn parse_cli_supports_sync_review_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "review",
        "--plan-file",
        "./plan.json",
        "--review-token",
        "reviewed-sync-plan",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Review(inner) => {
                assert_eq!(inner.plan_file, Path::new("./plan.json"));
                assert_eq!(inner.review_token, DEFAULT_REVIEW_TOKEN);
                assert_eq!(inner.output, SyncOutputFormat::Json);
                assert_eq!(inner.reviewed_by, None);
                assert_eq!(inner.reviewed_at, None);
                assert_eq!(inner.review_note, None);
            }
            _ => panic!("expected sync review"),
        },
        _ => panic!("expected sync group"),
    }
}

#[test]
fn parse_cli_supports_sync_review_group_command_with_note() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "review",
        "--plan-file",
        "./plan.json",
        "--review-note",
        "manual review complete",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Review(inner) => {
                assert_eq!(
                    inner.review_note,
                    Some("manual review complete".to_string())
                );
            }
            _ => panic!("expected sync review"),
        },
        _ => panic!("expected sync group"),
    }
}

#[test]
fn dispatch_routes_dashboard_group_to_dashboard_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "diff",
        "--import-dir",
        "./dashboards/raw",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |dashboard_args| {
            routed.borrow_mut().push(match dashboard_args.command {
                DashboardCommand::Diff(_) => "dashboard-diff".to_string(),
                _ => "other".to_string(),
            });
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_sync_args| {
            routed.borrow_mut().push("sync".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["dashboard-diff".to_string()]);
}

#[test]
fn dispatch_routes_access_group_to_access_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "access",
        "service-account",
        "list",
        "--json",
        "--token",
        "abc",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_sync_args| {
            routed.borrow_mut().push("sync".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["access".to_string()]);
}

#[test]
fn dispatch_routes_datasource_group_to_datasource_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "datasource",
        "list",
        "--json",
        "--token",
        "abc",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_sync_args| {
            routed.borrow_mut().push("sync".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["datasource".to_string()]);
}

#[test]
fn dispatch_routes_sync_group_to_sync_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "preflight",
        "--desired-file",
        "./desired.json",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_sync_args| {
            routed.borrow_mut().push("sync".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["sync".to_string()]);
}

#[test]
fn dispatch_routes_sync_review_to_sync_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "review",
        "--plan-file",
        "./plan.json",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_sync_args| {
            routed.borrow_mut().push("sync".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["sync".to_string()]);
}
