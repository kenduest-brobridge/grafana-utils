//! Access domain test suite.
//! Validates CLI parsing/help text surfaces and handler contract behavior with stubbed
//! request closures.
use super::{
    cli_defs::AccessCliRoot,
    cli_defs::CommonCliArgsNoOrgId,
    org::{
        delete_org_with_request, diff_orgs_with_request, export_orgs_with_request,
        import_orgs_with_request, list_orgs_with_request, modify_org_with_request, org_csv_headers,
        org_summary_line, org_table_headers, org_table_rows,
    },
    parse_cli_from,
    pending_delete::{
        delete_service_account_token_with_request, delete_service_account_with_request,
        delete_team_with_request,
    },
    render::{user_summary_line, user_table_rows},
    run_access_cli_with_request,
    service_account::{
        add_service_account_token_with_request, add_service_account_with_request,
        diff_service_accounts_with_request, export_service_accounts_with_request,
        import_service_accounts_with_request, list_service_accounts_command_with_request,
    },
    team::{
        add_team_with_request, build_team_import_dry_run_document, diff_teams_with_request,
        export_teams_with_request, import_teams_with_request, list_teams_command_with_request,
        modify_team_with_request,
    },
    user::{
        add_user_with_request, annotate_user_account_scope, build_user_import_dry_run_document,
        delete_user_with_request, diff_users_with_request, export_users_with_request,
        import_users_with_request, list_users_with_request, modify_user_with_request,
    },
    AccessCommand, CommonCliArgs, DryRunOutputFormat, OrgCommand, OrgDeleteArgs, OrgDiffArgs,
    OrgExportArgs, OrgImportArgs, OrgListArgs, OrgModifyArgs, Scope, ServiceAccountAddArgs,
    ServiceAccountCommand, ServiceAccountDeleteArgs, ServiceAccountDiffArgs,
    ServiceAccountExportArgs, ServiceAccountImportArgs, ServiceAccountListArgs,
    ServiceAccountTokenAddArgs, ServiceAccountTokenCommand, ServiceAccountTokenDeleteArgs,
    TeamAddArgs, TeamCommand, TeamDeleteArgs, TeamDiffArgs, TeamExportArgs, TeamImportArgs,
    TeamListArgs, TeamModifyArgs, UserAddArgs, UserCommand, UserDeleteArgs, UserDiffArgs,
    UserExportArgs, UserImportArgs, UserListArgs, UserModifyArgs,
};
use crate::common::TOOL_VERSION;
use clap::{CommandFactory, Parser};
use reqwest::Method;
use serde_json::{json, Map, Value};
use std::fs;
use tempfile::tempdir;

fn render_access_subcommand_help(path: &[&str]) -> String {
    let mut command = AccessCliRoot::command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing access subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn access_delete_help_mentions_prompt() {
    assert!(render_access_subcommand_help(&["user", "delete"]).contains("--prompt"));
    assert!(render_access_subcommand_help(&["team", "delete"]).contains("--prompt"));
    assert!(render_access_subcommand_help(&["org", "delete"]).contains("--prompt"));
    assert!(render_access_subcommand_help(&["service-account", "delete"]).contains("--prompt"));
    assert!(
        render_access_subcommand_help(&["service-account", "token", "delete"]).contains("--prompt")
    );
}

#[test]
fn parse_cli_supports_access_delete_prompt_flags() {
    let user_args = parse_cli_from(["grafana-util access", "user", "delete", "--prompt"]);
    match user_args.command {
        AccessCommand::User {
            command: UserCommand::Delete(inner),
        } => {
            assert!(inner.prompt);
            assert_eq!(inner.scope, None);
        }
        _ => panic!("expected access user delete"),
    }

    let org_args = parse_cli_from(["grafana-util access", "org", "delete", "--prompt"]);
    match org_args.command {
        AccessCommand::Org {
            command: OrgCommand::Delete(inner),
        } => assert!(inner.prompt),
        _ => panic!("expected access org delete"),
    }
}

fn make_token_common() -> CommonCliArgs {
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

fn make_basic_common() -> CommonCliArgs {
    CommonCliArgs {
        profile: None,
        url: "http://127.0.0.1:3000".to_string(),
        api_token: None,
        username: Some("admin".to_string()),
        password: Some("secret".to_string()),
        prompt_password: false,
        prompt_token: false,
        org_id: None,
        timeout: 30,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
    }
}

fn make_basic_common_no_org_id() -> CommonCliArgsNoOrgId {
    CommonCliArgsNoOrgId {
        profile: None,
        url: "http://127.0.0.1:3000".to_string(),
        api_token: None,
        username: Some("admin".to_string()),
        password: Some("secret".to_string()),
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
    }
}

fn read_json_file(path: &std::path::Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn load_access_bundle_contract_cases() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../fixtures/access_bundle_contract_cases.json"
    ))
    .unwrap()
    .get("cases")
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default()
}

