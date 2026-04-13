use clap::{Args, ValueEnum};
use std::path::PathBuf;

use super::super::CommonCliArgs;

/// Enum definition for ScreenshotOutputFormat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScreenshotOutputFormat {
    Png,
    Jpeg,
    Pdf,
}

/// Enum definition for ScreenshotTheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScreenshotTheme {
    Light,
    Dark,
}

/// Enum definition for ScreenshotFullPageOutput.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScreenshotFullPageOutput {
    Single,
    Tiles,
    Manifest,
}

/// Struct definition for ScreenshotArgs.
#[derive(Debug, Clone, Args)]
pub struct ScreenshotArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        required_unless_present = "dashboard_url",
        help_heading = "Target Options",
        help = "Grafana dashboard UID to capture from the browser-rendered UI. Required unless --dashboard-url is provided."
    )]
    pub dashboard_uid: Option<String>,
    #[arg(
        long,
        required_unless_present = "dashboard_uid",
        help_heading = "Target Options",
        help = "Full Grafana dashboard URL. When provided, the runtime can reuse URL state such as var-*, from, to, orgId, and panelId."
    )]
    pub dashboard_url: Option<String>,
    #[arg(
        long,
        help_heading = "Target Options",
        help = "Optional dashboard slug. When omitted, the runtime can reuse the UID as a fallback route segment."
    )]
    pub slug: Option<String>,
    #[arg(
        long,
        help_heading = "Output Options",
        help = "Write the captured browser output to this file path."
    )]
    pub output: PathBuf,
    #[arg(
        long,
        help_heading = "Target Options",
        help = "Capture only this Grafana panel ID through the solo dashboard route."
    )]
    pub panel_id: Option<i64>,
    #[arg(
        long,
        help_heading = "State Options",
        help = "Scope the browser session to this Grafana org ID by sending X-Grafana-Org-Id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help_heading = "State Options",
        help = "Grafana time range start, for example now-6h or 2026-03-16T00:00:00Z."
    )]
    pub from: Option<String>,
    #[arg(
        long,
        help_heading = "State Options",
        help = "Grafana time range end, for example now or 2026-03-16T12:00:00Z."
    )]
    pub to: Option<String>,
    #[arg(
        long,
        help_heading = "State Options",
        value_name = "QUERY",
        help = "Grafana variable query-string fragment, for example 'var-env=prod&var-host=web01'. Useful for pasting ${__all_variables} expansion output."
    )]
    pub vars_query: Option<String>,
    #[arg(
        long,
        help_heading = "Output Options",
        default_value_t = false,
        help = "Print the final resolved Grafana capture URL before launching Chromium."
    )]
    pub print_capture_url: bool,
    #[arg(
        long,
        help_heading = "Header Options",
        value_name = "TITLE",
        num_args = 0..=1,
        default_missing_value = "__auto__",
        help = "Add a header title block above PNG/JPEG output. Pass no value to auto-detect the dashboard or panel title."
    )]
    pub header_title: Option<String>,
    #[arg(
        long,
        help_heading = "Header Options",
        value_name = "URL",
        num_args = 0..=1,
        default_missing_value = "__auto__",
        help = "Add a header URL line above PNG/JPEG output. Pass no value to reuse the resolved capture URL."
    )]
    pub header_url: Option<String>,
    #[arg(
        long,
        help_heading = "Header Options",
        default_value_t = false,
        help = "Add a header capture timestamp above PNG/JPEG output using local time formatted as YYYY-MM-DD HH:MM:SS."
    )]
    pub header_captured_at: bool,
    #[arg(
        long,
        help_heading = "Header Options",
        help = "Add a free-form header text line above PNG/JPEG output."
    )]
    pub header_text: Option<String>,
    #[arg(
        long = "var",
        help_heading = "State Options",
        value_name = "NAME=VALUE",
        help = "Repeatable Grafana template variable assignment. Example: --var env=prod --var region=us-east-1."
    )]
    pub vars: Vec<String>,
    #[arg(
        long,
        help_heading = "Rendering Options",
        value_enum,
        default_value_t = ScreenshotTheme::Dark,
        help = "Override the Grafana UI theme used for the browser capture."
    )]
    pub theme: ScreenshotTheme,
    #[arg(
        long,
        help_heading = "Output Options",
        value_enum,
        help = "Force the output format instead of inferring it from the output filename."
    )]
    pub output_format: Option<ScreenshotOutputFormat>,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = 1440,
        help = "Browser viewport width in pixels."
    )]
    pub width: u32,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = 1024,
        help = "Browser viewport height in pixels."
    )]
    pub height: u32,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = 1.0,
        help = "Browser device scale factor for higher-density raster capture."
    )]
    pub device_scale_factor: f64,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = false,
        help = "Capture the full scrollable page instead of only the initial viewport. Ignored for PDF output."
    )]
    pub full_page: bool,
    #[arg(
        long,
        help_heading = "Output Options",
        value_enum,
        default_value_t = ScreenshotFullPageOutput::Single,
        help = "When --full-page is enabled, write one stitched file, a tiles directory, or a tiles directory plus manifest metadata."
    )]
    pub full_page_output: ScreenshotFullPageOutput,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = 5000,
        help = "Extra wait time in milliseconds after navigation so Grafana panels can finish rendering."
    )]
    pub wait_ms: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Skip Chromium startup checks before rendering."
    )]
    pub skip_browser_check: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Print the final screenshot browser URL before capture."
    )]
    pub print_browser_url: bool,
    #[arg(
        long,
        help_heading = "Rendering Options",
        help = "Optional Chromium or Chrome executable path for the headless browser session."
    )]
    pub browser_path: Option<PathBuf>,
}
