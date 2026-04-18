use super::*;

#[test]
fn org_export_with_request_writes_bundle_with_users() {
    let temp_dir = tempdir().unwrap();
    let args = OrgExportArgs {
        common: make_basic_common_no_org_id(),
        org_id: None,
        output_dir: temp_dir.path().to_path_buf(),
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
    assert_eq!(metadata.get("metadataVersion"), Some(&json!(2)));
    assert_eq!(metadata.get("domain"), Some(&json!("access")));
    assert_eq!(metadata.get("resourceKind"), Some(&json!("orgs")));
    assert_eq!(metadata.get("bundleKind"), Some(&json!("export-root")));
    assert_eq!(metadata["source"]["kind"], json!("live"));
    assert_eq!(metadata["capture"]["recordCount"], json!(1));
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
        input_dir: temp.path().to_path_buf(),
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
    let input_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&input_dir).unwrap();
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
        input_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        input_dir: input_dir.clone(),
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
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "GET" && path == "/api/orgs/1/users"));
    assert!(!calls
        .iter()
        .any(|(method, path, _)| method == "PATCH" && path == "/api/orgs/1/users/7"));
    assert!(!calls
        .iter()
        .any(|(method, path, _)| method == "POST" && path == "/api/orgs"));
}

#[test]
fn org_import_with_request_blocks_externally_synced_org_user_role_update_before_mutation() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&input_dir).unwrap();
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
        input_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        input_dir: input_dir.clone(),
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
                    {
                        "userId": 7,
                        "login": "alice",
                        "email": "alice@example.com",
                        "name": "Alice",
                        "role": "Viewer",
                        "isExternallySynced": true,
                        "authLabels": ["LDAP"]
                    }
                ]))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    let error = result.unwrap_err();
    assert!(error
        .to_string()
        .contains("externally synced user orgRole cannot be updated through Grafana org user API"));
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "GET" && path == "/api/orgs/1/users"));
    assert!(!calls
        .iter()
        .any(|(method, path, _)| method == "PATCH" && path == "/api/orgs/1/users/7"));
}

#[test]
fn org_import_with_request_adds_missing_org_user_to_existing_org() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&input_dir).unwrap();
    let bundle = json!({
        "kind": "grafana-utils-access-org-export-index",
        "version": 1,
        "records": [
            {
                "name": "Main Org",
                "users": [
                    {"login": "bob", "email": "bob@example.com", "name": "Bob", "orgRole": "Viewer"}
                ]
            }
        ]
    });
    fs::write(
        input_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        input_dir: input_dir.clone(),
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
                (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([]))),
                (Method::POST, "/api/orgs/1/users") => {
                    assert_eq!(
                        payload
                            .and_then(|value| value.as_object())
                            .unwrap()
                            .get("loginOrEmail"),
                        Some(&json!("bob"))
                    );
                    assert_eq!(
                        payload
                            .and_then(|value| value.as_object())
                            .unwrap()
                            .get("role"),
                        Some(&json!("Viewer"))
                    );
                    Ok(Some(json!({"message": "added"})))
                }
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "POST" && path == "/api/orgs/1/users"));
}

#[test]
fn org_import_with_request_updates_existing_org_users() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&input_dir).unwrap();
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
        input_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        input_dir: input_dir.clone(),
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
    let input_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&input_dir).unwrap();
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
        input_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgImportArgs {
        common: make_basic_common_no_org_id(),
        input_dir: input_dir.clone(),
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