#[test]
fn org_list_table_rows_include_user_summaries_only_when_requested() {
    let rows = vec![Map::from_iter(vec![
        ("id".to_string(), json!("1")),
        ("name".to_string(), json!("Main Org.")),
        ("userCount".to_string(), json!("2")),
        (
            "users".to_string(),
            json!([
                {"userId": "7", "login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Admin"},
                {"userId": "8", "login": "bob", "email": "bob@example.com", "name": "Bob", "orgRole": "Viewer"}
            ]),
        ),
    ])];

    assert_eq!(org_table_headers(false), vec!["ID", "NAME", "USER_COUNT"]);
    assert_eq!(org_csv_headers(false), vec!["id", "name", "userCount"]);
    assert_eq!(
        org_table_rows(&rows, false),
        vec![vec![
            "1".to_string(),
            "Main Org.".to_string(),
            "2".to_string()
        ]]
    );

    assert_eq!(
        org_table_headers(true),
        vec!["ID", "NAME", "USER_COUNT", "USERS"]
    );
    assert_eq!(
        org_csv_headers(true),
        vec!["id", "name", "userCount", "users"]
    );
    assert_eq!(
        org_table_rows(&rows, true),
        vec![vec![
            "1".to_string(),
            "Main Org.".to_string(),
            "2".to_string(),
            "alice(Admin); bob(Viewer)".to_string()
        ]]
    );
}

#[test]
fn org_summary_line_includes_users_when_requested() {
    let row = Map::from_iter(vec![
        ("id".to_string(), json!("4")),
        ("name".to_string(), json!("Audit Org")),
        ("userCount".to_string(), json!("1")),
        (
            "users".to_string(),
            json!([{"userId": "9", "email": "audit@example.com", "orgRole": "Editor"}]),
        ),
    ]);

    assert_eq!(
        org_summary_line(&row, false),
        "id=4 name=Audit Org userCount=1"
    );
    assert_eq!(
        org_summary_line(&row, true),
        "id=4 name=Audit Org userCount=1 users=audit@example.com(Editor)"
    );
}

#[path = "access_cli_rust_tests.rs"]
mod access_cli_rust_tests;

#[path = "access_runtime_org_rust_tests.rs"]
mod access_runtime_org_rust_tests;

#[path = "access_service_account_org_rust_tests.rs"]
mod access_service_account_org_rust_tests;

#[test]
fn user_import_with_request_org_scope_requires_yes_for_team_removal() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor", "teams": ["ops"]}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Org,
        replace_existing: true,
        dry_run: false,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
    };
    let err = import_users_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/org/users") => Ok(Some(json!([
                {"userId": "7", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
            ]))),
            (Method::GET, "/api/users/7/teams") => Ok(Some(json!([
                {"id": 12, "name": "legacy"}
            ]))),
            (Method::GET, "/api/teams/search") => Ok(Some(json!({
                "teams": [{"id": 11, "name": "ops"}]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap_err();

    assert!(err.to_string().contains("would remove team memberships"));
}

#[test]
fn user_import_with_request_dry_run_reports_org_role_and_team_drift() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor", "teams": ["ops"]}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Org,
        replace_existing: true,
        dry_run: true,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: true,
    };
    let mut calls = Vec::new();
    let result = import_users_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match (method, path) {
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": "7", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
                ]))),
                (Method::GET, "/api/users/7/teams") => Ok(Some(json!([
                    {"id": 12, "name": "legacy"}
                ]))),
                (Method::GET, "/api/teams/search") => Ok(Some(json!({
                    "teams": [{"id": 11, "name": "ops"}]
                }))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 1);
    assert!(calls
        .iter()
        .all(|(method, path, _, _)| !(method == "PATCH" && path == "/api/org/users/7")));
    assert!(calls
        .iter()
        .all(|(method, path, _, _)| !(method == "POST" && path == "/api/teams/11/members")));
    assert!(calls
        .iter()
        .all(|(method, path, _, _)| !(method == "DELETE" && path == "/api/teams/12/members/7")));
}

