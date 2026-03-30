//! Unified CLI test suite.
//! Focuses on canonical command routing and ensures handlers receive the expected
//! domain payload shapes.
use super::{
    dispatch_with_handlers, maybe_render_unified_help_from_os_args, parse_cli_from,
    render_unified_help_full_text, render_unified_help_text, CliArgs, UnifiedCommand,
};
use crate::alert::{parse_cli_from as parse_alert_cli_from, root_command as alert_root_command};
use crate::dashboard::DashboardCommand;
use crate::datasource::DatasourceGroupCommand;
use crate::sync::{SyncGroupCommand, SyncOutputFormat, DEFAULT_REVIEW_TOKEN};
use clap::{CommandFactory, Parser};
use std::cell::RefCell;
use std::path::Path;

fn render_unified_help() -> String {
    render_unified_help_text(false)
}

fn render_unified_help_full() -> String {
    render_unified_help_full_text(false)
}

fn render_alert_subcommand_help(path: &[&str]) -> String {
    let mut command = alert_root_command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing alert subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn unified_help_mentions_screenshot_and_inspect_vars_examples() {
    let help = render_unified_help();
    assert!(help.contains("--help-full"));
    assert!(help.contains("Print help with extended examples"));
    assert!(help.contains("[Dashboard Export] Export dashboards with Basic auth"));
    assert!(help.contains("[Dashboard Export] Export dashboards across all visible orgs"));
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
fn parse_cli_supports_dashboard_group_screenshot_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Screenshot(inner) => {
                assert_eq!(inner.dashboard_uid.as_deref(), Some("cpu-main"));
                assert_eq!(inner.output, Path::new("./cpu-main.png"));
                assert!(!inner.full_page);
                assert_eq!(inner.output_format, None);
            }
            _ => panic!("expected dashboard screenshot"),
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
fn parse_cli_supports_dashboard_group_graph_alias() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "graph",
        "--governance",
        "./governance.json",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Topology(topology_args) => {
                assert_eq!(topology_args.governance, Path::new("./governance.json"));
            }
            _ => panic!("expected dashboard topology"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_rejects_dashboard_list_datasources_subcommand() {
    let error =
        CliArgs::try_parse_from(["grafana-util", "dashboard", "list-data-sources", "--json"])
            .unwrap_err();

    assert!(error.to_string().contains("unrecognized subcommand"));
    assert!(error.to_string().contains("list-data-sources"));
}

#[test]
fn parse_cli_supports_datasource_list_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "datasource", "list", "--json"]);

    match args.command {
        UnifiedCommand::Datasource { command } => match command {
            DatasourceGroupCommand::List(inner) => assert!(inner.json),
            _ => panic!("expected datasource list"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_browse_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "datasource", "browse", "--org-id", "4"]);

    match args.command {
        UnifiedCommand::Datasource { command } => match command {
            DatasourceGroupCommand::Browse(inner) => {
                assert_eq!(inner.org_id, Some(4));
                assert!(!inner.all_orgs);
            }
            _ => panic!("expected datasource browse"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_browse_all_orgs() {
    let args: CliArgs = parse_cli_from(["grafana-util", "datasource", "browse", "--all-orgs"]);

    match args.command {
        UnifiedCommand::Datasource { command } => match command {
            DatasourceGroupCommand::Browse(inner) => {
                assert!(inner.all_orgs);
                assert_eq!(inner.org_id, None);
            }
            _ => panic!("expected datasource browse"),
        },
        _ => panic!("expected datasource group"),
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
fn parse_cli_supports_alert_plan_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "plan",
        "--desired-dir",
        "./alerts/desired",
        "--prune",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Plan(plan_args)) => {
                assert_eq!(plan_args.desired_dir, Path::new("./alerts/desired"));
                assert!(plan_args.prune);
                assert_eq!(format!("{:?}", plan_args.output), "Json");
            }
            _ => panic!("expected alert plan"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_apply_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
    ]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Apply(apply_args)) => {
                assert_eq!(apply_args.plan_file, Path::new("./plan.json"));
                assert!(apply_args.approve);
            }
            _ => panic!("expected alert apply"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_delete_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "delete",
        "--kind",
        "policy-tree",
        "--identity",
        "default",
        "--allow-policy-reset",
    ]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Delete(delete_args)) => {
                assert_eq!(format!("{:?}", delete_args.kind), "PolicyTree");
                assert_eq!(delete_args.identity, "default");
                assert!(delete_args.allow_policy_reset);
            }
            _ => panic!("expected alert delete"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_scaffolding_group_commands() {
    let init_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "init",
        "--desired-dir",
        "./alerts/desired",
    ]);
    match init_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Init(init_args)) => {
                assert_eq!(init_args.desired_dir, Path::new("./alerts/desired"));
            }
            _ => panic!("expected alert init"),
        },
        _ => panic!("expected alert group"),
    }

    let new_rule_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "new-rule",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "cpu-main",
    ]);
    match new_rule_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::NewRule(new_args)) => {
                assert_eq!(new_args.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(new_args.name, "cpu-main");
            }
            _ => panic!("expected alert new-rule"),
        },
        _ => panic!("expected alert group"),
    }

    let new_contact_point_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "new-contact-point",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "pagerduty-primary",
    ]);
    match new_contact_point_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::NewContactPoint(new_args)) => {
                assert_eq!(new_args.name, "pagerduty-primary");
            }
            _ => panic!("expected alert new-contact-point"),
        },
        _ => panic!("expected alert group"),
    }

    let new_template_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "new-template",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "sev1-notification",
    ]);
    match new_template_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::NewTemplate(new_args)) => {
                assert_eq!(new_args.name, "sev1-notification");
            }
            _ => panic!("expected alert new-template"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_authoring_group_commands() {
    let add_rule_args: CliArgs = parse_cli_from([
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
        "--label",
        "team=platform",
        "--annotation",
        "summary=CPU high",
        "--severity",
        "critical",
        "--for",
        "5m",
        "--expr",
        "A",
        "--threshold",
        "80",
        "--above",
        "--dry-run",
    ]);
    match add_rule_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::AddRule(add_args)) => {
                assert_eq!(add_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(add_args.name, "cpu-high");
                assert_eq!(add_args.folder, "platform-alerts");
                assert_eq!(add_args.rule_group, "cpu");
                assert_eq!(add_args.receiver.as_deref(), Some("pagerduty-primary"));
                assert_eq!(add_args.labels, vec!["team=platform".to_string()]);
                assert_eq!(add_args.annotations, vec!["summary=CPU high".to_string()]);
                assert_eq!(add_args.severity.as_deref(), Some("critical"));
                assert_eq!(add_args.for_duration.as_deref(), Some("5m"));
                assert_eq!(add_args.expr.as_deref(), Some("A"));
                assert_eq!(add_args.threshold, Some(80.0));
                assert!(add_args.above);
                assert!(!add_args.below);
                assert!(add_args.dry_run);
            }
            _ => panic!("expected alert add-rule"),
        },
        _ => panic!("expected alert group"),
    }

    let clone_rule_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "clone-rule",
        "--desired-dir",
        "./alerts/desired",
        "--source",
        "cpu-high",
        "--name",
        "cpu-high-staging",
        "--dry-run",
    ]);
    match clone_rule_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::CloneRule(clone_args)) => {
                assert_eq!(clone_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(clone_args.source, "cpu-high");
                assert_eq!(clone_args.name, "cpu-high-staging");
                assert!(clone_args.dry_run);
            }
            _ => panic!("expected alert clone-rule"),
        },
        _ => panic!("expected alert group"),
    }

    let add_contact_point_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "add-contact-point",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "pagerduty-primary",
        "--dry-run",
    ]);
    match add_contact_point_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::AddContactPoint(add_args)) => {
                assert_eq!(add_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(add_args.name, "pagerduty-primary");
                assert!(add_args.dry_run);
            }
            _ => panic!("expected alert add-contact-point"),
        },
        _ => panic!("expected alert group"),
    }

    let set_route_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "set-route",
        "--desired-dir",
        "./alerts/desired",
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
        "--severity",
        "critical",
        "--dry-run",
    ]);
    match set_route_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::SetRoute(set_args)) => {
                assert_eq!(set_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(set_args.receiver, "pagerduty-primary");
                assert_eq!(set_args.labels, vec!["team=platform".to_string()]);
                assert_eq!(set_args.severity.as_deref(), Some("critical"));
                assert!(set_args.dry_run);
            }
            _ => panic!("expected alert set-route"),
        },
        _ => panic!("expected alert group"),
    }

    let preview_route_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "preview-route",
        "--desired-dir",
        "./alerts/desired",
        "--label",
        "team=platform",
        "--severity",
        "critical",
    ]);
    match preview_route_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::PreviewRoute(preview_args)) => {
                assert_eq!(preview_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(preview_args.labels, vec!["team=platform".to_string()]);
                assert_eq!(preview_args.severity.as_deref(), Some("critical"));
            }
            _ => panic!("expected alert preview-route"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_plan_normalized_args() {
    let args = parse_alert_cli_from([
        "grafana-util alert",
        "plan",
        "--desired-dir",
        "./alerts/desired",
        "--prune",
        "--dashboard-uid-map",
        "./dashboard-map.json",
        "--panel-id-map",
        "./panel-map.json",
        "--output",
        "json",
    ]);

    assert_eq!(format!("{:?}", args.command_kind), "Some(Plan)");
    assert_eq!(
        args.desired_dir.as_deref(),
        Some(Path::new("./alerts/desired"))
    );
    assert!(args.prune);
    assert_eq!(
        args.dashboard_uid_map.as_deref(),
        Some(Path::new("./dashboard-map.json"))
    );
    assert_eq!(
        args.panel_id_map.as_deref(),
        Some(Path::new("./panel-map.json"))
    );
    assert_eq!(format!("{:?}", args.command_output), "Some(Json)");
    assert!(args.json);
}

#[test]
fn parse_cli_supports_alert_apply_normalized_args() {
    let args = parse_alert_cli_from([
        "grafana-util alert",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
    ]);

    assert_eq!(format!("{:?}", args.command_kind), "Some(Apply)");
    assert_eq!(args.plan_file.as_deref(), Some(Path::new("./plan.json")));
    assert!(args.approve);
}

#[test]
fn parse_cli_supports_alert_delete_normalized_args() {
    let args = parse_alert_cli_from([
        "grafana-util alert",
        "delete",
        "--kind",
        "rule",
        "--identity",
        "cpu-main",
    ]);

    assert_eq!(format!("{:?}", args.command_kind), "Some(Delete)");
    assert_eq!(format!("{:?}", args.resource_kind), "Some(Rule)");
    assert_eq!(args.resource_identity.as_deref(), Some("cpu-main"));
    assert!(!args.allow_policy_reset);
}

#[test]
fn parse_cli_supports_alert_scaffolding_normalized_args() {
    let init_args = parse_alert_cli_from([
        "grafana-util alert",
        "init",
        "--desired-dir",
        "./alerts/desired",
    ]);
    assert_eq!(format!("{:?}", init_args.command_kind), "Some(Init)");
    assert_eq!(
        init_args.desired_dir.as_deref(),
        Some(Path::new("./alerts/desired"))
    );

    let rule_args = parse_alert_cli_from([
        "grafana-util alert",
        "new-rule",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "cpu-main",
    ]);
    assert_eq!(format!("{:?}", rule_args.command_kind), "Some(NewRule)");
    assert_eq!(rule_args.scaffold_name.as_deref(), Some("cpu-main"));

    let contact_point_args = parse_alert_cli_from([
        "grafana-util alert",
        "new-contact-point",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "pagerduty-primary",
    ]);
    assert_eq!(
        format!("{:?}", contact_point_args.command_kind),
        "Some(NewContactPoint)"
    );
    assert_eq!(
        contact_point_args.scaffold_name.as_deref(),
        Some("pagerduty-primary")
    );

    let template_args = parse_alert_cli_from([
        "grafana-util alert",
        "new-template",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "sev1-notification",
    ]);
    assert_eq!(
        format!("{:?}", template_args.command_kind),
        "Some(NewTemplate)"
    );
    assert_eq!(
        template_args.scaffold_name.as_deref(),
        Some("sev1-notification")
    );
}

#[test]
fn parse_cli_supports_alert_authoring_normalized_args() {
    let add_rule_args = parse_alert_cli_from([
        "grafana-util alert",
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
        "--label",
        "team=platform",
        "--annotation",
        "summary=CPU high",
        "--severity",
        "critical",
        "--for",
        "5m",
        "--expr",
        "A",
        "--threshold",
        "80",
        "--above",
        "--dry-run",
    ]);
    assert_eq!(
        format!("{:?}", add_rule_args.authoring_command_kind),
        "Some(AddRule)"
    );
    assert_eq!(
        add_rule_args.desired_dir.as_deref(),
        Some(Path::new("./alerts/desired"))
    );
    assert_eq!(add_rule_args.scaffold_name.as_deref(), Some("cpu-high"));
    assert_eq!(add_rule_args.folder.as_deref(), Some("platform-alerts"));
    assert_eq!(add_rule_args.rule_group.as_deref(), Some("cpu"));
    assert_eq!(add_rule_args.receiver.as_deref(), Some("pagerduty-primary"));
    assert_eq!(add_rule_args.labels, vec!["team=platform".to_string()]);
    assert_eq!(
        add_rule_args.annotations,
        vec!["summary=CPU high".to_string()]
    );
    assert_eq!(add_rule_args.severity.as_deref(), Some("critical"));
    assert_eq!(add_rule_args.for_duration.as_deref(), Some("5m"));
    assert_eq!(add_rule_args.expr.as_deref(), Some("A"));
    assert_eq!(add_rule_args.threshold, Some(80.0));
    assert!(add_rule_args.above);
    assert!(!add_rule_args.below);
    assert!(add_rule_args.dry_run);

    let clone_rule_args = parse_alert_cli_from([
        "grafana-util alert",
        "clone-rule",
        "--desired-dir",
        "./alerts/desired",
        "--source",
        "cpu-high",
        "--name",
        "cpu-high-staging",
        "--no-route",
        "--dry-run",
    ]);
    assert_eq!(
        format!("{:?}", clone_rule_args.authoring_command_kind),
        "Some(CloneRule)"
    );
    assert_eq!(clone_rule_args.source_name.as_deref(), Some("cpu-high"));
    assert_eq!(
        clone_rule_args.scaffold_name.as_deref(),
        Some("cpu-high-staging")
    );
    assert!(clone_rule_args.no_route);
    assert!(clone_rule_args.dry_run);

    let add_contact_point_args = parse_alert_cli_from([
        "grafana-util alert",
        "add-contact-point",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "pagerduty-primary",
        "--dry-run",
    ]);
    assert_eq!(
        format!("{:?}", add_contact_point_args.authoring_command_kind),
        "Some(AddContactPoint)"
    );
    assert_eq!(
        add_contact_point_args.scaffold_name.as_deref(),
        Some("pagerduty-primary")
    );
    assert!(add_contact_point_args.dry_run);

    let set_route_args = parse_alert_cli_from([
        "grafana-util alert",
        "set-route",
        "--desired-dir",
        "./alerts/desired",
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
        "--dry-run",
    ]);
    assert_eq!(
        format!("{:?}", set_route_args.authoring_command_kind),
        "Some(SetRoute)"
    );
    assert_eq!(
        set_route_args.receiver.as_deref(),
        Some("pagerduty-primary")
    );
    assert_eq!(set_route_args.labels, vec!["team=platform".to_string()]);
    assert!(set_route_args.dry_run);

    let preview_route_args = parse_alert_cli_from([
        "grafana-util alert",
        "preview-route",
        "--desired-dir",
        "./alerts/desired",
        "--label",
        "team=platform",
        "--severity",
        "critical",
    ]);
    assert_eq!(
        format!("{:?}", preview_route_args.authoring_command_kind),
        "Some(PreviewRoute)"
    );
    assert_eq!(
        preview_route_args.desired_dir.as_deref(),
        Some(Path::new("./alerts/desired"))
    );
    assert_eq!(preview_route_args.labels, vec!["team=platform".to_string()]);
    assert_eq!(preview_route_args.severity.as_deref(), Some("critical"));
}

#[test]
fn parse_cli_supports_alert_apply_requires_approve_flag() {
    let error = crate::alert::root_command()
        .try_get_matches_from(["grafana-util alert", "apply", "--plan-file", "./plan.json"])
        .unwrap_err();

    assert!(error.to_string().contains("--approve"));
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
fn parse_cli_supports_overview_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "overview",
        "--dashboard-export-dir",
        "./dashboards/raw",
    ]);

    match args.command {
        UnifiedCommand::Overview(inner) => {
            assert!(inner.command.is_none());
            assert_eq!(
                inner.staged.dashboard_export_dir.as_deref(),
                Some(Path::new("./dashboards/raw"))
            );
        }
        _ => panic!("expected overview group"),
    }
}

