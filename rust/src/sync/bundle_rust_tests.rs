//! Sync bundle preflight contract tests.
//! Validates sync bundle planning output and sync-source dependency assembly.
use super::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
    SYNC_BUNDLE_PREFLIGHT_KIND,
};
use crate::sync::workbench::build_sync_source_bundle_document;
use serde_json::json;

#[test]
fn build_sync_bundle_preflight_document_aggregates_sync_and_provider_checks() {
    let source_bundle = json!({
        "dashboards": [
            {
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "body": {"datasourceUids": ["prom-main"]}
            }
        ],
        "datasources": [
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "Prometheus Main",
                "body": {"type": "prometheus"},
                "secureJsonDataProviders": {
                    "httpHeaderValue1": "${provider:vault:secret/data/prom/token}"
                }
            }
        ],
        "alerts": [
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition", "contactPoints"],
                "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]}
            }
        ]
    });
    let target_inventory = json!({"dashboards": [], "datasources": []});
    let availability = json!({
        "pluginIds": [],
        "datasourceUids": [],
        "contactPoints": [],
        "providerNames": []
    });

    let document = build_sync_bundle_preflight_document(
        &source_bundle,
        &target_inventory,
        Some(&availability),
    )
    .unwrap();

    assert_eq!(document["kind"], json!(SYNC_BUNDLE_PREFLIGHT_KIND));
    assert!(document["summary"]["syncBlockingCount"].as_i64().unwrap() >= 1);
    assert_eq!(document["summary"]["providerBlockingCount"], json!(1));
    assert_eq!(
        document["providerAssessment"]["plans"][0]["providerKind"],
        json!("external-provider-reference")
    );
}

#[test]
fn render_sync_bundle_preflight_text_renders_summary() {
    let document = build_sync_bundle_preflight_document(
        &json!({"folders": [{"kind": "folder", "uid": "ops", "title": "Operations"}]}),
        &json!({}),
        None,
    )
    .unwrap();

    let output = render_sync_bundle_preflight_text(&document)
        .unwrap()
        .join("\n");

    assert!(output.contains("Sync bundle preflight summary"));
    assert!(output.contains("Resources: 1 total"));
    assert!(output.contains("Sync blocking:"));
    assert!(output.contains("Provider blocking:"));
}

#[test]
fn render_sync_bundle_preflight_rejects_wrong_kind() {
    let error = render_sync_bundle_preflight_text(&json!({"kind": "wrong"}))
        .unwrap_err()
        .to_string();

    assert!(error.contains("kind is not supported"));
}

#[test]
fn build_sync_bundle_preflight_document_reads_provider_metadata_from_source_bundle_document() {
    let source_bundle = build_sync_source_bundle_document(
        &[json!({
            "kind": "dashboard",
            "uid": "cpu-main",
            "title": "CPU Main",
            "body": {"datasourceUids": ["loki-main"]},
        })],
        &[json!({
            "kind": "datasource",
            "uid": "loki-main",
            "name": "Loki Main",
            "title": "Loki Main",
            "body": {"uid": "loki-main", "name": "Loki Main", "type": "loki"},
            "secureJsonDataProviders": {
                "httpHeaderValue1": "${provider:vault:secret/data/loki/token}"
            },
            "secureJsonDataPlaceholders": {
                "basicAuthPassword": "${secret:loki-basic-auth}"
            }
        })],
        &[],
        &[],
        None,
        None,
    )
    .unwrap();
    let target_inventory = json!({"dashboards": [], "datasources": []});
    let availability = json!({
        "pluginIds": ["loki"],
        "datasourceUids": [],
        "datasourceNames": [],
        "contactPoints": [],
        "providerNames": ["vault"],
    });

    let document = build_sync_bundle_preflight_document(
        &source_bundle,
        &target_inventory,
        Some(&availability),
    )
    .unwrap();

    assert_eq!(document["summary"]["providerBlockingCount"], json!(0));
    assert_eq!(
        document["providerAssessment"]["plans"][0]["providers"][0]["providerName"],
        json!("vault")
    );
}

