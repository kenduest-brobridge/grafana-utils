//! Change CLI parser/help test suite.
//! Verifies change argument parsing and rendered help contracts.
use super::{SyncCliArgs, SyncGroupCommand, SyncOutputFormat, DEFAULT_REVIEW_TOKEN};
use clap::{CommandFactory, Parser};
use std::path::Path;

fn render_change_subcommand_help(name: &str) -> String {
    let mut command = SyncCliArgs::command();
    let subcommand = command
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing change subcommand help for {name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn change_summary_help_includes_examples_and_output_heading() {
    let help = render_change_subcommand_help("summary");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Output Options"));
}

#[test]
fn change_plan_help_includes_examples_and_live_heading() {
    let help = render_change_subcommand_help("plan");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("Live Options"));
    assert!(help.contains("--fetch-live"));
}

#[test]
fn change_apply_help_includes_examples_and_approval_flags() {
    let help = render_change_subcommand_help("apply");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Approval Options"));
    assert!(help.contains("Live Options"));
    assert!(help.contains("--approve"));
    assert!(help.contains("--execute-live"));
    assert!(help.contains("--allow-folder-delete"));
    assert!(help.contains("--allow-policy-reset"));
}

#[test]
fn change_audit_help_mentions_lock_and_drift_controls() {
    let help = render_change_subcommand_help("audit");
    assert!(help.contains("--managed-file"));
    assert!(help.contains("--lock-file"));
    assert!(help.contains("--write-lock"));
    assert!(help.contains("--fail-on-drift"));
    assert!(help.contains("--interactive"));
}

#[test]
fn change_review_help_mentions_interactive_review() {
    let help = render_change_subcommand_help("review");
    assert!(help.contains("--interactive"));
}

#[test]
fn change_bundle_preflight_help_includes_examples_and_grouped_headings() {
    let help = render_change_subcommand_help("bundle-preflight");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("Live Options"));
    assert!(help.contains("--availability-file"));
    assert!(help.contains("secretPlaceholderNames"));
    assert!(help.contains("\"providerNames\": [\"vault\"]"));
}

#[test]
fn change_promotion_preflight_help_includes_mapping_input() {
    let help = render_change_subcommand_help("promotion-preflight");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("staged review handoff"));
    assert!(help.contains("--mapping-file"));
    assert!(help.contains("--availability-file"));
    assert!(help.contains("grafana-utils-sync-promotion-mapping"));
    assert!(help.contains("\"sourceEnvironment\": \"staging\""));
    assert!(help.contains("\"targetEnvironment\": \"prod\""));
    assert!(help.contains("secretPlaceholderNames"));
}

#[test]
fn change_bundle_help_includes_examples_and_output_heading() {
    let help = render_change_subcommand_help("bundle");
    assert!(help.contains("Examples:"));
    assert!(help.contains("--dashboard-export-dir"));
    assert!(help.contains("--dashboard-provisioning-dir"));
    assert!(help.contains("--output-file"));
    assert!(help.contains("--datasource-provisioning-file"));
}

#[test]
fn change_root_help_includes_examples() {
    let mut command = SyncCliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("Examples:"));
    assert!(help.contains("grafana-util change summary"));
    assert!(help.contains("grafana-util change plan"));
    assert!(help.contains("grafana-util change apply"));
    assert!(help.contains("grafana-util change audit"));
    assert!(help.contains("grafana-util change bundle"));
    assert!(help.contains("grafana-util change bundle-preflight"));
    assert!(help.contains("Assess staged promotion review handoff"));
    assert!(help.contains("promotion-preflight"));
}

#[test]
fn parse_change_cli_supports_summary_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "summary",
        "--desired-file",
        "./desired.json",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Summary(inner) => {
            assert_eq!(inner.desired_file, Path::new("./desired.json"));
            assert_eq!(inner.output, SyncOutputFormat::Json);
        }
        _ => panic!("expected summary"),
    }
}

#[test]
fn parse_change_cli_supports_assess_alerts_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "assess-alerts",
        "--alerts-file",
        "./alerts.json",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::AssessAlerts(inner) => {
            assert_eq!(inner.alerts_file, Path::new("./alerts.json"));
            assert_eq!(inner.output, SyncOutputFormat::Json);
        }
        _ => panic!("expected assess-alerts"),
    }
}

