//! Import edge-case dashboard regression tests.
#![allow(unused_imports)]

use super::test_support;
use super::test_support::{import_dashboards_with_request, ImportArgs};
use super::{
    make_basic_common_args, make_import_args, with_dashboard_import_live_preflight,
    write_basic_provisioning_export, write_combined_export_root_metadata,
};
use crate::dashboard::DashboardImportInputFormat;
use serde_json::{json, Value};
use std::cell::RefCell;
use tempfile::tempdir;

#[test]
fn import_dashboards_with_use_export_org_rejects_single_org_provisioning_root() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    write_basic_provisioning_export(
        &provisioning_root,
        "2",
        "Org Two",
        "cpu-two",
        "CPU Two",
        "prom-two",
        "prometheus",
        "timeseries",
        "team/cpu-two.json",
        "expr",
        "up",
    );

    let mut args = make_import_args(provisioning_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.input_format = DashboardImportInputFormat::Provisioning;

    let error = test_support::import::import_dashboards_by_export_org_with_request(
        |_method, _path, _params, _payload| unreachable!("scope discovery should fail first"),
        |_target_org_id, _scoped_args| unreachable!("scope discovery should fail first"),
        |_target_org_id, _scoped_args| unreachable!("scope discovery should fail first"),
        &args,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("expects the combined export root, not one provisioning/ export directory"));
}

#[test]
fn import_dashboards_with_use_export_org_routes_combined_provisioning_exports() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_provisioning = export_root.join("org_2_Org_Two").join("provisioning");
    let org_nine_provisioning = export_root.join("org_9_Ops_Org").join("provisioning");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("2", "Org Two", "org_2_Org_Two"),
            ("9", "Ops Org", "org_9_Ops_Org"),
        ],
    );
    write_basic_provisioning_export(
        &org_two_provisioning,
        "2",
        "Org Two",
        "cpu-two",
        "CPU Two",
        "prom-two",
        "prometheus",
        "timeseries",
        "team/cpu-two.json",
        "expr",
        "up",
    );
    write_basic_provisioning_export(
        &org_nine_provisioning,
        "9",
        "Ops Org",
        "logs-nine",
        "Logs Nine",
        "loki-nine",
        "loki",
        "logs",
        "ops/logs-nine.json",
        "expr",
        "{job=\"grafana\"}",
    );

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.input_format = DashboardImportInputFormat::Provisioning;
    args.dry_run = false;

    let imported_orgs = RefCell::new(Vec::new());
    let count = test_support::import::import_dashboards_by_export_org_with_request(
        |method, path, _params, _payload| match (method, path) {
            (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 2, "name": "Org Two"},
                {"id": 9, "name": "Ops Org"}
            ]))),
            _ => Err(test_support::message(format!(
                "unexpected admin request {path}"
            ))),
        },
        |target_org_id, scoped_args| {
            imported_orgs.borrow_mut().push((
                target_org_id,
                scoped_args.input_dir.clone(),
                scoped_args.org_id,
                scoped_args.use_export_org,
                scoped_args.input_format,
            ));
            let (expected_uid, expected_datasources, expected_plugins) = match target_org_id {
                2 => (
                    "cpu-two",
                    json!([{"uid":"prom-two","name":"prom-two","type":"prometheus"}]),
                    json!([{"id":"timeseries"}]),
                ),
                9 => (
                    "logs-nine",
                    json!([{"uid":"loki-nine","name":"loki-nine","type":"loki"}]),
                    json!([{"id":"logs"}]),
                ),
                _ => unreachable!("unexpected target org"),
            };
            import_dashboards_with_request(
                with_dashboard_import_live_preflight(
                    expected_datasources,
                    expected_plugins,
                    move |method, path, _params, payload| match (method, path) {
                        (reqwest::Method::POST, "/api/dashboards/db") => {
                            assert_eq!(payload.unwrap()["dashboard"]["uid"], json!(expected_uid));
                            Ok(Some(json!({"status":"success"})))
                        }
                        (reqwest::Method::GET, _) if path.starts_with("/api/dashboards/uid/") => {
                            Ok(None)
                        }
                        _ => Err(test_support::message(format!(
                            "unexpected scoped request {path}"
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

    let imported_orgs = imported_orgs.into_inner();
    assert_eq!(count, 2);
    assert_eq!(imported_orgs.len(), 2);
    assert_eq!(
        imported_orgs,
        vec![
            (
                2,
                org_two_provisioning.clone(),
                Some(2),
                false,
                DashboardImportInputFormat::Provisioning,
            ),
            (
                9,
                org_nine_provisioning.clone(),
                Some(9),
                false,
                DashboardImportInputFormat::Provisioning,
            ),
        ]
    );
}
