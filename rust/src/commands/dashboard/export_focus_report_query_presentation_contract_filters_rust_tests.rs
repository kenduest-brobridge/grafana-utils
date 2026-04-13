//! Query presentation contract coverage for filtering, grouping, and analyzer dispatch.
use super::{load_inspection_analyzer_cases, make_core_family_report_row, test_support};
use crate::dashboard::inspect::{
    dispatch_query_analysis, resolve_query_analyzer_family, QueryAnalysis, QueryExtractionContext,
};

#[test]
fn apply_query_report_filters_matches_core_family_aliases() {
    let make_row = |dashboard_uid: &str,
                    panel_id: &str,
                    ref_id: &str,
                    datasource_uid: &str,
                    datasource_name: &str,
                    datasource_type: &str,
                    datasource_family: &str| {
        make_core_family_report_row(
            dashboard_uid,
            panel_id,
            ref_id,
            datasource_uid,
            datasource_name,
            datasource_type,
            datasource_family,
            "placeholder",
            &[],
        )
    };
    let report = test_support::ExportInspectionQueryReport {
        input_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 6,
            panel_count: 6,
            query_count: 6,
            report_row_count: 6,
        },
        queries: vec![
            make_row(
                "prom-main",
                "1",
                "A",
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "prometheus",
            ),
            make_row(
                "logs-main",
                "2",
                "A",
                "logs-main",
                "Logs Main",
                "loki",
                "loki",
            ),
            make_row(
                "flux-main",
                "3",
                "A",
                "flux-main",
                "Influx Main",
                "influxdb",
                "flux",
            ),
            make_row(
                "sql-main",
                "4",
                "A",
                "sql-main",
                "Postgres Main",
                "postgres",
                "postgres",
            ),
            make_row(
                "search-main",
                "5",
                "A",
                "search-main",
                "Elastic Main",
                "elasticsearch",
                "search",
            ),
            make_row(
                "trace-main",
                "6",
                "A",
                "trace-main",
                "Tempo Main",
                "tempo",
                "tracing",
            ),
        ],
    };
    let cases = [
        ("prometheus", "prom-main"),
        ("loki", "logs-main"),
        ("flux", "flux-main"),
        ("postgres", "sql-main"),
        ("search", "search-main"),
        ("tracing", "trace-main"),
    ];

    for (filter_value, expected_dashboard_uid) in cases {
        let filtered =
            test_support::apply_query_report_filters(report.clone(), Some(filter_value), None);
        assert_eq!(filtered.queries.len(), 1);
        assert_eq!(filtered.queries[0].dashboard_uid, expected_dashboard_uid);
    }

    let rendered = test_support::render_grouped_query_report(&report).join("\n");
    assert!(rendered.contains("datasourceFamily=search"));
    assert!(rendered.contains("datasourceFamily=tracing"));
}