#[test]
fn user_import_with_request_dry_run_json_reports_org_summary_and_rows() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor", "teams": ["ops"]}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Org,
        replace_existing: true,
        dry_run: true,
        table: false,
        json: true,
        output_format: DryRunOutputFormat::Json,
        yes: true,
    };

    let result = import_users_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/org/users") => Ok(Some(json!([
                {"userId": "7", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
            ]))),
            (Method::GET, "/api/users/7/teams") => Ok(Some(json!([
                {"id": 12, "name": "legacy"}
            ]))),
            (Method::GET, "/api/teams/search") => Ok(Some(json!({
                "teams": [{"id": 11, "name": "ops"}]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 0);
}

#[test]
fn user_import_with_request_updates_existing_org_user_role_and_team_membership() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor", "teams": ["ops"]}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Org,
        replace_existing: true,
        dry_run: false,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: true,
    };
    let mut calls = Vec::new();
    let result = import_users_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match (method, path) {
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": "7", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
                ]))),
                (Method::GET, "/api/users/7/teams") => Ok(Some(json!([
                    {"id": 12, "name": "legacy"}
                ]))),
                (Method::GET, "/api/teams/search") => Ok(Some(json!({
                    "teams": [{"id": 11, "name": "ops"}]
                }))),
                (Method::PATCH, "/api/org/users/7") => Ok(Some(json!({"message": "ok"}))),
                (Method::POST, "/api/teams/11/members") => Ok(Some(json!({"message": "ok"}))),
                (Method::DELETE, "/api/teams/12/members/7") => Ok(Some(json!({"message": "ok"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 1);
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PATCH" && path == "/api/org/users/7"));
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "POST" && path == "/api/teams/11/members"));
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "DELETE" && path == "/api/teams/12/members/7"));
}

