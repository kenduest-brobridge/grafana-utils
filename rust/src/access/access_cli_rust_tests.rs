//! CLI definitions for Access command surface and option compatibility behavior.

use super::*;
use crate::access::{
    build_auth_context, ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
    ACCESS_EXPORT_KIND_TEAMS, ACCESS_EXPORT_KIND_USERS, ACCESS_ORG_EXPORT_FILENAME,
    ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME, ACCESS_TEAM_EXPORT_FILENAME,
    ACCESS_USER_EXPORT_FILENAME,
};

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
fn access_user_browse_help_hides_deprecated_with_teams_flag() {
    let help = render_access_subcommand_help(&["user", "browse"]);
    assert!(!help.contains("--with-teams"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("--current-org"));
    assert!(help.contains("--scope"));
    assert!(help.contains("--input-dir"));
}

#[test]
fn parse_cli_supports_team_browse() {
    let args = parse_cli_from(["grafana-util access", "team", "browse", "--with-members"]);

    match args.command {
        AccessCommand::Team {
            command: TeamCommand::Browse(browse_args),
        } => {
            assert!(browse_args.with_members);
        }
        _ => panic!("expected team browse"),
    }
}

#[test]
fn parse_cli_supports_team_browse_local_input_dir() {
    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "browse",
        "--input-dir",
        "/tmp/access-teams",
        "--name",
        "platform-team",
    ]);

    match args.command {
        AccessCommand::Team {
            command: TeamCommand::Browse(browse_args),
        } => {
            assert_eq!(
                browse_args
                    .input_dir
                    .as_ref()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref(),
                "/tmp/access-teams"
            );
            assert_eq!(browse_args.name.as_deref(), Some("platform-team"));
        }
        _ => panic!("expected team browse"),
    }
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

#[test]
fn access_root_help_includes_examples() {
    let mut command = AccessCliRoot::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("Examples:"));
    assert!(help.contains("grafana-util access user list"));
    assert!(help.contains("grafana-util access user list --input-dir ./access-users"));
    assert!(help.contains("grafana-util access user browse"));
    assert!(help.contains("grafana-util access team import"));
}

#[test]
fn user_add_help_includes_examples_and_grouped_auth_headings() {
    let help = render_access_subcommand_help(&["user", "add"]);
    assert!(help.contains("Examples:"));
    assert!(help.contains("Authentication Options"));
    assert!(help.contains("Transport Options"));
}

#[test]
fn team_import_help_includes_examples_and_yes_flag() {
    let help = render_access_subcommand_help(&["team", "import"]);
    assert!(help.contains("Examples:"));
    assert!(help.contains("--yes"));
    assert!(help.contains("Authentication Options"));
}

#[test]
fn org_delete_help_includes_examples_and_yes_flag() {
    let help = render_access_subcommand_help(&["org", "delete"]);
    assert!(help.contains("Examples:"));
    assert!(help.contains("--yes"));
}

#[test]
fn org_diff_help_includes_examples() {
    let help = render_access_subcommand_help(&["org", "diff"]);
    assert!(help.contains("Examples:"));
    assert!(help.contains("--diff-dir"));
}

#[test]
fn service_account_token_add_help_includes_examples() {
    let help = render_access_subcommand_help(&["service-account", "token", "add"]);
    assert!(help.contains("Examples:"));
    assert!(help.contains("--token-name"));
}

