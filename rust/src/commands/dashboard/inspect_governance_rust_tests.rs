//! Feature-oriented governance inspect regressions.
//! Keeps the governance report and risk-registry coverage separate from the large dashboard test file.
use super::test_support;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn build_export_inspection_governance_document_summarizes_families_and_risks() {
    let summary = test_support::ExportInspectionSummary {
        input_dir: "/tmp/raw".to_string(),
        export_org: None,
        export_org_id: None,
        dashboard_count: 2,
        folder_count: 2,
        panel_count: 3,
        query_count: 3,
        datasource_inventory_count: 3,
        orphaned_datasource_count: 1,
        mixed_dashboard_count: 1,
        folder_paths: Vec::new(),
        datasource_usage: Vec::new(),
        datasource_inventory: vec![
            test_support::DatasourceInventorySummary {
                uid: "prom-main".to_string(),
                name: "Prometheus Main".to_string(),
                datasource_type: "prometheus".to_string(),
                access: "proxy".to_string(),
                url: "http://prometheus:9090".to_string(),
                is_default: "true".to_string(),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                reference_count: 1,
                dashboard_count: 1,
            },
            test_support::DatasourceInventorySummary {
                uid: "logs-main".to_string(),
                name: "Logs Main".to_string(),
                datasource_type: "loki".to_string(),
                access: "proxy".to_string(),
                url: "http://loki:3100".to_string(),
                is_default: "false".to_string(),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                reference_count: 1,
                dashboard_count: 1,
            },
            test_support::DatasourceInventorySummary {
                uid: "unused-main".to_string(),
                name: "Unused Main".to_string(),
                datasource_type: "tempo".to_string(),
                access: "proxy".to_string(),
                url: "http://tempo:3200".to_string(),
                is_default: "false".to_string(),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                reference_count: 0,
                dashboard_count: 0,
            },
        ],
        orphaned_datasources: vec![test_support::DatasourceInventorySummary {
            uid: "unused-main".to_string(),
            name: "Unused Main".to_string(),
            datasource_type: "tempo".to_string(),
            access: "proxy".to_string(),
            url: "http://tempo:3200".to_string(),
            is_default: "false".to_string(),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
            reference_count: 0,
            dashboard_count: 0,
        }],
        mixed_dashboards: vec![test_support::MixedDashboardSummary {
            uid: "mixed-main".to_string(),
            title: "Mixed Main".to_string(),
            folder_path: "Platform / Infra".to_string(),
            datasource_count: 2,
            datasources: vec!["custom-main".to_string(), "logs-main".to_string()],
        }],
    };
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
                dashboard_uid: "cpu-main".to_string(),
                dashboard_title: "CPU Main".to_string(),
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
                file_path: "/tmp/raw/cpu-main.json".to_string(),
            },
            test_support::ExportInspectionQueryRow {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_uid: "mixed-main".to_string(),
                dashboard_title: "Mixed Main".to_string(),
                dashboard_tags: Vec::new(),
                folder_path: "Platform / Infra".to_string(),
                folder_full_path: "/Platform/Infra".to_string(),
                folder_level: "2".to_string(),
                folder_uid: "infra".to_string(),
                parent_folder_uid: "platform".to_string(),
                panel_id: "8".to_string(),
                panel_title: "Logs".to_string(),
                panel_type: "logs".to_string(),
                panel_target_count: 0,
                panel_query_count: 0,
                panel_datasource_count: 0,
                panel_variables: Vec::new(),
                ref_id: "B".to_string(),
                datasource: "custom-main".to_string(),
                datasource_name: "custom-main".to_string(),
                datasource_uid: String::new(),
                datasource_org: String::new(),
                datasource_org_id: String::new(),
                datasource_database: String::new(),
                datasource_bucket: String::new(),
                datasource_organization: String::new(),
                datasource_index_pattern: String::new(),
                datasource_type: "custom-plugin".to_string(),
                datasource_family: "unknown".to_string(),
                query_field: "query".to_string(),
                target_hidden: "false".to_string(),
                target_disabled: "false".to_string(),
                query_text: "custom_query".to_string(),
                query_variables: Vec::new(),
                metrics: Vec::new(),
                functions: Vec::new(),
                measurements: Vec::new(),
                buckets: Vec::new(),
                file_path: "/tmp/raw/mixed-main.json".to_string(),
            },
        ],
    };

    let document = test_support::build_export_inspection_governance_document(&summary, &report);

    assert_eq!(document.summary.dashboard_count, 2);
    assert_eq!(document.summary.query_record_count, 2);
    assert_eq!(document.summary.datasource_family_count, 3);
    assert_eq!(document.summary.dashboard_datasource_edge_count, 2);
    assert_eq!(document.summary.datasource_risk_coverage_count, 2);
    assert_eq!(document.summary.dashboard_risk_coverage_count, 2);
    assert_eq!(document.summary.risk_record_count, 4);
    assert_eq!(document.dashboard_dependencies.len(), 2);
    assert_eq!(document.dashboard_governance.len(), 2);
    assert_eq!(document.datasource_governance.len(), 4);
    assert_eq!(document.risk_records.len(), 4);
    assert!(document
        .dashboard_governance
        .iter()
        .any(|row| row.dashboard_uid == "cpu-main" && row.dashboard_title == "CPU Main"));
    assert!(document
        .dashboard_governance
        .iter()
        .any(|row| row.dashboard_uid == "mixed-main" && row.dashboard_title == "Mixed Main"));
    assert!(document
        .datasource_governance
        .iter()
        .any(|row| row.datasource_uid == "prom-main"));
    assert!(document
        .dashboard_dependencies
        .iter()
        .any(|row| row.dashboard_uid == "cpu-main"));
    assert!(document
        .dashboard_dependencies
        .iter()
        .any(|row| row.dashboard_uid == "mixed-main"));
}

