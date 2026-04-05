//! Sync CLI audit/preflight regression test suite.
//! Verifies audit rendering/TUI contracts and staged preflight validation.
use super::audit::{build_sync_audit_document, render_sync_audit_text};
use super::{
    build_sync_audit_tui_groups, build_sync_audit_tui_rows, run_sync_cli, SyncApplyArgs,
    SyncAuditArgs, SyncGroupCommand, SyncOutputFormat, SyncPreflightArgs,
};
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
fn render_sync_audit_text_reports_drift_summary_and_rows() {
    let lines = render_sync_audit_text(&json!({
        "kind": "grafana-utils-sync-audit",
        "summary": {
            "managedCount": 2,
            "baselineCount": 2,
            "currentPresentCount": 1,
            "currentMissingCount": 1,
            "inSyncCount": 0,
            "driftCount": 2,
            "missingLockCount": 1,
            "missingLiveCount": 1
        },
        "drifts": [
            {"status":"drift-detected","kind":"dashboard","identity":"cpu-main","driftedFields":["title","refresh"]},
            {"status":"missing-live","kind":"datasource","identity":"prom-main","driftedFields":[]}
        ]
    }))
    .unwrap();

    assert_eq!(lines[0], "Sync audit");
    assert!(lines[1].contains("Managed: 2"));
    assert!(lines[2].contains("Drift: count=2"));
    assert!(lines[3].contains("[drift-detected] dashboard cpu-main"));
    assert!(lines[4].contains("[missing-live] datasource prom-main"));
}

#[test]
fn build_sync_audit_tui_groups_summarizes_triage_sections() {
    let audit = json!({
        "summary": {
            "managedCount": 4,
            "baselineCount": 4,
            "currentPresentCount": 3,
            "currentMissingCount": 1,
            "driftCount": 2,
            "inSyncCount": 1,
            "missingLockCount": 1,
            "missingLiveCount": 1
        },
        "drifts": []
    });

    let groups = build_sync_audit_tui_groups(&audit).expect("build groups");
    assert_eq!(groups[0].label, "All");
    assert_eq!(groups[0].count, 4);
    assert_eq!(groups[1].label, "Missing Live");
    assert_eq!(groups[1].count, 1);
    assert_eq!(groups[2].label, "Missing Lock");
    assert_eq!(groups[2].count, 1);
    assert_eq!(groups[3].label, "Drift");
    assert_eq!(groups[3].count, 2);
}

#[test]
fn build_sync_audit_tui_rows_filters_by_status() {
    let audit = json!({
        "summary": {
            "managedCount": 4,
            "baselineCount": 4,
            "currentPresentCount": 3,
            "currentMissingCount": 1,
            "driftCount": 1,
            "inSyncCount": 1,
            "missingLockCount": 1,
            "missingLiveCount": 1
        },
        "drifts": [
            {
                "status":"drift-detected",
                "kind":"dashboard",
                "identity":"cpu-main",
                "baselineStatus":"present",
                "currentStatus":"present",
                "driftedFields":["title","refresh"]
            },
            {
                "status":"missing-live",
                "kind":"datasource",
                "identity":"prom-main",
                "baselineStatus":"present",
                "currentStatus":"missing",
                "driftedFields":[]
            },
            {
                "status":"missing-lock",
                "kind":"contact-point",
                "identity":"ops-email",
                "baselineStatus":"missing",
                "currentStatus":"present",
                "driftedFields":[]
            }
        ]
    });

    let drift_rows = build_sync_audit_tui_rows(&audit, "drift-detected").expect("drift rows");
    let missing_live_rows =
        build_sync_audit_tui_rows(&audit, "missing-live").expect("missing-live rows");
    let all_rows = build_sync_audit_tui_rows(&audit, "all").expect("all rows");

    assert_eq!(drift_rows.len(), 1);
    assert_eq!(drift_rows[0].kind, "drift-detected");
    assert_eq!(missing_live_rows.len(), 1);
    assert_eq!(missing_live_rows[0].kind, "missing-live");
    assert_eq!(all_rows.len(), 3);
}