#[test]
fn access_root_help_includes_examples_and_grouped_options() {
    let help = render_access_root_help();
    let user_add_help = render_access_subcommand_help(&["user", "add"]);

    assert!(help.contains("Examples:"));
    assert!(help.contains("grafana-util access user list"));
    assert!(help.contains("grafana-util access service-account token add"));
    assert!(user_add_help.contains("Authentication Options"));
    assert!(user_add_help.contains("Transport Options"));
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
fn user_help_mentions_filter_and_output_flags() {
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
    assert!(delete_help.contains("Skip the interactive confirmation prompt"));
}

#[test]
fn team_and_service_account_help_mentions_membership_and_token_flags() {
    let org_help = render_access_subcommand_help(&["org", "list"]);
    assert!(org_help.contains("--with-users"));
    assert!(org_help.contains("Include org users and org roles"));
    assert!(org_help.contains("--input-dir"));
    assert!(org_help.contains("local"));
    assert!(org_help.contains("--output-format text"));
    assert!(org_help.contains("--output-format yaml"));

    let team_add_help = render_access_subcommand_help(&["team", "add"]);
    assert!(team_add_help.contains("--member"));
    assert!(team_add_help.contains("Add one or more members"));

    let team_help = render_access_subcommand_help(&["team", "modify"]);
    assert!(team_help.contains("--add-member"));
    assert!(team_help.contains("Add one or more members"));
    assert!(team_help.contains("--remove-admin"));
    assert!(team_help.contains("Remove team-admin status"));

    let service_account_help = render_access_subcommand_help(&["service-account", "add"]);
    assert!(service_account_help.contains("--role"));
    assert!(service_account_help.contains("Initial org role for the service account"));

    let service_account_list_help = render_access_subcommand_help(&["service-account", "list"]);
    assert!(service_account_list_help.contains("--input-dir"));
    assert!(service_account_list_help.contains("local"));
    assert!(service_account_list_help.contains("--output-format text"));
    assert!(service_account_list_help.contains("--output-format yaml"));

    let token_help = render_access_subcommand_help(&["service-account", "token", "add"]);
    assert!(token_help.contains("--token-name"));
    assert!(token_help.contains("Name for the new service-account token"));
    assert!(token_help.contains("--seconds-to-live"));
}

#[test]
fn parse_cli_supports_org_commands() {
    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "list",
        "--query",
        "main",
        "--with-users",
        "--output-format",
        "json",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::List(list_args),
        } => {
            assert_eq!(list_args.query.as_deref(), Some("main"));
            assert!(list_args.with_users);
            assert!(list_args.json);
            assert!(!list_args.yaml);
        }
        _ => panic!("expected org list"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "list",
        "--input-dir",
        "/tmp/access-orgs",
        "--output-format",
        "table",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::List(list_args),
        } => {
            assert_eq!(
                list_args.input_dir.as_ref().unwrap().to_string_lossy(),
                "/tmp/access-orgs"
            );
            assert!(list_args.table);
            assert!(!list_args.json);
            assert!(!list_args.csv);
            assert!(!list_args.yaml);
        }
        _ => panic!("expected org list"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "add",
        "--name",
        "Main Org",
        "--json",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::Add(add_args),
        } => {
            assert_eq!(add_args.name, "Main Org");
            assert!(add_args.json);
        }
        _ => panic!("expected org add"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "modify",
        "--org-id",
        "7",
        "--set-name",
        "Renamed Org",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::Modify(modify_args),
        } => {
            assert_eq!(modify_args.org_id, Some(7));
            assert_eq!(modify_args.set_name, "Renamed Org");
        }
        _ => panic!("expected org modify"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "export",
        "--with-users",
        "--output-dir",
        "/tmp/access-orgs",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::Export(export_args),
        } => {
            assert!(export_args.with_users);
            assert_eq!(export_args.output_dir.to_string_lossy(), "/tmp/access-orgs");
        }
        _ => panic!("expected org export"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "import",
        "--input-dir",
        "/tmp/access-orgs",
        "--replace-existing",
        "--dry-run",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::Import(import_args),
        } => {
            assert!(import_args.replace_existing);
            assert!(import_args.dry_run);
            assert_eq!(import_args.input_dir.to_string_lossy(), "/tmp/access-orgs");
        }
        _ => panic!("expected org import"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "diff",
        "--diff-dir",
        "/tmp/access-orgs",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::Diff(diff_args),
        } => {
            assert_eq!(diff_args.diff_dir.to_string_lossy(), "/tmp/access-orgs");
        }
        _ => panic!("expected org diff"),
    }
}

#[test]
fn parse_cli_supports_team_list_output_format_yaml() {
    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "list",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        AccessCommand::Team {
            command: TeamCommand::List(list_args),
        } => {
            assert!(list_args.yaml);
            assert!(!list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
        }
        _ => panic!("expected team list"),
    }
}

