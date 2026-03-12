use super::{dispatch_with_handlers, parse_cli_from, CliArgs, UnifiedCommand};
use crate::dashboard::DashboardCommand;
use clap::CommandFactory;
use std::cell::RefCell;
use std::path::Path;

fn render_unified_help() -> String {
    let mut command = CliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn parse_cli_supports_dashboard_group_command() {
    let args: CliArgs = parse_cli_from(["grafana-utils", "dashboard", "export", "--export-dir", "./dashboards"]);

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
fn parse_cli_supports_legacy_dashboard_command() {
    let args: CliArgs = parse_cli_from(["grafana-utils", "list", "--json"]);

    match args.command {
        UnifiedCommand::List(inner) => {
            assert!(inner.json);
        }
        _ => panic!("expected legacy list"),
    }
}

#[test]
fn parse_cli_supports_alert_group() {
    let args: CliArgs =
        parse_cli_from(["grafana-utils", "alert", "export", "--output-dir", "./alerts", "--overwrite"]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Export(export_args)) => {
                assert_eq!(export_args.output_dir, Path::new("./alerts"));
                assert!(export_args.overwrite);
            }
            _ => panic!("expected alert export"),
        }
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_legacy_alert_alias() {
    let args: CliArgs = parse_cli_from(["grafana-utils", "list-alert-rules", "--json"]);

    match args.command {
        UnifiedCommand::ListAlertRules(inner) => {
            assert!(inner.json);
        }
        _ => panic!("expected list-alert-rules"),
    }
}

#[test]
fn parse_cli_supports_access_group() {
    let args: CliArgs =
        parse_cli_from(["grafana-utils", "access", "user", "list", "--json", "--token", "abc"]);

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
    assert!(help.contains("grafana-utils access user list"));
    assert!(help.contains("grafana-access-utils"));
}

#[test]
fn dispatch_routes_dashboard_group_to_dashboard_handler() {
    let args: CliArgs = parse_cli_from(["grafana-utils", "dashboard", "diff", "--import-dir", "./dashboards/raw"]);
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
    let args: CliArgs =
        parse_cli_from(["grafana-utils", "access", "service-account", "list", "--json", "--token", "abc"]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
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
