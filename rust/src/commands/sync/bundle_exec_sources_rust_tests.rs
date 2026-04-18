//! Sync bundle execution tests for source-bundle writing and output format behavior.

use super::{write_alert_export_fixture, write_dashboard_provisioning_fixture};
use crate::sync::{run_sync_cli, SyncBundleArgs, SyncGroupCommand, SyncOutputFormat};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

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
        workspace: None,
        dashboard_export_dir: Some(dashboard_export_dir.clone()),
        dashboard_provisioning_dir: None,
        alert_export_dir: Some(alert_export_dir.clone()),
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: Some(metadata_file.clone()),
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
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
fn run_sync_cli_bundle_keeps_plain_file_output_when_also_stdout_is_enabled() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp.path().join("dashboards").join("raw");
    let output_file = temp.path().join("bundle.json");
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

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        workspace: None,
        dashboard_export_dir: Some(dashboard_export_dir.clone()),
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: true,
        output_format: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok(), "{result:?}");
    let raw = fs::read_to_string(&output_file).unwrap();
    assert!(!raw.contains('\u{1b}'));
    assert!(raw.ends_with('\n'));
    let bundle: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(bundle["kind"], json!("grafana-utils-sync-source-bundle"));
    assert_eq!(bundle["summary"]["dashboardCount"], json!(1));
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
        workspace: None,
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: Some(alert_export_dir.clone()),
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
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
        workspace: None,
        dashboard_export_dir: Some(dashboard_export_dir.clone()),
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
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
        workspace: None,
        dashboard_export_dir: None,
        dashboard_provisioning_dir: Some(provisioning_root.clone()),
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
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
