//! Import-focused dashboard regression tests for routed inventory and reporting behavior.
#![allow(unused_imports)]

use super::test_support;
use super::{
    make_basic_common_args, make_import_args, write_combined_export_root_metadata,
    EXPORT_METADATA_FILENAME, TOOL_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;

#[test]
fn build_routed_import_dry_run_json_with_request_reuses_org_inventory_lookup() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    fs::create_dir_all(&org_two_raw).unwrap();
    fs::create_dir_all(&org_nine_raw).unwrap();

    for (raw_dir, org_id, org_name, uid) in [
        (&org_two_raw, "2", "Org Two", "cpu-two"),
        (&org_nine_raw, "9", "Ops Org", "ops-main"),
    ] {
        fs::write(
            raw_dir.join(EXPORT_METADATA_FILENAME),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": TOOL_SCHEMA_VERSION,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "format": "grafana-web-import-preserve-uid"
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("index.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": uid,
                    "title": "CPU",
                    "path": "dash.json",
                    "format": "grafana-web-import-preserve-uid",
                    "org": org_name,
                    "orgId": org_id
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("dash.json"),
            serde_json::to_string_pretty(&json!({
                "dashboard": {"id": null, "uid": uid, "title": "CPU"}
            }))
            .unwrap(),
        )
        .unwrap();
    }

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.create_missing_orgs = true;
    args.dry_run = true;
    args.json = true;

    let mut admin_calls = Vec::new();
    let payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| {
                admin_calls.push((method.to_string(), path.to_string()));
                match (method, path) {
                    (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                        {"id": 2, "name": "Org Two"}
                    ]))),
                    _ => Err(test_support::message(format!("unexpected request {path}"))),
                }
            },
            |target_org_id, scoped_args| {
                Ok(test_support::import::ImportDryRunReport {
                    mode: "create-only".to_string(),
                    input_dir: scoped_args.input_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: vec![[
                        if target_org_id == 2 {
                            "cpu-two".to_string()
                        } else {
                            "ops-main".to_string()
                        },
                        "missing".to_string(),
                        "create".to_string(),
                        "General".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        scoped_args
                            .input_dir
                            .join("dash.json")
                            .display()
                            .to_string(),
                    ]],
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &args,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        admin_calls,
        vec![("GET".to_string(), "/api/orgs".to_string())]
    );
    assert_eq!(payload["orgs"].as_array().unwrap().len(), 2);
}

#[test]
fn build_routed_import_dry_run_json_with_request_reports_orgs_and_dashboards() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    fs::create_dir_all(export_root.join("raw")).unwrap();
    fs::create_dir_all(&org_two_raw).unwrap();
    fs::create_dir_all(&org_nine_raw).unwrap();
    fs::write(
        export_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-root",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "orgCount": 2
        }))
        .unwrap(),
    )
    .unwrap();

    for (raw_dir, org_id, org_name, uid) in [
        (&org_two_raw, "2", "Org Two", "cpu-two"),
        (&org_nine_raw, "9", "Ops Org", "ops-main"),
    ] {
        fs::write(
            raw_dir.join(EXPORT_METADATA_FILENAME),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": TOOL_SCHEMA_VERSION,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "format": "grafana-web-import-preserve-uid"
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("index.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": uid,
                    "title": "CPU",
                    "path": "dash.json",
                    "format": "grafana-web-import-preserve-uid",
                    "org": org_name,
                    "orgId": org_id
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("dash.json"),
            serde_json::to_string_pretty(&json!({
                "dashboard": {"id": null, "uid": uid, "title": "CPU"}
            }))
            .unwrap(),
        )
        .unwrap();
    }

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.create_missing_orgs = true;
    args.dry_run = true;
    args.json = true;

    let payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 2, "name": "Org Two"}
                ]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            },
            |target_org_id, scoped_args| {
                Ok(test_support::import::ImportDryRunReport {
                    mode: "create-only".to_string(),
                    input_dir: scoped_args.input_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: vec![[
                        if target_org_id == 2 {
                            "cpu-two".to_string()
                        } else {
                            "ops-main".to_string()
                        },
                        "missing".to_string(),
                        "create".to_string(),
                        "General".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        scoped_args
                            .input_dir
                            .join("dash.json")
                            .display()
                            .to_string(),
                    ]],
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &args,
        )
        .unwrap(),
    )
    .unwrap();

    let org_entries = payload["orgs"].as_array().unwrap();
    let import_entries = payload["imports"].as_array().unwrap();
    let existing_org = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let missing_org = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    let existing_import = import_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let missing_import = import_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();

    assert_eq!(payload["mode"], "routed-import-preview");
    assert_eq!(existing_org["orgAction"], "exists");
    assert_eq!(missing_org["orgAction"], "would-create");
    assert_eq!(existing_import["dashboards"][0]["uid"], "cpu-two");
    assert_eq!(missing_import["dashboards"], json!([]));
    assert_eq!(missing_import["summary"]["dashboardCount"], Value::from(1));
}

#[test]
fn import_dashboards_with_use_export_org_dry_run_table_returns_after_org_summary() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    fs::create_dir_all(export_root.join("raw")).unwrap();
    fs::create_dir_all(&org_two_raw).unwrap();
    fs::write(
        export_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-root",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "orgCount": 1
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_two_raw.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_two_raw.join("index.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "cpu-two",
                "title": "CPU",
                "path": "dash.json",
                "format": "grafana-web-import-preserve-uid",
                "org": "Org Two",
                "orgId": "2"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_two_raw.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": null, "uid": "cpu-two", "title": "CPU"}
        }))
        .unwrap(),
    )
    .unwrap();

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.dry_run = true;
    args.table = true;

    let mut import_calls = Vec::new();
    let count = test_support::import::import_dashboards_by_export_org_with_request(
        |method, path, _params, _payload| match (method, path) {
            (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 2, "name": "Org Two"}
            ]))),
            _ => Err(test_support::message(format!("unexpected request {path}"))),
        },
        |target_org_id, scoped_args| {
            import_calls.push((target_org_id, scoped_args.input_dir.clone()));
            Ok(0)
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
        &args,
    )
    .unwrap();

    assert_eq!(count, 0);
    assert!(import_calls.is_empty());
}
