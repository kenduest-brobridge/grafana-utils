//! Feature-oriented live inspect regressions.
//! Keeps shared helpers here while splitting output/governance and parity tests into modules.
use super::test_support::{
    self, parse_cli_from, CommonCliArgs, DashboardCommand, InspectOutputFormat,
};
use crate::common::GrafanaCliError;
use crate::dashboard::inspect_live::load_variant_index_entries;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

type TestRequestResult = crate::common::Result<Option<Value>>;

fn make_common_args(base_url: String) -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: None,
        url: base_url,
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

fn read_json_output_file(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap();
    assert!(
        raw.ends_with('\n'),
        "expected output file {} to end with a newline",
        path.display()
    );
    serde_json::from_str(&raw).unwrap()
}

fn normalize_governance_document_for_compare(document: &Value) -> Value {
    let mut normalized = document.clone();
    if let Some(rows) = normalized
        .get_mut("dashboardDependencies")
        .and_then(|value| value.as_array_mut())
    {
        for row in rows {
            if let Some(object) = row.as_object_mut() {
                object.remove("file");
            }
        }
    }
    normalized
}

fn normalize_queries_document_for_compare(document: &Value) -> Value {
    let mut normalized = document.clone();
    if let Some(rows) = normalized
        .get_mut("queries")
        .and_then(|value| value.as_array_mut())
    {
        for row in rows {
            if let Some(object) = row.as_object_mut() {
                object.remove("file");
                object.remove("datasourceOrg");
                object.remove("datasourceOrgId");
                object.remove("datasourceDatabase");
                object.remove("datasourceBucket");
                object.remove("datasourceOrganization");
                object.remove("datasourceIndexPattern");
            }
        }
    }
    normalized
}

#[test]
fn load_variant_index_entries_reports_json_error_for_invalid_index_file() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("index.json"), "[").unwrap();

    let error = load_variant_index_entries(temp.path(), None).unwrap_err();
    assert!(matches!(error, GrafanaCliError::Json(_)));
}

#[test]
fn parse_cli_supports_inspect_live_baseline_output_formats() {
    for (output_format, expected) in [
        ("text", InspectOutputFormat::Text),
        ("table", InspectOutputFormat::Table),
        ("csv", InspectOutputFormat::Csv),
        ("json", InspectOutputFormat::Json),
        ("yaml", InspectOutputFormat::Yaml),
    ] {
        let args = parse_cli_from([
            "grafana-util",
            "inspect-live",
            "--url",
            "https://grafana.example.com",
            "--output-format",
            output_format,
        ]);

        match args.command {
            DashboardCommand::InspectLive(inspect_args) => {
                assert_eq!(inspect_args.common.url, "https://grafana.example.com");
                assert_eq!(inspect_args.output_format, Some(expected));
                assert_eq!(inspect_args.report, None);
                assert!(!inspect_args.json);
                assert!(!inspect_args.table);
            }
            _ => panic!("expected inspect-live command"),
        }
    }
}

fn assert_governance_documents_match(export_document: &Value, live_document: &Value) {
    assert_eq!(
        normalize_governance_document_for_compare(export_document),
        normalize_governance_document_for_compare(live_document)
    );
}

fn json_query_report_row<'a>(document: &'a Value, ref_id: &str) -> &'a Value {
    document["queries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["refId"] == Value::String(ref_id.to_string()))
        .unwrap()
}

fn assert_json_query_report_row_parity(
    export_document: &Value,
    live_document: &Value,
    ref_id: &str,
) {
    let export_row = json_query_report_row(export_document, ref_id);
    let live_row = json_query_report_row(live_document, ref_id);
    for field in [
        "org",
        "orgId",
        "dashboardUid",
        "dashboardTitle",
        "dashboardTags",
        "folderPath",
        "folderFullPath",
        "folderLevel",
        "folderUid",
        "parentFolderUid",
        "panelId",
        "panelTitle",
        "panelType",
        "panelTargetCount",
        "panelQueryCount",
        "panelDatasourceCount",
        "panelVariables",
        "refId",
        "datasource",
        "datasourceName",
        "datasourceUid",
        "datasourceType",
        "datasourceFamily",
        "queryField",
        "targetHidden",
        "targetDisabled",
        "queryVariables",
        "metrics",
        "functions",
        "measurements",
        "buckets",
        "query",
    ] {
        assert_eq!(
            export_row[field], live_row[field],
            "field={field}, refId={ref_id}"
        );
    }
}

#[allow(clippy::type_complexity)]
fn core_family_inspect_live_request_fixture(
    datasource_inventory: Value,
    dashboard_payload: Value,
) -> impl FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> TestRequestResult {
    move |method, path, _params, _payload| {
        let method_name = method.to_string();
        match (method, path) {
            (reqwest::Method::GET, "/api/org") => Ok(Some(serde_json::json!({
                "id": 1,
                "name": "Main Org."
            }))),
            (reqwest::Method::GET, "/api/datasources") => Ok(Some(datasource_inventory.clone())),
            (reqwest::Method::GET, "/api/search") => Ok(Some(serde_json::json!([
                {
                    "uid": "core-main",
                    "title": "Core Main",
                    "type": "dash-db",
                    "folderUid": "general",
                    "folderTitle": "General"
                }
            ]))),
            (reqwest::Method::GET, "/api/folders/general") => Ok(Some(serde_json::json!({
                "uid": "general",
                "title": "General"
            }))),
            (reqwest::Method::GET, "/api/folders/general/permissions") => {
                Ok(Some(serde_json::json!([])))
            }
            (reqwest::Method::GET, "/api/dashboards/uid/core-main") => {
                Ok(Some(dashboard_payload.clone()))
            }
            (reqwest::Method::GET, "/api/dashboards/uid/core-main/permissions") => {
                Ok(Some(serde_json::json!([])))
            }
            _ => Err(test_support::message(format!(
                "unexpected request {method_name} {path}"
            ))),
        }
    }
}

#[cfg(test)]
#[path = "inspect_live_rust_tests_output_rust_tests.rs"]
mod inspect_live_rust_tests_output_rust_tests;

#[cfg(test)]
#[path = "inspect_live_rust_tests_core_family_rust_tests.rs"]
mod inspect_live_rust_tests_core_family_rust_tests;
