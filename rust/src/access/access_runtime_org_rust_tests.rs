use super::*;

#[test]
fn run_access_cli_with_request_routes_user_export() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "export",
        "--scope",
        "global",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--dry-run",
    ]);
    let result = run_access_cli_with_request(
        |method, path, _params, _payload| {
            assert_eq!(method.to_string(), Method::GET.to_string());
            if path == "/api/users" {
                Ok(Some(json!([])))
            } else {
                panic!("unexpected path {path}");
            }
        },
        &args,
    );
    assert!(result.is_ok());
}

#[test]
fn run_access_cli_with_request_routes_team_export() {
    let args = parse_cli_from(["grafana-util access", "team", "export", "--dry-run"]);
    let result = run_access_cli_with_request(
        |method, path, _params, _payload| {
            assert_eq!(method.to_string(), Method::GET.to_string());
            if path == "/api/teams/search" {
                Ok(Some(json!({"teams": []})))
            } else {
                panic!("unexpected path {path}");
            }
        },
        &args,
    );
    assert!(result.is_ok());
}

#[test]
fn run_access_cli_with_request_routes_team_import() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("access-teams");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("teams.json"),
        r#"[{"name":"Ops","email":"ops@example.com"}]"#,
    )
    .unwrap();

    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "import",
        "--import-dir",
        import_dir.to_str().unwrap(),
    ]);
    let mut calls = Vec::new();
    let result = run_access_cli_with_request(
        |method, path, _params, _payload| {
            calls.push((method.to_string(), path.to_string()));
            match (method, path) {
                (Method::GET, "/api/teams/search") => Ok(Some(json!({"teams": []}))),
                (Method::POST, "/api/teams") => Ok(Some(json!({"teamId": "3"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path)| method == "GET" && path == "/api/teams/search"));
    assert!(calls
        .iter()
        .any(|(method, path)| method == "POST" && path == "/api/teams"));
}

#[test]
fn run_access_cli_with_request_routes_org_export() {
    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "export",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--dry-run",
        "--with-users",
    ]);
    let result = run_access_cli_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([{"id": 1, "name": "Main Org"}]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "role": "Admin"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );
    assert!(result.is_ok());
}

#[test]
fn run_access_cli_with_request_routes_org_import() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("orgs.json"),
        r#"{
            "kind":"grafana-utils-access-org-export-index",
            "version":1,
            "records":[
                {
                    "name":"Main Org",
                    "users":[
                        {"login":"alice","email":"alice@example.com","orgRole":"Editor"}
                    ]
                }
            ]
        }"#,
    )
    .unwrap();
    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "import",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--import-dir",
        import_dir.to_str().unwrap(),
        "--replace-existing",
    ]);
    let mut calls = Vec::new();
    let result = run_access_cli_with_request(
        |method, path, _params, payload| {
            calls.push((method.to_string(), path.to_string()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([]))),
                (Method::POST, "/api/orgs") => {
                    assert_eq!(
                        payload
                            .and_then(|value| value.as_object())
                            .unwrap()
                            .get("name"),
                        Some(&json!("Main Org"))
                    );
                    Ok(Some(json!({"orgId": "3"})))
                }
                (Method::GET, "/api/orgs/3/users") => Ok(Some(json!([]))),
                (Method::POST, "/api/orgs/3/users") => Ok(Some(json!({"message": "added"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );
    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path)| method == "POST" && path == "/api/orgs"));
    assert!(calls
        .iter()
        .any(|(method, path)| method == "POST" && path == "/api/orgs/3/users"));
}

#[test]
fn run_access_cli_with_request_routes_org_diff() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&diff_dir).unwrap();
    fs::write(
        diff_dir.join("orgs.json"),
        r#"{
            "kind":"grafana-utils-access-org-export-index",
            "version":1,
            "records":[
                {
                    "name":"Main Org",
                    "users":[
                        {"login":"alice","email":"alice@example.com","orgRole":"Editor"}
                    ]
                }
            ]
        }"#,
    )
    .unwrap();
    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "diff",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--diff-dir",
        diff_dir.to_str().unwrap(),
    ]);
    let result = run_access_cli_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([{"id": 1, "name": "Main Org"}]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Editor"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );
    assert!(result.is_ok());
}

