use super::*;
use crate::common::message;
use crate::dashboard::{
    CommonCliArgs, DashboardPlanOutputFormat, InspectExportInputType, PlanArgs,
    EXPORT_METADATA_FILENAME, TOOL_SCHEMA_VERSION,
};
use reqwest::Method;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn make_common_args(base_url: String) -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: None,
        url: base_url,
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

fn make_basic_common_args(base_url: String) -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: None,
        url: base_url,
        api_token: None,
        username: Some("admin".to_string()),
        password: Some("admin".to_string()),
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

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
        orgs: vec![org],
    }
}

fn sample_missing_org_input(org_action: &str) -> DashboardPlanInput {
    let local = LocalDashboard {
        file_path: "./dashboards/raw/cpu-nine.json".to_string(),
        dashboard: json!({
            "uid": "cpu-nine",
            "title": "CPU Nine",
            "panels": []
        }),
        dashboard_uid: "cpu-nine".to_string(),
        title: "CPU Nine".to_string(),
        folder_uid: "general".to_string(),
        folder_path: "General".to_string(),
    };

    DashboardPlanInput {
        scope: "export-org".to_string(),
        input_type: "raw".to_string(),
        prune: false,
        orgs: vec![OrgPlanInput {
            source_org_id: Some("9".to_string()),
            source_org_name: "Ops Org".to_string(),
            target_org_id: None,
            target_org_name: "<new>".to_string(),
            org_action: org_action.to_string(),
            input_dir: PathBuf::from("./exports/org_9_Ops_Org/raw"),
            local_dashboards: vec![local],
            live_dashboards: Vec::new(),
            live_datasources: Vec::new(),
            folder_inventory: Vec::new(),
        }],
    }
}

fn make_plan_args(input_dir: PathBuf) -> PlanArgs {
    PlanArgs {
        common: make_basic_common_args("http://127.0.0.1:3000".to_string()),
        input_dir,
        input_type: InspectExportInputType::Raw,
        org_id: None,
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        prune: false,
        show_same: false,
        output_columns: Vec::new(),
        list_columns: false,
        no_header: false,
        output_format: DashboardPlanOutputFormat::Text,
    }
}

fn write_export_org_scope(root: &Path, org_id: &str, org_name: &str, uid: &str) -> PathBuf {
    let raw_dir = root.join(format!("org_{org_id}_{org_name}")).join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "org": org_name,
            "orgId": org_id
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("index.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": uid,
                "title": format!("Dashboard {uid}"),
                "path": format!("{uid}.json"),
                "format": "grafana-web-import-preserve-uid",
                "org": org_name,
                "orgId": org_id
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join(format!("{uid}.json")),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": uid,
                "title": format!("Dashboard {uid}"),
                "panels": [],
                "version": 1
            },
            "meta": {"folderUid": "general"}
        }))
        .unwrap(),
    )
    .unwrap();
    raw_dir
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

#[test]
fn build_dashboard_plan_aggregates_multiple_orgs_and_would_create_counts() {
    let mut input = sample_plan_input(true);
    input.scope = "export-org".to_string();
    input.orgs.push(OrgPlanInput {
        source_org_id: Some("9".to_string()),
        source_org_name: "Ops Org".to_string(),
        target_org_id: None,
        target_org_name: "<new>".to_string(),
        org_action: "would-create".to_string(),
        input_dir: PathBuf::from("./exports/org_9_Ops_Org/raw"),
        local_dashboards: vec![LocalDashboard {
            file_path: "./exports/org_9_Ops_Org/raw/ops.json".to_string(),
            dashboard: json!({
                "uid": "ops",
                "title": "Ops",
                "panels": []
            }),
            dashboard_uid: "ops".to_string(),
            title: "Ops".to_string(),
            folder_uid: "general".to_string(),
            folder_path: "General".to_string(),
        }],
        live_dashboards: Vec::new(),
        live_datasources: Vec::new(),
        folder_inventory: Vec::new(),
    });

    let report = build_dashboard_plan(input);

    assert_eq!(report.summary.org_count, 2);
    assert_eq!(report.summary.would_create_org_count, 1);
    assert_eq!(report.summary.same, 1);
    assert_eq!(report.summary.create, 2);
    assert_eq!(report.summary.delete, 1);
    assert!(report
        .orgs
        .iter()
        .any(|org| org.org_action == "would-create"));
    assert!(report.actions.iter().any(|action| action
        .review_hints
        .iter()
        .any(|hint| hint == "target-org-would-create")));
}

#[test]
fn build_dashboard_plan_blocks_missing_target_orgs_without_live_state() {
    let report = build_dashboard_plan(sample_missing_org_input("missing"));

    assert_eq!(report.summary.org_count, 1);
    assert_eq!(report.summary.blocked, 1);
    assert_eq!(report.summary.warning, 0);
    let action = &report.actions[0];
    assert_eq!(action.action, "would-create");
    assert_eq!(action.status, "blocked");
    assert_eq!(action.blocked_reason.as_deref(), Some("target-org-missing"));
    assert!(action
        .review_hints
        .iter()
        .any(|hint| hint == "target-org-missing"));
}

