use super::{
    parse_cli_from, render_alert_help, render_alert_subcommand_help, root_command, AlertCliArgs,
    AlertListKind,
};
use crate::common::DiffOutputFormat;
use std::path::Path;

#[test]
fn parse_cli_supports_diff_dir_and_dry_run() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "diff",
        "--url",
        "https://grafana.example.com",
        "--diff-dir",
        "./alerts/raw",
    ]);
    assert_eq!(args.url, "https://grafana.example.com");
    assert_eq!(args.diff_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.input_dir.is_none());
    assert!(!args.dry_run);
}

#[test]
fn parse_cli_supports_diff_json() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "diff",
        "--diff-dir",
        "./alerts/raw",
        "--json",
    ]);
    assert_eq!(args.diff_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.json);
    assert_eq!(args.diff_output, Some(DiffOutputFormat::Json));
}

#[test]
fn parse_cli_supports_diff_output_format_json() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-util alert",
        "diff",
        "--diff-dir",
        "./alerts/raw",
        "--output-format",
        "json",
    ]);
    assert_eq!(args.diff_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.json);
    assert_eq!(args.diff_output, Some(DiffOutputFormat::Json));
}

#[test]
fn parse_cli_supports_preferred_auth_aliases() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "--token",
        "abc123",
        "--basic-user",
        "user",
        "--basic-password",
        "pass",
    ]);
    assert_eq!(args.api_token.as_deref(), Some("abc123"));
    assert_eq!(args.username.as_deref(), Some("user"));
    assert_eq!(args.password.as_deref(), Some("pass"));
    assert!(!args.prompt_password);
}

#[test]
fn parse_cli_supports_prompt_password() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "--basic-user",
        "user",
        "--prompt-password",
    ]);
    assert_eq!(args.username.as_deref(), Some("user"));
    assert_eq!(args.password.as_deref(), None);
    assert!(args.prompt_password);
}

#[test]
fn parse_cli_supports_prompt_token() {
    let args: AlertCliArgs = parse_cli_from(["grafana-alert-utils", "--prompt-token"]);
    assert_eq!(args.api_token.as_deref(), None);
    assert!(args.prompt_token);
    assert!(!args.prompt_password);
}

#[test]
fn help_explains_flat_layout() {
    let help = render_alert_help();
    assert!(help.contains("export"));
    assert!(help.contains("import"));
    assert!(help.contains("diff"));
    assert!(help.contains("Write rule, contact-point, mute-timing, and template files directly"));
    assert!(help.contains("instead of nested subdirectories"));
    assert!(!help.contains("--username"));
    assert!(!help.contains("--password "));
}

#[test]
fn parse_cli_supports_import_subcommand() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "import",
        "--input-dir",
        "./alerts/raw",
        "--replace-existing",
        "--dry-run",
        "--json",
    ]);
    assert_eq!(args.input_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.replace_existing);
    assert!(args.dry_run);
    assert!(args.json);
    assert!(args.diff_dir.is_none());
}

#[test]
fn import_help_mentions_structured_dry_run_json() {
    let help = render_alert_subcommand_help(&["import"]);
    assert!(help.contains("--json"));
    assert!(help.contains("Only supported with --dry-run."));
}

#[test]
fn diff_help_mentions_structured_json() {
    let help = render_alert_subcommand_help(&["diff"]);
    assert!(help.contains("--json"));
    assert!(help.contains("Deprecated compatibility flag. Equivalent to --output-format json."));
    assert!(help.contains("--output-format"));
}

#[test]
fn parse_cli_supports_list_rules_subcommand() {
    let args: AlertCliArgs = parse_cli_from(["grafana-util alert", "list-rules", "--json"]);
    assert_eq!(args.list_kind, Some(AlertListKind::Rules));
    assert!(args.json);
    assert!(!args.text);
    assert!(!args.csv);
    assert!(!args.yaml);
    assert_eq!(args.org_id, None);
    assert!(!args.all_orgs);
}

