//! Dashboard export/import, topology, and import-format regression tests.
#![allow(unused_imports)]

use super::*;
use crate::dashboard::{resolve_dashboard_import_source, DashboardImportInputFormat};
use std::path::Path;

#[test]
fn build_export_metadata_serializes_expected_shape() {
    let value = serde_json::to_value(build_export_metadata(
        "raw",
        2,
        Some("grafana-web-import-preserve-uid"),
        Some(FOLDER_INVENTORY_FILENAME),
        Some(DATASOURCE_INVENTORY_FILENAME),
        Some(DASHBOARD_PERMISSION_BUNDLE_FILENAME),
        Some("Main Org."),
        Some("1"),
        None,
        "live",
        Some("http://127.0.0.1:3000"),
        None,
        None,
        Path::new("/tmp/raw"),
        Path::new("/tmp/raw/export-metadata.json"),
    ))
    .unwrap();

    assert_eq!(
        value,
        json!({
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "toolVersion": crate::common::TOOL_VERSION,
            "kind": "grafana-utils-dashboard-export-index",
            "variant": "raw",
            "dashboardCount": 2,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json",
            "datasourcesFile": "datasources.json",
            "permissionsFile": "permissions.json",
            "org": "Main Org.",
            "orgId": "1",
            "metadataVersion": 2,
            "domain": "dashboard",
            "resourceKind": "dashboards",
            "bundleKind": "export-root",
            "source": {
                "kind": "live",
                "url": "http://127.0.0.1:3000",
                "orgScope": "org",
                "orgId": "1",
                "orgName": "Main Org."
            },
            "capture": {
                "toolVersion": crate::common::TOOL_VERSION,
                "capturedAt": value["capture"]["capturedAt"],
                "recordCount": 2
            },
            "paths": {
                "artifact": "/tmp/raw",
                "metadata": "/tmp/raw/export-metadata.json"
            }
        })
    );
}

#[test]
fn build_root_export_index_serializes_expected_shape() {
    let summary = serde_json::from_value(json!({
        "uid": "cpu-main",
        "title": "CPU Overview",
        "folderTitle": "Infra",
        "orgName": "Main Org.",
        "orgId": 1
    }))
    .unwrap();
    let mut item = test_support::build_dashboard_index_item(&summary, "cpu-main");
    item.raw_path = Some("/tmp/raw/cpu-main.json".to_string());

    let value = serde_json::to_value(build_root_export_index(
        &[item],
        Some(Path::new("/tmp/raw/index.json")),
        None,
        None,
        &[],
    ))
    .unwrap();

    assert_eq!(
        value,
        json!({
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "toolVersion": crate::common::TOOL_VERSION,
            "kind": "grafana-utils-dashboard-export-index",
            "items": [
                {
                    "uid": "cpu-main",
                    "title": "CPU Overview",
                    "folderTitle": "Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                    "raw_path": "/tmp/raw/cpu-main.json"
                }
            ],
            "variants": {
                "raw": "/tmp/raw/index.json",
                "prompt": null,
                "provisioning": null
            },
            "folders": []
        })
    );
}

