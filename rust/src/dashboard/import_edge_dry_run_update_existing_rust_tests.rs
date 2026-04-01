//! Import edge-case dashboard regression tests for schema-version and update-existing behavior.
#![allow(unused_imports)]

use super::super::super::test_support;
use super::super::super::test_support::{
    import_dashboards_with_request, ImportArgs, DATASOURCE_INVENTORY_FILENAME,
    EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME, TOOL_SCHEMA_VERSION,
};
use super::{
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use crate::common::api_response;
use crate::dashboard::DashboardImportInputFormat;
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;

#[test]
fn import_dashboards_with_dry_run_replace_existing_reuses_summary_folder_uid() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 2,
            "indexFile": "index.json"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash-a.json"),
        serde_json::to_string_pretty(&json!({"dashboard":{"uid":"abc","title":"CPU"}})).unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash-b.json"),
        serde_json::to_string_pretty(&json!({"dashboard":{"uid":"def","title":"Mem"}})).unwrap(),
    )
    .unwrap();

    let mut args = make_import_args(raw_dir);
    args.replace_existing = true;
    let calls: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_for_request = Rc::clone(&calls);

    let count = import_dashboards_with_request(
        move |_method, path, _params, _payload| {
            calls_for_request.borrow_mut().push(format!("GET {path}"));
            match path {
                "/api/search" => Ok(Some(json!([
                    {"uid":"abc","folderUid":"folder-a"},
                    {"uid":"def","folderUid":"folder-b"}
                ]))),
                "/api/dashboards/db" => {
                    Err(test_support::message("dry-run must not post dashboards"))
                }
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        calls
            .borrow()
            .iter()
            .filter(|entry| entry.starts_with("GET /api/dashboards/uid/"))
            .count(),
        0
    );
}

#[test]
fn import_dashboards_with_matching_dependencies_posts_after_preflight() {
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
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "id": 7,
                "uid": "abc",
                "title": "CPU",
                "schemaVersion": 38,
                "panels": [
                    {
                        "type": "row",
                        "panels": [
                            {
                                "type": "timeseries",
                                "datasource": {
                                    "uid": "prom-main",
                                    "name": "Prometheus Main"
                                }
                            }
                        ]
                    }
                ]
            }
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
        import_dir: raw_dir,
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: false,
        strict_schema: false,
        target_schema_version: None,
        import_message: "sync dashboards".to_string(),
        interactive: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    };
    let mut calls = Vec::new();
    let mut posted_payloads = Vec::new();

    let count = import_dashboards_with_request(
        with_dashboard_import_live_preflight(
            json!([{"uid":"prom-main","name":"Prometheus Main","type":"prometheus"}]),
            json!([{ "id": "row" }, { "id": "timeseries" }]),
            |method, path, _params, payload| {
                calls.push(format!("{} {}", method.as_str(), path));
                match (method, path) {
                    (reqwest::Method::GET, "/api/search") => Ok(Some(json!([]))),
                    (reqwest::Method::GET, "/api/org") => {
                        Ok(Some(json!({"id": 1, "name": "Main Org."})))
                    }
                    (reqwest::Method::GET, "/api/dashboards/uid/abc") => Err(api_response(
                        404,
                        "http://127.0.0.1:3000/api/dashboards/uid/abc",
                        "{\"message\":\"not found\"}",
                    )),
                    (reqwest::Method::POST, "/api/dashboards/db") => {
                        posted_payloads.push(payload.cloned().unwrap());
                        Ok(Some(json!({"status": "success"})))
                    }
                    _ => Err(test_support::message(format!("unexpected path {path}"))),
                }
            },
        ),
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(calls, vec!["POST /api/dashboards/db".to_string()]);
    assert_eq!(posted_payloads.len(), 1);
    assert_eq!(posted_payloads[0]["dashboard"]["uid"], "abc");
}

#[test]
fn import_dashboards_rejects_unsupported_export_schema_version() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION + 1,
            "variant": "raw",
            "dashboardCount": 0,
            "indexFile": "index.json"
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
        import_dir: raw_dir,
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: false,
        strict_schema: false,
        target_schema_version: None,
        import_message: "sync dashboards".to_string(),
        interactive: false,
        dry_run: false,
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
        .contains("Unsupported dashboard export schemaVersion"));
}

#[test]
fn import_dashboards_with_update_existing_only_skips_missing_dashboards() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 2,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("exists.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "schemaVersion": 38}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("missing.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 8, "uid": "xyz", "title": "Memory", "schemaVersion": 38}
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
        import_dir: raw_dir,
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: true,
        require_matching_folder_path: false,
        require_matching_export_org: false,
        strict_schema: false,
        target_schema_version: None,
        import_message: "sync dashboards".to_string(),
        interactive: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    };
    let mut posted_payloads = Vec::new();
    let mut calls = Vec::new();
    let count = import_dashboards_with_request(
        |method, path, _params, payload| {
            calls.push(format!("{} {}", method.as_str(), path));
            match (method, path) {
                (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/plugins") => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/search") => {
                    Ok(Some(json!([{"uid": "abc", "folderUid": "source-folder"}])))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/abc") => Ok(Some(json!({
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU"}
                }))),
                (reqwest::Method::GET, "/api/dashboards/uid/xyz") => Err(api_response(
                    404,
                    "http://127.0.0.1:3000/api/dashboards/uid/xyz",
                    "{\"message\":\"not found\"}",
                )),
                (reqwest::Method::POST, "/api/dashboards/db") => {
                    posted_payloads.push(payload.cloned().unwrap());
                    Ok(Some(json!({"status": "success"})))
                }
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(
        count, 1,
        "calls: {:?}, posted: {:?}",
        calls, posted_payloads
    );
    assert_eq!(posted_payloads.len(), 1);
    assert_eq!(posted_payloads[0]["dashboard"]["uid"], "abc");
    assert_eq!(posted_payloads[0]["overwrite"], true);
}

#[test]
fn import_dashboards_with_update_existing_only_table_marks_missing_dashboards_as_skipped() {
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
        raw_dir.join("missing.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 8, "uid": "xyz", "title": "Memory"}
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
        import_dir: raw_dir,
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: true,
        require_matching_folder_path: false,
        require_matching_export_org: false,
        strict_schema: false,
        target_schema_version: None,
        import_message: "sync dashboards".to_string(),
        interactive: false,
        dry_run: true,
        table: true,
        json: false,
        output_format: None,
        no_header: true,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    };

    let count = import_dashboards_with_request(
        |method, path, _params, _payload| match (method, path) {
            (reqwest::Method::GET, "/api/search") => Ok(Some(json!([]))),
            (reqwest::Method::GET, "/api/dashboards/uid/xyz") => Err(api_response(
                404,
                "http://127.0.0.1:3000/api/dashboards/uid/xyz",
                "{\"message\":\"not found\"}",
            )),
            (reqwest::Method::POST, "/api/dashboards/db") => {
                Err(test_support::message("dry-run must not post dashboards"))
            }
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
}
