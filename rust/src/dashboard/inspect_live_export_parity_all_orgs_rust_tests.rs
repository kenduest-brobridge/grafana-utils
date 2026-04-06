//! Inspect-live export parity regression tests for the all-org governance case.
#![allow(unused_imports)]

use super::super::test_support;
use super::super::test_support::{
    export_dashboards_with_request, ExportArgs, InspectExportArgs, InspectExportReportFormat,
    InspectLiveArgs, InspectOutputFormat,
};
use super::super::{
    assert_all_orgs_export_live_documents_match, assert_governance_documents_match,
    make_common_args, read_json_output_file, DATASOURCE_INVENTORY_FILENAME,
    EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME, TOOL_SCHEMA_VERSION,
};
use serde_json::json;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn inspect_live_dashboards_with_request_all_orgs_matches_export_root_governance_contract() {
    let temp = tempdir().unwrap();
    let export_dir = temp.path().join("dashboards");
    let export_args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: export_dir.clone(),
        page_size: 500,
        org_id: None,
        all_orgs: true,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: true,
        include_history: false,
        provisioning_provider_name: "grafana-utils-dashboards".to_string(),
        provisioning_provider_org_id: None,
        provisioning_provider_path: None,
        provisioning_provider_disable_deletion: false,
        provisioning_provider_allow_ui_updates: false,
        provisioning_provider_update_interval_seconds: 30,
        dry_run: false,
        progress: false,
        verbose: false,
    };
    let inspect_root_temp = tempdir().unwrap();

    let mut request_fixture = |method: reqwest::Method,
                               path: &str,
                               params: &[(String, String)],
                               _payload: Option<&Value>|
     -> crate::common::Result<Option<Value>> {
        let scoped_org = params
            .iter()
            .find(|(key, _)| key == "orgId")
            .map(|(_, value)| value.as_str());
        match (method.clone(), path, scoped_org) {
            (reqwest::Method::GET, "/api/orgs", _) => Ok(Some(json!([
                {"id": 1, "name": "Main Org."},
                {"id": 2, "name": "Ops Org"}
            ]))),
            (reqwest::Method::GET, "/api/org", Some("1")) => {
                Ok(Some(json!({"id": 1, "name": "Main Org."})))
            }
            (reqwest::Method::GET, "/api/org", Some("2")) => {
                Ok(Some(json!({"id": 2, "name": "Ops Org"})))
            }
            (reqwest::Method::GET, "/api/datasources", Some("1")) => Ok(Some(json!([
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": true
                }
            ]))),
            (reqwest::Method::GET, "/api/datasources", Some("2")) => Ok(Some(json!([
                {
                    "uid": "prom-two",
                    "name": "Prometheus Two",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus-two:9090",
                    "isDefault": true
                }
            ]))),
            (reqwest::Method::GET, "/api/search", Some("1")) => Ok(Some(json!([
                {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "type": "dash-db",
                    "folderUid": "general",
                    "folderTitle": "General"
                }
            ]))),
            (reqwest::Method::GET, "/api/search", Some("2")) => Ok(Some(json!([
                {
                    "uid": "latency-main",
                    "title": "Latency Main",
                    "type": "dash-db",
                    "folderUid": "ops",
                    "folderTitle": "Ops"
                }
            ]))),
            (reqwest::Method::GET, "/api/folders/general", Some("1")) => Ok(Some(json!({
                "uid": "general",
                "title": "General"
            }))),
            (reqwest::Method::GET, "/api/folders/ops", Some("2")) => Ok(Some(json!({
                "uid": "ops",
                "title": "Ops"
            }))),
            (reqwest::Method::GET, "/api/folders/general/permissions", Some("1")) => {
                Ok(Some(json!([])))
            }
            (reqwest::Method::GET, "/api/folders/ops/permissions", Some("2")) => {
                Ok(Some(json!([])))
            }
            (reqwest::Method::GET, "/api/dashboards/uid/cpu-main", Some("1")) => Ok(Some(json!({
                "dashboard": {
                    "id": 11,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "panels": [{
                        "id": 7,
                        "title": "CPU Query",
                        "type": "timeseries",
                        "datasource": {"uid": "prom-main", "type": "prometheus"},
                        "targets": [{
                            "refId": "A",
                            "expr": "up"
                        }]
                    }]
                },
                "meta": {"folderUid": "general", "folderTitle": "General"}
            }))),
            (reqwest::Method::GET, "/api/dashboards/uid/latency-main", Some("2")) => {
                Ok(Some(json!({
                    "dashboard": {
                        "id": 12,
                        "uid": "latency-main",
                        "title": "Latency Main",
                        "panels": [{
                            "id": 8,
                            "title": "Latency Query",
                            "type": "timeseries",
                            "datasource": {"uid": "prom-two", "type": "prometheus"},
                            "targets": [{
                                "refId": "A",
                                "expr": "rate(http_requests_total[5m])"
                            }]
                        }]
                    },
                    "meta": {"folderUid": "ops", "folderTitle": "Ops"}
                })))
            }
            (reqwest::Method::GET, "/api/dashboards/uid/cpu-main/permissions", Some("1")) => {
                Ok(Some(json!([])))
            }
            (reqwest::Method::GET, "/api/dashboards/uid/latency-main/permissions", Some("2")) => {
                Ok(Some(json!([])))
            }
            _ => Err(test_support::message(format!(
                "unexpected request {method} {path} {scoped_org:?}"
            ))),
        }
    };

    let export_count = export_dashboards_with_request(&mut request_fixture, &export_args).unwrap();
    assert_eq!(export_count, 2);

    let export_import_dir =
        test_support::prepare_inspect_export_import_dir(inspect_root_temp.path(), &export_dir)
            .unwrap();

    let export_report_output = temp.path().join("export-report.json");
    let export_report_args = InspectExportArgs {
        import_dir: export_import_dir.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::QueriesJson),
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
        report_filter_datasource: None,
        report_filter_panel_id: None,
        progress: false,
        help_full: false,
        no_header: false,
        output_file: Some(live_report_output.clone()),
        also_stdout: false,
        interactive: false,
    };
    let live_report_count =
        test_support::inspect_live_dashboards_with_request(&mut request_fixture, &live_report_args)
            .unwrap();
    let live_report_document = read_json_output_file(&live_report_output);

    let export_governance_output = temp.path().join("export-governance.json");
    let export_governance_args = InspectExportArgs {
        import_dir: export_import_dir.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::GovernanceJson),
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
        all_orgs: true,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::GovernanceJson),
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
        &mut request_fixture,
        &live_governance_args,
    )
    .unwrap();
    let live_governance_document = read_json_output_file(&live_governance_output);

    let export_dependency_output = temp.path().join("export-dependency.json");
    let export_dependency_args = InspectExportArgs {
        import_dir: export_import_dir.clone(),
        input_type: None,
        input_format: crate::dashboard::DashboardImportInputFormat::Raw,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::DependencyJson),
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
        all_orgs: true,
        text: false,
        csv: false,
        json: false,
        table: false,
        yaml: false,
        output_format: Some(InspectOutputFormat::DependencyJson),
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
        &mut request_fixture,
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
