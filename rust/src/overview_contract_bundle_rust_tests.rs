use super::super::write_change_desired_fixture;
use super::{
    build_overview_artifacts, build_overview_document, render_overview_text, OverviewArgs,
    OverviewOutputFormat,
};
use crate::common::TOOL_VERSION;
use crate::project_status_command::render_project_status_text;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn build_overview_document_and_render_overview_text_for_bundle_preflight_assessment_views() {
    let temp = tempdir().unwrap();

    let source_bundle_file = temp.path().join("source-bundle.json");
    fs::write(
        &source_bundle_file,
        serde_json::to_string_pretty(&json!({
            "dashboards": [
                {
                    "kind": "dashboard",
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "body": {
                        "datasourceUids": ["prom-main"],
                        "datasourceNames": ["Prometheus Main"]
                    }
                }
            ],
            "datasources": [
                {
                    "kind": "datasource",
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "title": "Prometheus Main",
                    "body": {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus"
                    },
                    "secureJsonDataProviders": {
                        "httpHeaderValue1": "${provider:vault:secret/data/prom/token}"
                    },
                    "secureJsonDataPlaceholders": {
                        "basicAuthPassword": "${secret:prom-basic-auth}"
                    }
                }
            ],
            "alerts": [
                {
                    "kind": "alert",
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "managedFields": ["condition", "contactPoints"],
                    "body": {
                        "condition": "A > 90",
                        "contactPoints": ["pagerduty-primary"],
                        "datasourceUids": ["prom-main"],
                        "datasourceNames": ["Prometheus Main"]
                    }
                }
            ],
            "alerting": {
                "contactPoints": [
                    {
                        "sourcePath": "contact-points/PagerDuty_Main/PagerDuty_Main__pagerduty-primary.json",
                        "document": {
                            "kind": "grafana-contact-point",
                            "spec": {
                                "uid": "pagerduty-primary",
                                "name": "PagerDuty Main"
                            }
                        }
                    }
                ],
                "muteTimings": [
                    {
                        "sourcePath": "mute-timings/Off_Hours/Off_Hours.json",
                        "document": {
                            "kind": "grafana-mute-timing",
                            "spec": {
                                "name": "Off Hours"
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
                ],
                "templates": [
                    {
                        "sourcePath": "templates/slack.default.json",
                        "document": {
                            "kind": "grafana-notification-template",
                            "spec": {
                                "name": "slack.default"
                            }
                        }
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let target_inventory_file = temp.path().join("target-inventory.json");
    fs::write(
        &target_inventory_file,
        serde_json::to_string_pretty(&json!({
            "dashboards": [],
            "datasources": [],
            "folders": []
        }))
        .unwrap(),
    )
    .unwrap();

    let availability_file = temp.path().join("availability.json");
    fs::write(
        &availability_file,
        serde_json::to_string_pretty(&json!({
            "pluginIds": [],
            "datasourceUids": [],
            "datasourceNames": [],
            "contactPoints": [],
            "providerNames": [],
            "secretPlaceholderNames": []
        }))
        .unwrap(),
    )
    .unwrap();

    let args = OverviewArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: Some(source_bundle_file),
        target_inventory: Some(target_inventory_file),
        alert_export_dir: None,
        availability_file: Some(availability_file),
        mapping_file: None,
        output_format: OverviewOutputFormat::Json,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();
    let document = build_overview_document(artifacts).unwrap();
    let json_document = serde_json::to_value(&document).unwrap();
    let lines = render_overview_text(&document).unwrap();
    let views = json_document["sections"][0]["views"].as_array().unwrap();
    let view_labels = views
        .iter()
        .map(|view| view["label"].as_str().unwrap())
        .collect::<Vec<&str>>();

    assert_eq!(document.summary.artifact_count, 1);
    assert_eq!(document.summary.bundle_preflight_count, 1);
    assert_eq!(
        json_document["artifacts"][0]["kind"],
        json!("bundle-preflight")
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["status"],
        json!("blocked")
    );
    assert_eq!(
        json_document["projectStatus"]["domains"][0]["id"],
        json!("sync")
    );
    assert_eq!(
        json_document["projectStatus"]["domains"][0]["status"],
        json!("blocked")
    );
    assert_eq!(
        json_document["projectStatus"]["domains"][0]["reasonCode"],
        json!("blocked-by-blockers")
    );
    assert_eq!(
        json_document["projectStatus"]["domains"][0]["primaryCount"],
        json!(3)
    );
    assert_eq!(
        json_document["projectStatus"]["domains"][0]["warningCount"],
        json!(1)
    );
    assert!(json_document["projectStatus"]["domains"][0]["signalKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("summary.providerBlockingCount")));
    assert!(json_document["projectStatus"]["domains"][0]["nextActions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row == "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"));
    assert!(json_document["projectStatus"]["domains"][0]["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["kind"] == json!("provider-blocking") && row["count"] == json!(1)));
    assert!(json_document["projectStatus"]["domains"][0]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["kind"] == json!("alert-artifact-plan-only") && row["count"] == json!(1)));
    assert_eq!(
        view_labels,
        vec![
            "Summary",
            "Blocking Signals",
            "Sync Checks",
            "Secret Providers",
            "Secret Placeholders",
            "Alerting Artifacts",
            "Inputs",
        ]
    );
    assert!(views[2]["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["title"] == json!("cpu-main->prom-main")
            && item["meta"]
                .as_str()
                .unwrap()
                .contains("kind=dashboard-datasource")));
    assert!(views[3]["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["title"] == json!("prom-main->vault")
            && item["meta"].as_str().unwrap().contains("status=missing")));
    assert!(views[4]["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["title"] == json!("prom-main->prom-basic-auth")
            && item["meta"].as_str().unwrap().contains("blocking=true")));
    assert!(views[5]["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["title"] == json!("PagerDuty Main")
            && item["meta"].as_str().unwrap().contains("status=plan-only")));
    assert!(lines
        .iter()
        .any(|line| line.contains("# Sync bundle preflight")));
    assert!(lines.iter().any(|line| line.contains("Status: blocked")));
    assert!(lines
        .iter()
        .any(|line| line.contains("- sync status=blocked reason=blocked-by-blockers")));
    assert!(lines
        .iter()
        .any(|line| line.contains("provider-blocking=1")));
    assert!(lines
        .iter()
        .any(|line| line.contains("secret-placeholder-blocking=1")));
}

#[test]
fn build_overview_document_preserves_the_shared_project_status_render_contract() {
    let temp = tempdir().unwrap();
    let desired_file = temp.path().join("desired.json");
    write_change_desired_fixture(&desired_file);

    let args = OverviewArgs {
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
        output_format: OverviewOutputFormat::Json,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();
    let document = build_overview_document(artifacts).unwrap();
    let project_status_json = serde_json::to_value(&document.project_status).unwrap();
    let project_status_lines = render_project_status_text(&document.project_status);

    assert_eq!(project_status_json["schemaVersion"], json!(1));
    assert_eq!(project_status_json["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(project_status_json["scope"], json!("staged-only"));
    assert!(project_status_json["overall"].is_object());
    assert!(project_status_json["domains"].is_array());
    assert!(project_status_json["topBlockers"].is_array());
    assert!(project_status_json["nextActions"].is_array());
    assert_eq!(project_status_json["overall"]["status"], json!("partial"));
    assert_eq!(project_status_json["overall"]["domainCount"], json!(6));
    assert_eq!(project_status_json["overall"]["presentCount"], json!(1));
    assert_eq!(project_status_json["overall"]["blockedCount"], json!(0));
    assert_eq!(project_status_json["overall"]["blockerCount"], json!(0));
    assert_eq!(project_status_json["overall"]["warningCount"], json!(0));
    assert_eq!(
        project_status_lines,
        vec![
            "Project status".to_string(),
            "Overall: status=partial scope=staged-only domains=6 present=1 blocked=0 blockers=0 warnings=0 freshness=current"
                .to_string(),
            "Signals: sync sources=sync-summary signalKeys=1 blockers=0 warnings=0".to_string(),
            "Decision order:".to_string(),
            "1. sync next=re-run sync summary after staged changes".to_string(),
            "Domains:".to_string(),
            "- sync status=ready mode=staged-documents primary=4 blockers=0 warnings=0 freshness=current next=re-run sync summary after staged changes"
                .to_string(),
            "Next actions:".to_string(),
            "- sync reason=ready action=re-run sync summary after staged changes".to_string(),
        ]
    );
}
