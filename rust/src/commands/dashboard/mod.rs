//! Dashboard domain orchestrator for the unified dashboard CLI.
//!
//! This file is the boundary between unified command parsing and the lower-level
//! dashboard modules that implement export/import/live/inspect/screenshot logic.
use crate::common::Result;
use crate::http::JsonHttpClient;
use serde::Serialize;
use serde_json::Value;

// Keep the dashboard surface area split by concern. This file should stay focused
// on re-exports, shared constants, and top-level command dispatch.
mod analysis_source;
mod authoring;
mod browse;
mod browse_actions;
mod browse_edit_dialog;
mod browse_external_edit_dialog;
mod browse_history_dialog;
mod browse_input;
mod browse_render;
mod browse_state;
mod browse_support;
mod browse_terminal;
mod browse_tui;
mod cli_defs;
mod command_runner;
mod delete;
mod delete_interactive;
mod delete_render;
mod delete_support;
mod edit;
mod edit_external;
mod edit_live;
mod export;
mod facade_support;
mod files;
mod governance_gate;
mod governance_gate_rules;
mod governance_gate_tui;
mod governance_policy;
mod help;
mod history;
mod impact_tui;
mod import;
mod import_compare;
mod import_interactive;
mod import_interactive_context;
mod import_interactive_loader;
mod import_interactive_render;
mod import_interactive_review;
mod import_interactive_state;
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
mod inspect_dependency_render;
mod inspect_family;
mod inspect_governance;
mod inspect_live;
mod inspect_live_tui;
mod inspect_query;
mod inspect_render;
mod inspect_report;
mod inspect_summary;
mod inspect_workbench;
mod inspect_workbench_render;
mod inspect_workbench_state;
mod inspect_workbench_support;
mod list;
mod live;
mod live_project_status;
mod models;
mod project_status;
mod prompt;
mod prompt_helpers;
mod raw_to_prompt;
mod raw_to_prompt_output;
mod raw_to_prompt_plan;
mod raw_to_prompt_resolution;
mod raw_to_prompt_types;
mod run_inspect;
mod run_list;
mod screenshot;
mod serve;
mod source_loader;
mod topology;
mod topology_tui;
mod validate;
mod vars;

pub(crate) use authoring::{
    clone_live_dashboard_to_file_with_client, get_live_dashboard_to_file_with_client,
    patch_dashboard_file, publish_dashboard_with_client, render_dashboard_review_csv,
    render_dashboard_review_json, render_dashboard_review_table, render_dashboard_review_text,
    render_dashboard_review_yaml, review_dashboard_file as build_dashboard_review,
};
pub(crate) use cli_defs::materialize_dashboard_common_auth;
pub(crate) use cli_defs::{build_api_client, build_http_client_for_org_from_api};
pub use cli_defs::{
    build_auth_context, build_http_client, build_http_client_for_org, normalize_dashboard_cli_args,
    parse_cli_from, AnalyzeArgs, BrowseArgs, CloneLiveArgs, CommonCliArgs, DashboardAuthContext,
    DashboardCliArgs, DashboardCommand, DashboardHistoryArgs, DashboardHistorySubcommand,
    DashboardImportInputFormat, DashboardServeScriptFormat, DeleteArgs, DiffArgs, EditLiveArgs,
    ExportArgs, GetArgs, GovernanceGateArgs, GovernanceGateOutputFormat, GovernancePolicySource,
    HistoryDiffArgs, HistoryExportArgs, HistoryListArgs, HistoryOutputFormat, HistoryRestoreArgs,
    ImpactArgs, ImpactOutputFormat, ImportArgs, InspectExportArgs, InspectExportInputType,
    InspectExportReportFormat, InspectLiveArgs, InspectOutputFormat, InspectVarsArgs, ListArgs,
    PatchFileArgs, PublishArgs, RawToPromptArgs, RawToPromptLogFormat, RawToPromptOutputFormat,
    RawToPromptResolution, ReviewArgs, ScreenshotArgs, ScreenshotFullPageOutput,
    ScreenshotOutputFormat, ScreenshotTheme, ServeArgs, SimpleOutputFormat, TopologyArgs,
    TopologyOutputFormat, ValidateExportArgs, ValidationOutputFormat,
};
pub use command_runner::{
    execute_dashboard_inspect_export, execute_dashboard_inspect_live,
    execute_dashboard_inspect_vars, execute_dashboard_list,
};
pub use export::{build_export_variant_dirs, build_output_path, export_dashboards_with_client};
pub use help::{
    maybe_render_dashboard_help_full_from_os_args,
    maybe_render_dashboard_subcommand_help_from_os_args, render_inspect_export_help_full,
    render_inspect_live_help_full,
};
pub use import::{diff_dashboards_with_client, import_dashboards_with_client};
pub use list::list_dashboards_with_client;
pub use live::{
    delete_dashboard_request, delete_folder_request, fetch_dashboard, import_dashboard_request,
    list_dashboard_summaries, list_datasources,
};
pub use prompt::build_external_export_document;
pub(crate) use raw_to_prompt::run_raw_to_prompt;
pub use screenshot::capture_dashboard_screenshot;
pub(crate) use source_loader::{
    infer_dashboard_workspace_root, load_dashboard_source, resolve_dashboard_workspace_variant_dir,
    LoadedDashboardSource,
};

