use super::{
    collect_public_leaf_command_paths, has_examples_section,
    maybe_render_unified_help_from_os_args, render_cli_help_path, render_public_leaf_help, CliArgs,
};
use crate::cli::{render_unified_help_flat_text, render_unified_help_text};
use crate::cli_help::grouped_specs::GROUPED_HELP_ENTRYPOINTS;
use crate::cli_help_examples::{paint_section, HELP_PALETTE};
use crate::common::strip_ansi_codes;
use crate::help_styles::CLI_HELP_STYLES;
use clap::builder::styling::AnsiColor;
use clap::{CommandFactory, Parser};

#[test]
fn unified_help_mentions_common_surfaces_without_legacy_dashboard_paths() {
    let help = render_unified_help_text(false);
    assert!(help.contains("Start Here:"));
    assert!(help.contains("Read & Export:"));
    assert!(help.contains("Review & Apply:"));
    assert!(help.contains("Suggested flow:"));
    assert!(help.contains("grafana-util --version"));
    assert!(help.contains("grafana-util status live --url http://localhost:3000"));
    assert!(help.contains("grafana-util config profile add dev"));
    assert!(help.contains("status"));
    assert!(help.contains("completion"));
    assert!(help.contains("export"));
    assert!(help.contains("dashboard"));
    assert!(help.contains("workspace"));
    assert!(!help.contains("\nCommands:"));
    assert!(!help.contains("status overview --dashboard-export-dir"));
    assert!(!help.contains("--all-orgs"));
    assert!(!help.contains("--mapping-file"));
    assert!(!help.contains("governance-json"));
    assert!(!help.contains("output-format governance"));
    assert!(!help.contains("advanced dashboard"));
    assert!(!help.contains("observe"));
    assert!(!help.contains("grafana-util change"));
    assert!(!help.contains("dashboard live"));
    assert!(!help.contains("dashboard draft"));
    assert!(!help.contains("dashboard sync"));
    assert!(!help.contains("dashboard analyze"));
    assert!(!help.contains("dashboard capture"));
    assert!(!help.contains("alert migrate export"));
}

#[test]
fn unified_help_flat_lists_public_commands_with_purpose() {
    let help = render_unified_help_flat_text(false);
    assert!(help.contains("Flat command inventory"));
    assert!(help.contains("COMMAND"));
    assert!(help.contains("KIND"));
    assert!(help.contains("PURPOSE"));
    assert!(help.contains("grafana-util status"));
    assert!(help.contains("group"));
    assert!(help.contains("grafana-util status live"));
    assert!(help.contains("Render shared project-wide live status."));
    assert!(help.contains("grafana-util dashboard export"));
    assert!(help.contains("Export dashboards into a local artifact tree"));
    assert!(help.contains("grafana-util access user list"));
    assert!(help.contains("List live or local Grafana users"));
    assert!(!help.contains("Struct definition"));
    assert!(!help.contains("Arguments for"));
    assert!(!help.contains("grafana-util observe"));
    assert!(!help.contains("grafana-util dashboard live"));
}

#[test]
fn unified_help_flat_renders_from_preparse_args() {
    let help = maybe_render_unified_help_from_os_args(["grafana-util", "--help-flat"], false)
        .expect("expected flat command inventory");
    assert!(help.contains("grafana-util datasource list"));
    assert!(help.contains("grafana-util alert list-rules"));
    assert!(help.contains("grafana-util workspace scan"));

    let colored = maybe_render_unified_help_from_os_args(
        ["grafana-util", "--color", "always", "--help-flat"],
        false,
    )
    .expect("expected colored flat command inventory");
    let stripped = strip_ansi_codes(&colored);
    assert!(stripped.contains("grafana-util access org list"));
    assert!(colored.contains(HELP_PALETTE.command));
}

