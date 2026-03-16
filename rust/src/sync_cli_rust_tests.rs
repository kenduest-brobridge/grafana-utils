use super::{
    render_sync_apply_intent_text, render_sync_plan_text, render_sync_summary_text, run_sync_cli,
    SyncApplyArgs, SyncBundlePreflightArgs, SyncCliArgs, SyncGroupCommand, SyncOutputFormat,
    SyncPlanArgs, SyncPreflightArgs, SyncReviewArgs, SyncSummaryArgs, DEFAULT_REVIEW_TOKEN,
};
use clap::Parser;
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

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
            assert_eq!(inner.live_file, Path::new("./live.json"));
            assert!(inner.allow_prune);
            assert_eq!(inner.output, SyncOutputFormat::Json);
            assert_eq!(inner.trace_id, Some("trace-explicit".to_string()));
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

    let result = run_sync_cli(SyncGroupCommand::Plan(SyncPlanArgs {
        desired_file,
        live_file,
        allow_prune: false,
        output: SyncOutputFormat::Json,
        trace_id: None,
    }));

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
        live_file,
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
        output: SyncOutputFormat::Json,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }));

    assert!(result.is_ok());
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
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
}
