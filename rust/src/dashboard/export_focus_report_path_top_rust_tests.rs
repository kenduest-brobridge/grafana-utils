//! Dashboard domain test suite.
//! Covers parser surfaces, formatter/output contracts, and export/import/inspect/list/diff
//! behavior with in-memory/mocked request fixtures.
#![allow(unused_imports)]

use super::test_support;
use super::{export_dashboards_with_request, make_common_args, ExportArgs};
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;

#[test]
fn export_dashboards_with_request_all_orgs_aggregates_results() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: true,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: true,
        provisioning_provider_name: "grafana-utils-dashboards".to_string(),
        provisioning_provider_org_id: None,
        provisioning_provider_path: None,
        provisioning_provider_disable_deletion: false,
        provisioning_provider_allow_ui_updates: false,
        provisioning_provider_update_interval_seconds: 30,
        dry_run: false,
        progress: false,
        verbose: false,
    };
    let mut calls = Vec::new();

    let count = export_dashboards_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            let scoped_org = params
                .iter()
                .find(|(key, _)| key == "orgId")
                .map(|(_, value)| value.as_str());
            match (path, scoped_org) {
                ("/api/orgs", None) => Ok(Some(json!([
                    {"id": 1, "name": "Main Org"},
                    {"id": 2, "name": "Ops Org"}
                ]))),
                ("/api/org", Some("1")) => Ok(Some(json!({"id": 1, "name": "Main Org"}))),
                ("/api/org", Some("2")) => Ok(Some(json!({"id": 2, "name": "Ops Org"}))),
                ("/api/search", Some("1")) => Ok(Some(
                    json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }]),
                )),
                ("/api/datasources", Some("1")) => Ok(Some(json!([
                    {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus", "url": "http://prometheus:9090", "access": "proxy", "isDefault": true}
                ]))),
                ("/api/search", Some("2")) => Ok(Some(
                    json!([{ "uid": "xyz", "title": "Logs", "folderTitle": "Ops" }]),
                )),
                ("/api/datasources", Some("2")) => Ok(Some(json!([
                    {"uid": "logs-main", "name": "Logs Main", "type": "loki", "url": "http://loki:3100", "access": "proxy", "isDefault": false}
                ]))),
                ("/api/dashboards/uid/abc", Some("1")) => Ok(Some(
                    json!({"dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": [
                        {"datasource": {"uid": "prom-main", "type": "prometheus"}}
                    ]}}),
                )),
                ("/api/dashboards/uid/xyz", Some("2")) => Ok(Some(
                    json!({"dashboard": {"id": 8, "uid": "xyz", "title": "Logs", "panels": [
                        {"datasource": {"uid": "logs-main", "type": "loki"}}
                    ]}}),
                )),
                ("/api/dashboards/uid/abc/permissions", Some("1")) => Ok(Some(json!([
                    {"userId": 11, "userLogin": "ops", "permission": 4}
                ]))),
                ("/api/dashboards/uid/xyz/permissions", Some("2")) => Ok(Some(json!([
                    {"teamId": 21, "team": "SRE", "permission": 2}
                ]))),
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 2);
    assert!(args
        .export_dir
        .join("org_1_Main_Org/raw/Infra/CPU__abc.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_1_Main_Org/raw/index.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_1_Main_Org/raw/export-metadata.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_1_Main_Org/raw/folders.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_1_Main_Org/raw/datasources.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_1_Main_Org/raw/permissions.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_2_Ops_Org/raw/Ops/Logs__xyz.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_2_Ops_Org/raw/index.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_2_Ops_Org/raw/permissions.json")
        .is_file());
    let aggregate_root_index: Value =
        serde_json::from_str(&fs::read_to_string(args.export_dir.join("index.json")).unwrap())
            .unwrap();
    let aggregate_root_metadata: Value = serde_json::from_str(
        &fs::read_to_string(args.export_dir.join("export-metadata.json")).unwrap(),
    )
    .unwrap();
    let org_one_metadata: Value = serde_json::from_str(
        &fs::read_to_string(
            args.export_dir
                .join("org_1_Main_Org/raw/export-metadata.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let org_two_metadata: Value = serde_json::from_str(
        &fs::read_to_string(
            args.export_dir
                .join("org_2_Ops_Org/raw/export-metadata.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        org_one_metadata["org"],
        Value::String("Main Org".to_string())
    );
    assert_eq!(org_one_metadata["orgId"], Value::String("1".to_string()));
    assert_eq!(
        org_one_metadata["permissionsFile"],
        Value::String("permissions.json".to_string())
    );
    assert_eq!(
        org_two_metadata["org"],
        Value::String("Ops Org".to_string())
    );
    assert_eq!(org_two_metadata["orgId"], Value::String("2".to_string()));
    assert_eq!(
        org_two_metadata["permissionsFile"],
        Value::String("permissions.json".to_string())
    );
    assert_eq!(aggregate_root_index["items"].as_array().unwrap().len(), 2);
    assert!(aggregate_root_index["variants"]["raw"].is_null());
    assert!(aggregate_root_index["variants"]["provisioning"].is_null());
    assert_eq!(
        aggregate_root_index["items"][0]["raw_path"],
        Value::String(
            args.export_dir
                .join("org_1_Main_Org/raw/Infra/CPU__abc.json")
                .display()
                .to_string()
        )
    );
    assert_eq!(
        aggregate_root_index["items"][1]["raw_path"],
        Value::String(
            args.export_dir
                .join("org_2_Ops_Org/raw/Ops/Logs__xyz.json")
                .display()
                .to_string()
        )
    );
    assert_eq!(
        aggregate_root_metadata["variant"],
        Value::String("root".to_string())
    );
    assert_eq!(
        aggregate_root_metadata["indexFile"],
        Value::String("index.json".to_string())
    );
    assert_eq!(aggregate_root_metadata["orgCount"], Value::Number(2.into()));
    assert_eq!(aggregate_root_metadata["orgs"].as_array().unwrap().len(), 2);
    let org_entries = aggregate_root_metadata["orgs"].as_array().unwrap();
    let org_one_entry = org_entries
        .iter()
        .find(|entry| entry["orgId"] == Value::String("1".to_string()))
        .unwrap();
    let org_two_entry = org_entries
        .iter()
        .find(|entry| entry["orgId"] == Value::String("2".to_string()))
        .unwrap();
    assert_eq!(
        org_one_entry["usedDatasourceCount"],
        Value::Number(1.into())
    );
    assert_eq!(
        org_one_entry["exportDir"],
        Value::String(args.export_dir.join("org_1_Main_Org").display().to_string())
    );
    assert_eq!(
        org_one_entry["usedDatasources"][0]["uid"],
        Value::String("prom-main".to_string())
    );
    assert_eq!(
        org_two_entry["usedDatasourceCount"],
        Value::Number(1.into())
    );
    assert_eq!(
        org_two_entry["exportDir"],
        Value::String(args.export_dir.join("org_2_Ops_Org").display().to_string())
    );
    assert_eq!(
        org_two_entry["usedDatasources"][0]["uid"],
        Value::String("logs-main".to_string())
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, _, _)| path == "/api/orgs")
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params, _)| path == "/api/search"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "1"))
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params, _)| path == "/api/search"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "2"))
            .count(),
        1
    );
}

