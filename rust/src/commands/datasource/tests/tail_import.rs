//! Datasource import validation and import loader tests.

use crate::datasource::{
    load_import_records, run_datasource_cli, DatasourceCliArgs, DatasourceImportInputFormat,
};
use std::fs;
use tempfile::tempdir;

use super::*;

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
