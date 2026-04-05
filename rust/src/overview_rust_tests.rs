//! Overview contract and text-rendering regressions.

use super::overview::{
    build_overview_artifacts, build_overview_document, build_overview_summary_rows,
    render_overview_text, OverviewArgs, OverviewCliArgs, OverviewDocument, OverviewOutputFormat,
    OverviewProjectStatus, OverviewProjectStatusFreshness, OverviewProjectStatusOverall,
    OverviewSummary, OVERVIEW_KIND,
};
use crate::common::TOOL_VERSION;
use crate::dashboard::build_dashboard_domain_status;
use crate::overview::run_overview_live;
use crate::project_status_command::render_project_status_text;
use crate::project_status_command::{execute_project_status_live, ProjectStatusLiveArgs};
use crate::tabular_output::{render_summary_csv, render_summary_table};
use clap::{CommandFactory, Parser};
use serde_json::{json, Value};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

fn write_datasource_export_fixture(dir: &Path, variant: &str) {
    write_datasource_export_fixture_with_scope_kind(dir, variant, None);
}

fn write_datasource_export_fixture_with_scope_kind(
    dir: &Path,
    variant: &str,
    scope_kind: Option<&str>,
) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": variant,
            "scopeKind": scope_kind,
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

fn write_datasource_provisioning_fixture(path: &Path) {
    fs::write(
        path,
        r#"apiVersion: 1
datasources:
  - uid: prom-main
    name: Prometheus Main
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    orgId: 1
    isDefault: true
  - uid: loki-main
    name: Loki Main
    type: loki
    access: proxy
    url: http://loki:3100
    orgId: 2
    isDefault: false
"#,
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

fn write_dashboard_root_fixture(dir: &Path) {
    fs::create_dir_all(dir.join("raw")).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": 1,
            "variant": "root",
            "scopeKind": "org-root",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "org": "Main Org.",
            "orgId": "1"
        }))
        .unwrap(),
    )
    .unwrap();
    write_dashboard_export_fixture(&dir.join("raw"));
}

