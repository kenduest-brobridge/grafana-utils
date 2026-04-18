//! Snapshot review rendering and document tests.

use super::tests_fixtures::{
    write_complete_dashboard_scope, write_datasource_inventory_rows,
    write_datasource_provisioning_lane, write_snapshot_access_lane_bundle,
    write_snapshot_dashboard_index, write_snapshot_dashboard_metadata,
    write_snapshot_datasource_root_metadata,
};
use crate::overview::OverviewOutputFormat;
use crate::snapshot::{
    build_snapshot_overview_args, build_snapshot_review_browser_items,
    build_snapshot_review_document, build_snapshot_review_summary_lines,
    render_snapshot_review_text, SnapshotCliArgs, SnapshotReviewArgs,
};
use clap::Parser;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn snapshot_review_builds_overview_args_for_interactive_output() {
    let review_args = SnapshotReviewArgs {
        input_dir: std::path::PathBuf::from("./snapshot"),
        interactive: false,
        output_format: OverviewOutputFormat::Interactive,
    };

    let overview_args = build_snapshot_overview_args(&review_args);

    assert_eq!(
        overview_args.dashboard_export_dir,
        Some(std::path::PathBuf::from("./snapshot/dashboards"))
    );
    assert_eq!(
        overview_args.datasource_export_dir,
        Some(std::path::PathBuf::from("./snapshot/datasources"))
    );
    assert_eq!(
        overview_args.access_user_export_dir,
        Some(std::path::PathBuf::from("./snapshot/access/users"))
    );
    assert_eq!(
        overview_args.access_team_export_dir,
        Some(std::path::PathBuf::from("./snapshot/access/teams"))
    );
    assert_eq!(
        overview_args.access_org_export_dir,
        Some(std::path::PathBuf::from("./snapshot/access/orgs"))
    );
    assert_eq!(
        overview_args.access_service_account_export_dir,
        Some(std::path::PathBuf::from(
            "./snapshot/access/service-accounts"
        ))
    );
    assert_eq!(
        overview_args.output_format,
        OverviewOutputFormat::Interactive
    );

    let document = json!({
        "kind": "grafana-utils-snapshot-review",
        "schemaVersion": 1,
        "summary": {
            "orgCount": 2,
            "dashboardOrgCount": 2,
            "datasourceOrgCount": 1,
            "dashboardCount": 3,
            "datasourceCount": 4
        },
        "orgs": [
            {
                "org": "Main Org.",
                "orgId": "1",
                "dashboardCount": 2,
                "datasourceCount": 3
            }
        ],
        "warnings": [
            {
                "code": "org-partial-coverage",
                "message": "Org Main Org. (orgId=1) has 2 dashboard(s) and 3 datasource(s)."
            }
        ]
    });

    let summary_lines = build_snapshot_review_summary_lines(&document).unwrap();
    assert!(summary_lines.iter().any(|line| line
        .contains("Org coverage: 2 combined org(s), 2 dashboard org(s), 1 datasource org(s)")));
    assert!(summary_lines
        .iter()
        .any(|line| line.contains("Warnings: 1")));

    let browser_items = build_snapshot_review_browser_items(&document).unwrap();
    assert_eq!(browser_items[0].kind, "snapshot");
    assert_eq!(browser_items[1].kind, "warning");
    assert!(browser_items[0]
        .details
        .iter()
        .any(|line| line.contains("Combined orgs: 2")));
    assert!(browser_items
        .iter()
        .any(|item| item.kind == "org" && item.title == "Main Org."));
    assert!(browser_items
        .iter()
        .any(|item| item.kind == "warning" && item.title == "org-partial-coverage"));
}

#[test]
fn snapshot_review_parses_all_supported_output_modes() {
    let cases = [
        ("table", OverviewOutputFormat::Table),
        ("csv", OverviewOutputFormat::Csv),
        ("text", OverviewOutputFormat::Text),
        ("json", OverviewOutputFormat::Json),
        ("yaml", OverviewOutputFormat::Yaml),
    ];

    for (output, expected) in cases {
        let review_args = SnapshotReviewArgs {
            input_dir: std::path::PathBuf::from("./snapshot"),
            interactive: false,
            output_format: expected,
        };
        let overview_args = build_snapshot_overview_args(&review_args);

        assert_eq!(overview_args.output_format, expected);
        assert_eq!(
            match SnapshotCliArgs::parse_from([
                "grafana-util",
                "review",
                "--input-dir",
                "./snapshot",
                "--output-format",
                output,
            ])
            .command
            {
                crate::snapshot::SnapshotCommand::Review(review) => review.output_format,
                other => panic!("expected snapshot review, got {:?}", other),
            },
            expected
        );
    }
}

