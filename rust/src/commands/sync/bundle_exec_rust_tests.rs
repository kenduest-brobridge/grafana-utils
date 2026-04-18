//! Sync bundle CLI execution and artifact-writing regression test facade.
//!
//! Shared fixture builders stay here so the split test modules can reuse them
//! without duplicating setup logic.

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

fn write_dashboard_raw_fixture(root: &std::path::Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("folders.json"),
        serde_json::to_string_pretty(&json!([
            {"uid": "general", "title": "General", "path": "General"}
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("cpu-main.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            },
            "meta": {"folderUid": "general"}
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_nested_dashboard_raw_fixture(root: &std::path::Path) {
    fs::create_dir_all(root).unwrap();
    write_dashboard_raw_fixture(root);
}

fn write_alert_export_fixture(root: &std::path::Path) {
    fs::create_dir_all(root).unwrap();
    fs::create_dir_all(root.join("rules").join("general").join("cpu-alerts")).unwrap();
    fs::write(
        root.join("index.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "apiVersion": 1,
            "kind": "grafana-util-alert-export-index",
            "rules": [{
                "kind": "grafana-alert-rule",
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "cpu-alerts",
                "path": "rules/general/cpu-alerts/CPU_High__cpu-high.json"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("rules")
            .join("general")
            .join("cpu-alerts")
            .join("CPU_High__cpu-high.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "toolVersion": "test",
            "apiVersion": 1,
            "kind": "grafana-alert-rule",
            "metadata": {
                "uid": "cpu-high",
                "title": "CPU High"
            },
            "spec": {
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "cpu-alerts",
                "condition": "A",
                "data": [{
                    "refId": "A",
                    "datasourceUid": "prom-main",
                    "model": {
                        "expr": "up",
                        "refId": "A"
                    }
                }]
            }
        }))
        .unwrap(),
    )
    .unwrap();
}

#[cfg(test)]
#[path = "bundle_exec_sources_rust_tests.rs"]
mod sync_bundle_exec_sources_rust_tests;

#[cfg(test)]
#[path = "bundle_exec_domain_artifacts_rust_tests.rs"]
mod sync_bundle_exec_domain_artifacts_rust_tests;

#[cfg(test)]
#[path = "bundle_exec_preflight_rust_tests.rs"]
mod sync_bundle_exec_preflight_rust_tests;
