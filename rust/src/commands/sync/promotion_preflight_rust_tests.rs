//! Sync promotion-preflight contract and render coverage.
use crate::common::TOOL_VERSION;
use crate::sync::promotion_preflight::{
    build_sync_promotion_preflight_document, render_sync_promotion_preflight_text,
    SyncPromotionPreflightSummary, SYNC_PROMOTION_MAPPING_KIND,
    SYNC_PROMOTION_MAPPING_SCHEMA_VERSION, SYNC_PROMOTION_PREFLIGHT_KIND,
};
use serde_json::json;

#[test]
fn build_sync_promotion_preflight_document_reports_direct_mapped_and_missing_references() {
    let source_bundle = json!({
        "kind": "grafana-utils-sync-source-bundle",
        "summary": {
            "dashboardCount": 1,
            "datasourceCount": 1,
            "folderCount": 1,
            "alertRuleCount": 1,
            "contactPointCount": 0,
            "muteTimingCount": 0,
            "policyCount": 0,
            "templateCount": 0
        },
        "dashboards": [{
            "kind": "dashboard",
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "ops-src",
            "body": {
                "datasourceUids": ["prom-src"],
                "datasourceNames": ["Prometheus Source"]
            }
        }],
        "datasources": [{
            "kind": "datasource",
            "uid": "prom-src",
            "name": "Prometheus Source",
            "body": {"uid": "prom-src", "name": "Prometheus Source", "type": "prometheus"}
        }],
        "folders": [{"kind": "folder", "uid": "ops-src", "title": "Operations"}],
        "alerts": [{
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["datasourceUids", "datasourceNames"],
            "body": {
                "datasourceUids": ["loki-src"],
                "datasourceNames": ["Loki Source"]
            }
        }],
        "alerting": {"summary": {}},
        "metadata": {}
    });
    let target_inventory = json!({
        "folders": [{"kind": "folder", "uid": "ops-dst", "title": "Operations"}],
        "datasources": [
            {"uid": "prom-dst", "name": "Prometheus Prod"},
            {"uid": "loki-dst", "name": "Loki Prod"}
        ]
    });
    let mapping = json!({
        "kind": SYNC_PROMOTION_MAPPING_KIND,
        "schemaVersion": SYNC_PROMOTION_MAPPING_SCHEMA_VERSION,
        "metadata": {
            "sourceEnvironment": "staging",
            "targetEnvironment": "prod"
        },
        "folders": {"ops-src": "ops-dst"},
        "datasources": {
            "uids": {"prom-src": "prom-dst"},
            "names": {"Prometheus Source": "Prometheus Prod"}
        }
    });
    let availability = json!({
        "pluginIds": ["prometheus", "loki"],
        "datasourceUids": ["prom-dst", "loki-dst"],
        "datasourceNames": ["Prometheus Prod", "Loki Prod"],
        "contactPoints": []
    });

    let document = build_sync_promotion_preflight_document(
        &source_bundle,
        &target_inventory,
        Some(&availability),
        Some(&mapping),
    )
    .unwrap();

    assert_eq!(document["kind"], json!(SYNC_PROMOTION_PREFLIGHT_KIND));
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(document["summary"]["mappedCount"], json!(3));
    assert_eq!(document["summary"]["missingMappingCount"], json!(2));
    assert_eq!(document["summary"]["bundleBlockingCount"], json!(5));
    assert_eq!(document["summary"]["blockingCount"], json!(7));
    assert_eq!(document["handoffSummary"]["reviewRequired"], json!(true));
    assert_eq!(document["handoffSummary"]["readyForReview"], json!(false));
    assert_eq!(
        document["handoffSummary"]["nextStage"],
        json!("resolve-blockers")
    );
    assert_eq!(document["handoffSummary"]["blockingCount"], json!(7));
    assert_eq!(
        document["handoffSummary"]["reviewInstruction"],
        json!("promotion handoff is blocked until the listed remaps and bundle issues are cleared")
    );
    assert_eq!(document["continuationSummary"]["stagedOnly"], json!(true));
    assert_eq!(
        document["continuationSummary"]["liveMutationAllowed"],
        json!(false)
    );
    assert_eq!(
        document["continuationSummary"]["readyForContinuation"],
        json!(false)
    );
    assert_eq!(
        document["continuationSummary"]["nextStage"],
        json!("resolve-blockers")
    );
    assert_eq!(document["continuationSummary"]["resolvedCount"], json!(3));
    assert_eq!(document["continuationSummary"]["blockingCount"], json!(7));
    assert_eq!(
        document["continuationSummary"]["continuationInstruction"],
        json!(
            "keep the promotion staged until blockers clear; do not enter the apply continuation"
        )
    );
    assert_eq!(
        document["mappingSummary"]["mappingKind"],
        json!(SYNC_PROMOTION_MAPPING_KIND)
    );
    assert_eq!(
        document["mappingSummary"]["sourceEnvironment"],
        json!("staging")
    );
    assert_eq!(
        document["mappingSummary"]["targetEnvironment"],
        json!("prod")
    );
    assert_eq!(document["checkSummary"]["folderRemapCount"], json!(1));
    assert_eq!(document["checkSummary"]["resolvedCount"], json!(3));
    assert_eq!(
        document["checkSummary"]["datasourceUidRemapCount"],
        json!(2)
    );
    assert_eq!(
        document["checkSummary"]["datasourceNameRemapCount"],
        json!(2)
    );
    assert_eq!(document["checkSummary"]["mappedCount"], json!(3));
    assert_eq!(document["checkSummary"]["missingTargetCount"], json!(2));
    assert_eq!(document["resolvedChecks"].as_array().unwrap().len(), 3);
    assert_eq!(document["blockingChecks"].as_array().unwrap().len(), 2);
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "folder-remap"
            && item["resolution"] == "explicit-map"
            && item["mappingSource"] == "folders"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-datasource-uid-remap"
            && item["status"] == "missing-target"));
    assert!(document["resolvedChecks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["blocking"] == json!(false)));
    assert!(document["blockingChecks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["blocking"] == json!(true)));
}

#[test]
fn sync_promotion_preflight_summary_reads_counts_from_document() {
    let document = json!({
        "kind": SYNC_PROMOTION_PREFLIGHT_KIND,
        "summary": {
            "resourceCount": 3,
            "directMatchCount": 1,
            "mappedCount": 1,
            "missingMappingCount": 1,
            "bundleBlockingCount": 2,
            "blockingCount": 3
        }
    });

    let summary = SyncPromotionPreflightSummary::from_document(&document).unwrap();

    assert_eq!(summary.resource_count, 3);
    assert_eq!(summary.direct_match_count, 1);
    assert_eq!(summary.mapped_count, 1);
    assert_eq!(summary.blocking_count, 3);
}

#[test]
fn render_sync_promotion_preflight_text_renders_summary_and_bundle_context() {
    let document = json!({
        "kind": SYNC_PROMOTION_PREFLIGHT_KIND,
        "summary": {
            "resourceCount": 3,
            "directMatchCount": 1,
            "mappedCount": 1,
            "missingMappingCount": 1,
            "bundleBlockingCount": 0,
            "blockingCount": 1
        },
        "mappingSummary": {
            "mappingKind": SYNC_PROMOTION_MAPPING_KIND,
            "mappingSchemaVersion": 1,
            "sourceEnvironment": "staging",
            "targetEnvironment": "prod",
            "folderMappingCount": 1,
            "datasourceUidMappingCount": 1,
            "datasourceNameMappingCount": 0
        },
        "checkSummary": {
            "folderRemapCount": 1,
            "datasourceUidRemapCount": 0,
            "datasourceNameRemapCount": 0,
            "resolvedCount": 1,
            "directCount": 0,
            "mappedCount": 1,
            "missingTargetCount": 1
        },
        "handoffSummary": {
            "reviewRequired": true,
            "readyForReview": false,
            "nextStage": "resolve-blockers",
            "blockingCount": 1,
            "reviewInstruction": "promotion handoff is blocked until the listed remaps and bundle issues are cleared"
        },
        "continuationSummary": {
            "stagedOnly": true,
            "liveMutationAllowed": false,
            "readyForContinuation": false,
            "nextStage": "resolve-blockers",
            "resolvedCount": 1,
            "blockingCount": 1,
            "continuationInstruction": "keep the promotion staged until blockers clear; do not enter the apply continuation"
        },
        "resolvedChecks": [{
            "kind": "folder-remap",
            "identity": "cpu-main",
            "sourceValue": "ops-src",
            "targetValue": "ops-dst",
            "resolution": "explicit-map",
            "mappingSource": "folders",
            "status": "mapped",
            "detail": "Promotion mapping resolves this source identifier onto the target inventory.",
            "blocking": false
        }],
        "blockingChecks": [{
            "kind": "alert-datasource-uid-remap",
            "identity": "cpu-high",
            "sourceValue": "loki-src",
            "targetValue": "",
            "resolution": "missing-map",
            "mappingSource": "datasources.uids",
            "status": "missing-target",
            "detail": "Alert datasource UID is missing from the target inventory and has no valid promotion mapping.",
            "blocking": true
        }],
        "bundlePreflight": {
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 1,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactCount": 0,
                "alertArtifactBlockingCount": 0,
                "alertArtifactPlanOnlyCount": 0
            }
        }
    });

    let output = render_sync_promotion_preflight_text(&document)
        .unwrap()
        .join("\n");

    assert!(output.contains("Sync promotion preflight"));
    assert!(output.contains("missing-mappings=1"));
    assert!(output.contains("source-env=staging"));
    assert!(output.contains("target-env=prod"));
    assert!(output.contains("folder-remaps=1"));
    assert!(output.contains("resolved-remaps=1"));
    assert!(output.contains("blocking-remaps=1"));
    assert!(output.contains("mapped=1"));
    assert!(output.contains("folders=1"));
    assert!(output.contains("promotion stays blocked"));
    assert!(output.contains("# Controlled apply continuation"));
    assert!(output.contains("staged-only=true"));
    assert!(output.contains("ready-for-continuation=false"));
    assert!(output.contains(
        "Handoff: review-required=true ready-for-review=false next-stage=resolve-blockers blocking=1 instruction=promotion handoff is blocked until the listed remaps and bundle issues are cleared"
    ));
    assert!(output.contains("# Resolved remaps"));
    assert!(output.contains("# Blocking remaps"));
    assert!(output.contains("resolution=explicit-map"));
    assert!(output.contains("mapping-source=folders"));
    assert!(output.contains("status=missing-target"));
    assert!(output.contains("Sync bundle preflight summary"));
    assert!(output.contains("Secret placeholders blocking: 0"));
}

#[test]
fn build_sync_promotion_preflight_document_reports_review_handoff_when_clean() {
    let source_bundle = json!({
        "kind": "grafana-utils-sync-source-bundle",
        "summary": {
            "dashboardCount": 1,
            "datasourceCount": 0,
            "folderCount": 1,
            "alertRuleCount": 0,
            "contactPointCount": 0,
            "muteTimingCount": 0,
            "policyCount": 0,
            "templateCount": 0
        },
        "dashboards": [{
            "kind": "dashboard",
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "ops-src",
            "body": {}
        }],
        "datasources": [],
        "folders": [{"kind": "folder", "uid": "ops-src", "title": "Operations"}],
        "alerts": [],
        "alerting": {"summary": {}},
        "metadata": {}
    });
    let target_inventory = json!({
        "folders": [{"kind": "folder", "uid": "ops-dst", "title": "Operations"}],
        "datasources": []
    });
    let mapping = json!({
        "kind": SYNC_PROMOTION_MAPPING_KIND,
        "schemaVersion": SYNC_PROMOTION_MAPPING_SCHEMA_VERSION,
        "metadata": {
            "sourceEnvironment": "staging",
            "targetEnvironment": "prod"
        },
        "folders": {"ops-src": "ops-dst"},
        "datasources": {}
    });

    let document = build_sync_promotion_preflight_document(
        &source_bundle,
        &target_inventory,
        None,
        Some(&mapping),
    )
    .unwrap();

    assert_eq!(document["summary"]["blockingCount"], json!(0));
    assert_eq!(document["handoffSummary"]["reviewRequired"], json!(true));
    assert_eq!(document["handoffSummary"]["readyForReview"], json!(true));
    assert_eq!(document["handoffSummary"]["nextStage"], json!("review"));
    assert_eq!(document["handoffSummary"]["blockingCount"], json!(0));
    assert_eq!(
        document["handoffSummary"]["reviewInstruction"],
        json!("promotion handoff is ready to move into review")
    );
    assert_eq!(document["continuationSummary"]["stagedOnly"], json!(true));
    assert_eq!(
        document["continuationSummary"]["liveMutationAllowed"],
        json!(false)
    );
    assert_eq!(
        document["continuationSummary"]["readyForContinuation"],
        json!(true)
    );
    assert_eq!(
        document["continuationSummary"]["nextStage"],
        json!("staged-apply-continuation")
    );
    assert_eq!(document["continuationSummary"]["resolvedCount"], json!(1));
    assert_eq!(document["continuationSummary"]["blockingCount"], json!(0));
    assert_eq!(
        document["continuationSummary"]["continuationInstruction"],
        json!("reviewed remaps can continue into a staged apply continuation without enabling live mutation")
    );
}

#[test]
fn build_sync_promotion_preflight_document_rejects_unknown_mapping_kind() {
    let error = build_sync_promotion_preflight_document(
        &json!({"dashboards": [], "datasources": [], "folders": [], "alerts": [], "alerting": {}, "summary": {}}),
        &json!({"folders": [], "datasources": []}),
        None,
        Some(&json!({"kind": "wrong-kind"})),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("mapping input kind is not supported"));
}
