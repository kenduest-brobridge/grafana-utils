//! Shared snapshot test fixtures and local writers.

use std::fs;
use std::path::Path;

use crate::common::sanitize_path_component;
use crate::dashboard::{CommonCliArgs, EXPORT_METADATA_FILENAME, TOOL_SCHEMA_VERSION};
use crate::snapshot::{
    SNAPSHOT_DATASOURCE_EXPORT_FILENAME, SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME,
    SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND, SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION,
};
use serde_json::json;
use serde_json::Value;

pub(crate) fn sample_common_args() -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: Some("prod".to_string()),
        url: "http://grafana.example.com".to_string(),
        api_token: Some("token".to_string()),
        username: Some("admin".to_string()),
        password: Some("admin".to_string()),
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

pub(crate) fn write_snapshot_dashboard_metadata(
    dashboard_root: &Path,
    orgs: &[(&str, &str, usize)],
) {
    let org_entries: Vec<Value> = orgs
        .iter()
        .map(|(org_id, org, dashboard_count)| {
            json!({
                "org": org,
                "orgId": org_id,
                "dashboardCount": dashboard_count,
                "exportDir": format!("org_{org_id}_{}", sanitize_path_component(org))
            })
        })
        .collect();
    let dashboard_count = orgs.iter().map(|(_, _, count)| *count).sum::<usize>();
    fs::create_dir_all(dashboard_root).unwrap();
    fs::write(
        dashboard_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "root",
            "dashboardCount": dashboard_count,
            "indexFile": "index.json",
            "orgCount": orgs.len(),
            "orgs": org_entries
        }))
        .unwrap(),
    )
    .unwrap();
}

pub(crate) fn write_snapshot_dashboard_index(dashboard_root: &Path, folders: &[Value]) {
    fs::create_dir_all(dashboard_root).unwrap();
    fs::write(
        dashboard_root.join("index.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "items": [],
            "variants": {
                "raw": null,
                "prompt": null,
                "provisioning": null
            },
            "folders": folders
        }))
        .unwrap(),
    )
    .unwrap();
}

pub(crate) fn write_snapshot_datasource_root_metadata(
    datasource_root: &Path,
    datasource_count: usize,
    variant: &str,
) {
    fs::create_dir_all(datasource_root).unwrap();
    fs::write(
        datasource_root.join(SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION,
            "kind": SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND,
            "variant": variant,
            "resource": "datasource",
            "orgCount": 1,
            "datasourceCount": datasource_count,
            "datasourcesFile": SNAPSHOT_DATASOURCE_EXPORT_FILENAME,
            "indexFile": "index.json",
            "format": "grafana-datasource-inventory-v1"
        }))
        .unwrap(),
    )
    .unwrap();
}

pub(crate) fn write_datasource_inventory_rows(datasource_root: &Path, rows: &[Value]) {
    fs::create_dir_all(datasource_root).unwrap();
    fs::write(
        datasource_root.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME),
        serde_json::to_string_pretty(&Value::Array(rows.to_vec())).unwrap(),
    )
    .unwrap();
}

pub(crate) fn write_complete_dashboard_scope(scope_dir: &Path) {
    fs::create_dir_all(scope_dir.join("raw")).unwrap();
    fs::write(scope_dir.join("raw/index.json"), "[]").unwrap();

    fs::create_dir_all(scope_dir.join("prompt")).unwrap();
    fs::write(scope_dir.join("prompt/index.json"), "[]").unwrap();

    fs::create_dir_all(scope_dir.join("provisioning/provisioning")).unwrap();
    fs::write(scope_dir.join("provisioning/index.json"), "[]").unwrap();
    fs::write(
        scope_dir.join("provisioning/provisioning/dashboards.yaml"),
        "apiVersion: 1\nproviders: []\n",
    )
    .unwrap();
}

pub(crate) fn write_datasource_provisioning_lane(scope_dir: &Path) {
    fs::create_dir_all(scope_dir.join("provisioning")).unwrap();
    fs::write(
        scope_dir.join("provisioning/datasources.yaml"),
        "apiVersion: 1\n",
    )
    .unwrap();
}

pub(crate) fn write_snapshot_datasource_inventory_root(
    datasource_root: &Path,
    rows: &[Value],
    datasource_count: usize,
    variant: &str,
) {
    write_snapshot_datasource_root_metadata(datasource_root, datasource_count, variant);
    write_datasource_inventory_rows(datasource_root, rows);
}

pub(crate) fn write_snapshot_access_lane_bundle(
    lane_root: &Path,
    payload_filename: &str,
    kind: &str,
    record_count: usize,
) {
    fs::create_dir_all(lane_root).unwrap();
    fs::write(
        lane_root.join(payload_filename),
        serde_json::to_string_pretty(&Value::Array(
            (0..record_count)
                .map(|index| json!({ "id": index + 1 }))
                .collect(),
        ))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        lane_root.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": kind,
            "version": 1,
            "recordCount": record_count,
            "sourceUrl": "http://grafana.example.com",
            "sourceDir": lane_root.to_string_lossy(),
        }))
        .unwrap(),
    )
    .unwrap();
}