#[test]
fn collect_folder_inventory_with_request_records_parent_chain() {
    let summaries = vec![json!({
        "uid": "cpu-main",
        "title": "CPU Overview",
        "folderTitle": "Infra",
        "folderUid": "infra",
        "orgName": "Main Org.",
        "orgId": 1
    })
    .as_object()
    .unwrap()
    .clone()];

    let folders = test_support::collect_folder_inventory_with_request(
        |_method, path, _params, _payload| match path {
            "/api/folders/infra" => Ok(Some(json!({
                "uid": "infra",
                "title": "Infra",
                "parents": [
                    {"uid": "platform", "title": "Platform"},
                    {"uid": "team", "title": "Team"}
                ]
            }))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &summaries,
    )
    .unwrap();

    assert_eq!(
        serde_json::to_value(folders).unwrap(),
        json!([
            {
                "uid": "platform",
                "title": "Platform",
                "path": "Platform",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "team",
                "title": "Team",
                "path": "Platform / Team",
                "parentUid": "platform",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "infra",
                "title": "Infra",
                "path": "Platform / Team / Infra",
                "parentUid": "team",
                "org": "Main Org.",
                "orgId": "1"
            }
        ])
    );
}

#[test]
fn build_datasource_inventory_record_keeps_datasource_config_fields() {
    let datasource = json!({
        "uid": "influx-main",
        "name": "Influx Main",
        "type": "influxdb",
        "access": "proxy",
        "url": "http://influxdb:8086",
        "jsonData": {
            "dbName": "metrics_v1",
            "defaultBucket": "prod-default",
            "organization": "acme-observability"
        }
    })
    .as_object()
    .unwrap()
    .clone();
    let org = json!({
        "id": 1,
        "name": "Main Org."
    })
    .as_object()
    .unwrap()
    .clone();

    let record = test_support::build_datasource_inventory_record(&datasource, &org);
    assert_eq!(record.database, "metrics_v1");
    assert_eq!(record.default_bucket, "prod-default");
    assert_eq!(record.organization, "acme-observability");

    let elastic = json!({
        "uid": "elastic-main",
        "name": "Elastic Main",
        "type": "elasticsearch",
        "access": "proxy",
        "url": "http://elasticsearch:9200",
        "jsonData": {
            "indexPattern": "[logs-]YYYY.MM.DD"
        }
    })
    .as_object()
    .unwrap()
    .clone();
    let elastic_record = test_support::build_datasource_inventory_record(&elastic, &org);
    assert_eq!(elastic_record.index_pattern, "[logs-]YYYY.MM.DD");
}

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

#[test]
fn build_output_path_keeps_folder_structure() {
    let summary = json!({
        "folderTitle": "Infra Team",
        "title": "Cluster Health",
        "uid": "abc",
    });
    let path = build_output_path(Path::new("out"), summary.as_object().unwrap(), false);
    assert_eq!(path, Path::new("out/Infra_Team/Cluster_Health__abc.json"));
}

#[test]
fn build_folder_inventory_status_reports_missing_folder() {
    let folder = test_support::FolderInventoryItem {
        uid: "child".to_string(),
        title: "Child".to_string(),
        path: "Platform / Child".to_string(),
        parent_uid: Some("platform".to_string()),
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    };

    let status = build_folder_inventory_status(&folder, None);

    assert_eq!(status.kind, FolderInventoryStatusKind::Missing);
    assert_eq!(
        format_folder_inventory_status_line(&status),
        "Folder inventory missing uid=child title=Child parentUid=platform path=Platform / Child"
    );
}

#[test]
fn build_folder_inventory_status_reports_matching_folder() {
    let folder = test_support::FolderInventoryItem {
        uid: "child".to_string(),
        title: "Child".to_string(),
        path: "Platform / Child".to_string(),
        parent_uid: Some("platform".to_string()),
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    };
    let destination_folder = json!({
        "uid": "child",
        "title": "Child",
        "parents": [{"uid": "platform", "title": "Platform"}]
    })
    .as_object()
    .unwrap()
    .clone();

    let status = build_folder_inventory_status(&folder, Some(&destination_folder));

    assert_eq!(status.kind, FolderInventoryStatusKind::Matches);
    assert_eq!(
        format_folder_inventory_status_line(&status),
        "Folder inventory matches uid=child title=Child parentUid=platform path=Platform / Child"
    );
}

#[test]
fn build_folder_inventory_status_reports_mismatch_details() {
    let folder = test_support::FolderInventoryItem {
        uid: "child".to_string(),
        title: "Child".to_string(),
        path: "Platform / Child".to_string(),
        parent_uid: Some("platform".to_string()),
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    };
    let destination_folder = json!({
        "uid": "child",
        "title": "Ops Child",
        "parents": [{"uid": "ops", "title": "Ops"}]
    })
    .as_object()
    .unwrap()
    .clone();

    let status = build_folder_inventory_status(&folder, Some(&destination_folder));

    assert_eq!(status.kind, FolderInventoryStatusKind::Mismatch);
    assert_eq!(
        format_folder_inventory_status_line(&status),
        "Folder inventory mismatch uid=child expected(title=Child, parentUid=platform, path=Platform / Child) actual(title=Ops Child, parentUid=ops, path=Ops / Ops Child)"
    );
}

#[test]
fn render_folder_inventory_dry_run_table_supports_expected_columns() {
    let rows = vec![[
        "child".to_string(),
        "exists".to_string(),
        "mismatch".to_string(),
        "path".to_string(),
        "Platform / Child".to_string(),
        "Legacy / Child".to_string(),
    ]];

    let with_header = test_support::render_folder_inventory_dry_run_table(&rows, true);

    assert!(with_header[0].contains("EXPECTED_PATH"));
    assert!(with_header[0].contains("ACTUAL_PATH"));
    assert!(with_header[2].contains("Legacy / Child"));
}

#[test]
fn export_progress_line_uses_concise_counter_format() {
    assert_eq!(
        format_export_progress_line(2, 5, "cpu-main", false),
        "Exporting dashboard 2/5: cpu-main"
    );
    assert_eq!(
        format_export_progress_line(2, 5, "cpu-main", true),
        "Would export dashboard 2/5: cpu-main"
    );
}

#[test]
fn export_verbose_line_includes_variant_and_path() {
    assert_eq!(
        format_export_verbose_line("prompt", "cpu-main", Path::new("/tmp/out.json"), false),
        "Exported prompt cpu-main -> /tmp/out.json"
    );
    assert_eq!(
        format_export_verbose_line("raw", "cpu-main", Path::new("/tmp/out.json"), true),
        "Would export raw    cpu-main -> /tmp/out.json"
    );
}

#[test]
fn import_progress_line_uses_concise_counter_format() {
    assert_eq!(
        format_import_progress_line(3, 7, "/tmp/raw/cpu.json", false, None, None),
        "Importing dashboard 3/7: /tmp/raw/cpu.json"
    );
    assert_eq!(
        format_import_progress_line(
            3,
            7,
            "cpu-main",
            true,
            Some("would-update"),
            Some("General")
        ),
        "Dry-run dashboard 3/7: cpu-main dest=exists action=update folderPath=General"
    );
    assert_eq!(
        format_import_progress_line(3, 7, "cpu-main", true, Some("would-skip-missing"), Some("Platform / Infra")),
        "Dry-run dashboard 3/7: cpu-main dest=missing action=skip-missing folderPath=Platform / Infra"
    );
}

#[test]
fn render_import_dry_run_table_supports_optional_header() {
    let rows = vec![
        [
            "abc".to_string(),
            "exists".to_string(),
            "update".to_string(),
            "General".to_string(),
            "General".to_string(),
            "General".to_string(),
            "".to_string(),
            "/tmp/a.json".to_string(),
        ],
        [
            "xyz".to_string(),
            "missing".to_string(),
            "create".to_string(),
            "Platform / Infra".to_string(),
            "Platform / Infra".to_string(),
            "".to_string(),
            "".to_string(),
            "/tmp/b.json".to_string(),
        ],
    ];
    let with_header = test_support::render_import_dry_run_table(&rows, true, None);
    assert!(with_header[0].contains("UID"));
    assert!(with_header[0].contains("DESTINATION"));
    assert!(with_header[0].contains("ACTION"));
    assert!(with_header[0].contains("FOLDER_PATH"));
    assert!(with_header[0].contains("FILE"));
    assert!(with_header[2].contains("abc"));
    assert!(with_header[2].contains("exists"));
    assert!(with_header[2].contains("update"));
    assert!(with_header[2].contains("General"));
    assert!(with_header[2].contains("/tmp/a.json"));
    let without_header = test_support::render_import_dry_run_table(&rows, false, None);
    assert_eq!(without_header.len(), 2);
    assert!(without_header[0].contains("abc"));
    assert!(without_header[0].contains("exists"));
    assert!(without_header[0].contains("update"));
    assert!(without_header[0].contains("General"));
    assert!(without_header[0].contains("/tmp/a.json"));
}

#[test]
fn render_import_dry_run_table_honors_selected_columns() {
    let rows = vec![[
        "abc".to_string(),
        "exists".to_string(),
        "skip-folder-mismatch".to_string(),
        "Platform / Ops".to_string(),
        "Platform / Source".to_string(),
        "Platform / Dest".to_string(),
        "path".to_string(),
        "/tmp/a.json".to_string(),
    ]];

    let lines = test_support::render_import_dry_run_table(
        &rows,
        true,
        Some(&["uid".to_string(), "reason".to_string(), "file".to_string()]),
    );

    assert!(lines[0].contains("UID"));
    assert!(lines[0].contains("REASON"));
    assert!(lines[0].contains("FILE"));
    assert!(!lines[0].contains("DESTINATION"));
    assert!(lines[2].contains("abc"));
    assert!(lines[2].contains("path"));
    assert!(lines[2].contains("/tmp/a.json"));
}

#[test]
fn render_import_dry_run_json_returns_structured_document() {
    let folder_status = test_support::FolderInventoryStatus {
        uid: "infra".to_string(),
        expected_title: "Infra".to_string(),
        expected_parent_uid: Some("platform".to_string()),
        expected_path: "Platform / Infra".to_string(),
        actual_title: Some("Infra".to_string()),
        actual_parent_uid: Some("platform".to_string()),
        actual_path: Some("Platform / Infra".to_string()),
        kind: FolderInventoryStatusKind::Matches,
    };
    let rows = vec![[
        "abc".to_string(),
        "exists".to_string(),
        "update".to_string(),
        "Platform / Infra".to_string(),
        "Platform / Infra".to_string(),
        "Platform / Infra".to_string(),
        "".to_string(),
        "/tmp/a.json".to_string(),
    ]];

    let value: Value = serde_json::from_str(
        &test_support::render_import_dry_run_json(
            "create-or-update",
            &[folder_status],
            &rows,
            Path::new("/tmp/raw"),
            0,
            0,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(value["mode"], "create-or-update");
    assert_eq!(value["folders"][0]["uid"], "infra");
    assert_eq!(value["dashboards"][0]["folderPath"], "Platform / Infra");
    assert_eq!(
        value["dashboards"][0]["sourceFolderPath"],
        "Platform / Infra"
    );
    assert_eq!(
        value["dashboards"][0]["destinationFolderPath"],
        "Platform / Infra"
    );
    assert_eq!(value["summary"]["dashboardCount"], 1);
}

#[test]
fn render_routed_import_org_table_includes_org_level_columns() {
    let rows = vec![
        [
            "2".to_string(),
            "Org Two".to_string(),
            "exists".to_string(),
            "2".to_string(),
            "3".to_string(),
        ],
        [
            "9".to_string(),
            "Ops Org".to_string(),
            "would-create".to_string(),
            "<new>".to_string(),
            "1".to_string(),
        ],
    ];

    let lines = test_support::import::render_routed_import_org_table(&rows, true);

    assert!(lines[0].contains("SOURCE_ORG_ID"));
    assert!(lines[0].contains("ORG_ACTION"));
    assert!(lines[2].contains("Org Two"));
    assert!(lines[3].contains("would-create"));
}

#[test]
fn routed_import_scope_identity_matches_table_json_and_progress_surfaces() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("2", "Org Two", "org_2_Org_Two"),
            ("9", "Ops Org", "org_9_Ops_Org"),
        ],
    );
    write_basic_raw_export(
        &org_two_raw,
        "2",
        "Org Two",
        "cpu-two",
        "CPU Two",
        "prom-two",
        "prometheus",
        "timeseries",
        "general",
        "General",
        "expr",
        "up",
    );
    write_basic_raw_export(
        &org_nine_raw,
        "9",
        "Ops Org",
        "ops-main",
        "Ops Main",
        "loki-nine",
        "loki",
        "logs",
        "ops",
        "Ops",
        "expr",
        "{job=\"grafana\"}",
    );

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.create_missing_orgs = true;
    args.dry_run = true;
    args.json = true;

    let payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 2, "name": "Org Two"}
                ]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            },
            |_target_org_id, scoped_args| {
                Ok(test_support::import::ImportDryRunReport {
                    mode: "create-only".to_string(),
                    import_dir: scoped_args.import_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: Vec::new(),
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &args,
        )
        .unwrap(),
    )
    .unwrap();

    let org_entries = payload["orgs"].as_array().unwrap();
    let rows: Vec<[String; 5]> = org_entries
        .iter()
        .map(|entry| {
            [
                entry["sourceOrgId"].as_i64().unwrap().to_string(),
                entry["sourceOrgName"].as_str().unwrap().to_string(),
                entry["orgAction"].as_str().unwrap().to_string(),
                test_support::import::format_routed_import_target_org_label(
                    entry["targetOrgId"].as_i64(),
                ),
                entry["dashboardCount"].as_u64().unwrap().to_string(),
            ]
        })
        .collect();
    let table_lines = test_support::import::render_routed_import_org_table(&rows, true);

    let org_two = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let org_nine = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();

    let existing_summary = test_support::import::format_routed_import_scope_summary_fields(
        2,
        "Org Two",
        "exists",
        Some(2),
        Path::new(org_two["importDir"].as_str().unwrap()),
    );
    let would_create_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "would-create",
        None,
        Path::new(org_nine["importDir"].as_str().unwrap()),
    );

    assert_eq!(org_two["targetOrgId"], json!(2));
    assert_eq!(org_nine["targetOrgId"], Value::Null);
    assert!(table_lines[2].contains("Org Two"));
    assert!(table_lines[2].contains("2"));
    assert!(table_lines[3].contains("Ops Org"));
    assert!(table_lines[3].contains("<new>"));
    assert!(existing_summary.contains("export orgId=2"));
    assert!(existing_summary.contains("name=Org Two"));
    assert!(existing_summary.contains("orgAction=exists"));
    assert!(existing_summary.contains("targetOrgId=2"));
    assert!(existing_summary.contains(org_two["importDir"].as_str().unwrap()));
    assert!(would_create_summary.contains("export orgId=9"));
    assert!(would_create_summary.contains("name=Ops Org"));
    assert!(would_create_summary.contains("orgAction=would-create"));
    assert!(would_create_summary.contains("targetOrgId=<new>"));
    assert!(would_create_summary.contains(org_nine["importDir"].as_str().unwrap()));
}

