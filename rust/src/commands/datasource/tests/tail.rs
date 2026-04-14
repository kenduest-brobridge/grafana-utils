//! Tail-split datasource routed import, import validation, and diff tests.

use super::*;
use crate::datasource::{
    discover_export_org_import_scopes, format_routed_datasource_import_summary_line,
    format_routed_datasource_scope_summary_fields, format_routed_datasource_target_org_label,
    render_routed_datasource_import_org_table, run_datasource_cli, DatasourceCliArgs,
    DatasourceImportArgs, DatasourceImportInputFormat,
};
use std::fs;
use tempfile::tempdir;

#[path = "tail_diff.rs"]
mod datasource_tail_diff_rust_tests;
#[path = "tail_fixtures.rs"]
mod tail_fixtures;
#[path = "tail_import.rs"]
mod tail_import;
#[path = "tail_inspect.rs"]
mod tail_inspect;

use self::tail_fixtures::write_multi_org_import_fixture;

#[test]
fn routed_datasource_scope_identity_matches_table_json_and_progress_surfaces() {
    let dry_run = json!({
        "mode": "create-or-update",
        "orgs": [
            {
                "sourceOrgId": 2,
                "sourceOrgName": "Org Two",
                "orgAction": "exists",
                "targetOrgId": 2,
                "datasourceCount": 1,
                "importDir": "/tmp/datasource-export-all-orgs/org_2_Org_Two"
            },
            {
                "sourceOrgId": 9,
                "sourceOrgName": "Ops Org",
                "orgAction": "would-create",
                "targetOrgId": Value::Null,
                "datasourceCount": 1,
                "importDir": "/tmp/datasource-export-all-orgs/org_9_Ops_Org"
            }
        ],
        "imports": [
            {
                "sourceOrgId": 2,
                "sourceOrgName": "Org Two",
                "orgAction": "exists",
                "targetOrgId": 2
            },
            {
                "sourceOrgId": 9,
                "sourceOrgName": "Ops Org",
                "orgAction": "would-create",
                "targetOrgId": Value::Null
            }
        ]
    });
    let org_entries = dry_run["orgs"].as_array().unwrap();
    let org_two = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let org_nine = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    let rows: Vec<Vec<String>> = org_entries
        .iter()
        .map(|entry| {
            vec![
                entry["sourceOrgId"].as_i64().unwrap().to_string(),
                entry["sourceOrgName"].as_str().unwrap().to_string(),
                entry["orgAction"].as_str().unwrap().to_string(),
                format_routed_datasource_target_org_label(entry["targetOrgId"].as_i64()),
                entry["datasourceCount"].as_u64().unwrap().to_string(),
                entry["importDir"].as_str().unwrap().to_string(),
            ]
        })
        .collect();
    let table_lines = render_routed_datasource_import_org_table(&rows, true);

    let existing_summary = format_routed_datasource_scope_summary_fields(
        2,
        "Org Two",
        "exists",
        Some(2),
        Path::new(org_two["importDir"].as_str().unwrap()),
    );
    let would_create_summary = format_routed_datasource_scope_summary_fields(
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
    assert!(existing_summary.contains("orgAction=exists"));
    assert!(existing_summary.contains("targetOrgId=2"));
    assert!(would_create_summary.contains("orgAction=would-create"));
    assert!(would_create_summary.contains("targetOrgId=<new>"));
}

#[test]
fn routed_datasource_status_matrix_covers_exists_missing_would_create_and_created() {
    let missing_payload = json!({
        "summary": {
            "existingOrgCount": 1,
            "missingOrgCount": 1,
            "wouldCreateOrgCount": 0
        },
        "orgs": [
            {
                "sourceOrgId": 2,
                "sourceOrgName": "Org Two",
                "orgAction": "exists",
                "targetOrgId": 2,
                "importDir": "/tmp/datasource-export-all-orgs/org_2_Org_Two"
            },
            {
                "sourceOrgId": 9,
                "sourceOrgName": "Ops Org",
                "orgAction": "missing",
                "targetOrgId": Value::Null,
                "importDir": "/tmp/datasource-export-all-orgs/org_9_Ops_Org"
            }
        ]
    });
    let would_create_payload = json!({
        "summary": {
            "existingOrgCount": 1,
            "missingOrgCount": 0,
            "wouldCreateOrgCount": 1
        },
        "orgs": [
            {
                "sourceOrgId": 2,
                "sourceOrgName": "Org Two",
                "orgAction": "exists",
                "targetOrgId": 2,
                "importDir": "/tmp/datasource-export-all-orgs/org_2_Org_Two"
            },
            {
                "sourceOrgId": 9,
                "sourceOrgName": "Ops Org",
                "orgAction": "would-create",
                "targetOrgId": Value::Null,
                "importDir": "/tmp/datasource-export-all-orgs/org_9_Ops_Org"
            }
        ]
    });

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

    let existing_summary = format_routed_datasource_scope_summary_fields(
        2,
        "Org Two",
        "exists",
        Some(2),
        Path::new(missing_existing["importDir"].as_str().unwrap()),
    );
    let missing_summary = format_routed_datasource_scope_summary_fields(
        9,
        "Ops Org",
        "missing",
        None,
        Path::new(missing_missing["importDir"].as_str().unwrap()),
    );
    let would_create_summary = format_routed_datasource_scope_summary_fields(
        9,
        "Ops Org",
        "would-create",
        None,
        Path::new(would_create_missing["importDir"].as_str().unwrap()),
    );
    let created_summary = format_routed_datasource_scope_summary_fields(
        9,
        "Ops Org",
        "created",
        Some(19),
        Path::new(would_create_missing["importDir"].as_str().unwrap()),
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
fn routed_datasource_import_summary_line_reports_org_and_datasource_totals() {
    let summary = format_routed_datasource_import_summary_line(
        3,
        &["2:Org Two".to_string(), "9:Ops Org".to_string()],
        1,
        1,
        1,
        7,
        Path::new("/tmp/datasource-export-all-orgs"),
    );

    assert_eq!(
        summary,
        "Routed datasource import summary: orgs=3 sources=[2:Org Two, 9:Ops Org] existing=1 missing=1 would-create=1 datasources=7 from /tmp/datasource-export-all-orgs"
    );
}

#[test]
fn discover_export_org_import_scopes_reads_selected_multi_org_root() {
    let temp = tempdir().unwrap();
    let import_root = write_multi_org_import_fixture(
        temp.path(),
        &[
            (
                1,
                "Main Org",
                vec![
                    json!({"uid":"prom-main","name":"Prometheus Main","type":"prometheus","access":"proxy","url":"http://prometheus:9090","isDefault":"true","org":"Main Org","orgId":"1"}),
                ],
            ),
            (
                2,
                "Org Two",
                vec![
                    json!({"uid":"prom-two","name":"Prometheus Two","type":"prometheus","access":"proxy","url":"http://prometheus-2:9090","isDefault":"false","org":"Org Two","orgId":"2"}),
                ],
            ),
        ],
    );
    let args = DatasourceImportArgs {
        common: test_datasource_common_args(),
        input_dir: import_root,
        input_format: DatasourceImportInputFormat::Inventory,
        org_id: None,
        use_export_org: true,
        only_org_id: vec![2],
        create_missing_orgs: false,
        require_matching_export_org: false,
        replace_existing: false,
        update_existing_only: false,
        secret_values: None,
        secret_values_file: None,
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

    let scopes = discover_export_org_import_scopes(&args).unwrap();

    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].source_org_id, 2);
    assert_eq!(scopes[0].source_org_name, "Org Two");
}

#[test]
fn discover_export_org_import_scopes_accepts_workspace_root_and_sorts_children() {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("snapshot");
    let datasource_export_root = write_multi_org_import_fixture(
        &workspace_root,
        &[
            (
                9,
                "Ops Org",
                vec![
                    json!({"uid":"loki-ops","name":"Loki Ops","type":"loki","access":"proxy","url":"http://loki:3100","isDefault":"false","org":"Ops Org","orgId":"9"}),
                ],
            ),
            (
                2,
                "Org Two",
                vec![
                    json!({"uid":"prom-two","name":"Prometheus Two","type":"prometheus","access":"proxy","url":"http://prometheus-2:9090","isDefault":"false","org":"Org Two","orgId":"2"}),
                ],
            ),
        ],
    );
    let datasource_root = workspace_root.join("datasources");
    fs::rename(&datasource_export_root, &datasource_root).unwrap();
    fs::create_dir_all(workspace_root.join("dashboards")).unwrap();
    fs::write(
        datasource_root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "all-orgs-root",
            "scopeKind": "workspace-root",
            "resource": "datasource",
            "indexFile": "index.json",
            "datasourceCount": 2,
            "orgCount": 2,
            "format": "grafana-datasource-masked-recovery-v1",
            "exportMode": "masked-recovery",
            "masked": true,
            "recoveryCapable": true,
            "secretMaterial": "placeholders-only",
            "provisioningProjection": "derived-projection"
        }))
        .unwrap(),
    )
    .unwrap();
    let args = DatasourceImportArgs {
        common: test_datasource_common_args(),
        input_dir: workspace_root,
        input_format: DatasourceImportInputFormat::Inventory,
        org_id: None,
        use_export_org: true,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        require_matching_export_org: false,
        replace_existing: false,
        update_existing_only: false,
        secret_values: None,
        secret_values_file: None,
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

    let scopes = discover_export_org_import_scopes(&args).unwrap();

    assert_eq!(
        scopes
            .iter()
            .map(|scope| scope.source_org_id)
            .collect::<Vec<i64>>(),
        vec![2, 9]
    );
    assert_eq!(scopes[0].source_org_name, "Org Two");
    assert!(scopes[0].input_dir.ends_with("org_2_Org_Two"));
    assert!(scopes[1].input_dir.ends_with("org_9_Ops_Org"));
}

