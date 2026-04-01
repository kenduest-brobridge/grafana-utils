//! Live inspect output/governance file regressions.
use super::super::test_support::{self, InspectExportReportFormat, InspectLiveArgs};
use super::{make_common_args, read_json_output_file};
use serde_json::json;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn inspect_live_dashboards_with_request_writes_governance_json_to_output_file_matches_export_documents(
) {
    let temp = tempdir().unwrap();
    let output_file = temp.path().join("inspect-governance.json");
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
        report: Some(InspectExportReportFormat::GovernanceJson),
        output_format: None,
        report_columns: Vec::new(),
        report_filter_datasource: Some("prom-main".to_string()),
        report_filter_panel_id: Some("7".to_string()),
        progress: false,
        help_full: false,
        no_header: false,
        output_file: Some(output_file.clone()),
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

    let output = read_json_output_file(&output_file);
    assert_eq!(count, 1);
    assert_eq!(output["summary"]["dashboardCount"], Value::from(1));
    assert_eq!(output["summary"]["queryRecordCount"], Value::from(1));
    assert_eq!(output["datasourceFamilies"].as_array().unwrap().len(), 1);
    assert_eq!(
        output["datasourceFamilies"][0]["family"],
        Value::String("prometheus".to_string())
    );
    assert_eq!(
        output["dashboardDependencies"][0]["datasourceFamilies"],
        json!(["prometheus"])
    );
}
