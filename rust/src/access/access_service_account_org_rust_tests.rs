//! Rust regression coverage for Access behavior at this module boundary.

use super::*;

#[test]
fn service_account_list_with_request_reads_search() {
    let args = ServiceAccountListArgs {
        common: make_token_common(),
        query: Some("svc".to_string()),
        page: 1,
        per_page: 100,
        table: false,
        csv: false,
        json: false,
        yaml: true,
        output_format: None,
    };
    let mut calls = Vec::new();
    let result = list_service_accounts_command_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match path {
                "/api/serviceaccounts/search" => Ok(Some(
                    json!({"serviceAccounts": [{"id": 4, "name": "svc", "login": "sa-svc", "role": "Viewer", "isDisabled": false, "tokens": 1, "orgId": 1}]}),
                )),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert_eq!(result.unwrap(), 1);
    assert_eq!(calls[0].1, "/api/serviceaccounts/search");
}

#[test]
fn service_account_add_with_request_creates_account() {
    let args = ServiceAccountAddArgs {
        common: make_token_common(),
        name: "svc".to_string(),
        role: "Viewer".to_string(),
        disabled: false,
        json: true,
    };
    let mut calls = Vec::new();
    let result = add_service_account_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/serviceaccounts" => Ok(Some(
                    json!({"id": 4, "name": "svc", "login": "sa-svc", "role": "Viewer", "isDisabled": false, "tokens": 0, "orgId": 1}),
                )),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert_eq!(calls[0].1, "/api/serviceaccounts");
}

