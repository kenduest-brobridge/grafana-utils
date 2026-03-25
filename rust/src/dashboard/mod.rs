//! Dashboard domain orchestrator for the unified dashboard CLI.
//!
//! This file is the boundary between unified command parsing and the lower-level
//! dashboard modules that implement export/import/live/inspect/screenshot logic.
use crate::common::{message, Result};
use crate::http::JsonHttpClient;
use serde::Serialize;

// Keep the dashboard surface area split by concern. This file should stay focused
// on re-exports, shared constants, and top-level command dispatch.
mod browse;
mod browse_actions;
mod browse_edit_dialog;
mod browse_history_dialog;
mod browse_input;
mod browse_render;
mod browse_state;
mod browse_support;
mod browse_terminal;
mod browse_tui;
mod cli_defs;
mod delete;
mod delete_interactive;
mod delete_render;
mod delete_support;
mod edit;
mod edit_external;
mod edit_prompt;
mod export;
mod files;
mod governance_gate;
mod governance_gate_rules;
mod governance_gate_tui;
mod help;
mod history;
mod impact_tui;
mod import;
mod import_compare;
mod import_lookup;
mod import_render;
mod import_routed;
mod import_validation;
mod inspect;
mod inspect_analyzer_flux;
mod inspect_analyzer_loki;
mod inspect_analyzer_prometheus;
mod inspect_analyzer_search;
mod inspect_analyzer_sql;
mod inspect_governance;
mod inspect_live;
mod inspect_live_tui;
mod inspect_query;
mod inspect_render;
mod inspect_report;
mod inspect_summary;
mod list;
mod live;
mod models;
mod prompt;
mod screenshot;
mod topology;
mod topology_tui;
mod validate;
mod vars;

pub use cli_defs::{
    build_auth_context, build_http_client, build_http_client_for_org, normalize_dashboard_cli_args,
    parse_cli_from, BrowseArgs, CommonCliArgs, DashboardAuthContext, DashboardCliArgs,
    DashboardCommand, DeleteArgs, DiffArgs, ExportArgs, GovernanceGateArgs,
    GovernanceGateOutputFormat, ImpactArgs, ImpactOutputFormat, ImportArgs, InspectExportArgs,
    InspectExportReportFormat, InspectLiveArgs, InspectOutputFormat, InspectVarsArgs, ListArgs,
    ScreenshotArgs, ScreenshotFullPageOutput, ScreenshotOutputFormat, ScreenshotTheme,
    SimpleOutputFormat, TopologyArgs, TopologyOutputFormat, ValidateExportArgs,
    ValidationOutputFormat,
};
pub use export::{build_export_variant_dirs, build_output_path, export_dashboards_with_client};
pub use help::{
    maybe_render_dashboard_help_full_from_os_args, render_inspect_export_help_full,
    render_inspect_live_help_full,
};
pub use import::{diff_dashboards_with_client, import_dashboards_with_client};
pub use list::list_dashboards_with_client;
pub use live::{
    delete_dashboard_request, delete_folder_request, fetch_dashboard, import_dashboard_request,
    list_dashboard_summaries, list_datasources,
};
pub use prompt::build_external_export_document;

use browse::browse_dashboards_with_org_client;
use delete::delete_dashboards_with_org_clients;
use export::export_dashboards_with_org_clients;
use inspect::analyze_export_dir;
use inspect_live::inspect_live_dashboards_with_client;
use list::list_dashboards_with_org_clients;
use screenshot::capture_dashboard_screenshot;
use topology::{run_dashboard_impact, run_dashboard_topology};
use validate::run_dashboard_validate_export;
use vars::inspect_dashboard_variables;

pub(crate) use files::{
    build_dashboard_index_item, build_export_metadata, build_import_payload,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file, write_dashboard,
    write_json_document,
};
pub(crate) use inspect_report::ExportInspectionQueryRow;
pub(crate) use inspect_summary::ExportInspectionSummary;
pub(crate) use live::{
    build_datasource_inventory_record, build_folder_path, collect_folder_inventory_with_request,
    fetch_dashboard_if_exists_with_request, fetch_dashboard_permissions_with_request,
    fetch_dashboard_with_request, fetch_folder_if_exists_with_request,
    fetch_folder_permissions_with_request, format_folder_inventory_status_line,
    import_dashboard_request_with_request, list_dashboard_summaries_with_request,
    list_datasources_with_request,
};
pub(crate) use models::{
    DashboardIndexItem, DatasourceInventoryItem, ExportDatasourceUsageSummary, ExportMetadata,
    ExportOrgSummary, FolderInventoryItem, RootExportIndex, RootExportVariants, VariantIndexEntry,
};
pub(crate) use prompt::{
    build_datasource_catalog, collect_datasource_refs, datasource_type_alias,
    is_builtin_datasource_ref, is_placeholder_string, lookup_datasource,
    resolve_datasource_type_alias,
};

