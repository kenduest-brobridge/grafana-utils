//! Dashboard domain test suite.
//! Covers parser surfaces, formatter/output contracts, and export/import/inspect/list/diff
//! behavior with in-memory/mocked request fixtures.
#![allow(unused_imports)]

use super::test_support;
use super::{export_dashboards_with_request, make_common_args, ExportArgs};
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;

fn make_history_only_export_args(
    output_dir: std::path::PathBuf,
    org_id: Option<i64>,
    all_orgs: bool,
    overwrite: bool,
) -> ExportArgs {
    ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        output_dir,
        page_size: 500,
        org_id,
        all_orgs,
        flat: false,
        overwrite,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: true,
        include_history: true,
        provisioning_provider_name: "grafana-utils-dashboards".to_string(),
        provisioning_provider_org_id: None,
        provisioning_provider_path: None,
        provisioning_provider_disable_deletion: false,
        provisioning_provider_allow_ui_updates: false,
        provisioning_provider_update_interval_seconds: 30,
        dry_run: false,
        progress: false,
        verbose: false,
    }
}

#[test]
fn export_dashboards_with_request_all_orgs_aggregates_results() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        output_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: true,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: true,
        include_history: false,
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
        .output_dir
        .join("org_1_Main_Org/raw/Infra/CPU__abc.json")
        .is_file());
    assert!(args
        .output_dir
        .join("org_1_Main_Org/raw/index.json")
        .is_file());
    assert!(args
        .output_dir
        .join("org_1_Main_Org/raw/export-metadata.json")
        .is_file());
    assert!(args
        .output_dir
        .join("org_1_Main_Org/raw/folders.json")
        .is_file());
    assert!(args
        .output_dir
        .join("org_1_Main_Org/raw/datasources.json")
        .is_file());
    assert!(args
        .output_dir
        .join("org_1_Main_Org/raw/permissions.json")
        .is_file());
    assert!(args
        .output_dir
        .join("org_2_Ops_Org/raw/Ops/Logs__xyz.json")
        .is_file());
    assert!(args
        .output_dir
        .join("org_2_Ops_Org/raw/index.json")
        .is_file());
    assert!(args
        .output_dir
        .join("org_2_Ops_Org/raw/permissions.json")
        .is_file());
    let aggregate_root_index: Value =
        serde_json::from_str(&fs::read_to_string(args.output_dir.join("index.json")).unwrap())
            .unwrap();
    let aggregate_root_metadata: Value = serde_json::from_str(
        &fs::read_to_string(args.output_dir.join("export-metadata.json")).unwrap(),
    )
    .unwrap();
    let org_one_metadata: Value = serde_json::from_str(
        &fs::read_to_string(
            args.output_dir
                .join("org_1_Main_Org/raw/export-metadata.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let org_two_metadata: Value = serde_json::from_str(
        &fs::read_to_string(
            args.output_dir
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
            args.output_dir
                .join("org_1_Main_Org/raw/Infra/CPU__abc.json")
                .display()
                .to_string()
        )
    );
    assert_eq!(
        aggregate_root_index["items"][1]["raw_path"],
        Value::String(
            args.output_dir
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
        Value::String(args.output_dir.join("org_1_Main_Org").display().to_string())
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
        Value::String(args.output_dir.join("org_2_Ops_Org").display().to_string())
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
fn export_dashboards_with_request_include_history_writes_scope_history_artifacts() {
    let current_temp = tempdir().unwrap();
    let current_args =
        make_history_only_export_args(current_temp.path().join("current"), None, false, true);

    let current_count = export_dashboards_with_request(
        |_method, path, params, _payload| match path {
            "/api/org" => Ok(Some(json!({"id": 1, "name": "Main Org"}))),
            "/api/search" => Ok(Some(json!([
                { "uid": "cpu-main", "title": "CPU Main", "folderTitle": "General" }
            ]))),
            "/api/datasources" => Ok(Some(json!([]))),
            "/api/folders/general" => Ok(Some(json!({"uid": "general", "title": "General"}))),
            "/api/dashboards/uid/cpu-main" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "cpu-main", "title": "CPU Main", "version": 21}
            }))),
            "/api/dashboards/uid/cpu-main/versions" => {
                assert!(params
                    .iter()
                    .any(|(key, value)| key == "limit" && value == "20"));
                Ok(Some(json!([
                    {
                        "version": 21,
                        "created": "2026-04-02T12:00:00Z",
                        "createdBy": "ops",
                        "message": "Tune thresholds"
                    }
                ])))
            }
            "/api/dashboards/uid/cpu-main/versions/21" => Ok(Some(json!({
                "data": {"uid": "cpu-main", "title": "CPU Main", "version": 21}
            }))),
            "/api/dashboards/uid/cpu-main/permissions" => Ok(Some(json!([]))),
            "/api/folders/general/permissions" => Ok(Some(json!([]))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &current_args,
    )
    .unwrap();

    assert_eq!(current_count, 1);
    let current_history = current_temp
        .path()
        .join("current/history/cpu-main.history.json");
    assert!(current_history.is_file());
    let current_document: Value =
        serde_json::from_str(&fs::read_to_string(&current_history).unwrap()).unwrap();
    assert_eq!(
        current_document["kind"],
        Value::String("grafana-util-dashboard-history-export".to_string())
    );
    assert_eq!(
        current_document["dashboardUid"],
        Value::String("cpu-main".to_string())
    );
    assert_eq!(current_document["versionCount"], Value::Number(1.into()));

    let org_temp = tempdir().unwrap();
    let org_args = make_history_only_export_args(org_temp.path().join("org"), Some(2), false, true);

    let org_count = export_dashboards_with_request(
        |_method, path, params, _payload| match path {
            "/api/org" => {
                assert!(params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "2"));
                Ok(Some(json!({"id": 2, "name": "Ops Org"})))
            }
            "/api/search" => {
                assert!(params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "2"));
                Ok(Some(json!([
                    { "uid": "ops-main", "title": "Ops Main", "folderTitle": "Ops" }
                ])))
            }
            "/api/datasources" => Ok(Some(json!([]))),
            "/api/folders/ops" => Ok(Some(json!({"uid": "ops", "title": "Ops"}))),
            "/api/dashboards/uid/ops-main" => Ok(Some(json!({
                "dashboard": {"id": 8, "uid": "ops-main", "title": "Ops Main", "version": 4}
            }))),
            "/api/dashboards/uid/ops-main/versions" => Ok(Some(json!([
                {
                    "version": 4,
                    "created": "2026-04-03T12:00:00Z",
                    "createdBy": "ops",
                    "message": "Record ops baseline"
                }
            ]))),
            "/api/dashboards/uid/ops-main/versions/4" => Ok(Some(json!({
                "data": {"uid": "ops-main", "title": "Ops Main", "version": 4}
            }))),
            "/api/dashboards/uid/ops-main/permissions" => Ok(Some(json!([]))),
            "/api/folders/ops/permissions" => Ok(Some(json!([]))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &org_args,
    )
    .unwrap();

    assert_eq!(org_count, 1);
    let org_history = org_temp.path().join("org/history/ops-main.history.json");
    assert!(org_history.is_file());
    let org_document: Value =
        serde_json::from_str(&fs::read_to_string(&org_history).unwrap()).unwrap();
    assert_eq!(
        org_document["kind"],
        Value::String("grafana-util-dashboard-history-export".to_string())
    );
    assert_eq!(
        org_document["dashboardUid"],
        Value::String("ops-main".to_string())
    );
}

#[test]
fn export_dashboards_with_request_include_history_writes_all_org_history_artifacts() {
    let temp = tempdir().unwrap();
    let args = make_history_only_export_args(temp.path().join("all-orgs"), None, true, true);

    let count = export_dashboards_with_request(
        |method, path, params, _payload| {
            let scoped_org = params
                .iter()
                .find(|(key, _)| key == "orgId")
                .map(|(_, value)| value.as_str());
            match (method.clone(), path, scoped_org) {
                (reqwest::Method::GET, "/api/orgs", _) => Ok(Some(json!([
                    {"id": 1, "name": "Main Org"},
                    {"id": 2, "name": "Ops Org"}
                ]))),
                (reqwest::Method::GET, "/api/org", Some("1")) => {
                    Ok(Some(json!({"id": 1, "name": "Main Org"})))
                }
                (reqwest::Method::GET, "/api/org", Some("2")) => {
                    Ok(Some(json!({"id": 2, "name": "Ops Org"})))
                }
                (reqwest::Method::GET, "/api/search", Some("1")) => Ok(Some(json!([
                    { "uid": "cpu-main", "title": "CPU Main", "folderTitle": "General" }
                ]))),
                (reqwest::Method::GET, "/api/search", Some("2")) => Ok(Some(json!([
                    { "uid": "ops-main", "title": "Ops Main", "folderTitle": "Ops" }
                ]))),
                (reqwest::Method::GET, "/api/datasources", Some("1")) => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/datasources", Some("2")) => Ok(Some(json!([]))),
                (reqwest::Method::GET, "/api/folders/general", Some("1")) => {
                    Ok(Some(json!({"uid": "general", "title": "General"})))
                }
                (reqwest::Method::GET, "/api/folders/ops", Some("2")) => {
                    Ok(Some(json!({"uid": "ops", "title": "Ops"})))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/cpu-main", Some("1")) => {
                    Ok(Some(json!({"dashboard": {"id": 7, "uid": "cpu-main", "title": "CPU Main", "version": 21}})))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/ops-main", Some("2")) => {
                    Ok(Some(json!({"dashboard": {"id": 8, "uid": "ops-main", "title": "Ops Main", "version": 4}})))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/cpu-main/versions", Some("1")) => {
                    Ok(Some(json!([
                        {
                            "version": 21,
                            "created": "2026-04-02T12:00:00Z",
                            "createdBy": "ops",
                            "message": "Tune thresholds"
                        }
                    ])))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/ops-main/versions", Some("2")) => {
                    Ok(Some(json!([
                        {
                            "version": 4,
                            "created": "2026-04-03T12:00:00Z",
                            "createdBy": "ops",
                            "message": "Record ops baseline"
                        }
                    ])))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/cpu-main/versions/21", Some("1")) => {
                    Ok(Some(json!({
                        "data": {"uid": "cpu-main", "title": "CPU Main", "version": 21}
                    })))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/ops-main/versions/4", Some("2")) => {
                    Ok(Some(json!({
                        "data": {"uid": "ops-main", "title": "Ops Main", "version": 4}
                    })))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/cpu-main/permissions", Some("1")) => {
                    Ok(Some(json!([])))
                }
                (reqwest::Method::GET, "/api/dashboards/uid/ops-main/permissions", Some("2")) => {
                    Ok(Some(json!([])))
                }
                (reqwest::Method::GET, "/api/folders/general/permissions", Some("1")) => {
                    Ok(Some(json!([])))
                }
                (reqwest::Method::GET, "/api/folders/ops/permissions", Some("2")) => {
                    Ok(Some(json!([])))
                }
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 2);
    assert!(temp
        .path()
        .join("all-orgs/org_1_Main_Org/history/cpu-main.history.json")
        .is_file());
    assert!(temp
        .path()
        .join("all-orgs/org_2_Ops_Org/history/ops-main.history.json")
        .is_file());
}

#[test]
fn export_dashboards_with_request_include_history_respects_overwrite() {
    let temp = tempdir().unwrap();
    let output_dir = temp.path().join("current");
    fs::create_dir_all(output_dir.join("history")).unwrap();
    fs::write(
        output_dir.join("history/cpu-main.history.json"),
        "{\"kind\":\"existing\"}\n",
    )
    .unwrap();
    let args = make_history_only_export_args(output_dir.clone(), None, false, false);

    let error = export_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/org" => Ok(Some(json!({"id": 1, "name": "Main Org"}))),
            "/api/search" => Ok(Some(json!([
                { "uid": "cpu-main", "title": "CPU Main", "folderTitle": "General" }
            ]))),
            "/api/datasources" => Ok(Some(json!([]))),
            "/api/folders/general" => Ok(Some(json!({"uid": "general", "title": "General"}))),
            "/api/dashboards/uid/cpu-main" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "cpu-main", "title": "CPU Main", "version": 21}
            }))),
            "/api/dashboards/uid/cpu-main/versions" => Ok(Some(json!([
                {
                    "version": 21,
                    "created": "2026-04-02T12:00:00Z",
                    "createdBy": "ops",
                    "message": "Tune thresholds"
                }
            ]))),
            "/api/dashboards/uid/cpu-main/versions/21" => Ok(Some(json!({
                "data": {"uid": "cpu-main", "title": "CPU Main", "version": 21}
            }))),
            "/api/dashboards/uid/cpu-main/permissions" => Ok(Some(json!([]))),
            "/api/folders/general/permissions" => Ok(Some(json!([]))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("Refusing to overwrite existing file"));
}