#[test]
fn parse_cli_supports_team_list_local_input_dir() {
    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "list",
        "--input-dir",
        "/tmp/access-teams",
        "--output-format",
        "csv",
    ]);

    match args.command {
        AccessCommand::Team {
            command: TeamCommand::List(list_args),
        } => {
            assert_eq!(
                list_args.input_dir.as_ref().unwrap().to_string_lossy(),
                "/tmp/access-teams"
            );
            assert!(list_args.csv);
            assert!(!list_args.table);
            assert!(!list_args.json);
            assert!(!list_args.yaml);
        }
        _ => panic!("expected team list"),
    }
}

#[test]
fn parse_cli_supports_org_list_output_format_yaml() {
    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "list",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        AccessCommand::Org {
            command: OrgCommand::List(list_args),
        } => {
            assert!(list_args.yaml);
            assert!(!list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
        }
        _ => panic!("expected org list"),
    }
}

#[test]
fn parse_cli_supports_org_list_local_input_dir() {
    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "list",
        "--input-dir",
        "/tmp/access-orgs",
        "--output-format",
        "json",
    ]);

    match args.command {
        AccessCommand::Org {
            command: OrgCommand::List(list_args),
        } => {
            assert_eq!(
                list_args.input_dir.as_ref().unwrap().to_string_lossy(),
                "/tmp/access-orgs"
            );
            assert!(list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
            assert!(!list_args.yaml);
        }
        _ => panic!("expected org list"),
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
fn parse_cli_supports_group_delete_alias() {
    let args = parse_cli_from([
        "grafana-util access",
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
fn parse_cli_supports_insecure_on_destructive_commands() {
    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "delete",
        "--name",
        "ops",
        "--insecure",
        "--yes",
    ]);
    match args.command {
        AccessCommand::Team {
            command: TeamCommand::Delete(delete_args),
        } => {
            assert!(delete_args.common.insecure);
            assert_eq!(delete_args.name.as_deref(), Some("ops"));
            assert!(delete_args.yes);
        }
        _ => panic!("expected team delete"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "delete",
        "--name",
        "ops",
        "--insecure",
        "--yes",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::Delete(delete_args),
        } => {
            assert!(delete_args.common.insecure);
            assert_eq!(delete_args.name.as_deref(), Some("ops"));
            assert!(delete_args.yes);
        }
        _ => panic!("expected org delete"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "service-account",
        "delete",
        "--name",
        "svc",
        "--insecure",
        "--yes",
    ]);
    match args.command {
        AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::Delete(delete_args),
        } => {
            assert!(delete_args.common.insecure);
            assert_eq!(delete_args.name.as_deref(), Some("svc"));
            assert!(delete_args.yes);
        }
        _ => panic!("expected service-account delete"),
    }

    let args = parse_cli_from([
        "grafana-util access",
        "service-account",
        "token",
        "delete",
        "--name",
        "svc",
        "--token-name",
        "automation",
        "--insecure",
        "--yes",
    ]);
    match args.command {
        AccessCommand::ServiceAccount {
            command:
                ServiceAccountCommand::Token {
                    command: ServiceAccountTokenCommand::Delete(delete_args),
                },
        } => {
            assert!(delete_args.common.insecure);
            assert_eq!(delete_args.name.as_deref(), Some("svc"));
            assert_eq!(delete_args.token_name.as_deref(), Some("automation"));
            assert!(delete_args.yes);
        }
        _ => panic!("expected service-account token delete"),
    }
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

    assert_eq!(user_table_rows(&rows)[0][7], "global-shared");
    assert!(user_summary_line(&rows[0]).contains("accountScope=global-shared"));
}

#[test]
fn parse_cli_supports_user_export_and_import() {
    let export_args = parse_cli_from([
        "grafana-util access",
        "user",
        "export",
        "--scope",
        "global",
        "--with-teams",
        "--dry-run",
        "--overwrite",
        "--output-dir",
        "/tmp/access-users",
    ]);
    match export_args.command {
        AccessCommand::User {
            command: UserCommand::Export(args),
        } => {
            assert_eq!(args.scope, Scope::Global);
            assert!(args.with_teams);
            assert!(args.dry_run);
            assert!(args.overwrite);
        }
        _ => panic!("expected user export"),
    }

    let import_args = parse_cli_from([
        "grafana-util access",
        "user",
        "import",
        "--scope",
        "global",
        "--replace-existing",
        "--dry-run",
        "--input-dir",
        "/tmp/access-users",
        "--yes",
    ]);
    match import_args.command {
        AccessCommand::User {
            command: UserCommand::Import(args),
        } => {
            assert_eq!(args.scope, Scope::Global);
            assert!(args.replace_existing);
            assert!(args.dry_run);
            assert!(args.yes);
        }
        _ => panic!("expected user import"),
    }
}

#[test]
fn parse_cli_supports_user_diff() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "diff",
        "--scope",
        "global",
        "--diff-dir",
        "/tmp/access-users",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::Diff(args),
        } => {
            assert_eq!(args.scope, Scope::Global);
            assert_eq!(
                args.diff_dir.to_string_lossy().as_ref(),
                "/tmp/access-users"
            );
        }
        _ => panic!("expected user diff"),
    }
}

