//! Import edge-case dashboard regression tests for dry-run/dependency/folder-lookup behavior.
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
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn import_dashboards_with_dry_run_skips_post_requests() {
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
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"}
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
        replace_existing: true,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: false,
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

    let count = import_dashboards_with_request(
        with_dashboard_import_live_preflight(
            json!([]),
            json!([]),
            |_method, path, _params, _payload| match path {
                "/api/dashboards/uid/abc" => Ok(Some(json!({
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                    "meta": {"folderUid": "old-folder"}
                }))),
                "/api/folders/old-folder" => Ok(None),
                "/api/dashboards/db" => {
                    Err(test_support::message("dry-run must not post dashboards"))
                }
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            },
        ),
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn import_dashboards_rejects_missing_dependencies_before_dashboard_lookup() {
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
        input_dir: raw_dir,
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

    let error = import_dashboards_with_request(
        |method, path, _params, _payload| {
            calls.push(format!("{} {}", method.as_str(), path));
            match path {
                "/api/org" => Ok(Some(json!({"id": 1, "name": "Main Org."}))),
                "/api/datasources" => Ok(Some(json!([
                    {"uid": "other", "name": "Other", "type": "loki"}
                ]))),
                "/api/plugins" => Ok(Some(json!([
                    {"id": "row"}
                ]))),
                "/api/dashboards/db" => Err(test_support::message("preflight should block POST")),
                "/api/dashboards/uid/abc" => {
                    Err(test_support::message("preflight should block lookup"))
                }
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("Refusing dashboard import because preflight reports"));
    assert_eq!(
        calls,
        vec![
            "GET /api/datasources".to_string(),
            "GET /api/plugins".to_string()
        ]
    );
}

#[test]
fn import_dashboards_skips_dependency_preflight_for_dependency_free_dashboards() {
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
                "schemaVersion": 38
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
        input_dir: raw_dir,
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

    let count = import_dashboards_with_request(
        |method, path, _params, payload| {
            calls.push(format!("{} {}", method.as_str(), path));
            match (method, path) {
                (reqwest::Method::POST, "/api/dashboards/db") => {
                    let payload = payload.cloned().unwrap();
                    assert_eq!(payload["dashboard"]["uid"], "abc");
                    Ok(Some(json!({"status": "success"})))
                }
                (reqwest::Method::GET, "/api/datasources")
                | (reqwest::Method::GET, "/api/plugins")
                | (reqwest::Method::GET, "/api/dashboards/uid/abc") => {
                    Err(test_support::message(format!("unexpected lookup {path}")))
                }
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(calls, vec!["POST /api/dashboards/db".to_string()]);
}

#[test]
fn import_dashboards_with_shared_folder_lookup_reuses_folder_fetch_in_dry_run() {
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
                "path": "dash-a.json",
                "format": "grafana-web-import-preserve-uid"
            },
            {
                "uid": "def",
                "title": "Memory",
                "path": "dash-b.json",
                "format": "grafana-web-import-preserve-uid"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash-a.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "id": 7,
                "uid": "abc",
                "title": "CPU"
            },
            "meta": {
                "folderUid": "old-folder"
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash-b.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "id": 8,
                "uid": "def",
                "title": "Memory"
            },
            "meta": {
                "folderUid": "old-folder"
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let mut args = make_import_args(raw_dir);
    args.table = true;
    let calls: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_for_request = Rc::clone(&calls);

    let count = import_dashboards_with_request(
        move |method, path, _params, _payload| {
            calls_for_request
                .borrow_mut()
                .push(format!("{} {}", method.as_str(), path));
            match (method, path) {
                (reqwest::Method::GET, "/api/search") => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/plugins") => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/dashboards/uid/abc") => Err(api_response(
                    404,
                    "http://127.0.0.1:3000/api/dashboards/uid/abc",
                    "{\"message\":\"not found\"}",
                )),
                (reqwest::Method::GET, "/api/dashboards/uid/def") => Err(api_response(
                    404,
                    "http://127.0.0.1:3000/api/dashboards/uid/def",
                    "{\"message\":\"not found\"}",
                )),
                (reqwest::Method::GET, "/api/folders/old-folder") => Ok(Some(json!({
                    "uid": "old-folder",
                    "title": "Old Folder",
                    "parents": []
                }))),
                (reqwest::Method::POST, "/api/dashboards/db") => {
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
            .filter(|entry| entry.as_str() == "GET /api/folders/old-folder")
            .count(),
        1
    );
}

#[test]
fn import_dashboards_with_dry_run_summary_skips_unneeded_folder_lookup() {
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
                "title": "CPU"
            },
            "meta": {
                "folderUid": "old-folder"
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let mut args = make_import_args(raw_dir);
    args.replace_existing = true;
    args.dry_run = true;

    let calls: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_for_request = Rc::clone(&calls);

    let count = import_dashboards_with_request(
        move |method, path, _params, _payload| {
            calls_for_request
                .borrow_mut()
                .push(format!("{} {}", method.as_str(), path));
            match (method, path) {
                (reqwest::Method::GET, "/api/search") => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/dashboards/uid/abc") => Ok(Some(json!({
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                    "meta": {"folderUid": "new-folder"}
                }))),
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(calls.borrow().as_slice(), ["GET /api/search"]);
}

#[test]
fn import_dashboards_with_dry_run_uses_search_summary_for_missing_action_lookup() {
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

    let args = make_import_args(raw_dir);
    let calls: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_for_request = Rc::clone(&calls);

    let count = import_dashboards_with_request(
        move |_method, path, _params, _payload| {
            calls_for_request.borrow_mut().push(format!("GET {path}"));
            match path {
                "/api/search" => Ok(Some(json!([]))),
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
    assert_eq!(
        calls
            .borrow()
            .iter()
            .filter(|entry| entry == &"GET /api/search")
            .count(),
        1
    );
}
