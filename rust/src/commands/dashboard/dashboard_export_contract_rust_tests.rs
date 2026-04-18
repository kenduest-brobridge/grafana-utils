//! Dashboard export contract regression tests.
#![allow(unused_imports)]

use super::*;

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
fn build_output_path_keeps_folder_structure() {
    let summary = json!({
        "folderTitle": "Infra Team",
        "title": "Cluster Health",
        "uid": "abc",
    });
    let path = build_output_path(Path::new("out"), summary.as_object().unwrap(), false);
    assert_eq!(path, Path::new("out/Infra_Team/Cluster_Health__abc.json"));
}
