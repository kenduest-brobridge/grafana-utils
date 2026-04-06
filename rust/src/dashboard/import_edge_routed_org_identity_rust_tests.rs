//! Import edge-case dashboard regression tests.
#![allow(unused_imports)]

use super::super::super::super::test_support;
use super::super::super::super::test_support::{
    diff_dashboards_with_request, import_dashboards_with_request, DiffArgs, ImportArgs,
    DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    TOOL_SCHEMA_VERSION,
};
use super::super::super::{
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use crate::common::api_response;
use crate::dashboard::DashboardImportInputFormat;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn import_dashboards_rejects_mismatched_export_org_with_explicit_org_id() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
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
                "uid": "abc",
                "title": "CPU",
                "path": "dash.json",
                "format": "grafana-web-import-preserve-uid",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "schemaVersion": 38}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        org_id: Some(2),
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        input_dir: raw_dir,
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: true,
        strict_schema: false,
        target_schema_version: None,
        import_message: "sync dashboards".to_string(),
        interactive: false,
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    };

    let error = import_dashboards_with_request(|_method, _path, _params, _payload| Ok(None), &args)
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("export orgId 1 does not match target org 2"));
}

#[test]
fn import_dashboards_rejects_mismatched_export_org_with_current_token_org() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
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
                "uid": "abc",
                "title": "CPU",
                "path": "dash.json",
                "format": "grafana-web-import-preserve-uid",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "schemaVersion": 38}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        org_id: None,
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        input_dir: raw_dir,
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: true,
        strict_schema: false,
        target_schema_version: None,
        import_message: "sync dashboards".to_string(),
        interactive: false,
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    };

    let error = import_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/org" => Ok(Some(json!({"id": 2, "name": "Ops Org"}))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("export orgId 1 does not match target org 2"));
}

#[test]
fn import_dashboards_allows_matching_export_org_with_current_org_lookup() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
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
                "uid": "abc",
                "title": "CPU",
                "path": "dash.json",
                "format": "grafana-web-import-preserve-uid",
                "org": "Main Org.",
                "orgId": "2"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "schemaVersion": 38}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        org_id: None,
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        input_dir: raw_dir,
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: true,
        strict_schema: false,
        target_schema_version: None,
        import_message: "sync dashboards".to_string(),
        interactive: false,
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    };
    let mut calls = Vec::new();

    let count = import_dashboards_with_request(
        |method, path, _params, _payload| {
            calls.push(format!("{} {}", method.as_str(), path));
            match (method, path) {
                (reqwest::Method::GET, "/api/search") => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/org") => Ok(Some(json!({"id": 2, "name": "Ops Org"}))),
                (reqwest::Method::GET, "/api/dashboards/uid/abc") => Err(api_response(
                    404,
                    "http://127.0.0.1:3000/api/dashboards/uid/abc",
                    "{\"message\":\"not found\"}",
                )),
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(calls.contains(&"GET /api/org".to_string()));
    assert!(calls.contains(&"GET /api/search".to_string()));
}
