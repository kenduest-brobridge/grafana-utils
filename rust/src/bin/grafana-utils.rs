use grafana_utils_rust::cli::{parse_cli_from, run_cli};
use grafana_utils_rust::dashboard::maybe_render_dashboard_help_full_from_os_args;

fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    if let Some(help_text) = maybe_render_dashboard_help_full_from_os_args(args.clone()) {
        print!("{help_text}");
        return;
    }
    if let Err(error) = run_cli(parse_cli_from(args)) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
