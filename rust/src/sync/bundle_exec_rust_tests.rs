//! Sync bundle CLI execution and artifact-writing regression tests.
#![allow(unused_imports)]

use super::sync_common_args;
use crate::sync::{
    render_sync_apply_intent_text, run_sync_cli, SyncBundleArgs, SyncBundlePreflightArgs,
    SyncGroupCommand, SyncOutputFormat,
};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

fn write_datasource_provisioning_fixture(path: &std::path::Path) {
    fs::write(
        path,
        r#"apiVersion: 1
datasources:
  - uid: prom-main
    name: Prometheus Main
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    orgId: 1
    isDefault: true
"#,
    )
    .unwrap();
}

fn write_dashboard_provisioning_fixture(root: &std::path::Path) {
    let dashboards_dir = root.join("dashboards").join("team");
    fs::create_dir_all(&dashboards_dir).unwrap();
    fs::write(
        dashboards_dir.join("cpu-main.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("folders.json"),
        serde_json::to_string_pretty(&json!([
            {"uid": "team", "title": "Team", "path": "Team"}
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "provisioning",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-file-provisioning-dashboard"
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn run_sync_cli_bundle_writes_source_bundle_artifact() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp.path().join("dashboards").join("raw");
    let alert_export_dir = temp.path().join("alerts").join("raw");
    fs::create_dir_all(&dashboard_export_dir).unwrap();
    fs::create_dir_all(alert_export_dir.join("rules")).unwrap();
    fs::write(
        dashboard_export_dir.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dashboard_export_dir.join("folders.json"),
        serde_json::to_string_pretty(&json!([
            {"uid": "ops", "title": "Operations", "path": "Operations"}
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dashboard_export_dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([
            {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"}
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir.join("rules").join("cpu-high.json"),
        serde_json::to_string_pretty(&json!({
            "groups": [{
                "name": "CPU Alerts",
                "folderUid": "general",
                "rules": [{
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "condition": "A",
                    "data": [{
                        "refId": "A",
                        "datasourceUid": "prom-main",
                        "model": {
                            "expr": "up",
                            "refId": "A"
                        }
                    }],
                    "for": "5m",
                    "noDataState": "NoData",
                    "execErrState": "Alerting",
                    "annotations": {
                        "__dashboardUid__": "cpu-main",
                        "__panelId__": "1"
                    },
                    "notification_settings": {
                        "receiver": "pagerduty-primary"
                    }
                }]
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let metadata_file = temp.path().join("metadata.json");
    fs::write(
        &metadata_file,
        serde_json::to_string_pretty(&json!({
            "bundleLabel": "smoke-bundle"
        }))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: Some(dashboard_export_dir.clone()),
        dashboard_provisioning_dir: None,
        alert_export_dir: Some(alert_export_dir.clone()),
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: Some(metadata_file.clone()),
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["kind"], json!("grafana-utils-sync-source-bundle"));
    assert_eq!(bundle["summary"]["dashboardCount"], json!(1));
    assert_eq!(bundle["summary"]["datasourceCount"], json!(1));
    assert_eq!(bundle["summary"]["folderCount"], json!(1));
    assert_eq!(bundle["summary"]["alertRuleCount"], json!(1));
    assert_eq!(bundle["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["alerts"][0]["kind"], json!("alert"));
    assert_eq!(bundle["alerts"][0]["uid"], json!("cpu-high"));
    assert_eq!(bundle["alerts"][0]["title"], json!("CPU High"));
    assert_eq!(
        bundle["alerts"][0]["managedFields"],
        json!([
            "condition",
            "annotations",
            "contactPoints",
            "datasourceUids",
            "data"
        ])
    );
    assert_eq!(bundle["alerts"][0]["body"]["condition"], json!("A"));
    assert_eq!(
        bundle["alerts"][0]["body"]["contactPoints"],
        json!(["pagerduty-primary"])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["datasourceUids"],
        json!(["prom-main"])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["annotations"]["__dashboardUid__"],
        json!("cpu-main")
    );
    assert_eq!(bundle["metadata"]["bundleLabel"], json!("smoke-bundle"));
    assert_eq!(
        bundle["metadata"]["dashboardExportDir"],
        json!(dashboard_export_dir.display().to_string())
    );
    assert_eq!(
        bundle["alerting"]["exportDir"],
        json!(alert_export_dir.display().to_string())
    );
    assert_eq!(bundle["alerting"]["summary"]["ruleCount"], json!(1));
    assert_eq!(bundle["alerting"]["summary"]["contactPointCount"], json!(0));
    assert_eq!(bundle["alerting"]["summary"]["policyCount"], json!(0));
    assert_eq!(
        bundle["metadata"]["alertExportDir"],
        json!(alert_export_dir.display().to_string())
    );
}

#[test]
fn run_sync_cli_bundle_preserves_alert_export_artifact_metadata() {
    let temp = tempdir().unwrap();
    let alert_export_dir = temp.path().join("alerts").join("raw");
    fs::create_dir_all(
        alert_export_dir
            .join("contact-points")
            .join("Smoke_Webhook"),
    )
    .unwrap();
    fs::create_dir_all(alert_export_dir.join("mute-timings")).unwrap();
    fs::create_dir_all(alert_export_dir.join("policies")).unwrap();
    fs::create_dir_all(alert_export_dir.join("templates")).unwrap();
    fs::write(
        alert_export_dir
            .join("contact-points")
            .join("Smoke_Webhook")
            .join("Smoke_Webhook__smoke-webhook.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-contact-point",
            "apiVersion": 1,
            "schemaVersion": 1,
            "spec": {
                "uid": "smoke-webhook",
                "name": "Smoke Webhook",
                "type": "webhook",
                "settings": {"url": "http://127.0.0.1/notify"}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir.join("contact-points").join("index.json"),
        serde_json::to_string_pretty(&json!([
            {
                "kind": "grafana-contact-point",
                "uid": "smoke-webhook",
                "name": "Smoke Webhook",
                "type": "webhook",
                "path": "contact-points/Smoke_Webhook/Smoke_Webhook__smoke-webhook.json"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir.join("mute-timings").join("Off_Hours.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-mute-timing",
            "apiVersion": 1,
            "schemaVersion": 1,
            "spec": {
                "name": "Off Hours",
                "time_intervals": [{
                    "times": [{
                        "start_time": "00:00",
                        "end_time": "06:00"
                    }]
                }]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir
            .join("policies")
            .join("notification-policies.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-notification-policies",
            "apiVersion": 1,
            "schemaVersion": 1,
            "spec": {"receiver": "grafana-default-email"}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir
            .join("templates")
            .join("slack.default.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-notification-template",
            "apiVersion": 1,
            "schemaVersion": 1,
            "spec": {
                "name": "slack.default",
                "template": "{{ define \"slack.default\" }}ok{{ end }}"
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        alert_export_dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-util-alert-export-index",
            "apiVersion": 1,
            "schemaVersion": 1
        }))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: Some(alert_export_dir.clone()),
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["alerting"]["summary"]["contactPointCount"], json!(1));
    assert_eq!(bundle["alerting"]["summary"]["muteTimingCount"], json!(1));
    assert_eq!(bundle["alerting"]["summary"]["policyCount"], json!(1));
    assert_eq!(bundle["alerting"]["summary"]["templateCount"], json!(1));
    assert_eq!(bundle["alerts"].as_array().unwrap().len(), 4);
    assert!(bundle["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point" && item["uid"] == "smoke-webhook"));
    assert!(bundle["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-mute-timing" && item["name"] == "Off Hours"));
    assert!(bundle["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-policy" && item["title"] == "grafana-default-email"));
    assert!(bundle["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-template" && item["name"] == "slack.default"));
    assert_eq!(
        bundle["alerting"]["exportMetadata"]["kind"],
        json!("grafana-util-alert-export-index")
    );
    assert_eq!(
        bundle["alerting"]["muteTimings"][0]["sourcePath"],
        json!("mute-timings/Off_Hours.json")
    );
    assert_eq!(
        bundle["alerting"]["contactPoints"][0]["sourcePath"],
        json!("contact-points/Smoke_Webhook/Smoke_Webhook__smoke-webhook.json")
    );
    assert_eq!(
        bundle["alerting"]["policies"][0]["sourcePath"],
        json!("policies/notification-policies.json")
    );
    assert_eq!(
        bundle["alerting"]["templates"][0]["sourcePath"],
        json!("templates/slack.default.json")
    );
}

#[test]
fn run_sync_cli_bundle_ignores_dashboard_permissions_bundle() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp.path().join("dashboards").join("raw");
    fs::create_dir_all(&dashboard_export_dir).unwrap();
    fs::write(
        dashboard_export_dir.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dashboard_export_dir.join("permissions.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "dashboard-permissions",
            "permissions": [{
                "uid": "cpu-main",
                "role": "Viewer"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: Some(dashboard_export_dir.clone()),
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok(), "{result:?}");
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["summary"]["dashboardCount"], json!(1));
    assert_eq!(bundle["dashboards"].as_array().unwrap().len(), 1);
    assert_eq!(bundle["dashboards"][0]["uid"], json!("cpu-main"));
}

#[test]
fn run_sync_cli_bundle_supports_dashboard_provisioning_root() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("dashboards").join("provisioning");
    write_dashboard_provisioning_fixture(&provisioning_root);
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: Some(provisioning_root.clone()),
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok(), "{result:?}");
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["summary"]["dashboardCount"], json!(1));
    assert_eq!(
        bundle["dashboards"][0]["sourcePath"],
        json!("team/cpu-main.json")
    );
    assert_eq!(bundle["folders"][0]["uid"], json!("team"));
    assert_eq!(
        bundle["metadata"]["dashboardProvisioningDir"],
        json!(provisioning_root.display().to_string())
    );
    assert_eq!(
        bundle["metadata"]["dashboardExport"]["variant"],
        json!("provisioning")
    );
}

#[test]
fn run_sync_cli_bundle_rejects_conflicting_dashboard_inputs() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp.path().join("dashboards").join("raw");
    let dashboard_provisioning_dir = temp.path().join("dashboards").join("provisioning");
    fs::create_dir_all(&dashboard_export_dir).unwrap();
    fs::create_dir_all(&dashboard_provisioning_dir).unwrap();

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: Some(dashboard_export_dir),
        dashboard_provisioning_dir: Some(dashboard_provisioning_dir),
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: None,
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("only one dashboard input"));
}

#[test]
fn run_sync_cli_bundle_preserves_datasource_provider_metadata_from_inventory_file() {
    let temp = tempdir().unwrap();
    let datasource_export_file = temp.path().join("datasources.json");
    fs::write(
        &datasource_export_file,
        serde_json::to_string_pretty(&json!([
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "secureJsonDataProviders": {
                    "httpHeaderValue1": "${provider:vault:secret/data/loki/token}"
                },
                "secureJsonDataPlaceholders": {
                    "basicAuthPassword": "${secret:loki-basic-auth}"
                }
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: Some(datasource_export_file.clone()),
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["summary"]["datasourceCount"], json!(1));
    assert_eq!(
        bundle["metadata"]["datasourceExportFile"],
        json!(datasource_export_file.display().to_string())
    );
    assert_eq!(
        bundle["datasources"][0]["secureJsonDataProviders"]["httpHeaderValue1"],
        json!("${provider:vault:secret/data/loki/token}")
    );
    assert_eq!(
        bundle["datasources"][0]["secureJsonDataPlaceholders"]["basicAuthPassword"],
        json!("${secret:loki-basic-auth}")
    );
}

#[test]
fn run_sync_cli_bundle_preserves_datasource_metadata_from_provisioning_file() {
    let temp = tempdir().unwrap();
    let datasource_provisioning_file = temp.path().join("datasources.yaml");
    write_datasource_provisioning_fixture(&datasource_provisioning_file);
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: Some(datasource_provisioning_file.clone()),
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["summary"]["datasourceCount"], json!(1));
    assert_eq!(
        bundle["metadata"]["datasourceProvisioningFile"],
        json!(datasource_provisioning_file.display().to_string())
    );
    assert_eq!(bundle["datasources"][0]["uid"], json!("prom-main"));
    assert_eq!(
        bundle["datasources"][0]["body"]["name"],
        json!("Prometheus Main")
    );
}

#[test]
fn run_sync_cli_bundle_normalizes_tool_rule_export_into_top_level_alert_spec() {
    let temp = tempdir().unwrap();
    let alert_export_dir = temp.path().join("alerts").join("raw");
    fs::create_dir_all(alert_export_dir.join("rules")).unwrap();
    fs::write(
        alert_export_dir.join("rules").join("cpu-high.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "apiVersion": 1,
            "kind": "grafana-alert-rule",
            "metadata": {
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "CPU Alerts"
            },
            "spec": {
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "CPU Alerts",
                "condition": "A",
                "data": [{
                    "refId": "A",
                    "datasourceUid": "prom-main",
                    "model": {
                        "datasource": {
                            "uid": "prom-main",
                            "name": "Prometheus Main",
                            "type": "prometheus"
                        },
                        "expr": "up",
                        "refId": "A"
                    }
                }],
                "notificationSettings": {
                    "receiver": "pagerduty-primary"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: Some(alert_export_dir.clone()),
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(bundle["summary"]["alertRuleCount"], json!(1));
    assert_eq!(bundle["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(
        bundle["alerts"][0]["managedFields"],
        json!([
            "condition",
            "contactPoints",
            "datasourceUids",
            "datasourceNames",
            "pluginIds",
            "data"
        ])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["contactPoints"],
        json!(["pagerduty-primary"])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["datasourceNames"],
        json!(["Prometheus Main"])
    );
    assert_eq!(
        bundle["alerts"][0]["body"]["pluginIds"],
        json!(["prometheus"])
    );
    assert_eq!(
        bundle["metadata"]["alertExportDir"],
        json!(alert_export_dir.display().to_string())
    );
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
        fetch_live: false,
        common: sync_common_args(),
        org_id: None,
        output: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok());
}

#[test]
fn render_sync_apply_intent_text_includes_alert_artifact_bundle_counts() {
    let lines = render_sync_apply_intent_text(&json!({
        "kind": "grafana-utils-sync-apply-intent",
        "stage": "apply",
        "stepIndex": 3,
        "traceId": "sync-trace-demo",
        "parentTraceId": "sync-trace-demo",
        "mode": "apply",
        "reviewed": true,
        "reviewRequired": true,
        "allowPrune": false,
        "approved": true,
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
        "operations": [],
        "bundlePreflightSummary": {
            "resourceCount": 4,
            "syncBlockingCount": 0,
            "providerBlockingCount": 0,
            "secretPlaceholderBlockingCount": 1,
            "alertArtifactCount": 4,
            "alertArtifactPlanOnlyCount": 1,
            "alertArtifactBlockingCount": 3
        }
    }))
    .unwrap();

    let output = lines.join("\n");
    assert!(output.contains("secret-placeholder-blocking=1"));
    assert!(output.contains("alert-artifacts=4"));
    assert!(output.contains("plan-only=1"));
    assert!(output.contains("blocking=3"));
    assert!(
        output.contains("Reason: preflight and bundle-preflight blocking must be 0 before apply")
    );
}