#[test]
fn build_dashboard_plan_marks_would_create_target_orgs_as_warning() {
    let report = build_dashboard_plan(sample_missing_org_input("would-create"));

    assert_eq!(report.summary.org_count, 1);
    assert_eq!(report.summary.warning, 1);
    assert_eq!(report.summary.blocked, 0);
    let action = &report.actions[0];
    assert_eq!(action.action, "would-create");
    assert_eq!(action.status, "warning");
    assert_eq!(action.blocked_reason, None);
    assert!(action
        .review_hints
        .iter()
        .any(|hint| hint == "target-org-would-create"));
}

#[test]
fn collect_plan_input_with_export_org_requires_basic_auth() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("exports");
    write_export_org_scope(&root, "2", "Org_Two", "cpu-two");
    let mut args = make_plan_args(root);
    args.common = make_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;

    let mut request_count = 0usize;
    let error = collect_plan_input_with_request(&args, &mut |_method, _path, _params, _payload| {
        request_count += 1;
        Ok(Some(Value::Null))
    })
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("Dashboard plan with --use-export-org requires Basic auth"));
    assert_eq!(request_count, 0);
}

#[test]
fn collect_plan_input_with_export_org_filters_selected_orgs_and_collects_live_state() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("exports");
    let org_two_raw = write_export_org_scope(&root, "2", "Org_Two", "cpu-two");
    write_export_org_scope(&root, "9", "Ops_Org", "ops");
    let mut args = make_plan_args(root);
    args.use_export_org = true;
    args.only_org_id = vec![2];
    args.create_missing_orgs = false;

    let mut requests = Vec::new();
    let input = collect_plan_input_with_request(&args, &mut |method, path, _params, _payload| {
        requests.push((method.clone(), path.to_string()));
        match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 2, "name": "Org Two"},
                {"id": 99, "name": "Unrelated"}
            ]))),
            (Method::GET, "/api/datasources") => Ok(Some(json!([]))),
            (Method::GET, "/api/search") => Ok(Some(json!([
                {"uid": "cpu-two", "title": "Dashboard cpu-two", "folderUid": "general"}
            ]))),
            (Method::GET, "/api/dashboards/uid/cpu-two") => Ok(Some(json!({
                "dashboard": {
                    "uid": "cpu-two",
                    "title": "Dashboard cpu-two",
                    "version": 3,
                    "panels": []
                },
                "meta": {"folderUid": "general"}
            }))),
            _ => Err(message(format!("unexpected request {path}"))),
        }
    })
    .unwrap();

    assert_eq!(input.scope, "export-org");
    assert_eq!(input.input_type, "raw");
    assert_eq!(input.orgs.len(), 1);
    assert_eq!(input.orgs[0].source_org_id.as_deref(), Some("2"));
    assert_eq!(input.orgs[0].target_org_id.as_deref(), Some("2"));
    assert_eq!(input.orgs[0].org_action, "exists");
    assert_eq!(input.orgs[0].local_dashboards.len(), 1);
    assert_eq!(input.orgs[0].live_dashboards.len(), 1);
    assert!(requests
        .iter()
        .any(|(method, path)| *method == Method::GET && path == "/api/orgs"));
    assert!(requests
        .iter()
        .any(|(method, path)| *method == Method::GET && path == "/api/search"));
    assert!(org_two_raw.join("cpu-two.json").is_file());
}

#[test]
fn collect_plan_input_with_export_org_rejects_selected_missing_org_ids() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("exports");
    write_export_org_scope(&root, "2", "Org_Two", "cpu-two");
    let mut args = make_plan_args(root);
    args.use_export_org = true;
    args.only_org_id = vec![9];

    let error = collect_plan_input_with_request(&args, &mut |_method, _path, _params, _payload| {
        Ok(Some(Value::Null))
    })
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("did not find the selected exported org IDs (9)"));
}

#[test]
fn collect_plan_input_with_export_org_marks_missing_targets_without_live_calls() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("exports");
    write_export_org_scope(&root, "9", "Ops_Org", "ops");

    let mut missing_args = make_plan_args(root.clone());
    missing_args.use_export_org = true;
    missing_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    missing_args.create_missing_orgs = false;

    let mut missing_requests = Vec::new();
    let missing_input =
        collect_plan_input_with_request(&missing_args, &mut |method, path, _params, _payload| {
            missing_requests.push((method.clone(), path.to_string()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([]))),
                _ => Err(message(format!("unexpected request {path}"))),
            }
        })
        .unwrap();

    assert_eq!(missing_input.orgs.len(), 1);
    assert_eq!(missing_input.orgs[0].org_action, "missing");
    assert_eq!(missing_input.orgs[0].target_org_id, None);
    assert!(missing_requests
        .iter()
        .all(|(method, path)| *method == Method::GET && path == "/api/orgs"));

    let mut would_create_args = make_plan_args(root);
    would_create_args.use_export_org = true;
    would_create_args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    would_create_args.create_missing_orgs = true;

    let mut would_create_requests = Vec::new();
    let would_create_input = collect_plan_input_with_request(
        &would_create_args,
        &mut |method, path, _params, _payload| {
            would_create_requests.push((method.clone(), path.to_string()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([]))),
                _ => Err(message(format!("unexpected request {path}"))),
            }
        },
    )
    .unwrap();

    assert_eq!(would_create_input.orgs[0].org_action, "would-create");
    assert_eq!(would_create_input.orgs[0].target_org_id, None);
    assert!(would_create_requests
        .iter()
        .all(|(method, path)| *method == Method::GET && path == "/api/orgs"));
}
