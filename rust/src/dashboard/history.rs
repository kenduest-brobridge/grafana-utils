#![cfg_attr(not(any(feature = "tui", test)), allow(dead_code))]
#![allow(unused_imports)]

pub(crate) use crate::common::{string_field, tool_version};

#[path = "history_artifacts.rs"]
mod history_artifacts;
#[path = "history_diff.rs"]
mod history_diff;
#[path = "history_live.rs"]
mod history_live;
#[path = "history_render.rs"]
mod history_render;
#[path = "history_restore.rs"]
mod history_restore;
#[path = "history_types.rs"]
mod history_types;

pub(crate) use super::authoring::{
    clone_live_dashboard_to_file_with_client, get_live_dashboard_to_file_with_client,
    patch_dashboard_file, publish_dashboard_with_client, render_dashboard_review_csv,
    render_dashboard_review_json, render_dashboard_review_table, render_dashboard_review_text,
    render_dashboard_review_yaml, review_dashboard_file as build_dashboard_review,
};
pub(crate) use super::cli_defs::materialize_dashboard_common_auth;
pub(crate) use super::cli_defs::{build_api_client, build_http_client_for_org_from_api};
pub use super::cli_defs::{
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

pub(crate) use super::command_runner::{
    execute_dashboard_inspect_export, execute_dashboard_inspect_live,
    execute_dashboard_inspect_vars, execute_dashboard_list,
};
pub(crate) use super::export::{
    build_export_variant_dirs, build_output_path, export_dashboards_with_client,
};
pub(crate) use super::facade_support::{
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
pub(crate) use super::files::{
    build_dashboard_index_item, build_export_metadata, build_import_payload,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file, resolve_dashboard_import_source,
    write_dashboard, write_json_document, DashboardRepoLayoutKind, DashboardSourceKind,
    ResolvedDashboardImportSource,
};
pub(crate) use super::help::{
    maybe_render_dashboard_help_full_from_os_args,
    maybe_render_dashboard_subcommand_help_from_os_args, render_inspect_export_help_full,
    render_inspect_live_help_full,
};
pub(crate) use super::import::{diff_dashboards_with_client, import_dashboards_with_client};
use super::import_compare;
pub(crate) use super::inspect::build_export_inspection_summary_for_variant;
pub(crate) use super::inspect_live::TempInspectDir;
pub(crate) use super::inspect_report::ExportInspectionQueryRow;
pub(crate) use super::inspect_summary::{
    build_export_inspection_summary_document, ExportInspectionSummary,
};
pub(crate) use super::list::list_dashboards_with_client;
pub(crate) use super::live::{
    delete_dashboard_request, delete_folder_request, fetch_dashboard, import_dashboard_request,
    list_dashboard_summaries, list_datasources,
};
pub(crate) use super::models::{
    DashboardExportRootManifest, DashboardExportRootScopeKind, DashboardIndexItem,
    DatasourceInventoryItem, ExportDatasourceUsageSummary, ExportMetadata, ExportOrgSummary,
    FolderInventoryItem, RootExportIndex, RootExportVariants, VariantIndexEntry,
};
pub(crate) use super::project_status::build_dashboard_domain_status;
pub(crate) use super::prompt::build_external_export_document;
pub(crate) use super::prompt::{
    build_datasource_catalog, collect_datasource_refs, datasource_type_alias,
    is_builtin_datasource_ref, is_placeholder_string, lookup_datasource,
    resolve_datasource_type_alias,
};
pub(crate) use super::raw_to_prompt::run_raw_to_prompt;
pub use super::screenshot::capture_dashboard_screenshot;
pub(crate) use super::source_loader::{
    infer_dashboard_workspace_root, load_dashboard_source, resolve_dashboard_workspace_variant_dir,
    LoadedDashboardSource,
};

pub(crate) use history_artifacts::{
    build_dashboard_history_inventory_document,
    build_dashboard_history_list_document_from_export_document,
    ensure_history_artifact_uid_matches, load_dashboard_history_export_document,
    load_history_artifact_for_uid, load_history_artifacts_from_import_dir,
    run_dashboard_history_list_from_import_dir,
};
pub(crate) use history_diff::{
    build_dashboard_history_diff_document_with_request, run_dashboard_history_diff,
};
pub(crate) use history_live::{
    build_dashboard_history_export_document_with_request,
    build_dashboard_history_list_document_with_request,
    build_dashboard_restore_preview_with_request, export_dashboard_history_with_request,
    fetch_dashboard_history_version_data_with_request,
    list_dashboard_history_versions_with_request, run_dashboard_history_list,
};
pub(crate) use history_restore::{
    restore_dashboard_history_version_with_request,
    restore_dashboard_history_version_with_request_and_message, run_dashboard_history_restore,
};
pub(crate) use history_types::{
    DashboardHistoryDiffDocument, DashboardHistoryExportDocument, DashboardHistoryExportVersion,
    DashboardHistoryInventoryDocument, DashboardHistoryInventoryItem, DashboardHistoryListDocument,
    DashboardHistoryRestoreDocument, DashboardHistoryVersion, DashboardRestorePreview,
    HistoryDiffSource, LocalHistoryArtifact, ResolvedHistoryDiffSide,
    BROWSE_HISTORY_RESTORE_MESSAGE, DASHBOARD_HISTORY_DIFF_KIND, DASHBOARD_HISTORY_EXPORT_KIND,
    DASHBOARD_HISTORY_INVENTORY_KIND, DASHBOARD_HISTORY_LIST_KIND, DASHBOARD_HISTORY_RESTORE_KIND,
    DASHBOARD_HISTORY_RESTORE_MESSAGE, HISTORY_RESTORE_PROMPT_LIMIT,
};

pub(crate) use super::{
    run_dashboard_cli, run_dashboard_cli_with_client, DashboardWebRunOutput, FolderInventoryStatus,
    FolderInventoryStatusKind, DASHBOARD_PERMISSION_BUNDLE_FILENAME, DATASOURCE_INVENTORY_FILENAME,
    DEFAULT_DASHBOARD_TITLE, DEFAULT_EXPORT_DIR, DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID,
    DEFAULT_IMPORT_MESSAGE, DEFAULT_ORG_ID, DEFAULT_ORG_NAME, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT,
    DEFAULT_UNKNOWN_UID, DEFAULT_URL, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    PROMPT_EXPORT_SUBDIR, PROVISIONING_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR, ROOT_INDEX_KIND,
    TOOL_SCHEMA_VERSION,
};

#[cfg(not(feature = "tui"))]
pub(crate) use super::tui_not_built;

#[cfg(test)]
use super::test_support;

#[cfg(test)]
mod history_test_support {
    pub use super::*;
}
