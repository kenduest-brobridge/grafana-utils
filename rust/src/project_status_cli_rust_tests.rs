//! Project-status contract regressions kept separate from command wiring.

use crate::common::TOOL_VERSION;
use crate::project_status::{
    status_finding, ProjectDomainStatus, ProjectStatus, ProjectStatusAction,
    ProjectStatusFreshness, ProjectStatusOverall, ProjectStatusRankedFinding,
    PROJECT_STATUS_BLOCKED, PROJECT_STATUS_READY,
};
use crate::project_status_command::render_project_status_text;
use serde_json::{json, Value};

fn sample_live_project_status() -> ProjectStatus {
    ProjectStatus {
        schema_version: 1,
        tool_version: TOOL_VERSION.to_string(),
        scope: "live".to_string(),
        overall: ProjectStatusOverall {
            status: PROJECT_STATUS_BLOCKED.to_string(),
            domain_count: 2,
            present_count: 2,
            blocked_count: 1,
            blocker_count: 3,
            warning_count: 1,
            freshness: ProjectStatusFreshness {
                status: "current".to_string(),
                source_count: 2,
                newest_age_seconds: Some(30),
                oldest_age_seconds: Some(120),
            },
        },
        domains: vec![
            ProjectDomainStatus {
                id: "dashboard".to_string(),
                scope: "staged".to_string(),
                mode: "inspect-summary".to_string(),
                status: PROJECT_STATUS_READY.to_string(),
                reason_code: PROJECT_STATUS_READY.to_string(),
                primary_count: 4,
                blocker_count: 0,
                warning_count: 1,
                source_kinds: vec!["dashboard-export".to_string()],
                signal_keys: vec![
                    "summary.dashboardCount".to_string(),
                    "summary.queryCount".to_string(),
                ],
                blockers: Vec::new(),
                warnings: vec![status_finding("risk-records", 1, "summary.riskRecordCount")],
                next_actions: vec![
                    "review dashboard governance warnings before promotion or apply".to_string(),
                ],
                freshness: ProjectStatusFreshness {
                    status: "stale".to_string(),
                    source_count: 1,
                    newest_age_seconds: Some(86_400),
                    oldest_age_seconds: Some(86_400),
                },
            },
            ProjectDomainStatus {
                id: "sync".to_string(),
                scope: "staged".to_string(),
                mode: "staged-documents".to_string(),
                status: PROJECT_STATUS_BLOCKED.to_string(),
                reason_code: "blocked-by-blockers".to_string(),
                primary_count: 6,
                blocker_count: 3,
                warning_count: 0,
                source_kinds: vec!["sync-summary".to_string(), "bundle-preflight".to_string()],
                signal_keys: vec![
                    "summary.resourceCount".to_string(),
                    "summary.syncBlockingCount".to_string(),
                    "summary.providerBlockingCount".to_string(),
                    "summary.secretPlaceholderBlockingCount".to_string(),
                    "summary.alertArtifactBlockedCount".to_string(),
                    "summary.alertArtifactPlanOnlyCount".to_string(),
                ],
                blockers: vec![status_finding("sync-blocking", 3, "summary.syncBlockingCount")],
                warnings: Vec::new(),
                next_actions: vec![
                    "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"
                        .to_string(),
                ],
                freshness: ProjectStatusFreshness {
                    status: "current".to_string(),
                    source_count: 2,
                    newest_age_seconds: Some(15),
                    oldest_age_seconds: Some(45),
                },
            },
        ],
        top_blockers: vec![ProjectStatusRankedFinding {
            domain: "sync".to_string(),
            kind: "sync-blocking".to_string(),
            count: 3,
            source: "summary.syncBlockingCount".to_string(),
        }],
        next_actions: vec![ProjectStatusAction {
            domain: "sync".to_string(),
            reason_code: "blocked-by-blockers".to_string(),
            action: "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"
                .to_string(),
        }],
    }
}

fn assert_project_status_document_shape(document: &Value) {
    assert!(document["schemaVersion"].is_i64());
    assert!(document["toolVersion"].is_string());
    assert!(document["scope"].is_string());
    assert!(document["overall"].is_object());
    assert!(document["domains"].is_array());
    assert!(document["topBlockers"].is_array());
    assert!(document["nextActions"].is_array());
}

#[test]
fn project_status_live_document_serializes_the_shared_contract_shape() {
    let document = serde_json::to_value(sample_live_project_status()).unwrap();

    assert_project_status_document_shape(&document);
    assert_eq!(document["schemaVersion"], json!(1));
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(document["scope"], json!("live"));
    assert_eq!(document["overall"]["status"], json!(PROJECT_STATUS_BLOCKED));
    assert_eq!(document["overall"]["domainCount"], json!(2));
    assert_eq!(document["overall"]["presentCount"], json!(2));
    assert_eq!(document["overall"]["blockedCount"], json!(1));
    assert_eq!(document["overall"]["blockerCount"], json!(3));
    assert_eq!(document["overall"]["warningCount"], json!(1));
    assert_eq!(
        document["overall"]["freshness"],
        json!({
            "status": "current",
            "sourceCount": 2,
            "newestAgeSeconds": 30,
            "oldestAgeSeconds": 120,
        })
    );

    assert_eq!(document["domains"][0]["id"], json!("dashboard"));
    assert_eq!(
        document["domains"][0]["status"],
        json!(PROJECT_STATUS_READY)
    );
    assert_eq!(
        document["domains"][0]["reasonCode"],
        json!(PROJECT_STATUS_READY)
    );
    assert_eq!(
        document["domains"][0]["warnings"][0]["kind"],
        json!("risk-records")
    );
    assert_eq!(document["domains"][1]["id"], json!("sync"));
    assert_eq!(
        document["domains"][1]["status"],
        json!(PROJECT_STATUS_BLOCKED)
    );
    assert_eq!(
        document["domains"][1]["blockers"][0]["kind"],
        json!("sync-blocking")
    );
    assert_eq!(
        document["topBlockers"],
        json!([
            {
                "domain": "sync",
                "kind": "sync-blocking",
                "count": 3,
                "source": "summary.syncBlockingCount"
            }
        ])
    );
    assert_eq!(
        document["nextActions"],
        json!([
            {
                "domain": "sync",
                "reasonCode": "blocked-by-blockers",
                "action": "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"
            }
        ])
    );
}

#[test]
fn project_status_live_text_renderer_surfaces_overall_domain_and_action_sections() {
    let lines = render_project_status_text(&sample_live_project_status());
    assert_eq!(
        lines,
        vec![
            "Project status".to_string(),
            "Overall: status=blocked scope=live domains=2 present=2 blocked=1 blockers=3 warnings=1 freshness=current"
                .to_string(),
            "Domains:".to_string(),
            "- dashboard status=ready mode=inspect-summary primary=4 blockers=0 warnings=1 freshness=stale next=review dashboard governance warnings before promotion or apply"
                .to_string(),
            "- sync status=blocked mode=staged-documents primary=6 blockers=3 warnings=0 freshness=current next=resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"
                .to_string(),
            "Top blockers:".to_string(),
            "- sync sync-blocking count=3 source=summary.syncBlockingCount".to_string(),
            "Next actions:".to_string(),
            "- sync reason=blocked-by-blockers action=resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"
                .to_string(),
        ]
    );
}
