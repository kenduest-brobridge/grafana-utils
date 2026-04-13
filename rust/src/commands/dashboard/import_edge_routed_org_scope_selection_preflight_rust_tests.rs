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
fn import_dashboards_with_use_export_org_stops_on_scoped_preflight_failure() {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_one_raw = export_root.join("org_1_Main_Org").join("raw");
    let org_two_raw = export_root.join("org_2_Ops_Org").join("raw");
    let org_three_raw = export_root.join("org_3_App_Org").join("raw");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("1", "Main Org.", "org_1_Main_Org"),
            ("2", "Ops Org", "org_2_Ops_Org"),
            ("3", "App Org", "org_3_App_Org"),
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
    write_basic_raw_export(
        &org_three_raw,
        "3",
        "App Org",
        "app-main",
        "App Main",
        "prom-app",
        "prometheus",
        "timeseries",
        "apps",
        "Apps",
        "expr",
        "rate(http_requests_total[5m])",
    );

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.dry_run = false;

    let attempted_orgs: std::rc::Rc<std::cell::RefCell<Vec<i64>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let imported_dashboards: std::rc::Rc<
        std::cell::RefCell<std::collections::BTreeMap<i64, Vec<String>>>,
    > = std::rc::Rc::new(std::cell::RefCell::new(std::collections::BTreeMap::new()));

    let attempted_orgs_for_import = std::rc::Rc::clone(&attempted_orgs);
    let imported_dashboards_for_import = std::rc::Rc::clone(&imported_dashboards);
    let error = test_support::import::import_dashboards_by_export_org_with_request(
        |method: reqwest::Method,
         path: &str,
         _params: &[(String, String)],
         _payload: Option<&Value>| match (method.clone(), path) {
            (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 1, "name": "Main Org."},
                {"id": 2, "name": "Ops Org"},
                {"id": 3, "name": "App Org"}
            ]))),
            _ => Err(test_support::message(format!(
                "unexpected admin request {method} {path}"
            ))),
        },
        move |target_org_id, scoped_args| {
            attempted_orgs_for_import.borrow_mut().push(target_org_id);
            let imported_dashboards = std::rc::Rc::clone(&imported_dashboards_for_import);
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
                          payload: Option<&Value>| match (method.clone(), path)
                    {
                        (reqwest::Method::POST, "/api/dashboards/db") => {
                            let payload = payload.cloned().unwrap();
                            let uid = payload["dashboard"]["uid"].as_str().unwrap().to_string();
                            imported_dashboards
                                .borrow_mut()
                                .entry(target_org_id)
                                .or_default()
                                .push(uid);
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
    .unwrap_err();

    let error_text = error.to_string();
    assert!(error_text.contains(
        "Dashboard routed import failed for export orgId=2 name=Ops Org orgAction=exists targetOrgId=2"
    ));
    assert!(error_text.contains("org_2_Ops_Org/raw"));
    assert!(error_text.contains("Refusing dashboard import because preflight reports"));
    assert_eq!(attempted_orgs.borrow().as_slice(), &[1, 2]);
    assert_eq!(
        imported_dashboards.borrow().get(&1),
        Some(&vec!["cpu-main".to_string()])
    );
    assert_eq!(imported_dashboards.borrow().get(&2), None);
    assert_eq!(imported_dashboards.borrow().get(&3), None);
}
