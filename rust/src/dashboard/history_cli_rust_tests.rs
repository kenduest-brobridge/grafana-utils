//! Dashboard history CLI contracts and export artifact tests.
#![allow(unused_imports)]

use super::history::{
    build_dashboard_history_export_document_with_request,
    build_dashboard_history_list_document_with_request, export_dashboard_history_with_request,
    run_dashboard_history_restore,
};
use super::test_support;
use super::{
    discover_dashboard_files, CommonCliArgs, HistoryExportArgs, HistoryListArgs,
    HistoryOutputFormat, HistoryRestoreArgs,
};
use reqwest::Method;
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;

fn make_history_common_args() -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: None,
        url: "http://127.0.0.1:3000".to_string(),
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

#[test]
fn dashboard_history_list_document_collects_recent_versions() {
    let document = build_dashboard_history_list_document_with_request(
        |method, path, params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/dashboards/uid/cpu-main/versions") => {
                assert_eq!(
                    params,
                    vec![("limit".to_string(), "5".to_string())].as_slice()
                );
                Ok(Some(json!([
                    {
                        "version": 19,
                        "created": "2026-04-01T10:00:00Z",
                        "createdBy": "ops",
                        "message": "Tune CPU panel"
                    },
                    {
                        "version": 18,
                        "created": "2026-03-30T09:00:00Z",
                        "createdBy": "sre",
                        "message": "Add datasource override"
                    }
                ])))
            }
            _ => Err(test_support::message(format!(
                "unexpected request {method} {path}"
            ))),
        },
        "cpu-main",
        5,
    )
    .unwrap();

    assert_eq!(
        document.kind,
        crate::dashboard::history::DASHBOARD_HISTORY_LIST_KIND
    );
    assert_eq!(document.dashboard_uid, "cpu-main");
    assert_eq!(document.version_count, 2);
    assert_eq!(document.versions[0].version, 19);
    assert_eq!(document.versions[1].created_by, "sre");
}

#[test]
fn dashboard_history_restore_requires_yes_without_dry_run() {
    let args = HistoryRestoreArgs {
        common: make_history_common_args(),
        dashboard_uid: "cpu-main".to_string(),
        version: 17,
        dry_run: false,
        output_format: HistoryOutputFormat::Text,
        message: None,
        yes: false,
    };

    let error = run_dashboard_history_restore(
        |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {"uid": "cpu-main", "title": "CPU Main", "version": 21},
                "meta": {"folderUid": "infra"}
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions/17") => Ok(Some(json!({
                "data": {"uid": "cpu-main", "title": "CPU Main"}
            }))),
            _ => Err(test_support::message(format!(
                "unexpected request {method} {path}"
            ))),
        },
        &args,
    )
    .unwrap_err();

    assert!(error.to_string().contains("requires --yes"));
}

#[test]
fn dashboard_history_export_writes_json_artifact_with_dashboard_payloads() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("cpu-main.history.json");
    let args = HistoryExportArgs {
        common: make_history_common_args(),
        dashboard_uid: "cpu-main".to_string(),
        output: output.clone(),
        limit: 2,
        overwrite: false,
    };

    export_dashboard_history_with_request(
        |method, path, params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {"uid": "cpu-main", "title": "CPU Main", "version": 21}
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions") => {
                assert_eq!(
                    params,
                    vec![("limit".to_string(), "2".to_string())].as_slice()
                );
                Ok(Some(json!([
                    {
                        "version": 21,
                        "created": "2026-04-02T12:00:00Z",
                        "createdBy": "ops",
                        "message": "Tune thresholds"
                    },
                    {
                        "version": 20,
                        "created": "2026-04-01T12:00:00Z",
                        "createdBy": "sre",
                        "message": "Add region variable"
                    }
                ])))
            }
            (Method::GET, "/api/dashboards/uid/cpu-main/versions/21") => Ok(Some(json!({
                "data": {"uid": "cpu-main", "title": "CPU Main", "version": 21}
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions/20") => Ok(Some(json!({
                "data": {"uid": "cpu-main", "title": "CPU Main", "version": 20}
            }))),
            _ => Err(test_support::message(format!(
                "unexpected request {method} {path}"
            ))),
        },
        &args,
    )
    .unwrap();

    let artifact: Value = serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        artifact["kind"],
        crate::dashboard::history::DASHBOARD_HISTORY_EXPORT_KIND
    );
    assert_eq!(artifact["dashboardUid"], "cpu-main");
    assert_eq!(artifact["versionCount"], 2);
    assert_eq!(artifact["versions"][0]["dashboard"]["title"], "CPU Main");
}

