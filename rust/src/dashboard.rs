//! Dashboard domain orchestrator.
//!
//! Purpose:
//! - Own the dashboard command surface (`list`, `list-data-sources`, `export`,
//!   `import`, `diff`, `inspect`).
//! - Re-export shared parser and helper APIs from sibling modules for consumers.
//! - Keep transport setup, normalization, and execution branching in this module.
//!
//! Flow:
//! - Build command args via `dashboard_cli_defs` and normalize in `normalize_dashboard_cli_args`
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

#[path = "dashboard_cli_defs.rs"]
mod dashboard_cli_defs;
#[path = "dashboard_export.rs"]
mod dashboard_export;
#[path = "dashboard_files.rs"]
mod dashboard_files;
#[path = "dashboard_help.rs"]
mod dashboard_help;
#[path = "dashboard_import.rs"]
mod dashboard_import;
#[path = "dashboard_inspect.rs"]
mod dashboard_inspect;
#[path = "dashboard_inspect_analyzer_flux.rs"]
mod dashboard_inspect_analyzer_flux;
#[path = "dashboard_inspect_analyzer_loki.rs"]
mod dashboard_inspect_analyzer_loki;
#[path = "dashboard_inspect_analyzer_prometheus.rs"]
mod dashboard_inspect_analyzer_prometheus;
#[path = "dashboard_inspect_analyzer_sql.rs"]
mod dashboard_inspect_analyzer_sql;
#[path = "dashboard_inspect_governance.rs"]
mod dashboard_inspect_governance;
#[path = "dashboard_inspect_render.rs"]
mod dashboard_inspect_render;
#[path = "dashboard_inspect_report.rs"]
mod dashboard_inspect_report;
#[path = "dashboard_inspect_summary.rs"]
mod dashboard_inspect_summary;
#[path = "dashboard_list.rs"]
mod dashboard_list;
#[path = "dashboard_live.rs"]
mod dashboard_live;
#[path = "dashboard_models.rs"]
mod dashboard_models;
#[path = "dashboard_prompt.rs"]
mod dashboard_prompt;

pub use dashboard_cli_defs::{
    build_auth_context, build_http_client, build_http_client_for_org, normalize_dashboard_cli_args,
    parse_cli_from, CommonCliArgs, DashboardAuthContext, DashboardCliArgs, DashboardCommand,
    DiffArgs, ExportArgs, ImportArgs, InspectExportArgs, InspectExportReportFormat,
    InspectLiveArgs, InspectOutputFormat, ListArgs, ListDataSourcesArgs,
};
pub use dashboard_export::{
    build_export_variant_dirs, build_output_path, export_dashboards_with_client,
};
pub use dashboard_help::{
    maybe_render_dashboard_help_full_from_os_args, render_inspect_export_help_full,
    render_inspect_live_help_full,
};
pub use dashboard_import::{diff_dashboards_with_client, import_dashboards_with_client};
pub use dashboard_list::{list_dashboards_with_client, list_data_sources_with_client};
pub use dashboard_live::{
    fetch_dashboard, import_dashboard_request, list_dashboard_summaries, list_datasources,
};
pub use dashboard_prompt::build_external_export_document;

use dashboard_export::export_dashboards_with_org_clients;
use dashboard_inspect::analyze_export_dir;
use dashboard_list::list_dashboards_with_org_clients;

