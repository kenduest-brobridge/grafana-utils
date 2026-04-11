//! Unified CLI help examples and rendering helpers.
//!
//! Keeping the large example blocks and help rendering here lets `cli.rs`
//! stay focused on command topology and dispatch.

use clap::error::ErrorKind;
use clap::{ColorChoice, CommandFactory};

use crate::access::root_command as access_root_command;
use crate::alert::root_command as alert_root_command;
use crate::cli::CliArgs;
use crate::cli_help_examples::{
    colorize_dashboard_short_help, colorize_dashboard_subcommand_help, colorize_help_examples,
    inject_help_full_hint, ACCESS_HELP_FULL_TEXT, ALERT_HELP_FULL_TEXT, DATASOURCE_HELP_FULL_TEXT,
    SYNC_HELP_FULL_TEXT, UNIFIED_HELP_FULL_TEXT, UNIFIED_HELP_TEXT,
};
use crate::datasource::root_command as datasource_root_command;
use crate::sync::SyncCliArgs;

pub(crate) const UNIFIED_DASHBOARD_SHORT_HELP_TEXT: &str = "Usage: grafana-util dashboard <COMMAND>\n\nBrowse & Inspect:\n  browse       Browse dashboards interactively.\n  list         List dashboard summaries.\n  get          Fetch one dashboard JSON draft.\n  variables    List dashboard variables.\n  history      Inspect dashboard revision history.\n\nExport & Import:\n  export       Back up dashboards into raw/, prompt/, and provisioning/.\n  import       Import raw dashboard JSON through the API.\n  convert      Convert raw dashboard JSON into prompt artifacts.\n\nReview & Diff:\n  diff         Compare local raw dashboards against Grafana.\n  review       Check one local dashboard JSON draft.\n  summary      Analyze live or exported dashboards.\n  dependencies Show dashboard, datasource, variable, and alert dependencies.\n  impact       Show datasource blast radius.\n  policy       Evaluate governance policy.\n\nEdit & Publish:\n  get          Fetch one dashboard JSON draft.\n  clone        Clone one dashboard into a local draft.\n  patch        Modify one local dashboard JSON draft.\n  serve        Preview local dashboard drafts.\n  edit-live    Edit one live dashboard through a local editor.\n  publish      Publish one local dashboard JSON draft.\n  delete       Delete live dashboards after explicit selection.\n\nOperate & Capture:\n  screenshot   Capture dashboard evidence.\n\nMore help:\n  grafana-util dashboard <COMMAND> --help\n  grafana-util dashboard <COMMAND> --help-full\n";
pub(crate) const UNIFIED_DATASOURCE_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util datasource list --input-dir ./datasources --json\n  grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./datasources --dry-run --json";
pub(crate) const UNIFIED_SYNC_HELP_TEXT: &str = "Examples:\n\n  grafana-util workspace scan ./grafana-oac-repo --output-format table\n  grafana-util workspace preview ./grafana-oac-repo --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format json\n  grafana-util workspace apply --preview-file ./workspace-preview.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
pub(crate) const UNIFIED_ALERT_HELP_TEXT: &str = "Examples:\n\n  grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n  grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json\n  grafana-util alert list-rules --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json";
pub(crate) const ALERT_SHORT_HELP_TEXT: &str = "Usage: grafana-util alert <COMMAND>\n\nChoose the task first:\n  inventory    list-rules, list-contact-points, list-mute-timings, list-templates, delete\n  backup       export, import, diff\n  authoring    init, add-rule, clone-rule, add-contact-point, set-route, preview-route, new-rule, new-contact-point, new-template\n  review       plan, apply\n\nMore help:\n  grafana-util alert <COMMAND> --help\n  grafana-util alert <COMMAND> --help-full\n";
pub(crate) const UNIFIED_ACCESS_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access user list --input-dir ./access-users --json\n  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --yes\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly";

const DASHBOARD_DIFF_SCHEMA_HELP_TEXT: &str = include_str!("../../schemas/help/diff/dashboard.txt");
const ALERT_DIFF_SCHEMA_HELP_TEXT: &str = include_str!("../../schemas/help/diff/alert.txt");
const DATASOURCE_DIFF_SCHEMA_HELP_TEXT: &str =
    include_str!("../../schemas/help/diff/datasource.txt");

