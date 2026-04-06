//! Tail-split datasource routed import, import validation, and diff tests.

use super::*;
use crate::datasource::{
    classify_datasource_export_root_scope_kind, diff_datasources_with_live,
    discover_export_org_import_scopes, format_routed_datasource_scope_summary_fields,
    format_routed_datasource_target_org_label, load_datasource_export_root_manifest,
    load_datasource_inspect_export_source, load_datasource_inventory_records_from_export_root,
    load_import_records, prompt_datasource_inspect_export_input_format,
    render_datasource_inspect_export_output, render_routed_datasource_import_org_table,
    resolve_datasource_inspect_export_input_format, run_datasource_cli, DatasourceCliArgs,
    DatasourceExportRootScopeKind, DatasourceGroupCommand, DatasourceImportArgs,
    DatasourceImportInputFormat, DatasourceInspectExportRenderFormat,
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
    let input_dir = temp.path().join("datasources");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([])).unwrap(),
    )
    .unwrap();

    let error = run_datasource_cli(
        DatasourceCliArgs::parse_normalized_from([
            "grafana-util",
            "import",
            "--input-dir",
            input_dir.to_str().unwrap(),
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
    let input_dir = temp.path().join("datasources");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "root",
            "scopeKind": "org-root",
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
        input_dir.join("datasources.json"),
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
        input_dir.join("index.json"),
        serde_json::to_string_pretty(&json!({"items": []})).unwrap(),
    )
    .unwrap();

    let error =
        load_import_records(&input_dir, DatasourceImportInputFormat::Inventory).unwrap_err();

    assert!(error
        .to_string()
        .contains("unsupported datasource field(s): id, secureJsonData"));
}

#[test]
fn datasource_import_loads_provisioning_from_export_root_without_metadata() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("datasources");
    let provisioning_dir = export_root.join("provisioning");
    fs::create_dir_all(&provisioning_dir).unwrap();
    fs::write(
        provisioning_dir.join("datasources.yaml"),
        r#"apiVersion: 1
datasources:
  - uid: prom-main
    name: Prometheus Main
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    orgId: 7
"#,
    )
    .unwrap();

    let (metadata, records) =
        load_import_records(&export_root, DatasourceImportInputFormat::Provisioning).unwrap();

    assert_eq!(metadata.variant, "provisioning");
    assert_eq!(metadata.datasources_file, "provisioning/datasources.yaml");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].uid, "prom-main");
    assert_eq!(records[0].name, "Prometheus Main");
    assert_eq!(records[0].datasource_type, "prometheus");
    assert_eq!(records[0].access, "proxy");
    assert_eq!(records[0].url, "http://prometheus:9090");
    assert!(records[0].is_default);
    assert_eq!(records[0].org_id, "7");
}

#[test]
fn datasource_import_loads_provisioning_from_directory_or_yaml_file() {
    let temp = tempdir().unwrap();
    let provisioning_dir = temp.path().join("provisioning");
    fs::create_dir_all(&provisioning_dir).unwrap();
    let provisioning_file = provisioning_dir.join("datasources.yaml");
    fs::write(
        &provisioning_file,
        r#"apiVersion: 1
datasources:
  - uid: loki-main
    name: Loki Main
    type: loki
    access: proxy
    url: http://loki:3100
    isDefault: false
    orgId: 9
"#,
    )
    .unwrap();

    let (dir_metadata, dir_records) =
        load_import_records(&provisioning_dir, DatasourceImportInputFormat::Provisioning).unwrap();
    let (file_metadata, file_records) = load_import_records(
        &provisioning_file,
        DatasourceImportInputFormat::Provisioning,
    )
    .unwrap();

    assert_eq!(dir_metadata.datasources_file, "datasources.yaml");
    assert_eq!(file_metadata.datasources_file, "datasources.yaml");
    assert_eq!(dir_records.len(), 1);
    assert_eq!(dir_records[0].uid, "loki-main");
    assert_eq!(file_records, dir_records);
}

