//! Dashboard export/import inventory and path-discovery regression tests.
#![allow(unused_imports)]

use super::*;

#[test]
fn build_export_variant_dirs_returns_raw_and_prompt_dirs() {
    let (raw_dir, prompt_dir, provisioning_dir) =
        build_export_variant_dirs(Path::new("dashboards"));

    assert_eq!(raw_dir, Path::new("dashboards/raw"));
    assert_eq!(prompt_dir, Path::new("dashboards/prompt"));
    assert_eq!(provisioning_dir, Path::new("dashboards/provisioning"));
}

#[test]
fn discover_dashboard_files_rejects_combined_export_root() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw")).unwrap();
    fs::create_dir_all(temp.path().join("prompt")).unwrap();
    let error = discover_dashboard_files(temp.path()).unwrap_err();

    assert!(error.to_string().contains("combined export root"));
}

#[test]
fn discover_dashboard_files_ignores_export_metadata() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw/subdir")).unwrap();
    fs::write(
        temp.path().join("raw/subdir/dashboard.json"),
        serde_json::to_string_pretty(&json!({"uid": "abc", "title": "CPU"})).unwrap(),
    )
    .unwrap();
    fs::write(
        temp.path().join("raw").join(EXPORT_METADATA_FILENAME),
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

    let files = discover_dashboard_files(&temp.path().join("raw")).unwrap();
    assert_eq!(files, vec![temp.path().join("raw/subdir/dashboard.json")]);
}

#[test]
fn discover_dashboard_files_ignores_folder_inventory() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw/subdir")).unwrap();
    fs::write(
        temp.path().join("raw/subdir/dashboard.json"),
        serde_json::to_string_pretty(&json!({"uid": "abc", "title": "CPU"})).unwrap(),
    )
    .unwrap();
    fs::write(
        temp.path().join("raw").join(FOLDER_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([{
            "uid": "infra",
            "title": "Infra",
            "path": "Infra",
            "org": "Main Org.",
            "orgId": "1"
        }]))
        .unwrap(),
    )
    .unwrap();

    let files = discover_dashboard_files(&temp.path().join("raw")).unwrap();
    assert_eq!(files, vec![temp.path().join("raw/subdir/dashboard.json")]);
}

#[test]
fn discover_dashboard_files_ignores_permission_bundle() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw/subdir")).unwrap();
    fs::write(
        temp.path().join("raw/subdir/dashboard.json"),
        serde_json::to_string_pretty(&json!({"uid": "abc", "title": "CPU"})).unwrap(),
    )
    .unwrap();
    fs::write(
        temp.path().join("raw").join(DASHBOARD_PERMISSION_BUNDLE_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-permission-bundle",
            "schemaVersion": 1,
            "summary": {"resourceCount": 0, "dashboardCount": 0, "folderCount": 0, "permissionCount": 0},
            "resources": []
        }))
        .unwrap(),
    )
    .unwrap();

    let files = discover_dashboard_files(&temp.path().join("raw")).unwrap();
    assert_eq!(files, vec![temp.path().join("raw/subdir/dashboard.json")]);
}

#[test]
fn resolve_dashboard_import_source_accepts_provisioning_root() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    fs::create_dir_all(provisioning_root.join("dashboards/subdir")).unwrap();

    let resolved = resolve_dashboard_import_source(
        &provisioning_root,
        DashboardImportInputFormat::Provisioning,
    )
    .unwrap();

    assert_eq!(resolved.metadata_dir, provisioning_root);
    assert_eq!(
        resolved.dashboard_dir,
        temp.path().join("provisioning/dashboards")
    );
}

#[test]
fn resolve_dashboard_import_source_accepts_provisioning_dashboards_dir() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    let dashboards_dir = provisioning_root.join("dashboards");
    fs::create_dir_all(dashboards_dir.join("subdir")).unwrap();

    let resolved =
        resolve_dashboard_import_source(&dashboards_dir, DashboardImportInputFormat::Provisioning)
            .unwrap();

    assert_eq!(resolved.metadata_dir, provisioning_root);
    assert_eq!(resolved.dashboard_dir, dashboards_dir);
}

#[test]
fn import_dashboards_accepts_provisioning_root_with_explicit_format() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    let dashboards_dir = provisioning_root.join("dashboards");
    fs::create_dir_all(&dashboards_dir).unwrap();
    fs::write(
        provisioning_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "provisioning",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-file-provisioning-dashboard",
            "org": "Main Org.",
            "orgId": "1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dashboards_dir.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "uid": "cpu-main",
            "title": "CPU",
            "schemaVersion": 38
        }))
        .unwrap(),
    )
    .unwrap();
    let mut args = make_import_args(provisioning_root);
    args.input_format = DashboardImportInputFormat::Provisioning;
    args.dry_run = false;

    let count = import_dashboards_with_request(
        |_method, path, _params, payload| match path {
            "/api/dashboards/db" => {
                assert_eq!(payload.unwrap()["dashboard"]["uid"], "cpu-main");
                Ok(Some(json!({"status": "success"})))
            }
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn build_import_payload_accepts_wrapped_document() {
    let payload = build_import_payload(
        &json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
            "meta": {"folderUid": "old-folder"}
        }),
        Some("new-folder"),
        true,
        "sync dashboards",
    )
    .unwrap();

    assert_eq!(payload["dashboard"]["id"], Value::Null);
    assert_eq!(payload["folderUid"], "new-folder");
    assert_eq!(payload["overwrite"], true);
    assert_eq!(payload["message"], "sync dashboards");
}

#[test]
fn build_import_payload_omits_general_folder_uid() {
    let payload = build_import_payload(
        &json!({
            "uid": "abc",
            "title": "CPU",
            "meta": {"folderUid": "general"}
        }),
        None,
        false,
        "",
    )
    .unwrap();

    assert_eq!(payload["dashboard"]["id"], Value::Null);
    assert!(payload.get("folderUid").is_none());
}

#[test]
fn build_preserved_web_import_document_clears_numeric_id() {
    let document = build_preserved_web_import_document(&json!({
        "dashboard": {"id": 7, "uid": "abc", "title": "CPU"}
    }))
    .unwrap();

    assert_eq!(document["id"], Value::Null);
    assert_eq!(document["uid"], "abc");
}