#[test]
fn root_command_entrypoints_use_grouped_help_for_bare_and_help_forms() {
    for entrypoint in GROUPED_HELP_ENTRYPOINTS {
        let first_heading = entrypoint.spec.sections[0].heading;
        let paths = std::iter::once(entrypoint.path)
            .chain(entrypoint.aliases.iter().copied())
            .collect::<Vec<_>>();
        for path in paths {
            let mut bare_args = vec!["grafana-util"];
            bare_args.extend(path.iter().copied());
            let bare_help = maybe_render_unified_help_from_os_args(bare_args.clone(), false)
                .unwrap_or_else(|| panic!("missing grouped help for {}", bare_args.join(" ")));
            assert!(
                bare_help.contains(&format!("{first_heading}:")),
                "expected first grouped heading for {}\n{bare_help}",
                bare_args.join(" ")
            );
            assert!(
                !bare_help.contains("\nCommands:"),
                "grouped entrypoint help should not use an ungrouped Commands section for {}\n{bare_help}",
                bare_args.join(" ")
            );

            let mut help_args = bare_args.clone();
            help_args.push("--help");
            let help = maybe_render_unified_help_from_os_args(help_args.clone(), false)
                .unwrap_or_else(|| panic!("missing grouped help for {}", help_args.join(" ")));
            assert!(
                help.contains(&format!("{first_heading}:")),
                "expected first grouped heading for {}\n{help}",
                help_args.join(" ")
            );
            assert!(!help.contains("\nCommands:"));
        }
    }
}

#[test]
fn top_level_version_flags_stay_on_clap_version_path() {
    for args in [["grafana-util", "--version"], ["grafana-util", "-V"]] {
        assert!(
            maybe_render_unified_help_from_os_args(args, false).is_none(),
            "pre-flight help should not consume {}",
            args.join(" ")
        );
    }
}

#[test]
fn grouped_help_colorizes_sections_and_commands_with_shared_palette() {
    for (args, heading, command) in [
        (
            ["grafana-util", "--color", "always", "dashboard", "--help"],
            "Browse & Inspect:",
            "browse",
        ),
        (
            ["grafana-util", "--color", "always", "datasource", "--help"],
            "Browse & Inspect:",
            "browse",
        ),
        (
            ["grafana-util", "--color", "always", "workspace", "--help"],
            "Beginner Path:",
            "scan",
        ),
    ] {
        let help = maybe_render_unified_help_from_os_args(args, false)
            .expect("expected colored grouped help");
        assert!(help.contains(&format!(
            "{}{heading}{}",
            HELP_PALETTE.section, HELP_PALETTE.reset
        )));
        assert!(help.contains(&format!(
            "{}{command}{}",
            HELP_PALETTE.command, HELP_PALETTE.reset
        )));
        assert!(!help.contains(&format!(
            "{}{heading}{}",
            HELP_PALETTE.command, HELP_PALETTE.reset
        )));
        assert!(!help.contains("\nCommands:"));
    }
}

#[test]
fn inferred_root_subcommand_keeps_grouped_help_and_color() {
    let help = maybe_render_unified_help_from_os_args(["grafana-util", "dashb", "--help"], false)
        .expect("expected inferred dashboard grouped help");
    assert!(help.contains("Browse & Inspect:"));
    assert!(help.contains("Edit & Publish:"));
    assert!(!help.contains("\nCommands:"));

    let colored = maybe_render_unified_help_from_os_args(
        ["grafana-util", "--color", "always", "dashb", "--help"],
        false,
    )
    .expect("expected colored inferred dashboard grouped help");
    assert!(colored.contains(&format!(
        "{}Browse & Inspect:{}",
        HELP_PALETTE.section, HELP_PALETTE.reset
    )));
    assert!(colored.contains(&format!(
        "{}browse{}",
        HELP_PALETTE.command, HELP_PALETTE.reset
    )));
}

#[test]
fn inferred_nested_dashboard_subcommand_keeps_dashboard_help_renderer() {
    let args = ["grafana-util", "dashboard", "li", "--help"];
    let help = maybe_render_unified_help_from_os_args(args, false)
        .expect("expected inferred dashboard list help");
    assert!(help.contains("Usage: grafana-util dashboard list"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("grafana-util dashboard list"));
}

#[test]
fn inferred_dashboard_help_full_keeps_dashboard_full_help_renderer() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "dashb", "summ", "--help-full"],
        false,
    )
    .expect("expected inferred dashboard summary full help");
    assert!(help.contains("Usage: grafana-util dashboard summary"));
    assert!(help.contains("Extended Examples:"));
}

