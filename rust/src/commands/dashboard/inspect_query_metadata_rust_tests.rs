//! Feature-oriented query inspect regressions.

use super::super::test_support;
use super::super::{
    DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    TOOL_SCHEMA_VERSION,
};
use serde_json::json;
use std::fs;
use tempfile::tempdir;
#[test]
fn build_export_inspection_query_report_includes_dashboard_tags_variables_and_panel_counts() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("General")).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": FOLDER_INVENTORY_FILENAME
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(raw_dir.join(FOLDER_INVENTORY_FILENAME), "[]\n").unwrap();
    fs::write(
        raw_dir.join("General").join("vars.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "vars-main",
                "title": "Vars Main",
                "tags": ["ops", "production"],
                "panels": [
                    {
                        "id": 7,
                        "title": "Mixed",
                        "type": "timeseries",
                        "description": "owner=$team env=${env}",
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"uid": "prom-main", "type": "prometheus"},
                                "expr": "sum(rate(node_cpu_seconds_total{cluster=~\"$cluster\"}[$__interval]))"
                            },
                            {
                                "refId": "B",
                                "datasource": {"uid": "loki-main", "type": "loki"},
                                "expr": "{job=\"api\", cluster=~\"$cluster\"}"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();
    let rows = report
        .queries
        .iter()
        .map(|row| (row.ref_id.clone(), row.clone()))
        .collect::<std::collections::BTreeMap<String, test_support::ExportInspectionQueryRow>>();

    assert_eq!(
        rows["A"].dashboard_tags,
        vec!["ops".to_string(), "production".to_string()]
    );
    assert_eq!(rows["A"].panel_target_count, 2);
    assert_eq!(rows["A"].panel_query_count, 2);
    assert_eq!(rows["A"].panel_datasource_count, 2);
    assert_eq!(
        rows["A"].query_variables,
        vec!["cluster".to_string(), "__interval".to_string()]
    );
    assert_eq!(rows["B"].query_variables, vec!["cluster".to_string()]);
    assert_eq!(
        rows["A"].panel_variables,
        vec!["team".to_string(), "env".to_string()]
    );
    assert_eq!(
        rows["B"].panel_variables,
        vec!["team".to_string(), "env".to_string()]
    );

    let report_document = test_support::build_export_inspection_query_report_document(&report);
    let row_a = report_document
        .queries
        .iter()
        .find(|row| row.ref_id == "A")
        .unwrap();
    assert_eq!(
        row_a.dashboard_tags,
        vec!["ops".to_string(), "production".to_string()]
    );
    assert_eq!(row_a.panel_target_count, 2);
    assert_eq!(row_a.panel_query_count, 2);
    assert_eq!(row_a.panel_datasource_count, 2);
    assert_eq!(
        row_a.query_variables,
        vec!["cluster".to_string(), "__interval".to_string()]
    );
    assert_eq!(
        row_a.panel_variables,
        vec!["team".to_string(), "env".to_string()]
    );
}

#[test]
fn build_export_inspection_query_report_distinguishes_panel_target_count_from_query_count() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("General")).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("General").join("targets.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "target-counts",
                "title": "Target Counts",
                "panels": [
                    {
                        "id": 7,
                        "title": "Checks",
                        "type": "timeseries",
                        "targets": [
                            {
                                "refId": "A",
                                "expr": "up"
                            },
                            {
                                "refId": "B",
                                "expr": "sum(rate(http_requests_total[5m]))",
                                "hide": true
                            },
                            {
                                "refId": "C",
                                "expr": "ignored_metric",
                                "disabled": true
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();
    let row_a = report.queries.iter().find(|row| row.ref_id == "A").unwrap();
    let row_b = report.queries.iter().find(|row| row.ref_id == "B").unwrap();
    let row_c = report.queries.iter().find(|row| row.ref_id == "C").unwrap();

    assert_eq!(row_a.panel_target_count, 3);
    assert_eq!(row_a.panel_query_count, 2);
    assert_eq!(row_a.target_hidden, "false");
    assert_eq!(row_a.target_disabled, "false");
    assert_eq!(row_b.panel_target_count, 3);
    assert_eq!(row_b.panel_query_count, 2);
    assert_eq!(row_b.target_hidden, "true");
    assert_eq!(row_b.target_disabled, "false");
    assert_eq!(row_c.panel_target_count, 3);
    assert_eq!(row_c.panel_query_count, 2);
    assert_eq!(row_c.target_hidden, "false");
    assert_eq!(row_c.target_disabled, "true");
}

#[test]
fn build_export_inspection_query_report_extracts_dashboard_tags_variables_and_panel_counts() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("General")).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("General").join("variables.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "vars-main",
                "title": "Variables Main",
                "tags": ["prod", "infra"],
                "panels": [
                    {
                        "id": 7,
                        "title": "CPU $cluster",
                        "type": "timeseries",
                        "datasource": "${DS_PROM}",
                        "targets": [
                            {
                                "refId": "A",
                                "expr": "sum(rate(http_requests_total{instance=~\"$host\"}[5m]))"
                            },
                            {
                                "refId": "B",
                                "datasource": {"uid": "loki-main", "type": "loki"},
                                "expr": "sum(rate({job=~\"$job\"}[5m]))"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();
    let row_a = report.queries.iter().find(|row| row.ref_id == "A").unwrap();
    let row_b = report.queries.iter().find(|row| row.ref_id == "B").unwrap();

    assert_eq!(report.summary.dashboard_count, 1);
    assert_eq!(report.summary.panel_count, 1);
    assert_eq!(report.summary.query_count, 2);
    assert_eq!(
        row_a.dashboard_tags,
        vec!["prod".to_string(), "infra".to_string()]
    );
    assert!(row_a.panel_variables.contains(&"DS_PROM".to_string()));
    assert!(row_a.panel_variables.contains(&"cluster".to_string()));
    assert_eq!(row_a.panel_target_count, 2);
    assert_eq!(row_a.panel_query_count, 2);
    assert_eq!(row_a.panel_datasource_count, 2);
    assert_eq!(row_a.query_variables, vec!["host".to_string()]);
    assert_eq!(row_b.query_variables, vec!["job".to_string()]);

    let report_document = test_support::build_export_inspection_query_report_document(&report);
    let row_a = report_document
        .queries
        .iter()
        .find(|query| query.ref_id == "A")
        .unwrap();
    assert_eq!(
        row_a.dashboard_tags,
        vec!["prod".to_string(), "infra".to_string()]
    );
    assert_eq!(row_a.panel_target_count, 2);
    assert_eq!(row_a.panel_query_count, 2);
    assert_eq!(row_a.panel_datasource_count, 2);
    assert_eq!(row_a.query_variables, vec!["host".to_string()]);
}

#[test]
fn build_export_inspection_query_report_includes_datasource_config_fields() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("General")).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "datasourcesFile": DATASOURCE_INVENTORY_FILENAME
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join(DATASOURCE_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "influx-main",
                "name": "Influx Main",
                "type": "influxdb",
                "access": "proxy",
                "url": "http://influxdb:8086",
                "database": "metrics_v1",
                "defaultBucket": "prod-default",
                "organization": "acme-observability",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "elastic-main",
                "name": "Elastic Main",
                "type": "elasticsearch",
                "access": "proxy",
                "url": "http://elasticsearch:9200",
                "indexPattern": "[logs-]YYYY.MM.DD",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("General").join("main.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "main",
                "title": "Main",
                "panels": [
                    {
                        "id": 8,
                        "title": "Flux Query",
                        "type": "table",
                        "datasource": {"uid": "influx-main", "type": "influxdb"},
                        "targets": [
                            {"refId": "B", "query": "from(bucket: \"prod\") |> range(start: -1h)"}
                        ]
                    },
                    {
                        "id": 11,
                        "title": "Elastic Query",
                        "type": "table",
                        "datasource": {"uid": "elastic-main", "type": "elasticsearch"},
                        "targets": [
                            {"refId": "E", "query": "status:500"}
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();
    assert_eq!(report.queries.len(), 2);
    let row = |panel_id: &str| {
        report
            .queries
            .iter()
            .find(|query| query.panel_id == panel_id)
            .unwrap()
    };

    assert_eq!(row("8").datasource_database, "metrics_v1");
    assert_eq!(row("8").datasource_org, "Main Org.");
    assert_eq!(row("8").datasource_org_id, "1");
    assert_eq!(row("8").datasource_bucket, "prod-default");
    assert_eq!(row("8").datasource_organization, "acme-observability");
    assert_eq!(row("11").datasource_family, "search");
    assert_eq!(row("11").datasource_index_pattern, "[logs-]YYYY.MM.DD");
    assert_eq!(row("11").measurements, vec!["status".to_string()]);
    assert_eq!(row("11").metrics, Vec::<String>::new());

    let report_document = test_support::build_export_inspection_query_report_document(&report);
    let row = |panel_id: &str| {
        report_document
            .queries
            .iter()
            .find(|query| query.panel_id == panel_id)
            .unwrap()
    };
    assert_eq!(row("8").datasource_org, "Main Org.");
    assert_eq!(row("8").datasource_org_id, "1");
    assert_eq!(row("8").datasource_database, "metrics_v1");
    assert_eq!(row("8").datasource_bucket, "prod-default");
    assert_eq!(row("8").datasource_organization, "acme-observability");
    assert_eq!(row("11").datasource_index_pattern, "[logs-]YYYY.MM.DD");
    assert_eq!(row("11").datasource_family, "search");
}

#[test]
fn build_export_inspection_query_report_resolves_datasource_name_only_objects_against_inventory() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("General")).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "datasourcesFile": DATASOURCE_INVENTORY_FILENAME
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join(DATASOURCE_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("General").join("inventory.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "inventory",
                "title": "Inventory",
                "panels": [
                    {
                        "id": 1,
                        "title": "CPU",
                        "type": "timeseries",
                        "datasource": {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"name": "Prometheus Main"},
                                "expr": "up"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let report = test_support::build_export_inspection_query_report(&raw_dir).unwrap();
    let row = &report.queries[0];

    assert_eq!(row.datasource, "Prometheus Main");
    assert_eq!(row.datasource_name, "Prometheus Main");
    assert_eq!(row.datasource_uid, "prom-main");
    assert_eq!(row.datasource_type, "prometheus");
    assert_eq!(row.datasource_family, "prometheus");
    assert_eq!(row.datasource_org, "Main Org.");
    assert_eq!(row.panel_datasource_count, 1);
}
