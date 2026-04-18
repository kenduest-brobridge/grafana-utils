//! Snapshot path and metadata tests.

use super::tests_fixtures::{
    sample_common_args, write_datasource_inventory_rows, write_snapshot_access_lane_bundle,
    write_snapshot_dashboard_index, write_snapshot_dashboard_metadata,
    write_snapshot_datasource_root_metadata,
};
use crate::snapshot::{build_snapshot_paths, build_snapshot_root_metadata};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn snapshot_export_derives_expected_child_paths() {
    let paths = build_snapshot_paths(&std::path::PathBuf::from("./snapshot"));

    assert_eq!(
        paths.dashboards,
        std::path::PathBuf::from("./snapshot/dashboards")
    );
    assert_eq!(
        paths.datasources,
        std::path::PathBuf::from("./snapshot/datasources")
    );
    assert_eq!(paths.access, std::path::PathBuf::from("./snapshot/access"));
    assert_eq!(
        paths.access_users,
        std::path::PathBuf::from("./snapshot/access/users")
    );
    assert_eq!(
        paths.access_teams,
        std::path::PathBuf::from("./snapshot/access/teams")
    );
    assert_eq!(
        paths.access_orgs,
        std::path::PathBuf::from("./snapshot/access/orgs")
    );
    assert_eq!(
        paths.access_service_accounts,
        std::path::PathBuf::from("./snapshot/access/service-accounts")
    );
    assert_eq!(
        paths.metadata,
        std::path::PathBuf::from("./snapshot/snapshot-metadata.json")
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
