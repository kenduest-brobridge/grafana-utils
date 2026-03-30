//! Project-status contract regressions kept separate from command wiring.

use crate::common::TOOL_VERSION;
use crate::project_status::{
    status_finding, ProjectDomainStatus, ProjectStatus, ProjectStatusAction,
    ProjectStatusFreshness, ProjectStatusOverall, ProjectStatusRankedFinding,
    PROJECT_STATUS_BLOCKED, PROJECT_STATUS_READY,
};
use crate::project_status_command::{
    execute_project_status_staged, render_project_status_text, ProjectStatusOutputFormat,
    ProjectStatusStagedArgs,
};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

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

fn empty_live_project_status() -> ProjectStatus {
    ProjectStatus {
        schema_version: 1,
        tool_version: TOOL_VERSION.to_string(),
        scope: "live".to_string(),
        overall: ProjectStatusOverall {
            status: PROJECT_STATUS_READY.to_string(),
            domain_count: 0,
            present_count: 0,
            blocked_count: 0,
            blocker_count: 0,
            warning_count: 0,
            freshness: ProjectStatusFreshness {
                status: "unknown".to_string(),
                source_count: 0,
                newest_age_seconds: None,
                oldest_age_seconds: None,
            },
        },
        domains: Vec::new(),
        top_blockers: Vec::new(),
        next_actions: Vec::new(),
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

fn write_change_desired_fixture(path: &Path) {
    fs::write(
        path,
        serde_json::to_string_pretty(&json!([
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json"
            },
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "Prometheus Main",
                "body": {"type": "prometheus"},
                "sourcePath": "datasources/prom-main.json"
            },
            {
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "body": {
                    "folderUid": "ops",
                    "datasourceUids": ["prom-main"],
                    "datasourceNames": ["Prometheus Main"]
                },
                "sourcePath": "dashboards/cpu-main.json"
            },
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition"],
                "body": {"condition": "A > 90"},
                "sourcePath": "alerts/cpu-high.json"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
}

fn staged_args(desired_file: PathBuf) -> ProjectStatusStagedArgs {
    ProjectStatusStagedArgs {
        dashboard_export_dir: None,
        datasource_export_dir: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: Some(desired_file),
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output: ProjectStatusOutputFormat::Text,
    }
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

#[test]
fn project_status_live_text_renderer_skips_empty_blocker_and_action_sections() {
    let lines = render_project_status_text(&empty_live_project_status());

    assert_eq!(
        lines,
        vec![
            "Project status".to_string(),
            "Overall: status=ready scope=live domains=0 present=0 blocked=0 blockers=0 warnings=0 freshness=unknown"
                .to_string(),
        ]
    );
}

#[test]
fn project_status_live_text_renderer_limits_top_sections_to_five_items() {
    let mut status = empty_live_project_status();
    status.top_blockers = (0..6)
        .map(|index| ProjectStatusRankedFinding {
            domain: format!("domain-{index}"),
            kind: format!("kind-{index}"),
            count: 6 - index,
            source: format!("source-{index}"),
        })
        .collect();
    status.next_actions = (0..6)
        .map(|index| ProjectStatusAction {
            domain: format!("domain-{index}"),
            reason_code: format!("reason-{index}"),
            action: format!("action-{index}"),
        })
        .collect();

    let lines = render_project_status_text(&status);

    assert_eq!(lines[2], "Top blockers:");
    assert_eq!(
        &lines[3..8],
        [
            "- domain-0 kind-0 count=6 source=source-0",
            "- domain-1 kind-1 count=5 source=source-1",
            "- domain-2 kind-2 count=4 source=source-2",
            "- domain-3 kind-3 count=3 source=source-3",
            "- domain-4 kind-4 count=2 source=source-4",
        ]
    );
    assert!(!lines.iter().any(|line| line.contains("domain-5")));
    assert_eq!(lines[8], "Next actions:");
    assert_eq!(
        &lines[9..14],
        [
            "- domain-0 reason=reason-0 action=action-0",
            "- domain-1 reason=reason-1 action=action-1",
            "- domain-2 reason=reason-2 action=action-2",
            "- domain-3 reason=reason-3 action=action-3",
            "- domain-4 reason=reason-4 action=action-4",
        ]
    );
    assert!(!lines.iter().any(|line| line.contains("action-5")));
}

#[test]
fn project_status_staged_document_serializes_the_shared_contract_shape() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    write_change_desired_fixture(&desired_file);

    let status = execute_project_status_staged(&staged_args(desired_file)).unwrap();
    let document = serde_json::to_value(status).unwrap();

    assert_project_status_document_shape(&document);
    assert_eq!(document["schemaVersion"], json!(1));
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(document["scope"], json!("staged-only"));
    assert_eq!(document["overall"]["status"], json!("partial"));
    assert_eq!(document["overall"]["domainCount"], json!(6));
    assert_eq!(document["overall"]["presentCount"], json!(1));
    assert_eq!(document["overall"]["blockedCount"], json!(0));
    assert_eq!(document["overall"]["blockerCount"], json!(0));
    assert_eq!(document["overall"]["warningCount"], json!(0));
    assert_eq!(document["overall"]["freshness"]["status"], json!("current"));
    assert_eq!(document["overall"]["freshness"]["sourceCount"], json!(1));
    assert_eq!(document["domains"].as_array().unwrap().len(), 1);
    assert_eq!(document["domains"][0]["id"], json!("sync"));
    assert_eq!(document["domains"][0]["scope"], json!("staged"));
    assert_eq!(document["domains"][0]["mode"], json!("staged-documents"));
    assert_eq!(
        document["domains"][0]["status"],
        json!(PROJECT_STATUS_READY)
    );
    assert_eq!(
        document["domains"][0]["reasonCode"],
        json!(PROJECT_STATUS_READY)
    );
    assert_eq!(document["topBlockers"], json!([]));
    assert_eq!(
        document["nextActions"],
        json!([
            {
                "domain": "sync",
                "reasonCode": "ready",
                "action": "re-run sync summary after staged changes"
            }
        ])
    );
}

#[test]
fn project_status_staged_text_renderer_matches_the_shared_contract_fields() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    write_change_desired_fixture(&desired_file);

    let status = execute_project_status_staged(&staged_args(desired_file)).unwrap();
    let lines = render_project_status_text(&status);

    assert_eq!(
        lines,
        vec![
            "Project status".to_string(),
            "Overall: status=partial scope=staged-only domains=6 present=1 blocked=0 blockers=0 warnings=0 freshness=current"
                .to_string(),
            "Domains:".to_string(),
            "- sync status=ready mode=staged-documents primary=4 blockers=0 warnings=0 freshness=current next=re-run sync summary after staged changes"
                .to_string(),
            "Next actions:".to_string(),
            "- sync reason=ready action=re-run sync summary after staged changes".to_string(),
        ]
    );
}
