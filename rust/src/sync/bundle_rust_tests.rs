//! Sync bundle preflight contract tests.
//! Validates sync bundle planning output and sync-source dependency assembly.
use super::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
    SYNC_BUNDLE_PREFLIGHT_KIND,
};
use crate::sync::workbench::{build_sync_source_bundle_document, render_sync_source_bundle_text};
use serde_json::json;
use serde_json::Value;

fn load_alert_export_contract_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/alert_export_contract_cases.json"
    ))
    .unwrap()
}

fn load_sync_source_bundle_contract_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/sync_source_bundle_contract_cases.json"
    ))
    .unwrap()
}

fn build_alert_replay_artifact_fixture(include_rules: bool) -> Value {
    let fixture = load_alert_export_contract_fixture();
    let mut summary = fixture["syncAlertingArtifacts"]["summary"].clone();
    let artifacts = fixture["syncAlertingArtifacts"]["artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut alerting = serde_json::Map::new();
    if !include_rules {
        if let Some(summary_object) = summary.as_object_mut() {
            summary_object.insert("ruleCount".to_string(), json!(0));
        }
    }
    alerting.insert("summary".to_string(), summary);
    for artifact in artifacts {
        let section = artifact["section"].as_str().unwrap_or_default();
        if !include_rules && section == "rules" {
            continue;
        }
        let identity_field = artifact["identityField"].as_str().unwrap_or_default();
        let identity = artifact["identity"].clone();
        let source_path = artifact["sourcePath"].clone();
        let kind = match section {
            "rules" => "grafana-alert-rule",
            "contactPoints" => "grafana-contact-point",
            "muteTimings" => "grafana-mute-timing",
            "policies" => "grafana-notification-policies",
            "templates" => "grafana-notification-template",
            _ => "",
        };
        let mut spec = serde_json::Map::new();
        spec.insert(identity_field.to_string(), identity);
        let mut document = serde_json::Map::new();
        document.insert("kind".to_string(), json!(kind));
        document.insert("spec".to_string(), Value::Object(spec));
        let entry = json!({
            "sourcePath": source_path,
            "document": Value::Object(document)
        });
        alerting
            .entry(section.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        alerting
            .get_mut(section)
            .and_then(Value::as_array_mut)
            .unwrap()
            .push(entry);
    }
    Value::Object(alerting)
}

#[test]
fn build_sync_source_bundle_document_matches_cross_domain_summary_contract() {
    let fixture = load_sync_source_bundle_contract_fixture();
    let source_bundle = build_sync_source_bundle_document(
        &[
            json!({"kind": "dashboard", "uid": "cpu-main", "title": "CPU Main"}),
            json!({"kind": "dashboard", "uid": "logs-main", "title": "Logs Main"}),
        ],
        &[json!({
            "kind": "datasource",
            "uid": "prom-main",
            "name": "Prometheus Main",
            "title": "Prometheus Main"
        })],
        &[json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations"
        })],
        &[json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition"],
            "body": {"condition": "A"}
        })],
        Some(&build_alert_replay_artifact_fixture(true)),
        Some(&json!({"bundleLabel": "sync-smoke"})),
    )
    .unwrap();

    assert_eq!(
        source_bundle["summary"],
        fixture["crossDomainSummaryCase"]["summary"]
    );
    assert_eq!(
        source_bundle["dashboards"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        source_bundle["datasources"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(source_bundle["folders"].as_array().map(Vec::len), Some(1));
    assert_eq!(source_bundle["alerts"].as_array().map(Vec::len), Some(1));

    let text = render_sync_source_bundle_text(&source_bundle).unwrap();
    assert_eq!(
        text,
        fixture["crossDomainSummaryCase"]["textLines"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect::<Vec<String>>()
    );
}

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
fn build_sync_bundle_preflight_document_ignores_non_rule_alert_export_artifacts_for_fallback() {
    let source_bundle = json!({
        "dashboards": [],
        "datasources": [],
        "folders": [],
        "alerting": {
            "rules": [],
            "contactPoints": [
                {
                    "sourcePath": "contact-points/Smoke_Webhook/Smoke_Webhook__smoke-webhook.json",
                    "document": {
                        "kind": "grafana-contact-point",
                        "spec": {
                            "uid": "smoke-webhook",
                            "name": "Smoke Webhook",
                            "type": "webhook"
                        }
                    }
                }
            ],
            "policies": [
                {
                    "sourcePath": "policies/notification-policies.json",
                    "document": {
                        "kind": "grafana-notification-policies",
                        "spec": {
                            "receiver": "grafana-default-email"
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

    assert_eq!(document["summary"]["resourceCount"], json!(0));
    assert_eq!(document["syncPreflight"]["checks"], json!([]));
}

#[test]
fn build_sync_source_bundle_document_preserves_alert_replay_artifact_summary_and_paths() {
    let fixture = load_alert_export_contract_fixture();
    let source_bundle = build_sync_source_bundle_document(
        &[],
        &[],
        &[],
        &[],
        Some(&build_alert_replay_artifact_fixture(true)),
        None,
    )
    .unwrap();

    let expected_summary = &fixture["syncAlertingArtifacts"]["summary"];
    assert_eq!(source_bundle["alerting"]["summary"], *expected_summary);
    for artifact in fixture["syncAlertingArtifacts"]["artifacts"]
        .as_array()
        .unwrap_or(&Vec::new())
    {
        let section = artifact["section"].as_str().unwrap_or_default();
        let identity_field = artifact["identityField"].as_str().unwrap_or_default();
        let identity = artifact["identity"].clone();
        let source_path = artifact["sourcePath"].clone();
        let empty_entries = Vec::new();
        let entries = source_bundle["alerting"][section]
            .as_array()
            .unwrap_or(&empty_entries);
        assert!(entries.iter().any(|entry| {
            entry["sourcePath"] == source_path
                && entry["document"]["spec"][identity_field] == identity
        }));
    }
}

#[test]
fn render_sync_source_bundle_text_reports_alert_replay_artifact_counts() {
    let fixture = load_alert_export_contract_fixture();
    let source_bundle = build_sync_source_bundle_document(
        &[],
        &[],
        &[],
        &[],
        Some(&build_alert_replay_artifact_fixture(true)),
        None,
    )
    .unwrap();

    let lines = render_sync_source_bundle_text(&source_bundle).unwrap();
    let expected = format!(
        "Alerting: rules={} contact-points={} mute-timings={} policies={} templates={}",
        fixture["syncAlertingArtifacts"]["summary"]["ruleCount"],
        fixture["syncAlertingArtifacts"]["summary"]["contactPointCount"],
        fixture["syncAlertingArtifacts"]["summary"]["muteTimingCount"],
        fixture["syncAlertingArtifacts"]["summary"]["policyCount"],
        fixture["syncAlertingArtifacts"]["summary"]["templateCount"]
    );
    assert!(lines.iter().any(|line| line == &expected));
}

#[test]
fn build_sync_bundle_preflight_document_ignores_alert_replay_artifacts_but_keeps_zero_checks() {
    let alerting = build_alert_replay_artifact_fixture(false);
    let source_bundle = json!({
        "dashboards": [],
        "datasources": [],
        "folders": [],
        "alerting": alerting
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

    assert_eq!(document["summary"]["resourceCount"], json!(0));
    assert_eq!(document["summary"]["syncBlockingCount"], json!(0));
    assert_eq!(document["syncPreflight"]["checks"], json!([]));
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
