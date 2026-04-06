//! Task-first `grafana-util change` smoke regressions.
//! Exercises the repo-local inspect/check/preview/apply lane from one staged workspace.

use super::{ChangeOutputArgs, ChangePreviewArgs};
use crate::sync::{run_sync_cli, SyncApplyArgs, SyncCliArgs, SyncGroupCommand, SyncOutputFormat};
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

fn task_first_change_cli_args(
    command: &str,
    workspace: &Path,
    live_file: Option<&Path>,
    preview_file: Option<&Path>,
) -> SyncCliArgs {
    let mut argv = vec!["grafana-util", command, "--output-format", "json"];
    if command != "apply" {
        argv.extend(["--workspace", workspace.to_str().unwrap()]);
    }
    if let Some(live_file) = live_file {
        argv.extend(["--live-file", live_file.to_str().unwrap()]);
    }
    if let Some(preview_file) = preview_file {
        argv.extend(["--preview-file", preview_file.to_str().unwrap()]);
    }
    if command == "preview" {
        argv.extend(["--trace-id", "change-task-first-smoke"]);
    }
    if command == "apply" {
        argv.push("--approve");
    }
    SyncCliArgs::parse_from(argv)
}

#[test]
fn task_first_change_lane_smoke_runs_from_repo_local_workspace() {
    let temp = tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let dashboards_raw = workspace.join("dashboards").join("raw");
    write_dashboard_raw_fixture(&dashboards_raw);

    let live_file = workspace.join("live.json");
    fs::write(&live_file, "[]").unwrap();
    let preview_file = workspace.join("change-preview.json");

    let inspect_args = task_first_change_cli_args("inspect", &workspace, None, None);
    match inspect_args.command {
        SyncGroupCommand::Inspect(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert!(run_sync_cli(SyncGroupCommand::Inspect(inner)).is_ok());
        }
        _ => panic!("expected inspect"),
    }

    let check_args = task_first_change_cli_args("check", &workspace, None, None);
    match check_args.command {
        SyncGroupCommand::Check(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert!(run_sync_cli(SyncGroupCommand::Check(inner)).is_ok());
        }
        _ => panic!("expected check"),
    }

    let preview_args = task_first_change_cli_args("preview", &workspace, Some(&live_file), None);
    match preview_args.command {
        SyncGroupCommand::Preview(inner) => {
            assert_eq!(inner.inputs.workspace, workspace);
            assert_eq!(inner.live_file, Some(live_file.clone()));
            assert_eq!(inner.trace_id.as_deref(), Some("change-task-first-smoke"));
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
        json!("change-task-first-smoke")
    );
    assert_eq!(preview_document["reviewed"], json!(false));

    let apply_args = task_first_change_cli_args("apply", &workspace, None, Some(&preview_file));
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
