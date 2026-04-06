//! Query presentation rendering regression tests.
#![allow(unused_imports)]

use super::super::test_support;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn validate_inspect_export_report_args_rejects_panel_filter_without_report() {
    let args = test_support::InspectExportArgs {
        import_dir: PathBuf::from("./dashboards/raw"),
        input_type: None,
        input_format: test_support::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: None,
        report_filter_panel_id: Some("7".to_string()),
        help_full: false,
        no_header: false,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };

    let error = test_support::validate_inspect_export_report_args(&args).unwrap_err();
    assert!(error
        .to_string()
        .contains("--report-filter-panel-id is only supported together with table, csv, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output."));
}

#[test]
fn render_csv_uses_headers_and_escaping() {
    let lines = test_support::render_csv(
        &["DASHBOARD_UID", "QUERY"],
        &[vec![
            "mixed-main".to_string(),
            "{job=\"grafana\"},error".to_string(),
        ]],
    );

    assert_eq!(lines[0], "DASHBOARD_UID,QUERY");
    assert_eq!(lines[1], "mixed-main,\"{job=\"\"grafana\"\"},error\"");
}

#[test]
fn render_grouped_query_report_displays_dashboard_panel_and_query_tree() {
    let report = test_support::ExportInspectionQueryReport {
        import_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 1,
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
                panel_id: "7".to_string(),
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
                panel_id: "8".to_string(),
                panel_title: "Logs".to_string(),
                panel_type: "logs".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "B".to_string(),
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
                file_path: "/tmp/raw/main.json".to_string(),
            },
        ],
    };

    let lines = test_support::render_grouped_query_report(&report);
    let output = lines.join("\n");

    assert!(output.contains("Export inspection report: /tmp/raw"));
    assert!(output.contains("# Dashboard tree"));
    assert!(output.contains("[1] Dashboard: Main (uid=main, folder=General"));
    assert!(output.contains("datasources=prom-main,loki-main"));
    assert!(output.contains("families=prometheus,loki"));
    assert!(output.contains("folderUid=general"));
    assert!(output.contains("org=Main Org., orgId=1"));
    assert!(output.contains("  File: /tmp/raw/main.json"));
    assert!(output.contains("  Panel: CPU (id=7, type=timeseries, targets=1, queries=1, datasources=prom-main, families=prometheus, fields=expr)"));
    assert!(output.contains("  Panel: Logs (id=8, type=logs, targets=1, queries=1, datasources=loki-main, families=loki, fields=expr)"));
    assert!(output.contains(
        "    Query: refId=A datasource=prom-main datasourceName=prom-main datasourceUid=prom-main datasourceType=prometheus datasourceFamily=prometheus field=expr metrics=up"
    ));
    assert!(output.contains("      up"));
    assert!(output.contains(
        "    Query: refId=B datasource=loki-main datasourceName=loki-main datasourceUid=loki-main datasourceType=loki datasourceFamily=loki field=expr"
    ));
    assert!(output.contains("      {job=\"grafana\"}"));
}

#[test]
fn render_grouped_query_table_report_displays_dashboard_sections_with_tables() {
    let report = test_support::ExportInspectionQueryReport {
        import_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 1,
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
                panel_id: "7".to_string(),
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
                panel_id: "8".to_string(),
                panel_title: "Logs".to_string(),
                panel_type: "logs".to_string(),
                panel_target_count: 1,
                panel_query_count: 1,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "B".to_string(),
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
                file_path: "/tmp/raw/main.json".to_string(),
            },
        ],
    };

    let lines = test_support::render_grouped_query_table_report(
        &report,
        &[
            "panel_id".to_string(),
            "panel_title".to_string(),
            "datasource".to_string(),
            "query".to_string(),
        ],
        true,
    );
    let output = lines.join("\n");

    assert!(output.contains("# Dashboard sections"));
    assert!(output.contains("[1] Dashboard: Main (uid=main, folder=General"));
    assert!(output.contains("datasources=prom-main,loki-main"));
    assert!(output.contains("families=prometheus,loki"));
    assert!(output.contains("folderUid=general"));
    assert!(output.contains("org=Main Org., orgId=1"));
    assert!(output.contains("File: /tmp/raw/main.json"));
    assert!(output.contains("Panel: CPU (id=7, type=timeseries, targets=1, queries=1, datasources=prom-main, families=prometheus, fields=expr)"));
    assert!(output.contains("Panel: Logs (id=8, type=logs, targets=1, queries=1, datasources=loki-main, families=loki, fields=expr)"));
    assert!(output.contains("PANEL_ID  PANEL_TITLE  DATASOURCE  QUERY"));
    assert!(output.contains("7         CPU          prom-main   up"));
    assert!(output.contains("8         Logs         loki-main   {job=\"grafana\"}"));
}

