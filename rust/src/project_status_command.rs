//! Shared status command surface.
//!
//! Maintainer note:
//! - This module owns the top-level `grafana-util status ...` command.
//! - It should stay focused on command args, shared rendering, and high-level
//!   staged/live aggregation handoff.
//! - Domain-specific staged/live producer logic belongs in the owning domain
//!   modules, not here.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::common::{
    render_json_value, set_json_color_choice, CliColorChoice, Result as CommonResult,
};
use crate::overview::{self, OverviewArgs, OverviewOutputFormat};
use crate::project_status::ProjectStatus;
use crate::project_status_live_runtime::build_live_project_status;
use crate::project_status_staged::build_staged_project_status;
use crate::tabular_output::{print_lines, render_summary_csv, render_summary_table, render_yaml};

pub(crate) const PROJECT_STATUS_DOMAIN_COUNT: usize = 6;
const PROJECT_STATUS_HELP_TEXT: &str = "Examples:\n\n  Render staged project status as a summary table from raw dashboard artifacts:\n    grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output table\n\n  Render staged project status from dashboard provisioning artifacts:\n    grafana-util status staged --dashboard-provisioning-dir ./dashboards/provisioning --output csv\n\n  Render staged project status from datasource provisioning YAML:\n    grafana-util status staged --datasource-provisioning-file ./datasources/provisioning/datasources.yaml --output yaml\n\n  Render live project status with staged sync context:\n    grafana-util status live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --sync-summary-file ./sync-summary.json --bundle-preflight-file ./bundle-preflight.json --output json";
const PROJECT_STATUS_STAGED_HELP_TEXT: &str = "Examples:\n\n  Render staged project status as a summary table from raw dashboard artifacts:\n    grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output table\n\n  Render staged project status from dashboard provisioning artifacts in the interactive workbench:\n    grafana-util status staged --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts --output interactive\n\n  Render staged project status from datasource provisioning YAML:\n    grafana-util status staged --datasource-provisioning-file ./datasources/provisioning/datasources.yaml --output yaml";
const PROJECT_STATUS_LIVE_HELP_TEXT: &str = "Examples:\n\n  Render live project status as YAML:\n    grafana-util status live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output yaml\n\n  Render live status across visible orgs while layering staged sync context:\n    grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --sync-summary-file ./sync-summary.json --output interactive";

#[cfg(feature = "tui")]
const PROJECT_STATUS_OUTPUT_HELP: &str =
    "Render project status as table, csv, text, json, yaml, or interactive output.";

#[cfg(not(feature = "tui"))]
const PROJECT_STATUS_OUTPUT_HELP: &str =
    "Render project status as table, csv, text, json, or yaml output.";

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ProjectStatusOutputFormat {
    Table,
    Csv,
    Text,
    Json,
    Yaml,
    #[cfg(feature = "tui")]
    Interactive,
}