#[test]
fn org_diff_with_request_reports_same_state() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&diff_dir).unwrap();
    let bundle = json!({
        "kind": "grafana-utils-access-org-export-index",
        "version": 1,
        "records": [
            {
                "name": "Main Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Viewer"}
                ]
            }
        ]
    });
    fs::write(
        diff_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgDiffArgs {
        common: make_basic_common_no_org_id(),
        diff_dir: diff_dir.clone(),
    };
    let result = diff_orgs_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([{"id": 1, "name": "Main Org"}]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn org_diff_with_request_reports_user_role_drift() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&diff_dir).unwrap();
    let bundle = json!({
        "kind": "grafana-utils-access-org-export-index",
        "version": 1,
        "records": [
            {
                "name": "Main Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor"}
                ]
            }
        ]
    });
    fs::write(
        diff_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgDiffArgs {
        common: make_basic_common_no_org_id(),
        diff_dir: diff_dir.clone(),
    };
    let result = diff_orgs_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([{"id": 1, "name": "Main Org"}]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 1);
}

#[test]
fn org_export_with_request_writes_bundle_with_users() {
    let temp_dir = tempdir().unwrap();
    let args = OrgExportArgs {
        common: make_basic_common_no_org_id(),
        org_id: None,
        export_dir: temp_dir.path().to_path_buf(),
        overwrite: true,
        dry_run: false,
        name: Some("Main Org".to_string()),
        with_users: true,
    };
    let result = export_orgs_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 1, "name": "Main Org"},
                {"id": 2, "name": "Other Org"}
            ]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Editor"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert!(result.is_ok());
    let bundle: Value =
        serde_json::from_str(&fs::read_to_string(temp_dir.path().join("orgs.json")).unwrap())
            .unwrap();
    assert_eq!(
        bundle.get("kind"),
        Some(&json!("grafana-utils-access-org-export-index"))
    );
    let records = bundle
        .get("records")
        .and_then(Value::as_array)
        .expect("expected org export records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get("name"), Some(&json!("Main Org")));
    assert_eq!(
        records[0]
            .get("users")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        records[0]
            .get("users")
            .and_then(Value::as_array)
            .and_then(|users| users.first())
            .and_then(|user| user.get("orgRole")),
        Some(&json!("Editor"))
    );
    let metadata = read_json_file(&temp_dir.path().join("export-metadata.json"));
    assert_eq!(
        metadata.get("kind"),
        Some(&json!("grafana-utils-access-org-export-index"))
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
}

#[test]
fn org_import_rejects_kind_mismatch_and_future_version_bundle_contract() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("orgs.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-user-export-index",
            "version": 1,
            "records": []
        }))
        .unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        import_dir: temp.path().to_path_buf(),
        replace_existing: true,
        dry_run: true,
        yes: false,
    };
    let error =
        import_orgs_with_request(|_method, _path, _params, _payload| Ok(None), &args).unwrap_err();
    assert!(error.to_string().contains("Access import kind mismatch"));

    fs::write(
        temp.path().join("orgs.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-org-export-index",
            "version": 99,
            "records": []
        }))
        .unwrap(),
    )
    .unwrap();
    let error =
        import_orgs_with_request(|_method, _path, _params, _payload| Ok(None), &args).unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported access import version"));
}

