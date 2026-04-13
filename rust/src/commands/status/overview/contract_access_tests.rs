use super::*;

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
