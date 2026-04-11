use clap::error::ErrorKind;
use clap::{ColorChoice, Command, CommandFactory};

use super::grouped::render_short_help_text;
use super::grouped_specs::{find_grouped_help_entrypoint, UNIFIED_ROOT_HELP_SPEC};
use super::schema::{
    render_dashboard_history_schema_help, render_diff_schema_help, render_status_schema_help,
    render_workspace_schema_help,
};
use crate::access::root_command as access_root_command;
use crate::alert::root_command as alert_root_command;
use crate::cli::CliArgs;
use crate::cli_help_examples::{
    colorize_dashboard_subcommand_help, colorize_help_examples, ACCESS_HELP_FULL_TEXT,
    ALERT_HELP_FULL_TEXT, DATASOURCE_HELP_FULL_TEXT, SYNC_HELP_FULL_TEXT, UNIFIED_HELP_FULL_TEXT,
};
use crate::dashboard::{
    maybe_render_dashboard_help_full_from_os_args,
    maybe_render_dashboard_subcommand_help_from_os_args,
};
use crate::datasource::root_command as datasource_root_command;
use crate::sync::SyncCliArgs;

pub(crate) const UNIFIED_DATASOURCE_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util datasource list --input-dir ./datasources --json\n  grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./datasources --dry-run --json";
pub(crate) const UNIFIED_SYNC_HELP_TEXT: &str = "Examples:\n\n  grafana-util workspace scan ./grafana-oac-repo --output-format table\n  grafana-util workspace preview ./grafana-oac-repo --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format json\n  grafana-util workspace apply --preview-file ./workspace-preview.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
pub(crate) const UNIFIED_ALERT_HELP_TEXT: &str = "Examples:\n\n  grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n  grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json\n  grafana-util alert list-rules --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json";
pub(crate) const UNIFIED_ACCESS_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access user list --input-dir ./access-users --json\n  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --yes\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly";

pub(crate) fn ensure_trailing_blank_line(mut text: String) -> String {
    if text.ends_with("\n\n") {
        return text;
    }
    if text.ends_with('\n') {
        text.push('\n');
    } else {
        text.push_str("\n\n");
    }
    text
}

fn configure_help_command(command: &mut clap::Command, colorize: bool) {
    let configured = apply_inferred_help_headings(std::mem::take(command))
        .styles(crate::help_styles::CLI_HELP_STYLES)
        .next_line_help(true)
        .color(if colorize {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        });
    *command = configured;
}

fn apply_inferred_help_headings(command: clap::Command) -> clap::Command {
    command
        .mut_args(|arg| {
            if arg.is_positional() || arg.get_help_heading().is_some() {
                return arg;
            }
            match infer_help_heading_for_arg(&arg) {
                Some(heading) => arg.help_heading(heading),
                None => arg.help_heading("Command Options"),
            }
        })
        .mut_subcommands(apply_inferred_help_headings)
}

fn infer_help_heading_for_arg(arg: &clap::Arg) -> Option<&'static str> {
    let long = arg.get_long().unwrap_or_else(|| arg.get_id().as_str());
    let id = arg.get_id().as_str();
    for rule in HELP_HEADING_RULES {
        if rule.matches(long, id) {
            return Some(rule.heading);
        }
    }
    None
}

struct HelpHeadingRule {
    heading: &'static str,
    exact: &'static [&'static str],
    prefixes: &'static [&'static str],
    contains: &'static [&'static str],
}

impl HelpHeadingRule {
    fn matches(&self, long: &str, id: &str) -> bool {
        self.exact
            .iter()
            .any(|candidate| *candidate == long || *candidate == id)
            || self
                .prefixes
                .iter()
                .any(|prefix| long.starts_with(prefix) || id.starts_with(prefix))
            || self
                .contains
                .iter()
                .any(|needle| long.contains(needle) || id.contains(needle))
    }
}

