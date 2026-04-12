//! Dashboard CLI parser/help regressions kept separate from runtime-heavy tests.
use super::super::{
    parse_cli_from, DashboardCliArgs, DashboardCommand, DashboardHistorySubcommand,
    SimpleOutputFormat,
};
use crate::cli_help_examples::paint_section;
use crate::dashboard::DashboardImportInputFormat;
use clap::{CommandFactory, Parser};
use std::path::PathBuf;

pub(super) fn render_dashboard_subcommand_help(name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    command
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing {name} subcommand"))
        .render_help()
        .to_string()
}

pub(super) fn render_dashboard_help() -> String {
    let mut command = DashboardCliArgs::command();
    command.render_help().to_string()
}

pub(super) fn render_dashboard_history_subcommand_help(name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    let history = command
        .find_subcommand_mut("history")
        .unwrap_or_else(|| panic!("missing history subcommand"));
    history
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing history {name} subcommand"))
        .render_help()
        .to_string()
}

#[path = "dashboard_cli_parser_help_list_export_rust_tests.rs"]
mod dashboard_cli_parser_help_list_export_rust_tests;
#[path = "dashboard_cli_parser_help_mutation_history_rust_tests.rs"]
mod dashboard_cli_parser_help_mutation_history_rust_tests;
#[path = "dashboard_cli_parser_help_workflow_rust_tests.rs"]
mod dashboard_cli_parser_help_workflow_rust_tests;
