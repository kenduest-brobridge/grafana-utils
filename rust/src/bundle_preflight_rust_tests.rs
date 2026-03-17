//! Bundle preflight contract tests.
//! Validates preflight generation behavior for staged alert and sync documents.
use crate::alert_sync::{assess_alert_sync_specs, ALERT_SYNC_KIND};
use crate::bundle_preflight::{
    build_bundle_preflight_document, render_bundle_preflight_text, BUNDLE_PREFLIGHT_KIND,
};
use serde_json::json;

#[test]
fn assess_alert_sync_specs_reports_plan_only_and_blocked_states() {
    let document = assess_alert_sync_specs(&[
        json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "contactPoints"],
            "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]}
        }),
        json!({
            "kind": "alert",
            "uid": "disk-high",
            "title": "Disk High",
            "managedFields": ["labels"],
            "body": {"condition": "A > 80"}
        }),
    ])
    .unwrap();

    assert_eq!(document["kind"], json!(ALERT_SYNC_KIND));
    assert_eq!(document["summary"]["planOnlyCount"], json!(1));
    assert_eq!(document["summary"]["blockedCount"], json!(1));
}

#[test]
fn build_bundle_preflight_document_aggregates_sync_alert_and_provider_checks() {
    let source_bundle = json!({
        "environment": "staging",
        "dashboards": [
            {
                "uid": "cpu-main",
                "title": "CPU Main",
                "datasourceUids": ["prom-main"]
            }
        ],
        "datasources": [
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "secureJsonDataProviders": {
                    "httpHeaderValue1": "${provider:vault:secret/data/prom/token}"
                }
            }
        ],
        "folders": [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"}
            }
        ],
        "alerts": [
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition", "contactPoints"],
                "body": {
                    "condition": "A > 90",
                    "datasourceUid": "prom-main",
                    "datasourceName": "Prometheus Main",
                    "contactPoints": ["pagerduty-primary"],
                    "notificationSettings": {"receiver": "slack-primary"}
                }
            }
        ]
    });
    let target_inventory = json!({
        "dashboards": [{"uid": "cpu-main", "title": "CPU Main"}],
        "datasources": [],
        "folders": []
    });
    let availability = json!({
        "pluginIds": [],
        "datasourceUids": [],
        "datasourceNames": [],
        "contactPoints": [],
        "providerNames": []
    });

    let document = build_bundle_preflight_document(
        &source_bundle,
        Some(&target_inventory),
        Some(&availability),
    )
    .unwrap();

    assert_eq!(document["kind"], json!(BUNDLE_PREFLIGHT_KIND));
    assert!(document.get("sourceSummary").is_some());
    assert!(document.get("targetSummary").is_some());
    assert!(document.get("syncPreflight").is_some());
    assert!(document.get("alertAssessment").is_some());
    assert!(document.get("providerAssessment").is_some());
    assert_eq!(document["summary"]["alertPlanOnlyCount"], json!(1));
    assert_eq!(document["summary"]["providerBlockingCount"], json!(1));
    assert!(
        document["summary"]["syncBlockingCount"]
            .as_i64()
            .unwrap_or(0)
            >= 1
    );
}

#[test]
fn render_bundle_preflight_text_renders_summary() {
    let document = build_bundle_preflight_document(
        &json!({
            "datasources": [],
            "dashboards": [],
            "folders": [],
            "alerts": []
        }),
        None,
        None,
    )
    .unwrap();

    let lines = render_bundle_preflight_text(&document).unwrap();

    assert_eq!(lines[0], "Bundle preflight summary");
    assert!(lines.iter().any(|line| line.starts_with("Sync blocking: ")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("Provider blocking: ")));
}
