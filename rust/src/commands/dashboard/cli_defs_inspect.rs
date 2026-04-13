//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

#[path = "cli_defs_inspect_analysis.rs"]
mod cli_defs_inspect_analysis;
#[path = "cli_defs_inspect_policy.rs"]
mod cli_defs_inspect_policy;
#[path = "cli_defs_inspect_screenshot.rs"]
mod cli_defs_inspect_screenshot;

pub use cli_defs_inspect_analysis::*;
pub use cli_defs_inspect_policy::*;
pub use cli_defs_inspect_screenshot::*;

fn parse_dashboard_analysis_input_format(
    value: &str,
) -> Result<super::DashboardImportInputFormat, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "raw" => Ok(super::DashboardImportInputFormat::Raw),
        "provisioning" => Ok(super::DashboardImportInputFormat::Provisioning),
        "git-sync" => Ok(super::DashboardImportInputFormat::Raw),
        other => Err(format!(
            "unsupported dashboard analysis input format {other:?}; use raw, provisioning, or git-sync"
        )),
    }
}

fn parse_dashboard_validate_input_format(
    value: &str,
) -> Result<super::DashboardImportInputFormat, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "raw" => Ok(super::DashboardImportInputFormat::Raw),
        "provisioning" => Ok(super::DashboardImportInputFormat::Provisioning),
        "git-sync" => Ok(super::DashboardImportInputFormat::Raw),
        other => Err(format!(
            "unsupported dashboard validate input format {other:?}; use raw, provisioning, or git-sync"
        )),
    }
}
