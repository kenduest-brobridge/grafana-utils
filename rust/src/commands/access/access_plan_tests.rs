use super::*;
use crate::access::cli_defs::PlanOutputFormat;
use crate::access::{parse_cli_from, CommonCliArgs};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn make_common() -> CommonCliArgs {
    CommonCliArgs {
        profile: None,
        url: "http://127.0.0.1:3000".to_string(),
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        org_id: None,
        timeout: 30,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
    }
}

fn write_user_bundle(dir: &Path) {
    fs::write(
        dir.join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor"},
                {"login": "bob", "email": "bob@example.com", "name": "Bob", "orgRole": "Viewer"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_org_bundle(dir: &Path, records: Value) {
    fs::write(
        dir.join("orgs.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-org-export-index",
            "version": 1,
            "records": records
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_service_account_bundle(dir: &Path) {
    fs::write(
        dir.join("service-accounts.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-service-account-export-index",
            "version": 1,
            "records": [
                {"name": "svc-create", "login": "sa-create", "role": "Viewer", "disabled": false, "orgId": 1}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn parse_access_plan_defaults_to_user_resource() {
    let args = parse_cli_from(["grafana-util", "plan", "--input-dir", "./access-users"]);
    match args.command {
        crate::access::AccessCommand::Plan(plan) => {
            assert!(matches!(plan.resource, AccessPlanResource::User));
            assert!(matches!(plan.output_format, PlanOutputFormat::Text));
        }
        _ => panic!("expected access plan"),
    }
}

#[test]
fn parse_access_plan_supports_all_resource() {
    let args = parse_cli_from([
        "grafana-util",
        "plan",
        "--input-dir",
        "./access",
        "--resource",
        "all",
    ]);
    match args.command {
        crate::access::AccessCommand::Plan(plan) => {
            assert!(matches!(plan.resource, AccessPlanResource::All));
        }
        _ => panic!("expected access plan"),
    }
}

#[test]
fn user_plan_builds_summary_and_renderers() {
    let temp_dir = tempdir().unwrap();
    write_user_bundle(temp_dir.path());
    let args = AccessPlanArgs {
        common: make_common(),
        input_dir: temp_dir.path().to_path_buf(),
        resource: AccessPlanResource::User,
        prune: false,
        output_columns: vec![
            "identity".to_string(),
            "action".to_string(),
            "status".to_string(),
        ],
        list_columns: false,
        no_header: false,
        show_same: false,
        output_format: PlanOutputFormat::Text,
    };
    let document = build_access_plan_document(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/org/users") => Ok(Some(json!([
                {"userId": "1", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Editor"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();

    assert_eq!(document.kind, ACCESS_PLAN_KIND);
    assert_eq!(document.summary.checked, 2);
    assert_eq!(document.summary.same, 1);
    assert_eq!(document.summary.create, 1);
    assert_eq!(document.actions.len(), 2);
    assert!(document
        .actions
        .iter()
        .any(|action| action.identity == "bob"));

    let text = render_plan_text(&document, &args);
    assert!(text.contains("access plan:"));
    assert!(text.contains("would-create"));
    assert!(!text.contains("\nSAME "));

    let table = render_plan_table(&document, &args);
    assert!(table.contains("IDENTITY"));
    assert!(table.contains("bob"));

    let json = render_plan_json(&document).unwrap();
    assert!(json.contains("\"kind\": \"grafana-util-access-plan\""));
}

#[test]
fn all_plan_aggregates_present_bundles_and_reports_missing_resources() {
    let temp_dir = tempdir().unwrap();
    let users_dir = temp_dir.path().join("access-users");
    let service_accounts_dir = temp_dir.path().join("access-service-accounts");
    fs::create_dir_all(&users_dir).unwrap();
    fs::create_dir_all(&service_accounts_dir).unwrap();
    write_user_bundle(&users_dir);
    write_service_account_bundle(&service_accounts_dir);
    let args = AccessPlanArgs {
        common: make_common(),
        input_dir: temp_dir.path().to_path_buf(),
        resource: AccessPlanResource::All,
        prune: false,
        output_columns: Vec::new(),
        list_columns: false,
        no_header: false,
        show_same: false,
        output_format: PlanOutputFormat::Text,
    };
    let document = build_access_plan_document(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/org/users") => Ok(Some(json!([
                {"userId": "1", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Editor"}
            ]))),
            (Method::GET, "/api/serviceaccounts/search") => Ok(Some(json!({
                "serviceAccounts": []
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();

    assert_eq!(document.summary.resource_count, 4);
    assert_eq!(document.resources.len(), 4);
    assert_eq!(document.summary.create, 2);
    assert_eq!(document.summary.same, 1);
    assert!(document
        .resources
        .iter()
        .any(|resource| resource.resource_kind == "org" && !resource.bundle_present));
    assert!(document
        .resources
        .iter()
        .any(|resource| resource.resource_kind == "team" && !resource.bundle_present));
    assert!(
        document
            .actions
            .iter()
            .any(|action| action.resource_kind == "service-account"
                && action.identity == "svc-create")
    );
    let text = render_plan_text(&document, &args);
    assert!(text.contains("access plan: resources=4"));
    assert!(text.contains("bundle=missing"));
}

#[test]
fn all_plan_errors_when_no_bundle_dirs_are_present() {
    let temp_dir = tempdir().unwrap();
    let args = AccessPlanArgs {
        common: make_common(),
        input_dir: temp_dir.path().to_path_buf(),
        resource: AccessPlanResource::All,
        prune: false,
        output_columns: Vec::new(),
        list_columns: false,
        no_header: false,
        show_same: false,
        output_format: PlanOutputFormat::Text,
    };
    let error =
        build_access_plan_document(|_method, _path, _params, _payload| unreachable!(), &args)
            .unwrap_err();

    assert!(error
        .to_string()
        .contains("access plan --resource all did not find any access bundle directories"));
}

#[test]
fn org_plan_builds_summary_and_renderers() {
    let temp_dir = tempdir().unwrap();
    write_org_bundle(
        temp_dir.path(),
        json!([
            {
                "name": "Main Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "orgRole": "Editor"}
                ]
            },
            {
                "name": "New Org",
                "users": [
                    {"login": "bob", "email": "bob@example.com", "orgRole": "Viewer"}
                ]
            },
            {
                "name": "Ops Org",
                "users": [
                    {"login": "carol", "email": "carol@example.com", "orgRole": "Editor"}
                ]
            }
        ]),
    );
    let args = AccessPlanArgs {
        common: make_common(),
        input_dir: temp_dir.path().to_path_buf(),
        resource: AccessPlanResource::Org,
        prune: false,
        output_columns: vec![
            "identity".to_string(),
            "action".to_string(),
            "status".to_string(),
        ],
        list_columns: false,
        no_header: false,
        show_same: false,
        output_format: PlanOutputFormat::Text,
    };
    let document = build_access_plan_document(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 1, "name": "Main Org"},
                {"id": 2, "name": "Ops Org"},
                {"id": 3, "name": "Extra Org"}
            ]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "role": "Editor"}
            ]))),
            (Method::GET, "/api/orgs/2/users") => Ok(Some(json!([
                {"userId": 8, "login": "carol", "email": "carol@example.com", "role": "Viewer"}
            ]))),
            (Method::GET, "/api/orgs/3/users") => Ok(Some(json!([]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();

    assert_eq!(document.kind, ACCESS_PLAN_KIND);
    assert_eq!(document.summary.checked, 4);
    assert_eq!(document.summary.same, 1);
    assert_eq!(document.summary.create, 1);
    assert_eq!(document.summary.update, 1);
    assert_eq!(document.summary.extra_remote, 1);
    assert_eq!(document.summary.warning, 2);
    assert_eq!(document.actions.len(), 4);
    assert!(document
        .actions
        .iter()
        .any(|action| action.identity == "New Org" && action.action == "would-create"));
    assert!(document
        .actions
        .iter()
        .any(|action| action.identity == "Ops Org" && action.action == "would-update"));
    assert!(document
        .actions
        .iter()
        .any(|action| action.identity == "Main Org" && action.action == "same"));
    assert!(document
        .actions
        .iter()
        .any(|action| action.identity == "Extra Org" && action.action == "extra-remote"));

    let text = render_plan_text(&document, &args);
    assert!(text.contains("access plan:"));
    assert!(text.contains("would-create"));
    assert!(text.contains("would-update"));
    assert!(!text.contains("\nSAME "));
}

#[test]
fn org_plan_prune_marks_remote_orgs_for_delete() {
    let temp_dir = tempdir().unwrap();
    write_org_bundle(
        temp_dir.path(),
        json!([
            {
                "name": "Main Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "orgRole": "Editor"}
                ]
            }
        ]),
    );
    let args = AccessPlanArgs {
        common: make_common(),
        input_dir: temp_dir.path().to_path_buf(),
        resource: AccessPlanResource::Org,
        prune: true,
        output_columns: vec![
            "identity".to_string(),
            "action".to_string(),
            "status".to_string(),
        ],
        list_columns: false,
        no_header: false,
        show_same: false,
        output_format: PlanOutputFormat::Text,
    };
    let document = build_access_plan_document(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 1, "name": "Main Org"},
                {"id": 2, "name": "Extra Org"}
            ]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "role": "Editor"}
            ]))),
            (Method::GET, "/api/orgs/2/users") => Ok(Some(json!([]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();

    assert_eq!(document.summary.checked, 2);
    assert_eq!(document.summary.same, 1);
    assert_eq!(document.summary.extra_remote, 1);
    assert_eq!(document.summary.delete, 1);
    assert!(document
        .actions
        .iter()
        .any(|action| action.identity == "Extra Org" && action.action == "would-delete"));
    let text = render_plan_text(&document, &args);
    assert!(text.contains("would-delete"));
}
