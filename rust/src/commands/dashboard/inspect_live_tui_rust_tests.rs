//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use super::test_support::make_core_family_report_row;
use super::*;

#[allow(dead_code)]
fn make_inspect_live_tui_fixture() -> (
    test_support::ExportInspectionSummary,
    test_support::inspect_governance::ExportInspectionGovernanceDocument,
    test_support::ExportInspectionQueryReport,
) {
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
        datasource_inventory: Vec::new(),
        orphaned_datasources: Vec::new(),
        mixed_dashboards: Vec::new(),
    };
    let query = make_core_family_report_row(
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
    let governance = test_support::inspect_governance::ExportInspectionGovernanceDocument {
        summary: test_support::inspect_governance::GovernanceSummary {
            dashboard_count: 1,
            query_record_count: 1,
            datasource_inventory_count: 1,
            datasource_family_count: 1,
            datasource_coverage_count: 1,
            dashboard_datasource_edge_count: 1,
            datasource_risk_coverage_count: 1,
            high_blast_radius_datasource_count: 0,
            dashboard_risk_coverage_count: 1,
            mixed_datasource_dashboard_count: 0,
            orphaned_datasource_count: 0,
            risk_record_count: 2,
            query_audit_count: 1,
            dashboard_audit_count: 0,
        },
        datasource_families: Vec::new(),
        dashboard_dependencies: Vec::new(),
        dashboard_governance: vec![test_support::inspect_governance::DashboardGovernanceRow {
            dashboard_uid: "cpu-main".to_string(),
            dashboard_title: "CPU Main".to_string(),
            folder_path: "General".to_string(),
            panel_count: 1,
            query_count: 1,
            datasource_count: 1,
            datasource_family_count: 1,
            datasources: vec!["prom-main".to_string()],
            datasource_families: vec!["prometheus".to_string()],
            mixed_datasource: false,
            risk_count: 1,
            risk_kinds: vec!["prometheus-query-cost-score".to_string()],
        }],
        dashboard_datasource_edges: Vec::new(),
        datasource_governance: Vec::new(),
        datasources: Vec::new(),
        risk_records: vec![test_support::inspect_governance::GovernanceRiskRow {
            kind: "prometheus-query-cost-score".to_string(),
            severity: "high".to_string(),
            category: "cost".to_string(),
            dashboard_uid: "cpu-main".to_string(),
            panel_id: "7".to_string(),
            datasource: "Prometheus Main".to_string(),
            detail: "cost=3".to_string(),
            recommendation: "Reduce expensive Prometheus query shapes before broad rollout."
                .to_string(),
        }],
        query_audits: vec![test_support::inspect_governance::QueryAuditRow {
            dashboard_uid: "cpu-main".to_string(),
            dashboard_title: "CPU Main".to_string(),
            folder_path: "General".to_string(),
            panel_id: "7".to_string(),
            panel_title: "CPU".to_string(),
            ref_id: "A".to_string(),
            datasource: "Prometheus Main".to_string(),
            datasource_uid: "prom-main".to_string(),
            datasource_family: "prometheus".to_string(),
            aggregation_depth: 0,
            regex_matcher_count: 0,
            estimated_series_risk: "low".to_string(),
            query_cost_score: 3,
            score: 2,
            severity: "medium".to_string(),
            reasons: vec![
                "broad-prometheus-selector".to_string(),
                "prometheus-query-cost-score".to_string(),
            ],
            recommendations: vec![
                "Add label filters to the Prometheus selector.".to_string(),
                "Trim costly aggregation and range windows.".to_string(),
            ],
        }],
        dashboard_audits: Vec::new(),
    };

    (summary, governance, report)
}

#[test]
fn build_inspect_live_tui_groups_summarizes_dashboard_query_and_risk_sections() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let groups = test_support::build_inspect_live_tui_groups(&summary, &governance, &report);

    assert_eq!(groups.len(), 4);
    assert_eq!(groups[0].label, "Overview");
    assert_eq!(groups[0].count, 1);
    assert_eq!(groups[1].label, "Findings");
    assert_eq!(groups[1].count, 2);
    assert_eq!(groups[2].label, "Queries");
    assert_eq!(groups[2].count, 1);
    assert_eq!(groups[3].label, "Dependencies");
    assert_eq!(groups[3].count, 0);
}