#[test]
fn build_export_inspection_governance_document_flags_broad_loki_selectors() {
    let summary = test_support::ExportInspectionSummary {
        input_dir: "/tmp/raw".to_string(),
        export_org: Some("Main Org.".to_string()),
        export_org_id: Some("1".to_string()),
        dashboard_count: 1,
        folder_count: 1,
        panel_count: 1,
        query_count: 1,
        datasource_inventory_count: 1,
        orphaned_datasource_count: 0,
        mixed_dashboard_count: 0,
        folder_paths: Vec::new(),
        datasource_usage: Vec::new(),
        datasource_inventory: vec![test_support::DatasourceInventorySummary {
            uid: "logs-main".to_string(),
            name: "Logs Main".to_string(),
            datasource_type: "loki".to_string(),
            access: "proxy".to_string(),
            url: "http://loki:3100".to_string(),
            is_default: "false".to_string(),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
            reference_count: 1,
            dashboard_count: 1,
        }],
        orphaned_datasources: Vec::new(),
        mixed_dashboards: Vec::new(),
    };
    let mut query = test_support::make_core_family_report_row(
        "logs-main",
        "7",
        "A",
        "logs-main",
        "Logs Main",
        "loki",
        "loki",
        r#"{} |= "timeout""#,
        &["{}"],
    );
    query.functions = vec!["line_filter_contains".to_string()];
    query.measurements = vec!["{}".to_string()];

    let report = test_support::ExportInspectionQueryReport {
        input_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 1,
            panel_count: 1,
            query_count: 1,
            report_row_count: 1,
        },
        queries: vec![query],
    };

    let document = test_support::build_export_inspection_governance_document(&summary, &report);

    assert_eq!(document.summary.risk_record_count, 2);
    let risk = document
        .risk_records
        .iter()
        .find(|item| item.kind == "broad-loki-selector")
        .unwrap();
    assert_eq!(risk.kind, "broad-loki-selector");
    assert_eq!(risk.category, "cost");
    assert_eq!(risk.severity, "medium");
    assert_eq!(risk.dashboard_uid, "logs-main");
    assert_eq!(risk.panel_id, "7");
    assert_eq!(risk.datasource, "Logs Main");
    assert_eq!(risk.detail, "{}");
    assert!(risk
        .recommendation
        .contains("Narrow the Loki stream selector"));
}

#[test]
fn build_export_inspection_governance_document_keeps_known_family_without_inventory_match() {
    let summary = test_support::ExportInspectionSummary {
        input_dir: "/tmp/raw".to_string(),
        export_org: Some("Main Org.".to_string()),
        export_org_id: Some("1".to_string()),
        dashboard_count: 1,
        folder_count: 1,
        panel_count: 1,
        query_count: 1,
        datasource_inventory_count: 0,
        orphaned_datasource_count: 0,
        mixed_dashboard_count: 0,
        folder_paths: Vec::new(),
        datasource_usage: Vec::new(),
        datasource_inventory: Vec::new(),
        orphaned_datasources: Vec::new(),
        mixed_dashboards: Vec::new(),
    };
    let report = test_support::ExportInspectionQueryReport {
        input_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 1,
            panel_count: 1,
            query_count: 1,
            report_row_count: 1,
        },
        queries: vec![test_support::make_core_family_report_row(
            "elastic-main",
            "7",
            "A",
            "",
            "dehk4kxat5la8b",
            "prometheus",
            "prometheus",
            "label_values(up, job)",
            &[],
        )],
    };

    let document = test_support::build_export_inspection_governance_document(&summary, &report);
    let dashboard = document
        .dashboard_governance
        .iter()
        .find(|row| row.dashboard_uid == "elastic-main")
        .expect("dashboard row should exist");

    assert_eq!(
        dashboard.datasource_families,
        vec!["prometheus".to_string()]
    );
    assert!(!dashboard
        .datasource_families
        .iter()
        .any(|family| family == "unknown"));
}

