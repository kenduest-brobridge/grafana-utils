use super::*;

#[test]
fn run_access_cli_with_request_routes_user_export() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "export",
        "--url",
        "https://grafana.example.com",
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
    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "export",
        "--url",
        "https://grafana.example.com",
        "--dry-run",
    ]);
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
    let input_dir = temp.path().join("access-teams");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("teams.json"),
        r#"[{"name":"Ops","email":"ops@example.com"}]"#,
    )
    .unwrap();

    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "import",
        "--input-dir",
        input_dir.to_str().unwrap(),
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
        "--url",
        "https://grafana.example.com",
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
    let input_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(
        input_dir.join("orgs.json"),
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
        "--url",
        "https://grafana.example.com",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--input-dir",
        input_dir.to_str().unwrap(),
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
        "--url",
        "https://grafana.example.com",
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