#[test]
fn diff_teams_with_request_returns_expected_difference_count() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-teams");
    fs::create_dir_all(&diff_dir).unwrap();
    fs::write(
        diff_dir.join("teams.json"),
        r#"
[
  {"name":"Ops","email":"ops@example.com"},
  {"name":"Dev","email":"dev@example.com"}
]
"#,
    )
    .unwrap();
    let args = TeamDiffArgs {
        common: make_token_common(),
        diff_dir: diff_dir.clone(),
    };
    let result = diff_teams_with_request(
        |_method, path, _params, _payload| match path {
            "/api/teams/search" => Ok(Some(json!({
                "teams": [
                    {"id":"3","name":"Ops","email":"ops-two@example.com"},
                    {"id":"5","name":"SRE","email":"sre@example.com"}
                ]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 3);
}

#[test]
fn team_export_with_request_writes_bundle_with_members_and_admins() {
    let temp_dir = tempdir().unwrap();
    let args = TeamExportArgs {
        common: make_token_common(),
        output_dir: temp_dir.path().to_path_buf(),
        overwrite: true,
        dry_run: false,
        with_members: true,
    };
    let result = export_teams_with_request(
        |_method, path, _params, _payload| match path {
            "/api/teams/search" => Ok(Some(json!({
                "teams": [
                    {"id": 3, "name": "Ops", "email": "ops@example.com", "memberCount": 2}
                ]
            }))),
            "/api/teams/3/members" => Ok(Some(json!([
                {"userId": 7, "login": "alice@example.com", "email": "alice@example.com", "isAdmin": false},
                {"userId": 8, "login": "bob@example.com", "email": "bob@example.com", "isAdmin": true}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert!(result.is_ok());
    let bundle: Value =
        serde_json::from_str(&fs::read_to_string(temp_dir.path().join("teams.json")).unwrap())
            .unwrap();
    assert_eq!(
        bundle.get("kind"),
        Some(&json!("grafana-utils-access-team-export-index"))
    );
    let records = bundle
        .get("records")
        .and_then(Value::as_array)
        .expect("expected team export records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get("name"), Some(&json!("Ops")));
    assert_eq!(
        records[0].get("members"),
        Some(&json!(["alice@example.com", "bob@example.com"]))
    );
    assert_eq!(records[0].get("admins"), Some(&json!(["bob@example.com"])));
    let metadata = read_json_file(&temp_dir.path().join("export-metadata.json"));
    assert_eq!(
        metadata.get("kind"),
        Some(&json!("grafana-utils-access-team-export-index"))
    );
    assert_eq!(metadata.get("version"), Some(&json!(1)));
    assert_eq!(metadata.get("recordCount"), Some(&json!(1)));
    assert_eq!(
        metadata.get("sourceUrl"),
        Some(&json!("http://127.0.0.1:3000"))
    );
    assert_eq!(
        metadata.get("sourceDir"),
        Some(&json!(temp_dir.path().to_string_lossy().to_string()))
    );
    assert_eq!(metadata.get("metadataVersion"), Some(&json!(2)));
    assert_eq!(metadata.get("domain"), Some(&json!("access")));
    assert_eq!(metadata.get("resourceKind"), Some(&json!("teams")));
    assert_eq!(metadata.get("bundleKind"), Some(&json!("export-root")));
    assert_eq!(metadata["source"]["kind"], json!("live"));
    assert_eq!(metadata["capture"]["recordCount"], json!(1));
}

#[test]
fn team_import_rejects_kind_mismatch_and_future_version_bundle_contract() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("teams.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": []
        }))
        .unwrap(),
    )
    .unwrap();
    let args = TeamImportArgs {
        common: make_token_common(),
        input_dir: temp_dir.path().to_path_buf(),
        replace_existing: true,
        dry_run: true,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
    };
    let error =
        import_teams_with_request(|_method, _path, _params, _payload| Ok(None), &args).unwrap_err();
    assert!(error.to_string().contains("Access import kind mismatch"));

    fs::write(
        temp_dir.path().join("teams.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-team-export-index",
            "version": 99,
            "records": []
        }))
        .unwrap(),
    )
    .unwrap();
    let error =
        import_teams_with_request(|_method, _path, _params, _payload| Ok(None), &args).unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported access import version"));
}

#[test]
fn team_diff_with_request_reports_same_state_for_members_and_admins() {
    let temp_dir = tempdir().unwrap();
    let export_args = TeamExportArgs {
        common: make_token_common(),
        output_dir: temp_dir.path().to_path_buf(),
        overwrite: true,
        dry_run: false,
        with_members: false,
    };
    export_teams_with_request(
        |_method, path, _params, _payload| match path {
            "/api/teams/search" => Ok(Some(json!({
                "teams": [
                    {"id": 3, "name": "Ops", "email": "ops@example.com", "memberCount": 2}
                ]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &export_args,
    )
    .unwrap();
    let args = TeamDiffArgs {
        common: make_token_common(),
        diff_dir: temp_dir.path().to_path_buf(),
    };
    let result = diff_teams_with_request(
        |_method, path, _params, _payload| match path {
            "/api/teams/search" => Ok(Some(json!({
                "teams": [
                    {"id": 3, "name": "Ops", "email": "ops@example.com"}
                ]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert_eq!(result.unwrap(), 0);
}

#[test]
fn team_diff_with_request_reports_membership_drift() {
    let temp_dir = tempdir().unwrap();
    let export_args = TeamExportArgs {
        common: make_token_common(),
        output_dir: temp_dir.path().to_path_buf(),
        overwrite: true,
        dry_run: false,
        with_members: true,
    };
    export_teams_with_request(
        |_method, path, _params, _payload| match path {
            "/api/teams/search" => Ok(Some(json!({
                "teams": [
                    {"id": 3, "name": "Ops", "email": "ops@example.com", "memberCount": 2}
                ]
            }))),
            "/api/teams/3/members" => Ok(Some(json!([
                {"userId": 7, "login": "alice@example.com", "email": "alice@example.com", "isAdmin": false},
                {"userId": 8, "login": "bob@example.com", "email": "bob@example.com", "isAdmin": true}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &export_args,
    )
    .unwrap();
    let mut bundle: Value =
        serde_json::from_str(&fs::read_to_string(temp_dir.path().join("teams.json")).unwrap())
            .unwrap();
    if let Some(records) = bundle.get_mut("records").and_then(Value::as_array_mut) {
        if let Some(team) = records.get_mut(0).and_then(Value::as_object_mut) {
            team.insert(
                "members".to_string(),
                Value::Array(vec![Value::String("alice@example.com".to_string())]),
            );
            team.insert("admins".to_string(), Value::Array(Vec::new()));
        }
    }
    fs::write(
        temp_dir.path().join("teams.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = TeamDiffArgs {
        common: make_token_common(),
        diff_dir: temp_dir.path().to_path_buf(),
    };
    let result = diff_teams_with_request(
        |_method, path, _params, _payload| match path {
            "/api/teams/search" => Ok(Some(json!({
                "teams": [
                    {"id": 3, "name": "Ops", "email": "ops@example.com"}
                ]
            }))),
            "/api/teams/3/members" => Ok(Some(json!([
                {"userId": 7, "login": "alice@example.com", "email": "alice@example.com", "isAdmin": false},
                {"userId": 8, "login": "bob@example.com", "email": "bob@example.com", "isAdmin": true}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert_eq!(result.unwrap(), 1);
}

#[test]
fn team_import_dry_run_document_reports_summary_and_rows() {
    let rows = vec![
        serde_json::from_value::<serde_json::Map<String, Value>>(json!({
            "index": "1",
            "identity": "Ops",
            "action": "remove-member",
            "detail": "would remove team member bob@example.com"
        }))
        .unwrap(),
        serde_json::from_value::<serde_json::Map<String, Value>>(json!({
            "index": "1",
            "identity": "Ops",
            "action": "updated",
            "detail": "would update team"
        }))
        .unwrap(),
    ];
    let document = build_team_import_dry_run_document(
        &rows,
        1,
        0,
        1,
        0,
        std::path::Path::new("/tmp/access-teams"),
    );

    assert_eq!(
        document.get("kind"),
        Some(&json!("grafana-utils-access-import-dry-run"))
    );
    assert_eq!(document.get("resourceKind"), Some(&json!("team")));
    assert_eq!(document.get("schemaVersion"), Some(&json!(1)));
    assert_eq!(document.get("toolVersion"), Some(&json!(TOOL_VERSION)));
    assert_eq!(document.get("reviewRequired"), Some(&json!(true)));
    assert_eq!(document.get("reviewed"), Some(&json!(false)));
    assert_eq!(
        document
            .get("summary")
            .and_then(|summary| summary.get("processed")),
        Some(&json!(1))
    );
    assert_eq!(
        document
            .get("summary")
            .and_then(|summary| summary.get("created")),
        Some(&json!(0))
    );
    assert_eq!(
        document
            .get("summary")
            .and_then(|summary| summary.get("updated")),
        Some(&json!(1))
    );
    assert_eq!(
        document
            .get("summary")
            .and_then(|summary| summary.get("skipped")),
        Some(&json!(0))
    );
    assert_eq!(
        document
            .get("summary")
            .and_then(|summary| summary.get("source")),
        Some(&json!("/tmp/access-teams"))
    );
    assert_eq!(
        document.get("rows").and_then(Value::as_array).map(Vec::len),
        Some(2)
    );
}

#[test]
fn user_import_dry_run_document_reports_summary_and_rows() {
    let rows = vec![
        serde_json::from_value::<serde_json::Map<String, Value>>(json!({
            "index": "1",
            "identity": "alice",
            "action": "update-org-role",
            "detail": "would update orgRole -> Editor"
        }))
        .unwrap(),
        serde_json::from_value::<serde_json::Map<String, Value>>(json!({
            "index": "1",
            "identity": "alice",
            "action": "updated",
            "detail": "would update user"
        }))
        .unwrap(),
    ];
    let document = build_user_import_dry_run_document(
        &rows,
        1,
        0,
        1,
        0,
        std::path::Path::new("/tmp/access-users"),
    );

    assert_eq!(
        document.get("kind"),
        Some(&json!("grafana-utils-access-import-dry-run"))
    );
    assert_eq!(document.get("resourceKind"), Some(&json!("user")));
    assert_eq!(document.get("schemaVersion"), Some(&json!(1)));
    assert_eq!(document.get("toolVersion"), Some(&json!(TOOL_VERSION)));
    assert_eq!(document.get("reviewRequired"), Some(&json!(true)));
    assert_eq!(document.get("reviewed"), Some(&json!(false)));
    assert_eq!(
        document
            .get("summary")
            .and_then(|summary| summary.get("processed")),
        Some(&json!(1))
    );
    assert_eq!(
        document
            .get("summary")
            .and_then(|summary| summary.get("updated")),
        Some(&json!(1))
    );
    assert_eq!(
        document
            .get("summary")
            .and_then(|summary| summary.get("source")),
        Some(&json!("/tmp/access-users"))
    );
    assert_eq!(
        document.get("rows").and_then(Value::as_array).map(Vec::len),
        Some(2)
    );
}

#[test]
fn team_import_with_request_creates_team_and_memberships() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-teams");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("teams.json"),
        r#"[{"name":"Ops","email":"ops@example.com","members":["alice@example.com"],"admins":["bob@example.com"]}]"#,
    )
    .unwrap();
    let args = TeamImportArgs {
        common: make_token_common(),
        input_dir: input_dir.clone(),
        replace_existing: false,
        dry_run: false,
        yes: false,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
    };
    let mut calls = Vec::new();
    let result = import_teams_with_request(
        |method, path, _params, payload| {
            calls.push((method.to_string(), path.to_string(), payload.cloned()));
            match (method, path) {
                (Method::GET, "/api/teams/search") => Ok(Some(json!({"teams": []}))),
                (Method::POST, "/api/teams") => {
                    let payload = payload.expect("teams payload expected");
                    assert_eq!(payload.get("name"), Some(&json!("Ops")));
                    assert_eq!(payload.get("email"), Some(&json!("ops@example.com")));
                    Ok(Some(json!({"teamId": "3"})))
                }
                (Method::POST, "/api/teams/3/members") => Ok(Some(json!({"message": "ok"}))),
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": 7, "login": "alice@example.com", "email": "alice@example.com"},
                    {"userId": 8, "login": "bob@example.com", "email": "bob@example.com"},
                ]))),
                (Method::PUT, "/api/teams/3/members") => {
                    let payload = payload.expect("team members payload expected");
                    assert_eq!(payload.get("members"), Some(&json!(["alice@example.com"])));
                    assert_eq!(payload.get("admins"), Some(&json!(["bob@example.com"])));
                    Ok(Some(json!({"message": "ok"})))
                }
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 1);
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "POST" && path == "/api/teams"),);
    assert!(
        calls
            .iter()
            .filter(|(method, path, _)| method == "POST" && path == "/api/teams/3/members")
            .count()
            >= 2
    );
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "PUT" && path == "/api/teams/3/members"),);
}

#[test]
fn team_import_with_request_rejects_member_removals_without_yes() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-teams");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("teams.json"),
        r#"[{"name":"Ops","members":["alice@example.com"]}]"#,
    )
    .unwrap();
    let args = TeamImportArgs {
        common: make_token_common(),
        input_dir: input_dir.clone(),
        replace_existing: true,
        dry_run: false,
        yes: false,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
    };
    let result = import_teams_with_request(
        |_method, path, _params, _payload| match path {
            "/api/teams/search" => Ok(Some(json!({"teams": [{"id": "3", "name": "Ops"}]}))),
            "/api/teams/3/members" => Ok(Some(json!([
                {"userId": 7, "login": "alice@example.com"},
                {"userId": 9, "login": "carol@example.com"},
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error
        .to_string()
        .contains("Team import would remove team memberships for Ops"));
}

#[test]
fn team_import_with_request_updates_memberships_when_yes_is_set() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-teams");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("teams.json"),
        r#"[{"name":"Ops","members":["alice@example.com","bob@example.com"]}]"#,
    )
    .unwrap();
    let args = TeamImportArgs {
        common: make_token_common(),
        input_dir: input_dir.clone(),
        replace_existing: true,
        dry_run: false,
        yes: true,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
    };
    let mut calls = Vec::new();
    let result = import_teams_with_request(
        |method, path, _params, payload| {
            calls.push((method.to_string(), path.to_string(), payload.cloned()));
            match (method, path) {
                (Method::GET, "/api/teams/search") => {
                    Ok(Some(json!({"teams": [{"id": "3", "name": "Ops"}]})))
                }
                (Method::GET, "/api/teams/3/members") => Ok(Some(json!([
                    {"userId": 7, "login": "alice@example.com"},
                    {"userId": 9, "login": "carol@example.com"},
                ]))),
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": 7, "login": "alice@example.com", "email": "alice@example.com"},
                    {"userId": 8, "login": "bob@example.com", "email": "bob@example.com"},
                ]))),
                (Method::POST, "/api/teams/3/members") => Ok(Some(json!({"message": "ok"}))),
                (Method::DELETE, "/api/teams/3/members/9") => Ok(Some(json!({"message": "ok"}))),
                (Method::PUT, "/api/teams/3/members") => Ok(Some(json!({"message": "ok"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 1);
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "POST" && path == "/api/teams/3/members"));
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "DELETE" && path == "/api/teams/3/members/9"));
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "PUT" && path == "/api/teams/3/members"));
}

#[test]
fn user_add_with_request_requires_basic_auth_and_updates_role() {
    let args = UserAddArgs {
        common: make_basic_common(),
        login: "alice".to_string(),
        email: "alice@example.com".to_string(),
        name: "Alice".to_string(),
        new_user_password: Some("pw".to_string()),
        new_user_password_file: None,
        prompt_user_password: false,
        org_role: Some("Editor".to_string()),
        grafana_admin: Some(true),
        json: true,
    };
    let mut calls = Vec::new();
    let result = add_user_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/admin/users" => Ok(Some(json!({"id": 9}))),
                "/api/org/users/9" => Ok(Some(json!({"message": "ok"}))),
                "/api/admin/users/9/permissions" => Ok(Some(json!({"message": "ok"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/admin/users"));
    assert!(calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/org/users/9"));
    assert!(calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/admin/users/9/permissions"));
}

#[test]
fn user_add_with_request_reads_password_file() {
    let temp_dir = tempdir().unwrap();
    let password_path = temp_dir.path().join("user-password.txt");
    fs::write(&password_path, "pw-from-file\n").unwrap();
    let args = UserAddArgs {
        common: make_basic_common(),
        login: "alice".to_string(),
        email: "alice@example.com".to_string(),
        name: "Alice".to_string(),
        new_user_password: None,
        new_user_password_file: Some(password_path.clone()),
        prompt_user_password: false,
        org_role: None,
        grafana_admin: None,
        json: false,
    };
    let mut captured_password = None;
    let result = add_user_with_request(
        |method, path, _params, payload| {
            if method == Method::POST && path == "/api/admin/users" {
                captured_password = payload
                    .and_then(|value| value.get("password"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                return Ok(Some(json!({"id": 9})));
            }
            panic!("unexpected request");
        },
        &args,
    );

    assert!(result.is_ok());
    assert_eq!(captured_password.as_deref(), Some("pw-from-file"));
}

#[test]
fn user_modify_with_request_updates_profile_and_password() {
    let args = UserModifyArgs {
        common: make_basic_common(),
        user_id: Some("9".to_string()),
        login: None,
        email: None,
        set_login: Some("alice2".to_string()),
        set_email: None,
        set_name: Some("Alice Two".to_string()),
        set_password: Some("newpw".to_string()),
        set_password_file: None,
        prompt_set_password: false,
        set_org_role: None,
        set_grafana_admin: None,
        json: true,
    };
    let mut calls = Vec::new();
    let result = modify_user_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/users/9" if method == Method::GET => Ok(Some(
                    json!({"id": 9, "login": "alice", "email": "alice@example.com", "name": "Alice"}),
                )),
                "/api/users/9" if method == Method::PUT => Ok(Some(json!({"message": "ok"}))),
                "/api/admin/users/9/password" => Ok(Some(json!({"message": "ok"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PUT" && path == "/api/users/9"));
    assert!(calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/admin/users/9/password"));
}

#[test]
fn user_modify_with_request_reads_set_password_file() {
    let temp_dir = tempdir().unwrap();
    let password_path = temp_dir.path().join("replacement-password.txt");
    fs::write(&password_path, "newpw-from-file\n").unwrap();
    let args = UserModifyArgs {
        common: make_basic_common(),
        user_id: Some("9".to_string()),
        login: None,
        email: None,
        set_login: None,
        set_email: None,
        set_name: None,
        set_password: None,
        set_password_file: Some(password_path.clone()),
        prompt_set_password: false,
        set_org_role: None,
        set_grafana_admin: None,
        json: false,
    };
    let mut captured_password = None;
    let result = modify_user_with_request(
        |method, path, _params, payload| match path {
            "/api/users/9" if method == Method::GET => Ok(Some(
                json!({"id": 9, "login": "alice", "email": "alice@example.com", "name": "Alice"}),
            )),
            "/api/admin/users/9/password" if method == Method::PUT => {
                captured_password = payload
                    .and_then(|value| value.get("password"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                Ok(Some(json!({"message": "ok"})))
            }
            _ => panic!("unexpected request"),
        },
        &args,
    );

    assert!(result.is_ok());
    assert_eq!(captured_password.as_deref(), Some("newpw-from-file"));
}

#[test]
fn user_delete_with_request_requires_yes_and_deletes() {
    let args = UserDeleteArgs {
        common: make_basic_common(),
        user_id: Some("9".to_string()),
        login: None,
        email: None,
        scope: Some(Scope::Global),
        prompt: false,
        yes: true,
        json: true,
    };
    let mut calls = Vec::new();
    let result = delete_user_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match path {
                "/api/users/9" if method == Method::GET => {
                    Ok(Some(json!({"id": 9, "login": "alice"})))
                }
                "/api/admin/users/9" if method == Method::DELETE => {
                    Ok(Some(json!({"message": "deleted"})))
                }
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "DELETE" && path == "/api/admin/users/9"));
}

#[test]
fn team_list_with_request_reads_search_and_members() {
    let args = TeamListArgs {
        common: make_token_common(),
        query: Some("ops".to_string()),
        name: None,
        with_members: true,
        output_columns: Vec::new(),
        list_columns: false,
        page: 1,
        per_page: 100,
        input_dir: None,
        table: false,
        csv: false,
        json: false,
        yaml: true,
        output_format: None,
    };
    let mut calls = Vec::new();
    let result = list_teams_command_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match path {
                "/api/teams/search" => Ok(Some(
                    json!({"teams": [{"id": 5, "name": "Ops", "memberCount": 1}]}),
                )),
                "/api/teams/5/members" => Ok(Some(json!([{"login": "alice"}]))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert_eq!(result.unwrap(), 1);
    assert!(calls.iter().any(|(_, path, _)| path == "/api/teams/search"));
    assert!(calls
        .iter()
        .any(|(_, path, _)| path == "/api/teams/5/members"));
}

#[test]
fn team_list_all_output_columns_are_accepted() {
    let args = TeamListArgs {
        common: make_token_common(),
        query: None,
        name: None,
        with_members: false,
        output_columns: vec!["all".to_string()],
        list_columns: false,
        page: 1,
        per_page: 100,
        input_dir: None,
        table: true,
        csv: false,
        json: false,
        yaml: false,
        output_format: None,
    };

    let result = list_teams_command_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/teams/search") => Ok(Some(
                json!({"teams": [{"id": 5, "name": "Ops", "memberCount": 1}]}),
            )),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert_eq!(result.unwrap(), 1);
}

#[test]
fn team_add_with_request_creates_team_and_members() {
    let args = TeamAddArgs {
        common: make_token_common(),
        name: "Ops".to_string(),
        email: Some("ops@example.com".to_string()),
        members: vec!["alice@example.com".to_string()],
        admins: vec!["bob@example.com".to_string()],
        json: true,
    };
    let mut calls = Vec::new();
    let result = add_team_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/teams" => Ok(Some(json!({"teamId": 3}))),
                "/api/teams/3" => Ok(Some(
                    json!({"id": 3, "name": "Ops", "email": "ops@example.com"}),
                )),
                "/api/teams/3/members" if method == Method::POST => {
                    Ok(Some(json!({"message": "ok"})))
                }
                "/api/teams/3/members" if method == Method::GET => Ok(Some(json!([
                    {"login": "alice@example.com", "email": "alice@example.com", "userId": 7, "isAdmin": false}
                ]))),
                "/api/org/users" => Ok(Some(json!([
                    {"userId": 7, "login": "alice@example.com", "email": "alice@example.com"},
                    {"userId": 8, "login": "bob@example.com", "email": "bob@example.com"}
                ]))),
                "/api/teams/3/members" if method == Method::PUT => {
                    Ok(Some(json!({"message": "ok"})))
                }
                _ => panic!("unexpected path {path} {method:?}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls.iter().any(|(_, path, _, _)| path == "/api/teams"));
    let member_post_payload = calls
        .iter()
        .find(|(method, path, _, _)| method == "POST" && path == "/api/teams/3/members")
        .and_then(|(_, _, _, payload)| payload.as_ref())
        .expect("expected add-member payload");
    assert_eq!(member_post_payload.get("userId"), Some(&json!(7)));
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PUT" && path == "/api/teams/3/members"));
}

#[test]
fn team_add_with_request_creates_empty_team_without_members() {
    let args = TeamAddArgs {
        common: make_token_common(),
        name: "Ops".to_string(),
        email: Some("ops@example.com".to_string()),
        members: Vec::new(),
        admins: Vec::new(),
        json: false,
    };
    let mut calls = Vec::new();
    let result = add_team_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/teams" => Ok(Some(json!({"teamId": 3}))),
                "/api/teams/3" => Ok(Some(
                    json!({"id": 3, "name": "Ops", "email": "ops@example.com"}),
                )),
                _ => panic!("unexpected path {path} {method:?}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls.iter().any(|(_, path, _, _)| path == "/api/teams"));
    assert!(!calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/teams/3/members"));
}

#[test]
fn team_modify_with_request_updates_members_and_admins() {
    let args = TeamModifyArgs {
        common: make_token_common(),
        team_id: Some("3".to_string()),
        name: None,
        add_member: vec!["alice@example.com".to_string()],
        remove_member: vec![],
        add_admin: vec!["bob@example.com".to_string()],
        remove_admin: vec![],
        json: true,
    };
    let mut calls = Vec::new();
    let result = modify_team_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/teams/3" => Ok(Some(json!({"id": 3, "name": "Ops"}))),
                "/api/org/users" => Ok(Some(json!([
                    {"userId": 7, "login": "alice@example.com", "email": "alice@example.com"},
                    {"userId": 8, "login": "bob@example.com", "email": "bob@example.com"}
                ]))),
                "/api/teams/3/members" if method == Method::POST => {
                    Ok(Some(json!({"message": "ok"})))
                }
                "/api/teams/3/members" if method == Method::GET => Ok(Some(json!([
                    {"login": "alice@example.com", "email": "alice@example.com", "userId": 7, "isAdmin": false}
                ]))),
                "/api/teams/3/members" if method == Method::PUT => {
                    Ok(Some(json!({"message": "ok"})))
                }
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PUT" && path == "/api/teams/3/members"));
}

#[test]
fn run_access_cli_with_request_routes_user_list() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "list",
        "--url",
        "https://grafana.example.com",
        "--json",
        "--token",
        "abc",
    ]);
    let result = run_access_cli_with_request(
        |_method, path, _params, _payload| match path {
            "/api/org/users" => Ok(Some(json!([]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );
    assert!(result.is_ok());
}