fn write_alert_export_fixture(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("index.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "apiVersion": 1,
            "kind": "grafana-util-alert-export-index",
            "rules": [
                {
                    "kind": "grafana-alert-rule",
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "folderUID": "infra",
                    "ruleGroup": "cpu-alerts",
                    "path": "rules/infra/cpu-alerts/CPU_High__cpu-high.json"
                }
            ],
            "contact-points": [
                {
                    "kind": "grafana-contact-point",
                    "uid": "pagerduty-main",
                    "name": "PagerDuty Main",
                    "type": "pagerduty",
                    "path": "contact-points/PagerDuty_Main/PagerDuty_Main__pagerduty-main.json"
                }
            ],
            "mute-timings": [
                {
                    "kind": "grafana-mute-timing",
                    "name": "Off Hours",
                    "path": "mute-timings/Off_Hours/Off_Hours.json"
                }
            ],
            "policies": [
                {
                    "kind": "grafana-notification-policies",
                    "receiver": "grafana-default-email",
                    "path": "policies/notification-policies.json"
                }
            ],
            "templates": [
                {
                    "kind": "grafana-notification-template",
                    "name": "slack.default",
                    "path": "templates/slack.default.json"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_access_export_fixture(
    dir: &Path,
    payload_filename: &str,
    kind: &str,
    version: i64,
    records: serde_json::Value,
) {
    fs::create_dir_all(dir).unwrap();
    let record_count = records.as_array().map(Vec::len).unwrap_or(0);
    fs::write(
        dir.join(payload_filename),
        serde_json::to_string_pretty(&json!({
            "kind": kind,
            "version": version,
            "records": records,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": kind,
            "version": version,
            "sourceUrl": "http://localhost:3000",
            "recordCount": record_count,
            "sourceDir": dir.display().to_string(),
        }))
        .unwrap(),
    )
    .unwrap();
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

fn assert_project_status_domain_contract(domain: &Value, expected_id: &str) {
    assert_eq!(domain["id"], json!(expected_id));
    assert!(domain["scope"].is_string());
    assert!(domain["mode"].is_string());
    assert!(domain["status"].is_string());
    assert!(domain["reasonCode"].is_string());
    assert!(domain["primaryCount"].is_u64());
    assert!(domain["blockerCount"].is_u64());
    assert!(domain["warningCount"].is_u64());
    assert!(domain["sourceKinds"].is_array());
    assert!(domain["signalKeys"].is_array());
    assert!(domain["blockers"].is_array());
    assert!(domain["warnings"].is_array());
    assert!(domain["nextActions"].is_array());
    assert!(domain["freshness"].is_object());
}

fn assert_dashboard_domain_contract(domain: &Value) {
    assert_project_status_domain_contract(domain, "dashboard");
    assert_eq!(domain["scope"], json!("staged"));
    assert_eq!(domain["mode"], json!("inspect-summary"));
    assert_eq!(domain["sourceKinds"], json!(["dashboard-export"]));
    let signal_keys = domain["signalKeys"].as_array().unwrap();
    for key in [
        "summary.dashboardCount",
        "summary.queryCount",
        "summary.orphanedDatasourceCount",
        "summary.mixedDatasourceDashboardCount",
    ] {
        assert!(signal_keys.iter().any(|value| value == key));
    }
}

#[test]
fn build_overview_artifacts_rejects_empty_inputs() {
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
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Text,
    };

    let error = build_overview_artifacts(&args).unwrap_err().to_string();

    assert!(error.contains("at least one input artifact"));
}

#[test]
fn overview_args_parse_and_help_expose_output_mode() {
    let default_args = OverviewCliArgs::parse_from(["grafana-util"]);
    assert!(default_args.command.is_none());
    assert_eq!(
        default_args.staged.output_format,
        OverviewOutputFormat::Text
    );

    let table_args = OverviewCliArgs::parse_from(["grafana-util", "--output-format", "table"]);
    assert_eq!(table_args.staged.output_format, OverviewOutputFormat::Table);

    let csv_args = OverviewCliArgs::parse_from(["grafana-util", "--output-format", "csv"]);
    assert_eq!(csv_args.staged.output_format, OverviewOutputFormat::Csv);

    let json_args = OverviewCliArgs::parse_from(["grafana-util", "--output-format", "json"]);
    assert!(json_args.command.is_none());
    assert_eq!(json_args.staged.output_format, OverviewOutputFormat::Json);

    let yaml_args = OverviewCliArgs::parse_from(["grafana-util", "--output-format", "yaml"]);
    assert!(yaml_args.command.is_none());
    assert_eq!(yaml_args.staged.output_format, OverviewOutputFormat::Yaml);

    #[cfg(feature = "tui")]
    {
        let interactive_args =
            OverviewCliArgs::parse_from(["grafana-util", "--output-format", "interactive"]);
        assert!(interactive_args.command.is_none());
        assert_eq!(
            interactive_args.staged.output_format,
            OverviewOutputFormat::Interactive
        );
    }
    #[cfg(not(feature = "tui"))]
    {
        assert!(OverviewCliArgs::try_parse_from([
            "grafana-util",
            "--output-format",
            "interactive"
        ])
        .is_err());
    }

    let help = OverviewCliArgs::command().render_long_help().to_string();
    assert!(help.contains("--output-format <OUTPUT_FORMAT>"));
    assert!(help.contains(
        "Render the overview document as table, csv, text, json, yaml, or interactive output."
    ));
    assert!(help.contains("--dashboard-provisioning-dir"));
    assert!(help.contains("--datasource-provisioning-file"));
    assert!(help.contains("live"));
    #[cfg(feature = "tui")]
    assert!(help.contains("interactive"));
    #[cfg(not(feature = "tui"))]
    assert!(!help.contains("interactive"));
}

fn sample_overview_document() -> OverviewDocument {
    OverviewDocument {
        kind: OVERVIEW_KIND.to_string(),
        schema_version: 1,
        tool_version: TOOL_VERSION.to_string(),
        summary: OverviewSummary {
            artifact_count: 3,
            dashboard_export_count: 1,
            datasource_export_count: 1,
            alert_export_count: 1,
            access_user_export_count: 0,
            access_team_export_count: 0,
            access_org_export_count: 0,
            access_service_account_export_count: 0,
            sync_summary_count: 0,
            bundle_preflight_count: 0,
            promotion_preflight_count: 0,
        },
        project_status: OverviewProjectStatus {
            schema_version: 1,
            tool_version: TOOL_VERSION.to_string(),
            scope: "staged-only".to_string(),
            overall: OverviewProjectStatusOverall {
                status: "partial".to_string(),
                domain_count: 6,
                present_count: 1,
                blocked_count: 0,
                blocker_count: 0,
                warning_count: 0,
                freshness: OverviewProjectStatusFreshness {
                    status: "current".to_string(),
                    source_count: 1,
                    newest_age_seconds: Some(15),
                    oldest_age_seconds: Some(15),
                },
            },
            domains: Vec::new(),
            top_blockers: Vec::new(),
            next_actions: Vec::new(),
        },
        artifacts: Vec::new(),
        selected_section_index: 0,
        sections: Vec::new(),
    }
}

#[test]
fn overview_summary_rows_feed_the_shared_tabular_renderer() {
    let document = sample_overview_document();
    let rows = build_overview_summary_rows(&document);

    assert_eq!(rows[0], ("status", "partial".to_string()));
    assert_eq!(rows[1], ("scope", "staged-only".to_string()));
    assert_eq!(rows[11], ("artifactCount", "3".to_string()));

    let table = render_summary_table(&rows);
    assert_eq!(table.len(), rows.len() + 2);
    assert!(table[0].contains("field"));
    assert!(table[2].contains("status"));
    assert!(table[2].contains("partial"));

    let csv = render_summary_csv(&rows);
    assert_eq!(csv[0], "field,value");
    assert_eq!(csv[1], "status,partial");
}

#[test]
fn overview_args_support_datasource_provisioning_file() {
    let args = OverviewCliArgs::parse_from([
        "grafana-util",
        "--datasource-provisioning-file",
        "./datasources/provisioning/datasources.yaml",
        "--output-format",
        "json",
    ]);

    assert_eq!(
        args.staged.datasource_provisioning_file,
        Some(std::path::Path::new("./datasources/provisioning/datasources.yaml").to_path_buf())
    );
}

#[test]
fn overview_args_support_dashboard_provisioning_dir() {
    let args = OverviewCliArgs::parse_from([
        "grafana-util",
        "--dashboard-provisioning-dir",
        "./dashboards/provisioning",
        "--output-format",
        "json",
    ]);

    assert_eq!(
        args.staged.dashboard_provisioning_dir,
        Some(std::path::Path::new("./dashboards/provisioning").to_path_buf())
    );
}

#[test]
fn overview_args_reject_dashboard_export_and_provisioning_inputs_together() {
    let args = OverviewCliArgs::try_parse_from([
        "grafana-util",
        "--dashboard-export-dir",
        "./dashboards/raw",
        "--dashboard-provisioning-dir",
        "./dashboards/provisioning",
    ]);

    assert!(args.is_err());
}

#[test]
fn overview_cli_help_exposes_staged_and_live_shapes() {
    let overview_help = OverviewCliArgs::command().render_long_help().to_string();
    assert!(overview_help.contains("thin entrypoint into shared live status"));
    assert!(overview_help.contains("live"));
}

#[test]
fn build_overview_artifacts_rejects_incomplete_bundle_context() {
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
        source_bundle: Some("/tmp/source-bundle.json".into()),
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Text,
    };

    let error = build_overview_artifacts(&args).unwrap_err().to_string();

    assert!(error.contains("--source-bundle"));
    assert!(error.contains("--target-inventory"));
}

#[test]
fn build_dashboard_domain_status_uses_shared_contract_fields() {
    let document = json!({
        "summary": {
            "dashboardCount": 2,
            "queryCount": 5,
            "orphanedDatasourceCount": 1,
            "mixedDatasourceDashboardCount": 2,
        }
    });

    let domain = build_dashboard_domain_status(Some(&document)).unwrap();
    let domain = serde_json::to_value(domain).unwrap();

    assert_dashboard_domain_contract(&domain);
    assert_eq!(domain["status"], json!("blocked"));
    assert_eq!(domain["reasonCode"], json!("blocked-by-blockers"));
    assert_eq!(domain["primaryCount"], json!(5));
    assert_eq!(domain["blockerCount"], json!(3));
    assert_eq!(domain["warningCount"], json!(0));
    assert_eq!(
        domain["blockers"],
        json!([
            {
                "kind": "orphaned-datasources",
                "count": 1,
                "source": "summary.orphanedDatasourceCount"
            },
            {
                "kind": "mixed-dashboards",
                "count": 2,
                "source": "summary.mixedDatasourceDashboardCount"
            }
        ])
    );
    assert_eq!(domain["warnings"], json!([]));
    assert_eq!(
        domain["nextActions"],
        json!(["resolve orphaned datasources, then mixed dashboards"])
    );
}

#[test]
fn build_dashboard_domain_status_surfaces_governance_warning_rows_from_summary_fields() {
    let document = json!({
        "summary": {
            "dashboardCount": 2,
            "queryCount": 5,
            "riskRecordCount": 2,
            "highBlastRadiusDatasourceCount": 1,
            "queryAuditCount": 3,
            "dashboardAuditCount": 1,
        }
    });

    let domain = build_dashboard_domain_status(Some(&document)).unwrap();
    let domain = serde_json::to_value(domain).unwrap();

    assert_dashboard_domain_contract(&domain);
    assert_eq!(domain["status"], json!("ready"));
    assert_eq!(domain["reasonCode"], json!("ready"));
    assert_eq!(domain["primaryCount"], json!(5));
    assert_eq!(domain["blockerCount"], json!(0));
    assert_eq!(domain["warningCount"], json!(7));
    assert_eq!(
        domain["signalKeys"],
        json!([
            "summary.dashboardCount",
            "summary.queryCount",
            "summary.orphanedDatasourceCount",
            "summary.mixedDatasourceDashboardCount",
            "summary.riskRecordCount",
            "summary.highBlastRadiusDatasourceCount",
            "summary.queryAuditCount",
            "summary.dashboardAuditCount",
        ])
    );
    assert_eq!(
        domain["warnings"],
        json!([
            {
                "kind": "risk-records",
                "count": 2,
                "source": "summary.riskRecordCount"
            },
            {
                "kind": "high-blast-radius-datasources",
                "count": 1,
                "source": "summary.highBlastRadiusDatasourceCount"
            },
            {
                "kind": "query-audits",
                "count": 3,
                "source": "summary.queryAuditCount"
            },
            {
                "kind": "dashboard-audits",
                "count": 1,
                "source": "summary.dashboardAuditCount"
            }
        ])
    );
    assert_eq!(
        domain["nextActions"],
        json!(["review dashboard governance warnings before promotion or apply"])
    );
}

#[test]
fn build_overview_document_and_render_overview_text_for_all_sections() {
    let temp = tempdir().unwrap();

    let dashboard_export_dir = temp.path().join("dashboards");
    fs::create_dir_all(dashboard_export_dir.join("General")).unwrap();
    fs::write(
        dashboard_export_dir.join("export-metadata.json"),
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
        dashboard_export_dir.join("folders.json"),
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
        dashboard_export_dir.join("General").join("cpu.json"),
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

    let desired_file = temp.path().join("desired.json");
    fs::write(
        &desired_file,
        serde_json::to_string_pretty(&json!([
            {
                "kind": "dashboard",
                "uid": "cpu-src",
                "title": "CPU Src",
                "body": {
                    "folderUid": "ops-src",
                    "datasourceUids": ["prom-src"],
                    "datasourceNames": ["Prometheus Source"]
                }
            },
            {
                "kind": "datasource",
                "uid": "prom-src",
                "name": "Prometheus Source",
                "body": {"type": "prometheus"}
            },
            {
                "kind": "folder",
                "uid": "ops-src",
                "title": "Ops Source"
            },
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition"],
                "body": {"condition": "A > 90"}
            }
        ]))
        .unwrap(),
    )
    .unwrap();

    let source_bundle_file = temp.path().join("source-bundle.json");
    fs::write(
        &source_bundle_file,
        serde_json::to_string_pretty(&json!({
            "dashboards": [
                {
                    "kind": "dashboard",
                    "uid": "cpu-src",
                    "title": "CPU Src",
                    "body": {
                        "folderUid": "ops-src",
                        "datasourceUids": ["prom-src"],
                        "datasourceNames": ["Prometheus Source"]
                    }
                }
            ],
            "datasources": [
                {
                    "kind": "datasource",
                    "uid": "loki-main",
                    "name": "Loki Main",
                    "title": "Loki Main",
                    "body": {
                        "uid": "loki-main",
                        "name": "Loki Main",
                        "type": "loki"
                    },
                    "secureJsonDataProviders": {
                        "httpHeaderValue1": "${provider:vault:secret/data/loki/token}"
                    },
                    "secureJsonDataPlaceholders": {
                        "basicAuthPassword": "${secret:loki-basic-auth}"
                    }
                }
            ],
            "folders": [],
            "alerts": [],
            "alerting": {
                "contactPoints": [
                    {
                        "sourcePath": "alerting/contact-points/pagerduty-main.json",
                        "document": {
                            "kind": "grafana-contact-point",
                            "spec": {
                                "uid": "pagerduty-main",
                                "name": "PagerDuty Main"
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
            "folders": [
                {
                    "uid": "ops-prod",
                    "title": "Ops Prod"
                }
            ],
            "datasources": [
                {
                    "uid": "prom-prod",
                    "name": "Prometheus Prod",
                    "title": "Prometheus Prod"
                }
            ]
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

    let mapping_file = temp.path().join("mapping.json");
    fs::write(
        &mapping_file,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-sync-promotion-mapping",
            "schemaVersion": 1,
            "metadata": {
                "sourceEnvironment": "staging",
                "targetEnvironment": "prod"
            },
            "folders": {
                "ops-src": "ops-prod"
            },
            "datasources": {
                "uids": {
                    "prom-src": "prom-prod"
                },
                "names": {
                    "Prometheus Source": "Prometheus Prod"
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let args = OverviewArgs {
        dashboard_export_dir: Some(dashboard_export_dir),
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: Some(desired_file),
        source_bundle: Some(source_bundle_file),
        target_inventory: Some(target_inventory_file),
        alert_export_dir: None,
        availability_file: Some(availability_file),
        mapping_file: Some(mapping_file),
        output_format: OverviewOutputFormat::Text,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();
    let document = build_overview_document(artifacts).unwrap();
    let json_document = serde_json::to_value(&document).unwrap();
    let lines = render_overview_text(&document).unwrap();
    let dashboard_domain = json_document["projectStatus"]["domains"]
        .as_array()
        .unwrap()
        .iter()
        .find(|domain| domain["id"] == json!("dashboard"))
        .unwrap();

    assert_eq!(document.kind, OVERVIEW_KIND);
    assert_eq!(document.tool_version, TOOL_VERSION);
    assert_eq!(document.summary.artifact_count, 4);
    assert_eq!(document.summary.dashboard_export_count, 1);
    assert_eq!(document.summary.datasource_export_count, 0);
    assert_eq!(document.summary.alert_export_count, 0);
    assert_eq!(document.summary.sync_summary_count, 1);
    assert_eq!(document.summary.bundle_preflight_count, 1);
    assert_eq!(document.summary.promotion_preflight_count, 1);
    assert_eq!(
        json_document["projectStatus"]["scope"],
        json!("staged-only")
    );
    assert_eq!(json_document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(
        json_document["projectStatus"]["overall"]["status"],
        json!("blocked")
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["domainCount"],
        json!(6)
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["presentCount"],
        json!(3)
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["blockedCount"],
        json!(2)
    );
    assert!(
        json_document["projectStatus"]["overall"]["blockerCount"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["warningCount"],
        json!(1)
    );
    assert_dashboard_domain_contract(dashboard_domain);
    assert_eq!(dashboard_domain["status"], json!("ready"));
    assert_eq!(dashboard_domain["reasonCode"], json!("ready"));
    assert_eq!(dashboard_domain["primaryCount"], json!(1));
    assert_eq!(dashboard_domain["blockerCount"], json!(0));
    assert_eq!(dashboard_domain["warningCount"], json!(0));
    assert_eq!(dashboard_domain["blockers"], json!([]));
    assert_eq!(dashboard_domain["warnings"], json!([]));
    assert_eq!(dashboard_domain["nextActions"], json!([]));
    let sync_domain = json_document["projectStatus"]["domains"]
        .as_array()
        .unwrap()
        .iter()
        .find(|domain| domain["id"] == json!("sync"))
        .unwrap();
    assert_project_status_domain_contract(sync_domain, "sync");
    assert_eq!(sync_domain["scope"], json!("staged"));
    assert_eq!(sync_domain["mode"], json!("staged-documents"));
    assert_eq!(
        sync_domain["sourceKinds"],
        json!(["sync-summary", "bundle-preflight"])
    );
    assert_eq!(sync_domain["status"], json!("blocked"));
    assert_eq!(sync_domain["reasonCode"], json!("blocked-by-blockers"));
    assert!(
        sync_domain["blockerCount"].as_u64().unwrap()
            >= sync_domain["blockers"].as_array().unwrap().len() as u64
    );
    assert!(sync_domain["signalKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("summary.syncBlockingCount")));
    assert!(sync_domain["signalKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("summary.providerBlockingCount")));
    assert!(sync_domain["signalKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("summary.secretPlaceholderBlockingCount")));
    assert!(sync_domain["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["kind"] == json!("alert-artifact-plan-only")));
    assert!(
        sync_domain["nextActions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                == "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact")
    );
    assert!(json_document["projectStatus"]["domains"]
        .as_array()
        .unwrap()
        .iter()
        .any(|domain| domain["id"] == json!("promotion")
            && domain["status"] == json!("blocked")
            && domain["reasonCode"] == json!("blocked-by-blockers")
            && !domain["blockers"].as_array().unwrap().is_empty()));
    let promotion_domain = json_document["projectStatus"]["domains"]
        .as_array()
        .unwrap()
        .iter()
        .find(|domain| domain["id"] == json!("promotion"))
        .unwrap();
    assert_project_status_domain_contract(promotion_domain, "promotion");
    assert_eq!(
        promotion_domain["sourceKinds"],
        json!(["promotion-preflight"])
    );
    assert!(promotion_domain["signalKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("summary.blockingCount")));
    assert_eq!(json_document["selectedSectionIndex"], json!(0));
    assert_eq!(json_document["sections"].as_array().unwrap().len(), 4);
    assert_eq!(
        json_document["artifacts"][0]["kind"],
        json!("dashboard-export")
    );
    assert_eq!(json_document["sections"][0]["artifactIndex"], json!(0));
    assert_eq!(
        json_document["sections"][0]["label"],
        json!("Dashboard export")
    );
    assert!(json_document["sections"][0]["subtitle"]
        .as_str()
        .unwrap()
        .contains("dashboards=1"));
    assert!(json_document["sections"][0]["subtitle"]
        .as_str()
        .unwrap()
        .contains("folders=1"));
    assert!(json_document["sections"][0]["subtitle"]
        .as_str()
        .unwrap()
        .contains("mixed-dashboards=0"));
    let dashboard_section_views = json_document["sections"][0]["views"].as_array().unwrap();
    let dashboard_view_labels = dashboard_section_views
        .iter()
        .map(|view| view["label"].as_str().unwrap())
        .collect::<Vec<&str>>();
    assert!(dashboard_view_labels.contains(&"Summary"));
    assert!(dashboard_view_labels.contains(&"Coverage"));
    assert!(dashboard_view_labels.contains(&"Inputs"));
    let summary_view = dashboard_section_views
        .iter()
        .find(|view| view["label"] == json!("Summary"))
        .unwrap();
    let coverage_view = dashboard_section_views
        .iter()
        .find(|view| view["label"] == json!("Coverage"))
        .unwrap();
    assert_eq!(coverage_view["items"][0]["title"], json!("dashboards"));
    assert_eq!(summary_view["items"][0]["kind"], json!("dashboard"));
    assert_eq!(
        summary_view["items"][0]["facts"][0]["label"],
        json!("dashboards")
    );
    assert_eq!(summary_view["items"][0]["facts"][0]["value"], json!("1"));
    assert!(lines.iter().any(|line| line
        .contains("- dashboard status=ready reason=ready primary=1 blockers=0 warnings=0")));
    assert!(summary_view["items"][0]["details"][1]
        .as_str()
        .unwrap()
        .starts_with("Summary: dashboards=1"));
    let bundle_section = json_document["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["label"] == json!("Sync bundle preflight"))
        .unwrap();
    let bundle_views = bundle_section["views"].as_array().unwrap();
    assert!(bundle_views
        .iter()
        .any(|view| view["label"] == json!("Blocking Signals")));
    assert!(bundle_views
        .iter()
        .any(|view| view["label"] == json!("Sync Checks")));
    assert!(bundle_views
        .iter()
        .any(|view| view["label"] == json!("Secret Providers")));
    assert!(bundle_views
        .iter()
        .any(|view| view["label"] == json!("Secret Placeholders")));
    assert!(bundle_views
        .iter()
        .any(|view| view["label"] == json!("Alerting Artifacts")));

    let provider_view = bundle_views
        .iter()
        .find(|view| view["label"] == json!("Secret Providers"))
        .unwrap();
    assert_eq!(
        provider_view["items"][0]["title"],
        json!("loki-main->vault")
    );
    assert_eq!(
        provider_view["items"][0]["meta"],
        json!("kind=secret-provider status=missing blocking=true")
    );
    assert_eq!(
        provider_view["items"][0]["facts"][0]["value"],
        json!("Loki Main")
    );

    let placeholder_view = bundle_views
        .iter()
        .find(|view| view["label"] == json!("Secret Placeholders"))
        .unwrap();
    assert_eq!(
        placeholder_view["items"][0]["title"],
        json!("loki-main->loki-basic-auth")
    );
    assert_eq!(
        placeholder_view["items"][0]["meta"],
        json!("kind=secret-placeholder status=missing blocking=true")
    );
    assert_eq!(
        placeholder_view["items"][0]["details"][9],
        json!(
            "Detail: Datasource secret placeholder is not available in secretPlaceholderNames availability input."
        )
    );

    let alert_artifact_view = bundle_views
        .iter()
        .find(|view| view["label"] == json!("Alerting Artifacts"))
        .unwrap();
    assert_eq!(
        alert_artifact_view["items"][0]["title"],
        json!("PagerDuty Main")
    );
    assert_eq!(
        alert_artifact_view["items"][0]["meta"],
        json!("kind=alert-contact-point status=plan-only blocking=false")
    );
    assert_eq!(
        alert_artifact_view["items"][0]["facts"][1]["value"],
        json!("alerting/contact-points/pagerduty-main.json")
    );
    assert!(lines[0].contains("Project overview"));
    assert!(lines[1].contains("Status: blocked"));
    assert!(lines[2].contains("4 total"));
    assert!(lines.iter().any(|line| line.contains("Domain status:")));
    assert!(lines
        .iter()
        .any(|line| line.contains("- sync status=blocked reason=blocked-by-blockers")));
    assert!(lines
        .iter()
        .any(|line| line.contains("next=resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact")));
    assert!(lines.iter().any(|line| line.contains("# Dashboard export")));
    assert!(lines.iter().any(|line| line.contains("# Sync summary")));
    assert!(lines
        .iter()
        .any(|line| line.contains("# Sync bundle preflight")));
    assert!(lines
        .iter()
        .any(|line| line.contains("# Sync promotion preflight")));
    assert!(lines.iter().any(|line| line.contains("exportDir=")));
    assert!(lines.iter().any(|line| line.contains("desiredFile=")));
    assert!(lines.iter().any(|line| line.contains("sourceBundle=")));
    assert!(lines.iter().any(|line| line.contains("mappingFile=")));
}

#[test]
fn build_overview_document_and_render_overview_text_for_change_summary_domain_status() {
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
    let json_document = serde_json::to_value(&document).unwrap();
    let lines = render_overview_text(&document).unwrap();
    let sync_domain = json_document["projectStatus"]["domains"]
        .as_array()
        .unwrap()
        .iter()
        .find(|domain| domain["id"] == json!("sync"))
        .unwrap();

    assert_eq!(document.summary.artifact_count, 1);
    assert_eq!(document.summary.sync_summary_count, 1);
    assert_eq!(
        json_document["projectStatus"]["overall"]["status"],
        json!("partial")
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["presentCount"],
        json!(1)
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["blockedCount"],
        json!(0)
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["blockerCount"],
        json!(0)
    );
    assert_eq!(
        json_document["projectStatus"]["overall"]["warningCount"],
        json!(0)
    );
    assert_project_status_domain_contract(sync_domain, "sync");
    assert_eq!(sync_domain["scope"], json!("staged"));
    assert_eq!(sync_domain["mode"], json!("staged-documents"));
    assert_eq!(sync_domain["status"], json!("ready"));
    assert_eq!(sync_domain["reasonCode"], json!("ready"));
    assert_eq!(sync_domain["primaryCount"], json!(4));
    assert_eq!(sync_domain["blockerCount"], json!(0));
    assert_eq!(sync_domain["warningCount"], json!(0));
    assert_eq!(sync_domain["sourceKinds"], json!(["sync-summary"]));
    assert_eq!(sync_domain["signalKeys"], json!(["summary.resourceCount"]));
    assert_eq!(sync_domain["blockers"], json!([]));
    assert_eq!(sync_domain["warnings"], json!([]));
    assert_eq!(
        sync_domain["nextActions"],
        json!(["re-run sync summary after staged changes"])
    );
    assert!(lines.iter().any(|line| line.contains("# Sync summary")));
    assert!(lines
        .iter()
        .any(|line| line
            .contains("Summary: resources=4 dashboards=1 datasources=1 folders=1 alerts=1")));
    assert!(lines.iter().any(|line| line
        .contains("- sync status=ready reason=ready primary=4 blockers=0 warnings=0 freshness=")));
    assert!(lines.iter().any(|line| {
        line.contains("next=re-run sync summary after staged changes")
            && line.contains("- sync status=ready")
    }));
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
            "Domains:".to_string(),
            "- sync status=ready mode=staged-documents primary=4 blockers=0 warnings=0 freshness=current next=re-run sync summary after staged changes"
                .to_string(),
            "Next actions:".to_string(),
            "- sync reason=ready action=re-run sync summary after staged changes".to_string(),
        ]
    );
}

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
fn build_overview_document_and_render_overview_text_for_datasource_export_section() {
    let temp = tempdir().unwrap();
    let datasource_export_dir = temp.path().join("datasources");
    write_datasource_export_fixture(&datasource_export_dir, "root");

    let args = OverviewArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: Some(datasource_export_dir),
        datasource_provisioning_file: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Json,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();
    let document = build_overview_document(artifacts).unwrap();
    let json_document = serde_json::to_value(&document).unwrap();
    let lines = render_overview_text(&document).unwrap();

    assert_eq!(document.summary.artifact_count, 1);
    assert_eq!(document.summary.datasource_export_count, 1);
    assert_eq!(
        json_document["artifacts"][0]["kind"],
        json!("datasource-export")
    );
    assert_eq!(json_document["sections"].as_array().unwrap().len(), 1);
    assert_eq!(
        json_document["sections"][0]["views"][1]["label"],
        json!("Inventory Facts")
    );
    assert_eq!(
        json_document["sections"][0]["views"][2]["label"],
        json!("Datasource Inventory")
    );
    assert_eq!(
        json_document["sections"][0]["views"][2]["items"][0]["title"],
        json!("Prometheus Main")
    );
    assert_eq!(
        json_document["sections"][0]["views"][3]["label"],
        json!("Inputs")
    );
    assert_eq!(
        json_document["sections"][0]["views"][0]["items"][0]["facts"][0]["label"],
        json!("datasources")
    );
    assert_eq!(
        json_document["sections"][0]["views"][0]["items"][0]["facts"][0]["value"],
        json!("2")
    );
    assert_eq!(
        json_document["sections"][0]["views"][0]["items"][0]["meta"],
        json!("datasources=2 orgs=2 defaults=1 types=2")
    );
    assert_eq!(
        json_document["sections"][0]["views"][0]["items"][0]["details"][1],
        json!("Summary: datasources=2 orgs=2 defaults=1 types=2")
    );
    let datasource_domain = &json_document["projectStatus"]["domains"][0];
    assert_project_status_domain_contract(datasource_domain, "datasource");
    assert_eq!(datasource_domain["status"], json!("ready"));
    assert_eq!(datasource_domain["reasonCode"], json!("ready"));
    assert_eq!(datasource_domain["primaryCount"], json!(2));
    assert_eq!(datasource_domain["warningCount"], json!(0));
    assert_eq!(
        datasource_domain["sourceKinds"],
        json!(["datasource-export"])
    );
    assert_eq!(
        datasource_domain["signalKeys"],
        json!([
            "summary.datasourceCount",
            "summary.orgCount",
            "summary.defaultCount",
            "summary.typeCount",
            "summary.wouldCreate",
            "summary.wouldUpdate",
            "summary.wouldSkip",
            "summary.wouldBlock",
            "summary.wouldCreateOrgCount",
        ])
    );
    assert_eq!(datasource_domain["nextActions"], json!([]));
    assert!(lines
        .iter()
        .any(|line| line.contains("# Datasource export")));
    assert!(lines.iter().any(|line| line.contains("datasources=2")));
    assert!(lines.iter().any(|line| line.contains("orgs=2")));
    assert!(lines.iter().any(|line| line.contains("defaults=1")));
    assert!(lines.iter().any(|line| line.contains("types=2")));
    assert!(lines.iter().any(|line| line.contains("exportDir=")));
}

#[test]
fn build_overview_document_and_render_overview_text_for_datasource_provisioning_section() {
    let temp = tempdir().unwrap();
    let datasource_provisioning_file = temp.path().join("datasources.yaml");
    write_datasource_provisioning_fixture(&datasource_provisioning_file);

    let args = OverviewArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: Some(datasource_provisioning_file.clone()),
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Json,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();
    let document = build_overview_document(artifacts).unwrap();
    let json_document = serde_json::to_value(&document).unwrap();
    let lines = render_overview_text(&document).unwrap();

    assert_eq!(document.summary.artifact_count, 1);
    assert_eq!(document.summary.datasource_export_count, 1);
    assert_eq!(
        json_document["artifacts"][0]["title"],
        json!("Datasource provisioning")
    );
    assert_eq!(
        json_document["sections"][0]["views"][0]["items"][0]["meta"],
        json!("datasources=2 orgs=2 defaults=1 types=2")
    );
    assert!(lines
        .iter()
        .any(|line| line.contains("# Datasource provisioning")));
    assert!(lines
        .iter()
        .any(|line| line.contains("datasourceProvisioningFile=")));
}

#[test]
fn build_overview_document_and_render_overview_text_accepts_combined_dashboard_and_datasource_export_roots(
) {
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

    let args = OverviewArgs {
        dashboard_export_dir: Some(dashboard_export_dir),
        dashboard_provisioning_dir: None,
        datasource_export_dir: Some(datasource_export_dir),
        datasource_provisioning_file: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Json,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();
    let document = build_overview_document(artifacts).unwrap();
    let json_document = serde_json::to_value(&document).unwrap();
    let lines = render_overview_text(&document).unwrap();
    let dashboard_domain = json_document["projectStatus"]["domains"]
        .as_array()
        .unwrap()
        .iter()
        .find(|domain| domain["id"] == json!("dashboard"))
        .unwrap();
    let datasource_domain = json_document["projectStatus"]["domains"]
        .as_array()
        .unwrap()
        .iter()
        .find(|domain| domain["id"] == json!("datasource"))
        .unwrap();

    assert_eq!(document.summary.artifact_count, 2);
    assert_eq!(document.summary.dashboard_export_count, 1);
    assert_eq!(document.summary.datasource_export_count, 1);
    assert_eq!(
        json_document["projectStatus"]["scope"],
        json!("staged-only")
    );
    assert_dashboard_domain_contract(dashboard_domain);
    assert_eq!(dashboard_domain["status"], json!("ready"));
    assert_eq!(dashboard_domain["reasonCode"], json!("ready"));
    assert_eq!(dashboard_domain["sourceKinds"], json!(["dashboard-export"]));
    assert_eq!(dashboard_domain["nextActions"], json!([]));
    assert_project_status_domain_contract(datasource_domain, "datasource");
    assert_eq!(datasource_domain["status"], json!("ready"));
    assert_eq!(datasource_domain["reasonCode"], json!("ready"));
    assert_eq!(
        datasource_domain["sourceKinds"],
        json!(["datasource-export"])
    );
    assert_eq!(datasource_domain["nextActions"], json!([]));
    assert!(lines.iter().any(|line| line.contains("# Dashboard export")));
    assert!(lines
        .iter()
        .any(|line| line.contains("# Datasource export")));
}

#[test]
fn build_overview_artifacts_rejects_dashboard_root_for_dashboard_export_input() {
    let temp = tempdir().unwrap();
    let dashboard_root = temp.path().join("dashboards");
    write_dashboard_root_fixture(&dashboard_root);

    let args = OverviewArgs {
        dashboard_export_dir: Some(dashboard_root),
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Text,
    };

    let error = build_overview_artifacts(&args).unwrap_err().to_string();

    assert!(error.contains("Point this command at the raw/ directory"));
}

#[test]
fn build_overview_artifacts_accepts_workspace_root_datasource_manifest() {
    let temp = tempdir().unwrap();
    let datasource_export_dir = temp.path().join("datasources");
    write_datasource_export_fixture_with_scope_kind(
        &datasource_export_dir,
        "all-orgs-root",
        Some("workspace-root"),
    );
    write_datasource_scope_fixture(
        &datasource_export_dir.join("org_1_Main_Org"),
        "1",
        "Main Org.",
    );

    let args = OverviewArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: Some(datasource_export_dir),
        datasource_provisioning_file: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Json,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();

    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].title, "Datasource export");
}

#[test]
fn build_overview_artifacts_rejects_datasource_unknown_root_scope_kind() {
    let temp = tempdir().unwrap();
    let datasource_export_dir = temp.path().join("datasources");
    write_datasource_export_fixture_with_scope_kind(
        &datasource_export_dir,
        "all-orgs-root",
        Some("unexpected-root"),
    );

    let args = OverviewArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: Some(datasource_export_dir),
        datasource_provisioning_file: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Text,
    };

    let error = build_overview_artifacts(&args).unwrap_err().to_string();

    assert!(error.contains("Overview datasource export root is not supported"));
}

#[test]
fn build_overview_document_and_render_overview_text_for_alert_export_section() {
    let temp = tempdir().unwrap();
    let alert_export_dir = temp.path().join("alerts");
    write_alert_export_fixture(&alert_export_dir);

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
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: Some(alert_export_dir),
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Json,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();
    let document = build_overview_document(artifacts).unwrap();
    let json_document = serde_json::to_value(&document).unwrap();
    let lines = render_overview_text(&document).unwrap();

    assert_eq!(document.summary.artifact_count, 1);
    assert_eq!(document.summary.alert_export_count, 1);
    assert_eq!(json_document["artifacts"][0]["kind"], json!("alert-export"));
    assert_eq!(
        json_document["sections"][0]["views"][1]["label"],
        json!("Alert Assets")
    );
    assert_eq!(
        json_document["sections"][0]["views"][2]["label"],
        json!("Asset Inventory")
    );
    assert_eq!(
        json_document["sections"][0]["views"][2]["items"][0]["title"],
        json!("CPU High")
    );
    assert_eq!(
        json_document["sections"][0]["views"][3]["label"],
        json!("Inputs")
    );
    assert!(lines.iter().any(|line| line.contains("# Alert export")));
    assert!(lines.iter().any(|line| line.contains("rules=1")));
    assert!(lines.iter().any(|line| line.contains("contact-points=1")));
    assert!(lines.iter().any(|line| line.contains("mute-timings=1")));
    assert!(lines.iter().any(|line| line.contains("policies=1")));
    assert!(lines.iter().any(|line| line.contains("templates=1")));
    assert!(lines.iter().any(|line| line.contains("exportDir=")));
}

#[test]
fn build_overview_document_and_render_overview_text_for_access_export_sections() {
    let temp = tempdir().unwrap();

    let user_export_dir = temp.path().join("access-users");
    write_access_export_fixture(
        &user_export_dir,
        "users.json",
        "grafana-utils-access-user-export-index",
        1,
        json!([
            {
                "login": "alice",
                "email": "alice@example.com",
                "name": "Alice",
                "teams": ["ops", "infra"]
            },
            {
                "login": "bob",
                "email": "bob@example.com",
                "name": "Bob",
                "teams": ["ops"]
            }
        ]),
    );

    let team_export_dir = temp.path().join("access-teams");
    write_access_export_fixture(
        &team_export_dir,
        "teams.json",
        "grafana-utils-access-team-export-index",
        1,
        json!([
            {
                "name": "ops",
                "email": "ops@example.com",
                "members": ["alice", "bob"],
                "admins": ["alice"]
            },
            {
                "name": "infra",
                "email": "infra@example.com",
                "members": ["carol"],
                "admins": ["carol"]
            }
        ]),
    );

    let org_export_dir = temp.path().join("access-orgs");
    write_access_export_fixture(
        &org_export_dir,
        "orgs.json",
        "grafana-utils-access-org-export-index",
        1,
        json!([
            {
                "id": "1",
                "name": "Main Org",
                "users": [
                    {
                        "login": "alice",
                        "email": "alice@example.com",
                        "name": "Alice",
                        "orgRole": "Admin"
                    }
                ]
            },
            {
                "id": "2",
                "name": "Ops Org",
                "users": [
                    {
                        "login": "bob",
                        "email": "bob@example.com",
                        "name": "Bob",
                        "orgRole": "Editor"
                    }
                ]
            }
        ]),
    );

    let service_account_export_dir = temp.path().join("access-service-accounts");
    write_access_export_fixture(
        &service_account_export_dir,
        "service-accounts.json",
        "grafana-utils-access-service-account-export-index",
        1,
        json!([
            {
                "name": "deploy-bot",
                "role": "Admin",
                "disabled": false
            },
            {
                "name": "read-bot",
                "role": "Viewer",
                "disabled": true
            }
        ]),
    );

    let args = OverviewArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: None,
        access_user_export_dir: Some(user_export_dir),
        access_team_export_dir: Some(team_export_dir),
        access_org_export_dir: Some(org_export_dir),
        access_service_account_export_dir: Some(service_account_export_dir),
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Json,
    };

    let artifacts = build_overview_artifacts(&args).unwrap();
    let document = build_overview_document(artifacts).unwrap();
    let json_document = serde_json::to_value(&document).unwrap();
    let lines = render_overview_text(&document).unwrap();

    assert_eq!(document.summary.artifact_count, 4);
    assert_eq!(document.summary.access_user_export_count, 1);
    assert_eq!(document.summary.access_team_export_count, 1);
    assert_eq!(document.summary.access_org_export_count, 1);
    assert_eq!(document.summary.access_service_account_export_count, 1);
    assert_eq!(json_document["selectedSectionIndex"], json!(0));
    assert_eq!(json_document["sections"].as_array().unwrap().len(), 4);
    let access_domain = json_document["projectStatus"]["domains"]
        .as_array()
        .unwrap()
        .iter()
        .find(|domain| domain["id"] == json!("access"))
        .unwrap();
    assert_project_status_domain_contract(access_domain, "access");
    assert_eq!(access_domain["scope"], json!("staged"));
    assert_eq!(access_domain["mode"], json!("staged-export-bundles"));
    assert_eq!(access_domain["status"], json!("ready"));
    assert_eq!(access_domain["reasonCode"], json!("ready"));
    assert_eq!(access_domain["primaryCount"], json!(8));
    assert_eq!(access_domain["blockerCount"], json!(0));
    assert_eq!(access_domain["warningCount"], json!(0));
    assert_eq!(
        access_domain["sourceKinds"],
        json!([
            "grafana-utils-access-user-export-index",
            "grafana-utils-access-team-export-index",
            "grafana-utils-access-org-export-index",
            "grafana-utils-access-service-account-export-index",
        ])
    );
    assert_eq!(
        access_domain["signalKeys"],
        json!([
            "summary.users.recordCount",
            "summary.teams.recordCount",
            "summary.orgs.recordCount",
            "summary.serviceAccounts.recordCount",
        ])
    );
    assert_eq!(access_domain["blockers"], json!([]));
    assert_eq!(access_domain["warnings"], json!([]));
    assert_eq!(
        access_domain["nextActions"],
        json!(["re-run access export after membership changes"])
    );
    assert_eq!(
        json_document["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|artifact| artifact["kind"].as_str().unwrap())
            .collect::<Vec<&str>>(),
        vec![
            "grafana-utils-access-user-export-index",
            "grafana-utils-access-team-export-index",
            "grafana-utils-access-org-export-index",
            "grafana-utils-access-service-account-export-index",
        ]
    );
    assert_eq!(
        json_document["sections"][0]["label"],
        json!("Access user export")
    );
    assert_eq!(
        json_document["sections"][0]["views"][1]["label"],
        json!("Export Facts")
    );
    assert_eq!(
        json_document["sections"][0]["views"][2]["label"],
        json!("Users")
    );
    assert_eq!(
        json_document["sections"][0]["views"][2]["items"][0]["title"],
        json!("alice")
    );
    assert_eq!(
        json_document["sections"][0]["views"][3]["label"],
        json!("Inputs")
    );
    assert_eq!(
        json_document["sections"][3]["views"][0]["items"][0]["facts"][0]["label"],
        json!("service-accounts")
    );
    assert_eq!(
        json_document["sections"][3]["views"][0]["items"][0]["facts"][0]["value"],
        json!("2")
    );
    assert!(lines
        .iter()
        .any(|line| line.contains("# Access user export")));
    assert!(lines
        .iter()
        .any(|line| line.contains("# Access team export")));
    assert!(lines
        .iter()
        .any(|line| line.contains("# Access org export")));
    assert!(lines
        .iter()
        .any(|line| line.contains("# Access service-account export")));
    assert!(lines.iter().any(|line| line == "Summary: users=2"));
    assert!(lines.iter().any(|line| line == "Summary: teams=2"));
    assert!(lines.iter().any(|line| line == "Summary: orgs=2"));
    assert!(lines
        .iter()
        .any(|line| line == "Summary: service-accounts=2"));
}

#[test]
fn build_overview_artifacts_rejects_access_export_metadata_kind_mismatch() {
    let temp = tempdir().unwrap();
    let user_export_dir = temp.path().join("access-users");
    write_access_export_fixture(
        &user_export_dir,
        "users.json",
        "grafana-utils-access-user-export-index",
        1,
        json!([
            {
                "login": "alice",
                "email": "alice@example.com"
            }
        ]),
    );
    fs::write(
        user_export_dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "not-supported",
            "version": 1,
            "sourceUrl": "http://localhost:3000",
            "recordCount": 1,
            "sourceDir": user_export_dir.display().to_string(),
        }))
        .unwrap(),
    )
    .unwrap();

    let args = OverviewArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: None,
        access_user_export_dir: Some(user_export_dir),
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Text,
    };

    let error = build_overview_artifacts(&args).unwrap_err().to_string();

    assert!(error.contains("metadata kind mismatch"));
}

#[test]
fn build_overview_artifacts_rejects_access_export_version_too_new() {
    let temp = tempdir().unwrap();
    let user_export_dir = temp.path().join("access-users");
    write_access_export_fixture(
        &user_export_dir,
        "users.json",
        "grafana-utils-access-user-export-index",
        2,
        json!([
            {
                "login": "alice",
                "email": "alice@example.com"
            }
        ]),
    );

    let args = OverviewArgs {
        dashboard_export_dir: None,
        dashboard_provisioning_dir: None,
        datasource_export_dir: None,
        datasource_provisioning_file: None,
        access_user_export_dir: Some(user_export_dir),
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: OverviewOutputFormat::Text,
    };

    let error = build_overview_artifacts(&args).unwrap_err().to_string();

    assert!(error.contains("Unsupported access export version"));
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveRequestRecord {
    path: String,
    target: String,
    org_id: Option<String>,
}

fn live_response_body(target: &str, org_id: Option<&str>) -> String {
    let path = target.split('?').next().unwrap_or(target);
    let scoped_org_id = org_id.unwrap_or("1");
    let scoped_org_name = if scoped_org_id == "2" {
        "Ops Org"
    } else {
        "Main Org"
    };

    match path {
        "/api/search" => serde_json::to_string(&json!([
            {
                "uid": format!("dash-{scoped_org_id}"),
                "title": format!("Dashboard {scoped_org_id}"),
                "type": "dash-db",
                "folderUid": "general",
                "folderTitle": "General",
                "url": format!("/d/dash-{scoped_org_id}/dashboard-{scoped_org_id}")
            }
        ]))
        .unwrap(),
        "/api/datasources" => serde_json::to_string(&json!([
            {
                "id": scoped_org_id.parse::<i64>().unwrap_or(1),
                "uid": format!("ds-{scoped_org_id}"),
                "name": format!("Datasource {scoped_org_id}"),
                "type": "prometheus",
                "access": "proxy",
                "isDefault": true,
                "orgId": scoped_org_id
            }
        ]))
        .unwrap(),
        "/api/orgs" => serde_json::to_string(&json!([
            {"id": 1, "name": "Main Org"},
            {"id": 2, "name": "Ops Org"}
        ]))
        .unwrap(),
        "/api/org" => serde_json::to_string(&json!({
            "id": scoped_org_id.parse::<i64>().unwrap_or(1),
            "name": scoped_org_name
        }))
        .unwrap(),
        "/api/dashboards/uid/dash-1/versions" | "/api/dashboards/uid/dash-2/versions" => {
            serde_json::to_string(&json!({
                "versions": [{"created": "2026-03-30T00:00:00Z"}]
            }))
            .unwrap()
        }
        "/api/v1/provisioning/alert-rules" => "[]".to_string(),
        "/api/v1/provisioning/contact-points" => "[]".to_string(),
        "/api/v1/provisioning/mute-timings" => "[]".to_string(),
        "/api/v1/provisioning/policies" => "{}".to_string(),
        "/api/v1/provisioning/templates" => "[]".to_string(),
        "/api/org/users" => "[]".to_string(),
        "/api/teams/search" => r#"{"teams":[]}"#.to_string(),
        "/api/serviceaccounts/search" => r#"{"serviceAccounts":[]}"#.to_string(),
        _ => "{}".to_string(),
    }
}

#[allow(clippy::type_complexity)]
fn spawn_live_project_status_test_server() -> (
    String,
    Arc<Mutex<Vec<LiveRequestRecord>>>,
    mpsc::Sender<()>,
    thread::JoinHandle<()>,
) {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            let (stop_tx, _stop_rx) = mpsc::channel();
            return (
                String::new(),
                Arc::new(Mutex::new(Vec::new())),
                stop_tx,
                thread::spawn(|| {}),
            );
        }
        Err(error) => panic!("failed to bind live project-status test listener: {error}"),
    };
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let requests_for_thread = Arc::clone(&requests);
    let (stop_tx, stop_rx) = mpsc::channel();

    let handle = thread::spawn(move || loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();

                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                loop {
                    let bytes_read = match stream.read(&mut buffer) {
                        Ok(bytes_read) => bytes_read,
                        Err(error)
                            if matches!(
                                error.kind(),
                                ErrorKind::WouldBlock | ErrorKind::TimedOut
                            ) =>
                        {
                            0
                        }
                        Err(error) => panic!("failed to read live test request: {error}"),
                    };
                    if bytes_read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..bytes_read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }

                let request_text = String::from_utf8(request).unwrap();
                let mut lines = request_text.lines();
                let request_line = lines.next().unwrap_or_default();
                let target = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();
                let path = target.split('?').next().unwrap_or("/").to_string();
                let org_id = lines.find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("X-Grafana-Org-Id") {
                        Some(value.trim().to_string())
                    } else {
                        None
                    }
                });

                requests_for_thread.lock().unwrap().push(LiveRequestRecord {
                    path: path.clone(),
                    target: target.clone(),
                    org_id: org_id.clone(),
                });

                let body = live_response_body(&target, org_id.as_deref());
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
                let _ = stream.flush();
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => match stop_rx.try_recv() {
                Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(10));
                }
            },
            Err(error) => panic!("failed to accept live test request: {error}"),
        }
    });

    (format!("http://{address}"), requests, stop_tx, handle)
}

fn collect_scoped_paths<'a>(
    requests: &'a [LiveRequestRecord],
    path: &str,
    org_id: &str,
) -> Vec<&'a LiveRequestRecord> {
    requests
        .iter()
        .filter(|request| request.path == path && request.org_id.as_deref() == Some(org_id))
        .collect()
}

fn sample_project_status_live_args(base_url: String) -> ProjectStatusLiveArgs {
    ProjectStatusLiveArgs {
        profile: None,
        url: base_url,
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 5,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
        all_orgs: false,
        org_id: None,
        sync_summary_file: None,
        bundle_preflight_file: None,
        promotion_summary_file: None,
        mapping_file: None,
        availability_file: None,
        output_format: crate::project_status_command::ProjectStatusOutputFormat::Text,
    }
}

#[test]
fn project_status_live_org_id_scopes_live_reads() {
    let (base_url, requests, stop_tx, handle) = spawn_live_project_status_test_server();
    if base_url.is_empty() {
        return;
    }
    let mut args = sample_project_status_live_args(base_url);
    args.org_id = Some(7);

    let status = execute_project_status_live(&args).unwrap();

    stop_tx.send(()).unwrap();
    handle.join().unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(status.scope, "live");
    assert!(!collect_scoped_paths(&requests, "/api/search", "7").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/datasources", "7").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/org", "7").is_empty());
}

#[test]
fn project_status_live_all_orgs_fans_out_across_visible_orgs() {
    let (base_url, requests, stop_tx, handle) = spawn_live_project_status_test_server();
    if base_url.is_empty() {
        return;
    }
    let mut args = sample_project_status_live_args(base_url);
    args.api_token = None;
    args.username = Some("admin".to_string());
    args.password = Some("admin".to_string());
    args.all_orgs = true;

    let status = execute_project_status_live(&args).unwrap();

    stop_tx.send(()).unwrap();
    handle.join().unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(status.scope, "live");
    assert!(requests.iter().any(|request| request.path == "/api/orgs"));
    assert!(!collect_scoped_paths(&requests, "/api/search", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/search", "2").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/datasources", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/datasources", "2").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/org/users", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/org/users", "2").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/teams/search", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/teams/search", "2").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/serviceaccounts/search", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/serviceaccounts/search", "2").is_empty());
}

#[test]
fn overview_live_delegates_org_scoped_reads_to_shared_live_path() {
    let (base_url, requests, stop_tx, handle) = spawn_live_project_status_test_server();
    if base_url.is_empty() {
        return;
    }
    let mut args = sample_project_status_live_args(base_url);
    args.org_id = Some(9);

    run_overview_live(args).unwrap();

    stop_tx.send(()).unwrap();
    handle.join().unwrap();

    let requests = requests.lock().unwrap();
    assert!(!collect_scoped_paths(&requests, "/api/search", "9").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/datasources", "9").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/org", "9").is_empty());
}
