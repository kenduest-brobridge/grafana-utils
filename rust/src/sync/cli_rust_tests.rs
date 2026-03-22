//! Sync CLI test suite.
//! Verifies sync argument parsing, plan/review/apply routing, and rendering contracts.
use super::{
    execute_live_apply_with_request, fetch_live_availability_with_request,
    fetch_live_resource_specs_with_request, render_alert_sync_assessment_text,
    render_sync_apply_intent_text, render_sync_plan_text, render_sync_summary_text, run_sync_cli,
    SyncApplyArgs, SyncAssessAlertsArgs, SyncBundleArgs, SyncBundlePreflightArgs, SyncCliArgs,
    SyncGroupCommand, SyncOutputFormat, SyncPlanArgs, SyncPreflightArgs, SyncReviewArgs,
    SyncSummaryArgs, DEFAULT_REVIEW_TOKEN,
};
use crate::dashboard::CommonCliArgs;
use clap::{CommandFactory, Parser};
use reqwest::Method;
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn sync_common_args() -> CommonCliArgs {
    CommonCliArgs {
        url: "http://127.0.0.1:3000".to_string(),
        api_token: Some("test-token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

fn render_sync_subcommand_help(name: &str) -> String {
    let mut command = SyncCliArgs::command();
    let subcommand = command
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing sync subcommand help for {name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn sync_summary_help_includes_examples_and_output_heading() {
    let help = render_sync_subcommand_help("summary");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Output Options"));
}

#[test]
fn sync_plan_help_includes_examples_and_live_heading() {
    let help = render_sync_subcommand_help("plan");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("Live Options"));
    assert!(help.contains("--fetch-live"));
}

#[test]
fn sync_apply_help_includes_examples_and_approval_flags() {
    let help = render_sync_subcommand_help("apply");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Approval Options"));
    assert!(help.contains("Live Options"));
    assert!(help.contains("--approve"));
    assert!(help.contains("--execute-live"));
    assert!(help.contains("--allow-folder-delete"));
}

#[test]
fn sync_bundle_preflight_help_includes_examples_and_grouped_headings() {
    let help = render_sync_subcommand_help("bundle-preflight");
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("Live Options"));
}

#[test]
fn sync_bundle_help_includes_examples_and_output_heading() {
    let help = render_sync_subcommand_help("bundle");
    assert!(help.contains("Examples:"));
    assert!(help.contains("--dashboard-export-dir"));
    assert!(help.contains("--output-file"));
}

#[test]
fn sync_root_help_includes_examples() {
    let mut command = SyncCliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("Examples:"));
    assert!(help.contains("grafana-util sync summary"));
    assert!(help.contains("grafana-util sync plan"));
    assert!(help.contains("grafana-util sync apply"));
    assert!(help.contains("grafana-util sync bundle"));
    assert!(help.contains("grafana-util sync bundle-preflight"));
}

#[test]
fn parse_sync_cli_supports_summary_command() {
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
fn parse_sync_cli_supports_assess_alerts_command() {
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
fn parse_sync_cli_supports_plan_command() {
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
fn parse_sync_cli_supports_plan_fetch_live_mode() {
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
fn parse_sync_cli_supports_review_command() {
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
fn parse_sync_cli_supports_review_command_with_note() {
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
fn parse_sync_cli_supports_apply_command() {
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
            assert_eq!(inner.applied_by, None);
            assert_eq!(inner.applied_at, None);
            assert_eq!(inner.approval_reason, None);
            assert_eq!(inner.apply_note, None);
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn parse_sync_cli_supports_apply_execute_live_flags() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
        "--execute-live",
        "--allow-folder-delete",
        "--org-id",
        "9",
        "--token",
        "test-token",
    ]);

    match args.command {
        SyncGroupCommand::Apply(inner) => {
            assert!(inner.execute_live);
            assert!(inner.allow_folder_delete);
            assert_eq!(inner.org_id, Some(9));
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn parse_sync_cli_supports_preflight_fetch_live_mode() {
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
fn parse_sync_cli_supports_bundle_preflight_fetch_live_mode() {
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
fn parse_sync_cli_supports_apply_command_with_reason_and_note() {
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
fn parse_sync_cli_supports_bundle_command() {
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
fn render_sync_summary_text_renders_counts() {
    let lines = render_sync_summary_text(&json!({
        "kind": "grafana-utils-sync-summary",
        "summary": {
            "resourceCount": 3,
            "dashboardCount": 1,
            "datasourceCount": 1,
            "folderCount": 1,
            "alertCount": 0
        }
    }))
    .unwrap();

    assert_eq!(lines[0], "Sync summary");
    assert!(lines[1].contains("3 total"));
}

#[test]
fn render_alert_sync_assessment_text_renders_status_lines() {
    let lines = render_alert_sync_assessment_text(&json!({
        "kind": "grafana-utils-alert-sync-plan",
        "summary": {
            "alertCount": 1,
            "candidateCount": 0,
            "planOnlyCount": 1,
            "blockedCount": 0
        },
        "alerts": [
            {
                "identity": "cpu-high",
                "status": "plan-only",
                "liveApplyAllowed": false,
                "detail": "detail text"
            }
        ]
    }))
    .unwrap();

    assert_eq!(lines[0], "Alert sync assessment");
    assert!(lines[1].contains("plan-only"));
    assert!(lines[4].contains("cpu-high"));
}

#[test]
fn render_sync_plan_text_renders_counts() {
    let lines = render_sync_plan_text(&json!({
        "kind": "grafana-utils-sync-plan",
        "stage": "review",
        "stepIndex": 2,
        "parentTraceId": "sync-trace-demo",
        "summary": {
            "would_create": 1,
            "would_update": 2,
            "would_delete": 0,
            "noop": 3,
            "unmanaged": 1,
            "alert_candidate": 0,
            "alert_plan_only": 1,
            "alert_blocked": 0
        },
        "reviewRequired": true,
        "reviewed": false,
        "traceId": "sync-trace-demo",
        "reviewedBy": "alice",
        "reviewedAt": "staged:sync-trace-demo:reviewed",
        "reviewNote": "manual review complete"
    }))
    .unwrap();

    assert_eq!(lines[0], "Sync plan");
    assert!(lines[1].contains("sync-trace-demo"));
    assert!(lines[2].contains("stage=review"));
    assert!(lines[2].contains("step=2"));
    assert!(lines[2].contains("parent=sync-trace-demo"));
    assert!(lines[3].contains("create=1"));
    assert!(lines[4].contains("plan-only=1"));
    assert!(lines[5].contains("reviewed=false"));
    assert!(lines[6].contains("alice"));
    assert!(lines[7].contains("staged:sync-trace-demo:reviewed"));
    assert!(lines[8].contains("manual review complete"));
}

#[test]
fn render_sync_apply_intent_text_renders_counts() {
    let lines = render_sync_apply_intent_text(&json!({
        "kind": "grafana-utils-sync-apply-intent",
        "stage": "apply",
        "stepIndex": 3,
        "parentTraceId": "sync-trace-demo",
        "summary": {
            "would_create": 1,
            "would_update": 2,
            "would_delete": 1
        },
        "operations": [
            {"action":"would-create"},
            {"action":"would-update"}
        ],
        "preflightSummary": {
            "kind": "grafana-utils-sync-preflight",
            "checkCount": 4,
            "okCount": 4,
            "blockingCount": 0
        },
        "bundlePreflightSummary": {
            "kind": "grafana-utils-sync-bundle-preflight",
            "resourceCount": 4,
            "syncBlockingCount": 0,
            "providerBlockingCount": 0
        },
        "reviewRequired": true,
        "approved": true,
        "reviewed": true,
        "traceId": "sync-trace-demo",
        "appliedBy": "bob",
        "appliedAt": "staged:sync-trace-demo:applied",
        "approvalReason": "change-approved",
        "applyNote": "local apply intent only"
    }))
    .unwrap();

    assert_eq!(lines[0], "Sync apply intent");
    assert!(lines[1].contains("sync-trace-demo"));
    assert!(lines[2].contains("stage=apply"));
    assert!(lines[2].contains("step=3"));
    assert!(lines[2].contains("parent=sync-trace-demo"));
    assert!(lines[3].contains("executable=2"));
    assert!(lines[4].contains("required=true"));
    assert!(lines[4].contains("approved=true"));
    assert!(lines[5].contains("kind=grafana-utils-sync-preflight"));
    assert!(lines[5].contains("blocking=0"));
    assert!(lines[6].contains("sync-blocking=0"));
    assert!(lines[7].contains("bob"));
    assert!(lines[8].contains("staged:sync-trace-demo:applied"));
    assert!(lines[9].contains("change-approved"));
    assert!(lines[10].contains("local apply intent only"));
}

#[test]
fn render_sync_plan_text_defaults_lineage_when_missing() {
    let lines = render_sync_plan_text(&json!({
        "kind": "grafana-utils-sync-plan",
        "summary": {
            "would_create": 0,
            "would_update": 0,
            "would_delete": 0,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 0
        },
        "reviewRequired": true,
        "reviewed": false,
        "traceId": "sync-trace-demo"
    }))
    .unwrap();

    assert!(lines[2].contains("stage=missing"));
    assert!(lines[2].contains("step=0"));
    assert!(lines[2].contains("parent=none"));
}

#[test]
fn fetch_live_resource_specs_with_request_collects_alerts_and_alerting_resources() {
    let mut calls = Vec::new();
    let specs = fetch_live_resource_specs_with_request(
        |method, path, params, payload| {
            calls.push((
                method.clone(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match (method.clone(), path) {
                (Method::GET, "/api/folders") => Ok(Some(json!([
                    {"uid": "ops", "title": "Operations"}
                ]))),
                (Method::GET, "/api/search") => {
                    let page = params
                        .iter()
                        .find(|(key, _)| key == "page")
                        .map(|(_, value)| value.as_str())
                        .unwrap_or("1");
                    if page == "1" {
                        Ok(Some(json!([
                            {"uid": "cpu-main", "title": "CPU Main"}
                        ])))
                    } else {
                        Ok(Some(json!([])))
                    }
                }
                (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                    "dashboard": {"uid": "cpu-main", "title": "CPU Main", "panels": []}
                }))),
                (Method::GET, "/api/datasources") => Ok(Some(json!([
                    {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus", "access": "proxy", "url": "http://prometheus:9090"}
                ]))),
                (Method::GET, "/api/v1/provisioning/alert-rules") => Ok(Some(json!([
                    {
                        "uid": "cpu-high",
                        "title": "CPU High",
                        "folderUID": "general",
                        "ruleGroup": "CPU Alerts",
                        "condition": "A",
                        "data": [{"refId": "A"}]
                    }
                ]))),
                (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([
                    {
                        "uid": "cp-main",
                        "name": "PagerDuty Primary",
                        "type": "webhook",
                        "settings": {"url": "http://127.0.0.1/notify"}
                    }
                ]))),
                (Method::GET, "/api/v1/provisioning/mute-timings") => Ok(Some(json!([
                    {
                        "name": "Off Hours",
                        "time_intervals": [{"times": [{"start_time": "00:00", "end_time": "06:00"}]}]
                    }
                ]))),
                (Method::GET, "/api/v1/provisioning/policies") => Ok(Some(json!({
                    "receiver": "grafana-default-email"
                }))),
                (Method::GET, "/api/v1/provisioning/templates") => Ok(Some(json!([
                    {"name": "slack.default"}
                ]))),
                (Method::GET, "/api/v1/provisioning/templates/slack.default") => Ok(Some(json!({
                    "name": "slack.default",
                    "template": "{{ define \"slack.default\" }}ok{{ end }}"
                }))),
                _ => Err(crate::common::message(format!("unexpected {method} {path}"))),
            }
        },
        500,
    )
    .unwrap();

    assert!(specs.iter().any(|item| item["kind"] == "folder"));
    assert!(specs.iter().any(|item| item["kind"] == "dashboard"));
    assert!(specs.iter().any(|item| item["kind"] == "datasource"));
    assert!(specs.iter().any(|item| item["kind"] == "alert"));
    assert!(specs
        .iter()
        .any(|item| item["kind"] == "alert-contact-point" && item["uid"] == "cp-main"));
    assert!(specs
        .iter()
        .any(|item| item["kind"] == "alert-mute-timing" && item["name"] == "Off Hours"));
    assert!(specs
        .iter()
        .any(|item| item["kind"] == "alert-policy" && item["title"] == "grafana-default-email"));
    assert!(specs
        .iter()
        .any(|item| item["kind"] == "alert-template" && item["name"] == "slack.default"));
    assert!(calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/v1/provisioning/alert-rules"));
    assert!(calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/v1/provisioning/contact-points"));
    assert!(calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/v1/provisioning/templates/slack.default"));
}

#[test]
fn fetch_live_availability_with_request_collects_contact_points_and_plugins() {
    let availability =
        fetch_live_availability_with_request(|method, path, _, _| match (method, path) {
            (Method::GET, "/api/datasources") => Ok(Some(json!([
                {"uid": "prom-main", "name": "Prometheus Main"}
            ]))),
            (Method::GET, "/api/plugins") => Ok(Some(json!([
                {"id": "prometheus"}
            ]))),
            (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([
                {"uid": "cp-1", "name": "pagerduty-primary"}
            ]))),
            _ => Err(crate::common::message("unexpected request")),
        })
        .unwrap();

    assert_eq!(availability["datasourceUids"], json!(["prom-main"]));
    assert_eq!(availability["pluginIds"], json!(["prometheus"]));
    assert_eq!(
        availability["contactPoints"],
        json!(["pagerduty-primary", "cp-1"])
    );
}

#[test]
fn execute_live_apply_with_request_supports_alert_create() {
    let mut calls = Vec::new();
    let result = execute_live_apply_with_request(
        |method, path, _, payload| {
            calls.push((method.clone(), path.to_string(), payload.cloned()));
            match (method, path) {
                (Method::POST, "/api/v1/provisioning/alert-rules") => {
                    Ok(Some(json!({"uid": "cpu-high", "status": "created"})))
                }
                _ => Err(crate::common::message("unexpected request")),
            }
        },
        &[json!({
            "kind": "alert",
            "identity": "cpu-high",
            "action": "would-create",
            "desired": {
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "CPU Alerts",
                "condition": "A",
                "data": [{"refId": "A"}]
            }
        })],
        false,
    )
    .unwrap();

    assert_eq!(result["mode"], json!("live-apply"));
    assert_eq!(result["appliedCount"], json!(1));
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, Method::POST);
    assert_eq!(calls[0].1, "/api/v1/provisioning/alert-rules");
}

#[test]
fn execute_live_apply_with_request_supports_non_rule_alert_resources() {
    let mut calls = Vec::new();
    let result = execute_live_apply_with_request(
        |method, path, _, payload| {
            calls.push((method.clone(), path.to_string(), payload.cloned()));
            match (method, path) {
                (Method::PUT, "/api/v1/provisioning/contact-points/cp-main") => {
                    Ok(Some(json!({"uid": "cp-main", "status": "updated"})))
                }
                (Method::PUT, "/api/v1/provisioning/mute-timings/Off Hours") => {
                    Ok(Some(json!({"name": "Off Hours", "status": "updated"})))
                }
                (Method::PUT, "/api/v1/provisioning/policies") => Ok(Some(
                    json!({"receiver": "grafana-default-email", "status": "updated"}),
                )),
                (Method::PUT, "/api/v1/provisioning/templates/slack.default") => {
                    Ok(Some(json!({"name": "slack.default", "status": "updated"})))
                }
                _ => Err(crate::common::message("unexpected request")),
            }
        },
        &[
            json!({
                "kind": "alert-contact-point",
                "identity": "cp-main",
                "action": "would-update",
                "desired": {
                    "uid": "cp-main",
                    "name": "PagerDuty Primary",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/notify"}
                }
            }),
            json!({
                "kind": "alert-mute-timing",
                "identity": "Off Hours",
                "action": "would-update",
                "desired": {
                    "name": "Off Hours",
                    "time_intervals": [{"times": [{"start_time": "00:00", "end_time": "06:00"}]}]
                }
            }),
            json!({
                "kind": "alert-policy",
                "identity": "grafana-default-email",
                "action": "would-update",
                "desired": {
                    "receiver": "grafana-default-email"
                }
            }),
            json!({
                "kind": "alert-template",
                "identity": "slack.default",
                "action": "would-update",
                "desired": {
                    "name": "slack.default",
                    "template": "{{ define \"slack.default\" }}ok{{ end }}"
                }
            }),
        ],
        false,
    )
    .unwrap();

    assert_eq!(result["mode"], json!("live-apply"));
    assert_eq!(result["appliedCount"], json!(4));
    assert!(calls.iter().any(|(method, path, _)| *method == Method::PUT
        && path == "/api/v1/provisioning/contact-points/cp-main"));
    assert!(calls.iter().any(|(method, path, _)| *method == Method::PUT
        && path == "/api/v1/provisioning/mute-timings/Off Hours"));
    assert!(
        calls
            .iter()
            .any(|(method, path, _)| *method == Method::PUT
                && path == "/api/v1/provisioning/policies")
    );
    assert!(calls
        .iter()
        .any(|(method, path, payload)| *method == Method::PUT
            && path == "/api/v1/provisioning/templates/slack.default"
            && payload
                .as_ref()
                .and_then(|value| value.get("name"))
                .is_none()));
}

#[test]
fn execute_live_apply_with_request_supports_non_rule_alert_deletes() {
    let mut calls = Vec::new();
    let result = execute_live_apply_with_request(
        |method, path, params, _| {
            calls.push((method.clone(), path.to_string(), params.to_vec()));
            match (method, path) {
                (Method::DELETE, "/api/v1/provisioning/contact-points/cp-main") => Ok(None),
                (Method::DELETE, "/api/v1/provisioning/mute-timings/Off Hours") => Ok(None),
                (Method::DELETE, "/api/v1/provisioning/templates/slack.default") => Ok(None),
                _ => Err(crate::common::message("unexpected request")),
            }
        },
        &[
            json!({
                "kind": "alert-contact-point",
                "identity": "cp-main",
                "action": "would-delete"
            }),
            json!({
                "kind": "alert-mute-timing",
                "identity": "Off Hours",
                "action": "would-delete"
            }),
            json!({
                "kind": "alert-template",
                "identity": "slack.default",
                "action": "would-delete"
            }),
        ],
        false,
    )
    .unwrap();

    assert_eq!(result["appliedCount"], json!(3));
    assert!(calls
        .iter()
        .any(|(method, path, _)| *method == Method::DELETE
            && path == "/api/v1/provisioning/contact-points/cp-main"));
    assert!(calls
        .iter()
        .any(|(method, path, params)| *method == Method::DELETE
            && path == "/api/v1/provisioning/mute-timings/Off Hours"
            && params
                .iter()
                .any(|(key, value)| key == "version" && value.is_empty())));
    assert!(calls
        .iter()
        .any(|(method, path, params)| *method == Method::DELETE
            && path == "/api/v1/provisioning/templates/slack.default"
            && params
                .iter()
                .any(|(key, value)| key == "version" && value.is_empty())));
}

#[test]
fn run_sync_cli_summary_accepts_local_desired_file() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    fs::write(
        &desired_file,
        serde_json::to_string_pretty(&json!([
            {"kind":"folder","uid":"ops","title":"Operations"},
            {
                "kind":"alert",
                "uid":"cpu-high",
                "title":"CPU High",
                "managedFields":["condition"],
                "body":{"condition":"A > 90"}
            }
        ]))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Summary(SyncSummaryArgs {
        desired_file,
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn run_sync_cli_plan_accepts_local_inputs() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    let live_file = temp.path().join("live.json");
    fs::write(
        &desired_file,
        serde_json::to_string_pretty(&json!([
            {"kind":"folder","uid":"ops","title":"Operations","body":{"title":"Operations"}},
            {
                "kind":"alert",
                "uid":"cpu-high",
                "title":"CPU High",
                "managedFields":["condition","contactPoints"],
                "body":{"condition":"A > 90","contactPoints":["pagerduty-primary"]}
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(&live_file, "[]").unwrap();

    let result = run_sync_cli(SyncGroupCommand::Plan(SyncPlanArgs {
        desired_file,
        live_file: Some(live_file),
        fetch_live: false,
        common: sync_common_args(),
        org_id: None,
        page_size: 500,
        allow_prune: false,
        output: SyncOutputFormat::Json,
        trace_id: None,
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_assess_alerts_accepts_local_inputs() {
    let temp = tempdir().unwrap();
    let alerts_file = temp.path().join("alerts.json");
    fs::write(
        &alerts_file,
        serde_json::to_string_pretty(&json!([
            {
                "kind": "alert",
                "uid": "cpu-high",
                "managedFields": ["condition", "contactPoints"],
                "body": {
                    "condition": "A > 90",
                    "contactPoints": ["pagerduty-primary"]
                }
            }
        ]))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::AssessAlerts(SyncAssessAlertsArgs {
        alerts_file,
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_bundle_writes_source_bundle_artifact() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp.path().join("dashboards").join("raw");
    let alert_export_dir = temp.path().join("alerts").join("raw");
    fs::create_dir_all(&dashboard_export_dir).unwrap();
    fs::create_dir_all(alert_export_dir.join("rules")).unwrap();
    fs::write(
        dashboard_export_dir.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dashboard_export_dir.join("folders.json"),
        serde_json::to_string_pretty(&json!([
            {"uid": "ops", "title": "Operations", "path": "Operations"}
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dashboard_export_dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([
            {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"}
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir.join("rules").join("cpu-high.json"),
        serde_json::to_string_pretty(&json!({
            "groups": [{
                "name": "CPU Alerts",
                "folderUid": "general",
                "rules": [{
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "condition": "A",
                    "data": [{
                        "refId": "A",
                        "datasourceUid": "prom-main",
                        "model": {
                            "expr": "up",
                            "refId": "A"
                        }
                    }],
                    "for": "5m",
                    "noDataState": "NoData",
                    "execErrState": "Alerting",
                    "annotations": {
                        "__dashboardUid__": "cpu-main",
                        "__panelId__": "1"
                    },
                    "notification_settings": {
                        "receiver": "pagerduty-primary"
                    }
                }]
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let metadata_file = temp.path().join("metadata.json");
    fs::write(
        &metadata_file,
        serde_json::to_string_pretty(&json!({
            "bundleLabel": "smoke-bundle"
        }))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: Some(dashboard_export_dir.clone()),
        alert_export_dir: Some(alert_export_dir.clone()),
        datasource_export_file: None,
        metadata_file: Some(metadata_file.clone()),
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["kind"], json!("grafana-utils-sync-source-bundle"));
    assert_eq!(bundle["summary"]["dashboardCount"], json!(1));
    assert_eq!(bundle["summary"]["datasourceCount"], json!(1));
    assert_eq!(bundle["summary"]["folderCount"], json!(1));
    assert_eq!(bundle["summary"]["alertRuleCount"], json!(1));
    assert_eq!(bundle["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["alerts"][0]["kind"], json!("alert"));
    assert_eq!(bundle["alerts"][0]["uid"], json!("cpu-high"));
    assert_eq!(bundle["alerts"][0]["title"], json!("CPU High"));
    assert_eq!(
        bundle["alerts"][0]["managedFields"],
        json!([
            "condition",
            "annotations",
            "contactPoints",
            "datasourceUids",
            "data"
        ])
    );
    assert_eq!(bundle["alerts"][0]["body"]["condition"], json!("A"));
    assert_eq!(
        bundle["alerts"][0]["body"]["contactPoints"],
        json!(["pagerduty-primary"])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["datasourceUids"],
        json!(["prom-main"])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["annotations"]["__dashboardUid__"],
        json!("cpu-main")
    );
    assert_eq!(bundle["metadata"]["bundleLabel"], json!("smoke-bundle"));
    assert_eq!(
        bundle["metadata"]["dashboardExportDir"],
        json!(dashboard_export_dir.display().to_string())
    );
    assert_eq!(
        bundle["alerting"]["exportDir"],
        json!(alert_export_dir.display().to_string())
    );
    assert_eq!(bundle["alerting"]["summary"]["ruleCount"], json!(1));
    assert_eq!(bundle["alerting"]["summary"]["contactPointCount"], json!(0));
    assert_eq!(bundle["alerting"]["summary"]["policyCount"], json!(0));
    assert_eq!(
        bundle["metadata"]["alertExportDir"],
        json!(alert_export_dir.display().to_string())
    );
}

#[test]
fn run_sync_cli_bundle_preserves_alert_export_artifact_metadata() {
    let temp = tempdir().unwrap();
    let alert_export_dir = temp.path().join("alerts").join("raw");
    fs::create_dir_all(
        alert_export_dir
            .join("contact-points")
            .join("Smoke_Webhook"),
    )
    .unwrap();
    fs::create_dir_all(alert_export_dir.join("mute-timings")).unwrap();
    fs::create_dir_all(alert_export_dir.join("policies")).unwrap();
    fs::create_dir_all(alert_export_dir.join("templates")).unwrap();
    fs::write(
        alert_export_dir
            .join("contact-points")
            .join("Smoke_Webhook")
            .join("Smoke_Webhook__smoke-webhook.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-contact-point",
            "apiVersion": 1,
            "schemaVersion": 1,
            "spec": {
                "uid": "smoke-webhook",
                "name": "Smoke Webhook",
                "type": "webhook",
                "settings": {"url": "http://127.0.0.1/notify"}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir.join("contact-points").join("index.json"),
        serde_json::to_string_pretty(&json!([
            {
                "kind": "grafana-contact-point",
                "uid": "smoke-webhook",
                "name": "Smoke Webhook",
                "type": "webhook",
                "path": "contact-points/Smoke_Webhook/Smoke_Webhook__smoke-webhook.json"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir.join("mute-timings").join("Off_Hours.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-mute-timing",
            "apiVersion": 1,
            "schemaVersion": 1,
            "spec": {
                "name": "Off Hours",
                "time_intervals": [{
                    "times": [{
                        "start_time": "00:00",
                        "end_time": "06:00"
                    }]
                }]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir
            .join("policies")
            .join("notification-policies.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-notification-policies",
            "apiVersion": 1,
            "schemaVersion": 1,
            "spec": {"receiver": "grafana-default-email"}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir
            .join("templates")
            .join("slack.default.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-notification-template",
            "apiVersion": 1,
            "schemaVersion": 1,
            "spec": {
                "name": "slack.default",
                "template": "{{ define \"slack.default\" }}ok{{ end }}"
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-util-alert-export-index",
            "apiVersion": 1,
            "schemaVersion": 1
        }))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: None,
        alert_export_dir: Some(alert_export_dir.clone()),
        datasource_export_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["alerting"]["summary"]["contactPointCount"], json!(1));
    assert_eq!(bundle["alerting"]["summary"]["muteTimingCount"], json!(1));
    assert_eq!(bundle["alerting"]["summary"]["policyCount"], json!(1));
    assert_eq!(bundle["alerting"]["summary"]["templateCount"], json!(1));
    assert_eq!(bundle["alerts"].as_array().unwrap().len(), 4);
    assert!(bundle["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point" && item["uid"] == "smoke-webhook"));
    assert!(bundle["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-mute-timing" && item["name"] == "Off Hours"));
    assert!(bundle["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-policy" && item["title"] == "grafana-default-email"));
    assert!(bundle["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-template" && item["name"] == "slack.default"));
    assert_eq!(
        bundle["alerting"]["exportMetadata"]["kind"],
        json!("grafana-util-alert-export-index")
    );
    assert_eq!(
        bundle["alerting"]["muteTimings"][0]["sourcePath"],
        json!("mute-timings/Off_Hours.json")
    );
    assert_eq!(
        bundle["alerting"]["contactPoints"][0]["sourcePath"],
        json!("contact-points/Smoke_Webhook/Smoke_Webhook__smoke-webhook.json")
    );
    assert_eq!(
        bundle["alerting"]["policies"][0]["sourcePath"],
        json!("policies/notification-policies.json")
    );
    assert_eq!(
        bundle["alerting"]["templates"][0]["sourcePath"],
        json!("templates/slack.default.json")
    );
}

#[test]
fn run_sync_cli_bundle_ignores_dashboard_permissions_bundle() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp.path().join("dashboards").join("raw");
    fs::create_dir_all(&dashboard_export_dir).unwrap();
    fs::write(
        dashboard_export_dir.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dashboard_export_dir.join("permissions.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "dashboard-permissions",
            "permissions": [{
                "uid": "cpu-main",
                "role": "Viewer"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: Some(dashboard_export_dir.clone()),
        alert_export_dir: None,
        datasource_export_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok(), "{result:?}");
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["summary"]["dashboardCount"], json!(1));
    assert_eq!(bundle["dashboards"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["dashboards"][0]["uid"], json!("cpu-main"));
}

#[test]
fn run_sync_cli_bundle_preserves_datasource_provider_metadata_from_inventory_file() {
    let temp = tempdir().unwrap();
    let datasource_export_file = temp.path().join("datasources.json");
    fs::write(
        &datasource_export_file,
        serde_json::to_string_pretty(&json!([
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "secureJsonDataProviders": {
                    "httpHeaderValue1": "${provider:vault:secret/data/loki/token}"
                },
                "secureJsonDataPlaceholders": {
                    "basicAuthPassword": "${secret:loki-basic-auth}"
                }
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: None,
        alert_export_dir: None,
        datasource_export_file: Some(datasource_export_file.clone()),
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["summary"]["datasourceCount"], json!(1));
    assert_eq!(
        bundle["metadata"]["datasourceExportFile"],
        json!(datasource_export_file.display().to_string())
    );
    assert_eq!(
        bundle["datasources"][0]["secureJsonDataProviders"]["httpHeaderValue1"],
        json!("${provider:vault:secret/data/loki/token}")
    );
    assert_eq!(
        bundle["datasources"][0]["secureJsonDataPlaceholders"]["basicAuthPassword"],
        json!("${secret:loki-basic-auth}")
    );
}

#[test]
fn run_sync_cli_bundle_normalizes_tool_rule_export_into_top_level_alert_spec() {
    let temp = tempdir().unwrap();
    let alert_export_dir = temp.path().join("alerts").join("raw");
    fs::create_dir_all(alert_export_dir.join("rules")).unwrap();
    fs::write(
        alert_export_dir.join("rules").join("cpu-high.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "apiVersion": 1,
            "kind": "grafana-alert-rule",
            "metadata": {
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "CPU Alerts"
            },
            "spec": {
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "CPU Alerts",
                "condition": "A",
                "data": [{
                    "refId": "A",
                    "datasourceUid": "prom-main",
                    "model": {
                        "datasource": {
                            "uid": "prom-main",
                            "name": "Prometheus Main",
                            "type": "prometheus"
                        },
                        "expr": "up",
                        "refId": "A"
                    }
                }],
                "notificationSettings": {
                    "receiver": "pagerduty-primary"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: None,
        alert_export_dir: Some(alert_export_dir.clone()),
        datasource_export_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["summary"]["alertRuleCount"], json!(1));
    assert_eq!(bundle["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(
        bundle["alerts"][0]["managedFields"],
        json!([
            "condition",
            "contactPoints",
            "datasourceUids",
            "datasourceNames",
            "pluginIds",
            "data"
        ])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["contactPoints"],
        json!(["pagerduty-primary"])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["datasourceNames"],
        json!(["Prometheus Main"])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["pluginIds"],
        json!(["prometheus"])
    );
    assert_eq!(
        bundle["metadata"]["alertExportDir"],
        json!(alert_export_dir.display().to_string())
    );
}

#[test]
fn run_sync_cli_review_marks_plan_reviewed() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-review",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": false
        }))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Review(SyncReviewArgs {
        plan_file,
        review_token: DEFAULT_REVIEW_TOKEN.to_string(),
        output: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_review_rejects_wrong_review_token() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-review",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": false
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Review(SyncReviewArgs {
        plan_file,
        review_token: "wrong-token".to_string(),
        output: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("review token rejected"));
}

#[test]
fn run_sync_cli_review_rejects_missing_trace_id() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": false
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Review(SyncReviewArgs {
        plan_file,
        review_token: DEFAULT_REVIEW_TOKEN.to_string(),
        output: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("missing traceId"));
}

#[test]
fn run_sync_cli_review_rejects_partial_lineage_metadata() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-review",
            "stage": "plan",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": false
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Review(SyncReviewArgs {
        plan_file,
        review_token: DEFAULT_REVIEW_TOKEN.to_string(),
        output: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("missing lineage stepIndex metadata"));
}

#[test]
fn run_sync_cli_review_rejects_non_plan_lineage_stage() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-review",
            "stage": "apply",
            "stepIndex": 3,
            "parentTraceId": "sync-trace-review",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": false
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Review(SyncReviewArgs {
        plan_file,
        review_token: DEFAULT_REVIEW_TOKEN.to_string(),
        output: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("unexpected lineage stage"));
}

#[test]
fn run_sync_cli_review_rejects_plan_with_wrong_lineage_stage() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-review",
            "stage": "apply",
            "stepIndex": 3,
            "parentTraceId": "sync-trace-review",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": false
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Review(SyncReviewArgs {
        plan_file,
        review_token: DEFAULT_REVIEW_TOKEN.to_string(),
        output: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("unexpected lineage stage"));
}

#[test]
fn run_sync_cli_apply_accepts_reviewed_plan_file() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 1,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"},
                {"kind":"folder","identity":"old","action":"noop"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_apply_rejects_reviewed_plan_with_wrong_lineage_parent() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "other-trace",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 1,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"},
                {"kind":"folder","identity":"old","action":"noop"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("unexpected lineage parentTraceId"));
}

#[test]
fn run_sync_cli_apply_rejects_unreviewed_plan_file() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": false,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("marked reviewed"));
}

#[test]
fn run_sync_cli_apply_requires_explicit_approval() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: None,
        approve: false,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("explicit approval"));
}

#[test]
fn run_sync_cli_apply_accepts_non_blocking_preflight_file() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let preflight_file = temp.path().join("preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-preflight",
            "summary": {
                "checkCount": 3,
                "okCount": 3,
                "blockingCount": 0
            },
            "checks": []
        }))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: Some(preflight_file),
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_apply_rejects_preflight_with_mismatched_trace_id() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let preflight_file = temp.path().join("preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-preflight",
            "traceId": "other-trace",
            "summary": {
                "checkCount": 3,
                "okCount": 3,
                "blockingCount": 0
            },
            "checks": []
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: Some(preflight_file),
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("does not match sync plan traceId"));
}

#[test]
fn run_sync_cli_plan_accepts_explicit_trace_id() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    let live_file = temp.path().join("live.json");
    fs::write(
        &desired_file,
        serde_json::to_string_pretty(&json!([
            {"kind":"folder","uid":"ops","title":"Operations","body":{"title":"Operations"}}
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(&live_file, "[]").unwrap();

    let result = run_sync_cli(SyncGroupCommand::Plan(SyncPlanArgs {
        desired_file,
        live_file: Some(live_file),
        fetch_live: false,
        common: sync_common_args(),
        org_id: None,
        page_size: 500,
        allow_prune: false,
        output: SyncOutputFormat::Json,
        trace_id: Some("trace-explicit".to_string()),
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_apply_rejects_blocking_preflight_file() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let preflight_file = temp.path().join("preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-preflight",
            "summary": {
                "checkCount": 3,
                "okCount": 1,
                "blockingCount": 2
            },
            "checks": []
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: Some(preflight_file),
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("preflight reports 2 blocking checks"));
}

#[test]
fn run_sync_cli_apply_rejects_blocking_bundle_preflight_file() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let bundle_preflight_file = temp.path().join("bundle-preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &bundle_preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 1,
                "providerBlockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("bundle preflight reports 1 blocking checks"));
}

#[test]
fn run_sync_cli_apply_rejects_bundle_preflight_with_blocked_alert_artifacts() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let bundle_preflight_file = temp.path().join("bundle-preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &bundle_preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0
            },
            "alertArtifactAssessment": {
                "summary": {
                    "blockedCount": 2
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("bundle preflight reports 2 blocking checks"));
}

#[test]
fn run_sync_cli_apply_rejects_missing_trace_id() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("missing traceId"));
}

#[test]
fn run_sync_cli_apply_rejects_plan_with_non_review_lineage() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "stage": "plan",
            "stepIndex": 1,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("unexpected lineage stage"));
}

#[test]
fn run_sync_cli_apply_accepts_non_blocking_bundle_preflight_file() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let bundle_preflight_file = temp.path().join("bundle-preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &bundle_preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }));

    assert!(result.is_ok());
}

#[test]
fn render_sync_apply_intent_text_includes_alert_artifact_bundle_counts() {
    let lines = render_sync_apply_intent_text(&json!({
        "kind": "grafana-utils-sync-apply-intent",
        "stage": "apply",
        "stepIndex": 3,
        "traceId": "sync-trace-demo",
        "parentTraceId": "sync-trace-demo",
        "mode": "apply",
        "reviewed": true,
        "reviewRequired": true,
        "allowPrune": false,
        "approved": true,
        "summary": {
            "would_create": 1,
            "would_update": 0,
            "would_delete": 0,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 0
        },
        "operations": [],
        "bundlePreflightSummary": {
            "resourceCount": 4,
            "syncBlockingCount": 0,
            "providerBlockingCount": 0,
            "alertArtifactCount": 4,
            "alertArtifactPlanOnlyCount": 1,
            "alertArtifactBlockingCount": 3
        }
    }))
    .unwrap();

    let output = lines.join("\n");
    assert!(output.contains("alert-artifacts=4"));
    assert!(output.contains("plan-only=1"));
    assert!(output.contains("blocking=3"));
}

#[test]
fn run_sync_cli_apply_rejects_lineage_aware_preflight_without_trace_id() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let preflight_file = temp.path().join("preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-preflight",
            "stage": "preflight",
            "stepIndex": 2,
            "summary": {
                "checkCount": 3,
                "okCount": 3,
                "blockingCount": 0
            },
            "checks": []
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: Some(preflight_file),
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("missing traceId for lineage-aware staged validation"));
}

#[test]
fn run_sync_cli_apply_rejects_lineage_aware_bundle_preflight_with_mismatched_parent() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let bundle_preflight_file = temp.path().join("bundle-preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &bundle_preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "traceId": "sync-trace-apply",
            "stage": "bundle-preflight",
            "stepIndex": 2,
            "parentTraceId": "other-trace",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("parentTraceId"));
    assert!(error.contains("does not match sync plan traceId"));
}

#[test]
fn run_sync_cli_apply_rejects_bundle_preflight_with_mismatched_trace_id() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    let bundle_preflight_file = temp.path().join("bundle-preflight.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &bundle_preflight_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "traceId": "other-trace",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("does not match sync plan traceId"));
}

#[test]
fn run_sync_cli_review_accepts_explicit_audit_metadata() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-review",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": false
        }))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Review(SyncReviewArgs {
        plan_file,
        review_token: DEFAULT_REVIEW_TOKEN.to_string(),
        output: SyncOutputFormat::Json,
        reviewed_by: Some("alice".to_string()),
        reviewed_at: Some("manual-review".to_string()),
        review_note: Some("peer-reviewed".to_string()),
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_apply_accepts_explicit_audit_metadata() {
    let temp = tempdir().unwrap();
    let plan_file = temp.path().join("plan.json");
    fs::write(
        &plan_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-plan",
            "traceId": "sync-trace-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 1,
                "unmanaged": 0,
                "alert_candidate": 0,
                "alert_plan_only": 0,
                "alert_blocked": 0
            },
            "reviewRequired": true,
            "reviewed": true,
            "operations": [
                {"kind":"folder","identity":"ops","action":"would-create"},
                {"kind":"folder","identity":"old","action":"noop"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file,
        preflight_file: None,
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        output: SyncOutputFormat::Json,
        applied_by: Some("bob".to_string()),
        applied_at: Some("manual-apply".to_string()),
        approval_reason: Some("approved-change".to_string()),
        apply_note: Some("staged only".to_string()),
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_preflight_rejects_non_object_availability_file() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    let availability_file = temp.path().join("availability.json");
    fs::write(
        &desired_file,
        serde_json::to_string_pretty(&json!([
            {"kind":"folder","uid":"ops","title":"Operations"}
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(&availability_file, "[]").unwrap();

    let error = run_sync_cli(SyncGroupCommand::Preflight(SyncPreflightArgs {
        desired_file,
        availability_file: Some(availability_file),
        fetch_live: false,
        common: sync_common_args(),
        org_id: None,
        output: SyncOutputFormat::Text,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("Sync availability input file must contain a JSON object"));
}

#[test]
fn run_sync_cli_bundle_preflight_accepts_local_bundle_inputs() {
    let temp = tempdir().unwrap();
    let source_bundle = temp.path().join("source.json");
    let target_inventory = temp.path().join("target.json");
    fs::write(
        &source_bundle,
        serde_json::to_string_pretty(&json!({
            "dashboards": [],
            "datasources": [],
            "folders": [{"kind":"folder","uid":"ops","title":"Operations"}],
            "alerts": []
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &target_inventory,
        serde_json::to_string_pretty(&json!({})).unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::BundlePreflight(SyncBundlePreflightArgs {
        source_bundle,
        target_inventory,
        availability_file: None,
        fetch_live: false,
        common: sync_common_args(),
        org_id: None,
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
}
