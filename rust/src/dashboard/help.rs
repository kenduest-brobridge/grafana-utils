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

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_inspect_export_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("inspect-export");
    text.push_str(INSPECT_EXPORT_HELP_FULL_EXAMPLES);
    text
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_inspect_live_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("inspect-live");
    text.push_str(INSPECT_LIVE_HELP_FULL_EXAMPLES);
    text
}

/// maybe render dashboard help full from os args.
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

const INSPECT_EXPORT_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Flat per-query table report:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report\n\n  Inspect a combined multi-org export root directly:\n    grafana-util dashboard inspect-export --import-dir ./dashboards --report tree-table\n\n  Datasource governance tables:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report governance\n\n  Datasource governance JSON:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report governance-json\n\n  Dashboard-first grouped tables:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report tree-table\n\n  Narrow to one datasource and one panel id:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report tree-table --report-filter-datasource prom-main --report-filter-panel-id 7\n\n  Inspect query analysis fields such as metrics, functions, and buckets:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv --report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query\n\n  Compare Grafana folder identity with source file paths:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv --report-columns dashboard_uid,folder_path,folder_uid,parent_folder_uid,file\n\n  Inspect datasource-level org, database, bucket, or index-pattern fields:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv --report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query\n\n  Trim the per-query columns for flat or tree-table output:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report tree-table --report-columns dashboard_uid,datasource_uid,datasource_family,query,file\n";

const INSPECT_LIVE_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Flat per-query table report from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report\n\n  Datasource governance tables from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report governance\n\n  Datasource governance JSON from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report governance-json\n\n  Dashboard-first grouped tables from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report tree-table\n\n  Narrow live inspection to one datasource and one panel id:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report tree-table --report-filter-datasource prom-main --report-filter-panel-id 7\n\n  Inspect query analysis fields such as metrics, functions, and buckets:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report csv --report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query\n\n  Compare Grafana folder identity with source file paths:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report csv --report-columns dashboard_uid,folder_path,folder_uid,parent_folder_uid,file\n\n  Inspect datasource-level org, database, bucket, or index-pattern fields:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report csv --report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query\n\n  Trim the per-query columns for flat or tree-table output:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report tree-table --report-columns dashboard_uid,datasource_uid,datasource_family,query,file\n";
