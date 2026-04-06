//! Change CLI parser/help test suite.
//! Verifies task-first routing and advanced workflow help contracts.
use super::{
    SyncAdvancedCommand, SyncCliArgs, SyncGroupCommand, SyncOutputFormat, DEFAULT_REVIEW_TOKEN,
};
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

fn render_change_advanced_subcommand_help(name: &str) -> String {
    let mut command = SyncCliArgs::command();
    let advanced = command
        .find_subcommand_mut("advanced")
        .expect("missing change advanced help");
    let subcommand = advanced
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing change advanced subcommand help for {name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn change_inspect_help_includes_examples_and_output_heading() {
    let help = render_change_subcommand_help("inspect");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Output Options"));
}

#[test]
fn change_check_help_includes_examples_and_live_heading() {
    let help = render_change_subcommand_help("check");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("Live Options"));
    assert!(help.contains("--fetch-live"));
}

#[test]
fn change_preview_help_includes_examples_and_live_heading() {
    let help = render_change_subcommand_help("preview");
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
    assert!(help.contains("--preview-file"));
}

#[test]
fn change_advanced_help_mentions_lower_level_workflows() {
    let help = render_change_subcommand_help("advanced");
    assert!(help.contains("change advanced summary"));
    assert!(help.contains("change advanced review"));
    assert!(help.contains("change advanced bundle-preflight"));
}

#[test]
fn change_advanced_audit_help_mentions_lock_and_drift_controls() {
    let help = render_change_advanced_subcommand_help("audit");
    assert!(help.contains("--managed-file"));
    assert!(help.contains("--lock-file"));
    assert!(help.contains("--write-lock"));
    assert!(help.contains("--fail-on-drift"));
    assert!(help.contains("--interactive"));
}

#[test]
fn change_advanced_review_help_mentions_interactive_review() {
    let help = render_change_advanced_subcommand_help("review");
    assert!(help.contains("--interactive"));
}

#[test]
fn change_advanced_bundle_preflight_help_includes_examples_and_grouped_headings() {
    let help = render_change_advanced_subcommand_help("bundle-preflight");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("Live Options"));
    assert!(help.contains("--availability-file"));
    assert!(help.contains("secretPlaceholderNames"));
    assert!(help.contains("\"providerNames\": [\"vault\"]"));
}

#[test]
fn change_advanced_promotion_preflight_help_includes_mapping_input() {
    let help = render_change_advanced_subcommand_help("promotion-preflight");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("staged review handoff"));
    assert!(help.contains("--mapping-file"));
    assert!(help.contains("--availability-file"));
    assert!(help.contains("grafana-utils-sync-promotion-mapping"));
}

#[test]
fn change_advanced_bundle_help_includes_examples_and_output_heading() {
    let help = render_change_advanced_subcommand_help("bundle");
    assert!(help.contains("Examples:"));
    assert!(help.contains("--dashboard-export-dir"));
    assert!(help.contains("--dashboard-provisioning-dir"));
    assert!(help.contains("--output-file"));
    assert!(help.contains("--also-stdout"));
}

#[test]
fn change_root_help_includes_task_first_examples() {
    let mut command = SyncCliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("grafana-util change inspect"));
    assert!(help.contains("grafana-util change check"));
    assert!(help.contains("grafana-util change preview"));
    assert!(help.contains("grafana-util change apply"));
    assert!(help.contains("grafana-util change advanced bundle"));
    assert!(help.contains("grafana-util change advanced bundle-preflight"));
}

#[test]
fn parse_change_cli_supports_inspect_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "inspect",
        "--dashboard-export-dir",
        "./dashboards/raw",
        "--output-format",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Inspect(inner) => {
            assert_eq!(
                inner.inputs.dashboard_export_dir,
                Some(Path::new("./dashboards/raw").to_path_buf())
            );
            assert_eq!(inner.output.output_format, SyncOutputFormat::Json);
        }
        _ => panic!("expected inspect"),
    }
}

#[test]
fn parse_change_cli_supports_check_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "check",
        "--source-bundle",
        "./bundle.json",
        "--target-inventory",
        "./target.json",
        "--mapping-file",
        "./mapping.json",
        "--fetch-live",
        "--output-format",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Check(inner) => {
            assert_eq!(
                inner.inputs.source_bundle,
                Some(Path::new("./bundle.json").to_path_buf())
            );
            assert_eq!(
                inner.target_inventory,
                Some(Path::new("./target.json").to_path_buf())
            );
            assert_eq!(
                inner.mapping_file,
                Some(Path::new("./mapping.json").to_path_buf())
            );
            assert!(inner.fetch_live);
            assert_eq!(inner.output.output_format, SyncOutputFormat::Json);
        }
        _ => panic!("expected check"),
    }
}

