//! Import-focused dashboard regression tests for auth and routed org-scope behavior.
#![allow(unused_imports)]

use super::test_support;
use super::test_support::import::ImportDryRunReport;
use super::{
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata, ImportArgs, EXPORT_METADATA_FILENAME, TOOL_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;

#[test]
fn routed_import_status_matrix_covers_exists_missing_would_create_and_created() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("2", "Org Two", "org_2_Org_Two"),
            ("9", "Ops Org", "org_9_Ops_Org"),
        ],
    );
    write_basic_raw_export(
        &org_two_raw,
        "2",
        "Org Two",
        "cpu-two",
        "CPU Two",
        "prom-two",
        "prometheus",
        "timeseries",
        "general",
        "General",
        "expr",
        "up",
    );
    write_basic_raw_export(
        &org_nine_raw,
        "9",
        "Ops Org",
        "ops-main",
        "Ops Main",
        "loki-nine",
        "loki",
        "logs",
        "ops",
        "Ops",
        "expr",
        "{job=\"grafana\"}",
    );

    let mut missing_args = make_import_args(export_root.clone());
    missing_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    missing_args.use_export_org = true;
    missing_args.dry_run = true;
    missing_args.json = true;
    missing_args.create_missing_orgs = false;

    let missing_payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 2, "name": "Org Two"}
                ]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            },
            |_target_org_id, scoped_args| {
                Ok(ImportDryRunReport {
                    mode: "create-only".to_string(),
                    input_dir: scoped_args.input_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: Vec::new(),
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &missing_args,
        )
        .unwrap(),
    )
    .unwrap();

    let mut would_create_args = make_import_args(export_root.clone());
    would_create_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    would_create_args.use_export_org = true;
    would_create_args.dry_run = true;
    would_create_args.json = true;
    would_create_args.create_missing_orgs = true;

    let would_create_payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 2, "name": "Org Two"}
                ]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            },
            |_target_org_id, scoped_args| {
                Ok(ImportDryRunReport {
                    mode: "create-only".to_string(),
                    input_dir: scoped_args.input_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: Vec::new(),
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &would_create_args,
        )
        .unwrap(),
    )
    .unwrap();

    let mut live_args = make_import_args(export_root);
    live_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    live_args.use_export_org = true;
    live_args.dry_run = false;
    live_args.create_missing_orgs = true;

    let mut created_rows = Vec::new();
    let count = test_support::import::import_dashboards_by_export_org_with_request(
        |method, path, _params, payload| match (method, path) {
            (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 2, "name": "Org Two"}
            ]))),
            (reqwest::Method::POST, "/api/orgs") => {
                assert_eq!(
                    payload
                        .and_then(|value| value.as_object())
                        .unwrap()
                        .get("name"),
                    Some(&json!("Ops Org"))
                );
                Ok(Some(json!({"orgId": "19"})))
            }
            _ => Err(test_support::message(format!("unexpected request {path}"))),
        },
        |target_org_id, scoped_args| {
            created_rows.push((
                target_org_id,
                scoped_args.input_dir.clone(),
                scoped_args.org_id,
            ));
            Ok(1)
        },
        |_target_org_id, _scoped_args| unreachable!("dry-run collector should not be used"),
        &live_args,
    )
    .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        created_rows,
        vec![
            (2, org_two_raw.clone(), Some(2)),
            (19, org_nine_raw.clone(), Some(19))
        ]
    );

    let missing_orgs = missing_payload["orgs"].as_array().unwrap();
    let would_create_orgs = would_create_payload["orgs"].as_array().unwrap();
    let missing_existing = missing_orgs
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let missing_missing = missing_orgs
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    let would_create_existing = would_create_orgs
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let would_create_missing = would_create_orgs
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();

    assert_eq!(missing_payload["summary"]["existingOrgCount"], json!(1));
    assert_eq!(missing_payload["summary"]["missingOrgCount"], json!(1));
    assert_eq!(missing_payload["summary"]["wouldCreateOrgCount"], json!(0));
    assert_eq!(
        would_create_payload["summary"]["existingOrgCount"],
        json!(1)
    );
    assert_eq!(would_create_payload["summary"]["missingOrgCount"], json!(0));
    assert_eq!(
        would_create_payload["summary"]["wouldCreateOrgCount"],
        json!(1)
    );

    assert_eq!(missing_existing["orgAction"], json!("exists"));
    assert_eq!(missing_existing["targetOrgId"], json!(2));
    assert_eq!(missing_missing["orgAction"], json!("missing"));
    assert_eq!(missing_missing["targetOrgId"], Value::Null);
    assert_eq!(would_create_existing["orgAction"], json!("exists"));
    assert_eq!(would_create_existing["targetOrgId"], json!(2));
    assert_eq!(would_create_missing["orgAction"], json!("would-create"));
    assert_eq!(would_create_missing["targetOrgId"], Value::Null);

    let existing_summary = test_support::import::format_routed_import_scope_summary_fields(
        2,
        "Org Two",
        "exists",
        Some(2),
        &org_two_raw,
    );
    let missing_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "missing",
        None,
        &org_nine_raw,
    );
    let would_create_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "would-create",
        None,
        &org_nine_raw,
    );
    let created_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "created",
        Some(19),
        &org_nine_raw,
    );
    assert!(existing_summary.contains("orgAction=exists"));
    assert!(existing_summary.contains("targetOrgId=2"));
    assert!(missing_summary.contains("orgAction=missing"));
    assert!(missing_summary.contains("targetOrgId=<new>"));
    assert!(would_create_summary.contains("orgAction=would-create"));
    assert!(would_create_summary.contains("targetOrgId=<new>"));
    assert!(created_summary.contains("orgAction=created"));
    assert!(created_summary.contains("targetOrgId=19"));
}

#[test]
fn import_dashboards_with_use_export_org_dry_run_filters_selected_orgs_without_creating_missing_targets(
) {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_five_raw = export_root.join("org_5_Org_Five").join("raw");
    fs::create_dir_all(&org_two_raw).unwrap();
    fs::create_dir_all(&org_five_raw).unwrap();

    for (raw_dir, org_id, org_name, uid) in [
        (&org_two_raw, "2", "Org Two", "cpu-two"),
        (&org_five_raw, "5", "Org Five", "cpu-five"),
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
    args.only_org_id = vec![2, 5];
    args.dry_run = true;

    let mut admin_calls = Vec::new();
    let mut import_calls = Vec::new();
    test_support::import::import_dashboards_by_export_org_with_request(
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
            import_calls.push((
                target_org_id,
                scoped_args.input_dir.clone(),
                scoped_args.org_id,
            ));
            Ok(0)
        },
        |_target_org_id, scoped_args| {
            Ok(ImportDryRunReport {
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

    assert_eq!(
        admin_calls,
        vec![("GET".to_string(), "/api/orgs".to_string())]
    );
    assert_eq!(import_calls, vec![(2, org_two_raw.clone(), Some(2))]);
}
