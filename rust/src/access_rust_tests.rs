// Access domain test suite.
// Validates CLI parsing/help text surfaces and handler contract behavior with stubbed request closures.
use super::{
    add_service_account_token_with_request, add_service_account_with_request,
    add_team_with_request, add_user_with_request, delete_org_with_request,
    delete_service_account_token_with_request, delete_service_account_with_request,
    delete_team_with_request, delete_user_with_request, diff_service_accounts_with_request,
    diff_teams_with_request, diff_users_with_request, export_service_accounts_with_request,
    import_service_accounts_with_request, import_teams_with_request, list_orgs_with_request,
    list_service_accounts_command_with_request, list_teams_command_with_request,
    list_users_with_request, modify_org_with_request, modify_team_with_request,
    modify_user_with_request, parse_cli_from, run_access_cli_with_request, AccessCommand,
    CommonCliArgs, DryRunOutputFormat, OrgCommand, OrgDeleteArgs, OrgListArgs, OrgModifyArgs,
    Scope, ServiceAccountAddArgs, ServiceAccountCommand, ServiceAccountDeleteArgs,
    ServiceAccountDiffArgs, ServiceAccountExportArgs, ServiceAccountImportArgs,
    ServiceAccountListArgs, ServiceAccountTokenAddArgs, ServiceAccountTokenCommand,
    ServiceAccountTokenDeleteArgs, TeamAddArgs, TeamCommand, TeamDeleteArgs, TeamDiffArgs,
    TeamImportArgs, TeamListArgs, TeamModifyArgs, UserAddArgs, UserCommand, UserDeleteArgs,
    UserDiffArgs, UserListArgs, UserModifyArgs,
};
use crate::access::access_cli_defs::AccessCliRoot;
use crate::access::access_cli_defs::CommonCliArgsNoOrgId;
use clap::{CommandFactory, Parser};
use reqwest::Method;
use serde_json::json;
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

