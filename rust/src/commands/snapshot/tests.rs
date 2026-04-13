//! Snapshot bundle helpers and review entrypoints for all-org export consistency.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::access::{AccessCommand, OrgCommand, ServiceAccountCommand, TeamCommand, UserCommand};
use crate::common::sanitize_path_component;
use crate::dashboard::{
    CommonCliArgs, DashboardCommand, EXPORT_METADATA_FILENAME, TOOL_SCHEMA_VERSION,
};
use crate::datasource::DatasourceGroupCommand;
use crate::overview::OverviewOutputFormat;
use crate::snapshot::{
    build_snapshot_overview_args, build_snapshot_paths, build_snapshot_review_browser_items,
    build_snapshot_review_document, build_snapshot_review_summary_lines,
    build_snapshot_root_metadata, render_snapshot_review_text,
    run_snapshot_export_selected_with_handlers, run_snapshot_export_with_handlers,
    run_snapshot_review_document_with_handler, SnapshotCliArgs, SnapshotExportArgs,
    SnapshotExportLane, SnapshotExportSelection, SnapshotReviewArgs,
    SNAPSHOT_DATASOURCE_EXPORT_FILENAME, SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME,
    SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND, SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION,
};
use crate::staged_export_scopes::{
    resolve_dashboard_export_scope_dirs, resolve_datasource_export_scope_dirs,
};
use clap::Parser;
use serde_json::json;
use serde_json::Value;
use tempfile::tempdir;

fn sample_common_args() -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: Some("prod".to_string()),
        url: "http://grafana.example.com".to_string(),
        api_token: Some("token".to_string()),
        username: Some("admin".to_string()),
        password: Some("admin".to_string()),
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