const HELP_HEADING_RULES: &[HelpHeadingRule] = &[
    HelpHeadingRule {
        heading: "Profile Options",
        exact: &["set-default", "current", "default"],
        prefixes: &["profile-"],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Connection Options",
        exact: &[
            "profile",
            "url",
            "token",
            "api-token",
            "basic-user",
            "basic-password",
            "prompt-password",
            "prompt-token",
        ],
        prefixes: &[],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Transport Options",
        exact: &["timeout", "verify-ssl", "insecure", "ca-cert"],
        prefixes: &[],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Mapping Options",
        exact: &["dashboard-uid-map", "panel-id-map", "mapping-file"],
        prefixes: &[],
        contains: &["-map"],
    },
    HelpHeadingRule {
        heading: "Authoring Options",
        exact: &[
            "role",
            "disabled",
            "receiver",
            "severity",
            "expr",
            "threshold",
            "above",
            "for",
            "label",
            "annotation",
            "rule-group",
            "contact-point",
            "seconds-to-live",
        ],
        prefixes: &[],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Account Options",
        exact: &["org-role", "grafana-admin"],
        prefixes: &["set-org-", "set-grafana-"],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Membership Options",
        exact: &["member", "admin", "with-members", "with-teams"],
        prefixes: &["member-", "admin-"],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Target Options",
        exact: &[
            "dashboard-uid",
            "uid",
            "folder",
            "folder-uid",
            "folder-path",
            "identity",
            "kind",
            "resource",
            "name",
            "title",
            "email",
            "login",
            "service-account-id",
            "token-name",
        ],
        prefixes: &["target-", "source-"],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Scope Options",
        exact: &[
            "all-orgs",
            "org-id",
            "scope",
            "page",
            "per-page",
            "page-size",
            "query",
            "limit",
            "current-org",
        ],
        prefixes: &[],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Input Options",
        exact: &[
            "input",
            "input-dir",
            "input-format",
            "plan-file",
            "preview-file",
            "input-test-file",
            "desired-file",
            "sync-summary-file",
            "package-test-file",
            "bundle-preflight-file",
            "promotion-summary-file",
            "mapping-file",
            "availability-file",
        ],
        prefixes: &[],
        contains: &["export-dir", "provisioning-dir", "provisioning-file"],
    },
    HelpHeadingRule {
        heading: "Output Options",
        exact: &[
            "output",
            "output-dir",
            "output-format",
            "output-columns",
            "list-columns",
            "text",
            "table",
            "csv",
            "json",
            "yaml",
            "interactive",
            "color",
            "no-header",
            "progress",
            "verbose",
        ],
        prefixes: &[],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Layout Options",
        exact: &["flat"],
        prefixes: &[],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Safety Options",
        exact: &[
            "overwrite",
            "replace-existing",
            "dry-run",
            "yes",
            "approve",
            "apply-live",
            "allow-policy-reset",
        ],
        prefixes: &[],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Approval Options",
        exact: &["applied-by", "applied-at", "approval-reason", "apply-note"],
        prefixes: &["approval-", "approve-", "apply-"],
        contains: &[],
    },
    HelpHeadingRule {
        heading: "Secret Storage Options",
        exact: &[
            "password",
            "password-file",
            "prompt-user-password",
            "store-secret",
            "secret-file",
            "prompt-secret-passphrase",
            "secret-passphrase-env",
            "token-env",
            "password-env",
        ],
        prefixes: &[],
        contains: &["secret", "password"],
    },
    HelpHeadingRule {
        heading: "Review Options",
        exact: &["policy", "strict", "fail-on-warning", "validate"],
        prefixes: &["review-", "validation-"],
        contains: &[],
    },
];

fn render_long_help_with_color_choice(command: &mut clap::Command, colorize: bool) -> String {
    configure_help_command(command, colorize);
    let rendered = command.render_long_help();
    if colorize {
        rendered.ansi().to_string()
    } else {
        rendered.to_string()
    }
}

fn render_domain_help_full_text(
    mut command: clap::Command,
    extended_examples: &str,
    colorize: bool,
) -> String {
    let mut help = render_long_help_with_color_choice(&mut command, colorize);
    if colorize {
        help.push_str(&colorize_help_examples(extended_examples));
    } else {
        help.push_str(extended_examples);
    }
    ensure_trailing_blank_line(help)
}

fn render_workspace_domain_help_full_text(colorize: bool) -> String {
    render_domain_help_full_text(
        SyncCliArgs::command().name("grafana-util workspace"),
        SYNC_HELP_FULL_TEXT,
        colorize,
    )
}

fn render_grouped_entrypoint(path: &[String], colorize: bool) -> Option<String> {
    find_grouped_help_entrypoint(path).map(|entrypoint| {
        ensure_trailing_blank_line(render_short_help_text(entrypoint.spec, colorize))
    })
}