#[test]
fn render_query_report_column_supports_org_columns() {
    let row = test_support::ExportInspectionQueryRow {
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
        panel_target_count: 0,
        panel_query_count: 0,
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
    };

    assert_eq!(test_support::report_column_header("org"), "ORG");
    assert_eq!(test_support::report_column_header("org_id"), "ORG_ID");
    assert_eq!(
        test_support::render_query_report_column(&row, "org"),
        "Main Org."
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "org_id"),
        "1"
    );
}

#[test]
fn render_query_report_column_supports_variable_and_count_columns() {
    let row = test_support::ExportInspectionQueryRow {
        dashboard_tags: vec!["prod".to_string(), "infra".to_string()],
        query_variables: vec!["host".to_string(), "job".to_string()],
        panel_variables: vec!["cluster".to_string(), "team".to_string()],
        panel_target_count: 3,
        panel_query_count: 2,
        panel_datasource_count: 1,
        target_hidden: "true".to_string(),
        target_disabled: "false".to_string(),
        ..Default::default()
    };

    assert_eq!(
        test_support::report_column_header("dashboard_tags"),
        "DASHBOARD_TAGS"
    );
    assert_eq!(
        test_support::report_column_header("query_variables"),
        "QUERY_VARIABLES"
    );
    assert_eq!(
        test_support::report_column_header("panel_variables"),
        "PANEL_VARIABLES"
    );
    assert_eq!(
        test_support::report_column_header("panel_target_count"),
        "PANEL_TARGET_COUNT"
    );
    assert_eq!(
        test_support::report_column_header("panel_query_count"),
        "PANEL_EFFECTIVE_QUERY_COUNT"
    );
    assert_eq!(
        test_support::report_column_header("panel_datasource_count"),
        "PANEL_TOTAL_DATASOURCE_COUNT"
    );
    assert_eq!(
        test_support::report_column_header("target_hidden"),
        "TARGET_HIDDEN"
    );
    assert_eq!(
        test_support::report_column_header("target_disabled"),
        "TARGET_DISABLED"
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "dashboard_tags"),
        "prod,infra"
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "query_variables"),
        "host,job"
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "panel_variables"),
        "cluster,team"
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "panel_target_count"),
        "3"
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "panel_query_count"),
        "2"
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "panel_datasource_count"),
        "1"
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "target_hidden"),
        "true"
    );
    assert_eq!(
        test_support::render_query_report_column(&row, "target_disabled"),
        "false"
    );
}

#[test]
fn render_grouped_query_table_report_includes_loki_analysis_columns() {
    let report = test_support::ExportInspectionQueryReport {
        import_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 1,
            panel_count: 1,
            query_count: 1,
            report_row_count: 1,
        },
        queries: vec![test_support::ExportInspectionQueryRow {
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
            dashboard_uid: "logs-main".to_string(),
            dashboard_title: "Logs Main".to_string(),
            dashboard_tags: Vec::new(),
            folder_path: "Logs".to_string(),
            folder_full_path: "/Logs".to_string(),
            folder_level: "1".to_string(),
            folder_uid: "logs".to_string(),
            parent_folder_uid: String::new(),
            panel_id: "11".to_string(),
            panel_title: "Errors".to_string(),
            panel_type: "logs".to_string(),
            panel_target_count: 0,
            panel_query_count: 0,
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
            query_text: "{job=\"varlogs\",app=~\"api|web\"} |= \"error\" | json [5m]".to_string(),
            query_variables: Vec::new(),
            metrics: Vec::new(),
            functions: vec![
                "sum".to_string(),
                "count_over_time".to_string(),
                "filter_eq".to_string(),
                "json".to_string(),
            ],
            measurements: vec![
                "job=\"varlogs\"".to_string(),
                "app=~\"api|web\"".to_string(),
            ],
            buckets: vec!["5m".to_string()],
            file_path: "/tmp/raw/logs-main.json".to_string(),
        }],
    };

    let lines = test_support::render_grouped_query_table_report(
        &report,
        &[
            "panel_id".to_string(),
            "datasource".to_string(),
            "functions".to_string(),
            "measurements".to_string(),
            "buckets".to_string(),
            "query".to_string(),
        ],
        true,
    );
    let output = lines.join("\n");

    assert!(output.contains("PANEL_ID  DATASOURCE  FUNCTIONS"));
    assert!(output.contains("11"));
    assert!(output.contains("loki-main"));
    assert!(output.contains("sum,count_over_time,filter_eq,json"));
    assert!(output.contains("job=\"varlogs\",app=~\"api|web\""));
    assert!(output.contains("5m"));
    assert!(output.contains("{job=\"varlogs\",app=~\"api|web\"} |= \"error\" | json [5m]"));
}

#[test]
fn render_grouped_query_table_report_uses_default_column_set_when_requested() {
    let columns = test_support::resolve_report_column_ids(&[]).unwrap();
    assert_eq!(
        columns,
        test_support::DEFAULT_REPORT_COLUMN_IDS
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>()
    );
}
