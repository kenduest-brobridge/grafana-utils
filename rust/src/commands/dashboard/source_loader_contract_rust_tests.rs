//! Focused regression tests for dashboard source-loader contracts.

use super::*;
use std::fs;
use tempfile::tempdir;

fn make_dashboard_repo() -> tempfile::TempDir {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join(".git")).unwrap();
    fs::create_dir_all(temp.path().join("dashboards")).unwrap();
    temp
}

fn write_all_orgs_root_with_raw_variant(export_root: &std::path::Path) -> std::path::PathBuf {
    let org_export_dir = export_root.join("org_1_Main_Org/raw");
    fs::create_dir_all(&org_export_dir).unwrap();
    fs::write(
        export_root.join("export-metadata.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": crate::dashboard::TOOL_SCHEMA_VERSION,
            "variant": "root",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "orgCount": 1,
            "orgs": [{
                "org": "Main Org",
                "orgId": "1",
                "dashboardCount": 1,
                "exportDir": "org_1_Main_Org"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_export_dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": crate::dashboard::TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json",
            "datasourcesFile": "datasources.json",
            "org": "Main Org",
            "orgId": "1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_export_dir.join("folders.json"),
        serde_json::to_string_pretty(&serde_json::json!([{
            "uid": "general",
            "title": "General",
            "path": "General",
            "org": "Main Org",
            "orgId": "1"
        }]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_export_dir.join("datasources.json"),
        serde_json::to_string_pretty(&serde_json::json!([{
            "uid": "prom-main",
            "name": "prom-main",
            "type": "prometheus",
            "access": "proxy",
            "url": "http://grafana.example.internal",
            "isDefault": "true",
            "org": "Main Org",
            "orgId": "1"
        }]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_export_dir.join("index.json"),
        serde_json::to_string_pretty(&serde_json::json!([{
            "uid": "cpu-main",
            "title": "CPU Main",
            "path": "dash.json",
            "format": "grafana-web-import-preserve-uid",
            "org": "Main Org",
            "orgId": "1"
        }]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        org_export_dir.join("dash.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "dashboard": {
                "id": null,
                "uid": "cpu-main",
                "title": "CPU Main",
                "schemaVersion": 38,
                "panels": []
            },
            "meta": {
                "folderUid": "general",
                "folderTitle": "General"
            }
        }))
        .unwrap(),
    )
    .unwrap();
    export_root.to_path_buf()
}

#[test]
fn source_loader_contract_resolves_direct_raw_root() {
    let temp = make_dashboard_repo();
    let raw_root = temp.path().join("dashboards/raw");
    fs::create_dir_all(&raw_root).unwrap();

    let resolved =
        load_dashboard_source(&raw_root, DashboardImportInputFormat::Raw, None, true).unwrap();

    assert_eq!(resolved.workspace_root, temp.path());
    assert_eq!(resolved.input_dir, raw_root);
    assert_eq!(resolved.expected_variant, RAW_EXPORT_SUBDIR);
    assert_eq!(
        resolved.resolved.source_kind,
        DashboardSourceKind::RawExport
    );
    assert_eq!(resolved.resolved.dashboard_dir, raw_root);
    assert_eq!(resolved.resolved.metadata_dir, raw_root);
}

#[test]
fn source_loader_contract_resolves_direct_provisioning_root() {
    let temp = make_dashboard_repo();
    let provisioning_root = temp.path().join("dashboards/provisioning");
    fs::create_dir_all(provisioning_root.join("dashboards")).unwrap();

    let resolved = load_dashboard_source(
        &provisioning_root,
        DashboardImportInputFormat::Provisioning,
        None,
        true,
    )
    .unwrap();

    assert_eq!(resolved.workspace_root, temp.path());
    assert_eq!(resolved.input_dir, provisioning_root.join("dashboards"));
    assert_eq!(resolved.expected_variant, "provisioning");
    assert_eq!(
        resolved.resolved.source_kind,
        DashboardSourceKind::ProvisioningExport
    );
    assert_eq!(
        resolved.resolved.dashboard_dir,
        provisioning_root.join("dashboards")
    );
    assert_eq!(resolved.resolved.metadata_dir, provisioning_root);
}

#[test]
fn source_loader_contract_resolves_wrapped_git_sync_repo_root() {
    let temp = make_dashboard_repo();
    let wrapped_raw_root = temp.path().join("dashboards/git-sync/raw");
    fs::create_dir_all(&wrapped_raw_root).unwrap();

    let resolved =
        load_dashboard_source(temp.path(), DashboardImportInputFormat::Raw, None, true).unwrap();

    assert_eq!(resolved.workspace_root, temp.path());
    assert_eq!(resolved.input_dir, wrapped_raw_root);
    assert_eq!(resolved.expected_variant, RAW_EXPORT_SUBDIR);
    assert_eq!(
        resolved.resolved.source_kind,
        DashboardSourceKind::RawExport
    );
    assert_eq!(resolved.resolved.dashboard_dir, wrapped_raw_root);
    assert_eq!(resolved.resolved.metadata_dir, wrapped_raw_root);
}

#[test]
fn source_loader_contract_resolves_wrapped_git_sync_tree_from_dashboards_root() {
    let temp = make_dashboard_repo();
    let wrapped_provisioning_root = temp.path().join("dashboards/git-sync/provisioning");
    fs::create_dir_all(&wrapped_provisioning_root).unwrap();

    let dashboards_root = temp.path().join("dashboards");
    assert_eq!(
        resolve_dashboard_workspace_variant_dir(&dashboards_root, "provisioning"),
        Some(wrapped_provisioning_root)
    );
}

#[test]
fn source_loader_contract_prefers_root_export_over_conflicting_variant_child() {
    let temp = make_dashboard_repo();
    let dashboards_root = temp.path().join("dashboards");
    write_all_orgs_root_with_raw_variant(&dashboards_root);

    let conflicting_raw_root = dashboards_root.join("raw");
    fs::create_dir_all(&conflicting_raw_root).unwrap();
    fs::write(conflicting_raw_root.join("export-metadata.json"), "{}\n").unwrap();

    let resolved = load_dashboard_source(
        &dashboards_root,
        DashboardImportInputFormat::Raw,
        None,
        true,
    )
    .unwrap();

    let temp_root = resolved.temp_dir.as_ref().unwrap().path.clone();
    assert!(resolved.input_dir.starts_with(&temp_root));
    assert_eq!(resolved.workspace_root, temp.path());
    assert_ne!(resolved.input_dir, conflicting_raw_root);
    assert_eq!(resolved.expected_variant, RAW_EXPORT_SUBDIR);
    drop(resolved);
    assert!(!temp_root.exists());
}
