//! Dashboard help extension helpers.
//! Provides long-help fallback rendering for dashboard summary and review commands when `--help-full` is requested.
use clap::{ColorChoice, CommandFactory};

use super::DashboardCliArgs;
use crate::cli::CliArgs;
use crate::cli_help::canonicalize_inferred_subcommands;
use crate::cli_help_examples::colorize_dashboard_subcommand_help;

fn ensure_trailing_blank_line(mut text: String) -> String {
    if text.ends_with("\n\n") {
        return text;
    }
    if text.ends_with('\n') {
        text.push('\n');
    } else {
        text.push_str("\n\n");
    }
    text
}

fn render_dashboard_subcommand_help_text(subcommand_name: &str, colorize: bool) -> Option<String> {
    let canonical_name = subcommand_name;
    let mut command = DashboardCliArgs::command();
    let configured = std::mem::take(&mut command)
        .styles(crate::help_styles::CLI_HELP_STYLES)
        .color(if colorize {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        });
    command = configured;
    let subcommand = command.find_subcommand_mut(canonical_name)?;
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).ok()?;
    let text = String::from_utf8(output).ok()?;
    let usage_prefix = format!("Usage: {canonical_name}");
    let text = text.replacen(
        &usage_prefix,
        &format!("Usage: grafana-util dashboard {canonical_name}"),
        1,
    );
    if colorize {
        Some(ensure_trailing_blank_line(
            colorize_dashboard_subcommand_help(&text),
        ))
    } else {
        Some(ensure_trailing_blank_line(text))
    }
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_summary_export_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("summary", false).unwrap_or_default();
    text.push_str(INSPECT_EXPORT_HELP_FULL_EXAMPLES);
    text
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_summary_live_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("summary", false).unwrap_or_default();
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
    let args = canonicalize_inferred_subcommands(CliArgs::command(), &args);
    let rest = args.get(1..).unwrap_or(&[]);
    if rest.first().map(String::as_str) == Some("dashboard")
        && rest.iter().any(|value| value == "--help" || value == "-h")
    {
        if let Some(command) = rest
            .get(1)
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str)
        {
            if !matches!(command, "history" | "convert") {
                return render_dashboard_subcommand_help_text(command, colorize);
            }
        }
    }
    match rest {
        [dashboard, command, flag]
            if dashboard == "dashboard"
                && !matches!(command.as_str(), "history" | "convert")
                && (flag == "--help" || flag == "-h") =>
        {
            render_dashboard_subcommand_help_text(command, colorize)
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
    let args = canonicalize_inferred_subcommands(CliArgs::command(), &args);
    if !args.iter().any(|value| value == "--help-full") {
        return None;
    }
    let rest = args.get(1..).unwrap_or(&[]);
    match rest {
        [dashboard, command, ..] if dashboard == "dashboard" && command == "summary" => {
            let mut text = render_dashboard_subcommand_help_text("summary", false)?;
            text.push_str(SUMMARY_HELP_FULL_EXAMPLES);
            Some(text)
        }
        [command, ..] if command == "summary" => {
            let mut text = render_dashboard_subcommand_help_text("summary", false)?;
            text.push_str(SUMMARY_HELP_FULL_EXAMPLES);
            Some(text)
        }
        _ => None,
    }
}

const SUMMARY_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Render a live Grafana dashboard summary as governance JSON:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Open the interactive summary workbench over live Grafana:\n    grafana-util dashboard summary --url http://localhost:3000 --basic-user admin --basic-password admin --interactive\n\n  Summarize a raw export root as dashboard-first grouped tables:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format tree-table\n\n  Summarize a provisioning export root for governance tables:\n    grafana-util dashboard summary --input-dir ./dashboards/provisioning --input-format provisioning --output-format governance\n\n  Summarize a repo-backed Git Sync dashboard tree from the repo root:\n    grafana-util dashboard summary --input-dir ./grafana-oac-repo --input-format git-sync --output-format governance\n";

const INSPECT_EXPORT_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Operator-summary table output:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --table\n\n  Open the interactive summary workbench over export artifacts:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --interactive\n\n  Review a combined multi-org export root directly:\n    grafana-util dashboard summary --input-dir ./dashboards --input-format raw --output-format tree-table\n\n  Review a file-provisioning tree from the provisioning root:\n    grafana-util dashboard summary --input-dir ./dashboards/provisioning --input-format provisioning --output-format tree-table\n\n  Review a repo-backed Git Sync dashboard tree from the repo root:\n    grafana-util dashboard summary --input-dir ./grafana-oac-repo --input-format git-sync --output-format governance\n\n  Datasource governance tables:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format governance\n\n  Machine-readable governance contract JSON:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format governance-json\n\n  Dashboard-first grouped tables:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format tree-table\n\n  Narrow to one datasource and one panel id:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format tree-table --report-filter-datasource prom-main --report-filter-panel-id 7\n\n  Review query summary fields such as metrics, functions, and buckets:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format csv --report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query\n\n  Audit dashboard tags and per-panel variable and datasource counts:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format csv --report-columns dashboard_tags,panel_id,panel_query_count,panel_datasource_count,query_variables,panel_variables\n\n  Compare Grafana folder identity, slash paths, depth, and source file paths:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format csv --report-columns dashboard_uid,folder_path,folder_full_path,folder_level,folder_uid,parent_folder_uid,file\n\n  Review datasource-level org, database, bucket, or index-pattern fields:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format csv --report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query\n\n  Trim the per-query columns for flat or tree-table output:\n    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format tree-table --report-columns dashboard_uid,datasource_uid,datasource_family,query,file\n";

const INSPECT_LIVE_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Operator-summary table output from live Grafana:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --table\n\n  Open the interactive summary workbench over live Grafana:\n    grafana-util dashboard summary --url http://localhost:3000 --basic-user admin --basic-password admin --interactive\n\n  Datasource governance tables from live Grafana:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance\n\n  Machine-readable governance contract JSON from live Grafana:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Dashboard-first grouped tables from live Grafana:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format tree-table\n\n  Narrow live review to one datasource and one panel id:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format tree-table --report-filter-datasource prom-main --report-filter-panel-id 7\n\n  Review query summary fields such as metrics, functions, and buckets:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format csv --report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query\n\n  Audit dashboard tags and per-panel variable and datasource counts:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format csv --report-columns dashboard_tags,panel_id,panel_query_count,panel_datasource_count,query_variables,panel_variables\n\n  Compare Grafana folder identity, slash paths, depth, and source file paths:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format csv --report-columns dashboard_uid,folder_path,folder_full_path,folder_level,folder_uid,parent_folder_uid,file\n\n  Review datasource-level org, database, bucket, or index-pattern fields:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format csv --report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query\n\n  Trim the per-query columns for flat or tree-table output:\n    grafana-util dashboard summary --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format tree-table --report-columns dashboard_uid,datasource_uid,datasource_family,query,file\n";
