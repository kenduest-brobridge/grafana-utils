use super::*;
use reqwest::Method;
use serde_json::json;

#[test]
fn user_list_with_request_reads_org_users() {
    let list_help = render_access_subcommand_help(&["user", "list"]);
    assert!(!list_help.contains("--username"));
    assert!(!list_help.contains("--basic-user USERNAME, --username USERNAME"));

    let args = UserListArgs {
        common: make_token_common(),
        scope: Scope::Org,
        all_orgs: false,
        query: None,
        login: None,
        email: None,
        org_role: None,
        grafana_admin: None,
        with_teams: false,
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
    let count = list_users_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match path {
                "/api/org/users" => Ok(Some(json!([
                    {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Admin"}
                ]))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(calls[0].0, Method::GET.to_string());
    assert_eq!(calls[0].1, "/api/org/users");
}

#[test]
fn annotate_user_account_scope_marks_org_rows_as_global_shared_identity() {
    let mut rows = vec![Map::from_iter(vec![
        ("id".to_string(), Value::String("7".to_string())),
        ("login".to_string(), Value::String("alice".to_string())),
        ("scope".to_string(), Value::String("org".to_string())),
    ])];

    annotate_user_account_scope(&mut rows);

    assert_eq!(
        rows[0].get("accountScope"),
        Some(&Value::String("global-shared".to_string()))
    );
}

#[test]
fn user_list_render_shows_account_scope_for_shared_global_identity() {
    let mut rows = vec![Map::from_iter(vec![
        ("id".to_string(), Value::String("7".to_string())),
        ("login".to_string(), Value::String("alice".to_string())),
        (
            "email".to_string(),
            Value::String("alice@example.com".to_string()),
        ),
        ("name".to_string(), Value::String("Alice".to_string())),
        ("orgRole".to_string(), Value::String("Admin".to_string())),
        (
            "grafanaAdmin".to_string(),
            Value::String("true".to_string()),
        ),
        ("scope".to_string(), Value::String("org".to_string())),
        ("teams".to_string(), Value::String(String::new())),
    ])];

    annotate_user_account_scope(&mut rows);

    assert_eq!(user_table_rows(&rows, &[])[0][7], "global-shared");
    assert!(user_summary_line(&rows[0], &[]).contains("accountScope=global-shared"));
}

#[test]
fn normalize_user_row_includes_origin_and_last_active_metadata() {
    let row = normalize_user_row(
        &serde_json::from_value(json!({
            "id": 7,
            "login": "alice",
            "email": "alice@example.com",
            "name": "Alice",
            "isGrafanaAdmin": true,
            "isExternal": true,
            "authLabels": ["oauth"],
            "lastSeenAt": "2026-04-09T08:12:00Z",
            "lastSeenAtAge": "2m"
        }))
        .unwrap(),
        &Scope::Global,
    );

    assert_eq!(
        row.get("origin"),
        Some(&json!({
            "kind": "external",
            "external": true,
            "provisioned": false,
            "labels": ["oauth"]
        }))
    );
    assert_eq!(
        row.get("lastActive"),
        Some(&json!({
            "at": "2026-04-09T08:12:00Z",
            "age": "2m"
        }))
    );
}

#[test]
fn normalize_user_row_marks_provisioned_origin_when_present() {
    let row = normalize_user_row(
        &serde_json::from_value(json!({
            "id": 9,
            "login": "bootstrap-admin",
            "provisioned": true
        }))
        .unwrap(),
        &Scope::Global,
    );

    assert_eq!(row["origin"]["kind"], json!("provisioned"));
    assert_eq!(row["origin"]["provisioned"], json!(true));
    assert_eq!(row["origin"]["external"], json!(false));
}

#[test]
fn normalize_user_row_preserves_existing_structured_origin_and_last_active() {
    let row = normalize_user_row(
        &serde_json::from_value(json!({
            "id": "7",
            "login": "alice",
            "origin": {
                "kind": "external",
                "external": true,
                "provisioned": false,
                "labels": ["oauth"]
            },
            "lastActive": {
                "at": "2026-04-09T08:12:00Z",
                "age": "2m"
            }
        }))
        .unwrap(),
        &Scope::Global,
    );

    assert_eq!(row["origin"]["kind"], json!("external"));
    assert_eq!(row["origin"]["labels"], json!(["oauth"]));
    assert_eq!(row["lastActive"]["at"], json!("2026-04-09T08:12:00Z"));
    assert_eq!(row["lastActive"]["age"], json!("2m"));
}

#[test]
fn user_table_headers_expand_all_output_columns() {
    let headers = user_table_headers(&["all".to_string()]);

    assert_eq!(
        headers,
        vec![
            "ID",
            "LOGIN",
            "EMAIL",
            "NAME",
            "ORG_ROLE",
            "GRAFANA_ADMIN",
            "SCOPE",
            "ACCOUNT_SCOPE",
            "ORIGIN",
            "LAST_ACTIVE",
            "TEAMS",
        ]
    );
}

#[test]
fn build_auth_context_enables_verification_for_ca_cert() {
    let mut common = make_token_common();
    common.ca_cert = Some("/tmp/grafana-ca.pem".into());

    let context = build_auth_context(&common).unwrap();

    assert!(context.verify_ssl);
    assert_eq!(
        context.ca_cert.as_deref(),
        Some(std::path::Path::new("/tmp/grafana-ca.pem"))
    );
}
