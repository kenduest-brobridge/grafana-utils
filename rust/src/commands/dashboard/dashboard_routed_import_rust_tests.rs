//! Dashboard routed import regression tests.
#![allow(unused_imports)]

use super::*;

#[test]
fn render_routed_import_org_table_includes_org_level_columns() {
    let rows = vec![
        [
            "2".to_string(),
            "Org Two".to_string(),
            "exists".to_string(),
            "2".to_string(),
            "3".to_string(),
        ],
        [
            "9".to_string(),
            "Ops Org".to_string(),
            "would-create".to_string(),
            "<new>".to_string(),
            "1".to_string(),
        ],
    ];

    let lines = test_support::import::render_routed_import_org_table(&rows, true);

    assert!(lines[0].contains("SOURCE_ORG_ID"));
    assert!(lines[0].contains("ORG_ACTION"));
    assert!(lines[2].contains("Org Two"));
    assert!(lines[3].contains("would-create"));
}

#[test]
fn routed_import_scope_identity_matches_table_json_and_progress_surfaces() {
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
        .unwrap(),
    )
    .unwrap();

    let org_entries = payload["orgs"].as_array().unwrap();
    let rows: Vec<[String; 5]> = org_entries
        .iter()
        .map(|entry| {
            [
                entry["sourceOrgId"].as_i64().unwrap().to_string(),
                entry["sourceOrgName"].as_str().unwrap().to_string(),
                entry["orgAction"].as_str().unwrap().to_string(),
                test_support::import::format_routed_import_target_org_label(
                    entry["targetOrgId"].as_i64(),
                ),
                entry["dashboardCount"].as_u64().unwrap().to_string(),
            ]
        })
        .collect();
    let table_lines = test_support::import::render_routed_import_org_table(&rows, true);

    let org_two = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let org_nine = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();

    let existing_summary = test_support::import::format_routed_import_scope_summary_fields(
        2,
        "Org Two",
        "exists",
        Some(2),
        Path::new(org_two["importDir"].as_str().unwrap()),
    );
    let would_create_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "would-create",
        None,
        Path::new(org_nine["importDir"].as_str().unwrap()),
    );

    assert_eq!(org_two["targetOrgId"], json!(2));
    assert_eq!(org_nine["targetOrgId"], Value::Null);
    assert!(table_lines[2].contains("Org Two"));
    assert!(table_lines[2].contains("2"));
    assert!(table_lines[3].contains("Ops Org"));
    assert!(table_lines[3].contains("<new>"));
    assert!(existing_summary.contains("export orgId=2"));
    assert!(existing_summary.contains("name=Org Two"));
    assert!(existing_summary.contains("orgAction=exists"));
    assert!(existing_summary.contains("targetOrgId=2"));
    assert!(existing_summary.contains(org_two["importDir"].as_str().unwrap()));
    assert!(would_create_summary.contains("export orgId=9"));
    assert!(would_create_summary.contains("name=Ops Org"));
    assert!(would_create_summary.contains("orgAction=would-create"));
    assert!(would_create_summary.contains("targetOrgId=<new>"));
    assert!(would_create_summary.contains(org_nine["importDir"].as_str().unwrap()));
}

#[test]
fn routed_import_selected_scope_statuses_match_json_table_and_summary_contract() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_five_raw = export_root.join("org_5_Org_Five").join("raw");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("2", "Org Two", "org_2_Org_Two"),
            ("5", "Org Five", "org_5_Org_Five"),
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
        &org_five_raw,
        "5",
        "Org Five",
        "cpu-five",
        "CPU Five",
        "prom-five",
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

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.only_org_id = vec![2, 9];
    args.create_missing_orgs = false;
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
        .unwrap(),
    )
    .unwrap();

    let org_entries = payload["orgs"].as_array().unwrap();
    let import_entries = payload["imports"].as_array().unwrap();
    assert_eq!(org_entries.len(), 2);
    assert_eq!(import_entries.len(), 2);
    assert_eq!(payload["summary"]["orgCount"], json!(2));
    assert_eq!(payload["summary"]["existingOrgCount"], json!(1));
    assert_eq!(payload["summary"]["missingOrgCount"], json!(1));
    assert_eq!(payload["summary"]["wouldCreateOrgCount"], json!(0));

    let org_two = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let org_nine = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    assert!(org_entries
        .iter()
        .all(|entry| entry["sourceOrgId"] != json!(5)));

    assert_eq!(org_two["orgAction"], json!("exists"));
    assert_eq!(org_two["targetOrgId"], json!(2));
    assert_eq!(org_nine["orgAction"], json!("missing"));
    assert_eq!(org_nine["targetOrgId"], Value::Null);

    let org_nine_import = import_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    assert_eq!(org_nine_import["orgAction"], json!("missing"));
    assert_eq!(org_nine_import["dashboards"], json!([]));
    assert_eq!(org_nine_import["summary"]["dashboardCount"], json!(1));

    let rows: Vec<[String; 5]> = org_entries
        .iter()
        .map(|entry| {
            [
                entry["sourceOrgId"].as_i64().unwrap().to_string(),
                entry["sourceOrgName"].as_str().unwrap().to_string(),
                entry["orgAction"].as_str().unwrap().to_string(),
                test_support::import::format_routed_import_target_org_label(
                    entry["targetOrgId"].as_i64(),
                ),
                entry["dashboardCount"].as_u64().unwrap().to_string(),
            ]
        })
        .collect();
    let table_lines = test_support::import::render_routed_import_org_table(&rows, true);
    assert!(table_lines[2].contains("Org Two"));
    assert!(table_lines[2].contains("exists"));
    assert!(table_lines[2].contains("2"));
    assert!(table_lines[3].contains("Ops Org"));
    assert!(table_lines[3].contains("missing"));
    assert!(table_lines[3].contains("<new>"));

    let missing_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "missing",
        None,
        Path::new(org_nine["importDir"].as_str().unwrap()),
    );
    assert!(missing_summary.contains("export orgId=9"));
    assert!(missing_summary.contains("name=Ops Org"));
    assert!(missing_summary.contains("orgAction=missing"));
    assert!(missing_summary.contains("targetOrgId=<new>"));
    assert!(missing_summary.contains(org_nine["importDir"].as_str().unwrap()));
}