#[test]
fn parse_cli_supports_list_alert_output_formats() {
    fn assert_output_mode(args: &AlertCliArgs, mode: &str) {
        match mode {
            "text" => {
                assert!(args.text);
                assert!(!args.table);
                assert!(!args.csv);
                assert!(!args.json);
                assert!(!args.yaml);
            }
            "table" => {
                assert!(args.table);
                assert!(!args.text);
                assert!(!args.csv);
                assert!(!args.json);
                assert!(!args.yaml);
            }
            "csv" => {
                assert!(args.csv);
                assert!(!args.text);
                assert!(!args.table);
                assert!(!args.json);
                assert!(!args.yaml);
            }
            "json" => {
                assert!(args.json);
                assert!(!args.text);
                assert!(!args.table);
                assert!(!args.csv);
                assert!(!args.yaml);
            }
            "yaml" => {
                assert!(args.yaml);
                assert!(!args.text);
                assert!(!args.table);
                assert!(!args.csv);
                assert!(!args.json);
            }
            other => panic!("unexpected output mode {other}"),
        }
    }

    let cases = vec![
        (
            vec![
                "grafana-util alert",
                "list-rules",
                "--output-format",
                "text",
            ],
            AlertListKind::Rules,
            "text",
        ),
        (
            vec![
                "grafana-util alert",
                "list-contact-points",
                "--output-format",
                "yaml",
            ],
            AlertListKind::ContactPoints,
            "yaml",
        ),
        (
            vec!["grafana-util alert", "list-mute-timings", "--csv"],
            AlertListKind::MuteTimings,
            "csv",
        ),
        (
            vec!["grafana-util alert", "list-templates", "--json"],
            AlertListKind::Templates,
            "json",
        ),
    ];

    for (argv, kind, mode) in cases {
        let args: AlertCliArgs = parse_cli_from(argv);
        assert_eq!(args.list_kind, Some(kind));
        assert_output_mode(&args, mode);
    }
}

#[test]
fn parse_cli_supports_list_rules_output_format_yaml() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-util alert",
        "list-rules",
        "--output-format",
        "yaml",
    ]);
    assert_eq!(args.list_kind, Some(AlertListKind::Rules));
    assert!(args.yaml);
    assert!(!args.table);
    assert!(!args.csv);
    assert!(!args.json);
}

#[test]
fn parse_cli_supports_list_rules_output_format_csv() {
    let args: AlertCliArgs =
        parse_cli_from(["grafana-util alert", "list-rules", "--output-format", "csv"]);
    assert_eq!(args.list_kind, Some(AlertListKind::Rules));
    assert!(args.csv);
    assert!(!args.table);
    assert!(!args.json);
    assert!(!args.text);
    assert!(!args.yaml);
}

#[test]
fn parse_cli_supports_list_rules_org_routing_flags() {
    let org_args: AlertCliArgs = parse_cli_from([
        "grafana-util alert",
        "list-rules",
        "--org-id",
        "7",
        "--json",
    ]);
    assert_eq!(org_args.org_id, Some(7));
    assert!(!org_args.all_orgs);

    let all_orgs_args: AlertCliArgs =
        parse_cli_from(["grafana-util alert", "list-rules", "--all-orgs", "--json"]);
    assert_eq!(all_orgs_args.org_id, None);
    assert!(all_orgs_args.all_orgs);
}

#[test]
fn parse_cli_rejects_list_rules_org_id_with_all_orgs() {
    let error = root_command()
        .try_get_matches_from([
            "grafana-util alert",
            "list-rules",
            "--org-id",
            "7",
            "--all-orgs",
        ])
        .unwrap_err();
    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--all-orgs"));
}

#[test]
fn help_mentions_list_org_routing_flags() {
    let help = render_alert_subcommand_help(&["list-rules"]);
    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("This requires Basic auth."));
}

#[test]
fn help_mentions_list_output_formats() {
    let help = render_alert_subcommand_help(&["list-rules"]);
    assert!(help.contains("--text"));
    assert!(help.contains("--table"));
    assert!(help.contains("--csv"));
    assert!(help.contains("--json"));
    assert!(help.contains("--yaml"));
    assert!(help.contains("Use text, table, csv, json, or yaml."));
}
