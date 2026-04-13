//! Import-focused dashboard regression tests for auth and routed org-scope behavior.
#![allow(unused_imports)]

use super::test_support;
use super::test_support::import::ImportDryRunReport;
use super::{
    build_import_auth_context, import_dashboards_with_org_clients, import_dashboards_with_request,
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata, ImportArgs, EXPORT_METADATA_FILENAME, TOOL_SCHEMA_VERSION,
};
use crate::dashboard::DashboardImportInputFormat;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn import_dashboards_with_client_imports_discovered_files() {
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
        raw_dir.join("permissions.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-permission-export",
            "schemaVersion": 1,
            "items": []
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "schemaVersion": 38},
            "meta": {"folderUid": "old-folder"}
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
        import_folder_uid: Some("new-folder".to_string()),
        ensure_folders: false,
        replace_existing: true,
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
        list_columns: false,
        progress: false,
        verbose: false,
    };
    let mut posted_payloads = Vec::new();
    let count = import_dashboards_with_request(
        with_dashboard_import_live_preflight(
            json!([]),
            json!([]),
            |_method, path, _params, payload| {
                assert_eq!(path, "/api/dashboards/db");
                posted_payloads.push(payload.cloned().unwrap());
                Ok(Some(json!({"status": "success"})))
            },
        ),
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(posted_payloads.len(), 1);
    assert_eq!(posted_payloads[0]["folderUid"], "new-folder");
    assert_eq!(posted_payloads[0]["dashboard"]["id"], Value::Null);
}

#[test]
fn import_dashboards_with_org_id_requires_basic_auth() {
    let temp = tempdir().unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        org_id: Some(7),
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        input_dir: temp.path().join("raw"),
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
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        list_columns: false,
        progress: false,
        verbose: false,
    };

    let error = import_dashboards_with_org_clients(&args).unwrap_err();

    assert!(error
        .to_string()
        .contains("Dashboard import with --org-id requires Basic auth"));
}

#[test]
fn import_dashboards_with_use_export_org_requires_basic_auth() {
    let temp = tempdir().unwrap();
    let mut args = make_import_args(temp.path().join("exports"));
    args.use_export_org = true;

    let error = import_dashboards_with_org_clients(&args).unwrap_err();

    assert!(error
        .to_string()
        .contains("Dashboard import with --use-export-org requires Basic auth"));
}

#[test]
fn import_dashboards_with_create_missing_orgs_during_dry_run_previews_org_creation() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    fs::create_dir_all(&org_nine_raw).unwrap();
    fs::write(
        org_nine_raw.join(EXPORT_METADATA_FILENAME),
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
        org_nine_raw.join("index.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "ops",
                "title": "Ops",
                "path": "ops.json",
                "format": "grafana-web-import-preserve-uid",
                "org": "Ops Org",
                "orgId": "9"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_nine_raw.join("ops.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": null, "uid": "ops", "title": "Ops"}
        }))
        .unwrap(),
    )
    .unwrap();

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.create_missing_orgs = true;
    args.dry_run = true;

    let mut admin_calls = Vec::new();
    let mut import_calls = Vec::new();
    let count = test_support::import::import_dashboards_by_export_org_with_request(
        |method, path, _params, _payload| {
            admin_calls.push((method.to_string(), path.to_string()));
            match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            }
        },
        |target_org_id, scoped_args| {
            import_calls.push((target_org_id, scoped_args.input_dir.clone()));
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

    assert_eq!(count, 0);
    assert_eq!(
        admin_calls,
        vec![("GET".to_string(), "/api/orgs".to_string())]
    );
    assert!(import_calls.is_empty());
}