#[test]
fn ambiguous_root_subcommand_prefix_stays_on_clap_error_path() {
    let args = ["grafana-util", "da", "--help"];
    assert!(maybe_render_unified_help_from_os_args(args, false).is_none());
    assert!(
        crate::dashboard::maybe_render_dashboard_subcommand_help_from_os_args(args, false)
            .is_none()
    );

    let error = CliArgs::try_parse_from(args).expect_err("da should be ambiguous");
    let rendered = error.to_string();
    assert!(rendered.contains("da"));
    assert!(rendered.contains("dashboard"));
    assert!(rendered.contains("datasource"));
}

#[test]
fn grouped_help_only_advertises_supported_help_full_paths() {
    for args in [
        ["grafana-util", "dashboard"],
        ["grafana-util", "datasource"],
        ["grafana-util", "alert"],
    ] {
        let help = maybe_render_unified_help_from_os_args(args, false)
            .unwrap_or_else(|| panic!("missing grouped help for {}", args.join(" ")));
        assert!(
            !help.contains("<COMMAND> --help-full"),
            "grouped help should not advertise unsupported leaf --help-full for {}\n{help}",
            args.join(" ")
        );
    }

    for args in [
        vec!["grafana-util", "--help-full"],
        vec!["grafana-util", "datasource", "--help-full"],
        vec!["grafana-util", "alert", "--help-full"],
        vec!["grafana-util", "access", "--help-full"],
        vec!["grafana-util", "workspace", "--help-full"],
    ] {
        assert!(
            maybe_render_unified_help_from_os_args(args.clone(), false).is_some(),
            "advertised --help-full path should render: {}",
            args.join(" ")
        );
    }
}

#[test]
fn public_leaf_subcommand_help_includes_examples_section() {
    let command = CliArgs::command();
    let mut paths = Vec::new();
    collect_public_leaf_command_paths(&command, &mut Vec::new(), &mut paths);
    let missing = paths
        .iter()
        .filter_map(|path| {
            let help = render_public_leaf_help(path).unwrap_or_else(|| {
                panic!("missing public help for grafana-util {}", path.join(" "))
            });
            (!has_examples_section(&help)).then(|| format!("grafana-util {}", path.join(" ")))
        })
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "leaf command help missing Examples section:\n{}",
        missing.join("\n")
    );
}

#[test]
fn dashboard_convert_help_uses_unified_nested_renderer() {
    let convert_help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "dashboard", "convert", "--help"],
        false,
    )
    .expect("expected dashboard convert help");
    assert!(convert_help.contains("raw-to-prompt"));

    let raw_to_prompt_help = maybe_render_unified_help_from_os_args(
        [
            "grafana-util",
            "dashboard",
            "convert",
            "raw-to-prompt",
            "--help",
        ],
        false,
    )
    .expect("expected raw-to-prompt help");
    assert!(raw_to_prompt_help.contains("Examples:"));
    assert!(raw_to_prompt_help.contains("grafana-util dashboard convert raw-to-prompt"));
}

#[test]
fn removed_dashboard_group_help_paths_do_not_panic() {
    for legacy_group in ["live", "draft", "sync", "analyze", "capture"] {
        let args = ["grafana-util", "dashboard", legacy_group, "--help"];
        assert!(
            maybe_render_unified_help_from_os_args(args, false).is_none(),
            "unified help should not intercept removed dashboard group {legacy_group}"
        );
        assert!(
            crate::dashboard::maybe_render_dashboard_subcommand_help_from_os_args(args, false)
                .is_none(),
            "dashboard help hook should not panic or render removed dashboard group {legacy_group}"
        );
    }
}

