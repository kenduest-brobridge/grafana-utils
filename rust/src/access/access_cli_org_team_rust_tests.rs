use super::*;

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
fn parse_cli_supports_access_delete_prompt_flags() {
    let user_args = parse_cli_from(["grafana-util access", "user", "delete", "--prompt"]);
    match user_args.command {
        AccessCommand::User {
            command: UserCommand::Delete(inner),
        } => {
            assert!(inner.prompt);
            assert_eq!(inner.scope, None);
        }
        _ => panic!("expected access user delete"),
    }

    let org_args = parse_cli_from(["grafana-util access", "org", "delete", "--prompt"]);
    match org_args.command {
        AccessCommand::Org {
            command: OrgCommand::Delete(inner),
        } => assert!(inner.prompt),
        _ => panic!("expected access org delete"),
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