#[test]
fn snapshot_review_browser_items_prioritize_signals_before_folders_and_split_folder_metadata() {
    let document = json!({
        "kind": "grafana-utils-snapshot-review",
        "schemaVersion": 1,
        "summary": {
            "orgCount": 1,
            "dashboardOrgCount": 1,
            "datasourceOrgCount": 1,
            "dashboardCount": 2,
            "folderCount": 1,
            "datasourceCount": 1,
            "datasourceTypeCount": 1,
            "defaultDatasourceCount": 1
        },
        "warnings": [
            {
                "code": "org-count-mismatch",
                "message": "Dashboard export covers 1 org(s) while datasource inventory covers 1 org(s)."
            }
        ],
        "lanes": {
            "dashboard": {
                "scopeCount": 2,
                "rawScopeCount": 2,
                "promptScopeCount": 1,
                "provisioningScopeCount": 1
            },
            "datasource": {
                "scopeCount": 1,
                "inventoryExpectedScopeCount": 1,
                "inventoryScopeCount": 1,
                "provisioningExpectedScopeCount": 1,
                "provisioningScopeCount": 1
            }
        },
        "orgs": [
            {
                "org": "Main Org.",
                "orgId": "1",
                "dashboardCount": 2,
                "folderCount": 1,
                "datasourceCount": 1,
                "defaultDatasourceCount": 1,
                "datasourceTypes": {
                    "prometheus": 1
                }
            }
        ],
        "datasourceTypes": [
            {
                "type": "prometheus",
                "count": 1
            }
        ],
        "datasources": [
            {
                "name": "Prometheus",
                "uid": "prom",
                "type": "prometheus",
                "org": "Main Org.",
                "orgId": "1",
                "url": "http://prometheus:9090",
                "access": "proxy",
                "isDefault": true
            }
        ],
        "folders": [
            {
                "title": "Infra",
                "path": "Platform / Infra",
                "uid": "infra",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]
    });

    let browser_items = build_snapshot_review_browser_items(&document).unwrap();
    let kinds: Vec<&str> = browser_items
        .iter()
        .map(|item| item.kind.as_str())
        .collect();

    assert_eq!(
        kinds,
        vec![
            "snapshot",
            "warning",
            "lane",
            "lane",
            "org",
            "datasource-type",
            "datasource",
            "folder"
        ]
    );

    let folder = browser_items.last().expect("folder browser item");
    assert_eq!(folder.title, "Infra");
    assert_eq!(
        folder.meta,
        "depth=2 path=Platform / Infra org=Main Org. uid=infra"
    );
    assert!(folder.details.iter().any(|line| line == "Depth: 2"));
    assert!(folder
        .details
        .iter()
        .any(|line| line == "Path: Platform / Infra"));
    assert!(folder.details.iter().any(|line| line == "Org: Main Org."));
    assert!(folder.details.iter().any(|line| line == "UID: infra"));
}

#[test]
fn snapshot_review_document_summarizes_inventory_counts_without_actions() {
    let temp = tempdir().unwrap();
    let snapshot_root = temp.path().join("snapshot");
    let dashboard_root = snapshot_root.join("dashboards");
    let datasource_root = snapshot_root.join("datasources");
    let access_root = snapshot_root.join("access");

    write_snapshot_dashboard_metadata(
        &dashboard_root,
        &[("1", "Main Org.", 2), ("2", "Ops Org", 1)],
    );
    write_complete_dashboard_scope(&dashboard_root.join("org_1_Main_Org"));
    write_complete_dashboard_scope(&dashboard_root.join("org_2_Ops_Org"));
    write_snapshot_dashboard_index(
        &dashboard_root,
        &[json!({
            "title": "Platform",
            "path": "Platform / Infra",
            "uid": "platform",
            "org": "Main Org.",
            "orgId": "1"
        })],
    );
    write_snapshot_datasource_root_metadata(&datasource_root, 3, "root");
    write_datasource_inventory_rows(
        &datasource_root,
        &[
            json!({
                "uid": "prom-main",
                "name": "prom-main",
                "type": "prometheus",
                "url": "http://prometheus:9090",
                "isDefault": true,
                "org": "Main Org.",
                "orgId": "1"
            }),
            json!({
                "uid": "loki-main",
                "name": "loki-main",
                "type": "loki",
                "url": "http://loki:3100",
                "isDefault": false,
                "org": "Main Org.",
                "orgId": "1"
            }),
            json!({
                "uid": "tempo-ops",
                "name": "tempo-ops",
                "type": "tempo",
                "url": "http://tempo:3200",
                "isDefault": false,
                "org": "Ops Org",
                "orgId": "2"
            }),
        ],
    );
    write_datasource_provisioning_lane(&datasource_root);
    write_snapshot_access_lane_bundle(
        &access_root.join("users"),
        "users.json",
        "grafana-utils-access-user-export-index",
        2,
    );
    write_snapshot_access_lane_bundle(
        &access_root.join("teams"),
        "teams.json",
        "grafana-utils-access-team-export-index",
        3,
    );
    write_snapshot_access_lane_bundle(
        &access_root.join("orgs"),
        "orgs.json",
        "grafana-utils-access-org-export-index",
        1,
    );
    write_snapshot_access_lane_bundle(
        &access_root.join("service-accounts"),
        "service-accounts.json",
        "grafana-utils-access-service-account-export-index",
        4,
    );

    let document =
        build_snapshot_review_document(&dashboard_root, &datasource_root, &datasource_root)
            .unwrap();
    assert_eq!(document["kind"], json!("grafana-utils-snapshot-review"));
    assert_eq!(document["summary"]["orgCount"], json!(2));
    assert_eq!(document["summary"]["dashboardOrgCount"], json!(2));
    assert_eq!(document["summary"]["datasourceOrgCount"], json!(2));
    assert_eq!(document["summary"]["dashboardCount"], json!(3));
    assert_eq!(document["summary"]["datasourceCount"], json!(3));
    assert_eq!(document["summary"]["folderCount"], json!(1));
    assert_eq!(document["summary"]["datasourceTypeCount"], json!(3));
    assert_eq!(document["summary"]["accessUserCount"], json!(2));
    assert_eq!(document["summary"]["accessTeamCount"], json!(3));
    assert_eq!(document["summary"]["accessOrgCount"], json!(1));
    assert_eq!(document["summary"]["accessServiceAccountCount"], json!(4));
    let warning_codes: Vec<&str> = document["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|warning| warning["code"].as_str().unwrap())
        .collect();
    assert!(
        warning_codes.is_empty(),
        "unexpected warnings: {warning_codes:?}"
    );

    let orgs = document["orgs"].as_array().expect("orgs");
    assert_eq!(orgs.len(), 2);
    assert_eq!(orgs[0]["org"], json!("Main Org."));
    assert_eq!(orgs[0]["dashboardCount"], json!(2));
    assert_eq!(orgs[0]["datasourceCount"], json!(2));
    assert_eq!(orgs[1]["org"], json!("Ops Org"));
    assert_eq!(orgs[1]["dashboardCount"], json!(1));
    assert_eq!(orgs[1]["datasourceCount"], json!(1));

    let rendered = render_snapshot_review_text(&document).unwrap();
    assert!(rendered.iter().any(|line| line == "Snapshot review"));
    assert!(rendered.iter().any(|line| line == "Warnings: none"));
    assert!(rendered.iter().all(|line| !line.contains("Top action")));

    let summary_lines = build_snapshot_review_summary_lines(&document).unwrap();
    assert!(summary_lines
        .iter()
        .any(|line| line.contains("3 dashboard(s), 1 folder(s), 3 datasource(s)")));
    assert!(summary_lines
        .iter()
        .any(|line| line
            .contains("Access totals: 2 user(s), 3 team(s), 1 org(s), 4 service-account(s)")));
    assert_eq!(document["lanes"]["dashboard"]["scopeCount"], json!(2));
    assert_eq!(document["lanes"]["dashboard"]["rawScopeCount"], json!(2));
    assert_eq!(document["lanes"]["dashboard"]["promptScopeCount"], json!(2));
    assert_eq!(
        document["lanes"]["dashboard"]["provisioningScopeCount"],
        json!(2)
    );
    assert_eq!(document["lanes"]["datasource"]["scopeCount"], json!(1));
    assert_eq!(
        document["lanes"]["datasource"]["inventoryExpectedScopeCount"],
        json!(1)
    );
    assert_eq!(
        document["lanes"]["datasource"]["inventoryScopeCount"],
        json!(1)
    );
    assert_eq!(
        document["lanes"]["datasource"]["provisioningExpectedScopeCount"],
        json!(1)
    );
    assert_eq!(
        document["lanes"]["datasource"]["provisioningScopeCount"],
        json!(1)
    );
    assert_eq!(document["lanes"]["access"]["present"], json!(true));
    assert_eq!(
        document["lanes"]["access"]["users"]["recordCount"],
        json!(2)
    );
    assert_eq!(
        document["lanes"]["access"]["teams"]["recordCount"],
        json!(3)
    );
    assert_eq!(document["lanes"]["access"]["orgs"]["recordCount"], json!(1));
    assert_eq!(
        document["lanes"]["access"]["serviceAccounts"]["recordCount"],
        json!(4)
    );

    let browser_items = build_snapshot_review_browser_items(&document).unwrap();
    let kinds: Vec<&str> = browser_items
        .iter()
        .map(|item| item.kind.as_str())
        .collect();
    assert_eq!(
        &kinds[..6],
        ["snapshot", "lane", "lane", "lane", "org", "org"]
    );
    let folder_index = kinds
        .iter()
        .position(|kind| *kind == "folder")
        .expect("folder item");
    let datasource_type_index = kinds
        .iter()
        .position(|kind| *kind == "datasource-type")
        .expect("datasource-type item");
    let datasource_index = kinds
        .iter()
        .position(|kind| *kind == "datasource")
        .expect("datasource item");
    assert!(
        folder_index > 4,
        "folders must follow the higher-signal summary items"
    );
    assert!(
        datasource_type_index < folder_index,
        "datasource types should remain visible before folders"
    );
    assert!(
        datasource_index < folder_index,
        "datasources should remain visible before folders"
    );

    assert_eq!(browser_items[0].kind, "snapshot");
    assert_eq!(
        browser_items[0].meta,
        "2 org(s)  3 dashboard(s)  1 folder(s)  3 datasource(s)"
    );
    assert_eq!(browser_items[0].title, "Snapshot summary");
    assert!(browser_items[0]
        .details
        .iter()
        .any(|line| line == "Dashboard orgs: 2"));
    assert!(browser_items[0]
        .details
        .iter()
        .any(|line| line == "Datasource orgs: 2"));
    assert!(browser_items[0]
        .details
        .iter()
        .any(|line| line == "Access users: 2"));
    assert!(browser_items
        .iter()
        .any(|item| item.kind == "org" && item.title == "Main Org."));
    let access_lane = browser_items
        .iter()
        .find(|item| item.kind == "lane" && item.title == "Access lanes")
        .expect("access lane browser item");
    assert!(access_lane.meta.contains("users 2"));
    assert!(access_lane.details.iter().any(|line| line == "Users: 2"));
    assert!(access_lane.details.iter().any(|line| line == "Teams: 3"));
    let main_org = browser_items
        .iter()
        .find(|item| item.kind == "org" && item.title == "Main Org.")
        .expect("main org browser item");
    assert_eq!(
        main_org.meta,
        "orgId=1  dashboards=2  folders=1  datasources=2  defaults=1"
    );
    assert!(main_org.details.iter().any(|line| line == "Org: Main Org."));
    assert!(main_org.details.iter().any(|line| line == "Org ID: 1"));
    assert!(main_org
        .details
        .iter()
        .any(|line| line == "Datasource types: loki:1, prometheus:1"));

    let folder = browser_items
        .iter()
        .find(|item| item.kind == "folder" && item.title == "Platform")
        .expect("folder browser item");
    assert_eq!(
        folder.meta,
        "depth=2 path=Platform / Infra org=Main Org. uid=platform"
    );
    assert!(folder.details.iter().any(|line| line == "Depth: 2"));
    assert!(folder
        .details
        .iter()
        .any(|line| line == "Path: Platform / Infra"));
    assert!(folder.details.iter().any(|line| line == "Org: Main Org."));
    assert!(folder.details.iter().any(|line| line == "UID: platform"));

    let datasource_type = browser_items
        .iter()
        .find(|item| item.kind == "datasource-type" && item.title == "loki")
        .expect("datasource-type browser item");
    assert_eq!(datasource_type.meta, "count=1");
    assert!(datasource_type
        .details
        .iter()
        .any(|line| line == "Type: loki"));
    assert!(datasource_type
        .details
        .iter()
        .any(|line| line == "Count: 1"));

    let datasource = browser_items
        .iter()
        .find(|item| item.kind == "datasource" && item.title == "loki-main")
        .expect("datasource browser item");
    assert_eq!(datasource.meta, "loki  org=Main Org.  default=false");
    assert!(datasource.details.iter().any(|line| line == "Type: loki"));
    assert!(datasource
        .details
        .iter()
        .any(|line| line == "URL: http://loki:3100"));
    assert!(
        browser_items.iter().all(|item| item.kind != "warning"),
        "unexpected warning browser items: {browser_items:?}"
    );
}