#[test]
fn datasource_import_loads_inventory_recovery_bundle_passthrough_fields() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("datasources");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "root",
            "scopeKind": "org-root",
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
        input_dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([{
            "uid": "loki-main",
            "name": "Loki Main",
            "type": "loki",
            "access": "proxy",
            "url": "http://loki:3100",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": 7,
            "basicAuth": true,
            "basicAuthUser": "loki-user",
            "database": "logs-main",
            "jsonData": {
                "httpMethod": "POST",
                "httpHeaderName1": "X-Scope-OrgID"
            },
            "secureJsonDataPlaceholders": {
                "basicAuthPassword": "${secret:loki-basic-auth}",
                "httpHeaderValue1": "${secret:loki-tenant-token}"
            },
            "user": "query-user",
            "withCredentials": true
        }]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        input_dir.join("index.json"),
        serde_json::to_string_pretty(&json!({"items": []})).unwrap(),
    )
    .unwrap();

    let (_, records) =
        load_import_records(&input_dir, DatasourceImportInputFormat::Inventory).unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].uid, "loki-main");
    assert_eq!(records[0].org_id, "7");
    assert_eq!(records[0].basic_auth, Some(true));
    assert_eq!(records[0].basic_auth_user, "loki-user");
    assert_eq!(records[0].database, "logs-main");
    assert_eq!(records[0].user, "query-user");
    assert_eq!(records[0].with_credentials, Some(true));
    assert_eq!(
        records[0].json_data.as_ref().unwrap()["httpMethod"],
        json!("POST")
    );
    assert_eq!(
        records[0].json_data.as_ref().unwrap()["httpHeaderName1"],
        json!("X-Scope-OrgID")
    );
    assert_eq!(
        records[0].secure_json_data_placeholders.as_ref().unwrap()["basicAuthPassword"],
        json!("${secret:loki-basic-auth}")
    );
    assert_eq!(
        records[0].secure_json_data_placeholders.as_ref().unwrap()["httpHeaderValue1"],
        json!("${secret:loki-tenant-token}")
    );
}

#[test]
fn datasource_import_loads_provisioning_recovery_bundle_passthrough_fields() {
    let temp = tempdir().unwrap();
    let provisioning_dir = temp.path().join("provisioning");
    fs::create_dir_all(&provisioning_dir).unwrap();
    let provisioning_file = provisioning_dir.join("datasources.yaml");
    fs::write(
        &provisioning_file,
        r#"apiVersion: 1
datasources:
  - uid: loki-main
    name: Loki Main
    type: loki
    access: proxy
    url: http://loki:3100
    isDefault: false
    orgId: 9
    basicAuth: true
    basicAuthUser: loki-user
    database: logs-main
    user: query-user
    withCredentials: true
    jsonData:
      httpMethod: POST
      httpHeaderName1: X-Scope-OrgID
    secureJsonDataPlaceholders:
      basicAuthPassword: ${secret:loki-basic-auth}
      httpHeaderValue1: ${secret:loki-tenant-token}
"#,
    )
    .unwrap();

    let (_, records) = load_import_records(
        &provisioning_file,
        DatasourceImportInputFormat::Provisioning,
    )
    .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].uid, "loki-main");
    assert_eq!(records[0].org_id, "9");
    assert_eq!(records[0].basic_auth, Some(true));
    assert_eq!(records[0].basic_auth_user, "loki-user");
    assert_eq!(records[0].database, "logs-main");
    assert_eq!(records[0].user, "query-user");
    assert_eq!(records[0].with_credentials, Some(true));
    assert_eq!(
        records[0].json_data.as_ref().unwrap()["httpMethod"],
        json!("POST")
    );
    assert_eq!(
        records[0].json_data.as_ref().unwrap()["httpHeaderName1"],
        json!("X-Scope-OrgID")
    );
    assert_eq!(
        records[0].secure_json_data_placeholders.as_ref().unwrap()["basicAuthPassword"],
        json!("${secret:loki-basic-auth}")
    );
    assert_eq!(
        records[0].secure_json_data_placeholders.as_ref().unwrap()["httpHeaderValue1"],
        json!("${secret:loki-tenant-token}")
    );
}