#[test]
fn dashboard_help_uses_flat_paths_and_short_help() {
    let help =
        maybe_render_unified_help_from_os_args(["grafana-util", "dashboard", "--help"], false)
            .unwrap();
    assert!(help.contains("Browse & Inspect:"));
    assert!(help.contains("Export & Import:"));
    assert!(help.contains("Review & Diff:"));
    assert!(help.contains("Edit & Publish:"));
    assert!(help.contains("Operate & Capture:"));
    assert!(help.contains("browse"));
    assert!(help.contains("variables"));
    assert!(help.contains("get"));
    assert!(help.contains("clone"));
    assert!(help.contains("edit-live"));
    assert!(help.contains("export"));
    assert!(help.contains("import"));
    assert!(help.contains("review"));
    assert!(help.contains("patch"));
    assert!(help.contains("serve"));
    assert!(help.contains("publish"));
    assert!(help.contains("summary"));
    assert!(help.contains("dependencies"));
    assert!(help.contains("impact"));
    assert!(help.contains("policy"));
    assert!(help.contains("screenshot"));
    assert!(!help.contains("live         browse, list, vars, fetch, clone, edit, delete, history"));
    assert!(!help.contains("draft        review, patch, serve, publish"));
    assert!(!help.contains("sync         export, import, diff, convert"));
    assert!(!help.contains("analyze      summary, topology, impact, governance"));
    assert!(!help.contains("capture      screenshot"));
    assert!(!help.contains("advanced dashboard live"));
    assert!(!help.contains("migrate dashboard"));
    assert!(!help.contains("Common tasks:"));
}

#[test]
fn export_dashboard_help_shows_examples_and_grouped_headings() {
    let help = render_cli_help_path(&["export", "dashboard"]);
    assert!(help.contains("Connection Options"));
    assert!(help.contains("Selection Options"));
    assert!(help.contains("Export Variant Options"));
    assert!(help.contains("Notes:"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Preview the files and indexes without writing to disk."));
    assert!(help.contains("grafana-util export dashboard"));
    assert!(help.contains("--without-raw"));
    assert!(help.contains("--provider-name <NAME>"));
    assert!(help.contains("--provider-update-interval-seconds <SECONDS>"));
    assert!(!help.contains("--without-dashboard-raw"));
    assert!(!help.contains("--provisioning-provider-name"));
}

#[test]
fn export_dashboard_help_colorizes_notes_and_example_commands() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "export", "dashboard", "--help"],
        true,
    )
    .expect("expected export dashboard help");
    assert!(help.contains(&paint_section("Notes:")));
    assert!(help.contains(&paint_section("Examples:")));
    let caption_line = help
        .lines()
        .find(|line| strip_ansi_codes(line).contains("Export dashboards from the current org:"))
        .expect("expected example caption line");
    assert!(
        caption_line.contains(HELP_PALETTE.argument),
        "example captions should be highlighted\n{help}"
    );
    assert!(help.contains("grafana-util export dashboard"));
}

#[test]
fn export_dashboard_help_ends_with_blank_line() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "export", "dashboard", "--help"],
        false,
    )
    .expect("expected export dashboard help");
    assert!(help.ends_with("\n\n"));
}

#[test]
fn datasource_subcommand_help_uses_readable_option_spacing() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "datasource", "list", "--help"],
        false,
    )
    .expect("expected datasource list help");
    assert!(help.contains("Help Options:\n  -h, --help\n          Print help"));
    assert!(help.contains("Scope Options:\n      --org-id <ORG_ID>\n          List datasources"));
    assert!(help.contains("\n\n      --all-orgs\n          Enumerate all visible Grafana orgs"));
    assert!(help
        .contains("Connection Options:\n      --color <COLOR>\n          Colorize JSON output."));
    assert!(!help.contains("--org-id <ORG_ID>  List datasources"));
}

#[test]
fn status_live_help_groups_options_by_purpose() {
    let help =
        maybe_render_unified_help_from_os_args(["grafana-util", "status", "live", "--help"], false)
            .expect("expected status live help");
    for heading in [
        "Help Options:",
        "Connection Options:",
        "Transport Options:",
        "Scope Options:",
        "Input Options:",
        "Mapping Options:",
        "Output Options:",
    ] {
        assert!(help.contains(heading), "missing heading {heading}");
    }
    assert!(help.contains("Connection Options:\n      --profile <PROFILE>"));
    assert!(help.contains("Input Options:\n      --sync-summary-file <SYNC_SUMMARY_FILE>"));
    assert!(help.contains("Mapping Options:\n      --mapping-file <MAPPING_FILE>"));
    assert!(!help.contains("\nOptions:\n      --profile <PROFILE>"));
}

