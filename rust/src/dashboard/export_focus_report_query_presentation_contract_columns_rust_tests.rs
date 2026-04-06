//! Query presentation contract coverage for report columns and row output.
use super::{test_support, Value};

#[test]
fn resolve_report_column_ids_include_file_by_default_and_allow_datasource_uid() {
    let default_columns = test_support::resolve_report_column_ids(&[]).unwrap();
    assert!(default_columns.iter().any(|value| value == "file"));
    assert!(!default_columns
        .iter()
        .any(|value| value == "datasource_uid"));
    assert!(default_columns
        .iter()
        .any(|value| value == "datasource_type"));
    assert!(default_columns
        .iter()
        .any(|value| value == "datasource_family"));
    assert!(default_columns
        .iter()
        .any(|value| value == "dashboard_tags"));
    assert!(default_columns
        .iter()
        .any(|value| value == "panel_query_count"));
    assert!(default_columns
        .iter()
        .any(|value| value == "panel_datasource_count"));
    assert!(default_columns
        .iter()
        .any(|value| value == "panel_variables"));
    assert!(default_columns
        .iter()
        .any(|value| value == "query_variables"));

    let selected = test_support::resolve_report_column_ids(&[
        "dashboard_uid".to_string(),
        "datasource_uid".to_string(),
        "datasource_type".to_string(),
        "datasource_family".to_string(),
        "file".to_string(),
        "query".to_string(),
    ])
    .unwrap();
    assert_eq!(
        selected,
        vec![
            "dashboard_uid".to_string(),
            "datasource_uid".to_string(),
            "datasource_type".to_string(),
            "datasource_family".to_string(),
            "file".to_string(),
            "query".to_string(),
        ]
    );
}

#[test]
fn resolve_report_column_ids_for_format_defaults_csv_to_supported_columns() {
    let csv_columns = test_support::resolve_report_column_ids_for_format(
        Some(test_support::InspectExportReportFormat::Csv),
        &[],
    )
    .unwrap();
    assert!(csv_columns.iter().any(|value| value == "datasource_uid"));
    assert!(csv_columns
        .iter()
        .any(|value| value == "panel_target_count"));
    assert!(csv_columns.iter().any(|value| value == "target_hidden"));
    assert!(csv_columns.iter().any(|value| value == "target_disabled"));
    assert_eq!(
        csv_columns.len(),
        test_support::SUPPORTED_REPORT_COLUMN_IDS.len()
    );

    let table_columns = test_support::resolve_report_column_ids_for_format(
        Some(test_support::InspectExportReportFormat::Table),
        &[],
    )
    .unwrap();
    assert_eq!(
        table_columns,
        test_support::DEFAULT_REPORT_COLUMN_IDS
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>()
    );
}

#[test]
fn resolve_report_column_ids_accepts_json_style_aliases() {
    let selected = test_support::resolve_report_column_ids(&[
        "dashboardUid".to_string(),
        "dashboardTags".to_string(),
        "datasourceUid".to_string(),
        "datasourceType".to_string(),
        "datasourceFamily".to_string(),
        "panelQueryCount".to_string(),
        "panelDatasourceCount".to_string(),
        "panelVariables".to_string(),
        "queryField".to_string(),
        "queryVariables".to_string(),
        "file".to_string(),
    ])
    .unwrap();
    assert_eq!(
        selected,
        vec![
            "dashboard_uid".to_string(),
            "dashboard_tags".to_string(),
            "datasource_uid".to_string(),
            "datasource_type".to_string(),
            "datasource_family".to_string(),
            "panel_query_count".to_string(),
            "panel_datasource_count".to_string(),
            "panel_variables".to_string(),
            "query_field".to_string(),
            "query_variables".to_string(),
            "file".to_string(),
        ]
    );
}

#[test]
fn export_inspection_query_row_json_keeps_datasource_uid_and_file_fields() {
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
        datasource_uid: String::new(),
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

    let value = serde_json::to_value(&row).unwrap();

    assert_eq!(value["org"], Value::String("Main Org.".to_string()));
    assert_eq!(value["orgId"], Value::String("1".to_string()));
    assert_eq!(value["folderFullPath"], Value::String("/".to_string()));
    assert_eq!(value["folderLevel"], Value::String("1".to_string()));
    assert_eq!(value["datasourceUid"], Value::String(String::new()));
    assert_eq!(
        value["datasourceType"],
        Value::String("prometheus".to_string())
    );
    assert_eq!(
        value["datasourceFamily"],
        Value::String("prometheus".to_string())
    );
    assert_eq!(
        value["file"],
        Value::String("/tmp/raw/main.json".to_string())
    );
}

#[test]
fn resolve_report_column_ids_rejects_unknown_columns() {
    let error = test_support::resolve_report_column_ids(&["unknown".to_string()]).unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported --report-columns value"));
}

#[test]
fn resolve_report_column_ids_supports_all() {
    let columns = test_support::resolve_report_column_ids(&["all".to_string()]).unwrap();
    assert!(columns.contains(&"folder_full_path".to_string()));
    assert!(columns.contains(&"folder_level".to_string()));
    assert!(columns.contains(&"datasource_uid".to_string()));
    assert!(columns.contains(&"dashboard_tags".to_string()));
    assert!(columns.contains(&"panel_query_count".to_string()));
    assert!(columns.contains(&"panel_datasource_count".to_string()));
    assert!(columns.contains(&"panel_variables".to_string()));
    assert!(columns.contains(&"query_variables".to_string()));
    assert!(columns.contains(&"file".to_string()));
}

#[test]
fn report_format_supports_columns_matches_inspection_contract() {
    assert!(test_support::report_format_supports_columns(
        test_support::InspectExportReportFormat::Table
    ));
    assert!(test_support::report_format_supports_columns(
        test_support::InspectExportReportFormat::Csv
    ));
    assert!(test_support::report_format_supports_columns(
        test_support::InspectExportReportFormat::TreeTable
    ));
    assert!(!test_support::report_format_supports_columns(
        test_support::InspectExportReportFormat::QueriesJson
    ));
    assert!(!test_support::report_format_supports_columns(
        test_support::InspectExportReportFormat::Tree
    ));
}
