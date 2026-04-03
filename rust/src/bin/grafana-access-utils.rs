//! Backward-compatible access shim binary.
//!
//! Purpose:
//! - Preserve legacy `grafana-access-utils` launch path.
//! - Delegate parsing and execution directly to shared access CLI entrypoints.
use grafana_utils_rust::access::{parse_cli_from, run_access_cli};

fn main() {
    if let Err(error) = run_access_cli(parse_cli_from(std::env::args_os())) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
