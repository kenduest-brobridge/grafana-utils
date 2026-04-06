//! Sync CLI review/trace-lineage regression tests.
//! Covers review token handling and lineage validation for review/apply flows.
use super::super::{
    run_sync_cli, SyncApplyArgs, SyncGroupCommand, SyncOutputFormat, SyncPlanArgs, SyncReviewArgs,
    DEFAULT_REVIEW_TOKEN,
};
use super::sync_common_args;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

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
        output_format: SyncOutputFormat::Json,
        reviewed_by: Some("alice".to_string()),
        reviewed_at: Some("manual-review".to_string()),
        review_note: Some("peer-reviewed".to_string()),
        interactive: false,
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
        output_format: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
        interactive: false,
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
        output_format: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
        interactive: false,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("missing traceId"));
}

#[test]
fn run_sync_cli_plan_accepts_explicit_trace_id() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    let live_file = temp.path().join("live.json");
    fs::write(&desired_file, "[]").unwrap();
    fs::write(&live_file, "[]").unwrap();

    let result = run_sync_cli(SyncGroupCommand::Plan(SyncPlanArgs {
        desired_file,
        live_file: Some(live_file),
        fetch_live: false,
        common: sync_common_args(),
        org_id: None,
        page_size: 500,
        allow_prune: false,
        output_format: SyncOutputFormat::Json,
        trace_id: Some("plan-trace-123".to_string()),
    }));

    assert!(result.is_ok());
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
        plan_file: Some(plan_file),
        preflight_file: None,
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        allow_policy_reset: false,
        output_format: SyncOutputFormat::Json,
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

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: None,
        bundle_preflight_file: None,
        approve: true,
        common: sync_common_args(),
        org_id: None,
        execute_live: false,
        allow_folder_delete: false,
        allow_policy_reset: false,
        output_format: SyncOutputFormat::Json,
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
        output_format: SyncOutputFormat::Json,
        reviewed_by: Some("alice".to_string()),
        reviewed_at: Some("manual-review".to_string()),
        review_note: Some("peer-reviewed".to_string()),
        interactive: false,
    }));

    assert!(result.is_ok());
}