#[test]
fn build_export_inspection_governance_document_flags_query_quality_and_dashboard_pressure() {
    let temp = tempdir().unwrap();
    let dashboard_path = temp.path().join("cpu-main.json");
    fs::write(
        &dashboard_path,
        serde_json::to_vec_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "refresh": "5s"
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let summary = test_support::ExportInspectionSummary {
        input_dir: temp.path().display().to_string(),
        export_org: Some("Main Org.".to_string()),
        export_org_id: Some("1".to_string()),
        dashboard_count: 1,
        folder_count: 1,
        panel_count: 31,
        query_count: 4,
        datasource_inventory_count: 2,
        orphaned_datasource_count: 0,
        mixed_dashboard_count: 0,
        folder_paths: Vec::new(),
        datasource_usage: Vec::new(),
        datasource_inventory: vec![
            test_support::DatasourceInventorySummary {
                uid: "prom-main".to_string(),
                name: "Prometheus Main".to_string(),
                datasource_type: "prometheus".to_string(),
                access: "proxy".to_string(),
                url: "http://prometheus:9090".to_string(),
                is_default: "true".to_string(),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                reference_count: 3,
                dashboard_count: 1,
            },
            test_support::DatasourceInventorySummary {
                uid: "logs-main".to_string(),
                name: "Logs Main".to_string(),
                datasource_type: "loki".to_string(),
                access: "proxy".to_string(),
                url: "http://loki:3100".to_string(),
                is_default: "false".to_string(),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                reference_count: 1,
                dashboard_count: 1,
            },
        ],
        orphaned_datasources: Vec::new(),
        mixed_dashboards: Vec::new(),
    };

    let mut broad = test_support::make_core_family_report_row(
        "cpu-main",
        "7",
        "A",
        "prom-main",
        "Prometheus Main",
        "prometheus",
        "prometheus",
        "up",
        &[],
    );
    broad.file_path = dashboard_path.display().to_string();
    broad.metrics = vec!["up".to_string()];

    let mut regex = test_support::make_core_family_report_row(
        "cpu-main",
        "8",
        "B",
        "prom-main",
        "Prometheus Main",
        "prometheus",
        "prometheus",
        r#"sum(max by(instance) (rate(http_requests_total{instance=~"api|web"}[5m])))"#,
        &["instance=~\"api|web\""],
    );
    regex.file_path = dashboard_path.display().to_string();
    regex.metrics = vec!["http_requests_total".to_string()];
    regex.functions = vec!["sum".to_string(), "max".to_string(), "rate".to_string()];
    regex.buckets = vec!["5m".to_string()];

    let mut large_range = test_support::make_core_family_report_row(
        "cpu-main",
        "9",
        "C",
        "prom-main",
        "Prometheus Main",
        "prometheus",
        "prometheus",
        "sum(rate(process_cpu_seconds_total[6h]))",
        &[],
    );
    large_range.file_path = dashboard_path.display().to_string();
    large_range.metrics = vec!["process_cpu_seconds_total".to_string()];
    large_range.functions = vec!["sum".to_string(), "rate".to_string()];
    large_range.buckets = vec!["6h".to_string()];

    let mut loki = test_support::make_core_family_report_row(
        "cpu-main",
        "10",
        "D",
        "logs-main",
        "Logs Main",
        "loki",
        "loki",
        r#"{} |= "error""#,
        &["{}"],
    );
    loki.file_path = dashboard_path.display().to_string();
    loki.functions = vec!["line_filter_contains".to_string()];
    loki.measurements = vec!["{}".to_string()];

    let mut queries = vec![broad, regex, large_range, loki];
    for panel in 11..=37 {
        let mut extra = test_support::make_core_family_report_row(
            "cpu-main",
            &panel.to_string(),
            "Z",
            "prom-main",
            "Prometheus Main",
            "prometheus",
            "prometheus",
            "up",
            &[],
        );
        extra.file_path = dashboard_path.display().to_string();
        extra.metrics = vec!["up".to_string()];
        queries.push(extra);
    }
    let report = test_support::ExportInspectionQueryReport {
        input_dir: temp.path().display().to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 1,
            panel_count: 31,
            query_count: queries.len(),
            report_row_count: queries.len(),
        },
        queries,
    };

    let document = test_support::build_export_inspection_governance_document(&summary, &report);
    assert_eq!(document.summary.query_audit_count, 31);
    assert_eq!(document.summary.dashboard_audit_count, 1);
    assert!(document.query_audits.iter().any(|item| item
        .reasons
        .contains(&"broad-prometheus-selector".to_string())));
    assert!(document
        .query_audits
        .iter()
        .any(|item| item.reasons.contains(&"unscoped-loki-search".to_string())));
    let regex_audit = document
        .query_audits
        .iter()
        .find(|item| item.ref_id == "B")
        .unwrap();
    assert_eq!(regex_audit.aggregation_depth, 2);
    assert_eq!(regex_audit.regex_matcher_count, 1);
    assert_eq!(regex_audit.estimated_series_risk, "high");
    assert!(regex_audit.query_cost_score >= 2);
    let long_audit = document
        .query_audits
        .iter()
        .find(|item| item.ref_id == "C")
        .unwrap();
    assert_eq!(long_audit.query_cost_score, long_audit.score);
    assert_eq!(document.dashboard_audits.len(), 1);
    assert_eq!(
        document.dashboard_audits[0].reasons,
        vec![
            "dashboard-panel-pressure".to_string(),
            "dashboard-refresh-pressure".to_string()
        ]
    );
    let kinds = document
        .risk_records
        .iter()
        .map(|item| item.kind.as_str())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"broad-prometheus-selector"));
    assert!(kinds.contains(&"prometheus-regex-heavy"));
    assert!(kinds.contains(&"prometheus-high-cardinality-regex"));
    assert!(kinds.contains(&"prometheus-deep-aggregation"));
    assert!(kinds.contains(&"large-prometheus-range"));
    assert!(kinds.contains(&"unscoped-loki-search"));
    assert!(kinds.contains(&"dashboard-panel-pressure"));
    assert!(kinds.contains(&"dashboard-refresh-pressure"));
}