#[test]
fn parse_cli_supports_overview_live_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "overview", "live"]);

    match args.command {
        UnifiedCommand::Overview(inner) => match inner.command {
            Some(crate::overview::OverviewCommand::Live(_)) => {}
            _ => panic!("expected overview live"),
        },
        _ => panic!("expected overview group"),
    }
}

#[test]
fn parse_cli_supports_overview_live_org_scope_flags() {
    let args: CliArgs = parse_cli_from(["grafana-util", "overview", "live", "--org-id", "7"]);

    match args.command {
        UnifiedCommand::Overview(inner) => match inner.command {
            Some(crate::overview::OverviewCommand::Live(live)) => {
                assert_eq!(live.org_id, Some(7));
                assert!(!live.all_orgs);
            }
            _ => panic!("expected overview live"),
        },
        _ => panic!("expected overview group"),
    }
}

#[test]
fn parse_cli_supports_status_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "status", "staged"]);

    match args.command {
        UnifiedCommand::Status(_) => {}
        _ => panic!("expected status group"),
    }
}

#[test]
fn parse_cli_supports_status_live_staged_inputs() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "status",
        "live",
        "--sync-summary-file",
        "./sync-summary.json",
        "--bundle-preflight-file",
        "./bundle-preflight.json",
        "--promotion-summary-file",
        "./promotion-summary.json",
        "--mapping-file",
        "./mapping.json",
        "--availability-file",
        "./availability.json",
    ]);

    match args.command {
        UnifiedCommand::Status(inner) => match inner.command {
            crate::project_status_command::ProjectStatusSubcommand::Live(live) => {
                assert_eq!(
                    live.sync_summary_file.as_deref(),
                    Some(Path::new("./sync-summary.json"))
                );
                assert_eq!(
                    live.bundle_preflight_file.as_deref(),
                    Some(Path::new("./bundle-preflight.json"))
                );
                assert_eq!(
                    live.promotion_summary_file.as_deref(),
                    Some(Path::new("./promotion-summary.json"))
                );
                assert_eq!(
                    live.mapping_file.as_deref(),
                    Some(Path::new("./mapping.json"))
                );
                assert_eq!(
                    live.availability_file.as_deref(),
                    Some(Path::new("./availability.json"))
                );
            }
            _ => panic!("expected status live"),
        },
        _ => panic!("expected status group"),
    }
}

