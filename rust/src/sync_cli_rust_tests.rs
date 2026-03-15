use super::{
    build_sync_live_apply_text, render_sync_apply_intent_text, render_sync_plan_text,
    render_sync_summary_text, run_sync_apply_operations, run_sync_cli, SyncApplyArgs, SyncCliArgs,
    SyncGroupCommand, SyncOutputFormat, SyncReviewArgs, SyncSummaryArgs, DEFAULT_REVIEW_TOKEN,
};
use crate::sync_bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
};
use crate::sync_preflight::{build_sync_preflight_document, render_sync_preflight_text};
use clap::Parser;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn load_sync_preflight_cases() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/rust_sync_preflight_cases.json"))
        .unwrap()
}

fn sync_preflight_case(name: &str) -> Value {
    load_sync_preflight_cases()["preflightCases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .cloned()
        .unwrap_or_else(|| panic!("missing sync preflight case {name}"))
}

fn sync_bundle_preflight_case(name: &str) -> Value {
    load_sync_preflight_cases()["bundlePreflightCases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .cloned()
        .unwrap_or_else(|| panic!("missing sync bundle-preflight case {name}"))
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
            assert!(!inner.fetch_live);
            assert_eq!(inner.page_size, 500);
            assert_eq!(inner.output, SyncOutputFormat::Json);
            assert_eq!(inner.trace_id, Some("trace-explicit".to_string()));
        }
        _ => panic!("expected plan"),
    }
}

#[test]
fn parse_sync_cli_supports_plan_command_with_fetch_live_and_auth_flags() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "plan",
        "--desired-file",
        "./desired.json",
        "--fetch-live",
        "--url",
        "http://grafana.example.local",
        "--token",
        "abc123",
        "--org-id",
        "3",
        "--page-size",
        "250",
    ]);

    match args.command {
        SyncGroupCommand::Plan(inner) => {
            assert_eq!(inner.desired_file, Path::new("./desired.json"));
            assert_eq!(inner.live_file, None);
            assert!(inner.fetch_live);
            assert_eq!(inner.org_id, Some(3));
            assert_eq!(inner.page_size, 250);
            assert_eq!(inner.common.url, "http://grafana.example.local");
            assert_eq!(inner.common.api_token, Some("abc123".to_string()));
            assert_eq!(inner.output, SyncOutputFormat::Text);
            assert_eq!(inner.trace_id, None);
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
            assert_eq!(inner.applied_by, None);
            assert_eq!(inner.applied_at, None);
            assert_eq!(inner.approval_reason, None);
            assert_eq!(inner.apply_note, None);
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn parse_sync_cli_supports_apply_command_with_live_options() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "apply",
        "--plan-file",
        "./plan.json",
        "--execute-live",
        "--allow-folder-delete",
        "--org-id",
        "3",
        "--url",
        "http://grafana.example.local",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Apply(inner) => {
            assert_eq!(inner.plan_file, Path::new("./plan.json"));
            assert!(inner.execute_live);
            assert!(inner.allow_folder_delete);
            assert_eq!(inner.org_id, Some(3));
            assert_eq!(inner.common.url, "http://grafana.example.local");
            assert_eq!(inner.output, SyncOutputFormat::Json);
            assert!(!inner.approve);
            assert!(!inner.continue_on_error);
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn parse_sync_cli_supports_apply_command_with_continue_on_error() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "apply",
        "--plan-file",
        "./plan.json",
        "--execute-live",
        "--continue-on-error",
        "--url",
        "http://grafana.example.local",
    ]);

    match args.command {
        SyncGroupCommand::Apply(inner) => {
            assert!(inner.execute_live);
            assert!(inner.continue_on_error);
            assert_eq!(inner.common.url, "http://grafana.example.local");
        }
        _ => panic!("expected apply"),
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
fn parse_sync_cli_supports_preflight_command_with_trace_id() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "preflight",
        "--desired-file",
        "./desired.json",
        "--availability-file",
        "./availability.json",
        "--trace-id",
        "trace-preflight",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::Preflight(inner) => {
            assert_eq!(inner.desired_file, Path::new("./desired.json"));
            assert!(!inner.fetch_live);
            assert_eq!(
                inner.availability_file,
                Some(Path::new("./availability.json").to_path_buf())
            );
            assert_eq!(inner.trace_id, Some("trace-preflight".to_string()));
            assert_eq!(inner.output, SyncOutputFormat::Json);
        }
        _ => panic!("expected preflight"),
    }
}

