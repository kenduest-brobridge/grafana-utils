//! Live inspect core-family parity regressions.
use super::super::test_support::{
    self, InspectExportArgs, InspectExportReportFormat, InspectLiveArgs,
};
use super::{
    assert_governance_documents_match, assert_json_query_report_row_parity,
    core_family_inspect_live_request_fixture, make_common_args,
    normalize_queries_document_for_compare, read_json_output_file,
};
use serde_json::json;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn inspect_live_dashboards_with_request_reports_live_json_via_temp_raw_export() {
    let args = InspectLiveArgs {
        common: make_common_args("https://grafana.example.com".to_string()),
        page_size: 100,
        concurrency: 1,
        org_id: None,
        all_orgs: false,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        report: Some(InspectExportReportFormat::Json),
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: Some("prom-main".to_string()),
        report_filter_panel_id: Some("7".to_string()),
        progress: false,
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let count = test_support::inspect_live_dashboards_with_request(
        |method, path, _params, _payload| {
            let method_name = method.to_string();
            match (method, path) {
                (reqwest::Method::GET, "/api/org") => Ok(Some(json!({
                    "id": 1,
                    "name": "Main Org."
                }))),
                (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([
                    {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "access": "proxy",
                        "url": "http://prometheus:9090",
                        "isDefault": true
                    }
                ]))),
                (reqwest::Method::GET, "/api/search") => Ok(Some(json!([
                    {
                        "uid": "cpu-main",
                        "title": "CPU Main",
                        "type": "dash-db",
                        "folderUid": "general",
                        "folderTitle": "General"
                    }
                ]))),
                (reqwest::Method::GET, "/api/folders/general") => Ok(Some(json!({
                    "uid": "general",
                    "title": "General"
                }))),
                (reqwest::Method::GET, "/api/folders/general/permissions") => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                    "dashboard": {
                        "id": 11,
                        "uid": "cpu-main",
                        "title": "CPU Main",
                        "panels": [
                            {
                                "id": 7,
                                "title": "CPU Query",
                                "type": "timeseries",
                                "datasource": {"uid": "prom-main", "type": "prometheus"},
                                "targets": [
                                    {"refId": "A", "expr": "up"}
                                ]
                            },
                            {
                                "id": 8,
                                "title": "Memory Query",
                                "type": "timeseries",
                                "datasource": {"uid": "prom-main", "type": "prometheus"},
                                "targets": [
                                    {"refId": "A", "expr": "process_resident_memory_bytes"}
                                ]
                            }
                        ]
                    },
                    "meta": {}
                }))),
                (reqwest::Method::GET, "/api/dashboards/uid/cpu-main/permissions") => {
                    Ok(Some(json!([])))
                }
                _ => Err(test_support::message(format!(
                    "unexpected request {method_name} {path}"
                ))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn inspect_live_dashboards_with_request_matches_export_output_files_for_core_family_rows() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("export");
    fs::create_dir_all(export_root.join("General")).unwrap();

    let folder_inventory = json!([
        {
            "uid": "general",
            "title": "General",
            "path": "General",
            "org": "Main Org.",
            "orgId": "1"
        }
    ]);
    let datasource_inventory = json!([
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
    ]);
    let dashboard_payload = json!({
        "dashboard": {
            "id": 11,
            "uid": "core-main",
            "title": "Core Main",
            "panels": [
                {
                    "id": 7,
                    "title": "CPU Quantiles",
                    "type": "timeseries",
                    "datasource": {"uid": "prom-main", "type": "prometheus"},
                    "targets": [
                        {
                            "refId": "A",
                            "datasource": {"uid": "prom-main", "type": "prometheus"},
                            "expr": "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket{job=\"api\",handler=\"/orders\"}[5m])) by (le))"
                        }
                    ]
                },
                {
                    "id": 11,
                    "title": "Pipeline Errors",
                    "type": "logs",
                    "datasource": {"uid": "loki-main", "type": "loki"},
                    "targets": [
                        {
                            "refId": "B",
                            "datasource": {"uid": "loki-main", "type": "loki"},
                            "expr": "sum by (namespace) (count_over_time({job=\"grafana\",namespace!=\"kube-system\",cluster=~\"prod|stage\"} |= \"timeout\" | json | logfmt [10m]))"
                        }
                    ]
                },
                {
                    "id": 9,
                    "title": "Requests",
                    "type": "timeseries",
                    "datasource": {"uid": "influx-main", "type": "influxdb"},
                    "targets": [
                        {
                            "refId": "C",
                            "datasource": {"uid": "influx-main", "type": "influxdb"},
                            "query": "from(bucket: \"prod\") |> range(start: -1h) |> filter(fn: (r) => r._measurement == \"cpu\" and r.host == \"web-01\") |> aggregateWindow(every: 5m, fn: mean) |> yield(name: \"mean\")"
                        }
                    ]
                },
                {
                    "id": 13,
                    "title": "Host Ownership",
                    "type": "table",
                    "datasource": {"uid": "sql-main", "type": "postgres"},
                    "targets": [
                        {
                            "refId": "D",
                            "datasource": {"uid": "sql-main", "type": "postgres"},
                            "rawSql": "WITH recent_cpu AS (SELECT * FROM public.cpu_metrics) SELECT recent_cpu.host, hosts.owner FROM recent_cpu JOIN \"public\".\"hosts\" ON hosts.host = recent_cpu.host WHERE hosts.owner IS NOT NULL ORDER BY hosts.owner LIMIT 10"
                        }
                    ]
                },
                {
                    "id": 17,
                    "title": "OpenSearch Hits",
                    "type": "table",
                    "datasource": {"uid": "search-main", "type": "grafana-opensearch-datasource"},
                    "targets": [
                        {
                            "refId": "E",
                            "datasource": {"uid": "search-main", "type": "grafana-opensearch-datasource"},
                            "query": "_exists_:@timestamp AND resource.service.name:\"checkout\" AND status:[500 TO 599]"
                        }
                    ]
                },
                {
                    "id": 19,
                    "title": "Trace Search",
                    "type": "table",
                    "datasource": {"uid": "trace-main", "type": "tempo"},
                    "targets": [
                        {
                            "refId": "F",
                            "datasource": {"uid": "trace-main", "type": "tempo"},
                            "query": "resource.service.name:checkout AND trace.id:abc123 AND span.name:\"GET /orders\""
                        }
                    ]
                }
            ]
        },
        "meta": {
            "folderUid": "general",
            "folderTitle": "General"
        }
    });
    fs::write(
        export_root.join(super::super::EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": super::super::TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": super::super::FOLDER_INVENTORY_FILENAME,
            "datasourcesFile": super::super::DATASOURCE_INVENTORY_FILENAME
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        export_root.join(super::super::FOLDER_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&folder_inventory).unwrap(),
    )
    .unwrap();
    fs::write(
        export_root.join(super::super::DATASOURCE_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&datasource_inventory).unwrap(),
    )
    .unwrap();
    fs::write(
        export_root.join("General").join("core.json"),
        serde_json::to_string_pretty(&dashboard_payload).unwrap(),
    )
    .unwrap();

    let export_report_output = temp.path().join("export-report.json");
    let export_report_args = InspectExportArgs {
        import_dir: export_root.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        report: Some(InspectExportReportFormat::Json),
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: Some(export_report_output.clone()),
        also_stdout: false,
        interactive: false,
    };
    let export_report_count = test_support::analyze_export_dir(&export_report_args).unwrap();

    let live_report_output = temp.path().join("live-report.json");
    let live_report_args = InspectLiveArgs {
        common: make_common_args("https://grafana.example.com".to_string()),
        page_size: 100,
        concurrency: 1,
        org_id: None,
        all_orgs: false,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        report: Some(InspectExportReportFormat::Json),
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        progress: false,
        help_full: false,
        no_header: false,
        output_file: Some(live_report_output.clone()),
        also_stdout: false,
        interactive: false,
    };

    let live_report_count = test_support::inspect_live_dashboards_with_request(
        core_family_inspect_live_request_fixture(
            datasource_inventory.clone(),
            dashboard_payload.clone(),
        ),
        &live_report_args,
    )
    .unwrap();

    let export_report_document = read_json_output_file(&export_report_output);
    let live_report_document = read_json_output_file(&live_report_output);
    assert_eq!(export_report_count, 1);
    assert_eq!(live_report_count, 1);
    assert_eq!(
        normalize_queries_document_for_compare(&export_report_document),
        normalize_queries_document_for_compare(&live_report_document)
    );
    assert_eq!(
        export_report_document["summary"]["dashboardCount"],
        Value::from(1)
    );
    assert_eq!(
        export_report_document["summary"]["queryRecordCount"],
        Value::from(6)
    );

    for ref_id in ["A", "B", "C", "D", "E", "F"] {
        assert_json_query_report_row_parity(&export_report_document, &live_report_document, ref_id);
    }

    let export_governance_output = temp.path().join("export-governance.json");
    let export_governance_args = InspectExportArgs {
        import_dir: export_root.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        report: Some(InspectExportReportFormat::GovernanceJson),
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: Some(export_governance_output.clone()),
        also_stdout: false,
        interactive: false,
    };
    let export_governance_count =
        test_support::analyze_export_dir(&export_governance_args).unwrap();
    let export_governance_document = read_json_output_file(&export_governance_output);

    let live_governance_output = temp.path().join("live-governance.json");
    let live_governance_args = InspectLiveArgs {
        common: make_common_args("https://grafana.example.com".to_string()),
        page_size: 100,
        concurrency: 1,
        org_id: None,
        all_orgs: false,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        report: Some(InspectExportReportFormat::GovernanceJson),
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        progress: false,
        help_full: false,
        no_header: false,
        output_file: Some(live_governance_output.clone()),
        also_stdout: false,
        interactive: false,
    };
    let live_governance_count = test_support::inspect_live_dashboards_with_request(
        core_family_inspect_live_request_fixture(
            datasource_inventory.clone(),
            dashboard_payload.clone(),
        ),
        &live_governance_args,
    )
    .unwrap();
    let live_governance_document = read_json_output_file(&live_governance_output);

    let export_dependency_output = temp.path().join("export-dependency.json");
    let export_dependency_args = InspectExportArgs {
        import_dir: export_root.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        report: Some(InspectExportReportFormat::DependencyJson),
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: Some(export_dependency_output.clone()),
        also_stdout: false,
        interactive: false,
    };
    let export_dependency_count =
        test_support::analyze_export_dir(&export_dependency_args).unwrap();
    let export_dependency_document = read_json_output_file(&export_dependency_output);

    let live_dependency_output = temp.path().join("live-dependency.json");
    let live_dependency_args = InspectLiveArgs {
        common: make_common_args("https://grafana.example.com".to_string()),
        page_size: 100,
        concurrency: 1,
        org_id: None,
        all_orgs: false,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        report: Some(InspectExportReportFormat::DependencyJson),
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: None,
        progress: false,
        help_full: false,
        no_header: false,
        output_file: Some(live_dependency_output.clone()),
        also_stdout: false,
        interactive: false,
    };
    let live_dependency_count = test_support::inspect_live_dashboards_with_request(
        core_family_inspect_live_request_fixture(
            datasource_inventory.clone(),
            dashboard_payload.clone(),
        ),
        &live_dependency_args,
    )
    .unwrap();
    let live_dependency_document = read_json_output_file(&live_dependency_output);

    assert_eq!(export_governance_count, 1);
    assert_eq!(live_governance_count, 1);
    assert_governance_documents_match(&export_governance_document, &live_governance_document);
    assert_eq!(
        export_governance_document["summary"]["dashboardCount"],
        Value::from(1)
    );
    assert_eq!(
        export_governance_document["summary"]["queryRecordCount"],
        Value::from(6)
    );
    assert_eq!(
        export_governance_document["summary"]["datasourceFamilyCount"],
        Value::from(6)
    );
    assert_eq!(
        export_governance_document["summary"]["riskRecordCount"],
        Value::from(1)
    );
    assert_eq!(
        export_governance_document["dashboardDependencies"][0]["datasourceFamilies"],
        json!(["prometheus", "loki", "flux", "sql", "search", "tracing"])
    );
    assert_eq!(
        export_governance_document["dashboardDependencies"][0]["datasourceCount"],
        Value::from(6)
    );
    assert_eq!(
        export_governance_document["dashboardDependencies"][0]["datasourceFamilyCount"],
        Value::from(6)
    );

    assert_eq!(export_dependency_count, 1);
    assert_eq!(live_dependency_count, 1);
    assert_eq!(
        normalize_queries_document_for_compare(&export_dependency_document),
        normalize_queries_document_for_compare(&live_dependency_document)
    );
    assert_eq!(
        export_dependency_document["kind"],
        Value::String("grafana-utils-dashboard-dependency-contract".to_string())
    );
    assert_eq!(
        export_dependency_document["summary"]["queryCount"],
        Value::from(6)
    );
    assert_eq!(
        export_dependency_document["summary"]["dashboardCount"],
        Value::from(1)
    );
    assert_eq!(
        export_dependency_document["summary"]["panelCount"],
        Value::from(6)
    );
    assert_eq!(
        export_dependency_document["summary"]["datasourceCount"],
        Value::from(6)
    );
    assert_eq!(
        export_dependency_document["summary"]["orphanedDatasourceCount"],
        Value::from(0)
    );
    assert_eq!(
        export_dependency_document["queries"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    assert_eq!(
        export_dependency_document["datasourceUsage"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    assert_eq!(
        export_dependency_document["orphanedDatasources"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let dependency_row_a = export_dependency_document["queries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["refId"] == Value::String("A".to_string()))
        .unwrap();
    assert_eq!(
        dependency_row_a["dashboardUid"],
        Value::String("core-main".to_string())
    );
    assert_eq!(
        dependency_row_a["panelTitle"],
        Value::String("CPU Quantiles".to_string())
    );
    assert_eq!(
        dependency_row_a["datasourceUid"],
        Value::String("prom-main".to_string())
    );
    assert_eq!(
        dependency_row_a["datasourceFamily"],
        Value::String("prometheus".to_string())
    );
    assert_eq!(
        dependency_row_a["queryField"],
        Value::String("expr".to_string())
    );
    let dependency_analysis = dependency_row_a["analysis"].as_object().unwrap();
    for key in ["metrics", "measurements", "buckets", "labels"] {
        assert!(dependency_analysis.contains_key(key), "missing key {key}");
    }
    assert!(dependency_row_a["analysis"]["metrics"].is_array());
    assert!(dependency_row_a["analysis"]["measurements"].is_array());
    assert!(dependency_row_a["analysis"]["buckets"].is_array());
}