#[test]
fn parse_cli_supports_status_live_org_scope_flags() {
    let args: CliArgs = parse_cli_from(["grafana-util", "status", "live", "--all-orgs"]);

    match args.command {
        UnifiedCommand::Status(inner) => match inner.command {
            crate::project_status_command::ProjectStatusSubcommand::Live(live) => {
                assert!(live.all_orgs);
                assert_eq!(live.org_id, None);
            }
            _ => panic!("expected status live"),
        },
        _ => panic!("expected status group"),
    }
}

#[test]
fn unified_help_mentions_alert_access_and_shims() {
    let help = render_unified_help();
    assert!(help.contains("grafana-util access user list"));
    assert!(help.contains("[Alert Export]"));
    assert!(help.contains("[Datasource Inventory]"));
    assert!(help.contains("Run datasource browse, list, export, import, and diff workflows."));
    assert!(help.contains("[Access Inventory]"));
    assert!(help.contains("[Change Planning]"));
    assert!(help.contains("[Change Apply]"));
    assert!(help.contains("datasource"));
    assert!(help.contains("grafana-util change plan --desired-file ./desired.json --fetch-live"));
    assert!(help.contains(
        "grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live"
    ));
    assert!(help.contains(
        "Run review-first change workflows with optional live Grafana fetch/apply paths."
    ));
    assert!(help.contains("overview"));
    assert!(help.contains("Summarize project artifacts into a project-wide overview."));
    assert!(help.contains("overview live"));
    assert!(help.contains("Staged overview is the default"));
    assert!(help.contains("status"));
    assert!(help.contains("Render shared project-wide staged or live status."));
    assert!(help.contains("dashboard"));
    assert!(help.contains("[aliases: db]"));
    assert!(help.contains("[aliases: ds]"));
    assert!(!help.contains("Compatibility direct form"));
}