fn render_unified_subcommand_help(path: &[String], colorize: bool) -> Option<String> {
    let mut command = CliArgs::command();
    let mut current = &mut command;
    for segment in path {
        current = current.find_subcommand_mut(segment)?;
    }
    let help = render_long_help_with_color_choice(current, colorize);
    let usage_prefix = format!("Usage: {}", path.last()?);
    let help = help.replacen(
        &usage_prefix,
        &format!("Usage: grafana-util {}", path.join(" ")),
        1,
    );
    if !colorize {
        return Some(ensure_trailing_blank_line(help));
    }
    let help = if path.iter().any(|segment| segment == "dashboard") {
        colorize_dashboard_subcommand_help(&help)
    } else {
        colorize_help_examples(&help)
    };
    Some(ensure_trailing_blank_line(help))
}

pub fn render_unified_help_text(colorize: bool) -> String {
    ensure_trailing_blank_line(render_short_help_text(&UNIFIED_ROOT_HELP_SPEC, colorize))
}

pub fn render_unified_help_full_text(colorize: bool) -> String {
    let mut help = render_unified_help_text(colorize);
    if colorize {
        help.push_str(&colorize_help_examples(UNIFIED_HELP_FULL_TEXT));
    } else {
        help.push_str(UNIFIED_HELP_FULL_TEXT);
    }
    ensure_trailing_blank_line(help)
}

pub fn render_unified_version_text() -> String {
    crate::common::TOOL_VERSION_TEXT.to_string()
}

fn render_contextual_help_from_args(args: &[String], colorize: bool) -> Option<String> {
    let mut command = CliArgs::command();
    configure_help_command(&mut command, colorize);
    let error = command.try_get_matches_from(args).err()?;
    if !matches!(
        error.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    ) {
        return None;
    }
    let rendered = error.render();
    let help = if colorize {
        rendered.ansi().to_string()
    } else {
        rendered.to_string()
    };
    let help = group_default_options_section(&help);
    let help = normalize_option_entry_spacing(&help);
    let help = if colorize {
        colorize_contextual_help(&help, args)
    } else {
        help
    };
    Some(ensure_trailing_blank_line(help))
}

fn colorize_contextual_help(help: &str, args: &[String]) -> String {
    if args
        .iter()
        .any(|segment| segment == "dashboard" || segment == "db")
    {
        colorize_dashboard_subcommand_help(help)
    } else {
        colorize_help_examples(help)
    }
}

fn normalize_option_entry_spacing(help: &str) -> String {
    let mut lines = Vec::new();
    let mut previous_non_empty = false;
    let mut previous_was_section_heading = false;
    for line in help.lines() {
        if is_help_option_entry(line) && previous_non_empty && !previous_was_section_heading {
            lines.push(String::new());
        }
        lines.push(line.to_string());
        let trimmed = line.trim();
        previous_non_empty = !trimmed.is_empty();
        previous_was_section_heading =
            previous_non_empty && line == trimmed && trimmed.ends_with(':');
    }
    lines.join("\n")
}

fn group_default_options_section(help: &str) -> String {
    let lines = help.lines().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if lines[index] == "Options:" {
            let section_start = index + 1;
            let mut section_end = section_start;
            while section_end < lines.len() && !is_top_level_help_section(lines[section_end]) {
                section_end += 1;
            }
            let section = &lines[section_start..section_end];
            let entries = parse_help_option_entries(section);
            if !entries.is_empty() {
                output.extend(render_grouped_options_section(entries));
            } else {
                output.push(lines[index].to_string());
                output.extend(section.iter().map(|line| (*line).to_string()));
            }
            index = section_end;
            continue;
        }
        output.push(lines[index].to_string());
        index += 1;
    }
    output.join("\n")
}

fn is_top_level_help_section(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && line == trimmed && trimmed.ends_with(':') && trimmed != "Options:"
}

fn parse_help_option_entries(lines: &[&str]) -> Vec<Vec<String>> {
    let mut entries = Vec::new();
    let mut current = Vec::new();
    for line in lines {
        if is_help_option_entry(line) {
            if !current.is_empty() {
                entries.push(current);
                current = Vec::new();
            }
            current.push((*line).to_string());
        } else if !current.is_empty() {
            current.push((*line).to_string());
        }
    }
    if !current.is_empty() {
        entries.push(current);
    }
    entries
}