#[test]
fn parse_change_cli_supports_preview_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "preview",
        "--desired-file",
        "./desired.json",
        "--live-file",
        "./live.json",
        "--allow-prune",
        "--trace-id",
        "trace-explicit",
        "--output-format",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Preview(inner) => {
            assert_eq!(
                inner.inputs.desired_file,
                Some(Path::new("./desired.json").to_path_buf())
            );
            assert_eq!(
                inner.live_file,
                Some(Path::new("./live.json").to_path_buf())
            );
            assert!(inner.allow_prune);
            assert_eq!(inner.output.output_format, SyncOutputFormat::Json);
            assert_eq!(inner.trace_id, Some("trace-explicit".to_string()));
        }
        _ => panic!("expected preview"),
    }
}

#[test]
fn parse_change_cli_supports_preview_fetch_live_mode() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "preview",
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
        SyncGroupCommand::Preview(inner) => {
            assert_eq!(
                inner.inputs.desired_file,
                Some(Path::new("./desired.json").to_path_buf())
            );
            assert_eq!(inner.live_file, None);
            assert!(inner.fetch_live);
            assert_eq!(inner.org_id, Some(7));
            assert_eq!(inner.page_size, 250);
        }
        _ => panic!("expected preview"),
    }
}

#[test]
fn parse_change_cli_supports_apply_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "apply",
        "--preview-file",
        "./plan.json",
        "--preflight-file",
        "./preflight.json",
        "--bundle-preflight-file",
        "./bundle-preflight.json",
        "--approve",
        "--output-format",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Apply(inner) => {
            assert_eq!(inner.plan_file.as_deref(), Some(Path::new("./plan.json")));
            assert_eq!(
                inner.preflight_file,
                Some(Path::new("./preflight.json").to_path_buf())
            );
            assert_eq!(
                inner.bundle_preflight_file,
                Some(Path::new("./bundle-preflight.json").to_path_buf())
            );
            assert!(inner.approve);
            assert_eq!(inner.output_format, SyncOutputFormat::Json);
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
            assert_eq!(inner.plan_file.as_deref(), Some(Path::new("./plan.json")));
            assert!(inner.execute_live);
            assert!(inner.allow_folder_delete);
            assert!(inner.allow_policy_reset);
            assert_eq!(inner.org_id, Some(9));
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn parse_change_cli_supports_advanced_review_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "advanced",
        "review",
        "--plan-file",
        "./plan.json",
        "--output-format",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Advanced(inner) => match inner.command {
            SyncAdvancedCommand::Review(inner) => {
                assert_eq!(inner.plan_file, Path::new("./plan.json"));
                assert_eq!(inner.review_token, DEFAULT_REVIEW_TOKEN);
                assert_eq!(inner.output_format, SyncOutputFormat::Json);
            }
            _ => panic!("expected advanced review"),
        },
        _ => panic!("expected advanced"),
    }
}

#[test]
fn parse_change_cli_supports_advanced_audit_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "advanced",
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
        "--output-format",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Advanced(inner) => match inner.command {
            SyncAdvancedCommand::Audit(inner) => {
                assert_eq!(inner.managed_file.unwrap(), Path::new("./desired.json"));
                assert_eq!(inner.lock_file.unwrap(), Path::new("./sync-lock.json"));
                assert_eq!(inner.live_file.unwrap(), Path::new("./live.json"));
                assert_eq!(inner.write_lock.unwrap(), Path::new("./next-lock.json"));
                assert!(inner.fail_on_drift);
                assert!(inner.interactive);
                assert_eq!(inner.output_format, SyncOutputFormat::Json);
            }
            _ => panic!("expected advanced audit"),
        },
        _ => panic!("expected advanced"),
    }
}

#[test]
fn parse_change_cli_supports_advanced_promotion_preflight_command() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "advanced",
        "promotion-preflight",
        "--source-bundle",
        "./bundle.json",
        "--target-inventory",
        "./target.json",
        "--mapping-file",
        "./promotion-map.json",
        "--output-format",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Advanced(inner) => match inner.command {
            SyncAdvancedCommand::PromotionPreflight(inner) => {
                assert_eq!(inner.source_bundle, Path::new("./bundle.json"));
                assert_eq!(inner.target_inventory, Path::new("./target.json"));
                assert_eq!(
                    inner.mapping_file,
                    Some(Path::new("./promotion-map.json").to_path_buf())
                );
                assert_eq!(inner.output_format, SyncOutputFormat::Json);
            }
            _ => panic!("expected advanced promotion-preflight"),
        },
        _ => panic!("expected advanced"),
    }
}
