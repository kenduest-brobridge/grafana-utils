//! Import edge-case dashboard regression tests.
#![allow(unused_imports)]

use super::test_support;
use super::test_support::{
    diff_dashboards_with_request, import_dashboards_with_request, DiffArgs, ImportArgs,
    DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    TOOL_SCHEMA_VERSION,
};
use super::{
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use crate::common::api_response;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn build_routed_import_dry_run_json_and_live_failure_share_org_scope_identity() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_one_raw = export_root.join("org_1_Main_Org").join("raw");
    let org_two_raw = export_root.join("org_2_Ops_Org").join("raw");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("1", "Main Org.", "org_1_Main_Org"),
            ("2", "Ops Org", "org_2_Ops_Org"),
        ],
    );
    write_basic_raw_export(
        &org_one_raw,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "general",
        "General",
        "expr",
        "up",
    );
    write_basic_raw_export(
        &org_two_raw,
        "2",
        "Ops Org",
        "logs-main",
        "Logs Main",
        "loki-main",
        "loki",
        "logs",
        "ops",
        "Ops",
        "expr",
        "{job=\"grafana\"} |= \"error\"",
    );

    let mut dry_run_args = make_import_args(export_root.clone());
    dry_run_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    dry_run_args.use_export_org = true;
    dry_run_args.dry_run = true;
    dry_run_args.json = true;

    let dry_run_payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 1, "name": "Main Org."},
                    {"id": 2, "name": "Ops Org"}
                ]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            },
            |_target_org_id, scoped_args| {
                Ok(test_support::import::ImportDryRunReport {
                    mode: "create-only".to_string(),
                    input_dir: scoped_args.input_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: Vec::new(),
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &dry_run_args,
        )
        .unwrap(),
    )
    .unwrap();
    let dry_run_org_entry = dry_run_payload["orgs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap()
        .clone();
    let dry_run_import_entry = dry_run_payload["imports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap()
        .clone();

    let mut live_args = make_import_args(export_root);
    live_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    live_args.use_export_org = true;
    live_args.dry_run = false;

    let error = test_support::import::import_dashboards_by_export_org_with_request(
        |method: reqwest::Method,
         path: &str,
         _params: &[(String, String)],
         _payload: Option<&Value>| match (method.clone(), path) {
            (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 1, "name": "Main Org."},
                {"id": 2, "name": "Ops Org"}
            ]))),
            _ => Err(test_support::message(format!(
                "unexpected admin request {method} {path}"
            ))),
        },
        move |target_org_id, scoped_args| {
            let preflight_datasources = if target_org_id == 1 {
                json!([
                    {"uid":"prom-main","name":"prom-main","type":"prometheus"}
                ])
            } else {
                json!([
                    {"uid":"other","name":"other","type":"prometheus"}
                ])
            };
            let preflight_plugins = if target_org_id == 1 {
                json!([
                    {"id":"timeseries"}
                ])
            } else {
                json!([
                    {"id":"logs"}
                ])
            };
            import_dashboards_with_request(
                with_dashboard_import_live_preflight(
                    preflight_datasources,
                    preflight_plugins,
                    move |method: reqwest::Method,
                          path: &str,
                          _params: &[(String, String)],
                          _payload: Option<&Value>| match (method.clone(), path)
                    {
                        (reqwest::Method::POST, "/api/dashboards/db") => {
                            Ok(Some(json!({"status":"success"})))
                        }
                        (reqwest::Method::GET, _) if path.starts_with("/api/dashboards/uid/") => {
                            Ok(None)
                        }
                        _ => Err(test_support::message(format!(
                            "unexpected scoped request {method} {path}"
                        ))),
                    },
                ),
                scoped_args,
            )
        },
        |_target_org_id, _scoped_args| unreachable!("dry-run collector should not be used"),
        &live_args,
    )
    .unwrap_err();

    let error_text = error.to_string();
    assert!(error_text.contains(
        "Dashboard routed import failed for export orgId=2 name=Ops Org orgAction=exists targetOrgId=2"
    ));
    assert!(error_text.contains("org_2_Ops_Org/raw"));
    assert!(error_text.contains("Refusing dashboard import because preflight reports"));
    assert_eq!(dry_run_org_entry["sourceOrgId"], json!(2));
    assert_eq!(dry_run_org_entry["sourceOrgName"], json!("Ops Org"));
    assert_eq!(dry_run_org_entry["targetOrgId"], json!(2));
    assert_eq!(dry_run_import_entry["sourceOrgId"], json!(2));
    assert_eq!(dry_run_import_entry["sourceOrgName"], json!("Ops Org"));
    assert_eq!(dry_run_import_entry["targetOrgId"], json!(2));
}
