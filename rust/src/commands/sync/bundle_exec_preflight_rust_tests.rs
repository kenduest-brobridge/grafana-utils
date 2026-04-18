//! Sync bundle execution tests for workspace-root and preflight behavior.

use super::{
    sync_common_args, write_alert_export_fixture, write_dashboard_provisioning_fixture,
    write_dashboard_raw_fixture, write_datasource_provisioning_fixture,
};
use crate::sync::{
    render_sync_apply_intent_text, run_sync_cli, SyncAdvancedCliArgs, SyncAdvancedCommand,
    SyncBundleArgs, SyncBundlePreflightArgs, SyncGroupCommand, SyncOutputFormat,
};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn run_sync_cli_bundle_reports_canonical_workspace_root_for_wrapped_git_sync_tree() {
    let temp = tempdir().unwrap();
    let repo_root = temp.path();
    let raw_root = repo_root.join("dashboards").join("git-sync").join("raw");
    fs::create_dir_all(&raw_root).unwrap();
    fs::write(
        raw_root.join("cpu.json"),
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
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        workspace: Some(raw_root.clone()),
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok(), "{result:?}");
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(
        bundle["metadata"]["workspaceRoot"],
        json!(repo_root.display().to_string())
    );
    assert_eq!(
        bundle["metadata"]["dashboardExportDir"],
        json!(raw_root.display().to_string())
    );
}

#[test]
fn run_sync_cli_bundle_rejects_conflicting_dashboard_inputs() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp.path().join("dashboards").join("raw");
    let dashboard_provisioning_dir = temp.path().join("dashboards").join("provisioning");
    fs::create_dir_all(&dashboard_export_dir).unwrap();
    fs::create_dir_all(&dashboard_provisioning_dir).unwrap();

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        workspace: None,
        dashboard_export_dir: Some(dashboard_export_dir),
        dashboard_provisioning_dir: Some(dashboard_provisioning_dir),
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: None,
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
    }));

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("only one dashboard input"));
}

