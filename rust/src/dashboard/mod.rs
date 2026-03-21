//! Dashboard domain orchestrator.
//!
//! Purpose:
//! - Own the dashboard command surface (`list`, `export`,
//!   `import`, `diff`, `inspect`).
//! - Re-export shared parser and helper APIs from sibling modules for consumers.
//! - Keep transport setup, normalization, and execution branching in this module.
//!
//! Flow:
//! - Build command args via `cli_defs` and normalize in `normalize_dashboard_cli_args`
//!   where needed.
//! - Construct `JsonHttpClient` in command-specific branches.
//! - Delegate each branch to focused dashboard submodules (`list`, `export`, `import`, `diff`,
//!   `inspect`).
//!
//! Caveats:
//! - Avoid embedding HTTP retry/backoff logic; that belongs to `common`/`http` or submodules.
//! - Keep this module free of command-specific domain details beyond orchestration and normalization.
use crate::common::{message, Result};
use crate::http::JsonHttpClient;

mod cli_defs;
mod export;
mod files;
mod help;
mod import;
mod inspect;
mod inspect_analyzer_flux;
mod inspect_analyzer_loki;
mod inspect_analyzer_prometheus;
mod inspect_analyzer_search;
mod inspect_analyzer_sql;
mod inspect_governance;
mod inspect_render;
mod inspect_report;
mod inspect_summary;
mod list;
mod live;
mod models;
mod prompt;
mod screenshot;
mod vars;

pub use cli_defs::{
    build_auth_context, build_http_client, build_http_client_for_org, normalize_dashboard_cli_args,
    parse_cli_from, CommonCliArgs, DashboardAuthContext, DashboardCliArgs, DashboardCommand,
    DiffArgs, ExportArgs, ImportArgs, InspectExportArgs, InspectExportReportFormat,
    InspectLiveArgs, InspectOutputFormat, InspectVarsArgs, ListArgs, ScreenshotArgs,
    ScreenshotFullPageOutput, ScreenshotOutputFormat, ScreenshotTheme, SimpleOutputFormat,
};
pub use export::{build_export_variant_dirs, build_output_path, export_dashboards_with_client};
pub use help::{
    maybe_render_dashboard_help_full_from_os_args, render_inspect_export_help_full,
    render_inspect_live_help_full,
};
pub use import::{diff_dashboards_with_client, import_dashboards_with_client};
pub use list::list_dashboards_with_client;
pub use live::{
    fetch_dashboard, import_dashboard_request, list_dashboard_summaries, list_datasources,
};
pub use prompt::build_external_export_document;

use export::export_dashboards_with_org_clients;
use inspect::analyze_export_dir;
use list::list_dashboards_with_org_clients;
use screenshot::capture_dashboard_screenshot;
use vars::inspect_dashboard_variables;

#[cfg(test)]
pub(crate) use export::{
    export_dashboards_with_request, format_export_progress_line, format_export_verbose_line,
};
pub(crate) use files::{
    build_dashboard_index_item, build_export_metadata, build_import_payload,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file, write_dashboard,
    write_json_document,
};
#[cfg(test)]
pub(crate) use import::{
    build_import_auth_context, describe_dashboard_import_mode, diff_dashboards_with_request,
    format_import_progress_line, format_import_verbose_line, import_dashboards_with_org_clients,
    import_dashboards_with_request, render_folder_inventory_dry_run_table,
    render_import_dry_run_json, render_import_dry_run_table,
};
pub(crate) use inspect::inspect_live_dashboards_with_request;
#[cfg(test)]
pub(crate) use inspect::{
    apply_query_report_filters, build_export_inspection_query_report,
    build_export_inspection_summary, build_export_inspection_summary_rows, dispatch_query_analysis,
    prepare_inspect_export_import_dir, resolve_query_analyzer_family,
    resolve_query_analyzer_family_from_datasource_type,
    resolve_query_analyzer_family_from_query_signature, validate_inspect_export_report_args,
    QueryExtractionContext, DATASOURCE_FAMILY_FLUX, DATASOURCE_FAMILY_LOKI,
    DATASOURCE_FAMILY_PROMETHEUS, DATASOURCE_FAMILY_SEARCH, DATASOURCE_FAMILY_SQL,
    DATASOURCE_FAMILY_TRACING, DATASOURCE_FAMILY_UNKNOWN,
};
#[cfg(test)]
pub(crate) use inspect_governance::{
    build_export_inspection_governance_document, normalize_family_name,
    render_governance_table_report,
};
#[cfg(test)]
pub(crate) use inspect_render::{
    render_csv, render_grouped_query_report, render_grouped_query_table_report,
};
#[cfg(test)]
pub(crate) use inspect_report::normalize_query_report;
#[cfg(test)]
pub(crate) use inspect_report::resolve_report_column_ids;
pub(crate) use inspect_report::{
    build_export_inspection_query_report_document, build_query_report,
    refresh_filtered_query_report_summary, render_query_report_column, report_column_header,
    report_format_supports_columns, resolve_report_column_ids_for_format,
    ExportInspectionQueryReport, ExportInspectionQueryRow,
};
#[cfg(test)]
pub(crate) use inspect_report::{
    QueryReportSummary, DEFAULT_REPORT_COLUMN_IDS, SUPPORTED_REPORT_COLUMN_IDS,
};
pub(crate) use inspect_summary::{
    build_export_inspection_summary_document, DatasourceInventorySummary, ExportDatasourceUsage,
    ExportFolderUsage, ExportInspectionSummary, MixedDashboardSummary,
};
#[cfg(test)]
pub(crate) use list::{
    attach_dashboard_folder_paths_with_request, collect_dashboard_source_metadata,
    format_dashboard_summary_line, list_dashboards_with_request, render_dashboard_summary_csv,
    render_dashboard_summary_json, render_dashboard_summary_table,
};
#[cfg(test)]
pub(crate) use live::build_folder_inventory_status;
#[cfg(test)]
pub(crate) use live::collect_folder_inventory_statuses_with_request;
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
#[cfg(test)]
pub(crate) use screenshot::{
    build_dashboard_capture_url, infer_screenshot_output_format, resolve_manifest_title,
    validate_screenshot_args,
};
#[cfg(test)]
pub(crate) use vars::extract_dashboard_variables;