// Shared dashboard defaults and export filenames used across export/import/live flows.
pub const DEFAULT_URL: &str = "http://localhost:3000";
pub const DEFAULT_TIMEOUT: u64 = 30;
pub const DEFAULT_PAGE_SIZE: usize = 500;
pub const DEFAULT_EXPORT_DIR: &str = "dashboards";
pub const RAW_EXPORT_SUBDIR: &str = "raw";
pub const PROMPT_EXPORT_SUBDIR: &str = "prompt";
pub const DEFAULT_IMPORT_MESSAGE: &str = "Imported by grafana-utils";
pub const DEFAULT_DASHBOARD_TITLE: &str = "dashboard";
pub const DEFAULT_FOLDER_TITLE: &str = "General";
pub const DEFAULT_FOLDER_UID: &str = "general";
pub const DEFAULT_ORG_ID: &str = "1";
pub const DEFAULT_ORG_NAME: &str = "Main Org.";
pub const DEFAULT_UNKNOWN_UID: &str = "unknown";
pub const EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
pub const TOOL_SCHEMA_VERSION: i64 = 1;
pub const ROOT_INDEX_KIND: &str = "grafana-utils-dashboard-export-index";
pub const FOLDER_INVENTORY_FILENAME: &str = "folders.json";
pub const DATASOURCE_INVENTORY_FILENAME: &str = "datasources.json";
pub const DASHBOARD_PERMISSION_BUNDLE_FILENAME: &str = "permissions.json";
const BUILTIN_DATASOURCE_TYPES: &[&str] = &["__expr__", "grafana"];
const BUILTIN_DATASOURCE_NAMES: &[&str] = &[
    "-- Dashboard --",
    "-- Grafana --",
    "-- Mixed --",
    "grafana",
    "expr",
    "__expr__",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) enum FolderInventoryStatusKind {
    Missing,
    Matches,
    Mismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct FolderInventoryStatus {
    pub uid: String,
    pub expected_title: String,
    pub expected_parent_uid: Option<String>,
    pub expected_path: String,
    pub actual_title: Option<String>,
    pub actual_parent_uid: Option<String>,
    pub actual_path: Option<String>,
    pub kind: FolderInventoryStatusKind,
}

/// Run the dashboard CLI with an already configured client.
pub fn run_dashboard_cli_with_client(
    client: &JsonHttpClient,
    args: DashboardCliArgs,
) -> Result<()> {
    // Use this path when callers already resolved auth/org/client concerns and
    // only need dashboard command execution.
    match args.command {
        DashboardCommand::Browse(browse_args) => {
            let _ = browse::browse_dashboards_with_client(client, &browse_args)?;
            Ok(())
        }
        DashboardCommand::List(list_args) => {
            let _ = list_dashboards_with_client(client, &list_args)?;
            Ok(())
        }
        DashboardCommand::Export(export_args) => {
            let _ = export_dashboards_with_client(client, &export_args)?;
            Ok(())
        }
        DashboardCommand::Import(import_args) => {
            let _ = import_dashboards_with_client(client, &import_args)?;
            Ok(())
        }
        DashboardCommand::Delete(delete_args) => {
            let _ = delete::delete_dashboards_with_client(client, &delete_args)?;
            Ok(())
        }
        DashboardCommand::Diff(diff_args) => {
            // Diff is treated as a failing command when changes are found so shell
            // automation can gate on a non-zero exit status.
            let differences = diff_dashboards_with_client(client, &diff_args)?;
            if differences > 0 {
                return Err(message(format!(
                    "Dashboard diff found {} differing item(s).",
                    differences
                )));
            }
            Ok(())
        }
        DashboardCommand::InspectExport(inspect_args) => {
            // `--help-full` is handled here because inspect-export can be executed
            // both from the unified CLI and from direct dashboard parser paths.
            if inspect_args.help_full {
                print!("{}", render_inspect_export_help_full());
                return Ok(());
            }
            let _ = analyze_export_dir(&inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectLive(inspect_args) => {
            if inspect_args.help_full {
                print!("{}", render_inspect_live_help_full());
                return Ok(());
            }
            let _ = inspect_live_dashboards_with_client(client, &inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectVars(inspect_vars_args) => {
            inspect_dashboard_variables(&inspect_vars_args)
        }
        DashboardCommand::GovernanceGate(governance_gate_args) => {
            governance_gate::run_dashboard_governance_gate(&governance_gate_args)
        }
        DashboardCommand::Topology(topology_args) => run_dashboard_topology(&topology_args),
        DashboardCommand::Impact(impact_args) => run_dashboard_impact(&impact_args),
        DashboardCommand::ValidateExport(validate_args) => {
            run_dashboard_validate_export(&validate_args)
        }
        DashboardCommand::Screenshot(screenshot_args) => {
            capture_dashboard_screenshot(&screenshot_args)
        }
    }
}

/// Run the dashboard CLI after normalizing args and creating clients as needed.
pub fn run_dashboard_cli(args: DashboardCliArgs) -> Result<()> {
    // This is the main runtime entrypoint for dashboard commands from the binary:
    // normalize CLI args first, then decide whether each branch needs org fan-out,
    // one live client, or no client at all.
    let args = normalize_dashboard_cli_args(args);
    match args.command {
        DashboardCommand::Browse(browse_args) => {
            let _ = browse_dashboards_with_org_client(&browse_args)?;
            Ok(())
        }
        DashboardCommand::List(list_args) => {
            let _ = list_dashboards_with_org_clients(&list_args)?;
            Ok(())
        }
        DashboardCommand::Export(export_args) => {
            // Reject the "export nothing" shape early so lower layers can assume at
            // least one artifact variant will be written.
            if export_args.without_dashboard_raw && export_args.without_dashboard_prompt {
                return Err(message(
                    "At least one export variant must stay enabled. Remove --without-dashboard-raw or --without-dashboard-prompt.",
                ));
            }
            let _ = export_dashboards_with_org_clients(&export_args)?;
            Ok(())
        }
        DashboardCommand::Import(import_args) => {
            let _ = import::import_dashboards_with_org_clients(&import_args)?;
            Ok(())
        }
        DashboardCommand::Delete(delete_args) => {
            let _ = delete_dashboards_with_org_clients(&delete_args)?;
            Ok(())
        }
        DashboardCommand::Diff(diff_args) => {
            let client = build_http_client(&diff_args.common)?;
            let differences = diff_dashboards_with_client(&client, &diff_args)?;
            if differences > 0 {
                return Err(message(format!(
                    "Dashboard diff found {} differing item(s).",
                    differences
                )));
            }
            Ok(())
        }
        DashboardCommand::InspectExport(inspect_args) => {
            if inspect_args.help_full {
                print!("{}", render_inspect_export_help_full());
                return Ok(());
            }
            let _ = analyze_export_dir(&inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectLive(inspect_args) => {
            if inspect_args.help_full {
                print!("{}", render_inspect_live_help_full());
                return Ok(());
            }
            let client = build_http_client(&inspect_args.common)?;
            let _ = inspect_live_dashboards_with_client(&client, &inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectVars(inspect_vars_args) => {
            inspect_dashboard_variables(&inspect_vars_args)
        }
        DashboardCommand::GovernanceGate(governance_gate_args) => {
            governance_gate::run_dashboard_governance_gate(&governance_gate_args)
        }
        DashboardCommand::Topology(topology_args) => run_dashboard_topology(&topology_args),
        DashboardCommand::Impact(impact_args) => run_dashboard_impact(&impact_args),
        DashboardCommand::ValidateExport(validate_args) => {
            run_dashboard_validate_export(&validate_args)
        }
        DashboardCommand::Screenshot(screenshot_args) => {
            capture_dashboard_screenshot(&screenshot_args)
        }
    }
}

#[cfg(test)]
#[path = "dashboard_cli_rust_tests.rs"]
mod dashboard_cli_rust_tests;
#[cfg(test)]
#[path = "rust_tests.rs"]
mod dashboard_rust_tests;
#[cfg(test)]
#[path = "import_rust_tests.rs"]
mod import_rust_tests;
#[cfg(test)]
#[path = "inspect_export_rust_tests.rs"]
mod inspect_export_rust_tests;
#[cfg(test)]
#[path = "inspect_governance_document_rust_tests.rs"]
mod inspect_governance_document_rust_tests;
#[cfg(test)]
#[path = "inspect_governance_rust_tests.rs"]
mod inspect_governance_rust_tests;
#[cfg(test)]
#[path = "inspect_live_rust_tests.rs"]
mod inspect_live_rust_tests;
#[cfg(test)]
#[path = "inspect_vars_rust_tests.rs"]
mod inspect_vars_rust_tests;
#[cfg(test)]
#[path = "screenshot_rust_tests.rs"]
mod screenshot_rust_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
#[path = "topology_impact_document_rust_tests.rs"]
mod topology_impact_document_rust_tests;
#[cfg(test)]
#[path = "topology_impact_rust_tests.rs"]
mod topology_impact_rust_tests;