#[test]
fn render_unified_help_text_colorizes_example_labels_when_requested() {
    let help = render_unified_help_text(true);
    assert!(help.contains("\u{1b}[1;36m[Dashboard Export]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;31m[Alert Export]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;32m[Datasource Inventory]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;33m[Access Inventory]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;34m[Change Planning]\u{1b}[0m"));
}

#[test]
fn render_unified_help_text_colorizes_bracketed_usage_tokens_when_requested() {
    let help = render_unified_help_text(true);
    assert!(help.contains("\u{1b}[1m\u{1b}[32mUsage:\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1m\u{1b}[36m<COMMAND>\u{1b}[0m"));
    assert!(help.contains("\u{1b}[33m[aliases: \u{1b}[0m\u{1b}[33mdb\u{1b}[0m\u{1b}[33m]\u{1b}[0m"));
}

#[test]
fn unified_help_full_appends_extended_examples() {
    let help = render_unified_help_full();
    assert!(help.contains("Extended Examples:"));
    assert!(help.contains("[Dashboard Inspect Export]"));
    assert!(help.contains("grafana-util change review --plan-file ./sync-plan.json"));
}

#[test]
fn unified_help_full_colorizes_extended_example_labels_when_requested() {
    let help = render_unified_help_full_text(true);
    assert!(help.contains("\u{1b}[1;36m[Dashboard Inspect Export]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;31m[Alert Import]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;32m[Datasource Import]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;33m[Access Team Import]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;34m[Change Review]\u{1b}[0m"));
}

