use clap::{Command, CommandFactory};

use crate::cli::CliArgs;
use crate::cli_help_examples::HELP_PALETTE;

use super::contextual::ensure_trailing_blank_line;

struct FlatHelpRow {
    command: String,
    kind: &'static str,
    purpose: String,
}

fn command_purpose(command: &Command) -> String {
    command
        .get_about()
        .or_else(|| command.get_long_about())
        .map(|value| {
            value
                .to_string()
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("-")
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string())
}

fn collect_flat_help_rows(command: &Command, path: &mut Vec<String>, rows: &mut Vec<FlatHelpRow>) {
    let visible_subcommands = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .collect::<Vec<_>>();
    if !path.is_empty() {
        rows.push(FlatHelpRow {
            command: format!("grafana-util {}", path.join(" ")),
            kind: if visible_subcommands.is_empty() {
                "command"
            } else {
                "group"
            },
            purpose: command_purpose(command),
        });
    }
    for subcommand in visible_subcommands {
        path.push(subcommand.get_name().to_string());
        collect_flat_help_rows(subcommand, path, rows);
        path.pop();
    }
}

fn render_flat_help_table(rows: &[FlatHelpRow], colorize: bool) -> String {
    let command_width = rows
        .iter()
        .map(|row| row.command.len())
        .chain(std::iter::once("COMMAND".len()))
        .max()
        .unwrap_or("COMMAND".len());
    let kind_width = rows
        .iter()
        .map(|row| row.kind.len())
        .chain(std::iter::once("KIND".len()))
        .max()
        .unwrap_or("KIND".len());
    let mut lines = vec![
        "Flat command inventory".to_string(),
        "Use this when you need a grep-friendly list of public command paths and what each one is for.".to_string(),
        String::new(),
        format!(
            "{:<command_width$}  {:<kind_width$}  PURPOSE",
            "COMMAND",
            "KIND",
            command_width = command_width,
            kind_width = kind_width
        ),
        format!(
            "{:-<command_width$}  {:-<kind_width$}  {:-<7}",
            "",
            "",
            "",
            command_width = command_width,
            kind_width = kind_width
        ),
    ];
    for row in rows {
        let command = if colorize {
            format!(
                "{}{}{}",
                HELP_PALETTE.command, row.command, HELP_PALETTE.reset
            )
        } else {
            row.command.clone()
        };
        lines.push(format!(
            "{:<command_width$}  {:<kind_width$}  {}",
            command,
            row.kind,
            row.purpose,
            command_width = if colorize {
                command_width + HELP_PALETTE.command.len() + HELP_PALETTE.reset.len()
            } else {
                command_width
            },
            kind_width = kind_width
        ));
    }
    lines.join("\n")
}

pub fn render_unified_help_flat_text(colorize: bool) -> String {
    let command = CliArgs::command();
    let mut rows = Vec::new();
    collect_flat_help_rows(&command, &mut Vec::new(), &mut rows);
    ensure_trailing_blank_line(render_flat_help_table(&rows, colorize))
}
