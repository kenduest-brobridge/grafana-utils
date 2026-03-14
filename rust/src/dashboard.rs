use clap::CommandFactory;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;

#[path = "dashboard_cli_defs.rs"]
mod dashboard_cli_defs;
#[path = "dashboard_export.rs"]
mod dashboard_export;
#[path = "dashboard_files.rs"]
mod dashboard_files;
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
#[path = "dashboard_inspect_render.rs"]
mod dashboard_inspect_render;
#[path = "dashboard_inspect_governance.rs"]
mod dashboard_inspect_governance;
#[path = "dashboard_inspect_report.rs"]
mod dashboard_inspect_report;
#[path = "dashboard_list.rs"]
mod dashboard_list;
#[path = "dashboard_prompt.rs"]
mod dashboard_prompt;

pub use dashboard_cli_defs::{
    build_auth_context, build_http_client, build_http_client_for_org, parse_cli_from,
    CommonCliArgs, DashboardAuthContext, DashboardCliArgs, DashboardCommand, DiffArgs, ExportArgs,
    ImportArgs, InspectExportArgs, InspectExportReportFormat, InspectLiveArgs, ListArgs,
    ListDataSourcesArgs,
};
pub use dashboard_export::{
    build_export_variant_dirs, build_output_path, export_dashboards_with_client,
};
pub use dashboard_import::{diff_dashboards_with_client, import_dashboards_with_client};
pub use dashboard_list::{list_dashboards_with_client, list_data_sources_with_client};
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
    import_dashboards_with_request, render_import_dry_run_json, render_import_dry_run_table,
};
pub(crate) use dashboard_inspect::inspect_live_dashboards_with_request;
#[cfg(test)]
pub(crate) use dashboard_inspect::{
    apply_query_report_filters, build_export_inspection_query_report,
    build_export_inspection_summary, validate_inspect_export_report_args,
};
#[cfg(test)]
pub(crate) use dashboard_inspect_render::{
    render_csv, render_grouped_query_report, render_grouped_query_table_report,
};
#[cfg(test)]
pub(crate) use dashboard_inspect_governance::{
    build_export_inspection_governance_document, render_governance_table_report,
};
#[cfg(test)]
pub(crate) use dashboard_inspect_report::normalize_query_report;
pub(crate) use dashboard_inspect_report::{
    build_query_report, refresh_filtered_query_report_summary, render_query_report_column,
    report_column_header, report_format_supports_columns, resolve_report_column_ids,
    ExportInspectionQueryReport, ExportInspectionQueryRow,
};
#[cfg(test)]
pub(crate) use dashboard_inspect_report::{QueryReportSummary, DEFAULT_REPORT_COLUMN_IDS};
#[cfg(test)]
pub(crate) use dashboard_list::{
    attach_dashboard_folder_paths_with_request, collect_dashboard_source_metadata,
    format_dashboard_summary_line, format_data_source_line, list_dashboards_with_request,
    list_data_sources_with_request, render_dashboard_summary_csv, render_dashboard_summary_json,
    render_dashboard_summary_table, render_data_source_csv, render_data_source_json,
    render_data_source_table,
};
pub(crate) use dashboard_prompt::{
    build_datasource_catalog, collect_datasource_refs, datasource_type_alias,
    is_builtin_datasource_ref, is_placeholder_string, lookup_datasource,
    resolve_datasource_type_alias,
};