#[derive(Debug, Clone, Args)]
pub struct ProjectStatusStagedArgs {
    #[arg(
        long,
        conflicts_with = "dashboard_provisioning_dir",
        help = "Dashboard export directory to summarize from staged artifacts."
    )]
    pub dashboard_export_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "dashboard_export_dir",
        help = "Dashboard provisioning directory to summarize from staged artifacts."
    )]
    pub dashboard_provisioning_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "datasource_provisioning_file",
        help = "Datasource export directory to summarize from staged artifacts."
    )]
    pub datasource_export_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "datasource_export_dir",
        help = "Datasource provisioning YAML file to summarize instead of the stable inventory contract.",
        help_heading = "Input Options"
    )]
    pub datasource_provisioning_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Access user export directory to summarize from staged artifacts."
    )]
    pub access_user_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access team export directory to summarize from staged artifacts."
    )]
    pub access_team_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access org export directory to summarize from staged artifacts."
    )]
    pub access_org_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access service-account export directory to summarize from staged artifacts."
    )]
    pub access_service_account_export_dir: Option<PathBuf>,
    #[arg(long, help = "Desired change file to summarize from staged artifacts.")]
    pub desired_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Source bundle JSON file used by staged bundle/promotion checks."
    )]
    pub source_bundle: Option<PathBuf>,
    #[arg(
        long,
        help = "Target inventory JSON file used by staged bundle/promotion checks."
    )]
    pub target_inventory: Option<PathBuf>,
    #[arg(
        long,
        help = "Alert export directory to summarize from staged artifacts."
    )]
    pub alert_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional availability JSON reused by staged preflight builders."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional mapping JSON reused by staged promotion builders."
    )]
    pub mapping_file: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = ProjectStatusOutputFormat::Text,
        help = PROJECT_STATUS_OUTPUT_HELP
    )]
    pub output: ProjectStatusOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ProjectStatusLiveArgs {
    #[arg(
        long,
        help = "Load connection defaults from the selected repo-local profile in grafana-util.yaml."
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        default_value = "http://localhost:3000",
        help = "Grafana base URL."
    )]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN."
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token."
    )]
    pub prompt_token: bool,
    #[arg(long, default_value_t = 30, help = "HTTP timeout in seconds.")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default."
    )]
    pub verify_ssl: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["verify_ssl", "ca_cert"],
        help = "Disable TLS certificate verification explicitly."
    )]
    pub insecure: bool,
    #[arg(
        long = "ca-cert",
        value_name = "PATH",
        help = "PEM bundle file to trust for Grafana TLS verification."
    )]
    pub ca_cert: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Query live status across all Grafana organizations where the domain supports it."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        help = "Grafana organization id to scope live reads where supported."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help = "Optional staged change-summary JSON used to deepen live change status."
    )]
    pub sync_summary_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged bundle-preflight JSON used to deepen live change status."
    )]
    pub bundle_preflight_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged promotion-preflight JSON used to deepen live promotion status."
    )]
    pub promotion_summary_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged promotion mapping JSON used to deepen live promotion status."
    )]
    pub mapping_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged availability JSON used to deepen live promotion status."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = ProjectStatusOutputFormat::Text,
        help = PROJECT_STATUS_OUTPUT_HELP
    )]
    pub output: ProjectStatusOutputFormat,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ProjectStatusSubcommand {
    #[command(
        about = "Render project status from staged artifacts. Use exported project inputs.",
        after_help = PROJECT_STATUS_STAGED_HELP_TEXT
    )]
    Staged(ProjectStatusStagedArgs),
    #[command(
        about = "Render project status from live Grafana read surfaces. Use current Grafana state plus optional staged context files.",
        after_help = PROJECT_STATUS_LIVE_HELP_TEXT
    )]
    Live(ProjectStatusLiveArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util status",
    about = "Render project-wide staged or live status through the shared status contract. Staged subcommands read exports; live subcommands query Grafana.",
    after_help = PROJECT_STATUS_HELP_TEXT
)]
pub struct ProjectStatusCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[command(subcommand)]
    pub command: ProjectStatusSubcommand,
}

fn staged_args_to_overview_args(args: &ProjectStatusStagedArgs) -> OverviewArgs {
    OverviewArgs {
        dashboard_export_dir: args.dashboard_export_dir.clone(),
        dashboard_provisioning_dir: args.dashboard_provisioning_dir.clone(),
        datasource_export_dir: args.datasource_export_dir.clone(),
        datasource_provisioning_file: args.datasource_provisioning_file.clone(),
        access_user_export_dir: args.access_user_export_dir.clone(),
        access_team_export_dir: args.access_team_export_dir.clone(),
        access_org_export_dir: args.access_org_export_dir.clone(),
        access_service_account_export_dir: args.access_service_account_export_dir.clone(),
        desired_file: args.desired_file.clone(),
        source_bundle: args.source_bundle.clone(),
        target_inventory: args.target_inventory.clone(),
        alert_export_dir: args.alert_export_dir.clone(),
        availability_file: args.availability_file.clone(),
        mapping_file: args.mapping_file.clone(),
        output: OverviewOutputFormat::Text,
    }
}

#[cfg(feature = "tui")]
fn run_project_status_interactive(status: ProjectStatus) -> CommonResult<()> {
    crate::project_status_tui::run_project_status_interactive(status)
}

#[cfg(not(feature = "tui"))]
fn run_project_status_interactive(_status: ProjectStatus) -> CommonResult<()> {
    Err(crate::common::tui(
        "Project-status interactive mode requires the `tui` feature.",
    ))
}

pub(crate) fn render_project_status_text(status: &ProjectStatus) -> Vec<String> {
    let mut lines = vec![
        "Project status".to_string(),
        format!(
            "Overall: status={} scope={} domains={} present={} blocked={} blockers={} warnings={} freshness={}",
            status.overall.status,
            status.scope,
            status.overall.domain_count,
            status.overall.present_count,
            status.overall.blocked_count,
            status.overall.blocker_count,
            status.overall.warning_count,
            status.overall.freshness.status,
        ),
    ];
    if !status.domains.is_empty() {
        lines.push("Domains:".to_string());
        for domain in &status.domains {
            let mut line = format!(
                "- {} status={} mode={} primary={} blockers={} warnings={} freshness={}",
                domain.id,
                domain.status,
                domain.mode,
                domain.primary_count,
                domain.blocker_count,
                domain.warning_count,
                domain.freshness.status,
            );
            if let Some(action) = domain.next_actions.first() {
                line.push_str(&format!(" next={action}"));
            }
            lines.push(line);
        }
    }
    if !status.top_blockers.is_empty() {
        lines.push("Top blockers:".to_string());
        for blocker in status.top_blockers.iter().take(5) {
            lines.push(format!(
                "- {} {} count={} source={}",
                blocker.domain, blocker.kind, blocker.count, blocker.source
            ));
        }
    }
    if !status.next_actions.is_empty() {
        lines.push("Next actions:".to_string());
        for action in status.next_actions.iter().take(5) {
            lines.push(format!(
                "- {} reason={} action={}",
                action.domain, action.reason_code, action.action
            ));
        }
    }
    lines
}

