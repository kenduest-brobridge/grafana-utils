//! Sync CLI review/apply regression test suite.
//! Verifies lineage and approval gating for staged review/apply flows.
use super::{run_sync_cli, SyncApplyArgs, SyncGroupCommand, SyncOutputFormat, SyncReviewArgs};
use crate::dashboard::CommonCliArgs;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

fn sync_common_args() -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: None,
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
        review_token: super::DEFAULT_REVIEW_TOKEN.to_string(),
        output_format: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
        interactive: false,
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
        review_token: super::DEFAULT_REVIEW_TOKEN.to_string(),
        output_format: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
        interactive: false,
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
        review_token: super::DEFAULT_REVIEW_TOKEN.to_string(),
        output_format: SyncOutputFormat::Json,
        reviewed_by: None,
        reviewed_at: None,
        review_note: None,
        interactive: false,
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
        allow_policy_reset: false,
        output_format: SyncOutputFormat::Json,
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
        allow_policy_reset: false,
        output_format: SyncOutputFormat::Text,
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
        allow_policy_reset: false,
        output_format: SyncOutputFormat::Text,
        applied_by: None,
        applied_at: None,
        approval_reason: None,
        apply_note: None,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("explicit approval"));
}