#[test]
fn inspect_live_group_order_uses_human_review_modes() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let groups = test_support::build_inspect_live_tui_groups(&summary, &governance, &report);

    let labels = groups
        .iter()
        .map(|group| group.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec!["Overview", "Findings", "Queries", "Dependencies"]
    );
}

#[test]
fn filter_inspect_live_tui_items_limits_items_to_selected_mode() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let overview_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "overview");
    let query_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "queries");
    let finding_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "findings");
    let dependency_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "dependencies");

    assert_eq!(overview_items.len(), 1);
    assert!(overview_items
        .iter()
        .all(|item| item.kind == "dashboard-summary"));
    assert_eq!(query_items.len(), 1);
    assert!(query_items.iter().all(|item| item.kind == "query"));
    assert_eq!(finding_items.len(), 2);
    assert!(finding_items.iter().any(|item| item.kind == "finding"));
    assert!(finding_items.iter().any(|item| item.kind == "query-review"));
    assert!(dependency_items.is_empty());
}

#[test]
fn build_inspect_workbench_document_adds_dependency_coverage_views() {
    let (summary, mut governance, report) = make_inspect_live_tui_fixture();
    governance.datasources = vec![test_support::inspect_governance::DatasourceCoverageRow {
        datasource_uid: "prom-main".to_string(),
        datasource: "Prometheus Main".to_string(),
        family: "prometheus".to_string(),
        query_count: 1,
        dashboard_count: 1,
        panel_count: 1,
        dashboard_uids: vec!["cpu-main".to_string()],
        query_fields: vec!["expr".to_string()],
        orphaned: false,
    }];
    governance.datasource_governance =
        vec![test_support::inspect_governance::DatasourceGovernanceRow {
            datasource_uid: "prom-main".to_string(),
            datasource: "Prometheus Main".to_string(),
            family: "prometheus".to_string(),
            query_count: 1,
            dashboard_count: 1,
            panel_count: 1,
            mixed_dashboard_count: 0,
            risk_count: 1,
            risk_kinds: vec!["prometheus-query-cost-score".to_string()],
            folder_count: 1,
            high_blast_radius: false,
            cross_folder: false,
            folder_paths: vec!["General".to_string()],
            dashboard_uids: vec!["cpu-main".to_string()],
            dashboard_titles: vec!["CPU Main".to_string()],
            orphaned: false,
        }];

    let document = test_support::build_inspect_workbench_document(
        "export artifacts",
        &summary,
        &governance,
        &report,
    );

    assert_eq!(document.groups.len(), 4);
    assert_eq!(document.groups[0].label, "Overview");
    assert_eq!(document.groups[1].label, "Findings");
    assert_eq!(document.groups[2].label, "Queries");
    assert_eq!(document.groups[3].label, "Dependencies");
    let dependency_group = document
        .groups
        .iter()
        .find(|group| group.kind == "dependencies")
        .expect("dependency group");
    assert_eq!(dependency_group.views.len(), 2);
    assert_eq!(dependency_group.views[0].label, "Usage Coverage");
    assert_eq!(dependency_group.views[1].label, "Finding Coverage");
    assert_eq!(dependency_group.views[0].items.len(), 1);
    assert_eq!(dependency_group.views[1].items.len(), 1);
    assert!(document.summary_lines[0].contains("Source=export artifacts"));
    assert!(document.summary_lines[2].contains("Overview"));
}

#[test]
fn overview_mode_items_use_human_dashboard_summary_kind() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let overview_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "overview");

    assert_eq!(overview_items[0].kind, "dashboard-summary");
}

#[test]
fn finding_mode_items_use_human_finding_kinds() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let finding_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "findings");

    assert!(finding_items
        .iter()
        .all(|item| { item.kind == "finding" || item.kind == "query-review" }));
}
