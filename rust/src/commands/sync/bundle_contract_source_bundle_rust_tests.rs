//! Sync source-bundle document contract coverage.
use super::super::{
    build_alert_replay_artifact_fixture, load_alert_export_contract_fixture,
    load_sync_source_bundle_contract_fixture,
};
use crate::common::TOOL_VERSION;
use crate::sync::workbench::{build_sync_source_bundle_document, render_sync_source_bundle_text};
use serde_json::json;

#[test]
fn build_sync_source_bundle_document_matches_cross_domain_summary_contract() {
    let fixture = load_sync_source_bundle_contract_fixture();
    let source_bundle = build_sync_source_bundle_document(
        &[
            json!({"kind": "dashboard", "uid": "cpu-main", "title": "CPU Main"}),
            json!({"kind": "dashboard", "uid": "logs-main", "title": "Logs Main"}),
        ],
        &[json!({
            "kind": "datasource",
            "uid": "prom-main",
            "name": "Prometheus Main",
            "title": "Prometheus Main"
        })],
        &[json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations"
        })],
        &[json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition"],
            "body": {"condition": "A"}
        })],
        Some(&build_alert_replay_artifact_fixture(true)),
        Some(&json!({"bundleLabel": "sync-smoke"})),
    )
    .unwrap();

    assert_eq!(source_bundle["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(
        source_bundle["summary"],
        fixture["crossDomainSummaryCase"]["summary"]
    );
    assert_eq!(
        source_bundle["dashboards"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        source_bundle["datasources"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(source_bundle["folders"].as_array().map(Vec::len), Some(1));
    assert_eq!(source_bundle["alerts"].as_array().map(Vec::len), Some(1));

    let text = render_sync_source_bundle_text(&source_bundle).unwrap();
    assert_eq!(
        text,
        fixture["crossDomainSummaryCase"]["textLines"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect::<Vec<String>>()
    );
}

#[test]
fn render_sync_source_bundle_text_includes_discovery_summary_when_present() {
    let mut source_bundle = build_sync_source_bundle_document(
        &[],
        &[],
        &[],
        &[],
        Some(&build_alert_replay_artifact_fixture(true)),
        Some(&json!({"bundleLabel": "sync-smoke"})),
    )
    .unwrap();
    source_bundle.as_object_mut().unwrap().insert(
        "discovery".to_string(),
        json!({
            "workspaceRoot": "/tmp/grafana-oac-repo",
            "inputCount": 2,
            "inputs": {
                "dashboardExportDir": "/tmp/grafana-oac-repo/dashboards/raw",
                "alertExportDir": "/tmp/grafana-oac-repo/alerts/raw"
            }
        }),
    );

    let text = render_sync_source_bundle_text(&source_bundle).unwrap();
    assert_eq!(
        text[1],
        "Discovery: workspace-root=/tmp/grafana-oac-repo sources=dashboard-export, alert-export"
    );
}

#[test]
fn build_sync_source_bundle_document_keeps_normalized_alert_specs() {
    let source_bundle = build_sync_source_bundle_document(
        &[],
        &[],
        &[],
        &[json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "datasourceUids"],
            "body": {
                "condition": "A",
                "datasourceUids": ["prom-main"]
            },
            "sourcePath": "rules/infra/cpu-high.json"
        })],
        Some(&json!({"summary": {"ruleCount": 1}})),
        None,
    )
    .unwrap();

    assert_eq!(source_bundle["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(source_bundle["alerts"][0]["uid"], json!("cpu-high"));
    assert_eq!(
        source_bundle["alerts"][0]["body"]["datasourceUids"][0],
        json!("prom-main")
    );
}

#[test]
fn build_sync_source_bundle_document_includes_alert_contract() {
    let source_bundle = build_sync_source_bundle_document(
        &[
            json!({
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "body": {"datasourceUids": ["prom-main"]},
            }),
            json!({
                "kind": "dashboard",
                "uid": "logs-main",
                "title": "Logs Main",
                "body": {"datasourceUids": ["loki-main"]},
            }),
        ],
        &[json!({
            "kind": "datasource",
            "uid": "prom-main",
            "name": "Prometheus Main",
            "title": "Prometheus Main",
            "body": {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
        })],
        &[],
        &[],
        Some(&json!({
            "rules": [{
                "sourcePath": "rules/infra/cpu-high.json",
                "document": {
                    "kind": "grafana-alert-rule",
                    "metadata": {
                        "uid": "cpu-high",
                        "title": "CPU High",
                    },
                    "spec": {
                        "uid": "cpu-high",
                        "title": "CPU High",
                    },
                },
            }],
            "contactPoints": [{"uid": "pagerduty-primary", "name": "PagerDuty Primary"}],
            "summary": {"ruleCount": 1, "contactPointCount": 1},
        })),
        None,
    )
    .unwrap();

    let contract = &source_bundle["alertContract"];
    assert_eq!(contract["kind"], json!("grafana-utils-sync-alert-contract"));
    assert_eq!(contract["summary"]["total"], json!(2));
    assert_eq!(contract["summary"]["safeForSync"], json!(2));
    assert_eq!(contract["resources"].as_array().unwrap().len(), 2);
}

#[test]
fn build_sync_source_bundle_document_preserves_full_alert_contract_sections() {
    let source_bundle = build_sync_source_bundle_document(
        &[],
        &[],
        &[],
        &[],
        Some(&build_alert_replay_artifact_fixture(true)),
        None,
    )
    .unwrap();

    let contract = &source_bundle["alertContract"];
    assert_eq!(
        contract["summary"],
        json!({
            "total": 5,
            "safeForSync": 2
        })
    );
    assert_eq!(
        contract["countsByKind"],
        json!([
            {"kind": "grafana-alert-rule", "count": 1},
            {"kind": "grafana-contact-point", "count": 1},
            {"kind": "grafana-mute-timing", "count": 1},
            {"kind": "grafana-notification-policies", "count": 1},
            {"kind": "grafana-notification-template", "count": 1}
        ])
    );
    assert_eq!(contract["resources"].as_array().unwrap().len(), 5);
    assert!(contract["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| {
            resource["kind"] == "grafana-contact-point" && resource["safeForSync"] == json!(true)
        }));
    assert!(contract["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| {
            resource["kind"] == "grafana-mute-timing" && resource["safeForSync"] == json!(false)
        }));
    assert!(contract["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| {
            resource["kind"] == "grafana-notification-policies"
                && resource["safeForSync"] == json!(false)
        }));
    assert!(contract["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| {
            resource["kind"] == "grafana-notification-template"
                && resource["safeForSync"] == json!(false)
        }));
}