pub(crate) fn build_project_status_summary_rows(
    status: &ProjectStatus,
) -> Vec<(&'static str, String)> {
    vec![
        ("status", status.overall.status.clone()),
        ("scope", status.scope.clone()),
        ("domainCount", status.overall.domain_count.to_string()),
        ("presentCount", status.overall.present_count.to_string()),
        ("blockedCount", status.overall.blocked_count.to_string()),
        ("blockerCount", status.overall.blocker_count.to_string()),
        ("warningCount", status.overall.warning_count.to_string()),
        ("freshnessStatus", status.overall.freshness.status.clone()),
        (
            "freshnessSourceCount",
            status.overall.freshness.source_count.to_string(),
        ),
        (
            "freshnessNewestAgeSeconds",
            status
                .overall
                .freshness
                .newest_age_seconds
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
        (
            "freshnessOldestAgeSeconds",
            status
                .overall
                .freshness
                .oldest_age_seconds
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
        ("topBlockerCount", status.top_blockers.len().to_string()),
        ("nextActionCount", status.next_actions.len().to_string()),
    ]
}

/// Build the staged status document without rendering it.
pub fn execute_project_status_staged(
    args: &ProjectStatusStagedArgs,
) -> CommonResult<ProjectStatus> {
    let overview_args = staged_args_to_overview_args(args);
    let artifacts = overview::build_overview_artifacts(&overview_args)?;
    Ok(build_staged_project_status(&artifacts))
}

/// Build the live status document without rendering it.
pub fn execute_project_status_live(args: &ProjectStatusLiveArgs) -> CommonResult<ProjectStatus> {
    build_live_project_status(args)
}

pub fn run_project_status_staged(args: ProjectStatusStagedArgs) -> CommonResult<()> {
    let status = execute_project_status_staged(&args)?;
    match args.output {
        ProjectStatusOutputFormat::Table => {
            print_lines(&render_summary_table(&build_project_status_summary_rows(
                &status,
            )));
            Ok(())
        }
        ProjectStatusOutputFormat::Csv => {
            print_lines(&render_summary_csv(&build_project_status_summary_rows(
                &status,
            )));
            Ok(())
        }
        ProjectStatusOutputFormat::Text => {
            for line in render_project_status_text(&status) {
                println!("{line}");
            }
            Ok(())
        }
        ProjectStatusOutputFormat::Json => {
            println!("{}", render_json_value(&status)?);
            Ok(())
        }
        ProjectStatusOutputFormat::Yaml => {
            println!("{}", render_yaml(&status)?);
            Ok(())
        }
        #[cfg(feature = "tui")]
        ProjectStatusOutputFormat::Interactive => run_project_status_interactive(status),
    }
}

pub fn run_project_status_live(args: ProjectStatusLiveArgs) -> CommonResult<()> {
    let status = execute_project_status_live(&args)?;
    match args.output {
        ProjectStatusOutputFormat::Table => {
            print_lines(&render_summary_table(&build_project_status_summary_rows(
                &status,
            )));
            Ok(())
        }
        ProjectStatusOutputFormat::Csv => {
            print_lines(&render_summary_csv(&build_project_status_summary_rows(
                &status,
            )));
            Ok(())
        }
        ProjectStatusOutputFormat::Text => {
            for line in render_project_status_text(&status) {
                println!("{line}");
            }
            Ok(())
        }
        ProjectStatusOutputFormat::Json => {
            println!("{}", render_json_value(&status)?);
            Ok(())
        }
        ProjectStatusOutputFormat::Yaml => {
            println!("{}", render_yaml(&status)?);
            Ok(())
        }
        #[cfg(feature = "tui")]
        ProjectStatusOutputFormat::Interactive => run_project_status_interactive(status),
    }
}

pub fn run_project_status_cli(args: ProjectStatusCliArgs) -> CommonResult<()> {
    set_json_color_choice(args.color);
    match args.command {
        ProjectStatusSubcommand::Staged(inner) => run_project_status_staged(inner),
        ProjectStatusSubcommand::Live(inner) => run_project_status_live(inner),
    }
}