#[test]
fn service_account_export_with_request_writes_bundle() {
    let temp_dir = tempdir().unwrap();
    let args = ServiceAccountExportArgs {
        common: make_token_common(),
        export_dir: temp_dir.path().to_path_buf(),
        overwrite: true,
        dry_run: false,
    };
    let result = export_service_accounts_with_request(
        |_method, path, _params, _payload| match path {
            "/api/serviceaccounts/search" => Ok(Some(json!({
                "serviceAccounts": [
                    {"id": 4, "name": "svc", "login": "sa-svc", "role": "Viewer", "isDisabled": false, "tokens": 1, "orgId": 1}
                ]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );
    assert!(result.is_ok());
    let bundle = read_json_file(&temp_dir.path().join("service-accounts.json"));
    assert_eq!(
        bundle.get("kind"),
        Some(&json!("grafana-utils-access-service-account-export-index"))
    );
    assert_eq!(bundle.get("version"), Some(&json!(1)));
    assert_eq!(
        bundle
            .get("records")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    let metadata = read_json_file(&temp_dir.path().join("export-metadata.json"));
    assert_eq!(
        metadata.get("kind"),
        Some(&json!("grafana-utils-access-service-account-export-index"))
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
fn service_account_import_rejects_kind_mismatch_and_future_version_bundle_contract() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("service-accounts.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-team-export-index",
            "version": 1,
            "records": []
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ServiceAccountImportArgs {
        common: make_token_common(),
        import_dir: temp_dir.path().to_path_buf(),
        replace_existing: true,
        dry_run: true,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
    };
    let error =
        import_service_accounts_with_request(|_method, _path, _params, _payload| Ok(None), &args)
            .unwrap_err();
    assert!(error.to_string().contains("Access import kind mismatch"));

    fs::write(
        temp_dir.path().join("service-accounts.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-service-account-export-index",
            "version": 99,
            "records": []
        }))
        .unwrap(),
    )
    .unwrap();
    let error =
        import_service_accounts_with_request(|_method, _path, _params, _payload| Ok(None), &args)
            .unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported access import version"));
}

#[test]
fn access_import_rejects_kind_mismatch_and_future_version_from_shared_fixture() {
    for case in load_access_bundle_contract_cases() {
        let domain = case.get("domain").and_then(Value::as_str).unwrap_or("");
        let bundle_file = case.get("bundleFile").and_then(Value::as_str).unwrap_or("");
        let expected_kind = case
            .get("expectedKind")
            .and_then(Value::as_str)
            .unwrap_or("");
        let mismatched_kind = case
            .get("mismatchedKind")
            .and_then(Value::as_str)
            .unwrap_or("");

        let temp_dir = tempdir().unwrap();
        fs::write(
            temp_dir.path().join(bundle_file),
            serde_json::to_string_pretty(&json!({
                "kind": mismatched_kind,
                "version": 1,
                "records": []
            }))
            .unwrap(),
        )
        .unwrap();

        let mismatched_error = match domain {
            "user" => import_users_with_request(
                |_method, _path, _params, _payload| Ok(None),
                &UserImportArgs {
                    common: make_basic_common(),
                    import_dir: temp_dir.path().to_path_buf(),
                    scope: Scope::Global,
                    replace_existing: true,
                    dry_run: true,
                    table: false,
                    json: false,
                    output_format: DryRunOutputFormat::Text,
                    yes: false,
                },
            )
            .unwrap_err()
            .to_string(),
            "team" => import_teams_with_request(
                |_method, _path, _params, _payload| Ok(None),
                &TeamImportArgs {
                    common: make_token_common(),
                    import_dir: temp_dir.path().to_path_buf(),
                    replace_existing: true,
                    dry_run: true,
                    table: false,
                    json: false,
                    output_format: DryRunOutputFormat::Text,
                    yes: false,
                },
            )
            .unwrap_err()
            .to_string(),
            "org" => import_orgs_with_request(
                |_method, _path, _params, _payload| Ok(None),
                &OrgImportArgs {
                    common: make_basic_common_no_org_id(),
                    import_dir: temp_dir.path().to_path_buf(),
                    replace_existing: true,
                    dry_run: true,
                    yes: false,
                },
            )
            .unwrap_err()
            .to_string(),
            "service-account" => import_service_accounts_with_request(
                |_method, _path, _params, _payload| Ok(None),
                &ServiceAccountImportArgs {
                    common: make_token_common(),
                    import_dir: temp_dir.path().to_path_buf(),
                    replace_existing: true,
                    dry_run: true,
                    table: false,
                    json: false,
                    output_format: DryRunOutputFormat::Text,
                    yes: false,
                },
            )
            .unwrap_err()
            .to_string(),
            other => panic!("unexpected access bundle contract fixture domain {other}"),
        };
        assert!(mismatched_error.contains("Access import kind mismatch"));

        fs::write(
            temp_dir.path().join(bundle_file),
            serde_json::to_string_pretty(&json!({
                "kind": expected_kind,
                "version": 99,
                "records": []
            }))
            .unwrap(),
        )
        .unwrap();

        let future_version_error = match domain {
            "user" => import_users_with_request(
                |_method, _path, _params, _payload| Ok(None),
                &UserImportArgs {
                    common: make_basic_common(),
                    import_dir: temp_dir.path().to_path_buf(),
                    scope: Scope::Global,
                    replace_existing: true,
                    dry_run: true,
                    table: false,
                    json: false,
                    output_format: DryRunOutputFormat::Text,
                    yes: false,
                },
            )
            .unwrap_err()
            .to_string(),
            "team" => import_teams_with_request(
                |_method, _path, _params, _payload| Ok(None),
                &TeamImportArgs {
                    common: make_token_common(),
                    import_dir: temp_dir.path().to_path_buf(),
                    replace_existing: true,
                    dry_run: true,
                    table: false,
                    json: false,
                    output_format: DryRunOutputFormat::Text,
                    yes: false,
                },
            )
            .unwrap_err()
            .to_string(),
            "org" => import_orgs_with_request(
                |_method, _path, _params, _payload| Ok(None),
                &OrgImportArgs {
                    common: make_basic_common_no_org_id(),
                    import_dir: temp_dir.path().to_path_buf(),
                    replace_existing: true,
                    dry_run: true,
                    yes: false,
                },
            )
            .unwrap_err()
            .to_string(),
            "service-account" => import_service_accounts_with_request(
                |_method, _path, _params, _payload| Ok(None),
                &ServiceAccountImportArgs {
                    common: make_token_common(),
                    import_dir: temp_dir.path().to_path_buf(),
                    replace_existing: true,
                    dry_run: true,
                    table: false,
                    json: false,
                    output_format: DryRunOutputFormat::Text,
                    yes: false,
                },
            )
            .unwrap_err()
            .to_string(),
            other => panic!("unexpected access bundle contract fixture domain {other}"),
        };
        assert!(future_version_error.contains("Unsupported access import version"));
    }
}

#[test]
fn service_account_import_rejects_structured_output_without_dry_run() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("service-accounts.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-service-account-export-index",
            "version": 1,
            "records": [
                {"name": "svc", "role": "Viewer", "disabled": false}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ServiceAccountImportArgs {
        common: make_token_common(),
        import_dir: temp_dir.path().to_path_buf(),
        replace_existing: true,
        dry_run: false,
        table: false,
        json: true,
        output_format: DryRunOutputFormat::Text,
        yes: false,
    };

    let error =
        import_service_accounts_with_request(|_method, _path, _params, _payload| Ok(None), &args)
            .unwrap_err();

    assert!(error
        .to_string()
        .contains("--table/--json for service-account import are only supported with --dry-run."));
}

#[test]
fn service_account_import_with_request_creates_missing_when_replace_existing_is_set() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("service-accounts.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-service-account-export-index",
            "version": 1,
            "records": [
                {"name": "svc-create", "role": "Editor", "disabled": true}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ServiceAccountImportArgs {
        common: make_token_common(),
        import_dir: temp_dir.path().to_path_buf(),
        replace_existing: true,
        dry_run: false,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
    };
    let mut calls = Vec::new();
    let result = import_service_accounts_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/serviceaccounts/search" => Ok(Some(json!({"serviceAccounts": []}))),
                "/api/serviceaccounts" if method == Method::POST => Ok(Some(json!({
                    "id": 7,
                    "name": "svc-create",
                    "login": "sa-svc-create",
                    "role": "Editor",
                    "isDisabled": true,
                    "tokens": 0,
                    "orgId": 1
                }))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    let create_call = calls
        .iter()
        .find(|(method, path, _, _)| method == "POST" && path == "/api/serviceaccounts")
        .expect("expected service-account create call");
    let payload = create_call
        .3
        .as_ref()
        .expect("expected service-account create payload");
    assert_eq!(payload.get("name"), Some(&json!("svc-create")));
    assert_eq!(payload.get("role"), Some(&json!("Editor")));
    assert_eq!(payload.get("isDisabled"), Some(&json!(true)));
}

#[test]
fn service_account_import_with_request_updates_existing() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("service-accounts.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-service-account-export-index",
            "version": 1,
            "records": [
                {"name": "svc", "role": "Editor", "disabled": true}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ServiceAccountImportArgs {
        common: make_token_common(),
        import_dir: temp_dir.path().to_path_buf(),
        replace_existing: true,
        dry_run: false,
        table: false,
        json: false,
        output_format: DryRunOutputFormat::Text,
        yes: false,
    };
    let mut calls = Vec::new();
    let result = import_service_accounts_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/serviceaccounts/search" => Ok(Some(json!({
                    "serviceAccounts": [
                        {"id": 4, "name": "svc", "login": "sa-svc", "role": "Viewer", "isDisabled": false, "tokens": 0, "orgId": 1}
                    ]
                }))),
                "/api/serviceaccounts/4" if method == Method::PATCH => Ok(Some(json!({
                    "id": 4, "name": "svc", "login": "sa-svc", "role": "Editor", "isDisabled": true, "tokens": 0, "orgId": 1
                }))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );
    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PATCH" && path == "/api/serviceaccounts/4"));
}

#[test]
fn service_account_diff_with_request_reports_same_state() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("service-accounts.json"),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-access-service-account-export-index",
            "version": 1,
            "records": [
                {"name": "svc", "role": "Viewer", "disabled": false}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ServiceAccountDiffArgs {
        common: make_token_common(),
        diff_dir: temp_dir.path().to_path_buf(),
    };
    let result = diff_service_accounts_with_request(
        |_method, path, _params, _payload| match path {
            "/api/serviceaccounts/search" => Ok(Some(json!({
                "serviceAccounts": [
                    {"id": 4, "name": "svc", "login": "sa-svc", "role": "Viewer", "isDisabled": false, "tokens": 0, "orgId": 1}
                ]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );

    assert_eq!(result.unwrap(), 0);
}

#[test]
fn service_account_diff_with_request_reports_differences() {
    let temp_dir = tempdir().unwrap();
    fs::write(
        temp_dir.path().join("service-accounts.json"),
        serde_json::to_string_pretty(&json!([
            {"name": "svc", "role": "Editor", "disabled": false},
            {"name": "missing", "role": "Viewer", "disabled": false}
        ]))
        .unwrap(),
    )
    .unwrap();
    let args = ServiceAccountDiffArgs {
        common: make_token_common(),
        diff_dir: temp_dir.path().to_path_buf(),
    };
    let result = diff_service_accounts_with_request(
        |_method, path, _params, _payload| match path {
            "/api/serviceaccounts/search" => Ok(Some(json!({
                "serviceAccounts": [
                    {"id": 4, "name": "svc", "login": "sa-svc", "role": "Viewer", "isDisabled": false, "tokens": 0, "orgId": 1},
                    {"id": 5, "name": "extra", "login": "sa-extra", "role": "Viewer", "isDisabled": false, "tokens": 0, "orgId": 1}
                ]
            }))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    );
    assert_eq!(result.unwrap(), 3);
}

#[test]
fn service_account_token_add_with_request_resolves_name() {
    let args = ServiceAccountTokenAddArgs {
        common: make_token_common(),
        service_account_id: None,
        name: Some("svc".to_string()),
        token_name: "automation".to_string(),
        seconds_to_live: Some(3600),
        json: true,
    };
    let mut calls = Vec::new();
    let result = add_service_account_token_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            match path {
                "/api/serviceaccounts/search" => {
                    Ok(Some(json!({"serviceAccounts": [{"id": 4, "name": "svc"}]})))
                }
                "/api/serviceaccounts/4/tokens" => {
                    Ok(Some(json!({"name": "automation", "key": "token"})))
                }
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(_, path, _, _)| path == "/api/serviceaccounts/4/tokens"));
}

#[test]
fn team_delete_with_request_deletes_resolved_team() {
    let args = TeamDeleteArgs {
        common: make_token_common(),
        team_id: None,
        name: Some("Ops".to_string()),
        yes: true,
        json: true,
    };
    let mut calls = Vec::new();
    let result = delete_team_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match path {
                "/api/teams/search" => Ok(Some(
                    json!({"teams": [{"id": 3, "name": "Ops", "email": "ops@example.com"}]}),
                )),
                "/api/teams/3" if method == Method::DELETE => {
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
        .any(|(method, path, _)| method == "DELETE" && path == "/api/teams/3"));
}

#[test]
fn service_account_delete_with_request_deletes_by_name() {
    let args = ServiceAccountDeleteArgs {
        common: make_token_common(),
        service_account_id: None,
        name: Some("svc".to_string()),
        yes: true,
        json: false,
    };
    let mut calls = Vec::new();
    let result = delete_service_account_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match path {
                "/api/serviceaccounts/search" => Ok(Some(
                    json!({"serviceAccounts": [{"id": 4, "name": "svc", "login": "sa-svc"}]}),
                )),
                "/api/serviceaccounts/4" if method == Method::GET => {
                    Ok(Some(json!({"id": 4, "name": "svc", "login": "sa-svc"})))
                }
                "/api/serviceaccounts/4" if method == Method::DELETE => {
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
        .any(|(method, path, _)| method == "DELETE" && path == "/api/serviceaccounts/4"));
}

#[test]
fn service_account_token_delete_with_request_resolves_token_name() {
    let args = ServiceAccountTokenDeleteArgs {
        common: make_token_common(),
        service_account_id: Some("4".to_string()),
        name: None,
        token_id: None,
        token_name: Some("automation".to_string()),
        yes: true,
        json: true,
    };
    let mut calls = Vec::new();
    let result = delete_service_account_token_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match path {
                "/api/serviceaccounts/4" => Ok(Some(json!({"id": 4, "name": "svc"}))),
                "/api/serviceaccounts/4/tokens" if method == Method::GET => Ok(Some(json!([
                    {"id": 7, "name": "automation"},
                    {"id": 8, "name": "adhoc"}
                ]))),
                "/api/serviceaccounts/4/tokens/7" if method == Method::DELETE => {
                    Ok(Some(json!({"message": "deleted"})))
                }
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(calls.iter().any(|(method, path, _)| {
        method == "DELETE" && path == "/api/serviceaccounts/4/tokens/7"
    }));
}

#[test]
fn list_orgs_with_request_reads_orgs_and_memberships() {
    let args = OrgListArgs {
        common: make_basic_common_no_org_id(),
        org_id: None,
        name: None,
        query: None,
        with_users: true,
        table: false,
        csv: false,
        json: false,
        yaml: true,
        output_format: None,
    };
    let mut calls = Vec::new();
    let result = list_orgs_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 1, "name": "Main Org"}
                ]))),
                (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                    {"userId": 7, "login": "alice", "email": "alice@example.com", "role": "Admin"}
                ]))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );
    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "GET" && path == "/api/orgs/1/users"));
}

#[test]
fn modify_org_with_request_renames_resolved_org() {
    let args = OrgModifyArgs {
        common: make_basic_common_no_org_id(),
        org_id: Some(4),
        name: None,
        set_name: "Renamed Org".to_string(),
        json: false,
    };
    let mut calls = Vec::new();
    let result = modify_org_with_request(
        |method, path, params, payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 4, "name": "Main Org"}
                ]))),
                (Method::PUT, "/api/orgs/4") => {
                    assert_eq!(
                        payload
                            .and_then(|value| value.as_object())
                            .unwrap()
                            .get("name"),
                        Some(&json!("Renamed Org"))
                    );
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
        .any(|(method, path, _)| method == "PUT" && path == "/api/orgs/4"));
}

#[test]
fn delete_org_with_request_deletes_resolved_org() {
    let args = OrgDeleteArgs {
        common: make_basic_common_no_org_id(),
        org_id: None,
        name: Some("Main Org".to_string()),
        yes: true,
        json: true,
    };
    let mut calls = Vec::new();
    let result = delete_org_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            match (method, path) {
                (Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 4, "name": "Main Org"}
                ]))),
                (Method::DELETE, "/api/orgs/4") => Ok(Some(json!({"message": "deleted"}))),
                _ => panic!("unexpected path {path}"),
            }
        },
        &args,
    );
    assert!(result.is_ok());
    assert!(calls
        .iter()
        .any(|(method, path, _)| method == "DELETE" && path == "/api/orgs/4"));
}
