use super::*;

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
