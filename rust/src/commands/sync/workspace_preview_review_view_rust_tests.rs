//! Sync workspace review view regression tests.

use super::review_tui;
use super::workspace_preview_review_view::build_workspace_review_view;
use serde_json::json;

#[test]
fn build_workspace_review_view_normalizes_actions_domains_and_blockers() {
    let document = json!({
        "kind": "grafana-utils-sync-plan",
        "summary": {
            "would_create": 1,
            "would_update": 0,
            "would_delete": 0,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 1
        },
        "operations": [
            {
                "action": "would-create",
                "resourceKind": "folder",
                "identity": "folder-main"
            },
            {
                "action": "blocked-read-only",
                "resourceKind": "dashboard",
                "identity": "blocked-main",
                "status": "blocked",
                "reason": "target-read-only"
            }
        ]
    });

    let view = build_workspace_review_view(&document).unwrap();

    assert_eq!(view.actions.len(), 2);
    assert_eq!(view.actions[0].domain, "folder");
    assert_eq!(view.actions[0].resource_kind, "folder");
    assert_eq!(view.actions[0].status, "ready");
    assert_eq!(view.actions[1].status, "blocked");
    assert_eq!(view.blocked_reasons, vec!["target-read-only"]);
    assert!(view.domains.iter().any(|domain| domain.id == "dashboard"));
    assert!(view.domains.iter().any(|domain| domain.id == "folder"));
    assert_eq!(view.summary.action_count, 2);
    assert_eq!(view.summary.domain_count, 5);
    assert_eq!(view.summary.blocked_count, 1);
}

#[test]
fn filter_review_plan_operations_uses_workspace_review_view_contract() {
    let plan = json!({
        "kind": "grafana-utils-sync-plan",
        "traceId": "sync-trace-review",
        "summary": {
            "would_create": 2,
            "would_update": 1,
            "would_delete": 0,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 1,
            "alert_plan_only": 0,
            "alert_blocked": 0
        },
        "reviewRequired": true,
        "reviewed": false,
        "operations": [
            {"kind":"datasource","identity":"prom-main","action":"would-update"},
            {"kind":"alert-contact-point","identity":"ops-email","action":"would-create"},
            {
                "kind":"dashboard",
                "identity":"blocked-main",
                "action":"blocked-read-only",
                "status":"blocked",
                "reason":"target-read-only"
            }
        ]
    });
    let selected = ["alert-contact-point::ops-email".to_string()]
        .into_iter()
        .collect();

    let filtered = review_tui::filter_review_plan_operations(&plan, &selected).unwrap();

    assert_eq!(filtered["summary"]["would_create"], json!(1));
    assert_eq!(filtered["summary"]["blockedCount"], json!(1));
    assert_eq!(filtered["domains"].as_array().unwrap().len(), 4);
    assert_eq!(filtered["blockedReasons"], json!(["target-read-only"]));
    assert_eq!(filtered["operations"].as_array().unwrap().len(), 2);
}