#[test]
fn build_sync_audit_document_without_baseline_only_flags_missing_live() {
    let current_lock = json!({
        "kind": "grafana-utils-sync-lock",
        "resources": [
            {
                "kind": "dashboard",
                "identity": "cpu-main",
                "title": "CPU Main",
                "status": "present",
                "managedFields": ["title"],
                "checksum": "aaaa",
                "snapshot": {"title":"CPU Main"},
                "sourcePath": "dashboards/cpu.json"
            },
            {
                "kind": "datasource",
                "identity": "prom-main",
                "title": "Prometheus Main",
                "status": "missing-live",
                "managedFields": ["url"],
                "checksum": null,
                "snapshot": null,
                "sourcePath": "datasources/prom.json"
            }
        ]
    });

    let document = build_sync_audit_document(&current_lock, None).unwrap();

    assert_eq!(document["summary"]["driftCount"], json!(1));
    assert_eq!(document["summary"]["inSyncCount"], json!(1));
    assert_eq!(document["summary"]["missingLiveCount"], json!(1));
    assert_eq!(document["drifts"][0]["status"], json!("missing-live"));
}

#[test]
fn run_sync_cli_audit_builds_lock_and_allows_clean_write() {
    let temp = tempdir().unwrap();
    let managed_file = temp.path().join("desired.json");
    let live_file = temp.path().join("live.json");
    let lock_file = temp.path().join("sync-lock.json");
    fs::write(
        &managed_file,
        serde_json::to_string_pretty(&json!([
            {
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "managedFields": ["title", "refresh"],
                "body": {"title":"CPU Main","refresh":"5s"}
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &live_file,
        serde_json::to_string_pretty(&json!([
            {
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "managedFields": ["title", "refresh"],
                "body": {"title":"CPU Main","refresh":"5s"}
            }
        ]))
        .unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Audit(SyncAuditArgs {
        managed_file: Some(managed_file),
        lock_file: None,
        live_file: Some(live_file),
        fetch_live: false,
        common: sync_common_args(),
        org_id: None,
        page_size: 100,
        write_lock: Some(lock_file.clone()),
        fail_on_drift: false,
        output_format: SyncOutputFormat::Json,
        interactive: false,
    }));

    assert!(result.is_ok());
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(lock_file).unwrap()).unwrap();
    assert_eq!(lock["kind"], json!("grafana-utils-sync-lock"));
    assert_eq!(lock["summary"]["presentCount"], json!(1));
}

#[test]
fn run_sync_cli_audit_rejects_drift_when_fail_on_drift_is_set() {
    let temp = tempdir().unwrap();
    let lock_file = temp.path().join("sync-lock.json");
    let live_file = temp.path().join("live.json");
    fs::write(
        &lock_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-lock",
            "resources": [
                {
                    "kind": "dashboard",
                    "identity": "cpu-main",
                    "title": "CPU Main",
                    "status": "present",
                    "managedFields": ["title"],
                    "checksum": "aaaa",
                    "snapshot": {"title":"CPU Main"},
                    "sourcePath": "dashboards/cpu.json"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &live_file,
        serde_json::to_string_pretty(&json!([
            {
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "managedFields": ["title"],
                "body": {"title":"CPU Main Updated"}
            }
        ]))
        .unwrap(),
    )
    .unwrap();

    let error = run_sync_cli(SyncGroupCommand::Audit(SyncAuditArgs {
        managed_file: None,
        lock_file: Some(lock_file),
        live_file: Some(live_file),
        fetch_live: false,
        common: sync_common_args(),
        org_id: None,
        page_size: 100,
        write_lock: None,
        fail_on_drift: true,
        output_format: SyncOutputFormat::Json,
        interactive: false,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("detected 1 drifted resource"));
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
        allow_policy_reset: false,
        output_format: SyncOutputFormat::Json,
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
        output_format: SyncOutputFormat::Text,
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("Sync availability input file must contain a JSON object"));
}
