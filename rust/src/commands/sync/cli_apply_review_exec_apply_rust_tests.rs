//! Sync CLI apply/preflight regression tests.
//! Covers blocking and non-blocking preflight gates for apply execution.
use super::super::{run_sync_cli, SyncApplyArgs, SyncGroupCommand, SyncOutputFormat};
use super::sync_common_args;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

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
            "stage": "preflight",
            "stepIndex": 2,
            "traceId": "sync-trace-apply",
            "summary": {
                "checkCount": 3,
                "okCount": 3,
                "blockingCount": 0
            },
            "checks": [
                {"kind":"resource","name":"ok-1","state":"ok"},
                {"kind":"resource","name":"ok-2","state":"ok"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: Some(preflight_file),
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
fn run_sync_cli_apply_rejects_preflight_with_mismatched_trace_id() {
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
            "traceId": "other-trace",
            "summary": {
                "checkCount": 3,
                "okCount": 3,
                "blockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: Some(preflight_file),
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

    assert!(error.contains("does not match sync plan traceId"));
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
            "traceId": "sync-trace-apply",
            "summary": {
                "checkCount": 2,
                "okCount": 1,
                "blockingCount": 1
            },
            "checks": [
                {"kind":"resource","identity":"ok-1","status":"ok","blocking":false},
                {"kind":"dashboard-plugin","identity":"cpu-main->geomap","status":"missing","detail":"Dashboard plugin dependency is missing.","blocking":true}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: Some(preflight_file),
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

    assert!(error.contains("preflight reports 1 blocking checks"));
    assert!(error.contains("dashboard-plugin"));
    assert!(error.contains("Dashboard plugin dependency is missing."));
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
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
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
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0
            },
            "alertArtifactAssessment": {
                "summary": {
                    "blockedCount": 2
                },
                "checks": [
                    {
                        "kind": "alert-policy",
                        "identity": "grafana-default-email",
                        "status": "blocked",
                        "detail": "Notification policies are staged in the workspace package but are not live-wired yet.",
                        "blocking": true
                    },
                    {
                        "kind": "alert-template",
                        "identity": "slack.default",
                        "status": "blocked",
                        "detail": "Templates are staged in the workspace package but are not live-wired yet.",
                        "blocking": true
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
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

    assert!(error.contains("bundle preflight reports 2 blocking checks"));
    assert!(error.contains("source=alertArtifactAssessment"));
    assert!(error.contains("Notification policies are staged in the workspace package"));
}

#[test]
fn run_sync_cli_apply_rejects_bundle_preflight_with_missing_secret_placeholder_availability() {
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
                "resourceCount": 1,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 1
            },
            "secretPlaceholderAssessment": {
                "summary": {
                    "blockingCount": 1
                },
                "checks": [
                    {
                        "kind": "secret-placeholder",
                        "identity": "loki-main->loki-basic-auth",
                        "status": "missing",
                        "detail": "Datasource secret placeholder is not available in explicit availability input.",
                        "blocking": true
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
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

    assert!(error.contains("bundle preflight reports 1 blocking checks"));
    assert!(error.contains("source=secretPlaceholderAssessment"));
    assert!(error.contains("Datasource secret placeholder is not available"));
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
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
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
        plan_file: Some(plan_file),
        preflight_file: Some(preflight_file),
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
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
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
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
        plan_file: Some(plan_file),
        preflight_file: None,
        bundle_preflight_file: Some(bundle_preflight_file),
        approve: true,
        common: super::sync_common_args(),
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

    assert!(error.contains("does not match sync plan traceId"));
}
