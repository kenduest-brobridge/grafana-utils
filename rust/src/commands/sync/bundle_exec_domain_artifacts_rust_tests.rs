//! Sync bundle execution tests for domain artifact preservation and normalization.

use super::{write_datasource_provisioning_fixture, write_nested_dashboard_raw_fixture};
use crate::sync::{run_sync_cli, SyncBundleArgs, SyncGroupCommand, SyncOutputFormat};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn run_sync_cli_bundle_preserves_nested_raw_org_source_paths() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp
        .path()
        .join("dashboards")
        .join("raw")
        .join("org_1_Main_Org")
        .join("raw");
    write_nested_dashboard_raw_fixture(&dashboard_export_dir);
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
    assert_eq!(
        bundle["dashboards"][0]["sourcePath"],
        json!("org_1_Main_Org/raw/cpu-main.json")
    );
    assert_eq!(
        bundle["metadata"]["dashboardExportDir"],
        json!(dashboard_export_dir.display().to_string())
    );
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
        workspace: None,
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: Some(datasource_export_file.clone()),
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
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
        workspace: None,
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: Some(datasource_provisioning_file.clone()),
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
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
