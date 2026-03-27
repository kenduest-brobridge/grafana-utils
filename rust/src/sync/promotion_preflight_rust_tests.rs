//! Sync promotion-preflight contract and render coverage.
use crate::sync::promotion_preflight::{
    build_sync_promotion_preflight_document, render_sync_promotion_preflight_text,
    SyncPromotionPreflightSummary, SYNC_PROMOTION_PREFLIGHT_KIND,
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
    assert_eq!(document["summary"]["mappedCount"], json!(3));
    assert_eq!(document["summary"]["missingMappingCount"], json!(2));
    assert_eq!(document["summary"]["bundleBlockingCount"], json!(5));
    assert_eq!(document["summary"]["blockingCount"], json!(7));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-datasource-uid-remap"
            && item["status"] == "missing-target"));
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
            "folderMappingCount": 1,
            "datasourceUidMappingCount": 1,
            "datasourceNameMappingCount": 0
        },
        "checks": [{
            "kind": "folder-remap",
            "identity": "cpu-main",
            "sourceValue": "ops-src",
            "targetValue": "ops-dst",
            "status": "mapped",
            "detail": "Promotion mapping resolves this source identifier onto the target inventory.",
            "blocking": false
        }],
        "bundlePreflight": {
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 1,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
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
    assert!(output.contains("folders=1"));
    assert!(output.contains("promotion stays blocked"));
    assert!(output.contains("Sync bundle preflight summary"));
}
