//! Overview contract and text-rendering regressions.

use super::overview::{
    OverviewDocument, OverviewProjectStatus, OverviewProjectStatusFreshness,
    OverviewProjectStatusOverall, OverviewSummary, OVERVIEW_KIND,
};
use crate::common::TOOL_VERSION;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn write_datasource_export_fixture(dir: &Path, variant: &str) {
    write_datasource_export_fixture_with_scope_kind(dir, variant, None);
}

fn write_datasource_export_fixture_with_scope_kind(
    dir: &Path,
    variant: &str,
    scope_kind: Option<&str>,
) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": variant,
            "scopeKind": scope_kind,
            "resource": "datasource",
            "datasourcesFile": "datasources.json",
            "indexFile": "index.json",
            "datasourceCount": 2,
            "format": "grafana-datasource-inventory-v1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "true",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "access": "proxy",
                "url": "http://loki:3100",
                "isDefault": "false",
                "org": "Ops Org.",
                "orgId": "2"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
}

fn write_datasource_scope_fixture(dir: &Path, org_id: &str, org_name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": format!("prom-{}", org_id),
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "true",
                "org": org_name,
                "orgId": org_id
            }
        ]))
        .unwrap(),
    )
    .unwrap();
}

fn write_datasource_provisioning_fixture(path: &Path) {
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
  - uid: loki-main
    name: Loki Main
    type: loki
    access: proxy
    url: http://loki:3100
    orgId: 2
    isDefault: false
"#,
    )
    .unwrap();
}

fn write_dashboard_export_fixture(dir: &Path) {
    fs::create_dir_all(dir.join("General")).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
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
        dir.join("folders.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "general",
                "title": "General",
                "parentUid": null,
                "path": "General",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("General").join("cpu.json"),
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

fn write_dashboard_root_fixture(dir: &Path) {
    fs::create_dir_all(dir.join("raw")).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "root",
            "scopeKind": "org-root",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "org": "Main Org.",
            "orgId": "1"
        }))
        .unwrap(),
    )
    .unwrap();
    write_dashboard_export_fixture(&dir.join("raw"));
}

fn write_alert_export_fixture(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("index.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "apiVersion": 1,
            "kind": "grafana-util-alert-export-index",
            "rules": [
                {
                    "kind": "grafana-alert-rule",
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "folderUID": "infra",
                    "ruleGroup": "cpu-alerts",
                    "path": "rules/infra/cpu-alerts/CPU_High__cpu-high.json"
                }
            ],
            "contact-points": [
                {
                    "kind": "grafana-contact-point",
                    "uid": "pagerduty-main",
                    "name": "PagerDuty Main",
                    "type": "pagerduty",
                    "path": "contact-points/PagerDuty_Main/PagerDuty_Main__pagerduty-main.json"
                }
            ],
            "mute-timings": [
                {
                    "kind": "grafana-mute-timing",
                    "name": "Off Hours",
                    "path": "mute-timings/Off_Hours/Off_Hours.json"
                }
            ],
            "policies": [
                {
                    "kind": "grafana-notification-policies",
                    "receiver": "grafana-default-email",
                    "path": "policies/notification-policies.json"
                }
            ],
            "templates": [
                {
                    "kind": "grafana-notification-template",
                    "name": "slack.default",
                    "path": "templates/slack.default.json"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_access_export_fixture(
    dir: &Path,
    payload_filename: &str,
    kind: &str,
    version: i64,
    records: serde_json::Value,
) {
    fs::create_dir_all(dir).unwrap();
    let record_count = records.as_array().map(Vec::len).unwrap_or(0);
    fs::write(
        dir.join(payload_filename),
        serde_json::to_string_pretty(&json!({
            "kind": kind,
            "version": version,
            "records": records,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": kind,
            "version": version,
            "sourceUrl": "http://localhost:3000",
            "recordCount": record_count,
            "sourceDir": dir.display().to_string(),
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_change_desired_fixture(path: &Path) {
    fs::write(
        path,
        serde_json::to_string_pretty(&json!([
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json"
            },
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "Prometheus Main",
                "body": {"type": "prometheus"},
                "sourcePath": "datasources/prom-main.json"
            },
            {
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "body": {
                    "folderUid": "ops",
                    "datasourceUids": ["prom-main"],
                    "datasourceNames": ["Prometheus Main"]
                },
                "sourcePath": "dashboards/cpu-main.json"
            },
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition"],
                "body": {"condition": "A > 90"},
                "sourcePath": "alerts/cpu-high.json"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
}

fn assert_project_status_domain_contract(domain: &Value, expected_id: &str) {
    assert_eq!(domain["id"], json!(expected_id));
    assert!(domain["scope"].is_string());
    assert!(domain["mode"].is_string());
    assert!(domain["status"].is_string());
    assert!(domain["reasonCode"].is_string());
    assert!(domain["primaryCount"].is_u64());
    assert!(domain["blockerCount"].is_u64());
    assert!(domain["warningCount"].is_u64());
    assert!(domain["sourceKinds"].is_array());
    assert!(domain["signalKeys"].is_array());
    assert!(domain["blockers"].is_array());
    assert!(domain["warnings"].is_array());
    assert!(domain["nextActions"].is_array());
    assert!(domain["freshness"].is_object());
}

fn assert_dashboard_domain_contract(domain: &Value) {
    assert_project_status_domain_contract(domain, "dashboard");
    assert_eq!(domain["scope"], json!("staged"));
    assert_eq!(domain["mode"], json!("inspect-summary"));
    assert_eq!(domain["sourceKinds"], json!(["dashboard-export"]));
    let signal_keys = domain["signalKeys"].as_array().unwrap();
    for key in [
        "summary.dashboardCount",
        "summary.queryCount",
        "summary.orphanedDatasourceCount",
        "summary.mixedDatasourceDashboardCount",
    ] {
        assert!(signal_keys.iter().any(|value| value == key));
    }
}

fn sample_overview_document() -> OverviewDocument {
    OverviewDocument {
        kind: OVERVIEW_KIND.to_string(),
        schema_version: 1,
        tool_version: TOOL_VERSION.to_string(),
        discovery: None,
        summary: OverviewSummary {
            artifact_count: 3,
            dashboard_export_count: 1,
            datasource_export_count: 1,
            alert_export_count: 1,
            access_user_export_count: 0,
            access_team_export_count: 0,
            access_org_export_count: 0,
            access_service_account_export_count: 0,
            sync_summary_count: 0,
            bundle_preflight_count: 0,
            promotion_preflight_count: 0,
        },
        project_status: OverviewProjectStatus {
            schema_version: 1,
            tool_version: TOOL_VERSION.to_string(),
            discovery: None,
            scope: "staged-only".to_string(),
            overall: OverviewProjectStatusOverall {
                status: "partial".to_string(),
                domain_count: 6,
                present_count: 1,
                blocked_count: 0,
                blocker_count: 0,
                warning_count: 0,
                freshness: OverviewProjectStatusFreshness {
                    status: "current".to_string(),
                    source_count: 1,
                    newest_age_seconds: Some(15),
                    oldest_age_seconds: Some(15),
                },
            },
            domains: Vec::new(),
            top_blockers: Vec::new(),
            next_actions: Vec::new(),
        },
        artifacts: Vec::new(),
        selected_section_index: 0,
        sections: Vec::new(),
    }
}

#[path = "overview_contract_rust_tests.rs"]
mod overview_contract_rust_tests;

#[path = "overview_live_rust_tests.rs"]
mod overview_live_rust_tests;