#[test]
fn governance_risk_metadata_registry_covers_known_kinds() {
    let cases = [
        (
            "mixed-datasource-dashboard",
            "topology",
            "medium",
            "Split panel queries by datasource or document why mixed datasource composition is required.",
        ),
        (
            "orphaned-datasource",
            "inventory",
            "low",
            "Remove the unused datasource or reattach it to retained dashboards before the next cleanup cycle.",
        ),
        (
            "unknown-datasource-family",
            "coverage",
            "medium",
            "Map this datasource plugin type to a known governance family or extend analyzer support for it.",
        ),
        (
            "empty-query-analysis",
            "coverage",
            "low",
            "Review the query text and extend analyzer coverage if this datasource family should emit governance signals.",
        ),
        (
            "broad-loki-selector",
            "cost",
            "medium",
            "Narrow the Loki stream selector before running expensive line filters or aggregations.",
        ),
        (
            "broad-prometheus-selector",
            "cost",
            "medium",
            "Add label filters to the Prometheus selector before promoting this dashboard to shared or high-refresh use.",
        ),
        (
            "prometheus-regex-heavy",
            "cost",
            "medium",
            "Reduce Prometheus regex matcher scope or replace it with exact labels where possible.",
        ),
        (
            "prometheus-high-cardinality-regex",
            "cost",
            "high",
            "Avoid regex matchers on high-cardinality Prometheus labels such as instance, pod, or container unless the scope is already tightly bounded.",
        ),
        (
            "prometheus-deep-aggregation",
            "cost",
            "medium",
            "Reduce nested Prometheus aggregation layers or pre-aggregate upstream before adding more dashboard fanout.",
        ),
        (
            "large-prometheus-range",
            "cost",
            "medium",
            "Shorten the Prometheus range window or pre-aggregate the series before using long lookback queries in dashboards.",
        ),
        (
            "unscoped-loki-search",
            "cost",
            "high",
            "Add at least one concrete Loki label matcher before running full-text or regex log search.",
        ),
        (
            "dashboard-panel-pressure",
            "dashboard-load",
            "medium",
            "Split the dashboard into smaller views or collapse low-value panels before broad rollout.",
        ),
        (
            "dashboard-refresh-pressure",
            "dashboard-load",
            "medium",
            "Increase the dashboard refresh interval to reduce repeated load on Grafana and backing datasources.",
        ),
    ];

    for (kind, category, severity, recommendation) in cases {
        let metadata = test_support::inspect_governance::governance_risk_spec(kind);
        assert_eq!(metadata.0, category);
        assert_eq!(metadata.1, severity);
        assert_eq!(metadata.2, recommendation);
    }

    assert_eq!(
        test_support::inspect_governance::governance_risk_spec("custom-governance-kind"),
        (
            "other",
            "low",
            "Review this governance finding and assign a follow-up owner if action is needed.",
        )
    );
}