#[test]
fn routed_import_selected_scope_statuses_match_json_table_and_summary_contract() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_five_raw = export_root.join("org_5_Org_Five").join("raw");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("2", "Org Two", "org_2_Org_Two"),
            ("5", "Org Five", "org_5_Org_Five"),
            ("9", "Ops Org", "org_9_Ops_Org"),
        ],
    );
    write_basic_raw_export(
        &org_two_raw,
        "2",
        "Org Two",
        "cpu-two",
        "CPU Two",
        "prom-two",
        "prometheus",
        "timeseries",
        "general",
        "General",
        "expr",
        "up",
    );
    write_basic_raw_export(
        &org_five_raw,
        "5",
        "Org Five",
        "cpu-five",
        "CPU Five",
        "prom-five",
        "prometheus",
        "timeseries",
        "general",
        "General",
        "expr",
        "up",
    );
    write_basic_raw_export(
        &org_nine_raw,
        "9",
        "Ops Org",
        "ops-main",
        "Ops Main",
        "loki-nine",
        "loki",
        "logs",
        "ops",
        "Ops",
        "expr",
        "{job=\"grafana\"}",
    );

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.only_org_id = vec![2, 9];
    args.create_missing_orgs = false;
    args.dry_run = true;
    args.json = true;

    let payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 2, "name": "Org Two"}
                ]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            },
            |_target_org_id, scoped_args| {
                Ok(test_support::import::ImportDryRunReport {
                    mode: "create-only".to_string(),
                    import_dir: scoped_args.import_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: Vec::new(),
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &args,
        )
        .unwrap(),
    )
    .unwrap();

    let org_entries = payload["orgs"].as_array().unwrap();
    let import_entries = payload["imports"].as_array().unwrap();
    assert_eq!(org_entries.len(), 2);
    assert_eq!(import_entries.len(), 2);
    assert_eq!(payload["summary"]["orgCount"], json!(2));
    assert_eq!(payload["summary"]["existingOrgCount"], json!(1));
    assert_eq!(payload["summary"]["missingOrgCount"], json!(1));
    assert_eq!(payload["summary"]["wouldCreateOrgCount"], json!(0));

    let org_two = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let org_nine = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    assert!(org_entries
        .iter()
        .all(|entry| entry["sourceOrgId"] != json!(5)));

    assert_eq!(org_two["orgAction"], json!("exists"));
    assert_eq!(org_two["targetOrgId"], json!(2));
    assert_eq!(org_nine["orgAction"], json!("missing"));
    assert_eq!(org_nine["targetOrgId"], Value::Null);

    let org_nine_import = import_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    assert_eq!(org_nine_import["orgAction"], json!("missing"));
    assert_eq!(org_nine_import["dashboards"], json!([]));
    assert_eq!(org_nine_import["summary"]["dashboardCount"], json!(1));

    let rows: Vec<[String; 5]> = org_entries
        .iter()
        .map(|entry| {
            [
                entry["sourceOrgId"].as_i64().unwrap().to_string(),
                entry["sourceOrgName"].as_str().unwrap().to_string(),
                entry["orgAction"].as_str().unwrap().to_string(),
                test_support::import::format_routed_import_target_org_label(
                    entry["targetOrgId"].as_i64(),
                ),
                entry["dashboardCount"].as_u64().unwrap().to_string(),
            ]
        })
        .collect();
    let table_lines = test_support::import::render_routed_import_org_table(&rows, true);
    assert!(table_lines[2].contains("Org Two"));
    assert!(table_lines[2].contains("exists"));
    assert!(table_lines[2].contains("2"));
    assert!(table_lines[3].contains("Ops Org"));
    assert!(table_lines[3].contains("missing"));
    assert!(table_lines[3].contains("<new>"));

    let missing_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "missing",
        None,
        Path::new(org_nine["importDir"].as_str().unwrap()),
    );
    assert!(missing_summary.contains("export orgId=9"));
    assert!(missing_summary.contains("name=Ops Org"));
    assert!(missing_summary.contains("orgAction=missing"));
    assert!(missing_summary.contains("targetOrgId=<new>"));
    assert!(missing_summary.contains(org_nine["importDir"].as_str().unwrap()));
}