#[test]
fn colored_contextual_help_keeps_grouped_sections() {
    let help =
        maybe_render_unified_help_from_os_args(["grafana-util", "status", "live", "--help"], true)
            .expect("expected colored status live help");
    assert!(help.contains("Connection Options"));
    assert!(help.contains("Transport Options"));
    assert!(help.contains("Input Options"));
    assert!(help.contains("--sync-summary-file"));
    assert!(!help.contains("\nOptions:\n      --profile <PROFILE>"));
}

#[test]
fn profile_add_help_uses_secret_and_profile_groups() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "config", "profile", "add", "--help"],
        false,
    )
    .expect("expected profile add help");
    assert!(help.contains("Secret Storage Options:\n      --token-env <TOKEN_ENV>"));
    assert!(help.contains("Profile Options:\n      --set-default"));
    assert!(help.contains("Safety Options:\n      --replace-existing"));
    assert!(help.contains("Creates or updates one profile entry"));
    assert!(!help.contains("Other Options:"));
}

#[test]
fn access_create_help_uses_operator_groups_and_about_text() {
    let paths: &[&[&str]] = &[
        &["grafana-util", "access", "user", "add", "--help"],
        &["grafana-util", "access", "team", "add", "--help"],
        &["grafana-util", "access", "service-account", "add", "--help"],
        &[
            "grafana-util",
            "access",
            "service-account",
            "token",
            "add",
            "--help",
        ],
    ];
    for path in paths {
        let help =
            maybe_render_unified_help_from_os_args(*path, false).expect("expected access add help");
        assert!(!help.contains("Struct definition for"));
        assert!(!help.contains("Command Options:"));
        assert!(!help.contains("Other Options:"));
    }

    let user_help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "access", "user", "add", "--help"],
        false,
    )
    .expect("expected access user add help");
    assert!(user_help.contains("Create one Grafana user"));
    assert!(user_help.contains("Account Options:\n      --org-role <ORG_ROLE>"));
    assert!(user_help.contains("\n\n      --grafana-admin <GRAFANA_ADMIN>"));

    let team_help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "access", "team", "add", "--help"],
        false,
    )
    .expect("expected access team add help");
    assert!(team_help.contains("Create one Grafana team"));
    assert!(team_help.contains("Membership Options:\n      --member <MEMBERS>"));
    assert!(team_help.contains("\n\n      --admin <ADMINS>"));
}

#[test]
fn contextual_subcommand_help_preserves_parent_options() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "alert", "export", "--help"],
        false,
    )
    .expect("expected alert export help");
    assert!(help.contains("--color <COLOR>"));
    assert!(help.contains("Override JSON/YAML/table color for the alert namespace."));
    assert!(help.contains("\n\n      --output-dir <OUTPUT_DIR>\n          Directory to write"));
    assert!(!help.contains("--output-dir <OUTPUT_DIR>  Directory to write"));
}

#[test]
fn dashboard_subcommand_help_keeps_dashboard_notes_and_examples() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "dashboard", "export", "--help"],
        false,
    )
    .expect("expected dashboard export help");
    assert!(help.contains("Notes:"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Export dashboards to raw/, prompt/, provisioning/"));
    let help = crate::dashboard::maybe_render_dashboard_subcommand_help_from_os_args(
        ["grafana-util", "dashboard", "export", "--help"],
        false,
    )
    .expect("expected dashboard export help");
    assert!(help.contains("Notes:"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Export dashboards to raw/, prompt/, provisioning/"));
}

#[test]
fn dashboard_nested_help_uses_readable_contextual_spacing() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "dashboard", "history", "list", "--help"],
        false,
    )
    .expect("expected dashboard history list help");
    assert!(help.contains("Usage: grafana-util dashboard history list [OPTIONS]"));
    assert!(help.contains(
        "Target Options:\n      --dashboard-uid <DASHBOARD_UID>\n          Dashboard UID to inspect."
    ));
    assert!(help.contains(
        "Input Options:\n      --input <FILE>\n          Read one local history artifact"
    ));
    assert!(!help.contains("--dashboard-uid <DASHBOARD_UID>  Dashboard UID"));
}