#[test]
fn parse_change_cli_supports_promotion_preflight_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "promotion-preflight",
        "--source-bundle",
        "./bundle.json",
        "--target-inventory",
        "./target.json",
        "--mapping-file",
        "./promotion-map.json",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::PromotionPreflight(inner) => {
            assert_eq!(inner.source_bundle, Path::new("./bundle.json"));
            assert_eq!(inner.target_inventory, Path::new("./target.json"));
            assert_eq!(
                inner.mapping_file,
                Some(Path::new("./promotion-map.json").to_path_buf())
            );
            assert_eq!(inner.output, SyncOutputFormat::Json);
        }
        _ => panic!("expected promotion-preflight"),
    }
}

#[test]
fn parse_change_cli_supports_audit_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "audit",
        "--managed-file",
        "./desired.json",
        "--lock-file",
        "./sync-lock.json",
        "--live-file",
        "./live.json",
        "--write-lock",
        "./next-lock.json",
        "--fail-on-drift",
        "--interactive",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Audit(inner) => {
            assert_eq!(inner.managed_file.unwrap(), Path::new("./desired.json"));
            assert_eq!(inner.lock_file.unwrap(), Path::new("./sync-lock.json"));
            assert_eq!(inner.live_file.unwrap(), Path::new("./live.json"));
            assert_eq!(inner.write_lock.unwrap(), Path::new("./next-lock.json"));
            assert!(inner.fail_on_drift);
            assert!(inner.interactive);
            assert_eq!(inner.output, SyncOutputFormat::Json);
        }
        _ => panic!("expected audit"),
    }
}

#[test]
fn parse_change_cli_supports_plan_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "plan",
        "--desired-file",
        "./desired.json",
        "--live-file",
        "./live.json",
        "--allow-prune",
        "--trace-id",
        "trace-explicit",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Plan(inner) => {
            assert_eq!(inner.desired_file, Path::new("./desired.json"));
            assert_eq!(
                inner.live_file,
                Some(Path::new("./live.json").to_path_buf())
            );
            assert!(inner.allow_prune);
            assert_eq!(inner.output, SyncOutputFormat::Json);
            assert_eq!(inner.trace_id, Some("trace-explicit".to_string()));
        }
        _ => panic!("expected plan"),
    }
}

#[test]
fn parse_change_cli_supports_plan_fetch_live_mode() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "plan",
        "--desired-file",
        "./desired.json",
        "--fetch-live",
        "--org-id",
        "7",
        "--page-size",
        "250",
        "--token",
        "test-token",
    ]);

    match args.command {
        SyncGroupCommand::Plan(inner) => {
            assert_eq!(inner.desired_file, Path::new("./desired.json"));
            assert_eq!(inner.live_file, None);
            assert!(inner.fetch_live);
            assert_eq!(inner.org_id, Some(7));
            assert_eq!(inner.page_size, 250);
        }
        _ => panic!("expected plan"),
    }
}

#[test]
fn parse_change_cli_supports_review_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "review",
        "--plan-file",
        "./plan.json",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Review(inner) => {
            assert_eq!(inner.plan_file, Path::new("./plan.json"));
            assert_eq!(inner.review_token, DEFAULT_REVIEW_TOKEN);
            assert_eq!(inner.output, SyncOutputFormat::Json);
            assert_eq!(inner.reviewed_by, None);
            assert_eq!(inner.reviewed_at, None);
            assert_eq!(inner.review_note, None);
        }
        _ => panic!("expected review"),
    }
}

#[test]
fn parse_change_cli_supports_review_command_with_note() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "review",
        "--plan-file",
        "./plan.json",
        "--review-note",
        "manual review complete",
    ]);

    match args.command {
        SyncGroupCommand::Review(inner) => {
            assert_eq!(
                inner.review_note,
                Some("manual review complete".to_string())
            );
        }
        _ => panic!("expected review"),
    }
}

