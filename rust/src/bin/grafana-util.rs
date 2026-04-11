//! Unified Rust CLI binary entrypoint.
//!
//! Flow:
//! - Parse raw argv for the special `--help-full` pre-check path.
//! - Fall back to normal unified CLI parse and dispatch.
//! - Print any top-level error and exit with status 1.
use grafana_utils_rust::cli::{
    legacy_command_error_hint, maybe_render_unified_help_from_os_args, parse_cli_from, run_cli,
};
use std::io::IsTerminal;

/// Binary entrypoint for the Rust unified CLI.
///
/// Resolution order:
/// 1) unified pre-flight help hooks (including dashboard extensions)
/// 2) normal parse + dispatch via `run_cli`
fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    let string_args = args
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if let Some(help_text) =
        maybe_render_unified_help_from_os_args(args.clone(), std::io::stdout().is_terminal())
    {
        print!("{help_text}");
        return;
    }
    if let Some(hint) = legacy_command_error_hint(&string_args) {
        let command = if string_args.get(1).map(String::as_str) == Some("dashboard")
            && string_args.get(2).map(String::as_str) == Some("live")
        {
            "live"
        } else {
            string_args.get(1).map(String::as_str).unwrap_or_default()
        };
        eprintln!("error: unrecognized subcommand '{command}'\n\n  {hint}\n\nUsage: grafana-util [OPTIONS] <COMMAND>\n\nFor more information, try '--help'.");
        std::process::exit(2);
    }
    // Hand off to the shared dispatcher; non-zero exit indicates command-level error.
    if let Err(error) = run_cli(parse_cli_from(args)) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