#[test]
fn discover_export_org_import_scopes_errors_when_selected_org_missing() {
    let temp = tempdir().unwrap();
    let import_root = write_multi_org_import_fixture(
        temp.path(),
        &[(
            1,
            "Main Org",
            vec![
                json!({"uid":"prom-main","name":"Prometheus Main","type":"prometheus","access":"proxy","url":"http://prometheus:9090","isDefault":"true","org":"Main Org","orgId":"1"}),
            ],
        )],
    );
    let args = DatasourceImportArgs {
        common: test_datasource_common_args(),
        input_dir: import_root,
        input_format: DatasourceImportInputFormat::Inventory,
        org_id: None,
        use_export_org: true,
        only_org_id: vec![9],
        create_missing_orgs: false,
        require_matching_export_org: false,
        replace_existing: false,
        update_existing_only: false,
        secret_values: None,
        secret_values_file: None,
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

    let error = discover_export_org_import_scopes(&args).unwrap_err();

    assert!(error
        .to_string()
        .contains("Selected exported org IDs were not found"));
}

#[test]
fn datasource_import_with_use_export_org_requires_basic_auth() {
    let temp = tempdir().unwrap();
    let import_root = write_multi_org_import_fixture(
        temp.path(),
        &[(
            1,
            "Main Org",
            vec![
                json!({"uid":"prom-main","name":"Prometheus Main","type":"prometheus","access":"proxy","url":"http://prometheus:9090","isDefault":"true","org":"Main Org","orgId":"1"}),
            ],
        )],
    );

    let error = run_datasource_cli(
        DatasourceCliArgs::parse_normalized_from([
            "grafana-util",
            "import",
            "--url",
            "http://grafana.example",
            "--token",
            "token",
            "--input-dir",
            import_root.to_str().unwrap(),
            "--use-export-org",
            "--dry-run",
        ])
        .command,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("Datasource import with --use-export-org requires Basic auth"));
}