#[test]
fn dashboard_direct_help_with_options_keeps_dashboard_renderer() {
    let help = crate::dashboard::maybe_render_dashboard_subcommand_help_from_os_args(
        [
            "grafana-util",
            "dashboard",
            "export",
            "--output-dir",
            "./dashboards",
            "--help",
        ],
        false,
    )
    .expect("expected dashboard export help");
    assert!(help.contains("Notes:"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Export dashboards to raw/, prompt/, provisioning/"));
}

#[test]
fn export_dashboard_help_colorizes_default_context_bright_green() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "export", "dashboard", "--help"],
        true,
    )
    .expect("expected export dashboard help");
    assert!(help.contains("default:"));
    assert!(help.contains("500"));
    assert!(help.contains("grafana-utils-dashboards"));
    assert!(help.contains("possible values:"));
}

#[test]
fn cli_help_styles_use_bright_green_bold_context() {
    let rendered = format!("{}", CLI_HELP_STYLES.get_context());
    let expected = format!("{}", AnsiColor::BrightGreen.on_default().bold());
    assert_eq!(rendered, expected);
}

#[test]
fn cli_help_styles_render_command_literals_as_bright_white() {
    let rendered = format!("{}", CLI_HELP_STYLES.get_literal());
    let expected = format!("{}", AnsiColor::BrightWhite.on_default().bold());
    assert_eq!(rendered, expected);
}

#[test]
fn export_dashboard_help_colorizes_option_descriptions_as_secondary_text() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "export", "dashboard", "--help"],
        true,
    )
    .expect("expected export dashboard help");
    assert!(help.contains(&format!(
        "          {}Set the generated provisioning provider name.",
        HELP_PALETTE.support
    )));
}

#[test]
fn colored_contextual_help_highlights_option_entries_and_inline_flags() {
    for (args, option_name, inline_flag) in [
        (
            vec!["grafana-util", "dashboard", "export", "--help"],
            "--url",
            "--profile",
        ),
        (
            vec!["grafana-util", "alert", "export", "--help"],
            "--url",
            "--token",
        ),
        (
            vec!["grafana-util", "datasource", "list", "--help"],
            "--url",
            "--profile",
        ),
        (
            vec!["grafana-util", "config", "profile", "add", "--help"],
            "--token-env",
            "--set-default",
        ),
    ] {
        let help = maybe_render_unified_help_from_os_args(args.clone(), true)
            .unwrap_or_else(|| panic!("expected colored help for {}", args.join(" ")));
        let option_line = help
            .lines()
            .find(|line| strip_ansi_codes(line).trim_start().starts_with(option_name))
            .unwrap_or_else(|| {
                panic!(
                    "missing option entry {option_name} for {}\n{help}",
                    args.join(" ")
                )
            });
        assert!(
            option_line.contains(HELP_PALETTE.argument),
            "option entry should be highlighted for {}\n{help}",
            args.join(" ")
        );
        let inline_line = help
            .lines()
            .find(|line| strip_ansi_codes(line).contains(inline_flag))
            .unwrap_or_else(|| {
                panic!(
                    "missing inline flag {inline_flag} for {}\n{help}",
                    args.join(" ")
                )
            });
        assert!(
            inline_line.contains(HELP_PALETTE.argument),
            "inline flag should be highlighted for {}\n{help}",
            args.join(" ")
        );
    }
}

#[test]
fn status_overview_help_uses_canonical_examples() {
    assert!(crate::overview::OVERVIEW_HELP_TEXT
        .contains("grafana-util status overview --dashboard-export-dir ./dashboards/raw"));
    assert!(crate::overview::OVERVIEW_LIVE_HELP_TEXT
        .contains("grafana-util status overview live --url http://localhost:3000 --token"));
    let help = render_cli_help_path(&["status", "overview"]);
    assert!(!help.contains("grafana-util observe overview"));
}

#[test]
fn config_profile_help_uses_canonical_examples() {
    let help = render_cli_help_path(&["config", "profile"]);
    assert!(help.contains("grafana-util config profile list"));
    assert!(help.contains("grafana-util config profile show --profile prod --output-format yaml"));
    assert!(help.contains("grafana-util config profile validate --profile prod"));
    assert!(help.contains("grafana-util config profile add prod --url https://grafana.example.com"));
    assert!(help.contains("grafana-util config profile init --overwrite"));
    let current_help = render_cli_help_path(&["config", "profile", "current"]);
    assert!(current_help.contains("status live, status overview"));
    assert!(!help.contains("grafana-util profile"));
}

