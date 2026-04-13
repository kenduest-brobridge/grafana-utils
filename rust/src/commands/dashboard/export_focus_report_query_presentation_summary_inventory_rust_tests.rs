//! Inventory-backed query-report and merged-import-dir coverage for query presentation reports.
use super::super::test_support;
use super::super::{
    DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    TOOL_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;

#[test]
fn build_export_inspection_summary_uses_unique_folder_title_fallback_for_full_path() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("Infra")).unwrap();
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
    fs::write(
        raw_dir.join(FOLDER_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "infra",
                "title": "Infra",
                "parentUid": "platform",
                "path": "Platform / Infra",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("Infra").join("sub.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "sub",
                "title": "Sub",
                "panels": []
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let summary = test_support::build_export_inspection_summary(&raw_dir).unwrap();

    assert_eq!(summary.folder_paths[0].path, "Platform / Infra");
}

#[test]
fn build_export_inspection_summary_includes_zero_dashboard_ancestor_paths() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("Prod")).unwrap();
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
    fs::write(
        raw_dir.join(FOLDER_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "platform",
                "title": "Platform",
                "parentUid": null,
                "path": "Platform",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "team",
                "title": "Team",
                "parentUid": "platform",
                "path": "Platform / Team",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "apps",
                "title": "Apps",
                "parentUid": "team",
                "path": "Platform / Team / Apps",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "prod",
                "title": "Prod",
                "parentUid": "apps",
                "path": "Platform / Team / Apps / Prod",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("Prod").join("prod.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "prod-main",
                "title": "Prod Main",
                "panels": []
            },
            "meta": {"folderUid": "prod"}
        }))
        .unwrap(),
    )
    .unwrap();

    let summary = test_support::build_export_inspection_summary(&raw_dir).unwrap();
    let paths = summary
        .folder_paths
        .iter()
        .map(|item| (item.path.clone(), item.dashboards))
        .collect::<Vec<(String, usize)>>();

    assert_eq!(
        paths,
        vec![
            ("Platform".to_string(), 0),
            ("Platform / Team".to_string(), 0),
            ("Platform / Team / Apps".to_string(), 0),
            ("Platform / Team / Apps / Prod".to_string(), 1),
        ]
    );
}

