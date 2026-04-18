//! Staged export scope resolver tests for snapshot review inputs.

use std::fs;

use super::tests_fixtures::{
    write_complete_dashboard_scope, write_datasource_inventory_rows,
    write_datasource_provisioning_lane, write_snapshot_datasource_root_metadata,
};
use crate::dashboard::TOOL_SCHEMA_VERSION;
use crate::staged_export_scopes::{
    resolve_dashboard_export_scope_dirs, resolve_datasource_export_scope_dirs,
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn staged_export_scope_resolver_prefers_dashboard_export_dirs_when_they_exist() {
    let temp = tempdir().unwrap();
    let dashboard_root = temp.path().join("dashboards");
    let main_scope = dashboard_root.join("org_1_Main_Org");
    let ops_scope = dashboard_root.join("org_2_Ops_Org");

    write_complete_dashboard_scope(&main_scope);
    write_complete_dashboard_scope(&ops_scope);

    let dashboard_metadata = json!({
        "kind": "grafana-utils-dashboard-export-index",
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "variant": "root",
        "dashboardCount": 2,
        "indexFile": "index.json",
        "orgCount": 2,
        "orgs": [
            {
                "org": "Main Org.",
                "orgId": "1",
                "dashboardCount": 1,
                "exportDir": "org_1_Main_Org",
            },
            {
                "org": "Ops Org",
                "orgId": "2",
                "dashboardCount": 1,
                "exportDir": "org_2_Ops_Org",
            },
        ],
    });

    let scopes = resolve_dashboard_export_scope_dirs(&dashboard_root, &dashboard_metadata);

    assert_eq!(scopes, vec![main_scope, ops_scope]);
}

#[test]
fn staged_export_scope_resolver_falls_back_to_single_dashboard_root_when_export_dirs_are_missing() {
    let temp = tempdir().unwrap();
    let dashboard_root = temp.path().join("dashboards");
    write_complete_dashboard_scope(&dashboard_root);

    let dashboard_metadata = json!({
        "kind": "grafana-utils-dashboard-export-index",
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "variant": "root",
        "dashboardCount": 1,
        "indexFile": "index.json",
        "orgCount": 1,
        "orgs": [
            {
                "org": "Main Org.",
                "orgId": "1",
                "dashboardCount": 1,
                "exportDir": "org_1_Main_Org",
            },
        ],
    });

    let scopes = resolve_dashboard_export_scope_dirs(&dashboard_root, &dashboard_metadata);

    assert_eq!(scopes, vec![dashboard_root]);
}

#[test]
fn staged_export_scope_resolver_discovers_real_datasource_scope_dirs_and_ignores_empty_siblings() {
    let temp = tempdir().unwrap();
    let datasource_root = temp.path().join("datasources");
    let root_scope = datasource_root.clone();
    let main_scope = datasource_root.join("org_1_Main_Org");
    let ops_scope = datasource_root.join("org_2_Ops_Org");
    let ignored_scope = datasource_root.join("org_3_Empty");
    let ignored_dir = datasource_root.join("notes");

    write_snapshot_datasource_root_metadata(&datasource_root, 2, "root");
    write_datasource_inventory_rows(
        &datasource_root,
        &[json!({
            "uid": "prom-main",
            "name": "prom-main",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": "1"
        })],
    );
    write_datasource_inventory_rows(
        &main_scope,
        &[json!({
            "uid": "prom-main",
            "name": "prom-main",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": "1"
        })],
    );
    write_datasource_provisioning_lane(&ops_scope);
    fs::create_dir_all(&ignored_scope).unwrap();
    fs::create_dir_all(&ignored_dir).unwrap();

    let mut scopes = resolve_datasource_export_scope_dirs(&datasource_root);
    scopes.sort();

    assert_eq!(scopes, vec![root_scope, main_scope, ops_scope]);
}