fn render_grouped_options_section(entries: Vec<Vec<String>>) -> Vec<String> {
    let group_order = [
        "Connection Options",
        "Transport Options",
        "Target Options",
        "Scope Options",
        "Input Options",
        "Mapping Options",
        "Authoring Options",
        "Account Options",
        "Membership Options",
        "Output Options",
        "Layout Options",
        "Safety Options",
        "Approval Options",
        "Secret Storage Options",
        "Profile Options",
        "Review Options",
        "Command Options",
        "Help Options",
        "Other Options",
    ];
    let mut grouped = group_order
        .iter()
        .map(|group| (*group, Vec::new()))
        .collect::<Vec<(&str, Vec<Vec<String>>)>>();
    for entry in entries {
        let group = option_group_for_entry(&entry);
        if let Some((_, bucket)) = grouped.iter_mut().find(|(name, _)| *name == group) {
            bucket.push(entry);
        }
    }
    let mut output = Vec::new();
    for (group, entries) in grouped {
        if entries.is_empty() {
            continue;
        }
        output.push(format!("{group}:"));
        for entry in entries {
            output.extend(entry);
        }
    }
    output
}

fn option_group_for_entry(entry: &[String]) -> &'static str {
    let first = entry
        .first()
        .map(|line| line.trim_start())
        .unwrap_or_default();
    let first = strip_ansi_for_detection(first);
    let options = option_names_from_entry(&first);
    if options.iter().any(|option| option == "help") {
        return "Help Options";
    }
    options
        .iter()
        .find_map(|option| infer_help_heading_for_name(option))
        .unwrap_or("Other Options")
}

fn infer_help_heading_for_name(name: &str) -> Option<&'static str> {
    HELP_HEADING_RULES
        .iter()
        .find(|rule| rule.matches(name, name))
        .map(|rule| rule.heading)
}

fn option_names_from_entry(entry: &str) -> Vec<String> {
    entry
        .split_whitespace()
        .filter_map(|token| {
            let token = token.trim_end_matches(',');
            token
                .strip_prefix("--")
                .map(|long| long.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-'))
        })
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn strip_ansi_for_detection(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for code_ch in chars.by_ref() {
                if code_ch.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        output.push(ch);
    }
    output
}

fn is_help_option_entry(line: &str) -> bool {
    let stripped = strip_ansi_for_detection(line);
    let trimmed = stripped.trim_start();
    let indent_len = line.len() - trimmed.len();
    indent_len > 0 && trimmed.starts_with('-')
}

fn command_path_before_help(args: &[String], help_index: usize) -> Vec<String> {
    args.get(1..help_index)
        .unwrap_or(&[])
        .iter()
        .take_while(|value| !value.starts_with('-'))
        .cloned()
        .collect()
}

fn canonical_subcommand_name(command: &Command, token: &str) -> Option<String> {
    if let Some(subcommand) = command.find_subcommand(token) {
        return Some(subcommand.get_name().to_string());
    }

    let mut matches = command.get_subcommands().filter_map(|subcommand| {
        let matches_name = subcommand.get_name().starts_with(token);
        let matches_alias = subcommand
            .get_all_aliases()
            .any(|alias| alias.starts_with(token));
        (matches_name || matches_alias).then(|| subcommand.get_name().to_string())
    });
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

pub(crate) fn canonicalize_inferred_subcommands(command: Command, args: &[String]) -> Vec<String> {
    let mut normalized = args.to_vec();
    let mut current = &command;
    let mut index = 1;
    while index < normalized.len() {
        let token = normalized[index].clone();
        if token.starts_with('-') {
            break;
        }
        let Some(canonical) = canonical_subcommand_name(current, &token) else {
            break;
        };
        normalized[index] = canonical.clone();
        let Some(next) = current.find_subcommand(&canonical) else {
            break;
        };
        current = next;
        index += 1;
    }
    normalized
}

fn help_colorize_from_args(args: &[String], default_colorize: bool) -> bool {
    let mut colorize = default_colorize;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--color" => {
                if let Some(value) = args.get(index + 1) {
                    match value.as_str() {
                        "always" => colorize = true,
                        "never" | "none" | "off" => colorize = false,
                        _ => {}
                    }
                    index += 1;
                }
            }
            value if value.starts_with("--color=") => match value.trim_start_matches("--color=") {
                "always" => colorize = true,
                "never" | "none" | "off" => colorize = false,
                _ => {}
            },
            _ => {}
        }
        index += 1;
    }
    colorize
}

fn strip_global_help_options(args: &[String]) -> Vec<String> {
    let Some(binary) = args.first() else {
        return Vec::new();
    };
    let mut stripped = vec![binary.clone()];
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--color" => {
                index += 2;
            }
            value if value.starts_with("--color=") => {
                index += 1;
            }
            _ => {
                stripped.push(args[index].clone());
                index += 1;
            }
        }
    }
    stripped
}

