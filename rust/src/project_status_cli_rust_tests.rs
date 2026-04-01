//! Project-status contract regressions kept separate from command wiring.

use crate::common::TOOL_VERSION;
use crate::project_status::{
    status_finding, ProjectDomainStatus, ProjectStatus, ProjectStatusAction,
    ProjectStatusFreshness, ProjectStatusOverall, ProjectStatusRankedFinding,
    PROJECT_STATUS_BLOCKED, PROJECT_STATUS_READY,
};
use crate::project_status_command::{
    execute_project_status_staged, render_project_status_text, ProjectStatusCliArgs,
    ProjectStatusOutputFormat, ProjectStatusStagedArgs, ProjectStatusSubcommand,
};
use clap::{CommandFactory, Parser};
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

fn write_dashboard_export_fixture(dir: &Path) {
    fs::create_dir_all(dir.join("General")).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("folders.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "general",
                "title": "General",
                "parentUid": null,
                "path": "General",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("General").join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "panels": []
            },
            "meta": {"folderUid": "general"}
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_datasource_export_fixture(dir: &Path, variant: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": variant,
            "resource": "datasource",
            "datasourcesFile": "datasources.json",
            "indexFile": "index.json",
            "datasourceCount": 2,
            "format": "grafana-datasource-inventory-v1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "true",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "access": "proxy",
                "url": "http://loki:3100",
                "isDefault": "false",
                "org": "Ops Org.",
                "orgId": "2"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
}

fn write_datasource_scope_fixture(dir: &Path, org_id: &str, org_name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": format!("prom-{}", org_id),
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "true",
                "org": org_name,
                "orgId": org_id
            }
        ]))
        .unwrap(),
    )
    .unwrap();
}