#[test]
fn run_sync_cli_bundle_preflight_accepts_local_bundle_inputs() {
    let temp = tempdir().unwrap();
    let source_bundle = temp.path().join("source.json");
    let target_inventory = temp.path().join("target.json");
    fs::write(
        &source_bundle,
        serde_json::to_string_pretty(&json!({
            "dashboards": [],
            "datasources": [],
            "folders": [{"kind":"folder","uid":"ops","title":"Operations"}],
            "alerts": []
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &target_inventory,
        serde_json::to_string_pretty(&json!({})).unwrap(),
    )
    .unwrap();

    let result = run_sync_cli(SyncGroupCommand::Advanced(SyncAdvancedCliArgs {
        command: SyncAdvancedCommand::BundlePreflight(SyncBundlePreflightArgs {
            source_bundle,
            target_inventory,
            availability_file: None,
            fetch_live: false,
            common: sync_common_args(),
            org_id: None,
            output_format: SyncOutputFormat::Json,
        }),
    }));

    assert!(result.is_ok());
}

#[test]
fn render_sync_apply_intent_text_includes_alert_artifact_bundle_counts() {
    let lines = render_sync_apply_intent_text(&json!({
        "kind": "grafana-utils-sync-apply-intent",
        "stage": "apply",
        "stepIndex": 3,
        "traceId": "sync-trace-demo",
        "parentTraceId": "sync-trace-demo",
        "mode": "apply",
        "reviewed": true,
        "reviewRequired": true,
        "allowPrune": false,
        "approved": true,
        "summary": {
            "would_create": 1,
            "would_update": 0,
            "would_delete": 0,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 0
        },
        "operations": [],
        "bundlePreflightSummary": {
            "resourceCount": 4,
            "syncBlockingCount": 0,
            "providerBlockingCount": 0,
            "secretPlaceholderBlockingCount": 1,
            "alertArtifactCount": 4,
            "alertArtifactPlanOnlyCount": 1,
            "alertArtifactBlockingCount": 3
        }
    }))
    .unwrap();

    let output = lines.join("\n");
    assert!(output.contains("secret-placeholder-blocking=1"));
    assert!(output.contains("alert-artifacts=4"));
    assert!(output.contains("plan-only=1"));
    assert!(output.contains("blocking=3"));
    assert!(output.contains("Reason: input-test and package-test blocking must be 0 before apply"));
}

#[test]
fn run_sync_cli_bundle_accepts_mixed_git_sync_workspace_root() {
    let temp = tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join(".git")).unwrap();
    write_dashboard_raw_fixture(&workspace.join("dashboards").join("git-sync").join("raw"));
    write_dashboard_provisioning_fixture(
        &workspace
            .join("dashboards")
            .join("git-sync")
            .join("provisioning"),
    );
    write_alert_export_fixture(&workspace.join("alerts").join("raw"));
    fs::create_dir_all(workspace.join("datasources").join("provisioning")).unwrap();
    let datasource_provisioning_file = workspace
        .join("datasources")
        .join("provisioning")
        .join("datasources.yaml");
    write_datasource_provisioning_fixture(&datasource_provisioning_file);
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        workspace: Some(workspace.clone()),
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok(), "{result:?}");
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(
        bundle["metadata"]["workspaceRoot"],
        json!(workspace.display().to_string())
    );
    assert_eq!(
        bundle["discovery"]["workspaceRoot"],
        json!(workspace.display().to_string())
    );
    assert_eq!(bundle["discovery"]["inputCount"], json!(3));
    assert_eq!(
        bundle["discovery"]["inputs"]["dashboardExportDir"],
        json!(workspace
            .join("dashboards/git-sync/raw")
            .display()
            .to_string())
    );
    assert_eq!(
        bundle["metadata"]["dashboardExportDir"],
        json!(workspace
            .join("dashboards/git-sync/raw")
            .display()
            .to_string())
    );
    assert_eq!(
        bundle["metadata"]["alertExportDir"],
        json!(workspace.join("alerts/raw").display().to_string())
    );
    assert_eq!(
        bundle["metadata"]["datasourceProvisioningFile"],
        json!(workspace
            .join("datasources/provisioning/datasources.yaml")
            .display()
            .to_string())
    );
    assert_eq!(bundle["summary"]["dashboardCount"], json!(1));
    assert_eq!(bundle["summary"]["datasourceCount"], json!(1));
    assert_eq!(bundle["summary"]["alertRuleCount"], json!(1));
}

#[test]
fn run_sync_cli_bundle_accepts_git_sync_provisioning_workspace_root() {
    let temp = tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join(".git")).unwrap();
    write_dashboard_provisioning_fixture(
        &workspace
            .join("dashboards")
            .join("git-sync")
            .join("provisioning"),
    );
    write_alert_export_fixture(&workspace.join("alerts").join("raw"));
    fs::create_dir_all(workspace.join("datasources").join("provisioning")).unwrap();
    let datasource_provisioning_file = workspace
        .join("datasources")
        .join("provisioning")
        .join("datasources.yaml");
    write_datasource_provisioning_fixture(&datasource_provisioning_file);
    let output_file = temp.path().join("bundle.json");

    let result = run_sync_cli(SyncGroupCommand::Bundle(SyncBundleArgs {
        workspace: Some(workspace.clone()),
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        alert_export_dir: None,
        datasource_export_file: None,
        datasource_provisioning_file: None,
        metadata_file: None,
        output_file: Some(output_file.clone()),
        also_stdout: false,
        output_format: SyncOutputFormat::Json,
    }));

    assert!(result.is_ok(), "{result:?}");
    let bundle: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_file).unwrap()).unwrap();
    assert_eq!(
        bundle["metadata"]["workspaceRoot"],
        json!(workspace.display().to_string())
    );
    assert_eq!(
        bundle["discovery"]["inputs"]["dashboardProvisioningDir"],
        json!(workspace
            .join("dashboards/git-sync/provisioning")
            .display()
            .to_string())
    );
    assert_eq!(
        bundle["metadata"]["dashboardProvisioningDir"],
        json!(workspace
            .join("dashboards/git-sync/provisioning")
            .display()
            .to_string())
    );
    assert_eq!(
        bundle["metadata"]["dashboardExportDir"],
        serde_json::Value::Null
    );
    assert_eq!(
        bundle["metadata"]["dashboardExport"]["variant"],
        json!("provisioning")
    );
    assert_eq!(
        bundle["dashboards"][0]["sourcePath"],
        json!("team/cpu-main.json")
    );
    assert_eq!(bundle["summary"]["dashboardCount"], json!(1));
    assert_eq!(bundle["summary"]["datasourceCount"], json!(1));
    assert_eq!(bundle["summary"]["alertRuleCount"], json!(1));
}