fn render_dashboard_subcommand_help_text(subcommand_name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    let subcommand = command
        .find_subcommand_mut(subcommand_name)
        .unwrap_or_else(|| panic!("missing dashboard subcommand {subcommand_name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    String::from_utf8(output).expect("dashboard help should be valid UTF-8")
}

pub fn render_inspect_export_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("inspect-export");
    text.push_str(INSPECT_EXPORT_HELP_FULL_EXAMPLES);
    text
}

pub fn render_inspect_live_help_full() -> String {
    let mut text = render_dashboard_subcommand_help_text("inspect-live");
    text.push_str(INSPECT_LIVE_HELP_FULL_EXAMPLES);
    text
}

pub fn maybe_render_dashboard_help_full_from_os_args<I, T>(iter: I) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString>,
{
    let args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<String>>();
    if !args.iter().any(|value| value == "--help-full") {
        return None;
    }
    let rest = args.get(1..).unwrap_or(&[]);
    match rest {
        [dashboard, command, ..] if dashboard == "dashboard" && command == "inspect-export" => {
            Some(render_inspect_export_help_full())
        }
        [dashboard, command, ..] if dashboard == "dashboard" && command == "inspect-live" => {
            Some(render_inspect_live_help_full())
        }
        [command, ..] if command == "inspect-export" => Some(render_inspect_export_help_full()),
        [command, ..] if command == "inspect-live" => Some(render_inspect_live_help_full()),
        _ => None,
    }
}

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

const INSPECT_EXPORT_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Flat per-query table report:\n    grafana-utils dashboard inspect-export --import-dir ./dashboards/raw --report\n\n  Datasource governance tables:\n    grafana-utils dashboard inspect-export --import-dir ./dashboards/raw --report governance\n\n  Datasource governance JSON:\n    grafana-utils dashboard inspect-export --import-dir ./dashboards/raw --report governance-json\n\n  Dashboard-first grouped tables:\n    grafana-utils dashboard inspect-export --import-dir ./dashboards/raw --report tree-table\n\n  Filter to one datasource and narrow columns:\n    grafana-utils dashboard inspect-export --import-dir ./dashboards/raw --report tree-table --report-filter-datasource prom-main --report-columns panel_id,panel_title,datasource,query\n\n  Dashboard/panel tree text view:\n    grafana-utils dashboard inspect-export --import-dir ./dashboards/raw --report tree --report-filter-panel-id 7\n";

const INSPECT_LIVE_HELP_FULL_EXAMPLES: &str = "\nExtended Examples:\n\n  Flat per-query table report from live Grafana:\n    grafana-utils dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report\n\n  Datasource governance tables from live Grafana:\n    grafana-utils dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report governance\n\n  Datasource governance JSON from live Grafana:\n    grafana-utils dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report governance-json\n\n  Dashboard-first grouped tables from live Grafana:\n    grafana-utils dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report tree-table\n\n  Filter to one datasource and narrow columns:\n    grafana-utils dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report tree-table --report-filter-datasource prom-main --report-columns panel_id,panel_title,datasource,query\n\n  Dashboard/panel tree text view:\n    grafana-utils dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --report tree --report-filter-panel-id 7\n";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ExportMetadata {
    #[serde(rename = "schemaVersion")]
    schema_version: i64,
    kind: String,
    variant: String,
    #[serde(rename = "dashboardCount")]
    dashboard_count: u64,
    #[serde(rename = "indexFile")]
    index_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(rename = "foldersFile", skip_serializing_if = "Option::is_none")]
    folders_file: Option<String>,
    #[serde(rename = "datasourcesFile", skip_serializing_if = "Option::is_none")]
    datasources_file: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct DashboardIndexItem {
    uid: String,
    title: String,
    #[serde(rename = "folderTitle")]
    folder_title: String,
    org: String,
    #[serde(rename = "orgId")]
    org_id: String,
    #[serde(rename = "raw_path", skip_serializing_if = "Option::is_none")]
    raw_path: Option<String>,
    #[serde(rename = "prompt_path", skip_serializing_if = "Option::is_none")]
    prompt_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct VariantIndexEntry {
    uid: String,
    title: String,
    path: String,
    format: String,
    org: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct RootExportVariants {
    raw: Option<String>,
    prompt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct RootExportIndex {
    #[serde(rename = "schemaVersion")]
    schema_version: i64,
    kind: String,
    items: Vec<DashboardIndexItem>,
    variants: RootExportVariants,
    #[serde(default)]
    folders: Vec<FolderInventoryItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct FolderInventoryItem {
    uid: String,
    title: String,
    path: String,
    #[serde(rename = "parentUid", skip_serializing_if = "Option::is_none")]
    parent_uid: Option<String>,
    org: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct DatasourceInventoryItem {
    uid: String,
    name: String,
    #[serde(rename = "type")]
    datasource_type: String,
    access: String,
    url: String,
    #[serde(rename = "isDefault")]
    is_default: String,
    org: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

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

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ExportFolderUsage {
    path: String,
    dashboards: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ExportDatasourceUsage {
    datasource: String,
    reference_count: usize,
    dashboard_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct DatasourceInventorySummary {
    uid: String,
    name: String,
    #[serde(rename = "type")]
    datasource_type: String,
    access: String,
    url: String,
    #[serde(rename = "isDefault")]
    is_default: String,
    org: String,
    #[serde(rename = "orgId")]
    org_id: String,
    reference_count: usize,
    dashboard_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct MixedDashboardSummary {
    uid: String,
    title: String,
    folder_path: String,
    datasource_count: usize,
    datasources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionSummary {
    import_dir: String,
    dashboard_count: usize,
    folder_count: usize,
    panel_count: usize,
    query_count: usize,
    datasource_inventory_count: usize,
    mixed_dashboard_count: usize,
    folder_paths: Vec<ExportFolderUsage>,
    datasource_usage: Vec<ExportDatasourceUsage>,
    datasource_inventory: Vec<DatasourceInventorySummary>,
    mixed_dashboards: Vec<MixedDashboardSummary>,
}

fn list_dashboard_summaries_with_request<F>(
    mut request_json: F,
    page_size: usize,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut dashboards = Vec::new();
    let mut seen_uids = std::collections::BTreeSet::new();
    let mut page = 1;

    loop {
        let params = vec![
            ("type".to_string(), "dash-db".to_string()),
            ("limit".to_string(), page_size.to_string()),
            ("page".to_string(), page.to_string()),
        ];
        let response = request_json(Method::GET, "/api/search", &params, None)?;
        let batch = match response {
            Some(Value::Array(batch)) => batch,
            Some(_) => return Err(message("Unexpected search response from Grafana.")),
            None => Vec::new(),
        };

        if batch.is_empty() {
            break;
        }

        let batch_len = batch.len();
        for item in batch {
            let object =
                value_as_object(&item, "Unexpected dashboard summary payload from Grafana.")?;
            let uid = string_field(object, "uid", "");
            if uid.is_empty() || seen_uids.contains(&uid) {
                continue;
            }
            seen_uids.insert(uid);
            dashboards.push(object.clone());
        }

        if batch_len < page_size {
            break;
        }
        page += 1;
    }

    Ok(dashboards)
}

pub fn list_dashboard_summaries(
    client: &JsonHttpClient,
    page_size: usize,
) -> Result<Vec<Map<String, Value>>> {
    list_dashboard_summaries_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        page_size,
    )
}

fn fetch_folder_if_exists_with_request<F>(
    mut request_json: F,
    uid: &str,
) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, &format!("/api/folders/{uid}"), &[], None)? {
        Some(value) => {
            let object =
                value_as_object(&value, &format!("Unexpected folder payload for UID {uid}."))?;
            Ok(Some(object.clone()))
        }
        None => Ok(None),
    }
}

fn collect_folder_inventory_with_request<F>(
    mut request_json: F,
    summaries: &[Map<String, Value>],
) -> Result<Vec<FolderInventoryItem>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut seen = std::collections::BTreeSet::new();
    let mut folders = Vec::new();
    for summary in summaries {
        let folder_uid = string_field(summary, "folderUid", "");
        if folder_uid.is_empty() {
            continue;
        }
        let org_id = summary
            .get("orgId")
            .map(|value| match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            })
            .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
        let key = format!("{org_id}:{folder_uid}");
        if seen.contains(&key) {
            continue;
        }
        let Some(folder) = fetch_folder_if_exists_with_request(&mut request_json, &folder_uid)?
        else {
            continue;
        };
        let org = string_field(summary, "orgName", DEFAULT_ORG_NAME);
        let mut parent_path = Vec::new();
        let mut previous_parent_uid = None;
        if let Some(parents) = folder.get("parents").and_then(Value::as_array) {
            for parent in parents {
                let Some(parent_object) = parent.as_object() else {
                    continue;
                };
                let parent_uid = string_field(parent_object, "uid", "");
                let parent_title = string_field(parent_object, "title", "");
                if parent_uid.is_empty() || parent_title.is_empty() {
                    continue;
                }
                parent_path.push(parent_title.clone());
                let parent_key = format!("{org_id}:{parent_uid}");
                if !seen.contains(&parent_key) {
                    folders.push(FolderInventoryItem {
                        uid: parent_uid.clone(),
                        title: parent_title,
                        path: parent_path.join(" / "),
                        parent_uid: previous_parent_uid.clone(),
                        org: org.clone(),
                        org_id: org_id.clone(),
                    });
                    seen.insert(parent_key);
                }
                previous_parent_uid = Some(parent_uid);
            }
        }
        let folder_title = string_field(&folder, "title", DEFAULT_FOLDER_TITLE);
        parent_path.push(folder_title.clone());
        folders.push(FolderInventoryItem {
            uid: folder_uid.clone(),
            title: folder_title,
            path: parent_path.join(" / "),
            parent_uid: previous_parent_uid,
            org,
            org_id: org_id.clone(),
        });
        seen.insert(key);
    }
    folders.sort_by(|left, right| {
        left.org_id
            .cmp(&right.org_id)
            .then(left.path.cmp(&right.path))
            .then(left.uid.cmp(&right.uid))
    });
    Ok(folders)
}

pub(crate) fn build_datasource_inventory_record(
    datasource: &Map<String, Value>,
    org: &Map<String, Value>,
) -> DatasourceInventoryItem {
    DatasourceInventoryItem {
        uid: string_field(datasource, "uid", ""),
        name: string_field(datasource, "name", ""),
        datasource_type: string_field(datasource, "type", ""),
        access: string_field(datasource, "access", ""),
        url: string_field(datasource, "url", ""),
        is_default: if datasource
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "true".to_string()
        } else {
            "false".to_string()
        },
        org: string_field(org, "name", DEFAULT_ORG_NAME),
        org_id: org
            .get("id")
            .map(|value| match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            })
            .unwrap_or_else(|| DEFAULT_ORG_ID.to_string()),
    }
}

fn create_folder_with_request<F>(
    request_json: &mut F,
    title: &str,
    uid: &str,
    parent_uid: Option<&str>,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut payload = Map::new();
    payload.insert("uid".to_string(), Value::String(uid.to_string()));
    payload.insert("title".to_string(), Value::String(title.to_string()));
    if let Some(parent_uid) = parent_uid.filter(|value| !value.is_empty()) {
        payload.insert(
            "parentUid".to_string(),
            Value::String(parent_uid.to_string()),
        );
    }
    let _ = request_json(
        Method::POST,
        "/api/folders",
        &[],
        Some(&Value::Object(payload)),
    )?;
    Ok(())
}

fn ensure_folder_inventory_entry_with_request<F>(
    request_json: &mut F,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
    folder_uid: &str,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if folder_uid.is_empty() {
        return Ok(());
    }
    let mut create_chain = Vec::new();
    let mut current_uid = folder_uid.to_string();
    loop {
        if fetch_folder_if_exists_with_request(&mut *request_json, &current_uid)?.is_some() {
            break;
        }
        let folder = folders_by_uid.get(&current_uid).ok_or_else(|| {
            message(format!(
                "Missing exported folder inventory for folderUid {current_uid}."
            ))
        })?;
        create_chain.push(folder.clone());
        let Some(parent_uid) = folder.parent_uid.as_deref() else {
            break;
        };
        current_uid = parent_uid.to_string();
    }
    for folder in create_chain.into_iter().rev() {
        if fetch_folder_if_exists_with_request(&mut *request_json, &folder.uid)?.is_some() {
            continue;
        }
        create_folder_with_request(
            &mut *request_json,
            &folder.title,
            &folder.uid,
            folder.parent_uid.as_deref(),
        )?;
    }
    Ok(())
}

fn build_folder_path(folder: &Map<String, Value>, fallback_title: &str) -> String {
    let mut titles = Vec::new();
    if let Some(parents) = folder.get("parents").and_then(Value::as_array) {
        for parent in parents {
            if let Some(parent_object) = parent.as_object() {
                let title = string_field(parent_object, "title", "");
                if !title.is_empty() {
                    titles.push(title);
                }
            }
        }
    }
    let title = string_field(folder, "title", fallback_title);
    if !title.is_empty() {
        titles.push(title);
    }
    if titles.is_empty() {
        fallback_title.to_string()
    } else {
        titles.join(" / ")
    }
}

fn parent_uid_from_folder(folder: &Map<String, Value>) -> Option<String> {
    folder
        .get("parents")
        .and_then(Value::as_array)
        .and_then(|parents| parents.last())
        .and_then(Value::as_object)
        .map(|parent| string_field(parent, "uid", ""))
        .filter(|uid| !uid.is_empty())
}

pub(crate) fn build_folder_inventory_status(
    folder: &FolderInventoryItem,
    destination_folder: Option<&Map<String, Value>>,
) -> FolderInventoryStatus {
    let expected_parent_uid = folder.parent_uid.clone();
    let mut status = FolderInventoryStatus {
        uid: folder.uid.clone(),
        expected_title: folder.title.clone(),
        expected_parent_uid,
        expected_path: folder.path.clone(),
        actual_title: None,
        actual_parent_uid: None,
        actual_path: None,
        kind: FolderInventoryStatusKind::Missing,
    };
    let Some(destination_folder) = destination_folder else {
        return status;
    };

    status.actual_title = Some(string_field(destination_folder, "title", ""));
    status.actual_parent_uid = parent_uid_from_folder(destination_folder);
    status.actual_path = Some(build_folder_path(destination_folder, &folder.title));
    let title_matches = status.actual_title.as_deref() == Some(folder.title.as_str());
    let parent_matches = status.actual_parent_uid == folder.parent_uid;
    let path_matches = status.actual_path.as_deref() == Some(folder.path.as_str());
    status.kind = if title_matches && parent_matches && path_matches {
        FolderInventoryStatusKind::Matches
    } else {
        FolderInventoryStatusKind::Mismatch
    };
    status
}

pub(crate) fn format_folder_inventory_status_line(status: &FolderInventoryStatus) -> String {
    match status.kind {
        FolderInventoryStatusKind::Missing => format!(
            "Folder inventory missing uid={} title={} parentUid={} path={}",
            status.uid,
            status.expected_title,
            status.expected_parent_uid.as_deref().unwrap_or("-"),
            status.expected_path
        ),
        FolderInventoryStatusKind::Matches => format!(
            "Folder inventory matches uid={} title={} parentUid={} path={}",
            status.uid,
            status.expected_title,
            status.expected_parent_uid.as_deref().unwrap_or("-"),
            status.expected_path
        ),
        FolderInventoryStatusKind::Mismatch => format!(
            "Folder inventory mismatch uid={} expected(title={}, parentUid={}, path={}) actual(title={}, parentUid={}, path={})",
            status.uid,
            status.expected_title,
            status.expected_parent_uid.as_deref().unwrap_or("-"),
            status.expected_path,
            status.actual_title.as_deref().unwrap_or("-"),
            status.actual_parent_uid.as_deref().unwrap_or("-"),
            status.actual_path.as_deref().unwrap_or("-")
        ),
    }
}

fn build_folder_inventory_dry_run_record(status: &FolderInventoryStatus) -> [String; 6] {
    let destination = match status.kind {
        FolderInventoryStatusKind::Missing => "missing",
        _ => "exists",
    };
    let reason = match status.kind {
        FolderInventoryStatusKind::Missing => "would-create".to_string(),
        FolderInventoryStatusKind::Matches => String::new(),
        FolderInventoryStatusKind::Mismatch => {
            let mut reasons = Vec::new();
            if status.actual_title.as_deref() != Some(status.expected_title.as_str()) {
                reasons.push("title");
            }
            if status.actual_parent_uid != status.expected_parent_uid {
                reasons.push("parentUid");
            }
            if status.actual_path.as_deref() != Some(status.expected_path.as_str()) {
                reasons.push("path");
            }
            reasons.join(",")
        }
    };
    [
        status.uid.clone(),
        destination.to_string(),
        match status.kind {
            FolderInventoryStatusKind::Missing => "missing",
            FolderInventoryStatusKind::Matches => "match",
            FolderInventoryStatusKind::Mismatch => "mismatch",
        }
        .to_string(),
        reason,
        status.expected_path.clone(),
        status.actual_path.clone().unwrap_or_default(),
    ]
}

fn render_folder_inventory_dry_run_table(
    records: &[[String; 6]],
    include_header: bool,
) -> Vec<String> {
    let headers = [
        "UID",
        "DESTINATION",
        "STATUS",
        "REASON",
        "EXPECTED_PATH",
        "ACTUAL_PATH",
    ];
    let mut widths = headers.map(str::len);
    for row in records {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String; 6]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        let header_values = [
            headers[0].to_string(),
            headers[1].to_string(),
            headers[2].to_string(),
            headers[3].to_string(),
            headers[4].to_string(),
            headers[5].to_string(),
        ];
        let divider_values = [
            "-".repeat(widths[0]),
            "-".repeat(widths[1]),
            "-".repeat(widths[2]),
            "-".repeat(widths[3]),
            "-".repeat(widths[4]),
            "-".repeat(widths[5]),
        ];
        lines.push(format_row(&header_values));
        lines.push(format_row(&divider_values));
    }
    for row in records {
        lines.push(format_row(row));
    }
    lines
}

fn collect_folder_inventory_statuses_with_request<F>(
    request_json: &mut F,
    folder_inventory: &[FolderInventoryItem],
) -> Result<Vec<FolderInventoryStatus>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut statuses = Vec::new();
    for folder in folder_inventory {
        let destination_folder =
            fetch_folder_if_exists_with_request(&mut *request_json, &folder.uid)?;
        statuses.push(build_folder_inventory_status(
            folder,
            destination_folder.as_ref(),
        ));
    }
    Ok(statuses)
}

fn fetch_dashboard_with_request<F>(mut request_json: F, uid: &str) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(
        Method::GET,
        &format!("/api/dashboards/uid/{uid}"),
        &[],
        None,
    )? {
        Some(value) => {
            let object = value_as_object(
                &value,
                &format!("Unexpected dashboard payload for UID {uid}."),
            )?;
            if !object.contains_key("dashboard") {
                return Err(message(format!(
                    "Unexpected dashboard payload for UID {uid}."
                )));
            }
            Ok(value)
        }
        None => Err(message(format!(
            "Unexpected empty dashboard payload for UID {uid}."
        ))),
    }
}

pub fn fetch_dashboard(client: &JsonHttpClient, uid: &str) -> Result<Value> {
    fetch_dashboard_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        uid,
    )
}

