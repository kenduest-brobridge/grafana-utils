use crate::sync_preflight::{
    build_sync_preflight_document, render_sync_preflight_text, SYNC_PREFLIGHT_KIND,
};
use crate::sync_workbench::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_summary_document,
    normalize_resource_spec, summarize_resource_specs, SYNC_APPLY_INTENT_KIND, SYNC_SUMMARY_KIND,
};
use serde_json::json;

#[test]
fn normalize_resource_spec_requires_alert_managed_fields() {
    let error = normalize_resource_spec(&json!({
        "kind": "alert",
        "uid": "cpu-high",
        "title": "CPU High",
        "body": {
            "condition": "A > 90"
        }
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("managedFields"));
}

#[test]
fn build_sync_summary_document_counts_normalized_resource_kinds() {
    let raw_specs = vec![
        json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations",
            "body": {"title": "Operations"},
            "sourcePath": "folders/ops.json"
        }),
        json!({
            "kind": "datasource",
            "uid": "prom-main",
            "name": "Prometheus Main",
            "body": {"type": "prometheus"},
            "sourcePath": "datasources/prom-main.json"
        }),
        json!({
            "kind": "dashboard",
            "uid": "cpu-main",
            "title": "CPU Main",
            "body": {"datasourceUids": ["prom-main"]},
            "sourcePath": "dashboards/cpu-main.json"
        }),
        json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "contactPoints"],
            "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]},
            "sourcePath": "alerts/cpu-high.json"
        }),
    ];

    let document = build_sync_summary_document(&raw_specs).unwrap();

    assert_eq!(document["kind"], json!(SYNC_SUMMARY_KIND));
    assert_eq!(document["summary"]["resourceCount"], json!(4));
    assert_eq!(document["summary"]["dashboardCount"], json!(1));
    assert_eq!(document["summary"]["datasourceCount"], json!(1));
    assert_eq!(document["summary"]["folderCount"], json!(1));
    assert_eq!(document["summary"]["alertCount"], json!(1));
    assert_eq!(
        document["resources"][3]["managedFields"],
        json!(["condition", "contactPoints"])
    );
}

#[test]
fn summarize_resource_specs_reports_counts() {
    let specs = vec![
        normalize_resource_spec(&json!({"kind":"folder","uid":"ops","title":"Operations"}))
            .unwrap(),
        normalize_resource_spec(&json!({
            "kind":"alert",
            "uid":"cpu-high",
            "title":"CPU High",
            "managedFields":["condition"],
            "body":{"condition":"A > 90"}
        }))
        .unwrap(),
    ];

    let summary = summarize_resource_specs(&specs);

    assert_eq!(summary.resource_count, 2);
    assert_eq!(summary.folder_count, 1);
    assert_eq!(summary.alert_count, 1);
}

#[test]
fn build_sync_preflight_document_reports_plugin_dependency_and_alert_blocks() {
    let desired_specs = vec![
        json!({
            "kind": "datasource",
            "uid": "loki-main",
            "name": "Loki Main",
            "body": {"type": "loki"}
        }),
        json!({
            "kind": "dashboard",
            "uid": "cpu-main",
            "title": "CPU Main",
            "body": {
                "datasourceUids": ["loki-main", "prom-main"],
                "datasourceNames": ["Prometheus Main"],
                "pluginIds": ["timeseries", "geomap"]
            }
        }),
        json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "contactPoints"],
            "body": {
                "condition": "A > 90",
                "datasourceUid": "loki-main",
                "datasourceName": "Prometheus Main",
                "pluginIds": ["grafana-oncall-app"],
                "contactPoints": ["pagerduty-primary"],
                "notificationSettings": {"receiver": "slack-primary"}
            }
        }),
    ];
    let availability = json!({
        "pluginIds": ["prometheus", "timeseries"],
        "datasourceUids": ["prom-main"],
        "datasourceNames": [],
        "contactPoints": []
    });

    let document = build_sync_preflight_document(&desired_specs, Some(&availability)).unwrap();

    assert_eq!(document["kind"], json!(SYNC_PREFLIGHT_KIND));
    assert_eq!(document["summary"]["checkCount"], json!(13));
    assert_eq!(document["summary"]["blockingCount"], json!(10));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "plugin" && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "dashboard-datasource"
            && item["identity"] == "cpu-main->loki-main"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "dashboard-datasource-name"
            && item["identity"] == "cpu-main->Prometheus Main"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "dashboard-plugin"
            && item["identity"] == "cpu-main->timeseries"
            && item["status"] == "ok"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "dashboard-plugin"
            && item["identity"] == "cpu-main->geomap"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-live-apply" && item["status"] == "blocked"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-datasource"
            && item["identity"] == "cpu-high->loki-main"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-datasource-name"
            && item["identity"] == "cpu-high->Prometheus Main"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-plugin"
            && item["identity"] == "cpu-high->grafana-oncall-app"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point" && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point"
            && item["identity"] == "cpu-high->slack-primary"
            && item["status"] == "missing"));
}

