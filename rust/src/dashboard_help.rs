//! Dashboard help extension helpers.
//! Provides long-help fallback rendering for inspect-export/live commands when `--help-full` is requested.
use clap::CommandFactory;

use super::DashboardCliArgs;

fn render_dashboard_subcommand_help_text(subcommand_name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    let subcommand = command
        .find_subcommand_mut(subcommand_name)
        .unwrap_or_else(|| panic!("missing dashboard subcommand {subcommand_name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    String::from_utf8(output).expect("dashboard help should be valid UTF-8")
}

pub fn render_inspect_export_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("inspect-export");
    text.push_str(INSPECT_EXPORT_HELP_FULL_EXAMPLES);
    text
}

pub fn render_inspect_live_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("inspect-live");
    text.push_str(INSPECT_LIVE_HELP_FULL_EXAMPLES);
    text
}

pub fn maybe_render_dashboard_help_full_from_os_args<I, T>(iter: I) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString>,
{
    let args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<String>>();
    if !args.iter().any(|value| value == "--help-full") {
        return None;
    }
    let rest = args.get(1..).unwrap_or(&[]);
    match rest {
        [dashboard, command, ..] if dashboard == "dashboard" && command == "inspect-export" => {
            Some(render_inspect_export_help_full())
        }
        [dashboard, command, ..] if dashboard == "dashboard" && command == "inspect-live" => {
            Some(render_inspect_live_help_full())
        }
        [command, ..] if command == "inspect-export" => Some(render_inspect_export_help_full()),
        [command, ..] if command == "inspect-live" => Some(render_inspect_live_help_full()),
        _ => None,
    }
}

const INSPECT_EXPORT_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Summary JSON:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view summary --format json\n\n  Flat per-query table report:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view query --format table\n\n  Dashboard-first grouped tables:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view query --format table --layout tree\n\n  Dashboard/panel tree text view:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view query --format text --layout tree --report-filter-panel-id 7\n\n  Datasource dependency JSON:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view datasource --format json\n\n  Governance tables:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view governance --format table\n";

const INSPECT_LIVE_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Summary JSON from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --view summary --format json\n\n  Flat per-query table report from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --view query --format table\n\n  Dashboard-first grouped tables from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --view query --format table --layout tree\n\n  Dashboard/panel tree text view:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --view query --format text --layout tree --report-filter-panel-id 7\n\n  Datasource dependency JSON from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --view datasource --format json\n\n  Governance tables from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --view governance --format table\n";