fn fetch_dashboard_if_exists_with_request<F>(
    mut request_json: F,
    uid: &str,
) -> Result<Option<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match fetch_dashboard_with_request(&mut request_json, uid) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.status_code() == Some(404) => Ok(None),
        Err(error) => Err(error),
    }
}

fn import_dashboard_request_with_request<F>(mut request_json: F, payload: &Value) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::POST, "/api/dashboards/db", &[], Some(payload))? {
        Some(value) => {
            value_as_object(&value, "Unexpected dashboard import response from Grafana.")?;
            Ok(value)
        }
        None => Err(message(
            "Unexpected empty dashboard import response from Grafana.",
        )),
    }
}

pub fn import_dashboard_request(client: &JsonHttpClient, payload: &Value) -> Result<Value> {
    import_dashboard_request_with_request(
        |method, path, params, request_payload| {
            client.request_json(method, path, params, request_payload)
        },
        payload,
    )
}

fn list_datasources_with_request<F>(mut request_json: F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| {
                value_as_object(&item, "Unexpected datasource payload from Grafana.").cloned()
            })
            .collect(),
        Some(_) => Err(message("Unexpected datasource list response from Grafana.")),
        None => Ok(Vec::new()),
    }
}

pub fn list_datasources(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    list_datasources_with_request(|method, path, params, payload| {
        client.request_json(method, path, params, payload)
    })
}

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

pub fn run_dashboard_cli(args: DashboardCliArgs) -> Result<()> {
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