fn write_snapshot_dashboard_metadata(dashboard_root: &Path, orgs: &[(&str, &str, usize)]) {
    let org_entries: Vec<Value> = orgs
        .iter()
        .map(|(org_id, org, dashboard_count)| {
            json!({
                "org": org,
                "orgId": org_id,
                "dashboardCount": dashboard_count,
                "exportDir": format!("org_{org_id}_{}", sanitize_path_component(org))
            })
        })
        .collect();
    let dashboard_count = orgs.iter().map(|(_, _, count)| *count).sum::<usize>();
    fs::create_dir_all(dashboard_root).unwrap();
    fs::write(
        dashboard_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "root",
            "dashboardCount": dashboard_count,
            "indexFile": "index.json",
            "orgCount": orgs.len(),
            "orgs": org_entries
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_snapshot_dashboard_index(dashboard_root: &Path, folders: &[Value]) {
    fs::create_dir_all(dashboard_root).unwrap();
    fs::write(
        dashboard_root.join("index.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "items": [],
            "variants": {
                "raw": null,
                "prompt": null,
                "provisioning": null
            },
            "folders": folders
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_snapshot_datasource_root_metadata(
    datasource_root: &Path,
    datasource_count: usize,
    variant: &str,
) {
    fs::create_dir_all(datasource_root).unwrap();
    fs::write(
        datasource_root.join(SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION,
            "kind": SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND,
            "variant": variant,
            "resource": "datasource",
            "orgCount": 1,
            "datasourceCount": datasource_count,
            "datasourcesFile": SNAPSHOT_DATASOURCE_EXPORT_FILENAME,
            "indexFile": "index.json",
            "format": "grafana-datasource-inventory-v1"
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_datasource_inventory_rows(datasource_root: &Path, rows: &[Value]) {
    fs::create_dir_all(datasource_root).unwrap();
    fs::write(
        datasource_root.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME),
        serde_json::to_string_pretty(&Value::Array(rows.to_vec())).unwrap(),
    )
    .unwrap();
}

fn write_complete_dashboard_scope(scope_dir: &Path) {
    fs::create_dir_all(scope_dir.join("raw")).unwrap();
    fs::write(scope_dir.join("raw/index.json"), "[]").unwrap();

    fs::create_dir_all(scope_dir.join("prompt")).unwrap();
    fs::write(scope_dir.join("prompt/index.json"), "[]").unwrap();

    fs::create_dir_all(scope_dir.join("provisioning/provisioning")).unwrap();
    fs::write(scope_dir.join("provisioning/index.json"), "[]").unwrap();
    fs::write(
        scope_dir.join("provisioning/provisioning/dashboards.yaml"),
        "apiVersion: 1\nproviders: []\n",
    )
    .unwrap();
}

fn write_datasource_provisioning_lane(scope_dir: &Path) {
    fs::create_dir_all(scope_dir.join("provisioning")).unwrap();
    fs::write(
        scope_dir.join("provisioning/datasources.yaml"),
        "apiVersion: 1\n",
    )
    .unwrap();
}

fn write_snapshot_datasource_inventory_root(
    datasource_root: &Path,
    rows: &[Value],
    datasource_count: usize,
    variant: &str,
) {
    write_snapshot_datasource_root_metadata(datasource_root, datasource_count, variant);
    write_datasource_inventory_rows(datasource_root, rows);
}

fn write_snapshot_access_lane_bundle(
    lane_root: &Path,
    payload_filename: &str,
    kind: &str,
    record_count: usize,
) {
    fs::create_dir_all(lane_root).unwrap();
    fs::write(
        lane_root.join(payload_filename),
        serde_json::to_string_pretty(&Value::Array(
            (0..record_count)
                .map(|index| json!({ "id": index + 1 }))
                .collect(),
        ))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        lane_root.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "kind": kind,
            "version": 1,
            "recordCount": record_count,
            "sourceUrl": "http://grafana.example.com",
            "sourceDir": lane_root.to_string_lossy(),
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn snapshot_export_derives_expected_child_paths() {
    let paths = build_snapshot_paths(&PathBuf::from("./snapshot"));

    assert_eq!(paths.dashboards, PathBuf::from("./snapshot/dashboards"));
    assert_eq!(paths.datasources, PathBuf::from("./snapshot/datasources"));
    assert_eq!(paths.access, PathBuf::from("./snapshot/access"));
    assert_eq!(paths.access_users, PathBuf::from("./snapshot/access/users"));
    assert_eq!(paths.access_teams, PathBuf::from("./snapshot/access/teams"));
    assert_eq!(paths.access_orgs, PathBuf::from("./snapshot/access/orgs"));
    assert_eq!(
        paths.access_service_accounts,
        PathBuf::from("./snapshot/access/service-accounts")
    );
    assert_eq!(
        paths.metadata,
        PathBuf::from("./snapshot/snapshot-metadata.json")
    );
}

#[test]
fn snapshot_review_builds_overview_args_for_interactive_output() {
    let review_args = SnapshotReviewArgs {
        input_dir: PathBuf::from("./snapshot"),
        interactive: false,
        output_format: OverviewOutputFormat::Interactive,
    };

    let overview_args = build_snapshot_overview_args(&review_args);

    assert_eq!(
        overview_args.dashboard_export_dir,
        Some(PathBuf::from("./snapshot/dashboards"))
    );
    assert_eq!(
        overview_args.datasource_export_dir,
        Some(PathBuf::from("./snapshot/datasources"))
    );
    assert_eq!(
        overview_args.access_user_export_dir,
        Some(PathBuf::from("./snapshot/access/users"))
    );
    assert_eq!(
        overview_args.access_team_export_dir,
        Some(PathBuf::from("./snapshot/access/teams"))
    );
    assert_eq!(
        overview_args.access_org_export_dir,
        Some(PathBuf::from("./snapshot/access/orgs"))
    );
    assert_eq!(
        overview_args.access_service_account_export_dir,
        Some(PathBuf::from("./snapshot/access/service-accounts"))
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
            input_dir: PathBuf::from("./snapshot"),
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
fn snapshot_export_wrapper_calls_dashboard_then_datasource_runners() {
    let temp = tempdir().unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let dashboard_args = Rc::new(RefCell::new(None));
    let datasource_args = Rc::new(RefCell::new(None));

    let export_args = SnapshotExportArgs {
        common: sample_common_args(),
        output_dir: temp.path().join("snapshot"),
        overwrite: true,
        prompt: false,
    };

    let dashboard_calls = Rc::clone(&calls);
    let dashboard_seen = Rc::clone(&dashboard_args);
    let datasource_calls = Rc::clone(&calls);
    let datasource_seen = Rc::clone(&datasource_args);
    let access_calls = Rc::clone(&calls);

    run_snapshot_export_with_handlers(
        export_args,
        move |args| {
            dashboard_calls.borrow_mut().push("dashboard".to_string());
            match args.command {
                DashboardCommand::Export(inner) => {
                    *dashboard_seen.borrow_mut() = Some(inner);
                    Ok(())
                }
                other => panic!("unexpected dashboard command: {:?}", other),
            }
        },
        move |command| {
            datasource_calls.borrow_mut().push("datasource".to_string());
            match command {
                DatasourceGroupCommand::Export(inner) => {
                    *datasource_seen.borrow_mut() = Some(inner);
                    Ok(())
                }
                other => panic!("unexpected datasource command: {:?}", other),
            }
        },
        move |cli| {
            match cli.command {
                AccessCommand::User {
                    command: UserCommand::Export(_),
                } => access_calls.borrow_mut().push("access-user".to_string()),
                AccessCommand::Team {
                    command: TeamCommand::Export(_),
                } => access_calls.borrow_mut().push("access-team".to_string()),
                AccessCommand::Org {
                    command: OrgCommand::Export(_),
                } => access_calls.borrow_mut().push("access-org".to_string()),
                AccessCommand::ServiceAccount {
                    command: ServiceAccountCommand::Export(_),
                } => access_calls
                    .borrow_mut()
                    .push("access-service-account".to_string()),
                other => panic!("unexpected access command: {:?}", other),
            }
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(
        *calls.borrow(),
        vec![
            "dashboard".to_string(),
            "datasource".to_string(),
            "access-user".to_string(),
            "access-team".to_string(),
            "access-org".to_string(),
            "access-service-account".to_string()
        ]
    );

    let dashboard_args = dashboard_args.borrow().clone().expect("dashboard args");
    let datasource_args = datasource_args.borrow().clone().expect("datasource args");
    assert!(dashboard_args.all_orgs);
    assert_eq!(
        dashboard_args.output_dir,
        temp.path().join("snapshot").join("dashboards")
    );
    assert!(datasource_args.all_orgs);
    assert_eq!(
        datasource_args.output_dir,
        temp.path().join("snapshot").join("datasources")
    );
    assert!(dashboard_args.overwrite);
    assert!(datasource_args.overwrite);
}

#[test]
fn snapshot_export_selected_with_handlers_runs_only_selected_lanes() {
    let temp = tempdir().unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let selection = SnapshotExportSelection {
        lanes: vec![
            SnapshotExportLane::Datasources,
            SnapshotExportLane::AccessTeams,
            SnapshotExportLane::AccessServiceAccounts,
        ],
    };
    let export_args = SnapshotExportArgs {
        common: sample_common_args(),
        output_dir: temp.path().join("snapshot"),
        overwrite: false,
        prompt: false,
    };

    let dashboard_calls = Rc::clone(&calls);
    let datasource_calls = Rc::clone(&calls);
    let access_calls = Rc::clone(&calls);

    run_snapshot_export_selected_with_handlers(
        export_args,
        &selection,
        move |_args| {
            dashboard_calls.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        move |command| match command {
            DatasourceGroupCommand::Export(_) => {
                datasource_calls.borrow_mut().push("datasource".to_string());
                Ok(())
            }
            other => panic!("unexpected datasource command: {:?}", other),
        },
        move |cli| {
            match cli.command {
                AccessCommand::Team {
                    command: TeamCommand::Export(_),
                } => access_calls.borrow_mut().push("access-team".to_string()),
                AccessCommand::ServiceAccount {
                    command: ServiceAccountCommand::Export(_),
                } => access_calls
                    .borrow_mut()
                    .push("access-service-account".to_string()),
                other => panic!("unexpected access command: {:?}", other),
            }
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(
        *calls.borrow(),
        vec![
            "datasource".to_string(),
            "access-team".to_string(),
            "access-service-account".to_string()
        ]
    );
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

#[test]
fn snapshot_root_metadata_captures_access_and_staged_lane_counts() {
    let temp = tempdir().unwrap();
    let snapshot_root = temp.path().join("snapshot");
    let dashboard_root = snapshot_root.join("dashboards");
    let datasource_root = snapshot_root.join("datasources");
    let access_root = snapshot_root.join("access");

    write_snapshot_dashboard_metadata(&dashboard_root, &[("1", "Main Org.", 2)]);
    write_snapshot_dashboard_index(&dashboard_root, &[]);
    write_snapshot_datasource_root_metadata(&datasource_root, 3, "root");
    write_datasource_inventory_rows(&datasource_root, &[]);
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

    let metadata = build_snapshot_root_metadata(&snapshot_root, &sample_common_args()).unwrap();
    assert_eq!(metadata["kind"], json!("grafana-utils-snapshot-root"));
    assert_eq!(metadata["summary"]["dashboardCount"], json!(2));
    assert_eq!(metadata["summary"]["datasourceCount"], json!(3));
    assert_eq!(metadata["summary"]["accessUserCount"], json!(2));
    assert_eq!(metadata["summary"]["accessTeamCount"], json!(3));
    assert_eq!(metadata["summary"]["accessOrgCount"], json!(1));
    assert_eq!(metadata["summary"]["accessServiceAccountCount"], json!(4));
    assert_eq!(
        metadata["lanes"]["access"]["users"]["recordCount"],
        json!(2)
    );
    assert_eq!(
        metadata["lanes"]["access"]["teams"]["recordCount"],
        json!(3)
    );
    assert_eq!(metadata["lanes"]["access"]["orgs"]["recordCount"], json!(1));
    assert_eq!(
        metadata["lanes"]["access"]["serviceAccounts"]["recordCount"],
        json!(4)
    );
    assert_eq!(
        metadata["source"]["url"],
        json!("http://grafana.example.com")
    );
}

#[test]
fn staged_export_scope_resolver_prefers_dashboard_export_dirs_when_they_exist() {
    let temp = tempdir().unwrap();
    let dashboard_root = temp.path().join("dashboards");
    let main_scope = dashboard_root.join("org_1_Main_Org");
    let ops_scope = dashboard_root.join("org_2_Ops_Org");

    write_complete_dashboard_scope(&main_scope);
    write_complete_dashboard_scope(&ops_scope);

    let dashboard_metadata = json!({
        "kind": "grafana-utils-dashboard-export-index",
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "variant": "root",
        "dashboardCount": 2,
        "indexFile": "index.json",
        "orgCount": 2,
        "orgs": [
            {
                "org": "Main Org.",
                "orgId": "1",
                "dashboardCount": 1,
                "exportDir": "org_1_Main_Org",
            },
            {
                "org": "Ops Org",
                "orgId": "2",
                "dashboardCount": 1,
                "exportDir": "org_2_Ops_Org",
            },
        ],
    });

    let scopes = resolve_dashboard_export_scope_dirs(&dashboard_root, &dashboard_metadata);

    assert_eq!(scopes, vec![main_scope, ops_scope]);
}

#[test]
fn staged_export_scope_resolver_falls_back_to_single_dashboard_root_when_export_dirs_are_missing() {
    let temp = tempdir().unwrap();
    let dashboard_root = temp.path().join("dashboards");
    write_complete_dashboard_scope(&dashboard_root);

    let dashboard_metadata = json!({
        "kind": "grafana-utils-dashboard-export-index",
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "variant": "root",
        "dashboardCount": 1,
        "indexFile": "index.json",
        "orgCount": 1,
        "orgs": [
            {
                "org": "Main Org.",
                "orgId": "1",
                "dashboardCount": 1,
                "exportDir": "org_1_Main_Org",
            },
        ],
    });

    let scopes = resolve_dashboard_export_scope_dirs(&dashboard_root, &dashboard_metadata);

    assert_eq!(scopes, vec![dashboard_root]);
}

#[test]
fn staged_export_scope_resolver_discovers_real_datasource_scope_dirs_and_ignores_empty_siblings() {
    let temp = tempdir().unwrap();
    let datasource_root = temp.path().join("datasources");
    let root_scope = datasource_root.clone();
    let main_scope = datasource_root.join("org_1_Main_Org");
    let ops_scope = datasource_root.join("org_2_Ops_Org");
    let ignored_scope = datasource_root.join("org_3_Empty");
    let ignored_dir = datasource_root.join("notes");

    write_snapshot_datasource_root_metadata(&datasource_root, 2, "root");
    write_datasource_inventory_rows(
        &datasource_root,
        &[json!({
            "uid": "prom-main",
            "name": "prom-main",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": "1"
        })],
    );
    write_datasource_inventory_rows(
        &main_scope,
        &[json!({
            "uid": "prom-main",
            "name": "prom-main",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": "1"
        })],
    );
    write_datasource_provisioning_lane(&ops_scope);
    fs::create_dir_all(&ignored_scope).unwrap();
    fs::create_dir_all(&ignored_dir).unwrap();

    let mut scopes = resolve_datasource_export_scope_dirs(&datasource_root);
    scopes.sort();

    assert_eq!(scopes, vec![root_scope, main_scope, ops_scope]);
}

#[test]
fn snapshot_review_wrapper_normalizes_combined_datasource_root_before_building_document() {
    let temp = tempdir().unwrap();
    let snapshot_root = temp.path().join("snapshot");
    let dashboard_root = snapshot_root.join("dashboards");
    let datasource_root = snapshot_root.join("datasources");

    write_snapshot_dashboard_metadata(
        &dashboard_root,
        &[("1", "Main Org.", 1), ("2", "Ops Org", 1)],
    );
    write_complete_dashboard_scope(&dashboard_root.join("org_1_Main_Org"));
    write_complete_dashboard_scope(&dashboard_root.join("org_2_Ops_Org"));
    write_snapshot_datasource_inventory_root(
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
                "uid": "tempo-ops",
                "name": "tempo-ops",
                "type": "tempo",
                "url": "http://tempo:3200",
                "isDefault": false,
                "org": "Ops Org",
                "orgId": "2"
            }),
        ],
        2,
        "all-orgs-root",
    );
    write_datasource_provisioning_lane(&datasource_root);
    write_datasource_inventory_rows(
        &datasource_root.join("org_1_Main_Org"),
        &[json!({
            "uid": "prom-main",
            "name": "prom-main",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": "1"
        })],
    );
    write_datasource_inventory_rows(
        &datasource_root.join("org_2_Ops_Org"),
        &[json!({
            "uid": "tempo-ops",
            "name": "tempo-ops",
            "type": "tempo",
            "url": "http://tempo:3200",
            "isDefault": false,
            "org": "Ops Org",
            "orgId": "2"
        })],
    );
    write_datasource_provisioning_lane(&datasource_root.join("org_1_Main_Org"));
    write_datasource_provisioning_lane(&datasource_root.join("org_2_Ops_Org"));

    let seen = Rc::new(RefCell::new(None));
    let review_args = SnapshotReviewArgs {
        input_dir: snapshot_root,
        interactive: false,
        output_format: OverviewOutputFormat::Text,
    };
    let seen_args = Rc::clone(&seen);
    run_snapshot_review_document_with_handler(review_args, move |document| {
        *seen_args.borrow_mut() = Some(document);
        Ok(())
    })
    .unwrap();

    let document = seen.borrow().clone().expect("snapshot review document");
    assert_eq!(document["summary"]["orgCount"], json!(2));
    assert_eq!(document["summary"]["dashboardOrgCount"], json!(2));
    assert_eq!(document["summary"]["datasourceOrgCount"], json!(2));
    assert_eq!(document["summary"]["dashboardCount"], json!(2));
    assert_eq!(document["summary"]["datasourceCount"], json!(2));
    assert_eq!(document["lanes"]["dashboard"]["scopeCount"], json!(2));
    assert_eq!(document["lanes"]["datasource"]["scopeCount"], json!(3));
    assert_eq!(
        document["lanes"]["datasource"]["inventoryExpectedScopeCount"],
        json!(2)
    );
    assert_eq!(
        document["lanes"]["datasource"]["inventoryScopeCount"],
        json!(2)
    );
    assert_eq!(
        document["lanes"]["datasource"]["provisioningExpectedScopeCount"],
        json!(3)
    );
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
}

#[test]
fn snapshot_review_document_reports_missing_lane_warnings_for_incomplete_scope_dirs() {
    let temp = tempdir().unwrap();
    let snapshot_root = temp.path().join("snapshot");
    let dashboard_root = snapshot_root.join("dashboards");
    let datasource_root = snapshot_root.join("datasources");

    write_snapshot_dashboard_metadata(
        &dashboard_root,
        &[("1", "Main Org.", 2), ("2", "Ops Org", 1)],
    );
    write_complete_dashboard_scope(&dashboard_root.join("org_1_Main_Org"));
    fs::create_dir_all(dashboard_root.join("org_2_Ops_Org/raw")).unwrap();
    fs::write(dashboard_root.join("org_2_Ops_Org/raw/index.json"), "[]").unwrap();
    fs::create_dir_all(dashboard_root.join("org_2_Ops_Org/prompt")).unwrap();
    fs::write(dashboard_root.join("org_2_Ops_Org/prompt/index.json"), "[]").unwrap();
    fs::create_dir_all(dashboard_root.join("org_2_Ops_Org/provisioning")).unwrap();
    fs::write(
        dashboard_root.join("org_2_Ops_Org/provisioning/index.json"),
        "[]",
    )
    .unwrap();
    write_snapshot_datasource_root_metadata(&datasource_root, 2, "root");
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

    let document =
        build_snapshot_review_document(&dashboard_root, &datasource_root, &datasource_root)
            .unwrap();
    let warning_codes: Vec<&str> = document["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|warning| warning["code"].as_str().unwrap())
        .collect();

    assert_eq!(warning_codes, vec!["dashboard-provisioning-lane-missing"]);
}

#[test]
fn snapshot_review_document_reports_observational_warnings_for_org_mismatch() {
    let temp = tempdir().unwrap();
    let snapshot_root = temp.path().join("snapshot");
    let dashboard_root = snapshot_root.join("dashboards");
    let datasource_root = snapshot_root.join("datasources");

    write_snapshot_dashboard_metadata(
        &dashboard_root,
        &[("1", "Main Org.", 2), ("2", "Ops Org", 1)],
    );
    write_snapshot_datasource_root_metadata(&datasource_root, 1, "all-orgs-root");
    fs::create_dir_all(datasource_root.join("org_1_Main_Org")).unwrap();
    write_datasource_inventory_rows(
        &datasource_root.join("org_1_Main_Org"),
        &[json!({
            "uid": "prom-main",
            "name": "prom-main",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": "1"
        })],
    );

    let seen = Rc::new(RefCell::new(None));
    let review_args = SnapshotReviewArgs {
        input_dir: snapshot_root,
        interactive: false,
        output_format: OverviewOutputFormat::Text,
    };
    let seen_args = Rc::clone(&seen);
    run_snapshot_review_document_with_handler(review_args, move |document| {
        *seen_args.borrow_mut() = Some(document);
        Ok(())
    })
    .unwrap();
    let document = seen.borrow().clone().expect("snapshot review document");
    let warnings = document["warnings"].as_array().expect("warnings");
    let codes: Vec<&str> = warnings
        .iter()
        .map(|warning| warning["code"].as_str().unwrap())
        .collect();

    assert!(codes.contains(&"org-count-mismatch"));
    assert!(codes.contains(&"org-partial-coverage"));
    assert_eq!(document["summary"]["dashboardOrgCount"], json!(2));
    assert_eq!(document["summary"]["datasourceOrgCount"], json!(1));

    let browser_items = build_snapshot_review_browser_items(&document).unwrap();
    assert!(browser_items
        .iter()
        .any(|item| item.kind == "warning" && item.title == "org-count-mismatch"));
    let warning = browser_items
        .iter()
        .find(|item| item.kind == "warning" && item.title == "org-count-mismatch")
        .expect("warning browser item");
    assert_eq!(
        warning.meta,
        "Dashboard export covers 2 org(s) while datasource inventory covers 1 org(s)."
    );
    assert!(warning
        .details
        .iter()
        .any(|line| line == "Code: org-count-mismatch"));
    assert!(warning.details.iter().any(|line| line == "Message: Dashboard export covers 2 org(s) while datasource inventory covers 1 org(s)."));
}