#[test]
fn dashboard_history_list_reads_single_local_artifact() {
    let temp = tempdir().unwrap();
    let artifact_path = temp.path().join("cpu-main.history.json");
    fs::write(
        &artifact_path,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-util-dashboard-history-export",
            "schemaVersion": 1,
            "toolVersion": "0.8.0-dev",
            "dashboardUid": "cpu-main",
            "currentVersion": 21,
            "currentTitle": "CPU Main",
            "versionCount": 2,
            "versions": [
                {
                    "version": 21,
                    "created": "2026-04-02T12:00:00Z",
                    "createdBy": "ops",
                    "message": "Tune thresholds",
                    "dashboard": {"uid": "cpu-main", "title": "CPU Main", "version": 21}
                },
                {
                    "version": 20,
                    "created": "2026-04-01T12:00:00Z",
                    "createdBy": "sre",
                    "message": "Add region variable",
                    "dashboard": {"uid": "cpu-main", "title": "CPU Main", "version": 20}
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let args = HistoryListArgs {
        common: make_history_common_args(),
        dashboard_uid: Some("cpu-main".to_string()),
        input: Some(artifact_path),
        import_dir: None,
        limit: 20,
        output_format: HistoryOutputFormat::Json,
    };

    super::history::run_dashboard_history_list(
        |_method, _path, _params, _payload| Err(test_support::message("should not call Grafana")),
        &args,
    )
    .unwrap();
}

#[test]
fn dashboard_history_list_rejects_mismatched_local_artifact_uid() {
    let temp = tempdir().unwrap();
    let artifact_path = temp.path().join("cpu-main.history.json");
    fs::write(
        &artifact_path,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-util-dashboard-history-export",
            "schemaVersion": 1,
            "toolVersion": "0.8.0-dev",
            "dashboardUid": "cpu-main",
            "currentVersion": 21,
            "currentTitle": "CPU Main",
            "versionCount": 0,
            "versions": []
        }))
        .unwrap(),
    )
    .unwrap();

    let args = HistoryListArgs {
        common: make_history_common_args(),
        dashboard_uid: Some("memory-main".to_string()),
        input: Some(artifact_path),
        import_dir: None,
        limit: 20,
        output_format: HistoryOutputFormat::Json,
    };

    let error = super::history::run_dashboard_history_list(
        |_method, _path, _params, _payload| Err(test_support::message("should not call Grafana")),
        &args,
    )
    .unwrap_err();

    assert!(error.to_string().contains("contains dashboard UID cpu-main instead of memory-main"));
}

#[test]
fn dashboard_history_list_reads_export_tree_inventory_without_uid_filter() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("dashboards");
    fs::create_dir_all(import_dir.join("all-orgs/org_1_Main_Org/history")).unwrap();
    fs::create_dir_all(import_dir.join("all-orgs/org_2_Ops_Org/history")).unwrap();
    fs::write(
        import_dir.join("all-orgs/org_1_Main_Org/history/cpu-main.history.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-util-dashboard-history-export",
            "schemaVersion": 1,
            "toolVersion": "0.8.0-dev",
            "dashboardUid": "cpu-main",
            "currentVersion": 21,
            "currentTitle": "CPU Main",
            "versionCount": 2,
            "versions": []
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        import_dir.join("all-orgs/org_2_Ops_Org/history/ops-main.history.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-util-dashboard-history-export",
            "schemaVersion": 1,
            "toolVersion": "0.8.0-dev",
            "dashboardUid": "ops-main",
            "currentVersion": 12,
            "currentTitle": "Ops Main",
            "versionCount": 3,
            "versions": []
        }))
        .unwrap(),
    )
    .unwrap();

    let args = HistoryListArgs {
        common: make_history_common_args(),
        dashboard_uid: None,
        input: None,
        import_dir: Some(import_dir),
        limit: 20,
        output_format: HistoryOutputFormat::Json,
    };

    super::history::run_dashboard_history_list(
        |_method, _path, _params, _payload| Err(test_support::message("should not call Grafana")),
        &args,
    )
    .unwrap();
}

#[test]
fn dashboard_history_list_rejects_ambiguous_uid_in_export_tree() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("dashboards");
    fs::create_dir_all(import_dir.join("all-orgs/org_1_Main_Org/history")).unwrap();
    fs::create_dir_all(import_dir.join("all-orgs/org_2_Ops_Org/history")).unwrap();
    for path in [
        import_dir.join("all-orgs/org_1_Main_Org/history/cpu-main.history.json"),
        import_dir.join("all-orgs/org_2_Ops_Org/history/cpu-main.history.json"),
    ] {
        fs::write(
            path,
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-util-dashboard-history-export",
                "schemaVersion": 1,
                "toolVersion": "0.8.0-dev",
                "dashboardUid": "cpu-main",
                "currentVersion": 21,
                "currentTitle": "CPU Main",
                "versionCount": 2,
                "versions": []
            }))
            .unwrap(),
        )
        .unwrap();
    }

    let args = HistoryListArgs {
        common: make_history_common_args(),
        dashboard_uid: Some("cpu-main".to_string()),
        input: None,
        import_dir: Some(import_dir),
        limit: 20,
        output_format: HistoryOutputFormat::Table,
    };

    let error = super::history::run_dashboard_history_list(
        |_method, _path, _params, _payload| Err(test_support::message("should not call Grafana")),
        &args,
    )
    .unwrap_err();

    assert!(error.to_string().contains("Multiple dashboard history artifacts for UID cpu-main"));
}

#[test]
fn discover_dashboard_files_ignores_history_export_artifacts() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw/history")).unwrap();
    fs::create_dir_all(temp.path().join("raw/general")).unwrap();
    fs::write(
        temp.path().join("raw/general/cpu-main.json"),
        serde_json::to_string_pretty(&json!({"uid": "cpu-main", "title": "CPU Main"})).unwrap(),
    )
    .unwrap();
    fs::write(
        temp.path().join("raw/history/cpu-main.history.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-util-dashboard-history-export",
            "schemaVersion": 1,
            "dashboardUid": "cpu-main",
            "versions": []
        }))
        .unwrap(),
    )
    .unwrap();

    let files = discover_dashboard_files(&temp.path().join("raw")).unwrap();
    assert_eq!(files, vec![temp.path().join("raw/general/cpu-main.json")]);
}