#[test]
fn routed_interactive_import_rebinds_scoped_args_per_org() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    write_combined_export_root_metadata(&export_root, &[("9", "Ops Org", "org_9_Ops_Org")]);
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

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.interactive = true;
    args.dry_run = false;

    let mut import_calls = Vec::new();
    let count = test_support::import::import_dashboards_by_export_org_with_request(
        |method, path, _params, _payload| match (method, path) {
            (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 9, "name": "Ops Org"}
            ]))),
            _ => Err(test_support::message(format!("unexpected request {path}"))),
        },
        |target_org_id, scoped_args| {
            import_calls.push((
                target_org_id,
                scoped_args.input_dir.clone(),
                scoped_args.org_id,
                scoped_args.use_export_org,
                scoped_args.interactive,
            ));
            Ok(1)
        },
        |_target_org_id, _scoped_args| unreachable!("dry-run collector should not be used"),
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(
        import_calls,
        vec![(9, org_nine_raw.clone(), Some(9), false, true)]
    );
}

#[test]
fn routed_import_create_missing_orgs_dry_run_and_live_created_scope_stay_aligned() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    write_combined_export_root_metadata(&export_root, &[("9", "Ops Org", "org_9_Ops_Org")]);
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

    let mut dry_run_args = make_import_args(export_root.clone());
    dry_run_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    dry_run_args.use_export_org = true;
    dry_run_args.create_missing_orgs = true;
    dry_run_args.dry_run = true;
    dry_run_args.json = true;

    let dry_run_payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([]))),
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
            &dry_run_args,
        )
        .unwrap(),
    )
    .unwrap();

    let dry_run_org = dry_run_payload["orgs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap()
        .clone();
    let dry_run_import = dry_run_payload["imports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap()
        .clone();

    let mut live_args = make_import_args(export_root);
    live_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    live_args.use_export_org = true;
    live_args.create_missing_orgs = true;
    live_args.dry_run = false;

    let mut admin_calls = Vec::new();
    let mut import_calls = Vec::new();
    let count = test_support::import::import_dashboards_by_export_org_with_request(
        |method, path, _params, payload| {
            admin_calls.push((method.to_string(), path.to_string()));
            match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([]))),
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
            }
        },
        |target_org_id, scoped_args| {
            import_calls.push((
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

    assert_eq!(count, 1);
    assert_eq!(
        admin_calls,
        vec![
            ("GET".to_string(), "/api/orgs".to_string()),
            ("POST".to_string(), "/api/orgs".to_string()),
        ]
    );
    assert_eq!(import_calls, vec![(19, org_nine_raw.clone(), Some(19))]);

    assert_eq!(dry_run_org["orgAction"], json!("would-create"));
    assert_eq!(dry_run_org["targetOrgId"], Value::Null);
    assert_eq!(dry_run_import["orgAction"], json!("would-create"));
    assert_eq!(dry_run_import["targetOrgId"], Value::Null);
    assert_eq!(dry_run_import["dashboards"], json!([]));
    assert_eq!(dry_run_import["summary"]["dashboardCount"], json!(1));

    let would_create_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "would-create",
        None,
        Path::new(dry_run_org["importDir"].as_str().unwrap()),
    );
    let created_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "created",
        Some(19),
        &org_nine_raw,
    );
    assert!(would_create_summary.contains("export orgId=9"));
    assert!(would_create_summary.contains("orgAction=would-create"));
    assert!(would_create_summary.contains("targetOrgId=<new>"));
    assert!(would_create_summary.contains(&org_nine_raw.display().to_string()));
    assert!(created_summary.contains("export orgId=9"));
    assert!(created_summary.contains("orgAction=created"));
    assert!(created_summary.contains("targetOrgId=19"));
    assert!(created_summary.contains(&org_nine_raw.display().to_string()));
}

#[test]
fn build_import_auth_context_adds_org_header_for_basic_auth_imports() {
    let temp = tempdir().unwrap();
    let args = ImportArgs {
        common: make_basic_common_args("http://127.0.0.1:3000".to_string()),
        org_id: Some(7),
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        input_dir: temp.path().join("raw"),
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
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        list_columns: false,
        progress: false,
        verbose: false,
    };

    let context = build_import_auth_context(&args).unwrap();

    assert_eq!(context.auth_mode, "basic");
    assert!(context
        .headers
        .iter()
        .any(|(name, value)| { name == "X-Grafana-Org-Id" && value == "7" }));
}
