use super::{maybe_render_unified_help_from_os_args, CliArgs};
use clap::{Command, CommandFactory};

fn render_cli_help_path(path: &[&str]) -> String {
    let mut command = CliArgs::command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing cli subcommand {segment}"));
    }
    current.render_help().to_string()
}

fn collect_public_leaf_command_paths(
    command: &Command,
    path: &mut Vec<String>,
    output: &mut Vec<Vec<String>>,
) {
    let visible_subcommands = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .collect::<Vec<_>>();
    if visible_subcommands.is_empty() {
        if !path.is_empty() {
            output.push(path.clone());
        }
        return;
    }
    for subcommand in visible_subcommands {
        path.push(subcommand.get_name().to_string());
        collect_public_leaf_command_paths(subcommand, path, output);
        path.pop();
    }
}

fn render_public_leaf_help(path: &[String]) -> Option<String> {
    let mut args = vec!["grafana-util".to_string()];
    args.extend(path.iter().cloned());
    args.push("--help".to_string());
    maybe_render_unified_help_from_os_args(args, false)
}

fn has_examples_section(help: &str) -> bool {
    help.starts_with("Examples:") || help.contains("\nExamples:")
}

#[path = "cli_dispatch_docs_contract_rust_tests.rs"]
mod cli_dispatch_docs_contract_rust_tests;
#[path = "cli_help_rust_tests.rs"]
mod cli_help_rust_tests;
#[path = "cli_parser_surface_rust_tests.rs"]
mod cli_parser_surface_rust_tests;
