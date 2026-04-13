use super::*;

#[test]
fn parse_cli_supports_user_list() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "list",
        "--all-orgs",
        "--table",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert_eq!(list_args.scope, Scope::Global);
            assert!(list_args.all_orgs);
            assert!(list_args.table);
            assert!(!list_args.csv);
            assert!(!list_args.json);
            assert!(!list_args.yaml);
            assert!(list_args.output_columns.is_empty());
            assert!(!list_args.list_columns);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_infers_unique_long_option_prefixes() {
    let args = parse_cli_from(["grafana-util access", "user", "list", "--all-o", "--tab"]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert_eq!(list_args.scope, Scope::Global);
            assert!(list_args.all_orgs);
            assert!(list_args.table);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_rejects_ambiguous_long_option_prefixes() {
    let error =
        AccessCliRoot::try_parse_from(["grafana-util access", "user", "list", "--output", "json"])
            .unwrap_err();

    let rendered = error.to_string();
    assert!(rendered.contains("--output"));
    assert!(
        rendered.contains("--output-format")
            || rendered.contains("--output-columns")
            || rendered.contains("possible values")
    );
}

#[test]
fn parse_cli_supports_user_list_output_columns_all_and_list_columns() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "list",
        "--output-columns",
        "all",
        "--list-columns",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert_eq!(list_args.output_columns, vec!["all".to_string()]);
            assert!(list_args.list_columns);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_user_browse() {
    let args = parse_cli_from(["grafana-util access", "user", "browse"]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::Browse(browse_args),
        } => {
            assert_eq!(browse_args.scope, Scope::Global);
            assert!(!browse_args.all_orgs);
            assert!(!browse_args.current_org);
            assert!(!browse_args.with_teams);
        }
        _ => panic!("expected user browse"),
    }
}

#[test]
fn parse_cli_supports_user_browse_local_input_dir() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "browse",
        "--input-dir",
        "/tmp/access-users",
        "--login",
        "alice",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::Browse(browse_args),
        } => {
            assert_eq!(
                browse_args
                    .input_dir
                    .as_ref()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref(),
                "/tmp/access-users"
            );
            assert_eq!(browse_args.login.as_deref(), Some("alice"));
        }
        _ => panic!("expected user browse"),
    }
}

#[test]
fn parse_cli_supports_user_browse_current_org() {
    let args = parse_cli_from(["grafana-util access", "user", "browse", "--current-org"]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::Browse(browse_args),
        } => {
            assert_eq!(browse_args.scope, Scope::Org);
            assert!(browse_args.current_org);
            assert!(!browse_args.all_orgs);
        }
        _ => panic!("expected user browse"),
    }
}

#[test]
fn parse_cli_supports_safer_user_password_inputs() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "add",
        "--login",
        "alice",
        "--email",
        "alice@example.com",
        "--name",
        "Alice",
        "--password-file",
        "/tmp/new-user-password.txt",
    ]);
    match args.command {
        AccessCommand::User {
            command: UserCommand::Add(add_args),
        } => {
            assert_eq!(
                add_args
                    .new_user_password_file
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                Some("/tmp/new-user-password.txt".to_string())
            );
            assert!(add_args.new_user_password.is_none());
            assert!(!add_args.prompt_user_password);
        }
        _ => panic!("expected user add"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "modify",
        "--login",
        "alice",
        "--prompt-set-password",
    ]);
    match args.command {
        AccessCommand::User {
            command: UserCommand::Modify(modify_args),
        } => {
            assert!(modify_args.prompt_set_password);
            assert!(modify_args.set_password.is_none());
            assert!(modify_args.set_password_file.is_none());
        }
        _ => panic!("expected user modify"),
    }
}

#[test]
fn user_list_mentions_filter_and_output_flags() {
    let help = render_access_subcommand_help(&["user", "list"]);
    assert!(help.contains("--scope"));
    assert!(help.contains("current org scope"));
    assert!(help.contains("--input-dir"));
    assert!(help.contains("local"));
    assert!(help.contains("--with-teams"));
    assert!(help.contains("Include each user's current team memberships"));
    assert!(help.contains("--output-format text"));
    assert!(help.contains("--output-format yaml"));
}

