//! Snapshot review wrapper and warning behavior tests.

use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

use super::tests_fixtures::{
    write_complete_dashboard_scope, write_datasource_inventory_rows,
    write_datasource_provisioning_lane, write_snapshot_dashboard_metadata,
    write_snapshot_datasource_inventory_root, write_snapshot_datasource_root_metadata,
};
use crate::overview::OverviewOutputFormat;
use crate::snapshot::{
    build_snapshot_review_browser_items, build_snapshot_review_document,
    run_snapshot_review_document_with_handler, SnapshotReviewArgs,
};
use serde_json::json;
use tempfile::tempdir;

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
