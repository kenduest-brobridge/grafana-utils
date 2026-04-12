use super::{
    assert_dashboard_domain_contract, assert_project_status_domain_contract,
    sample_overview_document, write_access_export_fixture, write_alert_export_fixture,
    write_change_desired_fixture, write_dashboard_export_fixture, write_dashboard_root_fixture,
    write_datasource_export_fixture, write_datasource_export_fixture_with_scope_kind,
    write_datasource_provisioning_fixture, write_datasource_scope_fixture,
};
use crate::common::TOOL_VERSION;
use crate::dashboard::build_dashboard_domain_status;
use crate::overview::{
    build_overview_artifacts, build_overview_document, build_overview_summary_rows,
    render_overview_text, OverviewArgs, OverviewCliArgs, OverviewOutputFormat, OVERVIEW_KIND,
};
use crate::tabular_output::{render_summary_csv, render_summary_table};
use clap::{CommandFactory, Parser};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

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

    assert!(crate::overview::OVERVIEW_HELP_TEXT
        .contains("grafana-util status overview --dashboard-export-dir ./dashboards/raw"));
    assert!(crate::overview::OVERVIEW_LIVE_HELP_TEXT
        .contains("grafana-util status overview live --url http://localhost:3000 --token"));
    let help = OverviewCliArgs::command().render_long_help().to_string();
    assert!(!help.contains("grafana-util overview"));
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
fn overview_text_renderer_includes_compact_discovery_summary() {
    let mut document = sample_overview_document();
    document.discovery = Some(json!({
        "workspaceRoot": "/tmp/grafana-oac-repo",
        "inputCount": 3,
        "inputs": {
            "dashboardExportDir": "/tmp/grafana-oac-repo/dashboards/git-sync/raw",
            "alertExportDir": "/tmp/grafana-oac-repo/alerts/raw",
            "datasourceProvisioningFile": "/tmp/grafana-oac-repo/datasources/provisioning/datasources.yaml"
        }
    }));

    let lines = render_overview_text(&document).unwrap();
    assert!(lines.iter().any(|line| line.contains(
        "Discovery: workspace-root=/tmp/grafana-oac-repo sources=dashboard-export, datasource-provisioning, alert-export"
    )));
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
    assert!(crate::overview::OVERVIEW_HELP_TEXT
        .contains("grafana-util status overview --dashboard-export-dir ./dashboards/raw"));
    assert!(crate::overview::OVERVIEW_LIVE_HELP_TEXT
        .contains("grafana-util status overview live --url http://localhost:3000 --token"));
    let overview_help = OverviewCliArgs::command().render_long_help().to_string();
    assert!(!overview_help.contains("grafana-util overview"));
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
        json!(3)
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
        json!(["sync-summary", "package-test"])
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
    assert!(promotion_domain["signalKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("checkSummary.datasourceUidRemapCount")));
    assert!(promotion_domain["signalKeys"]
        .as_array()
        .unwrap()
        .contains(&json!("checkSummary.resolvedCount")));
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
    assert!(lines.iter().any(|line| line
        .contains("Signals: sync sources=sync-summary signalKeys=1 blockers=0 warnings=0")));
    assert!(lines.iter().any(|line| {
        line.contains("next=re-run sync summary after staged changes")
            && line.contains("- sync status=ready")
    }));
}

#[path = "overview_contract_bundle_rust_tests.rs"]
mod overview_contract_bundle_rust_tests;

#[path = "overview_contract_datasource_alert_rust_tests.rs"]
mod overview_contract_datasource_alert_rust_tests;

#[path = "overview_contract_access_rust_tests.rs"]
mod overview_contract_access_rust_tests;