#[test]
fn user_mutation_help_mentions_target_and_json_flags() {
    let add_help = render_access_subcommand_help(&["user", "add"]);
    assert!(add_help.contains("--login"));
    assert!(add_help.contains("Login name for the new Grafana user"));
    assert!(add_help.contains("--grafana-admin"));
    assert!(add_help.contains("server admin"));
    assert!(add_help.contains("--password"));
    assert!(add_help.contains("Initial password for the new Grafana user"));
    assert!(add_help.contains("--password-file"));
    assert!(add_help.contains("--prompt-user-password"));

    let modify_help = render_access_subcommand_help(&["user", "modify"]);
    assert!(modify_help.contains("--user-id"));
    assert!(modify_help.contains("Target one user by numeric Grafana user id"));
    assert!(modify_help.contains("--set-password"));
    assert!(modify_help.contains("Replace the user's password"));
    assert!(modify_help.contains("--set-password-file"));
    assert!(modify_help.contains("--prompt-set-password"));

    let delete_help = render_access_subcommand_help(&["user", "delete"]);
    assert!(delete_help.contains("--yes"));
    assert!(delete_help.contains("Skip the terminal confirmation prompt"));
    assert!(delete_help.contains("--prompt"));
}

#[test]
fn parse_cli_supports_prompt_token() {
    let args = parse_cli_from(["grafana-util access", "user", "list", "--prompt-token"]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert!(list_args.common.prompt_token);
            assert_eq!(list_args.common.api_token.as_deref(), None);
            assert!(!list_args.common.prompt_password);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_insecure_and_ca_cert_flags() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "list",
        "--ca-cert",
        "/tmp/grafana-ca.pem",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert_eq!(
                list_args.common.ca_cert.as_deref(),
                Some(std::path::Path::new("/tmp/grafana-ca.pem"))
            );
            assert!(!list_args.common.verify_ssl);
            assert!(!list_args.common.insecure);
        }
        _ => panic!("expected user list"),
    }

    let args = parse_cli_from(["grafana-util access", "user", "list", "--insecure"]);
    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert!(list_args.common.insecure);
            assert_eq!(list_args.common.ca_cert, None);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_user_list_output_format_json() {
    let args = parse_cli_from([
        "grafana-util access",
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
            assert!(!list_args.yaml);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_user_list_local_input_dir() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "list",
        "--input-dir",
        "/tmp/access-users",
        "--output-format",
        "table",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert_eq!(
                list_args.input_dir.as_ref().unwrap().to_string_lossy(),
                "/tmp/access-users"
            );
            assert!(list_args.table);
            assert!(!list_args.json);
            assert!(!list_args.csv);
            assert!(!list_args.yaml);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_user_list_output_format_text() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "list",
        "--output-format",
        "text",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert!(!list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
            assert!(!list_args.yaml);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_user_list_output_format_yaml() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "list",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::List(list_args),
        } => {
            assert!(list_args.yaml);
            assert!(!list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
        }
        _ => panic!("expected user list"),
    }
}

#[test]
fn parse_cli_supports_service_account_token_add() {
    let args = parse_cli_from([
        "grafana-util access",
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
fn parse_cli_supports_service_account_export_import_and_diff() {
    let export_args = parse_cli_from([
        "grafana-util access",
        "service-account",
        "export",
        "--output-dir",
        "/tmp/access-service-accounts",
        "--overwrite",
        "--dry-run",
    ]);
    match export_args.command {
        AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::Export(args),
        } => {
            assert_eq!(
                args.output_dir.to_string_lossy().as_ref(),
                "/tmp/access-service-accounts"
            );
            assert!(args.overwrite);
            assert!(args.dry_run);
        }
        _ => panic!("expected service-account export"),
    }

    let import_args = parse_cli_from([
        "grafana-util access",
        "service-account",
        "import",
        "--input-dir",
        "/tmp/access-service-accounts",
        "--replace-existing",
        "--dry-run",
        "--output-format",
        "table",
    ]);
    match import_args.command {
        AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::Import(args),
        } => {
            assert_eq!(
                args.input_dir.to_string_lossy().as_ref(),
                "/tmp/access-service-accounts"
            );
            assert!(args.replace_existing);
            assert!(args.dry_run);
            assert!(args.table);
        }
        _ => panic!("expected service-account import"),
    }

    let diff_args = parse_cli_from([
        "grafana-util access",
        "service-account",
        "diff",
        "--diff-dir",
        "/tmp/access-service-accounts",
    ]);
    match diff_args.command {
        AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::Diff(args),
        } => {
            assert_eq!(
                args.diff_dir.to_string_lossy().as_ref(),
                "/tmp/access-service-accounts"
            );
        }
        _ => panic!("expected service-account diff"),
    }
}

#[test]
fn parse_cli_supports_service_account_list_output_format_yaml() {
    let args = parse_cli_from([
        "grafana-util access",
        "service-account",
        "list",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::List(list_args),
        } => {
            assert!(list_args.yaml);
            assert!(!list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
        }
        _ => panic!("expected service-account list"),
    }
}

#[test]
fn parse_cli_supports_service_account_list_local_input_dir() {
    let args = parse_cli_from([
        "grafana-util access",
        "service-account",
        "list",
        "--input-dir",
        "/tmp/access-service-accounts",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::List(list_args),
        } => {
            assert_eq!(
                list_args.input_dir.as_ref().unwrap().to_string_lossy(),
                "/tmp/access-service-accounts"
            );
            assert!(list_args.yaml);
            assert!(!list_args.table);
            assert!(!list_args.csv);
            assert!(!list_args.json);
        }
        _ => panic!("expected service-account list"),
    }
}

#[test]
fn parse_cli_supports_service_account_token_delete() {
    let args = parse_cli_from([
        "grafana-util access",
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
fn parse_cli_rejects_invalid_service_account_role() {
    let error = AccessCliRoot::try_parse_from([
        "grafana-util access",
        "service-account",
        "add",
        "--name",
        "svc",
        "--role",
        "Owner",
    ])
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("valid values: Viewer, Editor, Admin, None"));
}

#[test]
fn parse_cli_rejects_non_positive_service_account_token_ttl() {
    let error = AccessCliRoot::try_parse_from([
        "grafana-util access",
        "service-account",
        "token",
        "add",
        "--service-account-id",
        "4",
        "--token-name",
        "automation",
        "--seconds-to-live",
        "0",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("value must be >= 1"));
}

#[test]
fn access_bundle_contract_fixture_matches_access_constants() {
    let cases = load_access_bundle_contract_cases();
    assert_eq!(cases.len(), 4);

    for case in cases {
        let domain = case.get("domain").and_then(Value::as_str).unwrap_or("");
        let bundle_file = case.get("bundleFile").and_then(Value::as_str).unwrap_or("");
        let expected_kind = case
            .get("expectedKind")
            .and_then(Value::as_str)
            .unwrap_or("");
        let supports_source_metadata = case
            .get("supportsSourceMetadata")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        match domain {
            "user" => {
                assert_eq!(bundle_file, ACCESS_USER_EXPORT_FILENAME);
                assert_eq!(expected_kind, ACCESS_EXPORT_KIND_USERS);
            }
            "team" => {
                assert_eq!(bundle_file, ACCESS_TEAM_EXPORT_FILENAME);
                assert_eq!(expected_kind, ACCESS_EXPORT_KIND_TEAMS);
            }
            "org" => {
                assert_eq!(bundle_file, ACCESS_ORG_EXPORT_FILENAME);
                assert_eq!(expected_kind, ACCESS_EXPORT_KIND_ORGS);
            }
            "service-account" => {
                assert_eq!(bundle_file, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME);
                assert_eq!(expected_kind, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS);
            }
            other => panic!("unexpected access bundle contract fixture domain {other}"),
        }

        assert!(supports_source_metadata);
    }
}