#[test]
fn build_export_inspection_summary_accepts_legacy_index_without_org_identity_fields() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(raw_dir.join("Legacy")).unwrap();
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
        raw_dir.join("index.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "legacy-main",
                "title": "Legacy Main",
                "folder": "Legacy",
                "path": "dashboards/raw/Legacy/Legacy_Main__legacy-main.json",
                "format": "grafana-web-import-preserve-uid"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("Legacy").join("Legacy_Main__legacy-main.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "legacy-main",
                "title": "Legacy Main",
                "panels": []
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let summary = test_support::build_export_inspection_summary(&raw_dir).unwrap();

    assert_eq!(summary.dashboard_count, 1);
    assert_eq!(summary.export_org, None);
    assert_eq!(summary.export_org_id, None);
}

#[test]
fn build_export_inspection_query_report_emits_search_family_for_inventory_backed_elasticsearch_and_opensearch_rows(
) {
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
                "uid": "elastic-main",
                "name": "Elastic Main",
                "type": "elasticsearch",
                "access": "proxy",
                "url": "http://elasticsearch:9200",
                "indexPattern": "[logs-]YYYY.MM.DD",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "opensearch-main",
                "name": "OpenSearch Main",
                "type": "grafana-opensearch-datasource",
                "access": "proxy",
                "url": "http://opensearch:9200",
                "indexPattern": "logs-*",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("General").join("search.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "search-main",
                "title": "Search Main",
                "panels": [
                    {
                        "id": 7,
                        "title": "Elastic Query",
                        "type": "table",
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"uid": "elastic-main"},
                                "query": "status:500 AND _exists_:trace.id AND service.name:\"api\""
                            }
                        ]
                    },
                    {
                        "id": 8,
                        "title": "OpenSearch Query",
                        "type": "table",
                        "targets": [
                            {
                                "refId": "B",
                                "datasource": {"uid": "opensearch-main"},
                                "query": "_exists_:host.name AND host.name:api AND response.status:404 AND category:\"auth\""
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
    let row = |ref_id: &str| {
        report
            .queries
            .iter()
            .find(|query| query.ref_id == ref_id)
            .unwrap()
    };

    assert_eq!(report.summary.dashboard_count, 1);
    assert_eq!(report.summary.panel_count, 2);
    assert_eq!(report.summary.query_count, 2);
    assert_eq!(report.summary.report_row_count, 2);

    let elastic = row("A");
    assert_eq!(elastic.datasource, "elastic-main");
    assert_eq!(elastic.datasource_name, "Elastic Main");
    assert_eq!(elastic.datasource_uid, "elastic-main");
    assert_eq!(elastic.datasource_type, "elasticsearch");
    assert_eq!(elastic.datasource_family, "search");
    assert_eq!(elastic.datasource_org, "Main Org.");
    assert_eq!(elastic.datasource_org_id, "1");
    assert_eq!(elastic.datasource_index_pattern, "[logs-]YYYY.MM.DD");
    assert_eq!(elastic.query_field, "query");
    assert_eq!(elastic.metrics, Vec::<String>::new());
    assert_eq!(elastic.functions, Vec::<String>::new());
    assert_eq!(elastic.buckets, Vec::<String>::new());
    assert_eq!(
        elastic.measurements,
        vec![
            "trace.id".to_string(),
            "status".to_string(),
            "service.name".to_string(),
        ]
    );

    let opensearch = row("B");
    assert_eq!(opensearch.datasource, "opensearch-main");
    assert_eq!(opensearch.datasource_name, "OpenSearch Main");
    assert_eq!(opensearch.datasource_uid, "opensearch-main");
    assert_eq!(opensearch.datasource_type, "grafana-opensearch-datasource");
    assert_eq!(opensearch.datasource_family, "search");
    assert_eq!(opensearch.datasource_org, "Main Org.");
    assert_eq!(opensearch.datasource_org_id, "1");
    assert_eq!(opensearch.datasource_index_pattern, "logs-*");
    assert_eq!(opensearch.query_field, "query");
    assert_eq!(opensearch.metrics, Vec::<String>::new());
    assert_eq!(opensearch.functions, Vec::<String>::new());
    assert_eq!(opensearch.buckets, Vec::<String>::new());
    assert_eq!(
        opensearch.measurements,
        vec![
            "host.name".to_string(),
            "response.status".to_string(),
            "category".to_string(),
        ]
    );

    let filtered = test_support::apply_query_report_filters(report.clone(), Some("search"), None);
    assert_eq!(filtered.summary.dashboard_count, 1);
    assert_eq!(filtered.summary.panel_count, 2);
    assert_eq!(filtered.summary.query_count, 2);
    assert_eq!(filtered.summary.report_row_count, 2);
    assert_eq!(filtered.queries.len(), 2);
    assert!(filtered
        .queries
        .iter()
        .all(|query| query.datasource_family == "search"));
    assert_eq!(
        filtered
            .queries
            .iter()
            .map(|query| query.datasource_type.as_str())
            .collect::<Vec<&str>>(),
        vec!["elasticsearch", "grafana-opensearch-datasource"]
    );
}

#[test]
fn build_export_inspection_query_report_emits_prometheus_and_loki_families_for_inventory_backed_rows(
) {
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
            },
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "access": "proxy",
                "url": "http://loki:3100",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("General").join("core.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "core-main",
                "title": "Core Main",
                "panels": [
                    {
                        "id": 7,
                        "title": "HTTP Requests",
                        "type": "timeseries",
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"uid": "prom-main"},
                                "expr": "sum by(instance) (rate(http_requests_total{job=\"api\", instance=~\"web-.+\", __name__=\"http_requests_total\"}[5m])) / ignoring(pod) group_left(namespace) kube_pod_info{namespace=\"prod\", pod=~\"api-.+\"}"
                            }
                        ]
                    },
                    {
                        "id": 11,
                        "title": "Errors",
                        "type": "logs",
                        "targets": [
                            {
                                "refId": "B",
                                "datasource": {"uid": "loki-main"},
                                "expr": "sum by (level) (count_over_time({job=\"grafana\",level=~\"error|warn\"} |= \"timeout\" | json | level=\"error\" [5m]))"
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
    assert_eq!(report.summary.dashboard_count, 1);
    assert_eq!(report.summary.panel_count, 2);
    assert_eq!(report.summary.query_count, 2);
    assert_eq!(report.summary.report_row_count, 2);

    let row = |ref_id: &str| {
        report
            .queries
            .iter()
            .find(|query| query.ref_id == ref_id)
            .unwrap()
    };

    let prometheus = row("A");
    assert_eq!(prometheus.datasource, "prom-main");
    assert_eq!(prometheus.datasource_name, "Prometheus Main");
    assert_eq!(prometheus.datasource_uid, "prom-main");
    assert_eq!(prometheus.datasource_type, "prometheus");
    assert_eq!(prometheus.datasource_family, "prometheus");
    assert_eq!(
        prometheus.metrics,
        vec![
            "http_requests_total".to_string(),
            "kube_pod_info".to_string(),
        ]
    );
    assert_eq!(prometheus.functions, vec!["rate".to_string()]);
    assert_eq!(prometheus.measurements, Vec::<String>::new());
    assert_eq!(prometheus.buckets, vec!["5m".to_string()]);

    let loki = row("B");
    assert_eq!(loki.datasource, "loki-main");
    assert_eq!(loki.datasource_name, "Loki Main");
    assert_eq!(loki.datasource_uid, "loki-main");
    assert_eq!(loki.datasource_type, "loki");
    assert_eq!(loki.datasource_family, "loki");
    assert_eq!(loki.metrics, Vec::<String>::new());
    assert_eq!(
        loki.functions,
        vec![
            "sum".to_string(),
            "count_over_time".to_string(),
            "json".to_string(),
            "line_filter_contains".to_string(),
            "line_filter_contains:timeout".to_string(),
        ]
    );
    assert_eq!(
        loki.measurements,
        vec![
            "{job=\"grafana\",level=~\"error|warn\"}".to_string(),
            "job=\"grafana\"".to_string(),
            "level=~\"error|warn\"".to_string(),
            "level".to_string(),
        ]
    );
    assert_eq!(loki.buckets, vec!["5m".to_string()]);
}

#[test]
fn prepare_inspect_export_import_dir_merges_multi_org_root_for_inspection() {
    let export_root_temp = tempdir().unwrap();
    let export_root = export_root_temp.path().join("dashboard");
    let org_one_raw = export_root.join("org_1_Main_Org").join("raw");
    let org_two_raw = export_root.join("org_2_Ops_Org").join("raw");
    fs::create_dir_all(&org_one_raw).unwrap();
    fs::create_dir_all(&org_two_raw).unwrap();
    fs::write(
        export_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "root",
            "dashboardCount": 2,
            "indexFile": "index.json",
            "orgCount": 2,
            "orgs": [
                {"org": "Main Org.", "orgId": "1", "dashboardCount": 1},
                {"org": "Ops Org", "orgId": "2", "dashboardCount": 1}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    for (raw_dir, org_id, org_name, uid) in [
        (&org_one_raw, "1", "Main Org.", "cpu-main"),
        (&org_two_raw, "2", "Ops Org", "logs-main"),
    ] {
        fs::write(
            raw_dir.join(EXPORT_METADATA_FILENAME),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": TOOL_SCHEMA_VERSION,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "format": "grafana-web-import-preserve-uid",
                "foldersFile": FOLDER_INVENTORY_FILENAME,
                "datasourcesFile": DATASOURCE_INVENTORY_FILENAME
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("index.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": uid,
                    "title": uid,
                    "path": format!("General/{uid}.json"),
                    "format": "grafana-web-import-preserve-uid",
                    "org": org_name,
                    "orgId": org_id
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join(FOLDER_INVENTORY_FILENAME),
            serde_json::to_string_pretty(&json!([
                {"uid": "general", "title": "General", "path": "General", "org": org_name, "orgId": org_id}
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join(DATASOURCE_INVENTORY_FILENAME),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": format!("ds-{org_id}"),
                    "name": format!("ds-{org_id}"),
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
        let dashboard_dir = raw_dir.join("General");
        fs::create_dir_all(&dashboard_dir).unwrap();
        fs::write(
            dashboard_dir.join(format!("{uid}.json")),
            serde_json::to_string_pretty(&json!({
                "dashboard": {
                    "uid": uid,
                    "title": uid,
                    "panels": [
                        {
                            "id": 7,
                            "title": "CPU",
                            "type": "timeseries",
                            "datasource": {"uid": format!("ds-{org_id}"), "type": "prometheus"},
                            "targets": [{"refId": "A", "expr": "up"}]
                        }
                    ]
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    let inspect_temp = tempdir().unwrap();
    let merged_raw_dir =
        test_support::prepare_inspect_export_import_dir(inspect_temp.path(), &export_root).unwrap();
    let merged_metadata: Value = serde_json::from_str(
        &fs::read_to_string(merged_raw_dir.join(EXPORT_METADATA_FILENAME)).unwrap(),
    )
    .unwrap();
    let merged_index: Value =
        serde_json::from_str(&fs::read_to_string(merged_raw_dir.join("index.json")).unwrap())
            .unwrap();
    let report = test_support::build_export_inspection_query_report(&merged_raw_dir).unwrap();

    assert_eq!(report.summary.dashboard_count, 2);
    assert_eq!(report.queries.len(), 2);
    assert_eq!(merged_metadata["variant"], Value::String("raw".to_string()));
    assert_eq!(merged_metadata["dashboardCount"], Value::Number(2.into()));
    assert_eq!(
        merged_metadata["foldersFile"],
        Value::String(FOLDER_INVENTORY_FILENAME.to_string())
    );
    assert_eq!(
        merged_metadata["datasourcesFile"],
        Value::String(DATASOURCE_INVENTORY_FILENAME.to_string())
    );
    assert!(merged_metadata.get("orgCount").is_none());
    assert!(merged_metadata.get("orgs").is_none());
    assert_eq!(
        fs::read_to_string(merged_raw_dir.join(".inspect-source-root"))
            .unwrap()
            .trim(),
        export_root.display().to_string()
    );
    let merged_index_items = merged_index.as_array().unwrap();
    assert_eq!(
        merged_index_items[0]["path"],
        Value::String("org_1_Main_Org/General/cpu-main.json".to_string())
    );
    assert_eq!(
        merged_index_items[1]["path"],
        Value::String("org_2_Ops_Org/General/logs-main.json".to_string())
    );
    assert_eq!(report.queries[0].org, "Main Org.");
    assert_eq!(report.queries[0].org_id, "1");
    assert_eq!(report.queries[0].folder_path, "General");
    assert_eq!(
        report.queries[0].file_path,
        export_root
            .join("org_1_Main_Org")
            .join("raw")
            .join("General")
            .join("cpu-main.json")
            .display()
            .to_string()
    );
    assert_eq!(report.queries[1].org, "Ops Org");
    assert_eq!(report.queries[1].org_id, "2");
    assert_eq!(report.queries[1].folder_path, "General");
}