#[test]
fn build_sync_source_bundle_document_keeps_normalized_alert_specs() {
    let source_bundle = build_sync_source_bundle_document(
        &[],
        &[],
        &[],
        &[json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "datasourceUids"],
            "body": {
                "condition": "A",
                "datasourceUids": ["prom-main"]
            },
            "sourcePath": "rules/infra/cpu-high.json"
        })],
        Some(&json!({"summary": {"ruleCount": 1}})),
        None,
    )
    .unwrap();

    assert_eq!(source_bundle["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(source_bundle["alerts"][0]["uid"], json!("cpu-high"));
    assert_eq!(
        source_bundle["alerts"][0]["body"]["datasourceUids"][0],
        json!("prom-main")
    );
}

#[test]
fn build_sync_source_bundle_document_includes_alert_contract() {
    let source_bundle = build_sync_source_bundle_document(
        &[
            json!({
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "body": {"datasourceUids": ["prom-main"]},
            }),
            json!({
                "kind": "dashboard",
                "uid": "logs-main",
                "title": "Logs Main",
                "body": {"datasourceUids": ["loki-main"]},
            }),
        ],
        &[json!({
            "kind": "datasource",
            "uid": "prom-main",
            "name": "Prometheus Main",
            "title": "Prometheus Main",
            "body": {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
        })],
        &[],
        &[],
        Some(&json!({
            "rules": [{
                "sourcePath": "rules/infra/cpu-high.json",
                "document": {
                    "kind": "grafana-alert-rule",
                    "metadata": {
                        "uid": "cpu-high",
                        "title": "CPU High",
                    },
                    "spec": {
                        "uid": "cpu-high",
                        "title": "CPU High",
                    },
                },
            }],
            "contactPoints": [{"uid": "pagerduty-primary", "name": "PagerDuty Primary"}],
            "summary": {"ruleCount": 1, "contactPointCount": 1},
        })),
        None,
    )
    .unwrap();

    let contract = &source_bundle["alertContract"];
    assert_eq!(contract["kind"], json!("grafana-utils-sync-alert-contract"));
    assert_eq!(contract["summary"]["total"], json!(2));
    assert_eq!(contract["summary"]["safeForSync"], json!(2));
    assert_eq!(contract["resources"].as_array().unwrap().len(), 2);
}

#[test]
fn build_sync_bundle_preflight_document_falls_back_to_alerting_rule_documents() {
    let source_bundle = json!({
        "dashboards": [],
        "datasources": [],
        "folders": [],
        "alerting": {
            "rules": [
                {
                    "sourcePath": "rules/infra/cpu-high.json",
                    "document": {
                        "kind": "grafana-alert-rule",
                        "metadata": {
                            "uid": "cpu-high",
                            "title": "CPU High"
                        },
                        "spec": {
                            "uid": "cpu-high",
                            "title": "CPU High",
                            "folderUID": "infra",
                            "ruleGroup": "CPU Alerts",
                            "condition": "A",
                            "data": [
                                {
                                    "refId": "A",
                                    "datasourceUid": "prom-main",
                                    "datasourceName": "Prometheus Main"
                                }
                            ],
                            "notificationSettings": {
                                "receiver": "pagerduty-primary"
                            }
                        }
                    }
                }
            ]
        }
    });
    let availability = json!({
        "pluginIds": [],
        "datasourceUids": [],
        "datasourceNames": [],
        "contactPoints": [],
        "providerNames": []
    });

    let document =
        build_sync_bundle_preflight_document(&source_bundle, &json!({}), Some(&availability))
            .unwrap();

    assert_eq!(document["summary"]["resourceCount"], json!(1));
    assert!(document["syncPreflight"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-datasource"
            && item["identity"] == "cpu-high->prom-main"
            && item["status"] == "missing"));
    assert!(document["syncPreflight"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point"
            && item["identity"] == "cpu-high->pagerduty-primary"
            && item["status"] == "missing"));
}

#[test]
fn build_sync_bundle_preflight_document_counts_top_level_alert_specs_from_source_bundle() {
    let source_bundle = build_sync_source_bundle_document(
        &[json!({
            "kind": "dashboard",
            "uid": "cpu-main",
            "title": "CPU Main",
            "body": {"datasourceUids": ["prom-main"]},
        })],
        &[json!({
            "kind": "datasource",
            "uid": "prom-main",
            "name": "Prometheus Main",
            "title": "Prometheus Main",
            "body": {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
        })],
        &[],
        &[json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "contactPoints", "datasourceUids"],
            "body": {
                "condition": "A",
                "contactPoints": ["pagerduty-primary"],
                "datasourceUids": ["prom-main"]
            }
        })],
        Some(&json!({
            "rules": [{
                "sourcePath": "rules/cpu-high.json",
                "document": {
                    "groups": [{
                        "name": "CPU Alerts",
                        "rules": [{"uid": "cpu-high", "title": "CPU High"}]
                    }]
                }
            }],
            "summary": {"ruleCount": 1}
        })),
        None,
    )
    .unwrap();
    let target_inventory = json!({"dashboards": [], "datasources": []});
    let availability = json!({
        "pluginIds": ["prometheus"],
        "datasourceUids": ["prom-main"],
        "datasourceNames": [],
        "contactPoints": ["pagerduty-primary"],
        "providerNames": []
    });

    let document = build_sync_bundle_preflight_document(
        &source_bundle,
        &target_inventory,
        Some(&availability),
    )
    .unwrap();

    assert_eq!(document["summary"]["resourceCount"], json!(3));
    assert!(document["syncPreflight"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-datasource"
            && item["identity"] == "cpu-high->prom-main"
            && item["status"] == "ok"));
    assert!(document["syncPreflight"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point"
            && item["identity"] == "cpu-high->pagerduty-primary"
            && item["status"] == "ok"));
}