#[test]
fn parse_sync_cli_supports_preflight_command_with_fetch_live() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "preflight",
        "--desired-file",
        "./desired.json",
        "--fetch-live",
        "--url",
        "http://grafana.example.local",
    ]);

    match args.command {
        SyncGroupCommand::Preflight(inner) => {
            assert_eq!(inner.desired_file, Path::new("./desired.json"));
            assert!(inner.fetch_live);
            assert_eq!(inner.availability_file, None);
            assert_eq!(inner.common.url, "http://grafana.example.local");
        }
        _ => panic!("expected preflight"),
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
fn parse_sync_cli_supports_bundle_preflight_command_with_trace_id() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "bundle-preflight",
        "--source-bundle",
        "./source.json",
        "--target-inventory",
        "./target.json",
        "--availability-file",
        "./availability.json",
        "--trace-id",
        "trace-bundle-preflight",
        "--output",
        "json",
    ]);

    match args.command {
        SyncGroupCommand::BundlePreflight(inner) => {
            assert_eq!(inner.source_bundle, Path::new("./source.json"));
            assert_eq!(inner.target_inventory, Path::new("./target.json"));
            assert!(!inner.fetch_live);
            assert_eq!(
                inner.availability_file,
                Some(Path::new("./availability.json").to_path_buf())
            );
            assert_eq!(inner.trace_id, Some("trace-bundle-preflight".to_string()));
            assert_eq!(inner.output, SyncOutputFormat::Json);
        }
        _ => panic!("expected bundle-preflight"),
    }
}

