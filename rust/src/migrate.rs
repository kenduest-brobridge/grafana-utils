//! Migration namespace for repair and reformat workflows.
//!
//! This module keeps command wiring thin and delegates the actual raw-to-prompt
//! conversion runtime to the dashboard domain implementation.
use clap::{ColorChoice, CommandFactory, Parser, Subcommand};

use crate::common::{set_json_color_choice, Result};
use crate::dashboard::{run_raw_to_prompt, RawToPromptArgs};
use crate::help_styles::CLI_HELP_STYLES;

const MIGRATE_ROOT_HELP_TEXT: &str = "Examples:\n\n  Repair one raw dashboard export before reimporting it elsewhere:\n    grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json\n\n  Convert one raw export root into a sibling prompt/ lane:\n    grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite";
const MIGRATE_HELP_FULL_TEXT: &str = "Extended Examples:\n\n  Repair a raw dashboard file with explicit datasource mapping:\n    grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --datasource-map ./datasource-map.json --resolution strict --output-format json\n\n  Repair a raw dashboard file while looking up datasources through a saved profile:\n    grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --profile prod --org-id 2\n\n  Repair a raw export root into a sibling prompt/ lane:\n    grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite";
const MIGRATE_DASHBOARD_HELP_TEXT: &str = "Examples:\n\n  grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json\n  grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite";
const MIGRATE_DASHBOARD_RAW_TO_PROMPT_HELP_TEXT: &str = "Examples:\n\n  Convert one raw dashboard file and rely on the sibling .prompt.json target:\n    grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json\n\n  Convert one raw export root into a sibling prompt/ lane:\n    grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite\n\n  Convert a raw file with explicit datasource resolution settings:\n    grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --datasource-map ./datasource-map.json --resolution exact --output-format json\n\n  Augment datasource resolution with live lookup from a profile:\n    grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --profile prod --org-id 2";

fn render_long_help_with_color_choice(command: &mut clap::Command, colorize: bool) -> String {
    let configured = std::mem::take(command).color(if colorize {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    });
    *command = configured;
    let rendered = command.render_long_help();
    if colorize {
        rendered.ansi().to_string()
    } else {
        rendered.to_string()
    }
}

fn render_migrate_command_help_text(path: &[&str], colorize: bool) -> String {
    let mut command = MigrateCliArgs::command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing migrate subcommand {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    let text = String::from_utf8(output).expect("migrate help should be valid UTF-8");
    if colorize {
        text
    } else {
        text
    }
}

fn render_migrate_help_text(colorize: bool) -> String {
    let mut command = MigrateCliArgs::command();
    let text = render_long_help_with_color_choice(&mut command, colorize);
    crate::cli_help_examples::inject_help_full_hint(text)
}

fn render_migrate_help_full_text(colorize: bool) -> String {
    let mut text = render_migrate_help_text(colorize);
    if colorize {
        text.push_str(&crate::cli_help_examples::colorize_help_examples(
            MIGRATE_HELP_FULL_TEXT,
        ));
    } else {
        text.push_str(MIGRATE_HELP_FULL_TEXT);
    }
    text
}

/// Render migrate help for `--help-full` preflight hooks before Clap parses args.
pub fn maybe_render_migrate_help_from_os_args<I, T>(iter: I, colorize: bool) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let rest = args.get(1..).unwrap_or(&[]);
    match rest {
        [migrate, flag] if migrate == "migrate" && flag == "--help-full" => {
            Some(render_migrate_help_full_text(colorize))
        }
        [migrate, flag] if migrate == "migrate" && (flag == "--help" || flag == "-h") => {
            Some(render_migrate_help_text(colorize))
        }
        [migrate, dashboard, flag]
            if migrate == "migrate"
                && dashboard == "dashboard"
                && (flag == "--help-full" || flag == "--help" || flag == "-h") =>
        {
            Some(render_migrate_command_help_text(&["dashboard"], colorize))
        }
        [migrate, dashboard, raw_to_prompt, flag]
            if migrate == "migrate"
                && dashboard == "dashboard"
                && raw_to_prompt == "raw-to-prompt"
                && (flag == "--help-full" || flag == "--help" || flag == "-h") =>
        {
            Some(render_migrate_command_help_text(
                &["dashboard", "raw-to-prompt"],
                colorize,
            ))
        }
        _ => None,
    }
}

/// Dashboard migration subcommands exposed through `grafana-util migrate`.
#[derive(Debug, Clone, Subcommand)]
pub enum MigrateDashboardCommand {
    #[command(
        name = "raw-to-prompt",
        about = "Convert raw dashboard exports into prompt lane artifacts.",
        after_help = MIGRATE_DASHBOARD_RAW_TO_PROMPT_HELP_TEXT
    )]
    RawToPrompt(RawToPromptArgs),
}

/// Migration subcommands exposed through `grafana-util migrate`.
#[derive(Debug, Clone, Subcommand)]
pub enum MigrateCommand {
    #[command(
        name = "dashboard",
        about = "Repair raw dashboard exports and prepare prompt-lane migration artifacts.",
        after_help = MIGRATE_DASHBOARD_HELP_TEXT
    )]
    Dashboard {
        #[command(subcommand)]
        command: MigrateDashboardCommand,
    },
}

/// Parsed root CLI arguments for `grafana-util migrate`.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util migrate",
    about = "Run dashboard migration and repair workflows.",
    after_help = MIGRATE_ROOT_HELP_TEXT,
    styles = CLI_HELP_STYLES
)]
pub struct MigrateCliArgs {
    #[command(subcommand)]
    pub command: MigrateCommand,
}

/// Run the migrate CLI.
pub fn run_migrate_cli(args: MigrateCliArgs) -> Result<()> {
    match args.command {
        MigrateCommand::Dashboard { command } => match command {
            MigrateDashboardCommand::RawToPrompt(raw_args) => {
                set_json_color_choice(raw_args.color);
                run_raw_to_prompt(&raw_args)
            }
        },
    }
}

#[cfg(test)]
#[path = "migrate_rust_tests.rs"]
mod migrate_rust_tests;
