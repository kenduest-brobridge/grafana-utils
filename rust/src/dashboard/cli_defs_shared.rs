//! CLI definitions for Dashboard command surface and option compatibility behavior.

use crate::common::CliColorChoice;
use clap::{Args, ValueEnum};

use super::super::{DEFAULT_TIMEOUT, DEFAULT_URL};

/// Shared tabular/list output selectors for dashboard commands.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SimpleOutputFormat {
    Text,
    Table,
    Csv,
    Json,
    Yaml,
}

/// Output selectors for dashboard raw-to-prompt migration summaries.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum RawToPromptOutputFormat {
    Text,
    Table,
    Json,
    Yaml,
}

/// Output selectors for dashboard dry-run style commands.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DryRunOutputFormat {
    Text,
    Table,
    Json,
}

/// Log renderers for dashboard raw-to-prompt migration events.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum RawToPromptLogFormat {
    Text,
    Json,
}

/// Datasource resolution modes for dashboard raw-to-prompt migration.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum RawToPromptResolution {
    #[value(alias = "infer")]
    InferFamily,
    Exact,
    Strict,
}

/// Output selectors for dashboard governance-gate reports.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum GovernanceGateOutputFormat {
    Text,
    Json,
}

/// Sources for dashboard governance policy input.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum GovernancePolicySource {
    File,
    Builtin,
}

/// Output selectors for dashboard topology rendering.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum TopologyOutputFormat {
    Text,
    Json,
    Mermaid,
    Dot,
}

/// Output selectors for dashboard impact reports.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ImpactOutputFormat {
    Text,
    Json,
}

/// Output selectors for dashboard history list/restore views.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum HistoryOutputFormat {
    Text,
    Table,
    Json,
    Yaml,
}

/// Output selectors for dashboard validation reports.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ValidationOutputFormat {
    Text,
    Json,
}

/// Shared Grafana connection/authentication arguments for dashboard commands.
#[derive(Debug, Clone, Args)]
pub struct CommonCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[arg(
        long,
        help = "Load connection defaults from the selected repo-local profile in grafana-util.yaml."
    )]
    pub profile: Option<String>,
    #[arg(long, default_value = DEFAULT_URL, help = "Grafana base URL.")]
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
        help = "Prompt for the Grafana Basic auth password without echo instead of passing --basic-password on the command line."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line."
    )]
    pub prompt_token: bool,
    #[arg(long, default_value_t = DEFAULT_TIMEOUT, help = "HTTP timeout in seconds.")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default."
    )]
    pub verify_ssl: bool,
}
