use super::*;
use crate::access::cli_defs::{AccessPlanArgs, AccessPlanResource, PlanOutputFormat};
use crate::access::{parse_cli_from, CommonCliArgs};
use serde_json::json;
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

fn write_team_bundle(dir: &Path, records: serde_json::Value) {
    fs::write(
        dir.join(ACCESS_TEAM_EXPORT_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": ACCESS_EXPORT_KIND_TEAMS,
            "version": 1,
            "records": records,
        }))
        .unwrap(),
    )
    .unwrap();
}

fn make_args(dir: &Path, prune: bool) -> AccessPlanArgs {
    AccessPlanArgs {
        common: make_common(),
        input_dir: dir.to_path_buf(),
        resource: AccessPlanResource::Team,
        prune,
        output_columns: Vec::new(),
        list_columns: false,
        no_header: false,
        show_same: false,
        output_format: PlanOutputFormat::Json,
    }
}

#[test]
fn parse_access_plan_supports_team_resource() {
    let args = parse_cli_from([
        "grafana-util",
        "plan",
        "--input-dir",
        "./access-teams",
        "--resource",
        "team",
    ]);
    match args.command {
        super::super::AccessCommand::Plan(plan) => {
            assert!(matches!(plan.resource, AccessPlanResource::Team));
        }
        _ => panic!("expected access plan"),
    }
}

#[test]
fn team_plan_builds_create_same_update_rows() {
    let temp_dir = tempdir().unwrap();
    write_team_bundle(
        temp_dir.path(),
        json!([
            {"name": "Ops", "email": "ops@example.com"},
            {"name": "Platform", "email": "platform@example.com"},
            {"name": "New", "email": "new@example.com"}
        ]),
    );
    fs::write(
        temp_dir.path().join(ACCESS_EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({"scope": "org"})).unwrap(),
    )
    .unwrap();
    let args = make_args(temp_dir.path(), false);
    let document = build_team_access_plan_document(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/teams/search") => Ok(Some(json!({
                "teams": [
                    {"id": "1", "name": "Ops", "email": "ops@example.com", "memberCount": 1},
                    {"id": "2", "name": "Platform", "email": "platform-old@example.com", "memberCount": 0},
                    {"id": "3", "name": "Remote", "email": "remote@example.com", "memberCount": 0}
                ]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
        "grafana-util-access-plan",
        1,
    )
    .unwrap();

    assert_eq!(document.kind, "grafana-util-access-plan");
    assert_eq!(document.summary.checked, 4);
    assert_eq!(document.summary.same, 1);
    assert_eq!(document.summary.create, 1);
    assert_eq!(document.summary.update, 1);
    assert_eq!(document.summary.extra_remote, 1);
    assert_eq!(document.summary.delete, 0);

    let same = document
        .actions
        .iter()
        .find(|action| action.identity == "Ops")
        .expect("same action");
    assert_eq!(same.action, "same");
    assert_eq!(same.status, "same");
    assert_eq!(same.resource_kind, "team");

    let update = document
        .actions
        .iter()
        .find(|action| action.identity == "Platform")
        .expect("update action");
    assert_eq!(update.action, "would-update");
    assert_eq!(update.status, "warning");
    assert!(update.changed_fields.iter().any(|field| field == "email"));
    assert!(update.review_hints.iter().any(|hint| hint.contains("team")));

    let create = document
        .actions
        .iter()
        .find(|action| action.identity == "New")
        .expect("create action");
    assert_eq!(create.action, "would-create");
    assert_eq!(create.status, "ready");
    assert!(create.target.as_ref().unwrap().get("email").is_some());
}

#[test]
fn team_plan_prune_marks_extra_remote_delete_candidate() {
    let temp_dir = tempdir().unwrap();
    write_team_bundle(
        temp_dir.path(),
        json!([{"name": "Ops", "email": "ops@example.com"}]),
    );
    let args = make_args(temp_dir.path(), true);
    let document = build_team_access_plan_document(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/teams/search") => Ok(Some(json!({
                "teams": [
                    {"id": "1", "name": "Ops", "email": "ops@example.com", "memberCount": 1},
                    {"id": "2", "name": "Remote", "email": "remote@example.com", "memberCount": 0}
                ]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
        "grafana-util-access-plan",
        1,
    )
    .unwrap();

    assert_eq!(document.summary.extra_remote, 1);
    assert_eq!(document.summary.delete, 1);
    let extra = document
        .actions
        .iter()
        .find(|action| action.identity == "Remote")
        .expect("delete action");
    assert_eq!(extra.action, "would-delete");
    assert_eq!(extra.status, "ready");
    assert!(extra.blocked_reason.is_none());
}

#[test]
fn team_plan_blocks_provisioned_membership_changes() {
    let temp_dir = tempdir().unwrap();
    write_team_bundle(
        temp_dir.path(),
        json!([{"name": "Ops", "email": "ops@example.com", "members": ["alice@example.com"]}]),
    );
    let args = make_args(temp_dir.path(), false);
    let document = build_team_access_plan_document(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/teams/search") => Ok(Some(json!({
                "teams": [
                    {"id": "1", "name": "Ops", "email": "ops@example.com", "isProvisioned": true, "memberCount": 0}
                ]
            }))),
            (Method::GET, "/api/teams/1/members") => Ok(Some(json!([]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
        "grafana-util-access-plan",
        1,
    )
    .unwrap();

    let action = document
        .actions
        .iter()
        .find(|action| action.identity == "Ops")
        .expect("blocked action");
    assert_eq!(action.action, "blocked");
    assert_eq!(action.status, "blocked");
    assert!(action
        .blocked_reason
        .as_deref()
        .unwrap()
        .contains("provisioned team memberships cannot be changed"));
}