#[test]
fn dispatch_query_analysis_matches_shared_analyzer_fixture_cases() {
    for case in load_inspection_analyzer_cases() {
        let case_name = case["name"].as_str().unwrap();
        let expected_family = case["expectedFamily"].as_str().unwrap();
        let expected_analysis = &case["expectedAnalysis"];
        let panel = case["panel"].as_object().unwrap().clone();
        let target = case["target"].as_object().unwrap().clone();
        let query_field = case["queryField"].as_str().unwrap();
        let query_text = case["queryText"].as_str().unwrap();
        let context = QueryExtractionContext {
            panel: &panel,
            target: &target,
            query_field,
            query_text,
            resolved_datasource_type: "",
        };

        assert_eq!(
            resolve_query_analyzer_family(&context),
            expected_family,
            "case={case_name}"
        );
        assert_eq!(
            dispatch_query_analysis(&context),
            QueryAnalysis {
                metrics: expected_analysis["metrics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<String>>(),
                functions: expected_analysis["functions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<String>>(),
                measurements: expected_analysis["measurements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<String>>(),
                buckets: expected_analysis["buckets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<String>>(),
            },
            "case={case_name}"
        );
    }
}

#[test]
fn apply_query_report_filters_keep_matching_rows_only() {
    let report = test_support::ExportInspectionQueryReport {
        input_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 2,
            panel_count: 2,
            query_count: 2,
            report_row_count: 2,
        },
        queries: vec![
            test_support::ExportInspectionQueryRow {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_uid: "main".to_string(),
                dashboard_title: "Main".to_string(),
                dashboard_tags: Vec::new(),
                folder_path: "General".to_string(),
                folder_full_path: "/".to_string(),
                folder_level: "1".to_string(),
                folder_uid: "general".to_string(),
                parent_folder_uid: String::new(),
                panel_id: "1".to_string(),
                panel_title: "CPU".to_string(),
                panel_type: "timeseries".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "A".to_string(),
                datasource: "prom-main".to_string(),
                datasource_name: "prom-main".to_string(),
                datasource_uid: "prom-uid".to_string(),
                datasource_org: String::new(),
                datasource_org_id: String::new(),
                datasource_database: String::new(),
                datasource_bucket: String::new(),
                datasource_organization: String::new(),
                datasource_index_pattern: String::new(),
                datasource_type: "prometheus".to_string(),
                datasource_family: "prometheus".to_string(),
                query_field: "expr".to_string(),
                target_hidden: "false".to_string(),
                target_disabled: "false".to_string(),
                query_text: "up{job=\"grafana\"}".to_string(),
                query_variables: Vec::new(),
                metrics: vec!["up".to_string()],
                functions: Vec::new(),
                measurements: Vec::new(),
                buckets: Vec::new(),
                file_path: "/tmp/raw/main.json".to_string(),
            },
            test_support::ExportInspectionQueryRow {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_uid: "logs".to_string(),
                dashboard_title: "Logs".to_string(),
                dashboard_tags: Vec::new(),
                folder_path: "General".to_string(),
                folder_full_path: "/".to_string(),
                folder_level: "1".to_string(),
                folder_uid: "general".to_string(),
                parent_folder_uid: String::new(),
                panel_id: "2".to_string(),
                panel_title: "Logs".to_string(),
                panel_type: "logs".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "A".to_string(),
                datasource: "logs-main".to_string(),
                datasource_name: "logs-main".to_string(),
                datasource_uid: "logs-uid".to_string(),
                datasource_org: String::new(),
                datasource_org_id: String::new(),
                datasource_database: String::new(),
                datasource_bucket: String::new(),
                datasource_organization: String::new(),
                datasource_index_pattern: String::new(),
                datasource_type: "loki".to_string(),
                datasource_family: "loki".to_string(),
                query_field: "expr".to_string(),
                target_hidden: "false".to_string(),
                target_disabled: "false".to_string(),
                query_text: "{job=\"grafana\"}".to_string(),
                query_variables: Vec::new(),
                metrics: Vec::new(),
                functions: Vec::new(),
                measurements: Vec::new(),
                buckets: Vec::new(),
                file_path: "/tmp/raw/logs.json".to_string(),
            },
        ],
    };

    let filtered = test_support::apply_query_report_filters(report, Some("prom-main"), Some("1"));

    assert_eq!(filtered.summary.dashboard_count, 1);
    assert_eq!(filtered.summary.panel_count, 1);
    assert_eq!(filtered.summary.query_count, 1);
    assert_eq!(filtered.summary.report_row_count, 1);
    assert_eq!(filtered.queries.len(), 1);
    assert_eq!(filtered.queries[0].datasource, "prom-main");
    assert_eq!(filtered.queries[0].panel_id, "1");
}

#[test]
fn apply_query_report_filters_match_datasource_uid_type_and_family() {
    let report = test_support::ExportInspectionQueryReport {
        input_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 2,
            panel_count: 2,
            query_count: 2,
            report_row_count: 2,
        },
        queries: vec![
            test_support::ExportInspectionQueryRow {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_uid: "main".to_string(),
                dashboard_title: "Main".to_string(),
                dashboard_tags: Vec::new(),
                folder_path: "General".to_string(),
                folder_full_path: "/".to_string(),
                folder_level: "1".to_string(),
                folder_uid: "general".to_string(),
                parent_folder_uid: String::new(),
                panel_id: "1".to_string(),
                panel_title: "CPU".to_string(),
                panel_type: "timeseries".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "A".to_string(),
                datasource: "prom-main".to_string(),
                datasource_name: "prom-main".to_string(),
                datasource_uid: "prom-uid".to_string(),
                datasource_org: String::new(),
                datasource_org_id: String::new(),
                datasource_database: String::new(),
                datasource_bucket: String::new(),
                datasource_organization: String::new(),
                datasource_index_pattern: String::new(),
                datasource_type: "prometheus".to_string(),
                datasource_family: "prometheus".to_string(),
                query_field: "expr".to_string(),
                target_hidden: "false".to_string(),
                target_disabled: "false".to_string(),
                query_text: "up".to_string(),
                query_variables: Vec::new(),
                metrics: vec!["up".to_string()],
                functions: Vec::new(),
                measurements: Vec::new(),
                buckets: Vec::new(),
                file_path: "/tmp/raw/main.json".to_string(),
            },
            test_support::ExportInspectionQueryRow {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_uid: "logs".to_string(),
                dashboard_title: "Logs".to_string(),
                dashboard_tags: Vec::new(),
                folder_path: "General".to_string(),
                folder_full_path: "/".to_string(),
                folder_level: "1".to_string(),
                folder_uid: "general".to_string(),
                parent_folder_uid: String::new(),
                panel_id: "2".to_string(),
                panel_title: "Logs".to_string(),
                panel_type: "logs".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "A".to_string(),
                datasource: "logs-main".to_string(),
                datasource_name: "logs-main".to_string(),
                datasource_uid: "logs-uid".to_string(),
                datasource_org: String::new(),
                datasource_org_id: String::new(),
                datasource_database: String::new(),
                datasource_bucket: String::new(),
                datasource_organization: String::new(),
                datasource_index_pattern: String::new(),
                datasource_type: "loki".to_string(),
                datasource_family: "loki".to_string(),
                query_field: "expr".to_string(),
                target_hidden: "false".to_string(),
                target_disabled: "false".to_string(),
                query_text: "{job=\"grafana\"}".to_string(),
                query_variables: Vec::new(),
                metrics: Vec::new(),
                functions: Vec::new(),
                measurements: Vec::new(),
                buckets: Vec::new(),
                file_path: "/tmp/raw/logs.json".to_string(),
            },
        ],
    };

    let filtered_uid =
        test_support::apply_query_report_filters(report.clone(), Some("prom-uid"), None);
    assert_eq!(filtered_uid.queries.len(), 1);
    assert_eq!(filtered_uid.queries[0].dashboard_uid, "main");

    let filtered_type =
        test_support::apply_query_report_filters(report.clone(), Some("loki"), None);
    assert_eq!(filtered_type.queries.len(), 1);
    assert_eq!(filtered_type.queries[0].dashboard_uid, "logs");

    let filtered_family =
        test_support::apply_query_report_filters(report, Some("prometheus"), None);
    assert_eq!(filtered_family.queries.len(), 1);
    assert_eq!(filtered_family.queries[0].dashboard_uid, "main");
}

#[test]
fn apply_query_report_filters_matches_normalized_search_family() {
    let report = test_support::ExportInspectionQueryReport {
        input_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 1,
            panel_count: 1,
            query_count: 1,
            report_row_count: 1,
        },
        queries: vec![test_support::ExportInspectionQueryRow {
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
            dashboard_uid: "search-main".to_string(),
            dashboard_title: "Search Main".to_string(),
            dashboard_tags: Vec::new(),
            folder_path: "General".to_string(),
            folder_full_path: "/".to_string(),
            folder_level: "1".to_string(),
            folder_uid: "general".to_string(),
            parent_folder_uid: String::new(),
            panel_id: "11".to_string(),
            panel_title: "Errors".to_string(),
            panel_type: "table".to_string(),
            panel_target_count: 1,
            panel_query_count: 1,
            panel_datasource_count: 0,
            panel_variables: Vec::new(),
            ref_id: "E".to_string(),
            datasource: "elastic-main".to_string(),
            datasource_name: "Elastic Main".to_string(),
            datasource_uid: "elastic-main".to_string(),
            datasource_org: "Main Org.".to_string(),
            datasource_org_id: "1".to_string(),
            datasource_database: String::new(),
            datasource_bucket: String::new(),
            datasource_organization: String::new(),
            datasource_index_pattern: "[logs-]YYYY.MM.DD".to_string(),
            datasource_type: "elasticsearch".to_string(),
            datasource_family: "search".to_string(),
            query_field: "query".to_string(),
            target_hidden: "false".to_string(),
            target_disabled: "false".to_string(),
            query_text: "status:500".to_string(),
            query_variables: Vec::new(),
            metrics: Vec::new(),
            functions: Vec::new(),
            measurements: vec!["status".to_string()],
            buckets: Vec::new(),
            file_path: "/tmp/raw/search.json".to_string(),
        }],
    };

    let filtered_family =
        test_support::apply_query_report_filters(report.clone(), Some("search"), None);
    assert_eq!(filtered_family.queries.len(), 1);
    assert_eq!(filtered_family.queries[0].dashboard_uid, "search-main");

    let filtered_type =
        test_support::apply_query_report_filters(report, Some("elasticsearch"), None);
    assert_eq!(filtered_type.queries.len(), 1);
    assert_eq!(filtered_type.queries[0].dashboard_uid, "search-main");
}

#[test]
fn normalize_query_report_groups_rows_by_dashboard_then_panel() {
    let report = test_support::ExportInspectionQueryReport {
        input_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 2,
            panel_count: 3,
            query_count: 3,
            report_row_count: 3,
        },
        queries: vec![
            test_support::ExportInspectionQueryRow {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_uid: "main".to_string(),
                dashboard_title: "Main".to_string(),
                dashboard_tags: Vec::new(),
                folder_path: "General".to_string(),
                folder_full_path: "/".to_string(),
                folder_level: "1".to_string(),
                folder_uid: "general".to_string(),
                parent_folder_uid: String::new(),
                panel_id: "1".to_string(),
                panel_title: "CPU".to_string(),
                panel_type: "timeseries".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "A".to_string(),
                datasource: "prom-main".to_string(),
                datasource_name: "prom-main".to_string(),
                datasource_uid: "prom-main".to_string(),
                datasource_org: String::new(),
                datasource_org_id: String::new(),
                datasource_database: String::new(),
                datasource_bucket: String::new(),
                datasource_organization: String::new(),
                datasource_index_pattern: String::new(),
                datasource_type: "prometheus".to_string(),
                datasource_family: "prometheus".to_string(),
                query_field: "expr".to_string(),
                target_hidden: "false".to_string(),
                target_disabled: "false".to_string(),
                query_text: "up".to_string(),
                query_variables: Vec::new(),
                metrics: vec!["up".to_string()],
                functions: Vec::new(),
                measurements: Vec::new(),
                buckets: Vec::new(),
                file_path: "/tmp/raw/main.json".to_string(),
            },
            test_support::ExportInspectionQueryRow {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_uid: "main".to_string(),
                dashboard_title: "Main".to_string(),
                dashboard_tags: Vec::new(),
                folder_path: "General".to_string(),
                folder_full_path: "/".to_string(),
                folder_level: "1".to_string(),
                folder_uid: "general".to_string(),
                parent_folder_uid: String::new(),
                panel_id: "2".to_string(),
                panel_title: "Memory".to_string(),
                panel_type: "timeseries".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "B".to_string(),
                datasource: "prom-main".to_string(),
                datasource_name: "prom-main".to_string(),
                datasource_uid: "prom-main".to_string(),
                datasource_org: String::new(),
                datasource_org_id: String::new(),
                datasource_database: String::new(),
                datasource_bucket: String::new(),
                datasource_organization: String::new(),
                datasource_index_pattern: String::new(),
                datasource_type: "prometheus".to_string(),
                datasource_family: "prometheus".to_string(),
                query_field: "expr".to_string(),
                target_hidden: "false".to_string(),
                target_disabled: "false".to_string(),
                query_text: "process_resident_memory_bytes".to_string(),
                query_variables: Vec::new(),
                metrics: vec!["process_resident_memory_bytes".to_string()],
                functions: Vec::new(),
                measurements: Vec::new(),
                buckets: Vec::new(),
                file_path: "/tmp/raw/main.json".to_string(),
            },
            test_support::ExportInspectionQueryRow {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_uid: "logs".to_string(),
                dashboard_title: "Logs".to_string(),
                dashboard_tags: Vec::new(),
                folder_path: "Platform / Logs".to_string(),
                folder_full_path: "/Platform/Logs".to_string(),
                folder_level: "2".to_string(),
                folder_uid: "logs".to_string(),
                parent_folder_uid: "platform".to_string(),
                panel_id: "7".to_string(),
                panel_title: "Errors".to_string(),
                panel_type: "logs".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "A".to_string(),
                datasource: "loki-main".to_string(),
                datasource_name: "loki-main".to_string(),
                datasource_uid: "loki-main".to_string(),
                datasource_org: String::new(),
                datasource_org_id: String::new(),
                datasource_database: String::new(),
                datasource_bucket: String::new(),
                datasource_organization: String::new(),
                datasource_index_pattern: String::new(),
                datasource_type: "loki".to_string(),
                datasource_family: "loki".to_string(),
                query_field: "expr".to_string(),
                target_hidden: "false".to_string(),
                target_disabled: "false".to_string(),
                query_text: "{job=\"grafana\"}".to_string(),
                query_variables: Vec::new(),
                metrics: Vec::new(),
                functions: Vec::new(),
                measurements: Vec::new(),
                buckets: Vec::new(),
                file_path: "/tmp/raw/logs.json".to_string(),
            },
        ],
    };

    let normalized = test_support::normalize_query_report(&report);

    assert_eq!(normalized.input_dir, "/tmp/raw");
    assert_eq!(normalized.summary, report.summary);
    assert_eq!(normalized.dashboards.len(), 2);
    assert_eq!(normalized.dashboards[0].org, "Main Org.");
    assert_eq!(normalized.dashboards[0].org_id, "1");
    assert_eq!(normalized.dashboards[0].dashboard_uid, "main");
    assert_eq!(normalized.dashboards[0].file_path, "/tmp/raw/main.json");
    assert_eq!(
        normalized.dashboards[0].datasources,
        vec!["prom-main".to_string()]
    );
    assert_eq!(
        normalized.dashboards[0].datasource_families,
        vec!["prometheus".to_string()]
    );
    assert_eq!(normalized.dashboards[0].panels.len(), 2);
    assert_eq!(normalized.dashboards[0].panels[0].panel_id, "1");
    assert_eq!(
        normalized.dashboards[0].panels[0].datasources,
        vec!["prom-main".to_string()]
    );
    assert_eq!(
        normalized.dashboards[0].panels[0].datasource_families,
        vec!["prometheus".to_string()]
    );
    assert_eq!(
        normalized.dashboards[0].panels[0].query_fields,
        vec!["expr".to_string()]
    );
    assert_eq!(normalized.dashboards[0].panels[0].queries.len(), 1);
    assert_eq!(normalized.dashboards[1].dashboard_uid, "logs");
    assert_eq!(normalized.dashboards[1].panels[0].panel_title, "Errors");
}
