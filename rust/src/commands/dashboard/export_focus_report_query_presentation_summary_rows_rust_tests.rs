//! Summary-row generation coverage for query presentation reports.
use super::super::test_support;

#[test]
fn build_export_inspection_summary_rows_include_export_org_metadata() {
    let summary = test_support::ExportInspectionSummary {
        input_dir: "/tmp/raw".to_string(),
        export_org: Some("Main Org.".to_string()),
        export_org_id: Some("1".to_string()),
        dashboard_count: 2,
        folder_count: 2,
        panel_count: 3,
        query_count: 4,
        datasource_inventory_count: 3,
        orphaned_datasource_count: 1,
        mixed_dashboard_count: 1,
        folder_paths: Vec::new(),
        datasource_usage: Vec::new(),
        datasource_inventory: Vec::new(),
        orphaned_datasources: Vec::new(),
        mixed_dashboards: Vec::new(),
    };

    let rows = test_support::build_export_inspection_summary_rows(&summary);

    assert!(rows.contains(&vec!["export_org".to_string(), "Main Org.".to_string()]));
    assert!(rows.contains(&vec!["export_org_id".to_string(), "1".to_string()]));
    assert!(rows.contains(&vec!["dashboard_count".to_string(), "2".to_string()]));
}