#[test]
fn export_dashboards_with_dry_run_keeps_output_dir_empty() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        output_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: true,
        include_history: false,
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
    assert!(!args.output_dir.exists());
}

#[test]
fn export_dashboards_writes_provisioning_artifacts_in_separate_lane() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        output_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: true,
        overwrite: true,
        without_dashboard_raw: true,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: false,
        include_history: false,
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
        .output_dir
        .join("provisioning/dashboards/Infra/CPU__cpu-main.json")
        .is_file());
    assert!(args.output_dir.join("provisioning/index.json").is_file());
    assert!(args
        .output_dir
        .join("provisioning/export-metadata.json")
        .is_file());
    assert!(args
        .output_dir
        .join("provisioning/provisioning/dashboards.yaml")
        .is_file());

    let root_index: Value =
        serde_json::from_str(&fs::read_to_string(args.output_dir.join("index.json")).unwrap())
            .unwrap();
    assert_eq!(
        root_index["variants"]["provisioning"],
        Value::String(
            args.output_dir
                .join("provisioning/index.json")
                .display()
                .to_string()
        )
    );
    assert_eq!(
        root_index["items"][0]["provisioning_path"],
        Value::String(
            args.output_dir
                .join("provisioning/dashboards/Infra/CPU__cpu-main.json")
                .display()
                .to_string()
        )
    );

    let metadata: Value = serde_json::from_str(
        &fs::read_to_string(args.output_dir.join("provisioning/export-metadata.json")).unwrap(),
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
        args.output_dir
            .join("provisioning/provisioning/dashboards.yaml"),
    )
    .unwrap();
    assert!(provider_yaml.contains("apiVersion: 1"));
    assert!(provider_yaml.contains("providers:"));
    assert!(provider_yaml.contains("orgId: 7"));
    assert!(provider_yaml.contains("type: file"));
    assert!(provider_yaml.contains("foldersFromFilesStructure: true"));
    let expected_dashboard_path = fs::canonicalize(args.output_dir.join("provisioning/dashboards"))
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
        output_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: true,
        overwrite: true,
        without_dashboard_raw: true,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: false,
        include_history: false,
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
        args.output_dir
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
