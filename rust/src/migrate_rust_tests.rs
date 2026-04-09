use super::{maybe_render_migrate_help_from_os_args, MigrateCliArgs, MigrateCommand};
use clap::Parser;

#[test]
fn migrate_root_help_full_mentions_dashboard_namespace() {
    let help =
        maybe_render_migrate_help_from_os_args(["grafana-util", "migrate", "--help-full"], false)
            .expect("migrate root help");
    assert!(help.contains("grafana-util migrate dashboard raw-to-prompt"));
    assert!(help.contains("Repair a raw dashboard file with explicit datasource mapping"));
}

#[test]
fn parse_migrate_root_supports_dashboard_namespace() {
    let args = MigrateCliArgs::parse_from([
        "grafana-util migrate",
        "dashboard",
        "raw-to-prompt",
        "--input-file",
        "./dashboards/raw/cpu-main.json",
    ]);

    match args.command {
        MigrateCommand::Dashboard { .. } => {}
    }
}