/// Constant for default url.
pub const DEFAULT_URL: &str = "http://localhost:3000";
/// Constant for default timeout.
pub const DEFAULT_TIMEOUT: u64 = 30;
/// Constant for default page size.
pub const DEFAULT_PAGE_SIZE: usize = 500;
/// Constant for default export dir.
pub const DEFAULT_EXPORT_DIR: &str = "dashboards";
/// Constant for raw export subdir.
pub const RAW_EXPORT_SUBDIR: &str = "raw";
/// Constant for prompt export subdir.
pub const PROMPT_EXPORT_SUBDIR: &str = "prompt";
/// Constant for default import message.
pub const DEFAULT_IMPORT_MESSAGE: &str = "Imported by grafana-utils";
/// Constant for default dashboard title.
pub const DEFAULT_DASHBOARD_TITLE: &str = "dashboard";
/// Constant for default folder title.
pub const DEFAULT_FOLDER_TITLE: &str = "General";
/// Constant for default folder uid.
pub const DEFAULT_FOLDER_UID: &str = "general";
/// Constant for default org id.
pub const DEFAULT_ORG_ID: &str = "1";
/// Constant for default org name.
pub const DEFAULT_ORG_NAME: &str = "Main Org.";
/// Constant for default unknown uid.
pub const DEFAULT_UNKNOWN_UID: &str = "unknown";
/// Constant for export metadata filename.
pub const EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
/// Constant for tool schema version.
pub const TOOL_SCHEMA_VERSION: i64 = 1;
/// Constant for root index kind.
pub const ROOT_INDEX_KIND: &str = "grafana-utils-dashboard-export-index";
/// Constant for folder inventory filename.
pub const FOLDER_INVENTORY_FILENAME: &str = "folders.json";
/// Constant for datasource inventory filename.
pub const DATASOURCE_INVENTORY_FILENAME: &str = "datasources.json";
/// Constant for dashboard/folder permission bundle filename.
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

/// Enum definition for FolderInventoryStatusKind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FolderInventoryStatusKind {
    Missing,
    Matches,
    Mismatch,
}

/// Struct definition for FolderInventoryStatus.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Execution path for callers that already own a configured client.
/// Useful for tests that want to inject transport behavior and avoid side-effects.
pub fn run_dashboard_cli_with_client(
    client: &JsonHttpClient,
    args: DashboardCliArgs,
) -> Result<()> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: common.rs:message, dashboard_export.rs:export_dashboards_with_client, dashboard_help.rs:render_inspect_export_help_full, dashboard_help.rs:render_inspect_live_help_full, dashboard_import.rs:diff_dashboards_with_client, dashboard_import.rs:import_dashboards_with_client, dashboard_list.rs:list_dashboards_with_client, dashboard_screenshot.rs:capture_dashboard_screenshot

    match args.command {
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
        DashboardCommand::Diff(diff_args) => {
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
            let _ = inspect_live_dashboards_with_request(
                |method, path, params, payload| client.request_json(method, path, params, payload),
                &inspect_args,
            )?;
            Ok(())
        }
        DashboardCommand::InspectVars(inspect_vars_args) => {
            inspect_dashboard_variables(&inspect_vars_args)
        }
        DashboardCommand::Screenshot(screenshot_args) => {
            capture_dashboard_screenshot(&screenshot_args)
        }
    }
}

/// Dashboard dispatcher for runtime execution.
///
/// Flow:
/// 1) normalize args, 2) build or reuse client(s), 3) delegate to domain handlers.
///
/// Errors are surfaced directly to the CLI caller for consistent exit behavior.
pub fn run_dashboard_cli(args: DashboardCliArgs) -> Result<()> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: common.rs:message, dashboard_cli_defs.rs:normalize_dashboard_cli_args, dashboard_help.rs:render_inspect_export_help_full, dashboard_help.rs:render_inspect_live_help_full, dashboard_import.rs:diff_dashboards_with_client, dashboard_screenshot.rs:capture_dashboard_screenshot

    let args = normalize_dashboard_cli_args(args);
    match args.command {
        DashboardCommand::List(list_args) => {
            let _ = list_dashboards_with_org_clients(&list_args)?;
            Ok(())
        }
        DashboardCommand::Export(export_args) => {
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
            let _ = inspect_live_dashboards_with_request(
                |method, path, params, payload| client.request_json(method, path, params, payload),
                &inspect_args,
            )?;
            Ok(())
        }
        DashboardCommand::InspectVars(inspect_vars_args) => {
            inspect_dashboard_variables(&inspect_vars_args)
        }
        DashboardCommand::Screenshot(screenshot_args) => {
            capture_dashboard_screenshot(&screenshot_args)
        }
    }
}

#[cfg(test)]
#[path = "rust_tests.rs"]
mod dashboard_rust_tests;