fn ensure_trailing_blank_line(mut text: String) -> String {
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

fn render_domain_help_text(mut command: clap::Command, colorize: bool) -> String {
    ensure_trailing_blank_line(inject_help_full_hint(render_long_help_with_color_choice(
        &mut command,
        colorize,
    )))
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

fn render_workspace_domain_help_text(colorize: bool) -> String {
    render_domain_help_text(
        SyncCliArgs::command().name("grafana-util workspace"),
        colorize,
    )
}

fn render_workspace_domain_help_full_text(colorize: bool) -> String {
    render_domain_help_full_text(
        SyncCliArgs::command().name("grafana-util workspace"),
        SYNC_HELP_FULL_TEXT,
        colorize,
    )
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
    let mut command = CliArgs::command();
    let help = inject_help_full_hint(render_long_help_with_color_choice(&mut command, colorize));
    if colorize {
        ensure_trailing_blank_line(help.replace(
            UNIFIED_HELP_TEXT,
            &colorize_help_examples(UNIFIED_HELP_TEXT),
        ))
    } else {
        ensure_trailing_blank_line(help)
    }
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

pub fn legacy_command_error_hint(args: &[String]) -> Option<&'static str> {
    let command = args.get(1).map(String::as_str)?;
    match command {
        "observe" => Some(
            "tip: 'observe' was removed. Use 'grafana-util status live' for the first read-only check, or 'grafana-util status overview live' for the live overview.",
        ),
        "overview" => Some(
            "tip: the top-level 'overview' root was removed. Use 'grafana-util status overview live' for live Grafana, or 'grafana-util status overview' with local artifact inputs.",
        ),
        "change" => Some(
            "tip: 'change' was renamed to 'workspace'. Use 'grafana-util workspace scan', 'workspace test', or 'workspace preview' before apply.",
        ),
        "advanced" if args.get(2).map(String::as_str) == Some("dashboard") => Some(
            "tip: 'advanced dashboard' was removed. Use the flat 'grafana-util dashboard ...' commands; start with 'dashboard --help' to choose browse, list, export, summary, diff, or policy.",
        ),
        "dashboard" if args.get(2).map(String::as_str) == Some("live") => Some(
            "tip: 'dashboard live' was removed. Use 'grafana-util status live' for a read-only Grafana check, or 'grafana-util dashboard browse|list' for dashboard inventory.",
        ),
        _ => None,
    }
}

fn render_workspace_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/root.help.txt"
            ))
            .to_string(),
        ),
        Some("summary") | Some("scan") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/summary.help.txt"
            ))
            .to_string(),
        ),
        Some("plan") | Some("preview") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/plan.help.txt"
            ))
            .to_string(),
        ),
        Some("review") | Some("mark-reviewed") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/review.help.txt"
            ))
            .to_string(),
        ),
        Some("apply") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/apply.help.txt"
            ))
            .to_string(),
        ),
        Some("audit") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/audit.help.txt"
            ))
            .to_string(),
        ),
        Some("preflight") | Some("test") | Some("input-test") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("assess-alerts") | Some("alert-readiness") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/assess-alerts.help.txt"
            ))
            .to_string(),
        ),
        Some("bundle-preflight") | Some("package-test") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/bundle-preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("promotion-preflight") | Some("promote-test") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/promotion-preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("bundle") | Some("package") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/bundle.help.txt"
            ))
            .to_string(),
        ),
        _ => None,
    }
}

fn render_dashboard_history_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/root.help.txt"
            ))
            .to_string(),
        ),
        Some("list") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/list.help.txt"
            ))
            .to_string(),
        ),
        Some("restore") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/restore.help.txt"
            ))
            .to_string(),
        ),
        Some("diff") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/diff.help.txt"
            ))
            .to_string(),
        ),
        Some("export") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/export.help.txt"
            ))
            .to_string(),
        ),
        _ => None,
    }
}

fn render_diff_schema_help(domain: &str) -> Option<String> {
    match domain {
        "dashboard" => Some(DASHBOARD_DIFF_SCHEMA_HELP_TEXT.to_string()),
        "alert" => Some(ALERT_DIFF_SCHEMA_HELP_TEXT.to_string()),
        "datasource" => Some(DATASOURCE_DIFF_SCHEMA_HELP_TEXT.to_string()),
        _ => None,
    }
}

fn render_status_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(include_str!("../../schemas/help/status/root.txt").to_string()),
        Some("staged") => Some(include_str!("../../schemas/help/status/staged.txt").to_string()),
        Some("live") => Some(include_str!("../../schemas/help/status/live.txt").to_string()),
        _ => None,
    }
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

fn should_render_path_help_before_root_match(path: &[String]) -> bool {
    match path.first().map(String::as_str) {
        None => false,
        Some("dashboard" | "db") if path.len() <= 2 => {
            matches!(path.get(1).map(String::as_str), Some("convert"))
        }
        Some(root)
            if path.len() == 1
                && matches!(root, "alert" | "datasource" | "access" | "workspace") =>
        {
            false
        }
        Some(_) => true,
    }
}

pub fn maybe_render_unified_help_from_os_args<I, T>(iter: I, colorize: bool) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
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
    }
    match args.as_slice() {
        [_binary] => Some(render_unified_help_text(colorize)),
        [_binary, flag] if flag == "--help" || flag == "-h" => {
            Some(render_unified_help_text(colorize))
        }
        [_binary, flag] if flag == "--help-full" => Some(render_unified_help_full_text(colorize)),
        [_binary, command, flag]
            if command == "datasource" && (flag == "--help" || flag == "-h") =>
        {
            Some(render_domain_help_text(datasource_root_command(), colorize))
        }
        [_binary, command, flag] if command == "access" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(access_root_command(), colorize))
        }
        [_binary, command, flag]
            if command == "workspace" && (flag == "--help" || flag == "-h") =>
        {
            Some(render_workspace_domain_help_text(colorize))
        }
        [_binary, command, flag]
            if command == "dashboard" && (flag == "--help" || flag == "-h") =>
        {
            Some(if colorize {
                colorize_dashboard_short_help(UNIFIED_DASHBOARD_SHORT_HELP_TEXT)
            } else {
                UNIFIED_DASHBOARD_SHORT_HELP_TEXT.to_string()
            })
        }
        [_binary, command, flag] if command == "alert" && (flag == "--help" || flag == "-h") => {
            Some(ALERT_SHORT_HELP_TEXT.to_string())
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
        _ => None,
    }
}