#[test]
fn build_sync_source_bundle_document_preserves_alert_replay_artifact_summary_and_paths() {
    let fixture = load_alert_export_contract_fixture();
    let source_bundle = build_sync_source_bundle_document(
        &[],
        &[],
        &[],
        &[],
        Some(&build_alert_replay_artifact_fixture(true)),
        None,
    )
    .unwrap();

    let expected_summary = &fixture["syncAlertingArtifacts"]["summary"];
    assert_eq!(source_bundle["alerting"]["summary"], *expected_summary);
    for artifact in fixture["syncAlertingArtifacts"]["artifacts"]
        .as_array()
        .unwrap_or(&Vec::new())
    {
        let section = artifact["section"].as_str().unwrap_or_default();
        let identity_field = artifact["identityField"].as_str().unwrap_or_default();
        let identity = artifact["identity"].clone();
        let source_path = artifact["sourcePath"].clone();
        let empty_entries = Vec::new();
        let entries = source_bundle["alerting"][section]
            .as_array()
            .unwrap_or(&empty_entries);
        assert!(entries.iter().any(|entry| {
            entry["sourcePath"] == source_path
                && entry["document"]["spec"][identity_field] == identity
        }));
    }
}

#[test]
fn render_sync_source_bundle_text_reports_alert_replay_artifact_counts() {
    let fixture = load_alert_export_contract_fixture();
    let source_bundle = build_sync_source_bundle_document(
        &[],
        &[],
        &[],
        &[],
        Some(&build_alert_replay_artifact_fixture(true)),
        None,
    )
    .unwrap();

    let lines = render_sync_source_bundle_text(&source_bundle).unwrap();
    let expected = format!(
        "Alerting: rules={} contact-points={} mute-timings={} policies={} templates={}",
        fixture["syncAlertingArtifacts"]["summary"]["ruleCount"],
        fixture["syncAlertingArtifacts"]["summary"]["contactPointCount"],
        fixture["syncAlertingArtifacts"]["summary"]["muteTimingCount"],
        fixture["syncAlertingArtifacts"]["summary"]["policyCount"],
        fixture["syncAlertingArtifacts"]["summary"]["templateCount"]
    );
    assert!(lines.iter().any(|line| line == &expected));
}
