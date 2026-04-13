//! Sync bundle preflight summary, rendering, and provider checks.
use super::super::build_alert_replay_artifact_fixture;
use crate::sync::bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
    SyncBundlePreflightSummary, SYNC_BUNDLE_PREFLIGHT_KIND,
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
        "providerNames": [],
        "secretPlaceholderNames": []
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
        document["summary"]["secretPlaceholderBlockingCount"],
        json!(0)
    );
    assert_eq!(
        document["providerAssessment"]["plans"][0]["providerKind"],
        json!("external-provider-reference")
    );
}

#[test]
fn sync_bundle_preflight_summary_reads_counts_from_document() {
    let document = build_sync_bundle_preflight_document(
        &json!({
            "folders": [
                {"kind": "folder", "uid": "ops", "title": "Operations"}
            ]
        }),
        &json!({}),
        None,
    )
    .unwrap();

    let summary = SyncBundlePreflightSummary::from_document(&document).unwrap();

    assert_eq!(summary.resource_count, 1);
    assert_eq!(summary.sync_blocking_count, 0);
    assert_eq!(summary.provider_blocking_count, 0);
    assert_eq!(summary.secret_placeholder_blocking_count, 0);
    assert_eq!(summary.alert_artifact_count, 0);
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
    assert!(output.contains("Secret placeholders: 0 datasources, 0 references, 0 blocking"));
    assert!(output.contains("Alert artifacts: 0 total"));
    assert!(output.contains("Sync blocking:"));
    assert!(output.contains("Provider blocking:"));
    assert!(output.contains("Secret placeholders blocking: 0"));
    assert!(
        output.contains("Reason: missing provider or secret placeholder availability blocks apply")
    );
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
        "secretPlaceholderNames": ["loki-basic-auth"],
    });

    let document = build_sync_bundle_preflight_document(
        &source_bundle,
        &target_inventory,
        Some(&availability),
    )
    .unwrap();

    assert_eq!(document["summary"]["providerBlockingCount"], json!(0));
    assert_eq!(
        document["summary"]["secretPlaceholderBlockingCount"],
        json!(0)
    );
    assert_eq!(
        document["providerAssessment"]["plans"][0]["providers"][0]["providerName"],
        json!("vault")
    );
    assert_eq!(
        document["secretPlaceholderAssessment"]["plans"][0]["placeholderNames"][0],
        json!("loki-basic-auth")
    );
    assert_eq!(
        document["secretPlaceholderAssessment"]["checks"][0]["detail"],
        json!(
            "Datasource secret placeholder is available for staged review via secretPlaceholderNames availability input."
        )
    );
}

#[test]
fn build_sync_bundle_preflight_document_blocks_missing_secret_placeholder_availability() {
    let source_bundle = build_sync_source_bundle_document(
        &[],
        &[json!({
            "kind": "datasource",
            "uid": "loki-main",
            "name": "Loki Main",
            "title": "Loki Main",
            "body": {"uid": "loki-main", "name": "Loki Main", "type": "loki"},
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

    let document = build_sync_bundle_preflight_document(
        &source_bundle,
        &json!({"datasources": []}),
        Some(&json!({
            "pluginIds": ["loki"],
            "datasourceUids": [],
            "datasourceNames": [],
            "contactPoints": [],
            "providerNames": [],
            "secretPlaceholderNames": [],
        })),
    )
    .unwrap();

    assert_eq!(
        document["summary"]["secretPlaceholderBlockingCount"],
        json!(1)
    );
    assert_eq!(
        document["secretPlaceholderAssessment"]["plans"][0]["providerKind"],
        json!("inline-placeholder-map")
    );
    assert!(document["secretPlaceholderAssessment"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "secret-placeholder"
            && item["identity"] == "loki-main->loki-basic-auth"
            && item["status"] == "missing"));
    assert_eq!(
        document["secretPlaceholderAssessment"]["checks"][0]["detail"],
        json!(
            "Datasource secret placeholder is not available in secretPlaceholderNames availability input."
        )
    );
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
fn build_sync_bundle_preflight_document_reports_non_rule_alert_export_artifacts_from_source_bundle()
{
    let source_bundle = json!({
        "dashboards": [],
        "datasources": [],
        "folders": [],
        "alerting": build_alert_replay_artifact_fixture(false)
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
fn build_sync_bundle_preflight_document_reports_alert_replay_artifacts_and_keeps_sync_checks_zero()
{
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

    assert_eq!(document["summary"]["alertArtifactCount"], json!(4));
    assert_eq!(document["summary"]["alertArtifactPlanOnlyCount"], json!(1));
    assert_eq!(document["summary"]["alertArtifactBlockedCount"], json!(3));
    assert_eq!(
        document["alertArtifactAssessment"]["summary"]["contactPointCount"],
        json!(1)
    );
    assert_eq!(
        document["alertArtifactAssessment"]["summary"]["muteTimingCount"],
        json!(1)
    );
    assert_eq!(
        document["alertArtifactAssessment"]["summary"]["policyCount"],
        json!(1)
    );
    assert_eq!(
        document["alertArtifactAssessment"]["summary"]["templateCount"],
        json!(1)
    );
    assert!(document["alertArtifactAssessment"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point"
            && item["identity"] == "smoke-webhook"
            && item["status"] == "plan-only"));
    assert!(document["alertArtifactAssessment"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-mute-timing"
            && item["identity"] == "Off Hours"
            && item["status"] == "blocked"));
    assert!(document["alertArtifactAssessment"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-policy"
            && item["identity"] == "grafana-default-email"
            && item["status"] == "blocked"));
    assert!(document["alertArtifactAssessment"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-template"
            && item["identity"] == "slack.default"
            && item["status"] == "blocked"));
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
