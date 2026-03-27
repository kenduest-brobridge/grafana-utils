//! Tail-split datasource routed import, import validation, and diff tests.

use super::*;
use crate::datasource::{
    diff_datasources_with_live, discover_export_org_import_scopes,
    format_routed_datasource_scope_summary_fields, format_routed_datasource_target_org_label,
    load_import_records, render_routed_datasource_import_org_table, run_datasource_cli,
    DatasourceGroupCommand, DatasourceImportArgs,
};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

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
fn datasource_import_rejects_output_columns_without_table_output() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("datasources");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([])).unwrap(),
    )
    .unwrap();

    let error = run_datasource_cli(
        DatasourceCliArgs::parse_normalized_from([
            "grafana-util",
            "import",
            "--import-dir",
            import_dir.to_str().unwrap(),
            "--token",
            "token",
            "--dry-run",
            "--output-columns",
            "uid",
        ])
        .command,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("--output-columns is only supported with --dry-run --table"));
}

#[test]
fn datasource_import_rejects_extra_secret_or_server_managed_fields() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("datasources");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "root",
            "resource": "datasource",
            "datasourcesFile": "datasources.json",
            "indexFile": "index.json",
            "datasourceCount": 1,
            "format": "grafana-datasource-inventory-v1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        import_dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([{
            "uid": "prom-main",
            "name": "Prometheus Main",
            "type": "prometheus",
            "access": "proxy",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": "1",
            "id": 7,
            "secureJsonData": {"httpHeaderValue1": "secret"}
        }]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        import_dir.join("index.json"),
        serde_json::to_string_pretty(&json!({"items": []})).unwrap(),
    )
    .unwrap();

    let error = load_import_records(&import_dir).unwrap_err();

    assert!(error
        .to_string()
        .contains("unsupported datasource field(s): id, secureJsonData"));
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
        import_dir: import_root,
        org_id: None,
        use_export_org: true,
        only_org_id: vec![2],
        create_missing_orgs: false,
        require_matching_export_org: false,
        replace_existing: false,
        update_existing_only: false,
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    };

    let scopes = discover_export_org_import_scopes(&args).unwrap();

    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].source_org_id, 2);
    assert_eq!(scopes[0].source_org_name, "Org Two");
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
        import_dir: import_root,
        org_id: None,
        use_export_org: true,
        only_org_id: vec![9],
        create_missing_orgs: false,
        require_matching_export_org: false,
        replace_existing: false,
        update_existing_only: false,
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
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
            "--import-dir",
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

#[test]
fn diff_help_explains_diff_dir_flag() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("diff")
        .unwrap_or_else(|| panic!("missing datasource diff help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--diff-dir"));
    assert!(help.contains("Compare datasource inventory"));
}

#[test]
fn parse_datasource_diff_preserves_requested_path() {
    let args =
        DatasourceCliArgs::parse_from(["grafana-util", "diff", "--diff-dir", "./datasources"]);

    match args.command {
        DatasourceGroupCommand::Diff(inner) => {
            assert_eq!(inner.diff_dir, Path::new("./datasources"));
        }
        _ => panic!("expected datasource diff"),
    }
}

#[test]
fn diff_datasources_with_live_returns_zero_for_matching_inventory() {
    let diff_dir = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })]);
    let live = vec![json!({
        "id": 7,
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })
    .as_object()
    .unwrap()
    .clone()];

    let (compared_count, differences) = diff_datasources_with_live(&diff_dir, &live).unwrap();

    assert_eq!(compared_count, 1);
    assert_eq!(differences, 0);
    fs::remove_dir_all(diff_dir).unwrap();
}

#[test]
fn diff_datasources_with_live_detects_changed_inventory() {
    let diff_dir = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })]);
    let live = vec![json!({
        "id": 7,
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "direct",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })
    .as_object()
    .unwrap()
    .clone()];

    let (compared_count, differences) = diff_datasources_with_live(&diff_dir, &live).unwrap();

    assert_eq!(compared_count, 1);
    assert_eq!(differences, 1);
    fs::remove_dir_all(diff_dir).unwrap();
}

fn write_diff_fixture(records: &[Value]) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("grafana-util-datasource-diff-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "root",
            "resource": "datasource",
            "datasourcesFile": "datasources.json",
            "indexFile": "index.json",
            "datasourceCount": records.len(),
            "format": "grafana-datasource-inventory-v1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("datasources.json"),
        serde_json::to_vec_pretty(&Value::Array(records.to_vec())).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("index.json"),
        serde_json::to_vec_pretty(&json!({"items": []})).unwrap(),
    )
    .unwrap();
    dir
}

fn write_multi_org_import_fixture(
    root: &Path,
    orgs: &[(i64, &str, Vec<Value>)],
) -> std::path::PathBuf {
    let import_root = root.join("datasource-export-all-orgs");
    fs::create_dir_all(&import_root).unwrap();
    for (org_id, org_name, records) in orgs {
        let org_dir = import_root.join(format!("org_{}_{}", org_id, org_name.replace(' ', "_")));
        fs::create_dir_all(&org_dir).unwrap();
        fs::write(
            org_dir.join("export-metadata.json"),
            serde_json::to_vec_pretty(&json!({
                "schemaVersion": 1,
                "kind": "grafana-utils-datasource-export-index",
                "variant": "root",
                "resource": "datasource",
                "datasourcesFile": "datasources.json",
                "indexFile": "index.json",
                "datasourceCount": records.len(),
                "format": "grafana-datasource-inventory-v1"
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            org_dir.join("datasources.json"),
            serde_json::to_vec_pretty(&Value::Array(records.clone())).unwrap(),
        )
        .unwrap();
        let index_items = records
            .iter()
            .map(|record| {
                let object = record.as_object().unwrap();
                json!({
                    "uid": object.get("uid").cloned().unwrap_or(Value::String(String::new())),
                    "name": object.get("name").cloned().unwrap_or(Value::String(String::new())),
                    "type": object.get("type").cloned().unwrap_or(Value::String(String::new())),
                    "org": object.get("org").cloned().unwrap_or(Value::String(org_name.to_string())),
                    "orgId": object.get("orgId").cloned().unwrap_or(Value::String(org_id.to_string())),
                })
            })
            .collect::<Vec<Value>>();
        fs::write(
            org_dir.join("index.json"),
            serde_json::to_vec_pretty(&json!({
                "kind": "grafana-utils-datasource-export-index",
                "schemaVersion": 1,
                "datasourcesFile": "datasources.json",
                "count": records.len(),
                "items": index_items
            }))
            .unwrap(),
        )
        .unwrap();
    }
    import_root
}
