//! Import edge-case dashboard regression tests.
#![allow(unused_imports)]

use super::super::test_support;
use super::super::test_support::{
    diff_dashboards_with_request, import_dashboards_with_request, DiffArgs, ImportArgs,
    DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    TOOL_SCHEMA_VERSION,
};
use super::super::{
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn import_dashboards_with_use_export_org_filters_selected_orgs_and_creates_missing_targets() {
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

    let mut args = make_import_args(export_root.clone());
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.create_missing_orgs = true;
    args.only_org_id = vec![2];
    args.dry_run = false;

    let mut admin_calls = Vec::new();
    let mut import_calls = Vec::new();
    let count = test_support::import::import_dashboards_by_export_org_with_request(
        |method: reqwest::Method,
         path: &str,
         _params: &[(String, String)],
         payload: Option<&Value>| {
            admin_calls.push((method.to_string(), path.to_string()));
            match (method.clone(), path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([]))),
                (reqwest::Method::POST, "/api/orgs") => {
                    assert_eq!(
                        payload
                            .and_then(|value| value.as_object())
                            .unwrap()
                            .get("name"),
                        Some(&json!("Org Two"))
                    );
                    Ok(Some(json!({"orgId": "9"})))
                }
                _ => Err(test_support::message(format!(
                    "unexpected request {method} {path}"
                ))),
            }
        },
        |target_org_id, scoped_args| {
            import_calls.push((
                target_org_id,
                scoped_args.input_dir.clone(),
                scoped_args.org_id,
            ));
            assert!(!scoped_args.use_export_org);
            assert!(scoped_args.only_org_id.is_empty());
            assert!(!scoped_args.create_missing_orgs);
            Ok(1)
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

    assert_eq!(count, 1);
    assert_eq!(
        admin_calls,
        vec![
            ("GET".to_string(), "/api/orgs".to_string()),
            ("POST".to_string(), "/api/orgs".to_string()),
        ]
    );
    assert_eq!(import_calls, vec![(9, org_two_raw.clone(), Some(9))]);
}

#[test]
fn import_dashboards_with_use_export_org_round_trips_combined_export_root_into_scoped_diff() {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_one_raw = export_root.join("org_1_Main_Org").join("raw");
    let org_two_raw = export_root.join("org_2_Ops_Org").join("raw");
    fs::create_dir_all(&org_one_raw).unwrap();
    fs::create_dir_all(&org_two_raw).unwrap();
    fs::write(
        export_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "root",
            "dashboardCount": 2,
            "indexFile": "index.json",
            "orgCount": 2,
            "orgs": [
                {
                    "org": "Main Org.",
                    "orgId": "1",
                    "dashboardCount": 1,
                    "exportDir": "org_1_Main_Org"
                },
                {
                    "org": "Ops Org",
                    "orgId": "2",
                    "dashboardCount": 1,
                    "exportDir": "org_2_Ops_Org"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    for (
        raw_dir,
        org_id,
        org_name,
        dashboard_uid,
        dashboard_title,
        datasource_uid,
        datasource_type,
        panel_type,
        folder_uid,
        folder_title,
        query_field,
        query_text,
    ) in [
        (
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
        ),
        (
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
        ),
    ] {
        fs::write(
            raw_dir.join(EXPORT_METADATA_FILENAME),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": TOOL_SCHEMA_VERSION,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "format": "grafana-web-import-preserve-uid",
                "foldersFile": FOLDER_INVENTORY_FILENAME,
                "datasourcesFile": DATASOURCE_INVENTORY_FILENAME,
                "org": org_name,
                "orgId": org_id
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join(FOLDER_INVENTORY_FILENAME),
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
            raw_dir.join(DATASOURCE_INVENTORY_FILENAME),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": datasource_uid,
                    "name": datasource_uid,
                    "type": datasource_type,
                    "access": "proxy",
                    "url": "http://grafana.example.internal",
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
                "dashboard": {
                    "id": null,
                    "uid": dashboard_uid,
                    "title": dashboard_title,
                    "schemaVersion": 38,
                    "panels": [{
                        "id": 7,
                        "title": dashboard_title,
                        "type": panel_type,
                        "datasource": {"uid": datasource_uid, "type": datasource_type},
                        "targets": [{
                            "refId": "A",
                            query_field: query_text
                        }]
                    }]
                },
                "meta": {
                    "folderUid": folder_uid,
                    "folderTitle": folder_title
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    let mut args = make_import_args(export_root.clone());
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.dry_run = false;

    let imported_dashboards: std::rc::Rc<
        std::cell::RefCell<
            std::collections::BTreeMap<i64, std::collections::BTreeMap<String, Value>>,
        >,
    > = std::rc::Rc::new(std::cell::RefCell::new(std::collections::BTreeMap::new()));
    let routed_scopes: std::rc::Rc<std::cell::RefCell<Vec<(i64, PathBuf)>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

    let imported_dashboards_for_import = std::rc::Rc::clone(&imported_dashboards);
    let routed_scopes_for_import = std::rc::Rc::clone(&routed_scopes);
    let count = test_support::import::import_dashboards_by_export_org_with_request(
        |method: reqwest::Method,
         path: &str,
         _params: &[(String, String)],
         _payload: Option<&Value>| {
            match (method.clone(), path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 1, "name": "Main Org."},
                    {"id": 2, "name": "Ops Org"}
                ]))),
                _ => Err(test_support::message(format!(
                    "unexpected admin request {method} {path}"
                ))),
            }
        },
        move |target_org_id, scoped_args| {
            routed_scopes_for_import
                .borrow_mut()
                .push((target_org_id, scoped_args.input_dir.clone()));
            let imported_dashboards = std::rc::Rc::clone(&imported_dashboards_for_import);
            import_dashboards_with_request(
                with_dashboard_import_live_preflight(
                    json!([
                        {"uid":"prom-main","name":"prom-main","type":"prometheus"},
                        {"uid":"loki-main","name":"loki-main","type":"loki"}
                    ]),
                    json!([
                        {"id":"timeseries"},
                        {"id":"logs"}
                    ]),
                    move |method: reqwest::Method,
                          path: &str,
                          _params: &[(String, String)],
                          payload: Option<&Value>| match (method.clone(), path)
                    {
                        (reqwest::Method::POST, "/api/dashboards/db") => {
                            let payload = payload.cloned().unwrap();
                            let uid = payload["dashboard"]["uid"].as_str().unwrap().to_string();
                            imported_dashboards
                                .borrow_mut()
                                .entry(target_org_id)
                                .or_default()
                                .insert(uid, payload);
                            Ok(Some(json!({"status":"success"})))
                        }
                        (reqwest::Method::GET, _) if path.starts_with("/api/dashboards/uid/") => {
                            let uid = path.trim_start_matches("/api/dashboards/uid/");
                            Ok(imported_dashboards
                                .borrow()
                                .get(&target_org_id)
                                .and_then(|dashboards| dashboards.get(uid).cloned()))
                        }
                        _ => Err(test_support::message(format!(
                            "unexpected scoped import request {method} {path}"
                        ))),
                    },
                ),
                scoped_args,
            )
        },
        |_target_org_id, _scoped_args| unreachable!("dry-run collector should not be used"),
        &args,
    )
    .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        routed_scopes.borrow().as_slice(),
        &[(1, org_one_raw.clone()), (2, org_two_raw.clone())]
    );
    assert_eq!(imported_dashboards.borrow().len(), 2);

    for (target_org_id, input_dir) in routed_scopes.borrow().iter() {
        let stored = imported_dashboards
            .borrow()
            .get(target_org_id)
            .cloned()
            .unwrap();
        let folder_uid = if *target_org_id == 1 {
            "general"
        } else {
            "ops"
        };
        let diff_args = DiffArgs {
            common: make_common_args("http://127.0.0.1:3000".to_string()),
            input_dir: input_dir.clone(),
            input_format: crate::dashboard::DashboardImportInputFormat::Raw,
            import_folder_uid: Some(folder_uid.to_string()),
            context_lines: 3,
            output_format: crate::common::DiffOutputFormat::Text,
        };
        let differences = diff_dashboards_with_request(
            |_method, path, _params, _payload| {
                let uid = path.trim_start_matches("/api/dashboards/uid/");
                Ok(stored.get(uid).cloned())
            },
            &diff_args,
        )
        .unwrap();
        assert_eq!(
            differences, 0,
            "expected clean diff for org {target_org_id}"
        );
    }
}

#[test]
fn import_dashboards_with_use_export_org_only_org_id_skips_unselected_org_preflight() {
    use std::cell::RefCell;
    use std::rc::Rc;

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

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.only_org_id = vec![2];
    args.dry_run = false;

    let attempted_orgs: Rc<RefCell<Vec<i64>>> = Rc::new(RefCell::new(Vec::new()));
    let posts: Rc<RefCell<Vec<(i64, String)>>> = Rc::new(RefCell::new(Vec::new()));

    let attempted_orgs_for_import = Rc::clone(&attempted_orgs);
    let posts_for_import = Rc::clone(&posts);
    let count = test_support::import::import_dashboards_by_export_org_with_request(
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
            attempted_orgs_for_import.borrow_mut().push(target_org_id);
            assert_eq!(target_org_id, 2, "unselected org should not be imported");
            let posts = Rc::clone(&posts_for_import);
            import_dashboards_with_request(
                with_dashboard_import_live_preflight(
                    json!([
                        {"uid":"loki-main","name":"loki-main","type":"loki"}
                    ]),
                    json!([
                        {"id":"logs"}
                    ]),
                    move |method, path, _params, payload| match (method.clone(), path) {
                        (reqwest::Method::POST, "/api/dashboards/db") => {
                            let payload = payload.cloned().unwrap();
                            posts.borrow_mut().push((
                                target_org_id,
                                payload["dashboard"]["uid"].as_str().unwrap().to_string(),
                            ));
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
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(attempted_orgs.borrow().as_slice(), &[2]);
    assert_eq!(posts.borrow().as_slice(), &[(2, "logs-main".to_string())]);
}