fn render_access_root_help() -> String {
    let mut command = AccessCliRoot::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn make_token_common() -> CommonCliArgs {
    CommonCliArgs {
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
fn access_root_help_includes_examples() {
    let mut command = AccessCliRoot::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("Examples:"));
    assert!(help.contains("grafana-util access user list"));
    assert!(help.contains("grafana-util access team import"));
}

#[test]
fn access_user_add_help_includes_examples_and_grouped_auth_headings() {
    let help = render_access_subcommand_help(&["user", "add"]);
    assert!(help.contains("Examples:"));
    assert!(help.contains("Authentication Options"));
    assert!(help.contains("Transport Options"));
}

#[test]
fn access_team_import_help_includes_examples_and_yes_flag() {
    let help = render_access_subcommand_help(&["team", "import"]);
    assert!(help.contains("Examples:"));
    assert!(help.contains("--yes"));
    assert!(help.contains("Authentication Options"));
}

#[test]
fn access_org_delete_help_includes_examples_and_yes_flag() {
    let help = render_access_subcommand_help(&["org", "delete"]);
    assert!(help.contains("Examples:"));
    assert!(help.contains("--yes"));
}

#[test]
fn access_service_account_token_add_help_includes_examples() {
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
        "grafana-access-utils",
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
        "grafana-access-utils",
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
    assert!(help.contains("--with-teams"));
    assert!(help.contains("Include each user's current team memberships"));
    assert!(help.contains("--output-format"));
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

    let token_help = render_access_subcommand_help(&["service-account", "token", "add"]);
    assert!(token_help.contains("--token-name"));
    assert!(token_help.contains("Name for the new service-account token"));
    assert!(token_help.contains("--seconds-to-live"));
}

#[test]
fn parse_cli_supports_org_commands() {
    let args = parse_cli_from([
        "grafana-access-utils",
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
        }
        _ => panic!("expected org list"),
    }

    let args = parse_cli_from([
        "grafana-access-utils",
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
        "grafana-access-utils",
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
        "grafana-access-utils",
        "org",
        "export",
        "--with-users",
        "--export-dir",
        "/tmp/access-orgs",
    ]);
    match args.command {
        AccessCommand::Org {
            command: OrgCommand::Export(export_args),
        } => {
            assert!(export_args.with_users);
            assert_eq!(export_args.export_dir.to_string_lossy(), "/tmp/access-orgs");
        }
        _ => panic!("expected org export"),
    }

    let args = parse_cli_from([
        "grafana-access-utils",
        "org",
        "import",
        "--import-dir",
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
            assert_eq!(import_args.import_dir.to_string_lossy(), "/tmp/access-orgs");
        }
        _ => panic!("expected org import"),
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
fn parse_cli_supports_user_list_output_format_text() {
    let args = parse_cli_from([
        "grafana-access-utils",
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
fn parse_cli_supports_service_account_export_import_and_diff() {
    let export_args = parse_cli_from([
        "grafana-access-utils",
        "service-account",
        "export",
        "--export-dir",
        "/tmp/access-service-accounts",
        "--overwrite",
        "--dry-run",
    ]);
    match export_args.command {
        AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::Export(args),
        } => {
            assert_eq!(
                args.export_dir.to_string_lossy().as_ref(),
                "/tmp/access-service-accounts"
            );
            assert!(args.overwrite);
            assert!(args.dry_run);
        }
        _ => panic!("expected service-account export"),
    }

    let import_args = parse_cli_from([
        "grafana-access-utils",
        "service-account",
        "import",
        "--import-dir",
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
                args.import_dir.to_string_lossy().as_ref(),
                "/tmp/access-service-accounts"
            );
            assert!(args.replace_existing);
            assert!(args.dry_run);
            assert!(args.table);
        }
        _ => panic!("expected service-account import"),
    }

    let diff_args = parse_cli_from([
        "grafana-access-utils",
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
fn parse_cli_rejects_invalid_service_account_role() {
    let error = AccessCliRoot::try_parse_from([
        "grafana-access-utils",
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
        "grafana-access-utils",
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
    let args = parse_cli_from(["grafana-access-utils", "user", "list", "--prompt-token"]);

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
        "grafana-access-utils",
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

    let args = parse_cli_from(["grafana-access-utils", "user", "list", "--insecure"]);
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
        "grafana-access-utils",
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
        "grafana-access-utils",
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
        "grafana-access-utils",
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
        "grafana-access-utils",
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

    let context = super::build_auth_context(&common).unwrap();

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
fn parse_cli_supports_user_export_and_import() {
    let export_args = parse_cli_from([
        "grafana-access-utils",
        "user",
        "export",
        "--scope",
        "global",
        "--with-teams",
        "--dry-run",
        "--overwrite",
        "--export-dir",
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
        "grafana-access-utils",
        "user",
        "import",
        "--scope",
        "global",
        "--replace-existing",
        "--dry-run",
        "--import-dir",
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
        "grafana-access-utils",
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
        "grafana-access-utils",
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
        "grafana-access-utils",
        "team",
        "export",
        "--with-members",
        "--dry-run",
        "--export-dir",
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
        "grafana-access-utils",
        "team",
        "import",
        "--replace-existing",
        "--dry-run",
        "--import-dir",
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
        "grafana-access-utils",
        "user",
        "import",
        "--import-dir",
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
        "grafana-access-utils",
        "user",
        "import",
        "--import-dir",
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
        "grafana-access-utils",
        "team",
        "import",
        "--import-dir",
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
        "grafana-access-utils",
        "team",
        "import",
        "--import-dir",
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

#[test]
fn run_access_cli_with_request_routes_user_export() {
    let args = parse_cli_from([
        "grafana-access-utils",
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
        args,
    );
    assert!(result.is_ok());
}

#[test]
fn run_access_cli_with_request_routes_team_export() {
    let args = parse_cli_from(["grafana-access-utils", "team", "export", "--dry-run"]);
    let result = run_access_cli_with_request(
        |method, path, _params, _payload| {
            assert_eq!(method.to_string(), Method::GET.to_string());
            if path == "/api/teams/search" {
                Ok(Some(json!({"teams": []})))
            } else {
                panic!("unexpected path {path}");
            }
        },
        args,
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
        "grafana-access-utils",
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
        args,
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
        "grafana-access-utils",
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
        args,
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
        "grafana-access-utils",
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
        args,
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
        "grafana-access-utils",
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
        args,
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
        "grafana-access-utils",
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
        args,
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
fn team_import_with_request_creates_team_and_memberships() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("access-teams");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("teams.json"),
        r#"[{"name":"Ops","email":"ops@example.com","members":["alice@example.com"],"admins":["bob@example.com"]}]"#,
    )
    .unwrap();
    let args = TeamImportArgs {
        common: make_token_common(),
        import_dir: import_dir.clone(),
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
    let import_dir = temp.path().join("access-teams");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("teams.json"),
        r#"[{"name":"Ops","members":["alice@example.com"]}]"#,
    )
    .unwrap();
    let args = TeamImportArgs {
        common: make_token_common(),
        import_dir: import_dir.clone(),
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
    let import_dir = temp.path().join("access-teams");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("teams.json"),
        r#"[{"name":"Ops","members":["alice@example.com","bob@example.com"]}]"#,
    )
    .unwrap();
    let args = TeamImportArgs {
        common: make_token_common(),
        import_dir: import_dir.clone(),
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
    let bundle = fs::read_to_string(temp_dir.path().join("service-accounts.json")).unwrap();
    assert!(bundle.contains("\"grafana-utils-access-service-account-export-index\""));
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
        json: true,
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