#[test]
fn render_sync_preflight_text_renders_deterministic_summary() {
    let document = build_sync_preflight_document(
        &[json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations"
        })],
        None,
    )
    .unwrap();

    let lines = render_sync_preflight_text(&document).unwrap();

    assert_eq!(lines[0], "Sync preflight summary");
    assert!(lines[1].contains("1 total"));
    assert!(lines
        .iter()
        .any(|line| line.contains("folder identity=ops status=ok")));
}

#[test]
fn render_sync_preflight_text_rejects_wrong_kind() {
    let error = render_sync_preflight_text(&json!({"kind": "wrong"}))
        .unwrap_err()
        .to_string();

    assert!(error.contains("kind is not supported"));
}

#[test]
fn build_sync_apply_intent_document_requires_review_and_approval() {
    let plan = build_sync_plan_document(
        &[json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations",
            "body": {"title": "Operations"}
        })],
        &[],
        false,
    )
    .unwrap();

    let not_reviewed = build_sync_apply_intent_document(&plan, true)
        .unwrap_err()
        .to_string();
    assert!(not_reviewed.contains("marked reviewed"));

    let mut reviewed = plan.as_object().cloned().unwrap();
    reviewed.insert("reviewed".to_string(), json!(true));
    let not_approved = build_sync_apply_intent_document(&json!(reviewed), false)
        .unwrap_err()
        .to_string();
    assert!(not_approved.contains("explicit approval"));
}

#[test]
fn build_sync_apply_intent_document_filters_non_mutating_operations() {
    let plan = json!({
        "kind": "grafana-utils-sync-plan",
        "reviewRequired": true,
        "reviewed": true,
        "allowPrune": false,
        "summary": {
            "would_create": 1,
            "would_update": 1,
            "would_delete": 0,
            "noop": 1,
            "unmanaged": 1,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 0
        },
        "alertAssessment": {
            "summary": {
                "candidateCount": 0,
                "planOnlyCount": 0,
                "blockedCount": 0
            }
        },
        "operations": [
            {"kind":"folder","identity":"ops","action":"would-create"},
            {"kind":"dashboard","identity":"cpu-main","action":"would-update"},
            {"kind":"datasource","identity":"prom-main","action":"noop"},
            {"kind":"folder","identity":"legacy","action":"unmanaged"}
        ]
    });

    let intent = build_sync_apply_intent_document(&plan, true).unwrap();

    assert_eq!(intent["kind"], json!(SYNC_APPLY_INTENT_KIND));
    assert_eq!(intent["mode"], json!("apply"));
    assert_eq!(intent["approved"], json!(true));
    assert_eq!(intent["operations"].as_array().unwrap().len(), 2);
    assert!(intent["operations"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| matches!(
            item["action"].as_str(),
            Some("would-create" | "would-update" | "would-delete")
        )));
}
