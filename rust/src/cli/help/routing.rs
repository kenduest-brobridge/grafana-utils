use clap::CommandFactory;

use super::contextual::{
    configure_help_command, ensure_trailing_blank_line, normalized_help_args,
    render_contextual_help_from_args,
};
use super::flat::render_unified_help_flat_text;
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

pub(crate) const UNIFIED_DATASOURCE_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util datasource list --input-dir ./datasources --json\n  grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util datasource plan --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./datasources --output-format table\n  grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./datasources --dry-run --json";
pub(crate) const UNIFIED_SYNC_HELP_TEXT: &str = "Examples:\n\n  grafana-util workspace scan ./grafana-oac-repo --output-format table\n  grafana-util workspace preview ./grafana-oac-repo --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format json\n  grafana-util workspace apply --preview-file ./workspace-preview.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
pub(crate) const UNIFIED_ALERT_HELP_TEXT: &str = "Examples:\n\n  grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n  grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json\n  grafana-util alert list-rules --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json";
pub(crate) const UNIFIED_ACCESS_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access user list --input-dir ./access-users --json\n  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --yes\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly";

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

fn command_path_before_help_args(args: &[String]) -> Option<Vec<String>> {
    let help_index = args
        .iter()
        .position(|value| value == "--help" || value == "-h")?;
    Some(
        args.get(1..help_index)
            .unwrap_or(&[])
            .iter()
            .take_while(|value| !value.starts_with('-'))
            .cloned()
            .collect(),
    )
}

pub fn maybe_render_unified_help_from_os_args<I, T>(iter: I, colorize: bool) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let (args, colorize) = normalized_help_args(iter, colorize);
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
    if let Some(path) = command_path_before_help_args(&args) {
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
        [_binary, flag] if flag == "--help-flat" => Some(render_unified_help_flat_text(colorize)),
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
            if path.is_empty() {
                return None;
            }
            render_grouped_entrypoint(&path, colorize)
        }
        _ => None,
    }
}