#[test]
fn maybe_render_unified_help_from_os_args_handles_root_help_and_help_full_flags() {
    let default_help = maybe_render_unified_help_from_os_args(["grafana-util"], false).unwrap();
    assert!(default_help.contains("[Dashboard Export]"));
    assert!(default_help.contains("Print help with extended examples"));

    let default_help_colorized =
        maybe_render_unified_help_from_os_args(["grafana-util"], true).unwrap_or_default();
    assert!(default_help_colorized.contains("\u{1b}[1;36m[Dashboard Export]\u{1b}[0m"));

    let root_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "--help"], false).unwrap();
    assert!(root_help.contains("[Dashboard Export]"));
    assert!(root_help.contains("--help-full"));

    let short_help = maybe_render_unified_help_from_os_args(["grafana-util", "-h"], false).unwrap();
    assert!(short_help.contains("[Change Apply]"));
    assert!(short_help.contains("Print help with extended examples"));

    let full_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "--help-full"], false).unwrap();
    assert!(full_help.contains("Extended Examples:"));
    assert!(full_help.contains("[Alert Import]"));

    let alert_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "alert", "--help-full"], false)
            .unwrap();
    assert!(alert_help.contains("Extended Examples:"));
    assert!(alert_help.contains("[Alert List]"));
    assert!(alert_help.contains("[Alert Plan]"));
    assert!(alert_help.contains("[Alert Apply]"));
    assert!(alert_help.contains("[Alert Delete]"));
    assert!(alert_help.contains("[Alert Add Rule]"));
    assert!(alert_help.contains("[Alert Clone Rule]"));
    assert!(alert_help.contains("[Alert Add Contact Point]"));
    assert!(alert_help.contains("[Alert Set Route]"));
    assert!(alert_help.contains("[Alert Preview Route]"));
    assert!(alert_help.contains("alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dry-run --json"));
    assert!(alert_help
        .contains("alert diff --url http://localhost:3000 --diff-dir ./alerts/raw --json"));
    assert!(alert_help.contains("alert plan --desired-dir ./alerts/desired --prune --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json --output json"));
    assert!(alert_help.contains("alert apply --plan-file ./alert-plan-reviewed.json --approve"));
    assert!(alert_help
        .contains("alert delete --kind policy-tree --identity default --allow-policy-reset"));
    assert!(alert_help.contains("alert add-rule --desired-dir ./alerts/desired --name cpu-high"));
    assert!(alert_help.contains("--dry-run"));
    assert!(alert_help.contains("alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical"));
    assert!(alert_help.contains("fully replaced on rerun instead of merged field-by-field"));
    assert!(alert_help.contains("low-level rule scaffold"));

    let datasource_help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "datasource", "--help-full"],
        false,
    )
    .unwrap();
    assert!(datasource_help.contains("[Datasource Diff]"));

    let access_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "access", "--help-full"], false)
            .unwrap();
    assert!(access_help.contains("[Access Token Add]"));

    let change_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "change", "--help-full"], false)
            .unwrap();
    assert!(change_help.contains("[Change Apply]"));
    assert!(change_help.contains("[Change Bundle]"));
    assert!(change_help.contains("[Change Bundle Preflight]"));

    let alert_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "alert", "-h"], false).unwrap();
    assert!(alert_short_help.contains("--help-full"));
    assert!(alert_short_help.contains("plan"));
    assert!(alert_short_help.contains("apply"));
    assert!(alert_short_help.contains("delete"));
    assert!(alert_short_help.contains("add-rule"));
    assert!(alert_short_help.contains("clone-rule"));
    assert!(alert_short_help.contains("add-contact-point"));
    assert!(alert_short_help.contains("set-route"));
    assert!(alert_short_help.contains("preview-route"));
    assert!(alert_short_help.contains("init"));
    assert!(alert_short_help.contains("new-rule"));
    assert!(alert_short_help.contains("new-contact-point"));
    assert!(alert_short_help.contains("new-template"));

    let datasource_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "datasource", "-h"], false)
            .unwrap();
    assert!(datasource_short_help.contains("--help-full"));

    let access_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "access", "-h"], false).unwrap();
    assert!(access_short_help.contains("--help-full"));

    let overview_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "overview", "--help-full"], false)
            .unwrap();
    assert!(overview_help.contains("overview"));
    assert!(overview_help.contains("project-wide overview"));
    assert!(overview_help.contains("overview live"));
    assert!(overview_help.contains("shared live status"));

    let project_status_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "status", "--help-full"], false)
            .unwrap();
    assert!(project_status_help.contains("status"));
    assert!(project_status_help
        .contains("Render project-wide staged or live status through the shared status contract."));
    assert!(project_status_help.contains("staged"));
    assert!(project_status_help.contains("live"));

    let change_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "change", "-h"], false).unwrap();
    assert!(change_short_help.contains("--help-full"));

    assert!(
        maybe_render_unified_help_from_os_args(["grafana-util", "dashboard", "--help"], false)
            .is_none()
    );
    assert!(maybe_render_unified_help_from_os_args(
        ["grafana-util", "dashboard", "--help-full"],
        false
    )
    .is_none());
    assert!(maybe_render_unified_help_from_os_args(
        ["grafana-util", "alert", "export", "--help-full"],
        false
    )
    .is_none());
}

