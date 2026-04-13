//! Feature-oriented query inspect regressions.

use super::super::test_support;
use super::super::{
    assert_core_family_query_row, CoreFamilyQueryRowExpectation, DATASOURCE_INVENTORY_FILENAME,
    EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME, TOOL_SCHEMA_VERSION,
};
use serde_json::json;
use std::fs;
use tempfile::tempdir;
#[test]
fn build_export_inspection_query_report_matches_core_family_query_row_contract() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    for folder in ["General", "Infra", "Logs", "SQL", "Search", "Tracing"] {
        fs::create_dir_all(raw_dir.join(folder)).unwrap();
    }
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 6,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": FOLDER_INVENTORY_FILENAME,
            "datasourcesFile": DATASOURCE_INVENTORY_FILENAME
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
                "path": "Platform",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "infra",
                "title": "Infra",
                "parentUid": "platform",
                "path": "Platform / Infra",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "logs",
                "title": "Logs",
                "path": "Logs",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "sql",
                "title": "SQL",
                "path": "SQL",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "search",
                "title": "Search",
                "path": "Search",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "tracing",
                "title": "Tracing",
                "path": "Tracing",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
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
            },
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
                "uid": "sql-main",
                "name": "SQL Main",
                "type": "postgres",
                "access": "proxy",
                "url": "postgresql://postgres:5432/metrics",
                "database": "analytics",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "search-main",
                "name": "OpenSearch Main",
                "type": "grafana-opensearch-datasource",
                "access": "proxy",
                "url": "http://opensearch:9200",
                "indexPattern": "logs-*",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "trace-main",
                "name": "Tempo Main",
                "type": "tempo",
                "access": "proxy",
                "url": "http://tempo:3200",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("General").join("prometheus.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "prom-main",
                "title": "Prometheus Main",
                "panels": [
                    {
                        "id": 7,
                        "title": "CPU Quantiles",
                        "type": "timeseries",
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"uid": "prom-main", "type": "prometheus"},
                                "expr": "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket{job=\"api\",handler=\"/orders\"}[5m])) by (le))"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("Logs").join("loki.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "logs-main",
                "title": "Logs Main",
                "panels": [
                    {
                        "id": 11,
                        "title": "Pipeline Errors",
                        "type": "logs",
                        "targets": [
                            {
                                "refId": "B",
                                "datasource": {"uid": "loki-main", "type": "loki"},
                                "expr": "sum by (namespace) (count_over_time({job=\"grafana\",namespace!=\"kube-system\",cluster=~\"prod|stage\"} |= \"timeout\" | json | logfmt [10m]))"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("Infra").join("flux.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "flux-main",
                "title": "Flux Main",
                "panels": [
                    {
                        "id": 9,
                        "title": "Requests",
                        "type": "timeseries",
                        "targets": [
                            {
                                "refId": "C",
                                "datasource": {"uid": "influx-main", "type": "influxdb"},
                                "query": "from(bucket: \"prod\") |> range(start: -1h) |> filter(fn: (r) => r._measurement == \"cpu\" and r.host == \"web-01\") |> aggregateWindow(every: 5m, fn: mean) |> yield(name: \"mean\")"
                            }
                        ]
                    }
                ]
            },
            "meta": {"folderUid": "infra"}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("SQL").join("sql.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "sql-main",
                "title": "SQL Main",
                "panels": [
                    {
                        "id": 13,
                        "title": "Host Ownership",
                        "type": "table",
                        "targets": [
                            {
                                "refId": "D",
                                "datasource": {"uid": "sql-main", "type": "postgres"},
                                "rawSql": "WITH recent_cpu AS (SELECT * FROM public.cpu_metrics) SELECT recent_cpu.host, hosts.owner FROM recent_cpu JOIN \"public\".\"hosts\" ON hosts.host = recent_cpu.host WHERE hosts.owner IS NOT NULL ORDER BY hosts.owner LIMIT 10"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("Search").join("search.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "search-main",
                "title": "Search Main",
                "panels": [
                    {
                        "id": 17,
                        "title": "OpenSearch Hits",
                        "type": "table",
                        "targets": [
                            {
                                "refId": "E",
                                "datasource": {"uid": "search-main", "type": "grafana-opensearch-datasource"},
                                "query": "_exists_:@timestamp AND resource.service.name:\"checkout\" AND status:[500 TO 599]"
                            }
                        ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("Tracing").join("tracing.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "trace-main",
                "title": "Trace Main",
                "panels": [
                    {
                        "id": 19,
                        "title": "Trace Search",
                        "type": "table",
                        "targets": [
                            {
                                "refId": "F",
                                "datasource": {"uid": "trace-main", "type": "tempo"},
                                "query": "resource.service.name:checkout AND trace.id:abc123 AND span.name:\"GET /orders\""
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

    assert_eq!(report.summary.dashboard_count, 6);
    assert_eq!(report.summary.panel_count, 6);
    assert_eq!(report.summary.query_count, 6);
    assert_eq!(report.summary.report_row_count, 6);
    assert_eq!(report.queries.len(), 6);
    assert_core_family_query_row(
        &report,
        CoreFamilyQueryRowExpectation {
            dashboard_uid: "prom-main",
            dashboard_title: "Prometheus Main",
            panel_id: "7",
            panel_title: "CPU Quantiles",
            panel_type: "timeseries",
            ref_id: "A",
            datasource: "prom-main",
            datasource_name: "Prometheus Main",
            datasource_uid: "prom-main",
            datasource_type: "prometheus",
            datasource_family: "prometheus",
            query_field: "expr",
            query_text: "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket{job=\"api\",handler=\"/orders\"}[5m])) by (le))",
            folder_path: "General",
            folder_full_path: "/",
            folder_level: "1",
            folder_uid: "general",
            parent_folder_uid: "",
            datasource_org: "Main Org.",
            datasource_org_id: "1",
            datasource_database: "",
            datasource_bucket: "",
            datasource_organization: "",
            datasource_index_pattern: "",
            metrics: &["http_request_duration_seconds_bucket"],
            functions: &["histogram_quantile", "sum", "rate"],
            measurements: &[],
            buckets: &["5m"],
        },
    );
    assert_core_family_query_row(
        &report,
        CoreFamilyQueryRowExpectation {
            dashboard_uid: "logs-main",
            dashboard_title: "Logs Main",
            panel_id: "11",
            panel_title: "Pipeline Errors",
            panel_type: "logs",
            ref_id: "B",
            datasource: "loki-main",
            datasource_name: "Loki Main",
            datasource_uid: "loki-main",
            datasource_type: "loki",
            datasource_family: "loki",
            query_field: "expr",
            query_text: "sum by (namespace) (count_over_time({job=\"grafana\",namespace!=\"kube-system\",cluster=~\"prod|stage\"} |= \"timeout\" | json | logfmt [10m]))",
            folder_path: "Logs",
            folder_full_path: "/Logs",
            folder_level: "1",
            folder_uid: "logs",
            parent_folder_uid: "",
            datasource_org: "Main Org.",
            datasource_org_id: "1",
            datasource_database: "",
            datasource_bucket: "",
            datasource_organization: "",
            datasource_index_pattern: "",
            metrics: &[],
            functions: &[
                "sum",
                "count_over_time",
                "json",
                "logfmt",
                "line_filter_contains",
                "line_filter_contains:timeout",
            ],
            measurements: &[
                "{job=\"grafana\",namespace!=\"kube-system\",cluster=~\"prod|stage\"}",
                "job=\"grafana\"",
                "namespace!=\"kube-system\"",
                "cluster=~\"prod|stage\"",
            ],
            buckets: &["10m"],
        },
    );
    assert_core_family_query_row(
        &report,
        CoreFamilyQueryRowExpectation {
            dashboard_uid: "flux-main",
            dashboard_title: "Flux Main",
            panel_id: "9",
            panel_title: "Requests",
            panel_type: "timeseries",
            ref_id: "C",
            datasource: "influx-main",
            datasource_name: "Influx Main",
            datasource_uid: "influx-main",
            datasource_type: "influxdb",
            datasource_family: "flux",
            query_field: "query",
            query_text: "from(bucket: \"prod\") |> range(start: -1h) |> filter(fn: (r) => r._measurement == \"cpu\" and r.host == \"web-01\") |> aggregateWindow(every: 5m, fn: mean) |> yield(name: \"mean\")",
            folder_path: "Platform / Infra",
            folder_full_path: "/Platform/Infra",
            folder_level: "2",
            folder_uid: "infra",
            parent_folder_uid: "platform",
            datasource_org: "Main Org.",
            datasource_org_id: "1",
            datasource_database: "metrics_v1",
            datasource_bucket: "prod-default",
            datasource_organization: "acme-observability",
            datasource_index_pattern: "",
            metrics: &[],
            functions: &["from", "aggregateWindow", "filter", "range", "yield"],
            measurements: &["cpu"],
            buckets: &["prod", "5m"],
        },
    );
    assert_core_family_query_row(
        &report,
        CoreFamilyQueryRowExpectation {
            dashboard_uid: "sql-main",
            dashboard_title: "SQL Main",
            panel_id: "13",
            panel_title: "Host Ownership",
            panel_type: "table",
            ref_id: "D",
            datasource: "sql-main",
            datasource_name: "SQL Main",
            datasource_uid: "sql-main",
            datasource_type: "postgres",
            datasource_family: "sql",
            query_field: "rawSql",
            query_text: "WITH recent_cpu AS (SELECT * FROM public.cpu_metrics) SELECT recent_cpu.host, hosts.owner FROM recent_cpu JOIN \"public\".\"hosts\" ON hosts.host = recent_cpu.host WHERE hosts.owner IS NOT NULL ORDER BY hosts.owner LIMIT 10",
            folder_path: "SQL",
            folder_full_path: "/SQL",
            folder_level: "1",
            folder_uid: "sql",
            parent_folder_uid: "",
            datasource_org: "Main Org.",
            datasource_org_id: "1",
            datasource_database: "analytics",
            datasource_bucket: "",
            datasource_organization: "",
            datasource_index_pattern: "",
            metrics: &[],
            functions: &["with", "select", "join", "where", "order_by", "limit"],
            measurements: &["public.hosts", "public.cpu_metrics"],
            buckets: &[],
        },
    );
    assert_core_family_query_row(
        &report,
        CoreFamilyQueryRowExpectation {
            dashboard_uid: "search-main",
            dashboard_title: "Search Main",
            panel_id: "17",
            panel_title: "OpenSearch Hits",
            panel_type: "table",
            ref_id: "E",
            datasource: "search-main",
            datasource_name: "OpenSearch Main",
            datasource_uid: "search-main",
            datasource_type: "grafana-opensearch-datasource",
            datasource_family: "search",
            query_field: "query",
            query_text:
                "_exists_:@timestamp AND resource.service.name:\"checkout\" AND status:[500 TO 599]",
            folder_path: "Search",
            folder_full_path: "/Search",
            folder_level: "1",
            folder_uid: "search",
            parent_folder_uid: "",
            datasource_org: "Main Org.",
            datasource_org_id: "1",
            datasource_database: "",
            datasource_bucket: "",
            datasource_organization: "",
            datasource_index_pattern: "logs-*",
            metrics: &[],
            functions: &[],
            measurements: &["@timestamp", "resource.service.name", "status"],
            buckets: &[],
        },
    );
    assert_core_family_query_row(
        &report,
        CoreFamilyQueryRowExpectation {
            dashboard_uid: "trace-main",
            dashboard_title: "Trace Main",
            panel_id: "19",
            panel_title: "Trace Search",
            panel_type: "table",
            ref_id: "F",
            datasource: "trace-main",
            datasource_name: "Tempo Main",
            datasource_uid: "trace-main",
            datasource_type: "tempo",
            datasource_family: "tracing",
            query_field: "query",
            query_text:
                "resource.service.name:checkout AND trace.id:abc123 AND span.name:\"GET /orders\"",
            folder_path: "Tracing",
            folder_full_path: "/Tracing",
            folder_level: "1",
            folder_uid: "tracing",
            parent_folder_uid: "",
            datasource_org: "Main Org.",
            datasource_org_id: "1",
            datasource_database: "",
            datasource_bucket: "",
            datasource_organization: "",
            datasource_index_pattern: "",
            metrics: &[],
            functions: &[],
            measurements: &["resource.service.name", "trace.id", "span.name"],
            buckets: &[],
        },
    );

    let report_document = test_support::build_export_inspection_query_report_document(&report);
    assert_eq!(report_document.summary.dashboard_count, 6);
    assert_eq!(report_document.summary.query_record_count, 6);
    assert_eq!(report_document.queries.len(), 6);

    let row = |ref_id: &str| {
        report_document
            .queries
            .iter()
            .find(|query| query.ref_id == ref_id)
            .unwrap()
    };
    assert_eq!(row("A").dashboard_uid, "prom-main");
    assert_eq!(row("A").dashboard_title, "Prometheus Main");
    assert_eq!(row("A").datasource_name, "Prometheus Main");
    assert_eq!(row("A").datasource_uid, "prom-main");
    assert_eq!(row("A").datasource_type, "prometheus");
    assert_eq!(row("A").datasource_family, "prometheus");
    assert_eq!(row("A").query_field, "expr");
    assert_eq!(
        row("A").query_text,
        "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket{job=\"api\",handler=\"/orders\"}[5m])) by (le))"
    );
    assert_eq!(
        row("A").metrics,
        vec!["http_request_duration_seconds_bucket".to_string()]
    );
    assert_eq!(
        row("A").functions,
        vec![
            "histogram_quantile".to_string(),
            "sum".to_string(),
            "rate".to_string()
        ]
    );
    assert_eq!(row("A").buckets, vec!["5m".to_string()]);
    assert_eq!(
        row("A").file_path,
        raw_dir
            .join("General")
            .join("prometheus.json")
            .display()
            .to_string()
    );

    assert_eq!(row("B").dashboard_uid, "logs-main");
    assert_eq!(row("B").datasource_name, "Loki Main");
    assert_eq!(row("B").datasource_uid, "loki-main");
    assert_eq!(row("B").datasource_type, "loki");
    assert_eq!(row("B").datasource_family, "loki");
    assert_eq!(row("B").query_field, "expr");
    assert_eq!(row("B").query_variables, Vec::<String>::new());
    assert_eq!(
        row("B").functions,
        vec![
            "sum".to_string(),
            "count_over_time".to_string(),
            "json".to_string(),
            "logfmt".to_string(),
            "line_filter_contains".to_string(),
            "line_filter_contains:timeout".to_string(),
        ]
    );
    assert_eq!(
        row("B").measurements,
        vec![
            "{job=\"grafana\",namespace!=\"kube-system\",cluster=~\"prod|stage\"}".to_string(),
            "job=\"grafana\"".to_string(),
            "namespace!=\"kube-system\"".to_string(),
            "cluster=~\"prod|stage\"".to_string(),
        ]
    );
    assert_eq!(row("B").buckets, vec!["10m".to_string()]);

    assert_eq!(row("C").datasource_type, "influxdb");
    assert_eq!(row("C").datasource_family, "flux");
    assert_eq!(row("C").query_field, "query");
    assert_eq!(row("C").buckets, vec!["prod".to_string(), "5m".to_string()]);

    assert_eq!(row("D").datasource_type, "postgres");
    assert_eq!(row("D").datasource_family, "sql");
    assert_eq!(row("D").query_field, "rawSql");
    assert_eq!(
        row("D").measurements,
        vec!["public.hosts".to_string(), "public.cpu_metrics".to_string()]
    );

    assert_eq!(row("E").datasource_type, "grafana-opensearch-datasource");
    assert_eq!(row("E").datasource_family, "search");
    assert_eq!(row("E").query_field, "query");
    assert_eq!(
        row("E").measurements,
        vec![
            "@timestamp".to_string(),
            "resource.service.name".to_string(),
            "status".to_string()
        ]
    );

    assert_eq!(row("F").datasource_type, "tempo");
    assert_eq!(row("F").datasource_family, "tracing");
    assert_eq!(row("F").query_field, "query");
    assert_eq!(
        row("F").measurements,
        vec![
            "resource.service.name".to_string(),
            "trace.id".to_string(),
            "span.name".to_string()
        ]
    );
}