#[test]
fn parse_sync_cli_supports_bundle_preflight_command_with_fetch_live() {
    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "bundle-preflight",
        "--source-bundle",
        "./source.json",
        "--target-inventory",
        "./target.json",
        "--fetch-live",
        "--url",
        "http://grafana.example.local",
    ]);

    match args.command {
        SyncGroupCommand::BundlePreflight(inner) => {
            assert_eq!(inner.source_bundle, Path::new("./source.json"));
            assert_eq!(inner.target_inventory, Path::new("./target.json"));
            assert!(inner.fetch_live);
            assert_eq!(inner.availability_file, None);
            assert_eq!(inner.common.url, "http://grafana.example.local");
        }
        _ => panic!("expected bundle-preflight"),
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
fn build_sync_live_apply_text_reports_status_counts_and_errors() {
    let lines = build_sync_live_apply_text(&json!({
        "mode": "live-apply",
        "appliedCount": 1,
        "failedCount": 1,
        "results": [
            {
                "status": "ok",
                "kind": "folder",
                "identity": "ops",
                "action": "would-create",
                "response": {"status":"ok"}
            },
            {
                "status": "error",
                "kind": "dashboard",
                "identity": "bad",
                "action": "would-update",
                "error": "missing identity mapping"
            }
        ]
    }))
    .unwrap();

    assert_eq!(lines[0], "Sync live apply");
    assert_eq!(lines[1], "AppliedCount: 1");
    assert_eq!(lines[2], "FailedCount: 1");
    assert_eq!(lines[3], "folder ops would-create [ok]");
    assert_eq!(
        lines[4],
        "dashboard bad would-update [error] missing identity mapping"
    );
}

#[test]
fn run_sync_apply_operations_fails_fast_without_continue_on_error() {
    let operations = vec![
        json!({"kind":"folder","identity":"ops","action":"would-create","desired":{}}),
        json!({"identity":"bad","action":"would-create","desired":{}}),
        json!({"kind":"folder","identity":"after","action":"would-create","desired":{}}),
    ];
    let mut request_count = 0u32;
    let error = run_sync_apply_operations(
        &operations,
        false,
        false,
        &mut |_method, _path, _params, _payload| {
            request_count += 1;
            Ok(Some(json!({"ok": true})))
        },
        &[],
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("Sync apply operation is missing kind."));
    assert_eq!(request_count, 1);
}

#[test]
fn run_sync_apply_operations_continues_when_enabled() {
    let operations = vec![
        json!({"kind":"folder","identity":"ops","action":"would-create","desired":{}}),
        json!({"identity":"bad","action":"would-create","desired":{}}),
        json!({"kind":"folder","identity":"after","action":"would-create","desired":{}}),
    ];
    let result = run_sync_apply_operations(
        &operations,
        false,
        true,
        &mut |_method, _path, _params, _payload| Ok(Some(json!({"ok": true}))),
        &[],
    )
    .unwrap();

    assert_eq!(result["mode"], json!("live-apply"));
    assert_eq!(result["appliedCount"], json!(2));
    assert_eq!(result["failedCount"], json!(1));
    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["status"], json!("ok"));
    assert_eq!(results[0]["kind"], json!("folder"));
    assert_eq!(results[0]["identity"], json!("ops"));
    assert_eq!(results[1]["status"], json!("error"));
    assert!(results[1]["error"]
        .as_str()
        .unwrap()
        .contains("missing kind"));
    assert_eq!(results[2]["status"], json!("ok"));
    assert_eq!(results[2]["identity"], json!("after"));
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

    assert!(result.is_ok());
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

    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "plan",
        "--desired-file",
        desired_file.to_str().unwrap(),
        "--live-file",
        live_file.to_str().unwrap(),
        "--output",
        "json",
    ]);
    let result = match args.command {
        SyncGroupCommand::Plan(inner) => run_sync_cli(SyncGroupCommand::Plan(inner)),
        _ => panic!("expected plan"),
    };

    assert!(result.is_ok());
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
            "traceId": "sync-trace-apply",
            "stage": "preflight",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_apply_rejects_preflight_with_wrong_lineage_stage() {
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
            "traceId": "sync-trace-apply",
            "stage": "bundle-preflight",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("unexpected lineage stage"));
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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

    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "plan",
        "--desired-file",
        desired_file.to_str().unwrap(),
        "--live-file",
        live_file.to_str().unwrap(),
        "--trace-id",
        "trace-explicit",
        "--output",
        "json",
    ]);
    let result = match args.command {
        SyncGroupCommand::Plan(inner) => run_sync_cli(SyncGroupCommand::Plan(inner)),
        _ => panic!("expected plan"),
    };

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_plan_rejects_missing_live_file_without_fetch_live() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    fs::write(
        &desired_file,
        serde_json::to_string_pretty(&json!([
            {"kind":"folder","uid":"ops","title":"Operations","body":{"title":"Operations"}}
        ]))
        .unwrap(),
    )
    .unwrap();

    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "plan",
        "--desired-file",
        desired_file.to_str().unwrap(),
        "--output",
        "json",
    ]);
    let error = match args.command {
        SyncGroupCommand::Plan(inner) => run_sync_cli(SyncGroupCommand::Plan(inner)).unwrap_err(),
        _ => panic!("expected plan"),
    };

    let message = error.to_string();
    assert!(message.contains("Sync plan requires --live-file unless --fetch-live is used."));
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
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
        output: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("bundle preflight reports 1 blocking checks"));
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
            "traceId": "sync-trace-apply",
            "stage": "bundle-preflight",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
    }));

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_apply_rejects_bundle_preflight_with_wrong_lineage_step() {
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
            "stepIndex": 3,
            "parentTraceId": "sync-trace-apply",
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("unexpected lineage stepIndex"));
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("unexpected lineage parentTraceId"));
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
        ..Default::default()
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
        output: SyncOutputFormat::Json,
        applied_by: Some("bob".to_string()),
        applied_at: Some("manual-apply".to_string()),
        approval_reason: Some("approved-change".to_string()),
        apply_note: Some("staged only".to_string()),
        ..Default::default()
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

    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "preflight",
        "--desired-file",
        desired_file.to_str().unwrap(),
        "--availability-file",
        availability_file.to_str().unwrap(),
        "--output",
        "text",
    ]);
    let error = match args.command {
        SyncGroupCommand::Preflight(inner) => run_sync_cli(SyncGroupCommand::Preflight(inner)),
        _ => panic!("expected preflight"),
    }
    .unwrap_err()
    .to_string();

    assert!(error.contains("Sync availability input file must contain a JSON object"));
}

