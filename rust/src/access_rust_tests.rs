use super::{
    add_service_account_token_with_request, add_service_account_with_request,
    add_team_with_request, add_user_with_request, delete_service_account_token_with_request,
    delete_service_account_with_request, delete_team_with_request, delete_user_with_request,
    list_service_accounts_command_with_request, list_teams_command_with_request,
    list_users_with_request, modify_team_with_request, modify_user_with_request, parse_cli_from,
    run_access_cli_with_request, AccessCommand, CommonCliArgs, Scope, ServiceAccountAddArgs,
    ServiceAccountCommand, ServiceAccountDeleteArgs, ServiceAccountListArgs,
    ServiceAccountTokenAddArgs, ServiceAccountTokenCommand, ServiceAccountTokenDeleteArgs,
    TeamAddArgs, TeamCommand, TeamDeleteArgs, TeamListArgs, TeamModifyArgs, UserAddArgs,
    UserCommand, UserDeleteArgs, UserListArgs, UserModifyArgs,
};
use reqwest::Method;
use serde_json::json;

fn make_token_common() -> CommonCliArgs {
    CommonCliArgs {
        url: "http://127.0.0.1:3000".to_string(),
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        org_id: None,
        timeout: 30,
        verify_ssl: false,
    }
}

fn make_basic_common() -> CommonCliArgs {
    CommonCliArgs {
        url: "http://127.0.0.1:3000".to_string(),
        api_token: None,
        username: Some("admin".to_string()),
        password: Some("secret".to_string()),
        prompt_password: false,
        org_id: None,
        timeout: 30,
        verify_ssl: false,
    }
}

#[test]
fn parse_cli_supports_user_list() {
    let args = parse_cli_from([
        "grafana-access-utils",
        "user",
        "list",
        "--scope",
        "global",
        "--table",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert_eq!(list_args.scope, Scope::Global);
            assert!(list_args.table);
            assert!(!list_args.csv);
            assert!(!list_args.json);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_user_list_output_format_json() {
    let args = parse_cli_from([
        "grafana-access-utils",
        "user",
        "list",
        "--output-format",
        "json",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert!(list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_service_account_token_add() {
    let args = parse_cli_from([
        "grafana-access-utils",
        "service-account",
        "token",
        "add",
        "--name",
        "sa-one",
        "--token-name",
        "automation",
    ]);

    match args.command {
        AccessCommand::ServiceAccount {
            command:
                ServiceAccountCommand::Token {
                    command: ServiceAccountTokenCommand::Add(token_args),
                },
        } => {
            assert_eq!(token_args.name.as_deref(), Some("sa-one"));
            assert_eq!(token_args.token_name, "automation");
        }
        _ => panic!("expected service-account token add"),
    }
}

#[test]
fn parse_cli_supports_group_delete_alias() {
    let args = parse_cli_from([
        "grafana-access-utils",
        "group",
        "delete",
        "--team-id",
        "7",
        "--yes",
    ]);

    match args.command {
        AccessCommand::Team {
            command: TeamCommand::Delete(delete_args),
        } => {
            assert_eq!(delete_args.team_id.as_deref(), Some("7"));
            assert!(delete_args.yes);
        }
        _ => panic!("expected group alias delete"),
    }
}

#[test]
fn parse_cli_supports_service_account_token_delete() {
    let args = parse_cli_from([
        "grafana-access-utils",
        "service-account",
        "token",
        "delete",
        "--name",
        "svc",
        "--token-name",
        "automation",
        "--yes",
    ]);

    match args.command {
        AccessCommand::ServiceAccount {
            command:
                ServiceAccountCommand::Token {
                    command: ServiceAccountTokenCommand::Delete(token_args),
                },
        } => {
            assert_eq!(token_args.name.as_deref(), Some("svc"));
            assert_eq!(token_args.token_name.as_deref(), Some("automation"));
            assert!(token_args.yes);
        }
        _ => panic!("expected service-account token delete"),
    }
}

#[test]
fn user_list_with_request_reads_org_users() {
    let args = UserListArgs {
        common: make_token_common(),
        scope: Scope::Org,
        query: None,
        login: None,
        email: None,
        org_role: None,
        grafana_admin: None,
        with_teams: false,
        page: 1,
        per_page: 100,
        table: false,
        csv: false,
        json: true,
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
fn user_add_with_request_requires_basic_auth_and_updates_role() {
    let args = UserAddArgs {
        common: make_basic_common(),
        login: "alice".to_string(),
        email: "alice@example.com".to_string(),
        name: "Alice".to_string(),
        new_user_password: "pw".to_string(),
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
fn user_delete_with_request_requires_yes_and_deletes() {
    let args = UserDeleteArgs {
        common: make_basic_common(),
        user_id: Some("9".to_string()),
        login: None,
        email: None,
        scope: Scope::Global,
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
        page: 1,
        per_page: 100,
        table: false,
        csv: false,
        json: true,
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
    assert!(calls
        .iter()
        .any(|(method, path, _, _)| method == "PUT" && path == "/api/teams/3/members"));
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
fn service_account_list_with_request_reads_search() {
    let args = ServiceAccountListArgs {
        common: make_token_common(),
        query: Some("svc".to_string()),
        page: 1,
        per_page: 100,
        table: false,
        csv: false,
        json: true,
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
fn run_access_cli_with_request_routes_user_list() {
    let args = parse_cli_from([
        "grafana-access-utils",
        "user",
        "list",
        "--json",
        "--token",
        "abc",
    ]);
    let result = run_access_cli_with_request(
        |_method, path, _params, _payload| match path {
            "/api/org/users" => Ok(Some(json!([]))),
            _ => panic!("unexpected path {path}"),
        },
        args,
    );
    assert!(result.is_ok());
}