#[test]
fn export_dashboards_with_dry_run_keeps_output_dir_empty() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: true,
        provisioning_provider_name: "grafana-utils-dashboards".to_string(),
        provisioning_provider_org_id: None,
        provisioning_provider_path: None,
        provisioning_provider_disable_deletion: false,
        provisioning_provider_allow_ui_updates: false,
        provisioning_provider_update_interval_seconds: 30,
        dry_run: true,
        progress: false,
        verbose: false,
    };

    let count = export_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/org" => Ok(Some(json!({"id": 1, "name": "Main Org."}))),
            "/api/search" => Ok(Some(
                json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }]),
            )),
            "/api/datasources" => Ok(Some(json!([
                {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus", "url": "http://prometheus:9090", "access": "proxy", "isDefault": true}
            ]))),
            "/api/dashboards/uid/abc" => Ok(Some(
                json!({"dashboard": {"id": 7, "uid": "abc", "title": "CPU"}}),
            )),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(!args.export_dir.exists());
}

#[test]
fn export_dashboards_writes_provisioning_artifacts_in_separate_lane() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: true,
        overwrite: true,
        without_dashboard_raw: true,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: false,
        provisioning_provider_name: "grafana-utils-dashboards".to_string(),
        provisioning_provider_org_id: None,
        provisioning_provider_path: None,
        provisioning_provider_disable_deletion: false,
        provisioning_provider_allow_ui_updates: false,
        provisioning_provider_update_interval_seconds: 30,
        dry_run: false,
        progress: false,
        verbose: false,
    };

    let count = export_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/org" => Ok(Some(json!({"id": 7, "name": "Platform Org"}))),
            "/api/search" => Ok(Some(
                json!([{ "uid": "cpu-main", "title": "CPU", "folderTitle": "Infra" }]),
            )),
            "/api/datasources" => Ok(Some(json!([
                {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus", "url": "http://prometheus:9090", "access": "proxy", "isDefault": true}
            ]))),
            "/api/dashboards/uid/cpu-main" => Ok(Some(
                json!({"dashboard": {"id": 7, "uid": "cpu-main", "title": "CPU", "panels": []}}),
            )),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(args
        .export_dir
        .join("provisioning/dashboards/Infra/CPU__cpu-main.json")
        .is_file());
    assert!(args.export_dir.join("provisioning/index.json").is_file());
    assert!(args
        .export_dir
        .join("provisioning/export-metadata.json")
        .is_file());
    assert!(args
        .export_dir
        .join("provisioning/provisioning/dashboards.yaml")
        .is_file());

    let root_index: Value =
        serde_json::from_str(&fs::read_to_string(args.export_dir.join("index.json")).unwrap())
            .unwrap();
    assert_eq!(
        root_index["variants"]["provisioning"],
        Value::String(
            args.export_dir
                .join("provisioning/index.json")
                .display()
                .to_string()
        )
    );
    assert_eq!(
        root_index["items"][0]["provisioning_path"],
        Value::String(
            args.export_dir
                .join("provisioning/dashboards/Infra/CPU__cpu-main.json")
                .display()
                .to_string()
        )
    );

    let metadata: Value = serde_json::from_str(
        &fs::read_to_string(args.export_dir.join("provisioning/export-metadata.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        metadata["variant"],
        Value::String("provisioning".to_string())
    );
    assert_eq!(
        metadata["format"],
        Value::String("grafana-file-provisioning-dashboard".to_string())
    );

    let provider_yaml = fs::read_to_string(
        args.export_dir
            .join("provisioning/provisioning/dashboards.yaml"),
    )
    .unwrap();
    assert!(provider_yaml.contains("apiVersion: 1"));
    assert!(provider_yaml.contains("providers:"));
    assert!(provider_yaml.contains("orgId: 7"));
    assert!(provider_yaml.contains("type: file"));
    assert!(provider_yaml.contains("foldersFromFilesStructure: true"));
    let expected_dashboard_path = fs::canonicalize(args.export_dir.join("provisioning/dashboards"))
        .unwrap()
        .display()
        .to_string();
    assert!(provider_yaml.contains(&format!("path: {expected_dashboard_path}")));
    assert!(!provider_yaml.contains("REPLACE_WITH_PROVISIONING_DASHBOARD_PATH"));
}

#[test]
fn export_dashboards_writes_custom_provisioning_provider_settings() {
    let temp = tempdir().unwrap();
    let custom_provider_path = temp.path().join("provider-target");
    fs::create_dir_all(&custom_provider_path).unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: true,
        overwrite: true,
        without_dashboard_raw: true,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: false,
        provisioning_provider_name: "grafana-utils-prod".to_string(),
        provisioning_provider_org_id: Some(42),
        provisioning_provider_path: Some(custom_provider_path.clone()),
        provisioning_provider_disable_deletion: true,
        provisioning_provider_allow_ui_updates: true,
        provisioning_provider_update_interval_seconds: 120,
        dry_run: false,
        progress: false,
        verbose: false,
    };

    let count = export_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/org" => Ok(Some(json!({"id": 7, "name": "Platform Org"}))),
            "/api/search" => Ok(Some(
                json!([{ "uid": "cpu-main", "title": "CPU", "folderTitle": "Infra" }]),
            )),
            "/api/datasources" => Ok(Some(json!([
                {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus", "url": "http://prometheus:9090", "access": "proxy", "isDefault": true}
            ]))),
            "/api/dashboards/uid/cpu-main" => Ok(Some(
                json!({"dashboard": {"id": 7, "uid": "cpu-main", "title": "CPU", "panels": []}}),
            )),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    let provider_yaml = fs::read_to_string(
        args.export_dir
            .join("provisioning/provisioning/dashboards.yaml"),
    )
    .unwrap();
    assert!(provider_yaml.contains("name: grafana-utils-prod"));
    assert!(provider_yaml.contains("orgId: 42"));
    assert!(provider_yaml.contains("disableDeletion: true"));
    assert!(provider_yaml.contains("allowUiUpdates: true"));
    assert!(provider_yaml.contains("updateIntervalSeconds: 120"));
    assert!(provider_yaml.contains("foldersFromFilesStructure: true"));
    let expected_provider_path = fs::canonicalize(&custom_provider_path)
        .unwrap()
        .display()
        .to_string();
    assert!(provider_yaml.contains(&format!("path: {expected_provider_path}")));
}
