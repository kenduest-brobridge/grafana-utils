use super::*;
use serde_json::json;
use std::path::PathBuf;

fn sample_plan_input(prune: bool) -> DashboardPlanInput {
    let folder_inventory = vec![FolderInventoryItem {
        uid: "infra".to_string(),
        title: "Infra".to_string(),
        path: "Platform / Infra".to_string(),
        parent_uid: None,
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    }];

    let local_same = LocalDashboard {
        file_path: "./dashboards/raw/Platform/Infra/cpu-main.json".to_string(),
        dashboard: json!({
            "uid": "cpu-main",
            "title": "CPU Overview",
            "panels": []
        }),
        dashboard_uid: "cpu-main".to_string(),
        title: "CPU Overview".to_string(),
        folder_uid: "infra".to_string(),
        folder_path: "Platform / Infra".to_string(),
    };

    let local_create = LocalDashboard {
        file_path: "./dashboards/raw/Platform/Infra/new.json".to_string(),
        dashboard: json!({
            "uid": "new-dash",
            "title": "New Dashboard",
            "panels": []
        }),
        dashboard_uid: "new-dash".to_string(),
        title: "New Dashboard".to_string(),
        folder_uid: "infra".to_string(),
        folder_path: "Platform / Infra".to_string(),
    };

    let live_same = LiveDashboard {
        uid: "cpu-main".to_string(),
        title: "CPU Overview".to_string(),
        folder_uid: "infra".to_string(),
        folder_path: "Platform / Infra".to_string(),
        version: Some(7),
        evidence: Vec::new(),
        payload: json!({
            "uid": "cpu-main",
            "title": "CPU Overview",
            "panels": []
        }),
    };

    let live_extra = LiveDashboard {
        uid: "orphan".to_string(),
        title: "Orphan".to_string(),
        folder_uid: "infra".to_string(),
        folder_path: "Platform / Infra".to_string(),
        version: Some(2),
        evidence: Vec::new(),
        payload: json!({
            "uid": "orphan",
            "title": "Orphan",
            "panels": []
        }),
    };

    let org = OrgPlanInput {
        source_org_id: Some("1".to_string()),
        source_org_name: "Main Org.".to_string(),
        target_org_id: Some("1".to_string()),
        target_org_name: "Main Org.".to_string(),
        org_action: "current-org".to_string(),
        input_dir: PathBuf::from("./dashboards/raw"),
        local_dashboards: vec![local_same, local_create],
        live_dashboards: vec![live_same, live_extra],
        live_datasources: Vec::new(),
        folder_inventory,
    };

    DashboardPlanInput {
        scope: "current-org".to_string(),
        input_type: "raw".to_string(),
        prune,
        org,
    }
}

#[test]
fn build_dashboard_plan_reports_same_create_and_delete_candidates() {
    let report = build_dashboard_plan(sample_plan_input(true));

    assert_eq!(report.kind, PLAN_KIND);
    assert_eq!(report.schema_version, PLAN_SCHEMA_VERSION);
    assert_eq!(report.summary.checked, 3);
    assert_eq!(report.summary.same, 1);
    assert_eq!(report.summary.create, 1);
    assert_eq!(report.summary.delete, 1);
    assert_eq!(report.summary.extra, 0);
    assert_eq!(report.actions.len(), 3);
    assert!(report.actions.iter().any(|action| action.action == "same"));
    assert!(report
        .actions
        .iter()
        .any(|action| action.action == "would-create"));
    assert!(report
        .actions
        .iter()
        .any(|action| action.action == "would-delete"));
}

#[test]
fn dashboard_plan_json_has_contract_shape() {
    let report = build_dashboard_plan(sample_plan_input(false));
    let json = build_dashboard_plan_json(&report).unwrap();

    assert_eq!(json["kind"], PLAN_KIND);
    assert_eq!(json["schemaVersion"], PLAN_SCHEMA_VERSION);
    assert_eq!(json["summary"]["checked"], 3);
    assert_eq!(json["orgs"].as_array().unwrap().len(), 1);
    assert_eq!(json["actions"].as_array().unwrap().len(), 3);
    assert!(json["actions"][0]["actionId"]
        .as_str()
        .unwrap()
        .starts_with("org:1/dashboard:"));
}

#[test]
fn dashboard_plan_table_and_text_render_are_stable() {
    let report = build_dashboard_plan(sample_plan_input(false));
    let table = render_plan_table(
        &report,
        false,
        true,
        &["action_id".to_string(), "dashboard_title".to_string()],
    );
    assert!(table[0].contains("ACTION_ID"));
    assert!(table[0].contains("DASHBOARD_TITLE"));
    let text = render_plan_text(&report, false);
    assert!(text.iter().any(|line| line.contains("would-create")));
    assert!(text.iter().all(|line| !line.contains("action=same")));
}