fn should_render_path_help_before_root_match(path: &[String]) -> bool {
    if find_grouped_help_entrypoint(path).is_some() {
        return false;
    }
    match path.first().map(String::as_str) {
        None => false,
        Some("dashboard" | "db") if path.len() <= 2 => {
            matches!(path.get(1).map(String::as_str), Some("convert"))
        }
        Some(_) => true,
    }
}

pub fn maybe_render_unified_help_from_os_args<I, T>(iter: I, colorize: bool) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let raw_args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let colorize = help_colorize_from_args(&raw_args, colorize);
    let stripped_args = strip_global_help_options(&raw_args);
    let args = canonicalize_inferred_subcommands(CliArgs::command(), &stripped_args);
    if args.len() >= 3
        && args.get(1).map(String::as_str) == Some("workspace")
        && args.iter().any(|value| value == "--help-schema")
    {
        let target = args
            .get(if args.get(2).map(String::as_str) == Some("ci") {
                3
            } else {
                2
            })
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str);
        return render_workspace_schema_help(target);
    }
    if args.len() >= 4
        && args.get(1).map(String::as_str) == Some("dashboard")
        && args.get(2).map(String::as_str) == Some("history")
        && args.iter().any(|value| value == "--help-schema")
    {
        let target = args
            .get(3)
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str);
        return render_dashboard_history_schema_help(target);
    }
    if args.len() >= 4
        && args.get(1).map(String::as_str) == Some("dashboard")
        && args.get(2).map(String::as_str) == Some("diff")
        && args.iter().any(|value| value == "--help-schema")
    {
        return render_diff_schema_help("dashboard");
    }
    if args.len() >= 4
        && args.get(1).map(String::as_str) == Some("alert")
        && args.get(2).map(String::as_str) == Some("diff")
        && args.iter().any(|value| value == "--help-schema")
    {
        return render_diff_schema_help("alert");
    }
    if args.len() >= 4
        && args.get(1).map(String::as_str) == Some("datasource")
        && args.get(2).map(String::as_str) == Some("diff")
        && args.iter().any(|value| value == "--help-schema")
    {
        return render_diff_schema_help("datasource");
    }
    if args.len() >= 3
        && args.get(1).map(String::as_str) == Some("status")
        && args.iter().any(|value| value == "--help-schema")
    {
        let target = args
            .get(2)
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str);
        return render_status_schema_help(target);
    }
    if let Some(help_index) = args
        .iter()
        .position(|value| value == "--help" || value == "-h")
    {
        let path = command_path_before_help(&args, help_index);
        if should_render_path_help_before_root_match(&path) {
            if let Some(help) = render_contextual_help_from_args(&args, colorize) {
                return Some(help);
            }
            if let Some(help) = render_unified_subcommand_help(&path, colorize) {
                return Some(help);
            }
        }
        if let Some(help) =
            maybe_render_dashboard_subcommand_help_from_os_args(args.clone(), colorize)
        {
            return Some(help);
        }
    }
    if args.iter().any(|value| value == "--help-full") {
        if let Some(help) = maybe_render_dashboard_help_full_from_os_args(args.clone()) {
            return Some(help);
        }
    }
    match args.as_slice() {
        [_binary] => Some(render_unified_help_text(colorize)),
        [_binary, flag] if flag == "--help" || flag == "-h" => {
            Some(render_unified_help_text(colorize))
        }
        [_binary, flag] if flag == "--help-full" => Some(render_unified_help_full_text(colorize)),
        [_binary, command, flag] if command == "dashboard" && flag == "--help-full" => {
            maybe_render_dashboard_help_full_from_os_args(args.clone())
        }
        [_binary, command, flag] if command == "alert" && flag == "--help-full" => Some(
            render_domain_help_full_text(alert_root_command(), ALERT_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "datasource" && flag == "--help-full" => {
            Some(render_domain_help_full_text(
                datasource_root_command(),
                DATASOURCE_HELP_FULL_TEXT,
                colorize,
            ))
        }
        [_binary, command, flag] if command == "access" && flag == "--help-full" => Some(
            render_domain_help_full_text(access_root_command(), ACCESS_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "workspace" && flag == "--help-full" => {
            Some(render_workspace_domain_help_full_text(colorize))
        }
        [_binary, ..] => {
            let path = args[1..]
                .iter()
                .take_while(|value| !value.starts_with('-'))
                .cloned()
                .collect::<Vec<_>>();
            render_grouped_entrypoint(&path, colorize)
        }
        _ => None,
    }
}