#[allow(unused_imports)]
pub(crate) use facade_support::{
    build_datasource_inventory_record, build_folder_path, build_live_dashboard_domain_status,
    build_live_dashboard_domain_status_from_inputs, collect_folder_inventory_with_request,
    collect_live_dashboard_project_status_inputs_with_request,
    fetch_dashboard_if_exists_with_request, fetch_dashboard_permissions_with_request,
    fetch_dashboard_with_request, fetch_folder_if_exists_with_request,
    fetch_folder_permissions_with_request, format_folder_inventory_status_line,
    import_dashboard_request_with_request, list_dashboard_summaries_with_request,
    list_datasources_with_request, load_builtin_governance_policy, load_governance_policy,
    load_governance_policy_file, load_governance_policy_source, LiveDashboardProjectStatusInputs,
};
#[allow(unused_imports)]
pub(crate) use files::{
    build_dashboard_index_item, build_export_metadata, build_import_payload,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file, resolve_dashboard_import_source,
    write_dashboard, write_json_document, DashboardRepoLayoutKind, DashboardSourceKind,
    ResolvedDashboardImportSource,
};
pub(crate) use inspect::build_export_inspection_summary_for_variant;
pub(crate) use inspect_live::TempInspectDir;
pub(crate) use inspect_report::ExportInspectionQueryRow;
pub(crate) use inspect_summary::{
    build_export_inspection_summary_document, ExportInspectionSummary,
};
pub(crate) use models::{
    DashboardExportRootManifest, DashboardExportRootScopeKind, DashboardIndexItem,
    DatasourceInventoryItem, ExportDatasourceUsageSummary, ExportMetadata, ExportOrgSummary,
    FolderInventoryItem, RootExportIndex, RootExportVariants, VariantIndexEntry,
};
pub(crate) use project_status::build_dashboard_domain_status;
pub(crate) use prompt::{
    build_datasource_catalog, collect_datasource_refs, datasource_type_alias,
    is_builtin_datasource_ref, is_placeholder_string, lookup_datasource,
    resolve_datasource_type_alias,
};

#[cfg(not(feature = "tui"))]
pub(crate) fn tui_not_built<T>(action: &str) -> Result<T> {
    Err(crate::common::message(format!(
        "Dashboard {action} requires TUI support, but it was not built in."
    )))
}

// Shared dashboard defaults and export filenames used across export/import/live flows.
pub const DEFAULT_URL: &str = "http://localhost:3000";
pub const DEFAULT_TIMEOUT: u64 = 30;
pub const DEFAULT_PAGE_SIZE: usize = 500;
pub const DEFAULT_EXPORT_DIR: &str = "dashboards";
pub const RAW_EXPORT_SUBDIR: &str = "raw";
pub const PROMPT_EXPORT_SUBDIR: &str = "prompt";
pub const PROVISIONING_EXPORT_SUBDIR: &str = "provisioning";
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

#[derive(Clone, Debug, PartialEq)]
pub struct DashboardWebRunOutput {
    pub document: Value,
    pub text_lines: Vec<String>,
}

/// Run the dashboard CLI with an already configured client.
/// Thin wrapper that keeps the public module surface stable while the runtime lives in `command_runner`.
pub fn run_dashboard_cli_with_client(
    client: &JsonHttpClient,
    args: DashboardCliArgs,
) -> Result<()> {
    command_runner::run_dashboard_cli_with_client(client, args)
}

/// Run the dashboard CLI after normalizing args and creating clients as needed.
/// Thin wrapper that exposes the dashboard runtime boundary from the module root.
pub fn run_dashboard_cli(args: DashboardCliArgs) -> Result<()> {
    command_runner::run_dashboard_cli(args)
}

#[cfg(test)]
#[path = "authoring_rust_tests.rs"]
mod authoring_rust_tests;
#[cfg(test)]
#[path = "dashboard_cli_rust_tests.rs"]
mod dashboard_cli_rust_tests;
#[cfg(test)]
#[path = "rust_tests.rs"]
mod dashboard_rust_tests;
#[cfg(test)]
#[path = "history_cli_rust_tests.rs"]
mod history_cli_rust_tests;
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
#[path = "raw_to_prompt_rust_tests.rs"]
mod raw_to_prompt_rust_tests;
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
