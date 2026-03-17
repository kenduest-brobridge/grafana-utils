// Unified CLI test suite.
// Focuses on canonical command routing and ensures handlers receive the expected domain payload shapes.
use super::{dispatch_with_handlers, parse_cli_from, CliArgs, UnifiedCommand};
use crate::dashboard::DashboardCommand;
use crate::datasource::DatasourceGroupCommand;
use crate::sync::{SyncGroupCommand, SyncOutputFormat, DEFAULT_REVIEW_TOKEN};
use clap::{CommandFactory, Parser};
use std::cell::RefCell;
use std::path::Path;

fn render_unified_help() -> String {
    let mut command = CliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn unified_help_mentions_screenshot_and_inspect_vars_examples() {
    let help = render_unified_help();
    assert!(help.contains("Export dashboards with Basic auth"));
    assert!(help.contains("Export dashboards across all visible orgs"));
    assert!(help.contains("--basic-user admin --basic-password admin"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("dashboard screenshot"));
    assert!(help.contains("dashboard inspect-vars"));
    assert!(help.contains("--dashboard-url"));
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
fn parse_cli_supports_datasource_group_alias() {
    let args: CliArgs = parse_cli_from(["grafana-util", "ds", "list", "--json"]);

    match args.command {
        UnifiedCommand::Datasource { command } => match command {
            DatasourceGroupCommand::List(inner) => {
                assert!(inner.json);
            }
            _ => panic!("expected datasource list"),
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
fn parse_cli_supports_dashboard_group_alias() {
    let args: CliArgs = parse_cli_from(["grafana-util", "db", "list", "--json"]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::List(inner) => assert!(inner.json),
            _ => panic!("expected dashboard list"),
        },
        _ => panic!("expected dashboard group"),
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
fn unified_help_mentions_alert_access_and_shims() {
    let help = render_unified_help();
    assert!(help.contains("grafana-util access user list"));
    assert!(help.contains("datasource"));
    assert!(help.contains("Run datasource list, export, import, and diff workflows."));
    assert!(help.contains("grafana-util sync plan --desired-file ./desired.json --fetch-live"));
    assert!(help.contains(
        "grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --execute-live"
    ));
    assert!(help.contains(
        "Run staged sync planning workflows with optional live Grafana fetch/apply paths."
    ));
    assert!(help.contains("dashboard"));
    assert!(help.contains("[aliases: db]"));
    assert!(help.contains("[aliases: ds]"));
    assert!(help.contains("[aliases: sy]"));
    assert!(!help.contains("Compatibility direct form"));
}

#[test]
fn parse_cli_rejects_legacy_dashboard_direct_command() {
    let error = CliArgs::try_parse_from(["grafana-util", "list-dashboard", "--json"]).unwrap_err();

    assert!(error.to_string().contains("unrecognized subcommand"));
    assert!(error.to_string().contains("list-dashboard"));
}

#[test]
fn parse_cli_rejects_legacy_alert_direct_command() {
    let error =
        CliArgs::try_parse_from(["grafana-util", "export-alert", "--output-dir", "./alerts"])
            .unwrap_err();

    assert!(error.to_string().contains("unrecognized subcommand"));
    assert!(error.to_string().contains("export-alert"));
}

#[test]
fn parse_cli_supports_sync_group_alias() {
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
        _ => panic!("expected sync group"),
    }
}

#[test]
fn parse_cli_supports_sync_assess_alerts_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "assess-alerts",
        "--alerts-file",
        "./alerts.json",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::AssessAlerts(inner) => {
                assert_eq!(inner.alerts_file, Path::new("./alerts.json"));
                assert_eq!(inner.output, SyncOutputFormat::Json);
            }
            _ => panic!("expected sync assess-alerts"),
        },
        _ => panic!("expected sync group"),
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
fn parse_cli_supports_sync_plan_fetch_live_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "plan",
        "--desired-file",
        "./desired.json",
        "--fetch-live",
        "--org-id",
        "7",
        "--page-size",
        "250",
        "--url",
        "http://localhost:3000",
        "--token",
        "token-value",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Plan(inner) => {
                assert_eq!(inner.desired_file, Path::new("./desired.json"));
                assert!(inner.fetch_live);
                assert_eq!(inner.org_id, Some(7));
                assert_eq!(inner.page_size, 250);
                assert_eq!(inner.common.url, "http://localhost:3000");
                assert_eq!(inner.common.api_token, Some("token-value".to_string()));
            }
            _ => panic!("expected sync plan"),
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
fn parse_cli_supports_sync_apply_execute_live_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "sync",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
        "--execute-live",
        "--allow-folder-delete",
        "--org-id",
        "9",
        "--url",
        "http://localhost:3000",
        "--token",
        "token-value",
    ]);

    match args.command {
        UnifiedCommand::Sync { command } => match command {
            SyncGroupCommand::Apply(inner) => {
                assert_eq!(inner.plan_file, Path::new("./plan.json"));
                assert!(inner.approve);
                assert!(inner.execute_live);
                assert!(inner.allow_folder_delete);
                assert_eq!(inner.org_id, Some(9));
                assert_eq!(inner.common.url, "http://localhost:3000");
                assert_eq!(inner.common.api_token, Some("token-value".to_string()));
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
