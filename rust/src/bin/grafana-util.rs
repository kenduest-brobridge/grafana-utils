//! Unified Rust CLI binary entrypoint.
//!
//! Flow:
//! - Parse raw argv for the special `--help-full` pre-check path.
//! - Fall back to normal unified CLI parse and dispatch.
//! - Print any top-level error and exit with status 1.
use grafana_utils_rust::cli::{maybe_render_unified_help_from_os_args, parse_cli_from, run_cli};
use grafana_utils_rust::dashboard::maybe_render_dashboard_help_full_from_os_args;
use std::io::IsTerminal;

fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    if let Some(help_text) =
        maybe_render_unified_help_from_os_args(args.clone(), std::io::stdout().is_terminal())
    {
        print!("{help_text}");
        return;
    }
    if let Some(help_text) = maybe_render_dashboard_help_full_from_os_args(args.clone()) {
        print!("{help_text}");
        return;
    }
    if let Err(error) = run_cli(parse_cli_from(args)) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