#[test]
fn org_import_with_request_dry_run_reports_user_role_update_without_mutating() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&import_dir).unwrap();
    let bundle = json!({
        "kind": "grafana-utils-access-org-export-index",
        "version": 1,
        "records": [
            {
                "name": "Main Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor"}
                ]
            }
        ]
    });
    fs::write(
        import_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        import_dir: import_dir.clone(),
        replace_existing: true,
        dry_run: true,
        yes: true,
    };
    let mut calls = Vec::new();
    let result = import_orgs_with_request(
        |method, path, _params, payload| {
            calls.push((method.to_string(), path.to_string(), payload.cloned()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([{"id": 1, "name": "Main Org"}]))),
                (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                    {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
                ]))),
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
                ]))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 0);
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "GET" && path == "/api/orgs"));
    assert!(!calls
        .iter()
        .any(|(method, path, _)| method == "PATCH" && path == "/api/orgs/1/users/7"));
    assert!(!calls
        .iter()
        .any(|(method, path, _)| method == "POST" && path == "/api/orgs"));
}

#[test]
fn org_import_with_request_updates_existing_org_users() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&import_dir).unwrap();
    let bundle = json!({
        "kind": "grafana-utils-access-org-export-index",
        "version": 1,
        "records": [
            {
                "name": "Main Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor"}
                ]
            }
        ]
    });
    fs::write(
        import_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        import_dir: import_dir.clone(),
        replace_existing: true,
        dry_run: false,
        yes: true,
    };
    let mut calls = Vec::new();
    let result = import_orgs_with_request(
        |method, path, _params, payload| {
            calls.push((method.to_string(), path.to_string(), payload.cloned()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([{"id": 1, "name": "Main Org"}]))),
                (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                    {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
                ]))),
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
                ]))),
                (Method::PATCH, "/api/orgs/1/users/7") => Ok(Some(json!({"message": "ok"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 0);
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "PATCH" && path == "/api/orgs/1/users/7"));
}

#[test]
fn org_import_with_request_creates_missing_org_and_users_when_replace_existing_is_set() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&import_dir).unwrap();
    let bundle = json!({
        "kind": "grafana-utils-access-org-export-index",
        "version": 1,
        "records": [
            {
                "name": "New Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor"}
                ]
            }
        ]
    });
    fs::write(
        import_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        import_dir: import_dir.clone(),
        replace_existing: true,
        dry_run: false,
        yes: true,
    };
    let mut calls = Vec::new();
    let result = import_orgs_with_request(
        |method, path, _params, payload| {
            calls.push((method.to_string(), path.to_string(), payload.cloned()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([]))),
                (Method::POST, "/api/orgs") => {
                    assert_eq!(
                        payload
                            .and_then(|value| value.as_object())
                            .unwrap()
                            .get("name"),
                        Some(&json!("New Org"))
                    );
                    Ok(Some(json!({"orgId": "3"})))
                }
                (Method::GET, "/api/orgs/3/users") => Ok(Some(json!([]))),
                (Method::POST, "/api/orgs/3/users") => Ok(Some(json!({"message": "added"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 0);
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "POST" && path == "/api/orgs"));
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "POST" && path == "/api/orgs/3/users"));
}

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
fn run_access_cli_with_request_routes_team_diff() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-teams");
    fs::create_dir_all(&diff_dir).unwrap();
    fs::write(
        diff_dir.join("teams.json"),
        r#"[{"name":"Ops","email":"ops@example.com"}]"#,
    )
    .unwrap();

    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "diff",
        "--diff-dir",
        diff_dir.to_str().unwrap(),
    ]);
    let result = run_access_cli_with_request(
        |_method, path, _params, _payload| match path {
            "/api/teams/search" => Ok(Some(
                json!({"teams": [{"id": "3", "name":"Ops", "email":"ops@example.com"}]}),
            )),
            _ => panic!("unexpected path {path}"),
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
        export_dir: temp_dir.path().to_path_buf(),
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
                    {"id": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "isGrafanaAdmin": false}
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
        import_dir: temp_dir.path().to_path_buf(),
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
        import_dir: temp_dir.path().to_path_buf(),
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
        import_dir: temp_dir.path().to_path_buf(),
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
        import_dir: temp_dir.path().to_path_buf(),
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
        import_dir: temp_dir.path().to_path_buf(),
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
        export_dir: temp_dir.path().to_path_buf(),
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