#[test]
fn parse_cli_supports_team_diff() {
    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "diff",
        "--diff-dir",
        "/tmp/access-teams",
    ]);

    match args.command {
        AccessCommand::Team {
            command: TeamCommand::Diff(args),
        } => {
            assert_eq!(
                args.diff_dir.to_string_lossy().as_ref(),
                "/tmp/access-teams"
            );
        }
        _ => panic!("expected team diff"),
    }
}

#[test]
fn parse_cli_supports_team_export_and_import() {
    let export_args = parse_cli_from([
        "grafana-util access",
        "team",
        "export",
        "--with-members",
        "--dry-run",
        "--output-dir",
        "/tmp/access-teams",
    ]);
    match export_args.command {
        AccessCommand::Team {
            command: TeamCommand::Export(args),
        } => {
            assert!(args.with_members);
            assert!(args.dry_run);
        }
        _ => panic!("expected team export"),
    }

    let import_args = parse_cli_from([
        "grafana-util access",
        "team",
        "import",
        "--replace-existing",
        "--dry-run",
        "--input-dir",
        "/tmp/access-teams",
        "--yes",
    ]);
    match import_args.command {
        AccessCommand::Team {
            command: TeamCommand::Import(args),
        } => {
            assert!(args.replace_existing);
            assert!(args.dry_run);
            assert!(args.yes);
        }
        _ => panic!("expected team import"),
    }
}

#[test]
fn parse_cli_supports_user_import_dry_run_output_format_table() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "import",
        "--input-dir",
        "/tmp/access-users",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::Import(args),
        } => {
            assert!(args.dry_run);
            assert!(args.table);
            assert!(!args.json);
            assert_eq!(args.output_format, DryRunOutputFormat::Table);
        }
        _ => panic!("expected user import"),
    }
}

#[test]
fn parse_cli_supports_user_import_dry_run_output_format_json() {
    let args = parse_cli_from([
        "grafana-util access",
        "user",
        "import",
        "--input-dir",
        "/tmp/access-users",
        "--dry-run",
        "--output-format",
        "json",
    ]);

    match args.command {
        AccessCommand::User {
            command: UserCommand::Import(args),
        } => {
            assert!(args.dry_run);
            assert!(args.json);
            assert!(!args.table);
            assert_eq!(args.output_format, DryRunOutputFormat::Json);
        }
        _ => panic!("expected user import"),
    }
}

#[test]
fn parse_cli_supports_team_import_dry_run_output_format_table() {
    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "import",
        "--input-dir",
        "/tmp/access-teams",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        AccessCommand::Team {
            command: TeamCommand::Import(args),
        } => {
            assert!(args.dry_run);
            assert!(args.table);
            assert!(!args.json);
            assert_eq!(args.output_format, DryRunOutputFormat::Table);
        }
        _ => panic!("expected team import"),
    }
}

#[test]
fn parse_cli_supports_team_import_dry_run_output_format_json() {
    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "import",
        "--input-dir",
        "/tmp/access-teams",
        "--dry-run",
        "--output-format",
        "json",
    ]);

    match args.command {
        AccessCommand::Team {
            command: TeamCommand::Import(args),
        } => {
            assert!(args.dry_run);
            assert!(args.json);
            assert!(!args.table);
            assert_eq!(args.output_format, DryRunOutputFormat::Json);
        }
        _ => panic!("expected team import"),
    }
}
