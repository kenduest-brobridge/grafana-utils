use super::*;

fn assert_contains_all(rendered: &str, expected: &[&str]) {
    for needle in expected {
        assert!(
            rendered.contains(needle),
            "expected help to contain {needle:?}\n{rendered}"
        );
    }
}

fn assert_lacks_all(rendered: &str, rejected: &[&str]) {
    for needle in rejected {
        assert!(
            !rendered.contains(needle),
            "expected help to omit {needle:?}\n{rendered}"
        );
    }
}

#[test]
fn access_delete_help_mentions_prompt() {
    assert!(render_access_subcommand_help(&["user", "delete"]).contains("--prompt"));
    assert!(render_access_subcommand_help(&["team", "delete"]).contains("--prompt"));
    assert!(render_access_subcommand_help(&["org", "delete"]).contains("--prompt"));
    assert!(render_access_subcommand_help(&["service-account", "delete"]).contains("--prompt"));
    assert!(
        render_access_subcommand_help(&["service-account", "token", "delete"]).contains("--prompt")
    );
}

#[test]
fn access_user_browse_help_hides_deprecated_with_teams_flag() {
    let help = render_access_subcommand_help(&["user", "browse"]);
    assert_lacks_all(&help, &["--with-teams"]);
    assert_contains_all(
        &help,
        &["--all-orgs", "--current-org", "--scope", "--input-dir"],
    );
}

#[test]
fn access_root_help_includes_examples() {
    let mut command = AccessCliRoot::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert_contains_all(
        &help,
        &[
            "Examples:",
            "grafana-util access user list",
            "grafana-util access user list --input-dir ./access-users",
            "grafana-util access user browse",
            "grafana-util access team import",
        ],
    );
}

#[test]
fn user_add_help_includes_examples_and_grouped_auth_headings() {
    let help = render_access_subcommand_help(&["user", "add"]);
    assert_contains_all(
        &help,
        &["Examples:", "Authentication Options", "Transport Options"],
    );
}

#[test]
fn team_import_help_includes_examples_and_yes_flag() {
    let help = render_access_subcommand_help(&["team", "import"]);
    assert_contains_all(&help, &["Examples:", "--yes", "Authentication Options"]);
}

#[test]
fn org_delete_help_includes_examples_and_yes_flag() {
    let help = render_access_subcommand_help(&["org", "delete"]);
    assert_contains_all(&help, &["Examples:", "--yes"]);
}

#[test]
fn org_diff_help_includes_examples() {
    let help = render_access_subcommand_help(&["org", "diff"]);
    assert_contains_all(&help, &["Examples:", "--diff-dir"]);
}

#[test]
fn service_account_token_add_help_includes_examples() {
    let help = render_access_subcommand_help(&["service-account", "token", "add"]);
    assert_contains_all(&help, &["Examples:", "--token-name"]);
}

#[test]
fn access_root_help_includes_examples_and_grouped_options() {
    let help = render_access_root_help();
    let user_add_help = render_access_subcommand_help(&["user", "add"]);

    assert_contains_all(
        &help,
        &[
            "Examples:",
            "grafana-util access user list",
            "grafana-util access service-account token add",
        ],
    );
    assert_lacks_all(
        &help,
        &[
            "Enum definition for UserCommand",
            "Enum definition for OrgCommand",
            "Enum definition for TeamCommand",
            "Enum definition for ServiceAccountCommand",
        ],
    );
    assert_contains_all(
        &user_add_help,
        &["Authentication Options", "Transport Options"],
    );
}

#[test]
fn user_help_mentions_filter_and_output_flags() {
    let help = render_access_subcommand_help(&["user", "list"]);
    assert_contains_all(
        &help,
        &[
            "--scope",
            "current org scope",
            "--input-dir",
            "local",
            "--with-teams",
            "Include each user's current team memberships",
            "--output-format text",
            "--output-format yaml",
        ],
    );
}

#[test]
fn user_mutation_help_mentions_target_and_json_flags() {
    let add_help = render_access_subcommand_help(&["user", "add"]);
    assert_contains_all(
        &add_help,
        &[
            "--login",
            "Login name for the new Grafana user",
            "--grafana-admin",
            "server admin",
            "--password",
            "Initial password for the new Grafana user",
            "--password-file",
            "--prompt-user-password",
        ],
    );

    let modify_help = render_access_subcommand_help(&["user", "modify"]);
    assert_contains_all(
        &modify_help,
        &[
            "--user-id",
            "Target one user by numeric Grafana user id",
            "--set-password",
            "Replace the user's password",
            "--set-password-file",
            "--prompt-set-password",
        ],
    );

    let delete_help = render_access_subcommand_help(&["user", "delete"]);
    assert_contains_all(
        &delete_help,
        &["--yes", "Skip the terminal confirmation prompt", "--prompt"],
    );
}

#[test]
fn team_and_service_account_help_mentions_membership_and_token_flags() {
    let org_help = render_access_subcommand_help(&["org", "list"]);
    assert_contains_all(
        &org_help,
        &[
            "--with-users",
            "Include org users and org roles",
            "--input-dir",
            "local",
            "--output-format text",
            "--output-format yaml",
        ],
    );

    let team_add_help = render_access_subcommand_help(&["team", "add"]);
    assert_contains_all(&team_add_help, &["--member", "Add one or more members"]);

    let team_help = render_access_subcommand_help(&["team", "modify"]);
    assert_contains_all(
        &team_help,
        &[
            "--add-member",
            "Add one or more members",
            "--remove-admin",
            "Remove team-admin status",
        ],
    );

    let service_account_help = render_access_subcommand_help(&["service-account", "add"]);
    assert_contains_all(
        &service_account_help,
        &["--role", "Initial org role for the service account"],
    );

    let service_account_list_help = render_access_subcommand_help(&["service-account", "list"]);
    assert_contains_all(
        &service_account_list_help,
        &[
            "--input-dir",
            "local",
            "--output-format text",
            "--output-format yaml",
        ],
    );

    let token_help = render_access_subcommand_help(&["service-account", "token", "add"]);
    assert_contains_all(
        &token_help,
        &[
            "--token-name",
            "Name for the new service-account token",
            "--seconds-to-live",
        ],
    );
}
