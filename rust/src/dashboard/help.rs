//! Dashboard help extension helpers.
//! Provides long-help fallback rendering for dashboard analysis commands when `--help-full` is requested.
use clap::{ColorChoice, CommandFactory};

use super::DashboardCliArgs;
use crate::cli_help_examples::colorize_dashboard_subcommand_help;

fn render_dashboard_subcommand_help_text(subcommand_name: &str, colorize: bool) -> String {
    let canonical_name = match subcommand_name {
        "inspect-export" => "analyze-export",
        "inspect-live" => "analyze-live",
        "inspect-vars" => "list-vars",
        other => other,
    };
    let mut command = DashboardCliArgs::command();
    let configured = std::mem::take(&mut command).color(if colorize {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    });
    command = configured;
    let subcommand = command
        .find_subcommand_mut(canonical_name)
        .unwrap_or_else(|| panic!("missing dashboard subcommand {canonical_name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let text = String::from_utf8(output).expect("dashboard help should be valid UTF-8");
    let usage_prefix = format!("Usage: {canonical_name}");
    let text = text.replacen(
        &usage_prefix,
        &format!("Usage: grafana-util dashboard {canonical_name}"),
        1,
    );
    if colorize {
        colorize_dashboard_subcommand_help(&text)
    } else {
        text
    }
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_inspect_export_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("inspect-export", false);
    text.push_str(INSPECT_EXPORT_HELP_FULL_EXAMPLES);
    text
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_inspect_live_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("inspect-live", false);
    text.push_str(INSPECT_LIVE_HELP_FULL_EXAMPLES);
    text
}

pub fn maybe_render_dashboard_subcommand_help_from_os_args<I, T>(
    iter: I,
    colorize: bool,
) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString>,
{
    let args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<String>>();
    let rest = args.get(1..).unwrap_or(&[]);
    match rest {
        [dashboard, command, flag]
            if dashboard == "dashboard" && (flag == "--help" || flag == "-h") =>
        {
            Some(render_dashboard_subcommand_help_text(command, colorize))
        }
        _ => None,
    }
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
        [dashboard, command, ..] if dashboard == "dashboard" && command == "analyze" => {
            let mut text = render_dashboard_subcommand_help_text("analyze", false);
            text.push_str(ANALYZE_HELP_FULL_EXAMPLES);
            Some(text)
        }
        [dashboard, command, ..]
            if dashboard == "dashboard"
                && (command == "analyze-export" || command == "inspect-export") =>
        {
            Some(render_inspect_export_help_full())
        }
        [dashboard, command, ..]
            if dashboard == "dashboard"
                && (command == "analyze-live" || command == "inspect-live") =>
        {
            Some(render_inspect_live_help_full())
        }
        [command, ..] if command == "analyze" => {
            let mut text = render_dashboard_subcommand_help_text("analyze", false);
            text.push_str(ANALYZE_HELP_FULL_EXAMPLES);
            Some(text)
        }
        [command, ..] if command == "analyze-export" || command == "inspect-export" => {
            Some(render_inspect_export_help_full())
        }
        [command, ..] if command == "analyze-live" || command == "inspect-live" => {
            Some(render_inspect_live_help_full())
        }
        _ => None,
    }
}

const ANALYZE_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Analyze live Grafana and render governance JSON:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Open the interactive analysis workbench over live Grafana:\n    grafana-util dashboard analyze --url http://localhost:3000 --basic-user admin --basic-password admin --interactive\n\n  Analyze a raw export root as dashboard-first grouped tables:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format tree-table\n\n  Analyze a provisioning export root for governance tables:\n    grafana-util dashboard analyze --input-dir ./dashboards/provisioning --input-format provisioning --output-format governance\n";

const INSPECT_EXPORT_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Operator-summary table output:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --table\n\n  Open the interactive inspect workbench over export artifacts:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --interactive\n\n  Inspect a combined multi-org export root directly:\n    grafana-util dashboard analyze --input-dir ./dashboards --input-format raw --output-format tree-table\n\n  Inspect a file-provisioning tree from the provisioning root:\n    grafana-util dashboard analyze --input-dir ./dashboards/provisioning --input-format provisioning --output-format tree-table\n\n  Datasource governance tables:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format governance\n\n  Machine-readable governance contract JSON:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format governance-json\n\n  Dashboard-first grouped tables:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format tree-table\n\n  Narrow to one datasource and one panel id:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format tree-table --report-filter-datasource prom-main --report-filter-panel-id 7\n\n  Inspect query analysis fields such as metrics, functions, and buckets:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format csv --report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query\n\n  Audit dashboard tags and per-panel variable and datasource counts:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format csv --report-columns dashboard_tags,panel_id,panel_query_count,panel_datasource_count,query_variables,panel_variables\n\n  Compare Grafana folder identity, slash paths, depth, and source file paths:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format csv --report-columns dashboard_uid,folder_path,folder_full_path,folder_level,folder_uid,parent_folder_uid,file\n\n  Inspect datasource-level org, database, bucket, or index-pattern fields:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format csv --report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query\n\n  Trim the per-query columns for flat or tree-table output:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format tree-table --report-columns dashboard_uid,datasource_uid,datasource_family,query,file\n";

const INSPECT_LIVE_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Operator-summary table output from live Grafana:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --table\n\n  Open the interactive inspect workbench over live Grafana:\n    grafana-util dashboard analyze --url http://localhost:3000 --basic-user admin --basic-password admin --interactive\n\n  Datasource governance tables from live Grafana:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance\n\n  Machine-readable governance contract JSON from live Grafana:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Dashboard-first grouped tables from live Grafana:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format tree-table\n\n  Narrow live inspection to one datasource and one panel id:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format tree-table --report-filter-datasource prom-main --report-filter-panel-id 7\n\n  Inspect query analysis fields such as metrics, functions, and buckets:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format csv --report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query\n\n  Audit dashboard tags and per-panel variable and datasource counts:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format csv --report-columns dashboard_tags,panel_id,panel_query_count,panel_datasource_count,query_variables,panel_variables\n\n  Compare Grafana folder identity, slash paths, depth, and source file paths:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format csv --report-columns dashboard_uid,folder_path,folder_full_path,folder_level,folder_uid,parent_folder_uid,file\n\n  Inspect datasource-level org, database, bucket, or index-pattern fields:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format csv --report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query\n\n  Trim the per-query columns for flat or tree-table output:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format tree-table --report-columns dashboard_uid,datasource_uid,datasource_family,query,file\n";
