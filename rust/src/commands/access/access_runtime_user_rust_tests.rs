//! Access user runtime tests for CLI routing and bundle behavior.

use super::*;

#[test]
fn run_access_cli_with_request_routes_user_diff() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-users");
    fs::create_dir_all(&diff_dir).unwrap();
    fs::write(
        diff_dir.join("users.json"),
        r#"[
            {"login":"alice","email":"alice@example.com","name":"Alice","orgRole":"Admin","grafanaAdmin":true}
        ]"#,
    )
    .unwrap();

    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "diff",
        "--diff-dir",
        diff_dir.to_str().unwrap(),
        "--scope",
        "org",
    ]);
    let result = run_access_cli_with_request(
        |method, path, _params, _payload| {
            assert_eq!(method.to_string(), Method::GET.to_string());
            match path {
                "/api/org/users" => Ok(Some(json!([
                    {"userId":"11","login":"alice","email":"alice@example.com","name":"Alice","role":"Admin"}
                ]))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );
    assert!(result.is_ok());
}

#[test]
fn diff_users_with_request_returns_expected_difference_count() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-users");
    fs::create_dir_all(&diff_dir).unwrap();
    fs::write(
        diff_dir.join("users.json"),
        r#"
[
  {"login":"alice","email":"alice@example.com","name":"Alice","orgRole":"Admin","grafanaAdmin":true},
  {"login":"bob","email":"bob@example.com","name":"Bob","orgRole":"Viewer","grafanaAdmin":false},
  {"login":"carol","email":"carol@example.com","name":"Carol","orgRole":"Viewer","grafanaAdmin":false}
]
"#,
    )
    .unwrap();
    let args = UserDiffArgs {
        common: make_token_common(),
        diff_dir: diff_dir.clone(),
        scope: Scope::Org,
    };
    let result = diff_users_with_request(
        |method, path, _params, _payload| {
            assert_eq!(method.to_string(), Method::GET.to_string());
            match path {
                "/api/org/users" => Ok(Some(json!([
                    {
                        "userId": "11",
                        "login": "alice",
                        "email": "alice@example.com",
                        "name": "Alice",
                        "role": "Editor"
                    },
                    {
                        "userId": "12",
                        "login": "dave",
                        "email": "dave@example.com",
                        "name": "Dave",
                        "role": "Viewer"
                    }
                ]))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 4);
}

#[test]
fn user_export_with_request_writes_global_bundle() {
    let temp_dir = tempdir().unwrap();
    let args = UserExportArgs {
        common: make_basic_common(),
        output_dir: temp_dir.path().to_path_buf(),
        overwrite: true,
        dry_run: false,
        scope: Scope::Global,
        with_teams: false,
    };
    let result = export_users_with_request(
        |method, path, params, _payload| match (method, path) {
            (Method::GET, "/api/users") => {
                assert_eq!(params[0], ("page".to_string(), "1".to_string()));
                Ok(Some(json!([
                    {
                        "id": 7,
                        "login": "alice",
                        "email": "alice@example.com",
                        "name": "Alice",
                        "isGrafanaAdmin": false,
                        "isExternal": true,
                        "authLabels": ["oauth"],
                        "lastSeenAt": "2026-04-09T08:12:00Z",
                        "lastSeenAtAge": "2m"
                    }
                ])))
            }
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert!(result.is_ok());
    let bundle: Value =
        serde_json::from_str(&fs::read_to_string(temp_dir.path().join("users.json")).unwrap())
            .unwrap();
    assert_eq!(
        bundle.get("kind"),
        Some(&json!("grafana-utils-access-user-export-index"))
    );
    let records = bundle
        .get("records")
        .and_then(Value::as_array)
        .expect("expected user export records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get("login"), Some(&json!("alice")));
    assert_eq!(
        records[0].get("origin"),
        Some(&json!({
            "kind": "external",
            "external": true,
            "provisioned": false,
            "labels": ["oauth"]
        }))
    );
    assert_eq!(
        records[0].get("lastActive"),
        Some(&json!({
            "at": "2026-04-09T08:12:00Z",
            "age": "2m"
        }))
    );
    let metadata = read_json_file(&temp_dir.path().join("export-metadata.json"));
    assert_eq!(
        metadata.get("kind"),
        Some(&json!("grafana-utils-access-user-export-index"))
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
    assert_eq!(metadata.get("resourceKind"), Some(&json!("users")));
    assert_eq!(metadata.get("bundleKind"), Some(&json!("export-root")));
    assert_eq!(metadata["source"]["kind"], json!("live"));
    assert_eq!(metadata["capture"]["recordCount"], json!(1));
}

#[test]
fn user_import_rejects_kind_mismatch_and_future_version_bundle_contract() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-team-export-index",
            "version": 1,
            "records": []
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Global,
        replace_existing: true,
        dry_run: true,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
    };
    let error =
        import_users_with_request(|_method, _path, _params, _payload| Ok(None), &args).unwrap_err();
    assert!(error.to_string().contains("Access import kind mismatch"));

    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 99,
            "records": []
        }))
        .unwrap(),
    )
    .unwrap();
    let error =
        import_users_with_request(|_method, _path, _params, _payload| Ok(None), &args).unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported access import version"));
}

#[test]
fn user_diff_with_request_reports_same_state_for_global_bundle() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Viewer", "grafanaAdmin": false}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserDiffArgs {
        common: make_basic_common(),
        diff_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Global,
    };
    let result = diff_users_with_request(
        |method, path, params, _payload| match (method, path) {
            (Method::GET, "/api/users") => {
                assert_eq!(params[0], ("page".to_string(), "1".to_string()));
                Ok(Some(json!([
                    {"id": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer", "isGrafanaAdmin": false}
                ])))
            }
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert_eq!(result.unwrap(), 0);
}

#[test]
fn user_import_with_request_dry_run_reports_global_profile_and_admin_drift() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice Two", "orgRole": "Editor", "grafanaAdmin": true}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Global,
        replace_existing: true,
        dry_run: true,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
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
                (Method::GET, "/api/users") => Ok(Some(json!([
                    {"id": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer", "isGrafanaAdmin": false}
                ]))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 1);
    assert!(calls
        .iter()
        .all(|(method, path, _, _)| !(method == "PUT" && path == "/api/users/7")));
    assert!(
        calls
            .iter()
            .all(|(method, path, _, _)| !(method == "PUT"
                && path == "/api/admin/users/7/permissions"))
    );
}

#[test]
fn user_import_with_request_dry_run_json_reports_global_summary_and_rows() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice Two", "orgRole": "Editor", "grafanaAdmin": true}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Global,
        replace_existing: true,
        dry_run: true,
        table: false,
        json: true,
        output_format: DryRunOutputFormat::Json,
        yes: false,
    };

    let result = import_users_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/users") => Ok(Some(json!([
                {"id": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer", "isGrafanaAdmin": false}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 0);
}

#[test]
fn user_import_with_request_updates_existing_global_user() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice Two", "orgRole": "Editor", "grafanaAdmin": true}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Global,
        replace_existing: true,
        dry_run: false,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
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
                (Method::GET, "/api/users") => Ok(Some(json!([
                    {"id": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer", "isGrafanaAdmin": false}
                ]))),
                (Method::PUT, "/api/users/7") => Ok(Some(json!({"message": "ok"}))),
                (Method::PATCH, "/api/org/users/7") => Ok(Some(json!({"message": "ok"}))),
                (Method::PUT, "/api/admin/users/7/permissions") => Ok(Some(json!({"message": "ok"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 1);
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PUT" && path == "/api/users/7"));
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PATCH" && path == "/api/org/users/7"));
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PUT" && path == "/api/admin/users/7/permissions"));
    let update_payload = calls
        .iter()
        .find(|(method, path, _, _)| method == "PUT" && path == "/api/users/7")
        .and_then(|(_, _, _, payload)| payload.as_ref())
        .expect("expected user update payload");
    assert_eq!(update_payload.get("login"), Some(&json!("alice")));
    assert_eq!(
        update_payload.get("email"),
        Some(&json!("alice@example.com"))
    );
    assert_eq!(update_payload.get("name"), Some(&json!("Alice Two")));
}

#[test]
fn user_import_with_request_creates_missing_global_user_when_password_present() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice", "password": "secret123", "orgRole": "Editor", "grafanaAdmin": true}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserImportArgs {
        common: make_basic_common(),
        input_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Global,
        replace_existing: true,
        dry_run: false,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
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
                (Method::GET, "/api/users") => Ok(Some(json!([]))),
                (Method::POST, "/api/admin/users") => Ok(Some(json!({"id": 7}))),
                (Method::PATCH, "/api/org/users/7") => Ok(Some(json!({"message": "ok"}))),
                (Method::PUT, "/api/admin/users/7/permissions") => {
                    Ok(Some(json!({"message": "ok"})))
                }
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(result, 1);
    let create_payload = calls
        .iter()
        .find(|(method, path, _, _)| method == "POST" && path == "/api/admin/users")
        .and_then(|(_, _, _, payload)| payload.as_ref())
        .expect("expected user create payload");
    assert_eq!(create_payload.get("password"), Some(&json!("secret123")));
}

#[test]
fn user_export_with_request_writes_org_bundle_with_teams() {
    let temp_dir = tempdir().unwrap();
    let args = UserExportArgs {
        common: make_basic_common(),
        output_dir: temp_dir.path().to_path_buf(),
        overwrite: true,
        dry_run: false,
        scope: Scope::Org,
        with_teams: true,
    };
    let result = export_users_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/org/users") => Ok(Some(json!([
                {"userId": "7", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
            ]))),
            (Method::GET, "/api/users/7/teams") => Ok(Some(json!([
                {"id": 11, "name": "ops"},
                {"id": 12, "name": "db"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert!(result.is_ok());
    let bundle: Value =
        serde_json::from_str(&fs::read_to_string(temp_dir.path().join("users.json")).unwrap())
            .unwrap();
    let records = bundle
        .get("records")
        .and_then(Value::as_array)
        .expect("expected user export records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get("login"), Some(&json!("alice")));
    assert_eq!(records[0].get("teams"), Some(&json!(["db", "ops"])));
    let metadata = read_json_file(&temp_dir.path().join("export-metadata.json"));
    assert_eq!(
        metadata.get("kind"),
        Some(&json!("grafana-utils-access-user-export-index"))
    );
    assert_eq!(metadata.get("version"), Some(&json!(1)));
    assert_eq!(metadata.get("recordCount"), Some(&json!(1)));
    assert_eq!(metadata.get("metadataVersion"), Some(&json!(2)));
    assert_eq!(metadata.get("domain"), Some(&json!("access")));
    assert_eq!(metadata.get("resourceKind"), Some(&json!("users")));
    assert_eq!(metadata.get("bundleKind"), Some(&json!("export-root")));
    assert_eq!(metadata["source"]["kind"], json!("live"));
    assert_eq!(metadata["capture"]["recordCount"], json!(1));
}

#[test]
fn user_diff_with_request_reports_same_state_for_org_bundle_with_teams() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("users.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": [
                {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Viewer", "teams": ["db", "ops"]}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = UserDiffArgs {
        common: make_basic_common(),
        diff_dir: temp_dir.path().to_path_buf(),
        scope: Scope::Org,
    };
    let result = diff_users_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/org/users") => Ok(Some(json!([
                {"userId": "7", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
            ]))),
            (Method::GET, "/api/users/7/teams") => Ok(Some(json!([
                {"id": 11, "name": "ops"},
                {"id": 12, "name": "db"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert_eq!(result.unwrap(), 0);
}

#[test]
fn run_access_cli_with_request_routes_user_list_local_input_dir() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-users");
    write_local_access_bundle(
        &input_dir,
        "users.json",
        r#"{
            "kind":"grafana-utils-access-user-export-index",
            "version":1,
            "records":[
                {"login":"alice","email":"alice@example.com","name":"Alice","orgRole":"Editor","teams":["ops"]}
            ]
        }"#,
    );

    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "list",
        "--input-dir",
        input_dir.to_str().unwrap(),
        "--scope",
        "org",
        "--output-format",
        "json",
    ]);
    let mut request_called = false;
    let result = run_access_cli_with_request(
        |_method, _path, _params, _payload| {
            request_called = true;
            panic!("local user list should not hit the request layer");
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(!request_called);
}