#[test]
fn datasource_inspect_export_renders_inventory_root_in_multiple_output_modes() {
    let root = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "org": "Main Org",
        "orgId": "1"
    })]);

    let source =
        load_datasource_inspect_export_source(&root, DatasourceImportInputFormat::Inventory)
            .unwrap();
    let table = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Table,
    )
    .unwrap();
    let text =
        render_datasource_inspect_export_output(&source, DatasourceInspectExportRenderFormat::Text)
            .unwrap();
    let json_output =
        render_datasource_inspect_export_output(&source, DatasourceInspectExportRenderFormat::Json)
            .unwrap();
    let yaml_output =
        render_datasource_inspect_export_output(&source, DatasourceInspectExportRenderFormat::Yaml)
            .unwrap();

    assert!(table.contains("UID"));
    assert!(table.contains("Layer: operator-summary"));
    assert!(table.contains("Mode: inventory"));
    assert!(table.contains("Prometheus Main"));
    assert!(text.contains("Layer: operator-summary"));
    assert!(text.contains("Mode: inventory"));
    assert!(text.contains("Bundle: recovery-capable masked export"));
    assert!(text.contains("Datasource count: 1"));
    assert!(text.contains("Prometheus Main"));
    assert!(json_output.contains("\"inputMode\": \"inventory\""));
    assert!(json_output.contains("\"bundleKind\": \"masked-recovery\""));
    assert!(json_output.contains("\"masked\": true"));
    assert!(json_output.contains("\"recoveryCapable\": true"));
    assert!(json_output.contains("\"datasourceCount\": 1"));
    assert!(yaml_output.contains("inputMode: inventory"));
    assert!(yaml_output.contains("bundleKind: masked-recovery"));
    assert!(yaml_output.contains("masked: true"));
    assert!(yaml_output.contains("recoveryCapable: true"));
    assert!(yaml_output.contains("datasourceCount: 1"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn datasource_inspect_export_renders_provisioning_yaml_file_as_csv_and_yaml() {
    let root = write_provisioning_diff_fixture();
    let provisioning_file = root.join("provisioning/datasources.yaml");

    let source = load_datasource_inspect_export_source(
        &provisioning_file,
        DatasourceImportInputFormat::Provisioning,
    )
    .unwrap();
    let csv_output =
        render_datasource_inspect_export_output(&source, DatasourceInspectExportRenderFormat::Csv)
            .unwrap();
    let yaml_output =
        render_datasource_inspect_export_output(&source, DatasourceInspectExportRenderFormat::Yaml)
            .unwrap();

    assert!(csv_output.contains("uid,name,type,url,isDefault"));
    assert!(csv_output.contains("Prometheus Main"));
    assert!(yaml_output.contains("bundleKind: masked-recovery"));
    assert!(yaml_output.contains("masked: true"));
    assert!(yaml_output.contains("recoveryCapable: true"));
    assert!(yaml_output.contains("inputMode: provisioning"));
    assert!(yaml_output.contains("Prometheus Main"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn datasource_list_help_mentions_local_inventory_source_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("list")
        .unwrap_or_else(|| panic!("missing datasource list help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--input-dir"));
    assert!(help.contains("--input-format"));
    assert!(help.contains("local"));
    assert!(help.contains("inventory"));
}

#[test]
fn datasource_export_root_manifest_classifies_org_and_workspace_roots() {
    let temp = tempdir().unwrap();
    let org_root = temp.path().join("org-root");
    fs::create_dir_all(&org_root).unwrap();
    fs::write(
        org_root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "root",
            "scopeKind": "org-root",
            "resource": "datasource",
            "datasourcesFile": "datasources.json",
            "datasourceCount": 0,
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
    let workspace_root = temp.path().join("workspace-root");
    fs::create_dir_all(&workspace_root).unwrap();
    fs::write(
        workspace_root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "all-orgs-root",
            "scopeKind": "workspace-root",
            "resource": "datasource",
            "indexFile": "index.json",
            "datasourceCount": 0,
            "orgCount": 0,
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

    let org_manifest =
        load_datasource_export_root_manifest(&org_root.join("export-metadata.json")).unwrap();
    let workspace_manifest =
        load_datasource_export_root_manifest(&workspace_root.join("export-metadata.json")).unwrap();

    assert_eq!(
        classify_datasource_export_root_scope_kind(&org_manifest.metadata),
        DatasourceExportRootScopeKind::OrgRoot
    );
    assert_eq!(
        org_manifest.scope_kind,
        DatasourceExportRootScopeKind::OrgRoot
    );
    assert_eq!(
        workspace_manifest.scope_kind,
        DatasourceExportRootScopeKind::WorkspaceRoot
    );
}

#[test]
fn datasource_inventory_root_loader_combines_all_orgs_children() {
    let temp = tempdir().unwrap();
    let root = write_multi_org_import_fixture(
        temp.path(),
        &[
            (
                1,
                "Main Org",
                vec![json!({
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": "true",
                    "org": "Main Org",
                    "orgId": "1"
                })],
            ),
            (
                2,
                "Ops Org",
                vec![json!({
                    "uid": "loki-ops",
                    "name": "Loki Ops",
                    "type": "loki",
                    "access": "proxy",
                    "url": "http://loki:3100",
                    "isDefault": "false",
                    "org": "Ops Org",
                    "orgId": "2"
                })],
            ),
        ],
    );

    let (manifest, records) = load_datasource_inventory_records_from_export_root(&root).unwrap();

    assert_eq!(
        manifest.scope_kind,
        DatasourceExportRootScopeKind::AllOrgsRoot
    );
    assert_eq!(records.len(), 2);
    assert_eq!(
        records
            .iter()
            .map(|record| record.org_id.as_str())
            .collect::<Vec<_>>(),
        vec!["1", "2"]
    );
}

#[test]
fn datasource_diff_help_mentions_operator_summary_report() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("diff")
        .unwrap_or_else(|| panic!("missing datasource diff help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--diff-dir"));
    assert!(help.contains("--input-format"));
    assert!(help.contains("provisioning"));
    assert!(help.contains("operator-summary diff report"));
    assert!(help.contains("datasource list --input-dir"));
}

#[test]
fn datasource_inspect_export_accepts_all_orgs_root_inventory() {
    let temp = tempdir().unwrap();
    let root = write_multi_org_import_fixture(
        temp.path(),
        &[
            (
                1,
                "Main Org",
                vec![json!({
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": "true",
                    "org": "Main Org",
                    "orgId": "1"
                })],
            ),
            (
                2,
                "Ops Org",
                vec![json!({
                    "uid": "loki-ops",
                    "name": "Loki Ops",
                    "type": "loki",
                    "access": "proxy",
                    "url": "http://loki:3100",
                    "isDefault": "false",
                    "org": "Ops Org",
                    "orgId": "2"
                })],
            ),
        ],
    );
    fs::write(
        root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "all-orgs-root",
            "scopeKind": "all-orgs-root",
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

    let source =
        load_datasource_inspect_export_source(&root, DatasourceImportInputFormat::Inventory)
            .unwrap();
    let text =
        render_datasource_inspect_export_output(&source, DatasourceInspectExportRenderFormat::Text)
            .unwrap();

    assert!(text.contains("Datasource count: 2"));
    assert!(text.contains("Bundle: recovery-capable masked export"));
    assert!(text.contains("Prometheus Main"));
    assert!(text.contains("Loki Ops"));
}

#[test]
fn datasource_inspect_export_resolves_workspace_root_inventory() {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("snapshot");
    let datasource_export_root = write_multi_org_import_fixture(
        &workspace_root,
        &[
            (
                1,
                "Main Org",
                vec![json!({
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": "true",
                    "org": "Main Org",
                    "orgId": "1"
                })],
            ),
            (
                3,
                "Ops Org",
                vec![json!({
                    "uid": "loki-ops",
                    "name": "Loki Ops",
                    "type": "loki",
                    "access": "proxy",
                    "url": "http://loki:3100",
                    "isDefault": "false",
                    "org": "Ops Org",
                    "orgId": "3"
                })],
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

    let input_format = resolve_datasource_inspect_export_input_format(&workspace_root, None)
        .unwrap()
        .unwrap();
    assert_eq!(input_format, DatasourceImportInputFormat::Inventory);

    let source = load_datasource_inspect_export_source(
        &workspace_root,
        DatasourceImportInputFormat::Inventory,
    )
    .unwrap();
    let text =
        render_datasource_inspect_export_output(&source, DatasourceInspectExportRenderFormat::Text)
            .unwrap();

    assert!(text.contains("Variant: all-orgs-root"));
    assert!(text.contains("Datasource count: 2"));
    assert!(text.contains("Prometheus Main"));
    assert!(text.contains("Loki Ops"));
}

#[test]
fn datasource_inspect_export_prefers_inventory_for_noninteractive_ambiguous_root() {
    let root = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "org": "Main Org",
        "orgId": "1"
    })]);
    fs::create_dir_all(root.join("provisioning")).unwrap();
    fs::write(
        root.join("provisioning/datasources.yaml"),
        r#"apiVersion: 1
datasources:
  - name: Provisioned Loki
    uid: loki-prov
    type: loki
    access: proxy
    url: http://loki:3100
"#,
    )
    .unwrap();

    let mode = resolve_datasource_inspect_export_input_format(
        &root,
        Some(DatasourceImportInputFormat::Inventory),
    )
    .unwrap()
    .unwrap();
    assert_eq!(mode, DatasourceImportInputFormat::Inventory);

    let source = load_datasource_inspect_export_source(&root, mode).unwrap();
    let text =
        render_datasource_inspect_export_output(&source, DatasourceInspectExportRenderFormat::Text)
            .unwrap();
    assert!(text.contains("Prometheus Main"));
    assert!(!text.contains("Provisioned Loki"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn datasource_inspect_export_requires_explicit_input_type_without_tty_for_ambiguous_root() {
    let root = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "org": "Main Org",
        "orgId": "1"
    })]);
    fs::create_dir_all(root.join("provisioning")).unwrap();
    fs::write(
        root.join("provisioning/datasources.yaml"),
        "apiVersion: 1\ndatasources: []\n",
    )
    .unwrap();

    let error = prompt_datasource_inspect_export_input_format(&root).unwrap_err();
    assert!(error
        .to_string()
        .contains("--input-format inventory or --input-format provisioning"));

    fs::remove_dir_all(root).unwrap();
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

#[test]
fn parse_datasource_diff_preserves_requested_path() {
    let args =
        DatasourceCliArgs::parse_from(["grafana-util", "diff", "--diff-dir", "./datasources"]);

    match args.command {
        DatasourceGroupCommand::Diff(inner) => {
            assert_eq!(inner.diff_dir, Path::new("./datasources"));
            assert_eq!(inner.input_format, DatasourceImportInputFormat::Inventory);
        }
        _ => panic!("expected datasource diff"),
    }
}

#[test]
fn parse_datasource_diff_supports_provisioning_input_format() {
    let args = DatasourceCliArgs::parse_from([
        "grafana-util",
        "diff",
        "--diff-dir",
        "./datasources/provisioning",
        "--input-format",
        "provisioning",
    ]);

    match args.command {
        DatasourceGroupCommand::Diff(inner) => {
            assert_eq!(inner.diff_dir, Path::new("./datasources/provisioning"));
            assert_eq!(
                inner.input_format,
                DatasourceImportInputFormat::Provisioning
            );
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

    let (compared_count, differences) = diff_datasources_with_live(
        &diff_dir,
        DatasourceImportInputFormat::Inventory,
        &live,
        crate::common::DiffOutputFormat::Text,
    )
    .unwrap();

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

    let (compared_count, differences) = diff_datasources_with_live(
        &diff_dir,
        DatasourceImportInputFormat::Inventory,
        &live,
        crate::common::DiffOutputFormat::Text,
    )
    .unwrap();

    assert_eq!(compared_count, 1);
    assert_eq!(differences, 1);
    fs::remove_dir_all(diff_dir).unwrap();
}

#[test]
fn diff_datasources_with_live_supports_provisioning_root_directory_and_file() {
    let diff_root = write_provisioning_diff_fixture();
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

    let provisioning_dir = diff_root.join("provisioning");
    let provisioning_file = provisioning_dir.join("datasources.yaml");

    let (root_count, root_differences) = diff_datasources_with_live(
        &diff_root,
        DatasourceImportInputFormat::Provisioning,
        &live,
        crate::common::DiffOutputFormat::Text,
    )
    .unwrap();
    let (dir_count, dir_differences) = diff_datasources_with_live(
        &provisioning_dir,
        DatasourceImportInputFormat::Provisioning,
        &live,
        crate::common::DiffOutputFormat::Text,
    )
    .unwrap();
    let (file_count, file_differences) = diff_datasources_with_live(
        &provisioning_file,
        DatasourceImportInputFormat::Provisioning,
        &live,
        crate::common::DiffOutputFormat::Text,
    )
    .unwrap();

    assert_eq!((root_count, root_differences), (1, 0));
    assert_eq!((dir_count, dir_differences), (1, 0));
    assert_eq!((file_count, file_differences), (1, 0));
    fs::remove_dir_all(diff_root).unwrap();
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

fn write_provisioning_diff_fixture() -> std::path::PathBuf {
    let root = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })]);
    let provisioning_dir = root.join("provisioning");
    fs::create_dir_all(&provisioning_dir).unwrap();
    fs::write(
        provisioning_dir.join("datasources.yaml"),
        r#"apiVersion: 1
datasources:
  - uid: prom-main
    name: Prometheus Main
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    jsonData:
      httpMethod: POST
    isDefault: true
    orgId: 1
"#,
    )
    .unwrap();
    root
}

fn write_multi_org_import_fixture(
    root: &Path,
    orgs: &[(i64, &str, Vec<Value>)],
) -> std::path::PathBuf {
    let import_root = root.join("datasource-export-all-orgs");
    fs::create_dir_all(&import_root).unwrap();
    let total_datasource_count = orgs
        .iter()
        .map(|(_, _, records)| records.len())
        .sum::<usize>();
    fs::write(
        import_root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "all-orgs-root",
            "scopeKind": "all-orgs-root",
            "resource": "datasource",
            "indexFile": "index.json",
            "datasourceCount": total_datasource_count,
            "orgCount": orgs.len(),
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
    for (org_id, org_name, records) in orgs {
        let org_dir = import_root.join(format!("org_{}_{}", org_id, org_name.replace(' ', "_")));
        fs::create_dir_all(&org_dir).unwrap();
        fs::write(
            org_dir.join("export-metadata.json"),
            serde_json::to_vec_pretty(&json!({
                "schemaVersion": 1,
                "kind": "grafana-utils-datasource-export-index",
                "variant": "root",
                "scopeKind": "org-root",
                "resource": "datasource",
                "datasourcesFile": "datasources.json",
                "indexFile": "index.json",
                "datasourceCount": records.len(),
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
                "exportMode": "masked-recovery",
                "masked": true,
                "recoveryCapable": true,
                "secretMaterial": "placeholders-only",
                "variants": {
                    "inventory": "datasources.json",
                    "provisioning": "provisioning/datasources.yaml"
                },
                "count": records.len(),
                "items": index_items
            }))
            .unwrap(),
        )
        .unwrap();
    }
    import_root
}
