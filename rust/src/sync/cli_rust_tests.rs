//! Sync CLI test suite.
//! Verifies sync routing and rendering contracts that remain outside the split execution slices.
use super::{
    run_sync_cli, SyncAdvancedCliArgs, SyncAdvancedCommand, SyncAssessAlertsArgs, SyncGroupCommand,
    SyncOutputFormat, SyncPlanArgs, SyncSummaryArgs,
};
use crate::dashboard::CommonCliArgs;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

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

    let result = run_sync_cli(SyncGroupCommand::Advanced(SyncAdvancedCliArgs {
        command: SyncAdvancedCommand::Summary(SyncSummaryArgs {
            desired_file,
            output_format: SyncOutputFormat::Json,
        }),
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

    let result = run_sync_cli(SyncGroupCommand::Advanced(SyncAdvancedCliArgs {
        command: SyncAdvancedCommand::Plan(SyncPlanArgs {
            desired_file,
            live_file: Some(live_file),
            fetch_live: false,
            common: CommonCliArgs {
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
            },
            org_id: None,
            page_size: 500,
            allow_prune: false,
            output_format: SyncOutputFormat::Json,
            trace_id: None,
        }),
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

    let result = run_sync_cli(SyncGroupCommand::Advanced(SyncAdvancedCliArgs {
        command: super::SyncAdvancedCommand::AssessAlerts(SyncAssessAlertsArgs {
            alerts_file,
            output_format: SyncOutputFormat::Json,
        }),
    }));

    assert!(result.is_ok());
}

#[test]
fn filter_review_plan_operations_recalculates_summary_and_alert_assessment() {
    let plan = json!({
        "kind": "grafana-utils-sync-plan",
        "traceId": "sync-trace-review",
        "summary": {
            "would_create": 2,
            "would_update": 1,
            "would_delete": 0,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 1,
            "alert_plan_only": 0,
            "alert_blocked": 0
        },
        "reviewRequired": true,
        "reviewed": false,
        "operations": [
            {"kind":"datasource","identity":"prom-main","action":"would-update"},
            {"kind":"alert-contact-point","identity":"ops-email","action":"would-create"},
            {"kind":"folder","identity":"infra","action":"noop"}
        ]
    });
    let selected = ["alert-contact-point::ops-email".to_string()]
        .into_iter()
        .collect();

    let filtered = super::review_tui::filter_review_plan_operations(&plan, &selected).unwrap();

    assert_eq!(filtered["summary"]["would_create"], json!(1));
    assert_eq!(filtered["summary"]["would_update"], json!(0));
    assert_eq!(filtered["summary"]["noop"], json!(1));
    assert_eq!(
        filtered["alertAssessment"]["summary"]["candidateCount"],
        json!(1)
    );
    assert_eq!(filtered["operations"].as_array().unwrap().len(), 2);
}