#[test]
fn alert_help_subcommands_document_management_flags_and_examples() {
    let plan_help = render_alert_subcommand_help(&["plan"]);
    assert!(plan_help.contains("--desired-dir"));
    assert!(plan_help.contains("--prune"));
    assert!(plan_help.contains("--dashboard-uid-map"));
    assert!(plan_help.contains("--panel-id-map"));
    assert!(plan_help.contains("--output"));
    assert!(plan_help.contains("grafana-util alert plan"));

    let apply_help = render_alert_subcommand_help(&["apply"]);
    assert!(apply_help.contains("--plan-file"));
    assert!(apply_help.contains("--approve"));
    assert!(apply_help.contains("grafana-util alert apply"));

    let delete_help = render_alert_subcommand_help(&["delete"]);
    assert!(delete_help.contains("--kind"));
    assert!(delete_help.contains("--identity"));
    assert!(delete_help.contains("--allow-policy-reset"));
    assert!(delete_help.contains("grafana-util alert delete"));

    let init_help = render_alert_subcommand_help(&["init"]);
    assert!(init_help.contains("--desired-dir"));
    assert!(init_help.contains("grafana-util alert init"));

    let add_rule_help = render_alert_subcommand_help(&["add-rule"]);
    assert!(add_rule_help.contains("--desired-dir"));
    assert!(add_rule_help.contains("--name"));
    assert!(add_rule_help.contains("--folder"));
    assert!(add_rule_help.contains("--rule-group"));
    assert!(add_rule_help.contains("--receiver"));
    assert!(add_rule_help.contains("--no-route"));
    assert!(add_rule_help.contains("--label"));
    assert!(add_rule_help.contains("--annotation"));
    assert!(add_rule_help.contains("--severity"));
    assert!(add_rule_help.contains("--for"));
    assert!(add_rule_help.contains("--expr"));
    assert!(add_rule_help.contains("--threshold"));
    assert!(add_rule_help.contains("--above"));
    assert!(add_rule_help.contains("--below"));
    assert!(add_rule_help.contains("--dry-run"));
    assert!(add_rule_help.contains("grafana-util alert add-rule"));

    let clone_rule_help = render_alert_subcommand_help(&["clone-rule"]);
    assert!(clone_rule_help.contains("--desired-dir"));
    assert!(clone_rule_help.contains("--source"));
    assert!(clone_rule_help.contains("--name"));
    assert!(clone_rule_help.contains("--dry-run"));
    assert!(clone_rule_help.contains("grafana-util alert clone-rule"));

    let add_contact_point_help = render_alert_subcommand_help(&["add-contact-point"]);
    assert!(add_contact_point_help.contains("--desired-dir"));
    assert!(add_contact_point_help.contains("--name"));
    assert!(add_contact_point_help.contains("--dry-run"));
    assert!(add_contact_point_help.contains("grafana-util alert add-contact-point"));

    let set_route_help = render_alert_subcommand_help(&["set-route"]);
    assert!(set_route_help.contains("--desired-dir"));
    assert!(set_route_help.contains("--receiver"));
    assert!(set_route_help.contains("--label"));
    assert!(set_route_help.contains("--severity"));
    assert!(set_route_help.contains("--dry-run"));
    assert!(set_route_help.contains("fully replaces that managed route instead of merging fields"));
    assert!(set_route_help.contains("grafana-util alert set-route"));

    let preview_route_help = render_alert_subcommand_help(&["preview-route"]);
    assert!(preview_route_help.contains("--desired-dir"));
    assert!(preview_route_help.contains("--label"));
    assert!(preview_route_help.contains("--severity"));
    assert!(preview_route_help.contains("fully replaces the tool-owned route on rerun"));
    assert!(preview_route_help.contains("grafana-util alert preview-route"));

    let new_rule_help = render_alert_subcommand_help(&["new-rule"]);
    assert!(new_rule_help.contains("--desired-dir"));
    assert!(new_rule_help.contains("--name"));
    assert!(new_rule_help.contains("low-level staged alert rule scaffold"));
    assert!(new_rule_help.contains("grafana-util alert new-rule"));
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
fn parse_cli_supports_change_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "summary",
        "--desired-file",
        "./desired.json",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Summary(inner) => {
                assert_eq!(inner.desired_file, Path::new("./desired.json"));
                assert_eq!(inner.output, SyncOutputFormat::Json);
            }
            _ => panic!("expected change summary"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_assess_alerts_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "assess-alerts",
        "--alerts-file",
        "./alerts.json",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::AssessAlerts(inner) => {
                assert_eq!(inner.alerts_file, Path::new("./alerts.json"));
                assert_eq!(inner.output, SyncOutputFormat::Json);
            }
            _ => panic!("expected change assess-alerts"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_plan_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
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
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Plan(inner) => {
                assert_eq!(inner.desired_file, Path::new("./desired.json"));
                assert_eq!(
                    inner.live_file,
                    Some(Path::new("./live.json").to_path_buf())
                );
                assert_eq!(inner.trace_id, Some("trace-explicit".to_string()));
                assert_eq!(inner.output, SyncOutputFormat::Json);
            }
            _ => panic!("expected change plan"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_plan_fetch_live_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
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
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Plan(inner) => {
                assert_eq!(inner.desired_file, Path::new("./desired.json"));
                assert!(inner.fetch_live);
                assert_eq!(inner.org_id, Some(7));
                assert_eq!(inner.page_size, 250);
                assert_eq!(inner.common.url, "http://localhost:3000");
                assert_eq!(inner.common.api_token, Some("token-value".to_string()));
            }
            _ => panic!("expected change plan"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_apply_group_command_with_reason_and_note() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
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
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Apply(inner) => {
                assert_eq!(inner.approval_reason, Some("change-approved".to_string()));
                assert_eq!(
                    inner.apply_note,
                    Some("local apply intent only".to_string())
                );
            }
            _ => panic!("expected change apply"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_apply_execute_live_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
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
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Apply(inner) => {
                assert_eq!(inner.plan_file, Path::new("./plan.json"));
                assert!(inner.approve);
                assert!(inner.execute_live);
                assert!(inner.allow_folder_delete);
                assert_eq!(inner.org_id, Some(9));
                assert_eq!(inner.common.url, "http://localhost:3000");
                assert_eq!(inner.common.api_token, Some("token-value".to_string()));
            }
            _ => panic!("expected change apply"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_review_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "review",
        "--plan-file",
        "./plan.json",
        "--review-token",
        "reviewed-change-plan",
        "--output",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Review(inner) => {
                assert_eq!(inner.plan_file, Path::new("./plan.json"));
                assert_eq!(inner.review_token, DEFAULT_REVIEW_TOKEN);
                assert_eq!(inner.output, SyncOutputFormat::Json);
                assert_eq!(inner.reviewed_by, None);
                assert_eq!(inner.reviewed_at, None);
                assert_eq!(inner.review_note, None);
            }
            _ => panic!("expected change review"),
        },
        _ => panic!("expected change group"),
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
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
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
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
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
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
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
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["access".to_string()]);
}

#[test]
fn dispatch_routes_overview_group_to_overview_handler() {
    let args: CliArgs = parse_cli_from(["grafana-util", "overview"]);
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
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
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
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["overview".to_string()]);
}

#[test]
fn dispatch_routes_change_group_to_change_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
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
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
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
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["change".to_string()]);
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
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
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
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["datasource".to_string()]);
}

#[test]
fn dispatch_routes_status_group_to_status_handler() {
    let args: CliArgs = parse_cli_from(["grafana-util", "status", "staged"]);
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
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
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
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["status".to_string()]);
}

#[test]
fn overview_live_help_exposes_shared_live_status_contract() {
    let mut command = crate::overview::OverviewCliArgs::command();
    let live_command = command
        .find_subcommand_mut("live")
        .expect("overview live subcommand should exist");
    let help = live_command.render_long_help().to_string();

    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util overview live"));
}

#[test]
fn status_live_help_exposes_live_org_scope_contract() {
    let mut command = crate::project_status_command::ProjectStatusCliArgs::command();
    let live_command = command
        .find_subcommand_mut("live")
        .expect("status live subcommand should exist");
    let help = live_command.render_long_help().to_string();

    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util status live"));
}