#[test]
fn parse_change_cli_supports_apply_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
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
            assert!(!inner.execute_live);
            assert!(!inner.allow_folder_delete);
            assert!(!inner.allow_policy_reset);
            assert_eq!(inner.applied_by, None);
            assert_eq!(inner.applied_at, None);
            assert_eq!(inner.approval_reason, None);
            assert_eq!(inner.apply_note, None);
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn parse_change_cli_supports_apply_execute_live_flags() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
        "--execute-live",
        "--allow-folder-delete",
        "--allow-policy-reset",
        "--org-id",
        "9",
        "--token",
        "test-token",
    ]);

    match args.command {
        SyncGroupCommand::Apply(inner) => {
            assert!(inner.execute_live);
            assert!(inner.allow_folder_delete);
            assert!(inner.allow_policy_reset);
            assert_eq!(inner.org_id, Some(9));
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn parse_change_cli_supports_preflight_fetch_live_mode() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "preflight",
        "--desired-file",
        "./desired.json",
        "--fetch-live",
        "--org-id",
        "3",
        "--token",
        "test-token",
    ]);

    match args.command {
        SyncGroupCommand::Preflight(inner) => {
            assert!(inner.fetch_live);
            assert_eq!(inner.org_id, Some(3));
        }
        _ => panic!("expected preflight"),
    }
}

#[test]
fn parse_change_cli_supports_bundle_preflight_fetch_live_mode() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "bundle-preflight",
        "--source-bundle",
        "./bundle.json",
        "--target-inventory",
        "./target.json",
        "--fetch-live",
        "--org-id",
        "5",
        "--token",
        "test-token",
    ]);

    match args.command {
        SyncGroupCommand::BundlePreflight(inner) => {
            assert!(inner.fetch_live);
            assert_eq!(inner.org_id, Some(5));
        }
        _ => panic!("expected bundle-preflight"),
    }
}

#[test]
fn parse_change_cli_supports_apply_command_with_reason_and_note() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
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
        SyncGroupCommand::Apply(inner) => {
            assert_eq!(inner.approval_reason, Some("change-approved".to_string()));
            assert_eq!(
                inner.apply_note,
                Some("local apply intent only".to_string())
            );
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn parse_change_cli_supports_bundle_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "bundle",
        "--dashboard-export-dir",
        "./dashboards/raw",
        "--alert-export-dir",
        "./alerts/raw",
        "--datasource-export-file",
        "./datasources.json",
        "--metadata-file",
        "./metadata.json",
        "--output-file",
        "./bundle.json",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Bundle(inner) => {
            assert_eq!(
                inner.dashboard_export_dir,
                Some(Path::new("./dashboards/raw").to_path_buf())
            );
            assert_eq!(
                inner.alert_export_dir,
                Some(Path::new("./alerts/raw").to_path_buf())
            );
            assert_eq!(
                inner.datasource_export_file,
                Some(Path::new("./datasources.json").to_path_buf())
            );
            assert_eq!(inner.datasource_provisioning_file, None);
            assert_eq!(
                inner.metadata_file,
                Some(Path::new("./metadata.json").to_path_buf())
            );
            assert_eq!(
                inner.output_file,
                Some(Path::new("./bundle.json").to_path_buf())
            );
            assert_eq!(inner.output, SyncOutputFormat::Json);
        }
        _ => panic!("expected bundle"),
    }
}

#[test]
fn parse_change_cli_supports_bundle_provisioning_file() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "bundle",
        "--dashboard-export-dir",
        "./dashboards/raw",
        "--datasource-provisioning-file",
        "./dashboards/provisioning/datasources.yaml",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Bundle(inner) => {
            assert_eq!(
                inner.datasource_provisioning_file,
                Some(Path::new("./dashboards/provisioning/datasources.yaml").to_path_buf())
            );
            assert_eq!(inner.datasource_export_file, None);
        }
        _ => panic!("expected bundle"),
    }
}

#[test]
fn parse_change_cli_supports_bundle_provisioning_dir() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "bundle",
        "--dashboard-provisioning-dir",
        "./dashboards/provisioning",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Bundle(inner) => {
            assert_eq!(
                inner.dashboard_provisioning_dir,
                Some(Path::new("./dashboards/provisioning").to_path_buf())
            );
            assert_eq!(inner.dashboard_export_dir, None);
        }
        _ => panic!("expected bundle"),
    }
}

#[test]
fn parse_change_cli_rejects_conflicting_dashboard_bundle_inputs() {
    let error = SyncCliArgs::try_parse_from([
        "grafana-util",
        "bundle",
        "--dashboard-export-dir",
        "./dashboards/raw",
        "--dashboard-provisioning-dir",
        "./dashboards/provisioning",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("cannot be used with"));
}
