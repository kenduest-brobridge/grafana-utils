//! Task-first `grafana-util workspace` smoke regressions.
//! Exercises the repo-local scan/test/preview/apply lane from one staged workspace.

use super::{ChangeOutputArgs, ChangePreviewArgs};
use crate::access::{
    ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_KIND_TEAMS,
    ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_ORG_EXPORT_FILENAME,
    ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME, ACCESS_TEAM_EXPORT_FILENAME,
    ACCESS_USER_EXPORT_FILENAME,
};
use crate::common::tool_version;
use crate::overview::{execute_overview, OverviewArgs, OverviewOutputFormat};
use crate::project_status_command::{
    execute_project_status_staged, ProjectStatusOutputFormat, ProjectStatusStagedArgs,
};
use crate::sync::{
    discover_change_staged_inputs, run_sync_cli, SyncApplyArgs, SyncCliArgs, SyncGroupCommand,
    SyncOutputFormat,
};
use clap::Parser;
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_dashboard_raw_fixture(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("folders.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "general",
                "title": "General",
                "parentUid": null,
                "path": "General",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            },
            "meta": {
                "folderUid": "general"
            }
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_dashboard_provisioning_fixture(root: &Path) {
    fs::create_dir_all(root.join("dashboards").join("team")).unwrap();
    fs::write(
        root.join("folders.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "team",
                "title": "Team",
                "parentUid": null,
                "path": "Team",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "provisioning",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-file-provisioning-dashboard"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("dashboards").join("team").join("cpu-main.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_datasource_provisioning_fixture(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("datasources.yaml"),
        r#"apiVersion: 1
datasources:
  - name: Prometheus Main
    type: prometheus
    uid: prom-main
    access: proxy
    url: http://prometheus:9090
    isDefault: true
"#,
    )
    .unwrap();
}

fn write_alert_export_fixture(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::create_dir_all(root.join("rules").join("general").join("cpu-alerts")).unwrap();
    fs::write(
        root.join("index.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "apiVersion": 1,
            "kind": "grafana-util-alert-export-index",
            "rules": [{
                "kind": "grafana-alert-rule",
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "cpu-alerts",
                "path": "rules/general/cpu-alerts/CPU_High__cpu-high.json"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("rules")
            .join("general")
            .join("cpu-alerts")
            .join("CPU_High__cpu-high.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "toolVersion": tool_version(),
            "apiVersion": 1,
            "kind": "grafana-alert-rule",
            "metadata": {
                "uid": "cpu-high",
                "title": "CPU High"
            },
            "spec": {
                "uid": "cpu-high",
                "title": "CPU High",
                "folderUID": "general",
                "ruleGroup": "cpu-alerts",
                "condition": "A",
                "data": [{
                    "refId": "A",
                    "datasourceUid": "prom-main",
                    "model": {
                        "expr": "up",
                        "refId": "A"
                    }
                }]
            }
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_access_export_fixture(root: &Path, payload_filename: &str, kind: &str) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join(payload_filename),
        serde_json::to_string_pretty(&json!({
            "kind": kind,
            "version": 1,
            "records": [{
                "uid": "sample",
                "name": "sample"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join(ACCESS_EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": kind,
            "version": 1,
            "recordCount": 1
        }))
        .unwrap(),
    )
    .unwrap();
}

fn task_first_workspace_cli_args(
    command: &str,
    workspace: &Path,
    live_file: Option<&Path>,
    preview_file: Option<&Path>,
) -> SyncCliArgs {
    let mut argv = vec!["grafana-util", command, "--output-format", "json"];
    if command != "apply" {
        argv.push(workspace.to_str().unwrap());
    }
    if let Some(live_file) = live_file {
        argv.extend(["--live-file", live_file.to_str().unwrap()]);
    }
    if let Some(preview_file) = preview_file {
        argv.extend(["--preview-file", preview_file.to_str().unwrap()]);
    }
    if command == "preview" {
        argv.extend(["--trace-id", "workspace-task-first-smoke"]);
    }
    if command == "apply" {
        argv.push("--approve");
    }
    SyncCliArgs::parse_from(argv)
}

#[test]
fn task_first_workspace_lane_smoke_runs_from_repo_local_workspace() {
    let temp = tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let dashboards_raw = workspace.join("dashboards").join("raw");
    write_dashboard_raw_fixture(&dashboards_raw);

    let live_file = workspace.join("live.json");
    fs::write(&live_file, "[]").unwrap();
    let preview_file = workspace.join("workspace-preview.json");

    let inspect_args = task_first_workspace_cli_args("scan", &workspace, None, None);
    match inspect_args.command {
        SyncGroupCommand::Inspect(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert!(run_sync_cli(SyncGroupCommand::Inspect(inner)).is_ok());
        }
        _ => panic!("expected scan"),
    }

    let check_args = task_first_workspace_cli_args("test", &workspace, None, None);
    match check_args.command {
        SyncGroupCommand::Check(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert!(run_sync_cli(SyncGroupCommand::Check(inner)).is_ok());
        }
        _ => panic!("expected test"),
    }

    let preview_args = task_first_workspace_cli_args("preview", &workspace, Some(&live_file), None);
    match preview_args.command {
        SyncGroupCommand::Preview(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert_eq!(inner.live_file, Some(live_file.clone()));
            assert_eq!(
                inner.trace_id.as_deref(),
                Some("workspace-task-first-smoke")
            );
            assert!(run_sync_cli(SyncGroupCommand::Preview(ChangePreviewArgs {
                output: ChangeOutputArgs {
                    output_file: Some(preview_file.clone()),
                    ..inner.output.clone()
                },
                ..inner
            }))
            .is_ok());
        }
        _ => panic!("expected preview"),
    }

    let preview_raw = fs::read_to_string(&preview_file).unwrap();
    assert!(!preview_raw.contains('\u{1b}'));
    assert!(preview_raw.ends_with('\n'));
    let preview_document: serde_json::Value = serde_json::from_str(&preview_raw).unwrap();
    assert_eq!(preview_document["kind"], json!("grafana-utils-sync-plan"));
    assert_eq!(
        preview_document["traceId"],
        json!("workspace-task-first-smoke")
    );
    assert_eq!(preview_document["reviewed"], json!(false));
    assert_eq!(
        preview_document["discovery"]["workspaceRoot"],
        json!(workspace.display().to_string())
    );
    assert!(preview_document["actions"].as_array().is_some());
    assert!(preview_document["operations"].as_array().is_some());
    assert!(preview_document["domains"].as_array().is_some());
    assert!(preview_document["blockedReasons"].as_array().is_some());
    let domains = preview_document["domains"].as_array().unwrap();
    assert!(domains
        .iter()
        .any(|domain| domain["id"] == json!("dashboard")));
    assert!(domains
        .iter()
        .any(|domain| domain["id"] == json!("datasource")));
    assert!(domains.iter().any(|domain| domain["id"] == json!("alert")));
    let first_action = preview_document["actions"]
        .as_array()
        .unwrap()
        .first()
        .unwrap();
    assert!(first_action["actionId"].as_str().is_some());
    assert!(first_action["domain"].as_str().is_some());
    assert!(first_action["resourceKind"].as_str().is_some());

    let apply_args = task_first_workspace_cli_args("apply", &workspace, None, Some(&preview_file));
    match apply_args.command {
        SyncGroupCommand::Apply(inner) => {
            assert_eq!(inner.plan_file, Some(preview_file.clone()));
            assert!(run_sync_cli(SyncGroupCommand::Apply(SyncApplyArgs {
                output_format: SyncOutputFormat::Json,
                ..inner
            }))
            .is_ok());
        }
        _ => panic!("expected apply"),
    }
}

#[test]
fn task_first_workspace_lane_smoke_runs_from_git_sync_workspace_root() {
    let temp = tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join(".git")).unwrap();
    let dashboards_raw = workspace.join("dashboards").join("git-sync").join("raw");
    write_dashboard_raw_fixture(&dashboards_raw);
    let live_file = workspace.join("live.json");
    fs::write(&live_file, "[]").unwrap();

    let inspect_args = task_first_workspace_cli_args("scan", &workspace, None, None);
    match inspect_args.command {
        SyncGroupCommand::Inspect(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert!(run_sync_cli(SyncGroupCommand::Inspect(inner)).is_ok());
        }
        _ => panic!("expected scan"),
    }

    let check_args = task_first_workspace_cli_args("test", &workspace, None, None);
    match check_args.command {
        SyncGroupCommand::Check(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert!(run_sync_cli(SyncGroupCommand::Check(inner)).is_ok());
        }
        _ => panic!("expected test"),
    }

    let preview_args = task_first_workspace_cli_args("preview", &workspace, Some(&live_file), None);
    match preview_args.command {
        SyncGroupCommand::Preview(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert_eq!(inner.live_file, Some(live_file.clone()));
            assert!(run_sync_cli(SyncGroupCommand::Preview(ChangePreviewArgs {
                output: ChangeOutputArgs {
                    output_file: None,
                    ..inner.output.clone()
                },
                ..inner
            }))
            .is_ok());
        }
        _ => panic!("expected preview"),
    }
}

#[test]
fn task_first_workspace_lane_smoke_runs_from_git_sync_mixed_workspace_root() {
    let temp = tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join(".git")).unwrap();
    let dashboards_raw = workspace.join("dashboards").join("git-sync").join("raw");
    let dashboards_provisioning = workspace
        .join("dashboards")
        .join("git-sync")
        .join("provisioning");
    write_dashboard_raw_fixture(&dashboards_raw);
    write_dashboard_provisioning_fixture(&dashboards_provisioning);
    write_datasource_provisioning_fixture(&workspace.join("datasources").join("provisioning"));
    write_alert_export_fixture(&workspace.join("alerts").join("raw"));
    write_access_export_fixture(
        &workspace.join("access-users"),
        ACCESS_USER_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_USERS,
    );
    write_access_export_fixture(
        &workspace.join("access-teams"),
        ACCESS_TEAM_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_TEAMS,
    );
    write_access_export_fixture(
        &workspace.join("access-orgs"),
        ACCESS_ORG_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_ORGS,
    );
    write_access_export_fixture(
        &workspace.join("access-service-accounts"),
        ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
    );
    let live_file = workspace.join("live.json");
    fs::write(&live_file, "[]").unwrap();
    let preview_file = workspace.join("workspace-preview.json");

    let inspect_args = task_first_workspace_cli_args("scan", &workspace, None, None);
    match inspect_args.command {
        SyncGroupCommand::Inspect(inner) => {
            let result = run_sync_cli(SyncGroupCommand::Inspect(inner));
            assert!(result.is_ok(), "{result:?}");
        }
        _ => panic!("expected scan"),
    }

    let check_args = task_first_workspace_cli_args("test", &workspace, None, None);
    match check_args.command {
        SyncGroupCommand::Check(inner) => {
            let result = run_sync_cli(SyncGroupCommand::Check(inner));
            assert!(result.is_ok(), "{result:?}");
        }
        _ => panic!("expected test"),
    }

    let preview_args = task_first_workspace_cli_args("preview", &workspace, Some(&live_file), None);
    match preview_args.command {
        SyncGroupCommand::Preview(inner) => {
            let result = run_sync_cli(SyncGroupCommand::Preview(ChangePreviewArgs {
                output: ChangeOutputArgs {
                    output_file: Some(preview_file.clone()),
                    ..inner.output.clone()
                },
                ..inner
            }));
            assert!(result.is_ok(), "{result:?}");
        }
        _ => panic!("expected preview"),
    }

    let discovered = discover_change_staged_inputs(Some(&workspace)).unwrap();
    let overview = execute_overview(&OverviewArgs {
        dashboard_export_dir: discovered.dashboard_export_dir.clone(),
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: discovered.datasource_provisioning_file.clone(),
        access_user_export_dir: discovered.access_user_export_dir.clone(),
        access_team_export_dir: discovered.access_team_export_dir.clone(),
        access_org_export_dir: discovered.access_org_export_dir.clone(),
        access_service_account_export_dir: discovered.access_service_account_export_dir.clone(),
        desired_file: discovered.desired_file.clone(),
        source_bundle: discovered.source_bundle.clone(),
        target_inventory: discovered.target_inventory.clone(),
        alert_export_dir: discovered.alert_export_dir.clone(),
        availability_file: discovered.availability_file.clone(),
        mapping_file: discovered.mapping_file.clone(),
        output_format: OverviewOutputFormat::Json,
    })
    .unwrap();
    assert_eq!(overview.summary.artifact_count, 7);
    assert_eq!(overview.summary.dashboard_export_count, 1);
    assert_eq!(overview.summary.datasource_export_count, 1);
    assert_eq!(overview.summary.alert_export_count, 1);
    assert_eq!(overview.summary.access_user_export_count, 1);
    assert_eq!(overview.summary.access_team_export_count, 1);
    assert_eq!(overview.summary.access_org_export_count, 1);
    assert_eq!(overview.summary.access_service_account_export_count, 1);
    assert!(overview
        .project_status
        .domains
        .iter()
        .any(|domain| domain.id == "dashboard"));
    assert!(overview
        .project_status
        .domains
        .iter()
        .any(|domain| domain.id == "datasource"));
    assert!(overview
        .project_status
        .domains
        .iter()
        .any(|domain| domain.id == "access"));
    assert!(overview
        .project_status
        .domains
        .iter()
        .any(|domain| domain.id == "alert"));

    let project_status = execute_project_status_staged(&ProjectStatusStagedArgs {
        dashboard_export_dir: discovered.dashboard_export_dir.clone(),
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: discovered.datasource_provisioning_file.clone(),
        access_user_export_dir: discovered.access_user_export_dir.clone(),
        access_team_export_dir: discovered.access_team_export_dir.clone(),
        access_org_export_dir: discovered.access_org_export_dir.clone(),
        access_service_account_export_dir: discovered.access_service_account_export_dir.clone(),
        desired_file: discovered.desired_file.clone(),
        source_bundle: discovered.source_bundle.clone(),
        target_inventory: discovered.target_inventory.clone(),
        alert_export_dir: discovered.alert_export_dir.clone(),
        availability_file: discovered.availability_file.clone(),
        mapping_file: discovered.mapping_file.clone(),
        output_format: ProjectStatusOutputFormat::Json,
    })
    .unwrap();
    assert_eq!(project_status.scope, "staged-only");
    assert_eq!(project_status.domains.len(), 4);
    assert!(project_status
        .domains
        .iter()
        .any(|domain| domain.id == "dashboard"));
    assert!(project_status
        .domains
        .iter()
        .any(|domain| domain.id == "datasource"));
    assert!(project_status
        .domains
        .iter()
        .any(|domain| domain.id == "access"));
    assert!(project_status
        .domains
        .iter()
        .any(|domain| domain.id == "alert"));

    let preview_document: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&preview_file).unwrap()).unwrap();
    assert_eq!(preview_document["kind"], json!("grafana-utils-sync-plan"));
    assert_eq!(
        preview_document["discovery"]["workspaceRoot"],
        json!(workspace.display().to_string())
    );
    assert!(preview_document["operations"].as_array().is_some());
    assert!(preview_document["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|operation| operation["kind"] == json!("dashboard")));
    assert!(preview_document["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|operation| operation["kind"] == json!("datasource")));
    assert!(preview_document["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|operation| operation["kind"] == json!("alert")));
    assert_eq!(
        preview_document["alertAssessment"]["kind"],
        json!("grafana-utils-alert-sync-plan")
    );
}
