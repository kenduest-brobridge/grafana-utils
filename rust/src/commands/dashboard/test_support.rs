#![allow(unused_imports)]

pub(super) use crate::common::message;
pub(super) use crate::dashboard::browse_support::build_dashboard_browse_document;
pub(super) use crate::dashboard::cli_defs::{
    build_auth_context, build_http_client, build_http_client_for_org, normalize_dashboard_cli_args,
    parse_cli_from, BrowseArgs, CommonCliArgs, DashboardAuthContext, DashboardCliArgs,
    DashboardCommand, DashboardHistorySubcommand, DashboardImportInputFormat, DiffArgs, ExportArgs,
    GovernanceGateArgs, GovernanceGateOutputFormat, GovernancePolicySource, HistoryDiffArgs,
    HistoryExportArgs, HistoryListArgs, HistoryOutputFormat, HistoryRestoreArgs, ImpactArgs,
    ImpactOutputFormat, ImportArgs, InspectExportArgs, InspectExportReportFormat, InspectLiveArgs,
    InspectOutputFormat, InspectVarsArgs, ListArgs, RawToPromptArgs, RawToPromptLogFormat,
    RawToPromptOutputFormat, RawToPromptResolution, ScreenshotArgs, ScreenshotFullPageOutput,
    ScreenshotOutputFormat, ScreenshotTheme, SimpleOutputFormat, TopologyArgs,
    TopologyOutputFormat, ValidateExportArgs, ValidationOutputFormat,
};
pub(super) use crate::dashboard::export::{
    build_export_variant_dirs, build_output_path, export_dashboards_with_client,
    export_dashboards_with_request, format_export_progress_line, format_export_verbose_line,
};
pub(super) use crate::dashboard::files::{
    build_dashboard_index_item, build_export_metadata, build_import_payload,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    discover_dashboard_files, extract_dashboard_object, load_dashboard_export_root_manifest,
    load_datasource_inventory, load_export_metadata, load_folder_inventory, load_json_file,
    resolve_dashboard_export_root, write_dashboard, write_json_document,
};
pub(super) use crate::dashboard::governance_gate::{
    evaluate_dashboard_governance_gate, render_dashboard_governance_gate_result,
    run_dashboard_governance_gate, DashboardGovernanceGateFinding, DashboardGovernanceGateResult,
    DashboardGovernanceGateSummary,
};
pub(super) use crate::dashboard::governance_gate_tui::{
    build_governance_gate_tui_groups, build_governance_gate_tui_items,
};
pub(super) use crate::dashboard::governance_policy::{
    load_builtin_governance_policy, load_governance_policy, load_governance_policy_file,
    load_governance_policy_source,
};
pub(super) use crate::dashboard::help::{
    maybe_render_dashboard_help_full_from_os_args, render_summary_export_help_full,
    render_summary_live_help_full,
};
pub(super) use crate::dashboard::history::{
    build_dashboard_history_diff_document_with_request,
    build_dashboard_history_export_document_with_request,
    build_dashboard_history_list_document_with_request, export_dashboard_history_with_request,
    restore_dashboard_history_version_with_request_and_message, run_dashboard_history_diff,
    run_dashboard_history_list, run_dashboard_history_restore, DashboardHistoryExportDocument,
    DashboardHistoryListDocument, DashboardHistoryRestoreDocument, DashboardHistoryVersion,
    DASHBOARD_HISTORY_DIFF_KIND, DASHBOARD_HISTORY_EXPORT_KIND, DASHBOARD_HISTORY_LIST_KIND,
    DASHBOARD_HISTORY_RESTORE_KIND,
};
pub(super) use crate::dashboard::impact_tui::{build_impact_tui_groups, filter_impact_tui_items};
pub(super) use crate::dashboard::import::{
    build_import_auth_context, describe_dashboard_import_mode, diff_dashboards_with_client,
    diff_dashboards_with_request, format_import_progress_line, format_import_verbose_line,
    import_dashboards_with_client, import_dashboards_with_org_clients,
    import_dashboards_with_request, render_folder_inventory_dry_run_table,
    render_import_dry_run_json, render_import_dry_run_table,
};
pub(super) use crate::dashboard::inspect::{
    analyze_export_dir, apply_query_report_filters, build_export_inspection_query_report,
    build_export_inspection_summary, prepare_inspect_export_import_dir,
    resolve_query_analyzer_family, validate_inspect_export_report_args,
};
pub(in crate::dashboard) mod import {
    pub(in crate::dashboard) use crate::dashboard::import::*;
}
pub(in crate::dashboard) mod inspect_governance {
    pub(in crate::dashboard) use crate::dashboard::inspect_governance::*;
}
pub(super) use crate::dashboard::inspect_family::normalize_family_name;
pub(super) use crate::dashboard::inspect_governance::{
    build_export_inspection_governance_document, render_governance_table_report,
};
pub(super) use crate::dashboard::inspect_live::{
    inspect_live_dashboards_with_client, inspect_live_dashboards_with_request,
    snapshot_live_dashboard_export_with_fetcher,
};
pub(super) use crate::dashboard::inspect_live_tui::{
    build_inspect_live_tui_groups, filter_inspect_live_tui_items,
};
pub(super) use crate::dashboard::inspect_query::{
    dispatch_query_analysis, resolve_query_analyzer_family_from_datasource_type,
    resolve_query_analyzer_family_from_query_signature, QueryExtractionContext,
    DATASOURCE_FAMILY_FLUX, DATASOURCE_FAMILY_LOKI, DATASOURCE_FAMILY_PROMETHEUS,
    DATASOURCE_FAMILY_SEARCH, DATASOURCE_FAMILY_SQL, DATASOURCE_FAMILY_TRACING,
    DATASOURCE_FAMILY_UNKNOWN,
};
pub(super) use crate::dashboard::inspect_render::{
    render_csv, render_grouped_query_report, render_grouped_query_table_report,
};
pub(super) use crate::dashboard::inspect_report::{
    build_export_inspection_query_report_document, normalize_query_report,
    render_query_report_column, report_column_header, report_format_supports_columns,
    resolve_report_column_ids, resolve_report_column_ids_for_format, ExportInspectionQueryReport,
    ExportInspectionQueryReportDocument, ExportInspectionQueryReportJsonSummary,
    ExportInspectionQueryRow, QueryReportSummary, DEFAULT_REPORT_COLUMN_IDS,
    SUPPORTED_REPORT_COLUMN_IDS,
};
pub(super) use crate::dashboard::inspect_summary::{
    build_export_inspection_summary_document, build_export_inspection_summary_rows,
    DatasourceInventorySummary, ExportInspectionSummary, ExportInspectionSummaryDocument,
    ExportInspectionSummaryJsonSummary, MixedDashboardSummary,
};
pub(super) use crate::dashboard::inspect_workbench_support::build_inspect_workbench_document;
pub(super) use crate::dashboard::list::{
    attach_dashboard_folder_paths_with_request, collect_dashboard_source_metadata,
    format_dashboard_summary_line, list_dashboards_with_client, list_dashboards_with_request,
    render_dashboard_summary_csv, render_dashboard_summary_json, render_dashboard_summary_table,
};
pub(super) use crate::dashboard::live::{
    build_datasource_inventory_record, build_folder_inventory_status, build_folder_path,
    collect_folder_inventory_statuses_with_request, collect_folder_inventory_with_request,
    fetch_dashboard_if_exists_with_request, fetch_dashboard_permissions_with_request,
    fetch_dashboard_with_request, fetch_folder_if_exists_with_request,
    fetch_folder_permissions_with_request, format_folder_inventory_status_line,
    import_dashboard_request_with_request, list_dashboard_summaries_with_request,
    list_datasources_with_request,
};
pub(super) use crate::dashboard::models::{
    DashboardExportRootManifest, DashboardExportRootScopeKind, DashboardIndexItem,
    DatasourceInventoryItem, ExportDatasourceUsageSummary, ExportMetadata, ExportOrgSummary,
    FolderInventoryItem, RootExportIndex, RootExportVariants, VariantIndexEntry,
};
pub(super) use crate::dashboard::prompt::{
    build_datasource_catalog, build_external_export_document, collect_datasource_refs,
    datasource_type_alias, is_builtin_datasource_ref, is_placeholder_string, lookup_datasource,
    resolve_datasource_type_alias,
};
pub(super) use crate::dashboard::raw_to_prompt::run_raw_to_prompt;
pub(super) use crate::dashboard::screenshot::{
    build_dashboard_capture_url, infer_screenshot_output_format, resolve_manifest_title,
    validate_screenshot_args,
};
pub(super) use crate::dashboard::topology::{
    build_impact_browser_items, build_impact_document, build_topology_document, render_impact_text,
    render_topology_dot, render_topology_mermaid, ImpactAlertResource, ImpactDashboard,
    ImpactDocument, ImpactSummary, TopologyDocument,
};
pub(super) use crate::dashboard::topology_tui::{
    build_topology_tui_groups, filter_topology_tui_items,
};
pub(super) use crate::dashboard::validate::{
    render_validation_result_json, validate_dashboard_export_dir,
};
pub(super) use crate::dashboard::vars::extract_dashboard_variables;
pub(super) use crate::dashboard::{
    FolderInventoryStatus, FolderInventoryStatusKind, DASHBOARD_PERMISSION_BUNDLE_FILENAME,
    DATASOURCE_INVENTORY_FILENAME, DEFAULT_DASHBOARD_TITLE, DEFAULT_EXPORT_DIR,
    DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID, DEFAULT_IMPORT_MESSAGE, DEFAULT_ORG_ID,
    DEFAULT_ORG_NAME, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_UNKNOWN_UID, DEFAULT_URL,
    EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME, PROMPT_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR,
    ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn make_core_family_report_row(
    dashboard_uid: &str,
    panel_id: &str,
    ref_id: &str,
    datasource_uid: &str,
    datasource_name: &str,
    datasource_type: &str,
    datasource_family: &str,
    query_text: &str,
    measurements: &[&str],
) -> ExportInspectionQueryRow {
    ExportInspectionQueryRow {
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
        dashboard_uid: dashboard_uid.to_string(),
        dashboard_title: format!("{dashboard_uid} Dashboard"),
        dashboard_tags: Vec::new(),
        folder_path: "General".to_string(),
        folder_full_path: "/".to_string(),
        folder_level: "1".to_string(),
        folder_uid: "general".to_string(),
        parent_folder_uid: String::new(),
        panel_id: panel_id.to_string(),
        panel_title: "Query".to_string(),
        panel_type: "table".to_string(),
        panel_target_count: 1,
        panel_query_count: 1,
        panel_datasource_count: 0,
        panel_variables: Vec::new(),
        ref_id: ref_id.to_string(),
        datasource: datasource_name.to_string(),
        datasource_name: datasource_name.to_string(),
        datasource_uid: datasource_uid.to_string(),
        datasource_org: String::new(),
        datasource_org_id: String::new(),
        datasource_database: String::new(),
        datasource_bucket: String::new(),
        datasource_organization: String::new(),
        datasource_index_pattern: String::new(),
        datasource_type: datasource_type.to_string(),
        datasource_family: datasource_family.to_string(),
        query_field: "query".to_string(),
        target_hidden: "false".to_string(),
        target_disabled: "false".to_string(),
        query_text: query_text.to_string(),
        query_variables: Vec::new(),
        metrics: Vec::new(),
        functions: Vec::new(),
        measurements: measurements.iter().map(|value| value.to_string()).collect(),
        buckets: Vec::new(),
        file_path: format!("/tmp/raw/{dashboard_uid}.json"),
    }
}
