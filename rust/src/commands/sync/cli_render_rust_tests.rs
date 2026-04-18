//! Sync CLI render helper regressions.
//! Keeps the pure text renderer coverage separate from the CLI execution tests.
use super::{
    render_alert_sync_assessment_text, render_sync_apply_intent_text, render_sync_plan_text,
    render_sync_summary_text,
};
use serde_json::json;

#[test]
fn render_sync_summary_text_renders_counts() {
    let lines = render_sync_summary_text(&json!({
        "kind": "grafana-utils-sync-summary",
        "summary": {
            "resourceCount": 3,
            "dashboardCount": 1,
            "datasourceCount": 1,
            "folderCount": 1,
            "alertCount": 0
        }
    }))
    .unwrap();

    assert_eq!(lines[0], "Sync summary");
    assert!(lines[1].contains("3 total"));
}

#[test]
fn render_alert_sync_assessment_text_renders_status_lines() {
    let lines = render_alert_sync_assessment_text(&json!({
        "kind": "grafana-utils-alert-sync-plan",
        "summary": {
            "alertCount": 1,
            "candidateCount": 0,
            "planOnlyCount": 1,
            "blockedCount": 0
        },
        "alerts": [
            {
                "identity": "cpu-high",
                "status": "plan-only",
                "liveApplyAllowed": false,
                "detail": "detail text"
            }
        ]
    }))
    .unwrap();

    assert_eq!(lines[0], "Alert sync assessment");
    assert!(lines[1].contains("plan-only"));
    assert!(lines[4].contains("cpu-high"));
}

#[test]
fn render_sync_plan_text_renders_counts() {
    let lines = render_sync_plan_text(&json!({
        "kind": "grafana-utils-sync-plan",
        "stage": "review",
        "stepIndex": 2,
        "parentTraceId": "sync-trace-demo",
        "summary": {
            "would_create": 1,
            "would_update": 2,
            "would_delete": 0,
            "noop": 3,
            "unmanaged": 1,
            "alert_candidate": 0,
            "alert_plan_only": 1,
            "alert_blocked": 0
        },
        "reviewRequired": true,
        "reviewed": false,
        "traceId": "sync-trace-demo",
        "reviewedBy": "alice",
        "reviewedAt": "staged:sync-trace-demo:reviewed",
        "reviewNote": "manual review complete"
    }))
    .unwrap();

    assert_eq!(lines[0], "Sync plan");
    assert!(lines[1].contains("sync-trace-demo"));
    assert!(lines[2].contains("stage=review"));
    assert!(lines[2].contains("step=2"));
    assert!(lines[2].contains("parent=sync-trace-demo"));
    assert!(lines[3].contains("create=1"));
    assert!(lines[4].contains("plan-only=1"));
    assert!(lines[5].contains("review-only"));
    assert!(lines[6].contains("reviewed=false"));
    assert!(lines[7].contains("alice"));
    assert!(lines[8].contains("staged:sync-trace-demo:reviewed"));
    assert!(lines[9].contains("manual review complete"));
}

#[test]
fn render_sync_apply_intent_text_renders_counts() {
    let lines = render_sync_apply_intent_text(&json!({
        "kind": "grafana-utils-sync-apply-intent",
        "stage": "apply",
        "stepIndex": 3,
        "parentTraceId": "sync-trace-demo",
        "summary": {
            "would_create": 1,
            "would_update": 2,
            "would_delete": 1
        },
        "operations": [
            {"action":"would-create"},
            {"action":"would-update"}
        ],
        "preflightSummary": {
            "kind": "grafana-utils-sync-preflight",
            "checkCount": 4,
            "okCount": 4,
            "blockingCount": 0
        },
        "bundlePreflightSummary": {
            "kind": "grafana-utils-sync-bundle-preflight",
            "resourceCount": 4,
            "syncBlockingCount": 0,
            "providerBlockingCount": 0,
            "secretPlaceholderBlockingCount": 1
        },
        "reviewRequired": true,
        "approved": true,
        "reviewed": true,
        "traceId": "sync-trace-demo",
        "appliedBy": "bob",
        "appliedAt": "staged:sync-trace-demo:applied",
        "approvalReason": "change-approved",
        "applyNote": "local apply intent only"
    }))
    .unwrap();

    assert_eq!(lines[0], "Sync apply intent");
    assert!(lines[1].contains("sync-trace-demo"));
    assert!(lines[2].contains("stage=apply"));
    assert!(lines[2].contains("step=3"));
    assert!(lines[2].contains("parent=sync-trace-demo"));
    assert!(lines[3].contains("executable=2"));
    assert!(lines[4].contains("required=true"));
    assert!(lines[4].contains("approved=true"));
    assert!(lines[5].contains("kind=grafana-utils-sync-preflight"));
    assert!(lines[5].contains("blocking=0"));
    assert!(lines[6].contains("sync-blocking=0"));
    assert!(lines[6].contains("secret-placeholder-blocking=1"));
    assert!(
        lines[7].contains("Reason: input-test and package-test blocking must be 0 before apply")
    );
    assert!(lines[8].contains("bob"));
    assert!(lines[9].contains("staged:sync-trace-demo:applied"));
    assert!(lines[10].contains("change-approved"));
    assert!(lines[11].contains("local apply intent only"));
}

#[test]
fn render_sync_plan_text_defaults_lineage_when_missing() {
    let lines = render_sync_plan_text(&json!({
        "kind": "grafana-utils-sync-plan",
        "summary": {
            "would_create": 0,
            "would_update": 0,
            "would_delete": 0,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 0
        },
        "reviewRequired": true,
        "reviewed": false,
        "traceId": "sync-trace-demo"
    }))
    .unwrap();

    assert!(lines[2].contains("stage=missing"));
    assert!(lines[2].contains("step=0"));
    assert!(lines[2].contains("parent=none"));
}

#[test]
fn render_sync_plan_text_shows_domain_summary_and_blocked_reasons() {
    let lines = render_sync_plan_text(&json!({
        "kind": "grafana-utils-sync-plan",
        "traceId": "sync-trace-demo",
        "stage": "plan",
        "stepIndex": 1,
        "summary": {
            "would_create": 1,
            "would_update": 1,
            "would_delete": 1,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 1,
            "blocked_reasons": ["target-read-only"]
        },
        "domains": [
            {"id": "dashboard", "checked": 1},
            {"id": "datasource", "checked": 1},
            {"id": "alert", "checked": 1}
        ],
        "blockedReasons": ["target-read-only"],
        "reviewRequired": true,
        "reviewed": false
    }))
    .unwrap();

    assert!(lines.iter().any(|line| line.contains("Domains:")));
    assert!(lines.iter().any(|line| line.contains("dashboard=1")));
    assert!(lines.iter().any(|line| line.contains("datasource=1")));
    assert!(lines.iter().any(|line| line.contains("alert=1")));
    assert!(lines
        .iter()
        .any(|line| line.contains("Blocked reason: target-read-only")));
}