#[cfg(test)]
pub(crate) use dashboard_export::{
    export_dashboards_with_request, format_export_progress_line, format_export_verbose_line,
};
pub(crate) use dashboard_files::{
    build_dashboard_index_item, build_export_metadata, build_import_payload,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file, write_dashboard,
    write_json_document,
};
#[cfg(test)]
pub(crate) use dashboard_import::{
    build_import_auth_context, describe_dashboard_import_mode, diff_dashboards_with_request,
    format_import_progress_line, format_import_verbose_line, import_dashboards_with_org_clients,
    import_dashboards_with_request, render_folder_inventory_dry_run_table,
    render_import_dry_run_json, render_import_dry_run_table,
};
pub(crate) use dashboard_inspect::inspect_live_dashboards_with_request;
#[cfg(test)]
pub(crate) use dashboard_inspect::{
    apply_query_report_filters, build_export_inspection_query_report,
    build_export_inspection_summary, validate_inspect_export_report_args,
};
#[cfg(test)]
pub(crate) use dashboard_inspect_governance::{
    build_export_inspection_governance_document, render_governance_table_report,
};
#[cfg(test)]
pub(crate) use dashboard_inspect_render::{
    render_csv, render_grouped_query_report, render_grouped_query_table_report,
};
#[cfg(test)]
pub(crate) use dashboard_inspect_report::normalize_query_report;
pub(crate) use dashboard_inspect_report::{
    build_export_inspection_query_report_document, build_query_report,
    refresh_filtered_query_report_summary, render_query_report_column, report_column_header,
    report_format_supports_columns, resolve_report_column_ids, ExportInspectionQueryReport,
    ExportInspectionQueryRow,
};
#[cfg(test)]
pub(crate) use dashboard_inspect_report::{QueryReportSummary, DEFAULT_REPORT_COLUMN_IDS};
pub(crate) use dashboard_inspect_summary::{
    build_export_inspection_summary_document, DatasourceInventorySummary, ExportDatasourceUsage,
    ExportFolderUsage, ExportInspectionSummary, MixedDashboardSummary,
};
#[cfg(test)]
pub(crate) use dashboard_list::{
    attach_dashboard_folder_paths_with_request, collect_dashboard_source_metadata,
    format_dashboard_summary_line, format_data_source_line, list_dashboards_with_request,
    list_data_sources_with_request, render_dashboard_summary_csv, render_dashboard_summary_json,
    render_dashboard_summary_table, render_data_source_csv, render_data_source_json,
    render_data_source_table,
};
#[cfg(test)]
pub(crate) use dashboard_live::build_folder_inventory_status;
pub(crate) use dashboard_live::{
    build_datasource_inventory_record, build_folder_path,
    collect_folder_inventory_statuses_with_request, collect_folder_inventory_with_request,
    ensure_folder_inventory_entry_with_request, fetch_dashboard_if_exists_with_request,
    fetch_dashboard_with_request, fetch_folder_if_exists_with_request,
    format_folder_inventory_status_line, import_dashboard_request_with_request,
    list_dashboard_summaries_with_request, list_datasources_with_request,
};
pub(crate) use dashboard_models::{
    DashboardIndexItem, DatasourceInventoryItem, ExportMetadata, FolderInventoryItem,
    RootExportIndex, RootExportVariants, VariantIndexEntry,
};
pub(crate) use dashboard_prompt::{
    build_datasource_catalog, collect_datasource_refs, datasource_type_alias,
    is_builtin_datasource_ref, is_placeholder_string, lookup_datasource,
    resolve_datasource_type_alias,
};

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
const BUILTIN_DATASOURCE_TYPES: &[&str] = &["__expr__", "grafana"];
const BUILTIN_DATASOURCE_NAMES: &[&str] = &[
    "-- Dashboard --",
    "-- Grafana --",
    "-- Mixed --",
    "grafana",
    "expr",
    "__expr__",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FolderInventoryStatusKind {
    Missing,
    Matches,
    Mismatch,
}

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
    match args.command {
        DashboardCommand::List(list_args) => {
            let _ = list_dashboards_with_client(client, &list_args)?;
            Ok(())
        }
        DashboardCommand::ListDataSources(list_data_sources_args) => {
            let _ = list_data_sources_with_client(client, &list_data_sources_args)?;
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
    }
}

/// Dashboard dispatcher for runtime execution.
///
/// Flow:
/// 1) normalize args, 2) build or reuse client(s), 3) delegate to domain handlers.
///
/// Errors are surfaced directly to the CLI caller for consistent exit behavior.
pub fn run_dashboard_cli(args: DashboardCliArgs) -> Result<()> {
    let args = normalize_dashboard_cli_args(args);
    match args.command {
        DashboardCommand::List(list_args) => {
            let _ = list_dashboards_with_org_clients(&list_args)?;
            Ok(())
        }
        DashboardCommand::ListDataSources(list_data_sources_args) => {
            let client = build_http_client(&list_data_sources_args.common)?;
            let _ = list_data_sources_with_client(&client, &list_data_sources_args)?;
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
            let _ = dashboard_import::import_dashboards_with_org_clients(&import_args)?;
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
    }
}

#[cfg(test)]
#[path = "dashboard_rust_tests.rs"]
mod dashboard_rust_tests;
