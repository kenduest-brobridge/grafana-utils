//! Inspect-live export parity regression tests for all-orgs governance coverage.
use super::test_support;
use super::test_support::{
    export_dashboards_with_request, ExportArgs, InspectExportArgs, InspectExportReportFormat,
    InspectLiveArgs, InspectOutputFormat,
};
use super::{
    all_orgs_inspect_live_request_fixture, assert_all_orgs_export_live_documents_match,
    make_common_args, read_json_output_file,
};
use serde_json::json;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn inspect_live_dashboards_with_request_all_orgs_aggregates_multiple_org_exports() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_one_raw = export_root.join("org_1_Main_Org").join("raw");
    let org_two_raw = export_root.join("org_2_Ops_Org").join("raw");
    fs::create_dir_all(&org_one_raw).unwrap();
    fs::create_dir_all(&org_two_raw).unwrap();
    fs::write(
        export_root.join(super::EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": super::TOOL_SCHEMA_VERSION,
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

    for (
        raw_dir,
        org_id,
        org_name,
        folder_uid,
        folder_title,
        dashboard_uid,
        dashboard_title,
        datasource_uid,
        datasource_name,
        file_name,
        query_text,
    ) in [
        (
            &org_one_raw,
            "1",
            "Main Org.",
            "general",
            "General",
            "cpu-main",
            "CPU Main",
            "prom-main",
            "Prometheus Main",
            "CPU_Main__cpu-main.json",
            "up",
        ),
        (
            &org_two_raw,
            "2",
            "Ops Org",
            "ops",
            "Ops",
            "latency-main",
            "Latency Main",
            "prom-two",
            "Prometheus Two",
            "Latency_Main__latency-main.json",
            "rate(http_requests_total[5m])",
        ),
    ] {
        fs::write(
            raw_dir.join(super::EXPORT_METADATA_FILENAME),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": super::TOOL_SCHEMA_VERSION,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "format": "grafana-web-import-preserve-uid",
                "foldersFile": super::FOLDER_INVENTORY_FILENAME,
                "datasourcesFile": super::DATASOURCE_INVENTORY_FILENAME,
                "org": org_name,
                "orgId": org_id
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join(super::FOLDER_INVENTORY_FILENAME),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": folder_uid,
                    "title": folder_title,
                    "path": folder_title,
                    "org": org_name,
                    "orgId": org_id
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join(super::DATASOURCE_INVENTORY_FILENAME),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": datasource_uid,
                    "name": datasource_name,
                    "type": "prometheus",
                    "access": "proxy",
                    "url": if org_id == "1" {
                        "http://prometheus:9090"
                    } else {
                        "http://prometheus-two:9090"
                    },
                    "isDefault": "true",
                    "org": org_name,
                    "orgId": org_id
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("index.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": dashboard_uid,
                    "title": dashboard_title,
                    "path": format!("{folder_title}/{file_name}"),
                    "format": "grafana-web-import-preserve-uid",
                    "org": org_name,
                    "orgId": org_id
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::create_dir_all(raw_dir.join(folder_title)).unwrap();
        fs::write(
            raw_dir.join(folder_title).join(file_name),
            serde_json::to_string_pretty(&json!({
                "dashboard": {
                    "id": if org_id == "1" { 11 } else { 12 },
                    "uid": dashboard_uid,
                    "title": dashboard_title,
                    "panels": [{
                        "id": if org_id == "1" { 7 } else { 8 },
                        "title": if org_id == "1" { "CPU Query" } else { "Latency Query" },
                        "type": "timeseries",
                        "datasource": {"uid": datasource_uid, "type": "prometheus"},
                        "targets": [{
                            "refId": "A",
                            "expr": query_text
                        }]
                    }]
                },
                "meta": {"folderUid": folder_uid, "folderTitle": folder_title}
            }))
            .unwrap(),
        )
        .unwrap();
    }

    let export_report_output = temp.path().join("export-report.json");
    let export_report_args = InspectExportArgs {
        input_dir: export_root.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::QueriesJson),
        report_columns: Vec::new(),
        list_columns: false,
        report_filter_datasource: None,
        report_filter_panel_id: None,
        help_full: false,
        no_header: false,
        output_file: Some(export_report_output.clone()),
        also_stdout: false,
        interactive: false,
    };
    let export_report_count = test_support::analyze_export_dir(&export_report_args).unwrap();
    let export_report_document = read_json_output_file(&export_report_output);

    let live_report_output = temp.path().join("live-report.json");
    let live_report_args = InspectLiveArgs {
        common: make_common_args("https://grafana.example.com".to_string()),
        page_size: 100,
        concurrency: 1,
        org_id: None,
        all_orgs: true,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::QueriesJson),
        report_columns: Vec::new(),
        list_columns: false,
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
        all_orgs_inspect_live_request_fixture(),
        &live_report_args,
    )
    .unwrap();
    let live_report_document = read_json_output_file(&live_report_output);

    let export_governance_output = temp.path().join("export-governance.json");
    let export_governance_args = InspectExportArgs {
        input_dir: export_root.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::GovernanceJson),
        report_columns: Vec::new(),
        list_columns: false,
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
        all_orgs: true,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::GovernanceJson),
        report_columns: Vec::new(),
        list_columns: false,
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
        all_orgs_inspect_live_request_fixture(),
        &live_governance_args,
    )
    .unwrap();
    let live_governance_document = read_json_output_file(&live_governance_output);

    let export_dependency_output = temp.path().join("export-dependency.json");
    let export_dependency_args = InspectExportArgs {
        input_dir: export_root.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::DependencyJson),
        report_columns: Vec::new(),
        list_columns: false,
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
        all_orgs: true,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::DependencyJson),
        report_columns: Vec::new(),
        list_columns: false,
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
        all_orgs_inspect_live_request_fixture(),
        &live_dependency_args,
    )
    .unwrap();
    let live_dependency_document = read_json_output_file(&live_dependency_output);

    assert_eq!(export_report_count, 2);
    assert_eq!(live_report_count, 2);
    assert_eq!(export_governance_count, 2);
    assert_eq!(live_governance_count, 2);
    assert_eq!(export_dependency_count, 2);
    assert_eq!(live_dependency_count, 2);

    assert_all_orgs_export_live_documents_match(
        &export_report_document,
        &live_report_document,
        &export_dependency_document,
        &live_dependency_document,
        &export_governance_document,
        &live_governance_document,
    );
    assert_eq!(
        export_report_document["summary"]["dashboardCount"],
        Value::from(2)
    );
    assert_eq!(
        export_report_document["summary"]["queryRecordCount"],
        Value::from(2)
    );
    assert_eq!(
        live_report_document["summary"]["dashboardCount"],
        Value::from(2)
    );
    assert_eq!(
        live_report_document["summary"]["queryRecordCount"],
        Value::from(2)
    );
    assert_eq!(
        export_dependency_document["summary"]["queryCount"],
        Value::from(2)
    );
    assert_eq!(
        export_dependency_document["summary"]["dashboardCount"],
        Value::from(2)
    );
    assert_eq!(
        export_dependency_document["summary"]["panelCount"],
        Value::from(2)
    );
    assert_eq!(
        export_dependency_document["summary"]["datasourceCount"],
        Value::from(2)
    );
    assert_eq!(
        export_dependency_document["summary"]["orphanedDatasourceCount"],
        Value::from(0)
    );
    assert_eq!(
        export_governance_document["summary"]["dashboardCount"],
        Value::from(2)
    );
    assert_eq!(
        export_governance_document["summary"]["queryRecordCount"],
        Value::from(2)
    );
    assert_eq!(
        live_governance_document["summary"]["dashboardCount"],
        Value::from(2)
    );
    assert_eq!(
        live_governance_document["summary"]["queryRecordCount"],
        Value::from(2)
    );
    assert_eq!(
        export_governance_document["summary"]["datasourceFamilyCount"],
        Value::from(1)
    );
    assert_eq!(
        export_governance_document["summary"]["riskRecordCount"],
        Value::from(1)
    );
    assert_eq!(
        live_governance_document["summary"]["datasourceFamilyCount"],
        Value::from(1)
    );
    assert_eq!(
        live_governance_document["summary"]["riskRecordCount"],
        Value::from(1)
    );
    assert_eq!(
        export_governance_document["dashboardDependencies"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        export_governance_document["dashboardDependencies"][0]["datasourceFamilies"],
        json!(["prometheus"])
    );
    assert_eq!(
        export_dependency_document["queries"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        export_dependency_document["datasourceUsage"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        export_dependency_document["kind"],
        Value::String("grafana-utils-dashboard-dependency-contract".to_string())
    );
    assert_eq!(
        export_dependency_document["summary"]["queryCount"],
        Value::from(2)
    );
    assert_eq!(
        export_dependency_document["summary"]["dashboardCount"],
        Value::from(2)
    );
    assert_eq!(
        export_dependency_document["summary"]["datasourceCount"],
        Value::from(2)
    );
    assert_eq!(
        export_dependency_document["summary"]["orphanedDatasourceCount"],
        Value::from(0)
    );
    assert_eq!(
        export_dependency_document["orphanedDatasources"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let dependency_rows = export_dependency_document["queries"].as_array().unwrap();
    let cpu_row = dependency_rows
        .iter()
        .find(|row| row["dashboardUid"] == Value::String("cpu-main".to_string()))
        .unwrap();
    assert_eq!(
        cpu_row["panelTitle"],
        Value::String("CPU Query".to_string())
    );
    assert_eq!(
        cpu_row["datasourceUid"],
        Value::String("prom-main".to_string())
    );
    assert_eq!(
        cpu_row["datasourceFamily"],
        Value::String("prometheus".to_string())
    );
    assert_eq!(cpu_row["queryField"], Value::String("expr".to_string()));
    assert!(cpu_row["analysis"]["metrics"].is_array());
    let latency_row = dependency_rows
        .iter()
        .find(|row| row["dashboardUid"] == Value::String("latency-main".to_string()))
        .unwrap();
    assert_eq!(
        latency_row["panelTitle"],
        Value::String("Latency Query".to_string())
    );
    assert_eq!(
        latency_row["datasourceUid"],
        Value::String("prom-two".to_string())
    );
    assert_eq!(
        latency_row["datasourceFamily"],
        Value::String("prometheus".to_string())
    );
    assert_eq!(latency_row["queryField"], Value::String("expr".to_string()));
    assert!(latency_row["analysis"]["metrics"].is_array());
    assert_eq!(
        export_dependency_document["datasourceUsage"][0]["queryFields"],
        json!(["expr"])
    );
    assert_eq!(
        export_dependency_document["datasourceUsage"][1]["queryFields"],
        json!(["expr"])
    );
}