#[test]
fn run_sync_cli_preflight_fixture_case_renders_dependency_and_policy_summary() {
    let case = sync_preflight_case("dependency_and_policy_summary");
    let desired_specs = case["desiredSpecs"].as_array().unwrap().clone();
    let availability = case["availability"].clone();
    let expected_summary = case["expectedSummary"].as_object().unwrap();
    let expected_text = case["expectedTextSubstrings"].as_array().unwrap();

    let document = build_sync_preflight_document(&desired_specs, Some(&availability)).unwrap();
    let rendered = render_sync_preflight_text(&document).unwrap().join("\n");

    assert_eq!(
        document["summary"]["resourceCount"],
        expected_summary["resourceCount"]
    );
    assert_eq!(
        document["summary"]["checkCount"],
        expected_summary["checkCount"]
    );
    assert_eq!(document["summary"]["okCount"], expected_summary["okCount"]);
    assert_eq!(
        document["summary"]["createPlannedCount"],
        expected_summary["createPlannedCount"]
    );
    assert_eq!(
        document["summary"]["missingCount"],
        expected_summary["missingCount"]
    );
    assert_eq!(
        document["summary"]["blockingCount"],
        expected_summary["blockingCount"]
    );
    assert_eq!(
        document["summary"]["dependencyBlockingCount"],
        expected_summary["dependencyBlockingCount"]
    );
    assert_eq!(
        document["summary"]["policyBlockingCount"],
        expected_summary["policyBlockingCount"]
    );
    assert_eq!(
        document["summary"]["alertPolicyCount"],
        expected_summary["alertPolicyCount"]
    );
    for fragment in expected_text {
        assert!(rendered.contains(fragment.as_str().unwrap()));
    }
}

#[test]
fn run_sync_cli_assess_alerts_accepts_local_input() {
    let temp = tempdir().unwrap();
    let alerts_file = temp.path().join("alerts.json");
    fs::write(
        &alerts_file,
        serde_json::to_string_pretty(&json!([
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition", "contactPoints"],
                "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]}
            }
        ]))
        .unwrap(),
    )
    .unwrap();

    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "assess-alerts",
        "--alerts-file",
        alerts_file.to_str().unwrap(),
        "--output",
        "json",
    ]);
    let result = match args.command {
        SyncGroupCommand::AssessAlerts(inner) => {
            run_sync_cli(SyncGroupCommand::AssessAlerts(inner))
        }
        _ => panic!("expected assess-alerts"),
    };

    assert!(result.is_ok());
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

    let args = SyncCliArgs::parse_from([
        "grafana-util",
        "bundle-preflight",
        "--source-bundle",
        source_bundle.to_str().unwrap(),
        "--target-inventory",
        target_inventory.to_str().unwrap(),
        "--output",
        "json",
    ]);
    let result = match args.command {
        SyncGroupCommand::BundlePreflight(inner) => {
            run_sync_cli(SyncGroupCommand::BundlePreflight(inner))
        }
        _ => panic!("expected bundle-preflight"),
    };

    assert!(result.is_ok());
}

#[test]
fn run_sync_cli_bundle_preflight_fixture_case_renders_sync_and_provider_summary() {
    let case = sync_bundle_preflight_case("sync_and_provider_summary");
    let source_bundle = case["sourceBundle"].clone();
    let target_inventory = case["targetInventory"].clone();
    let availability = case["availability"].clone();
    let expected_summary = case["expectedSummary"].as_object().unwrap();
    let expected_text = case["expectedTextSubstrings"].as_array().unwrap();

    let document = build_sync_bundle_preflight_document(
        &source_bundle,
        &target_inventory,
        Some(&availability),
    )
    .unwrap();
    let rendered = render_sync_bundle_preflight_text(&document)
        .unwrap()
        .join("\n");

    assert_eq!(
        document["summary"]["resourceCount"],
        expected_summary["resourceCount"]
    );
    assert_eq!(
        document["summary"]["syncCheckCount"],
        expected_summary["syncCheckCount"]
    );
    assert_eq!(
        document["summary"]["syncBlockingCount"],
        expected_summary["syncBlockingCount"]
    );
    assert_eq!(
        document["summary"]["syncDependencyBlockingCount"],
        expected_summary["syncDependencyBlockingCount"]
    );
    assert_eq!(
        document["summary"]["syncPolicyBlockingCount"],
        expected_summary["syncPolicyBlockingCount"]
    );
    assert_eq!(
        document["summary"]["alertPolicyCount"],
        expected_summary["alertPolicyCount"]
    );
    assert_eq!(
        document["summary"]["providerBlockingCount"],
        expected_summary["providerBlockingCount"]
    );
    assert_eq!(
        document["summary"]["totalBlockingCount"],
        expected_summary["totalBlockingCount"]
    );
    for fragment in expected_text {
        assert!(rendered.contains(fragment.as_str().unwrap()));
    }
}
