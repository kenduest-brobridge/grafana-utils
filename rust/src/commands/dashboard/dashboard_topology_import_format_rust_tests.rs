//! Dashboard topology and import-format regression tests.
#![allow(unused_imports)]

use super::*;

#[test]
fn build_topology_tui_groups_summarize_node_kinds() {
    let document = sample_topology_tui_document();
    let groups = build_topology_tui_groups(&document);

    let counts = groups
        .iter()
        .map(|group| (group.label.as_str(), group.count))
        .collect::<Vec<_>>();
    assert_eq!(
        counts,
        vec![
            ("All", 5),
            ("Datasources", 1),
            ("Dashboards", 1),
            ("Panels", 1),
            ("Variables", 1),
            ("Alert Rules", 1),
            ("Contact Points", 0),
            ("Mute Timings", 0),
            ("Policies", 0),
            ("Templates", 0),
            ("Alert Resources", 0),
        ]
    );
}

#[test]
fn filter_topology_tui_items_limits_items_to_selected_group() {
    let document = sample_topology_tui_document();

    let variables = filter_topology_tui_items(&document, "variable");
    assert_eq!(variables.len(), 1);
    assert_eq!(variables[0].kind, "variable");
    assert_eq!(variables[0].title, "cluster");

    let panels = filter_topology_tui_items(&document, "panel");
    assert_eq!(panels.len(), 1);
    assert_eq!(panels[0].kind, "panel");
    assert_eq!(panels[0].title, "Panel 7");

    let all = filter_topology_tui_items(&document, "all");
    assert_eq!(all.len(), document.nodes.len());
}

#[test]
fn validate_dashboard_export_dir_detects_custom_plugin_legacy_layout_and_schema_migration() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join("legacy.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "legacy-main",
                "title": "Legacy Main",
                "schemaVersion": 30,
                "rows": [],
                "panels": [
                    {"id": 7, "type": "acme-panel", "datasource": {"type": "acme-ds"}}
                ]
            },
            "__inputs": [{"name": "DS_PROM"}]
        }))
        .unwrap(),
    )
    .unwrap();

    let result =
        test_support::validate_dashboard_export_dir(&raw_dir, true, true, Some(39)).unwrap();
    let output = test_support::render_validation_result_json(&result).unwrap();

    assert_eq!(result.dashboard_count, 1);
    assert!(result.error_count >= 4);
    assert!(output.contains("custom-panel-plugin"));
    assert!(output.contains("custom-datasource-plugin"));
    assert!(output.contains("legacy-row-layout"));
    assert!(output.contains("schema-migration-required"));
}

#[test]
fn validate_dashboard_export_dir_warns_for_dashboard_v2_resource_shape() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join("v2.json"),
        serde_json::to_string_pretty(&json!({
            "apiVersion": "dashboard.grafana.app/v2",
            "kind": "Dashboard",
            "metadata": {"name": "v2-main"},
            "spec": {
                "title": "V2 Main",
                "elements": {},
                "variables": []
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let result =
        test_support::validate_dashboard_export_dir(&raw_dir, true, true, Some(39)).unwrap();
    let output = test_support::render_validation_result_json(&result).unwrap();

    assert_eq!(result.dashboard_count, 1);
    assert_eq!(result.error_count, 0);
    assert_eq!(result.warning_count, 1);
    assert!(output.contains("unsupported-dashboard-v2-resource"));
}

#[test]
fn snapshot_live_dashboard_export_with_fetcher_writes_dashboards_in_parallel() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    let summaries = vec![
        json!({"uid": "cpu-main", "title": "CPU Main", "folderTitle": "Infra"})
            .as_object()
            .unwrap()
            .clone(),
        json!({"uid": "logs-main", "title": "Logs Main", "folderTitle": "Ops"})
            .as_object()
            .unwrap()
            .clone(),
    ];

    let count = test_support::snapshot_live_dashboard_export_with_fetcher(
        &raw_dir,
        &summaries,
        4,
        false,
        |uid| {
            Ok(json!({
                "dashboard": {
                    "uid": uid,
                    "title": uid,
                    "schemaVersion": 39,
                    "panels": []
                },
                "meta": {}
            }))
        },
    )
    .unwrap();

    assert_eq!(count, 2);
    assert!(raw_dir.join("Infra/CPU_Main__cpu-main.json").is_file());
    assert!(raw_dir.join("Ops/Logs_Main__logs-main.json").is_file());
}

#[test]
fn import_dashboards_with_strict_schema_rejects_custom_plugins_before_live_write() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("custom.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "custom-main",
                "title": "Custom Main",
                "schemaVersion": 39,
                "panels": [
                    {"id": 7, "type": "acme-panel", "datasource": {"type": "prometheus"}}
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let mut args = make_import_args(raw_dir);
    args.strict_schema = true;
    args.dry_run = true;
    let error = test_support::import_dashboards_with_request(
        |_method, _path, _params, _payload| Ok(None),
        &args,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("custom-panel-plugin"));
    assert!(error.contains("unsupported custom panel plugin type"));
}