#[test]
fn describe_dashboard_import_mode_uses_expected_labels() {
    assert_eq!(
        test_support::describe_dashboard_import_mode(false, false),
        "create-only"
    );
    assert_eq!(
        test_support::describe_dashboard_import_mode(true, false),
        "create-or-update"
    );
    assert_eq!(
        test_support::describe_dashboard_import_mode(false, true),
        "update-or-skip-missing"
    );
}

#[test]
fn import_verbose_line_includes_dry_run_action() {
    assert_eq!(
        format_import_verbose_line(Path::new("/tmp/raw/cpu.json"), false, None, None, None),
        "Imported /tmp/raw/cpu.json"
    );
    assert_eq!(
        format_import_verbose_line(
            Path::new("/tmp/raw/cpu.json"),
            true,
            Some("cpu-main"),
            Some("would-update"),
            Some("General")
        ),
        "Dry-run import uid=cpu-main dest=exists action=update folderPath=General file=/tmp/raw/cpu.json"
    );
    assert_eq!(
        format_import_verbose_line(
            Path::new("/tmp/raw/cpu.json"),
            true,
            Some("cpu-main"),
            Some("would-skip-missing"),
            Some("Platform / Infra")
        ),
        "Dry-run import uid=cpu-main dest=missing action=skip-missing folderPath=Platform / Infra file=/tmp/raw/cpu.json"
    );
}

#[path = "dashboard_export_import_inventory_rust_tests.rs"]
mod dashboard_export_import_inventory_rust_tests;

#[path = "dashboard_list_render_rust_tests.rs"]
mod dashboard_list_render_rust_tests;