fn staged_args(desired_file: PathBuf) -> ProjectStatusStagedArgs {
    ProjectStatusStagedArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: None,
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

#[test]
fn project_status_cli_help_and_parse_support_datasource_provisioning_file() {
    let mut command = ProjectStatusCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("staged")
        .expect("missing staged help");
    let help = subcommand.render_long_help().to_string();
    assert!(help.contains("--dashboard-provisioning-dir"));
    assert!(help.contains("--datasource-provisioning-file"));
    assert!(help.contains("Render project status as table, csv, text, json, yaml"));

    let args = ProjectStatusCliArgs::parse_from([
        "grafana-util",
        "staged",
        "--datasource-provisioning-file",
        "./datasources/provisioning/datasources.yaml",
        "--output",
        "json",
    ]);

    match args.command {
        ProjectStatusSubcommand::Staged(inner) => {
            assert_eq!(
                inner.datasource_provisioning_file,
                Some(Path::new("./datasources/provisioning/datasources.yaml").to_path_buf())
            );
        }
        _ => panic!("expected staged"),
    }
}

#[test]
fn project_status_cli_supports_all_output_modes_for_staged_and_live_commands() {
    for (output, expected) in [
        ("table", ProjectStatusOutputFormat::Table),
        ("csv", ProjectStatusOutputFormat::Csv),
        ("text", ProjectStatusOutputFormat::Text),
        ("json", ProjectStatusOutputFormat::Json),
        ("yaml", ProjectStatusOutputFormat::Yaml),
    ] {
        let staged_args = ProjectStatusCliArgs::parse_from([
            "grafana-util",
            "staged",
            "--desired-file",
            "./desired.json",
            "--output",
            output,
        ]);
        let live_args = ProjectStatusCliArgs::parse_from([
            "grafana-util",
            "live",
            "--url",
            "http://127.0.0.1:3000",
            "--output",
            output,
        ]);

        match staged_args.command {
            ProjectStatusSubcommand::Staged(inner) => {
                assert_eq!(inner.output, expected);
            }
            _ => panic!("expected staged"),
        }

        match live_args.command {
            ProjectStatusSubcommand::Live(inner) => {
                assert_eq!(inner.output, expected);
            }
            _ => panic!("expected live"),
        }
    }

    #[cfg(feature = "tui")]
    {
        let staged_args = ProjectStatusCliArgs::parse_from([
            "grafana-util",
            "staged",
            "--desired-file",
            "./desired.json",
            "--output",
            "interactive",
        ]);
        let live_args = ProjectStatusCliArgs::parse_from([
            "grafana-util",
            "live",
            "--url",
            "http://127.0.0.1:3000",
            "--output",
            "interactive",
        ]);

        match staged_args.command {
            ProjectStatusSubcommand::Staged(inner) => {
                assert_eq!(inner.output, ProjectStatusOutputFormat::Interactive);
            }
            _ => panic!("expected staged"),
        }

        match live_args.command {
            ProjectStatusSubcommand::Live(inner) => {
                assert_eq!(inner.output, ProjectStatusOutputFormat::Interactive);
            }
            _ => panic!("expected live"),
        }
    }
}

#[test]
fn project_status_cli_supports_dashboard_provisioning_dir() {
    let args = ProjectStatusCliArgs::parse_from([
        "grafana-util",
        "staged",
        "--dashboard-provisioning-dir",
        "./dashboards/provisioning",
        "--output",
        "json",
    ]);

    match args.command {
        ProjectStatusSubcommand::Staged(inner) => {
            assert_eq!(
                inner.dashboard_provisioning_dir,
                Some(Path::new("./dashboards/provisioning").to_path_buf())
            );
        }
        _ => panic!("expected staged"),
    }
}

#[test]
fn project_status_cli_supports_combined_dashboard_and_datasource_export_roots() {
    let temp = tempdir().unwrap();
    let dashboard_export_dir = temp.path().join("dashboards");
    let datasource_export_dir = temp.path().join("datasources");
    write_dashboard_export_fixture(&dashboard_export_dir);
    write_datasource_export_fixture(&datasource_export_dir, "all-orgs-root");
    write_datasource_scope_fixture(
        &datasource_export_dir.join("org_1_Main_Org"),
        "1",
        "Main Org.",
    );
    write_datasource_scope_fixture(
        &datasource_export_dir.join("org_2_Ops_Org"),
        "2",
        "Ops Org.",
    );

    let args = ProjectStatusCliArgs::parse_from([
        "grafana-util",
        "staged",
        "--dashboard-export-dir",
        dashboard_export_dir.to_str().unwrap(),
        "--datasource-export-dir",
        datasource_export_dir.to_str().unwrap(),
        "--output",
        "json",
    ]);

    match args.command {
        ProjectStatusSubcommand::Staged(inner) => {
            assert_eq!(inner.dashboard_export_dir, Some(dashboard_export_dir));
            assert_eq!(inner.datasource_export_dir, Some(datasource_export_dir));

            let status = execute_project_status_staged(&inner).unwrap();
            let dashboard_domain = status
                .domains
                .iter()
                .find(|domain| domain.id == "dashboard")
                .expect("dashboard domain");
            let datasource_domain = status
                .domains
                .iter()
                .find(|domain| domain.id == "datasource")
                .expect("datasource domain");

            assert_eq!(status.scope, "staged-only");
            assert_eq!(status.overall.present_count, 2);
            assert_eq!(dashboard_domain.status, PROJECT_STATUS_READY);
            assert_eq!(dashboard_domain.reason_code, PROJECT_STATUS_READY);
            assert_eq!(
                dashboard_domain.source_kinds,
                vec!["dashboard-export".to_string()]
            );
            assert_eq!(datasource_domain.status, PROJECT_STATUS_READY);
            assert_eq!(datasource_domain.reason_code, PROJECT_STATUS_READY);
            assert_eq!(
                datasource_domain.source_kinds,
                vec!["datasource-export".to_string()]
            );
        }
        _ => panic!("expected staged"),
    }
}

#[test]
fn project_status_cli_rejects_dashboard_export_and_provisioning_inputs_together() {
    let args = ProjectStatusCliArgs::try_parse_from([
        "grafana-util",
        "staged",
        "--dashboard-export-dir",
        "./dashboards/raw",
        "--dashboard-provisioning-dir",
        "./dashboards/provisioning",
    ]);

    assert!(args.is_err());
}