#[test]
fn dashboard_convert_help_shows_flat_subcommand() {
    let help = render_cli_help_path(&["dashboard", "convert"]);
    assert!(help.contains("Run dashboard format conversion workflows."));
    assert!(help.contains("raw-to-prompt"));
    assert!(!help.contains("advanced dashboard"));
    assert!(!help.contains("sync"));
}

#[test]
fn dashboard_export_help_uses_short_export_flags() {
    let help = render_cli_help_path(&["dashboard", "export"]);
    assert!(help.contains("--without-raw"));
    assert!(help.contains("--provider-name <NAME>"));
    assert!(help.contains("--provider-update-interval-seconds <SECONDS>"));
    assert!(!help.contains("--without-dashboard-raw"));
    assert!(!help.contains("--provisioning-provider-name"));
}

#[test]
fn dashboard_short_help_uses_flat_paths_only() {
    let help =
        maybe_render_unified_help_from_os_args(["grafana-util", "dashboard", "--help"], false)
            .unwrap();
    assert!(help.contains("Browse & Inspect:"));
    assert!(help.contains("Export & Import:"));
    assert!(help.contains("Review & Diff:"));
    assert!(help.contains("Edit & Publish:"));
    assert!(help.contains("Operate & Capture:"));
    assert!(help.contains("browse"));
    assert!(help.contains("variables"));
    assert!(help.contains("get"));
    assert!(help.contains("clone"));
    assert!(help.contains("edit-live"));
    assert!(help.contains("delete"));
    assert!(help.contains("history"));
    assert!(help.contains("review"));
    assert!(help.contains("patch"));
    assert!(help.contains("serve"));
    assert!(help.contains("publish"));
    assert!(help.contains("export"));
    assert!(help.contains("import"));
    assert!(help.contains("diff"));
    assert!(help.contains("summary"));
    assert!(help.contains("dependencies"));
    assert!(help.contains("impact"));
    assert!(help.contains("policy"));
    assert!(help.contains("screenshot"));
    assert_eq!(help.matches("\n  get").count(), 1);
    assert!(!help.contains("live         browse, list, vars, fetch, clone, edit, delete, history"));
    assert!(!help.contains("draft        review, patch, serve, publish"));
    assert!(!help.contains("sync         export, import, diff, convert"));
    assert!(!help.contains("analyze      summary, topology, impact, governance"));
    assert!(!help.contains("capture      screenshot"));
    assert!(!help.contains("Common tasks:"));
}

#[test]
fn alert_short_help_uses_flat_task_groups_only() {
    let help =
        maybe_render_unified_help_from_os_args(["grafana-util", "alert", "-h"], false).unwrap();
    assert!(help.contains("Inventory:"));
    assert!(help.contains("Backup & Compare:"));
    assert!(help.contains("Author Desired State:"));
    assert!(help.contains("Review & Apply:"));
    assert!(help.contains("list-rules"));
    assert!(help.contains("list-contact-points"));
    assert!(help.contains("list-mute-timings"));
    assert!(help.contains("list-templates"));
    assert!(help.contains("export"));
    assert!(help.contains("import"));
    assert!(help.contains("diff"));
    assert!(help.contains("init"));
    assert!(help.contains("add-rule"));
    assert!(help.contains("clone-rule"));
    assert!(help.contains("add-contact-point"));
    assert!(help.contains("set-route"));
    assert!(help.contains("preview-route"));
    assert!(help.contains("new-rule"));
    assert!(help.contains("new-contact-point"));
    assert!(help.contains("new-template"));
    assert!(help.contains("plan"));
    assert!(help.contains("apply"));
    assert!(!help.contains("inventory  list-rules"));
    assert!(!help.contains("backup     export"));
    assert!(!help.contains("authoring  init"));
    assert!(!help.contains("review     plan"));
    assert!(!help.contains("live         list-rules"));
    assert!(!help.contains("migrate      export, import, diff"));
    assert!(!help.contains("author       init, rule add|clone"));
    assert!(!help.contains("scaffold     rule, contact-point, template"));
    assert!(!help.contains("change       plan, apply"));
}
