use regex::Regex;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{message, object_field, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;

#[path = "dashboard_cli_defs.rs"]
mod dashboard_cli_defs;
#[path = "dashboard_export.rs"]
mod dashboard_export;
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
pub use dashboard_list::{list_dashboards_with_client, list_data_sources_with_client};
pub use dashboard_prompt::build_external_export_document;

use dashboard_export::export_dashboards_with_org_clients;
use dashboard_list::list_dashboards_with_org_clients;

#[cfg(test)]
pub(crate) use dashboard_export::{
    export_dashboards_with_request, format_export_progress_line, format_export_verbose_line,
};
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ExportMetadata {
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
struct DashboardIndexItem {
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
struct VariantIndexEntry {
    uid: String,
    title: String,
    path: String,
    format: String,
    org: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct RootExportVariants {
    raw: Option<String>,
    prompt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct RootExportIndex {
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
struct ExportInspectionSummary {
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

#[derive(Clone, Debug, PartialEq, Serialize)]
struct QueryReportSummary {
    dashboard_count: usize,
    panel_count: usize,
    query_count: usize,
    report_row_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ExportInspectionQueryRow {
    #[serde(rename = "dashboardUid")]
    dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    dashboard_title: String,
    #[serde(rename = "folderPath")]
    folder_path: String,
    #[serde(rename = "panelId")]
    panel_id: String,
    #[serde(rename = "panelTitle")]
    panel_title: String,
    #[serde(rename = "panelType")]
    panel_type: String,
    #[serde(rename = "refId")]
    ref_id: String,
    datasource: String,
    #[serde(rename = "datasourceUid")]
    datasource_uid: String,
    #[serde(rename = "queryField")]
    query_field: String,
    #[serde(rename = "queryText")]
    query_text: String,
    metrics: Vec<String>,
    measurements: Vec<String>,
    buckets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ExportInspectionQueryReport {
    import_dir: String,
    summary: QueryReportSummary,
    queries: Vec<ExportInspectionQueryRow>,
}

const DEFAULT_REPORT_COLUMN_IDS: &[&str] = &[
    "dashboard_uid",
    "dashboard_title",
    "folder_path",
    "panel_id",
    "panel_title",
    "panel_type",
    "ref_id",
    "datasource",
    "query_field",
    "metrics",
    "measurements",
    "buckets",
    "query",
];

const SUPPORTED_REPORT_COLUMN_IDS: &[&str] = &[
    "dashboard_uid",
    "dashboard_title",
    "folder_path",
    "panel_id",
    "panel_title",
    "panel_type",
    "ref_id",
    "datasource",
    "datasource_uid",
    "query_field",
    "metrics",
    "measurements",
    "buckets",
    "query",
];

pub fn discover_dashboard_files(import_dir: &Path) -> Result<Vec<PathBuf>> {
    if !import_dir.exists() {
        return Err(message(format!(
            "Import directory does not exist: {}",
            import_dir.display()
        )));
    }
    if !import_dir.is_dir() {
        return Err(message(format!(
            "Import path is not a directory: {}",
            import_dir.display()
        )));
    }
    if import_dir.join(RAW_EXPORT_SUBDIR).is_dir() && import_dir.join(PROMPT_EXPORT_SUBDIR).is_dir()
    {
        return Err(message(format!(
            "Import path {} looks like the combined export root. Point --import-dir at {}.",
            import_dir.display(),
            import_dir.join(RAW_EXPORT_SUBDIR).display()
        )));
    }

    let mut files = Vec::new();
    collect_json_files(import_dir, &mut files)?;
    files.retain(|path| {
        let file_name = path.file_name().and_then(|name| name.to_str());
        file_name != Some("index.json")
            && file_name != Some(EXPORT_METADATA_FILENAME)
            && file_name != Some(FOLDER_INVENTORY_FILENAME)
            && file_name != Some(DATASOURCE_INVENTORY_FILENAME)
    });
    files.sort();

    if files.is_empty() {
        return Err(message(format!(
            "No dashboard JSON files found in {}",
            import_dir.display()
        )));
    }

    Ok(files)
}

fn build_export_metadata(
    variant: &str,
    dashboard_count: usize,
    format_name: Option<&str>,
    folders_file: Option<&str>,
    datasources_file: Option<&str>,
) -> ExportMetadata {
    ExportMetadata {
        schema_version: TOOL_SCHEMA_VERSION,
        kind: ROOT_INDEX_KIND.to_string(),
        variant: variant.to_string(),
        dashboard_count: dashboard_count as u64,
        index_file: "index.json".to_string(),
        format: format_name.map(str::to_owned),
        folders_file: folders_file.map(str::to_owned),
        datasources_file: datasources_file.map(str::to_owned),
    }
}

fn validate_export_metadata(
    metadata: &ExportMetadata,
    metadata_path: &Path,
    expected_variant: Option<&str>,
) -> Result<()> {
    if metadata.kind != ROOT_INDEX_KIND {
        return Err(message(format!(
            "Unexpected dashboard export manifest kind in {}: {:?}",
            metadata_path.display(),
            metadata.kind
        )));
    }
    if metadata.schema_version != TOOL_SCHEMA_VERSION {
        return Err(message(format!(
            "Unsupported dashboard export schemaVersion {:?} in {}. Expected {}.",
            metadata.schema_version,
            metadata_path.display(),
            TOOL_SCHEMA_VERSION
        )));
    }
    if let Some(expected_variant) = expected_variant {
        if metadata.variant != expected_variant {
            return Err(message(format!(
                "Dashboard export manifest {} describes variant {:?}. Point this command at the {expected_variant}/ directory.",
                metadata_path.display(),
                metadata.variant
            )));
        }
    }
    Ok(())
}

fn load_export_metadata(
    import_dir: &Path,
    expected_variant: Option<&str>,
) -> Result<Option<ExportMetadata>> {
    let metadata_path = import_dir.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Ok(None);
    }
    let value = load_json_file(&metadata_path)?;
    value_as_object(&value, "Dashboard export metadata must be a JSON object.")?;
    let metadata: ExportMetadata = serde_json::from_value(value).map_err(|error| {
        message(format!(
            "Invalid dashboard export metadata in {}: {error}",
            metadata_path.display()
        ))
    })?;
    validate_export_metadata(&metadata, &metadata_path, expected_variant)?;
    Ok(Some(metadata))
}

fn collect_json_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, files)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(())
}

pub fn load_json_file(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&raw)?;
    if !value.is_object() {
        return Err(message(format!(
            "Dashboard file must contain a JSON object: {}",
            path.display()
        )));
    }
    Ok(value)
}

pub fn build_import_payload(
    document: &Value,
    folder_uid_override: Option<&str>,
    replace_existing: bool,
    message_text: &str,
) -> Result<Value> {
    let document_object = value_as_object(document, "Dashboard payload must be a JSON object.")?;
    if document_object.contains_key("__inputs") {
        return Err(message(
            "Dashboard file contains Grafana web-import placeholders (__inputs). Import it through the Grafana web UI after choosing datasources.",
        ));
    }

    let dashboard = extract_dashboard_object(document_object)?;
    let mut dashboard = dashboard.clone();
    dashboard.insert("id".to_string(), Value::Null);

    let folder_uid = folder_uid_override.map(str::to_owned).or_else(|| {
        object_field(document_object, "meta")
            .and_then(|meta| meta.get("folderUid"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    });

    let mut payload = Map::new();
    payload.insert("dashboard".to_string(), Value::Object(dashboard));
    payload.insert("overwrite".to_string(), Value::Bool(replace_existing));
    payload.insert(
        "message".to_string(),
        Value::String(message_text.to_string()),
    );
    if let Some(folder_uid) = folder_uid.filter(|value| !value.is_empty()) {
        payload.insert("folderUid".to_string(), Value::String(folder_uid));
    }
    Ok(Value::Object(payload))
}

pub fn build_preserved_web_import_document(payload: &Value) -> Result<Value> {
    let object = value_as_object(payload, "Unexpected dashboard payload from Grafana.")?;
    let mut dashboard = extract_dashboard_object(object)?.clone();
    dashboard.insert("id".to_string(), Value::Null);
    Ok(Value::Object(dashboard))
}

fn extract_dashboard_object(document: &Map<String, Value>) -> Result<&Map<String, Value>> {
    match document.get("dashboard") {
        Some(value) => value_as_object(value, "Dashboard payload must be a JSON object."),
        None => Ok(document),
    }
}

fn write_dashboard(payload: &Value, output_path: &Path, overwrite: bool) -> Result<()> {
    if output_path.exists() && !overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            output_path.display()
        )));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, serde_json::to_string_pretty(payload)? + "\n")?;
    Ok(())
}

fn write_json_document<T: Serialize>(payload: &T, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, serde_json::to_string_pretty(payload)? + "\n")?;
    Ok(())
}

fn build_dashboard_index_item(summary: &Map<String, Value>, uid: &str) -> DashboardIndexItem {
    DashboardIndexItem {
        uid: uid.to_string(),
        title: string_field(summary, "title", DEFAULT_DASHBOARD_TITLE),
        folder_title: string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
        org: string_field(summary, "orgName", DEFAULT_ORG_NAME),
        org_id: summary
            .get("orgId")
            .map(|value| match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            })
            .unwrap_or_else(|| DEFAULT_ORG_ID.to_string()),
        raw_path: None,
        prompt_path: None,
    }
}

fn build_variant_index(
    items: &[DashboardIndexItem],
    path_selector: impl Fn(&DashboardIndexItem) -> Option<&str>,
    export_format: &str,
) -> Vec<VariantIndexEntry> {
    items
        .iter()
        .filter_map(|item| {
            path_selector(item).map(|path| VariantIndexEntry {
                uid: item.uid.clone(),
                title: item.title.clone(),
                path: path.to_string(),
                format: export_format.to_string(),
                org: item.org.clone(),
                org_id: item.org_id.clone(),
            })
        })
        .collect()
}

fn build_root_export_index(
    items: &[DashboardIndexItem],
    raw_index_path: Option<&Path>,
    prompt_index_path: Option<&Path>,
    folders: &[FolderInventoryItem],
) -> RootExportIndex {
    RootExportIndex {
        schema_version: TOOL_SCHEMA_VERSION,
        kind: ROOT_INDEX_KIND.to_string(),
        items: items.to_vec(),
        variants: RootExportVariants {
            raw: raw_index_path.map(|path| path.display().to_string()),
            prompt: prompt_index_path.map(|path| path.display().to_string()),
        },
        folders: folders.to_vec(),
    }
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

fn load_folder_inventory(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<Vec<FolderInventoryItem>> {
    let folders_file = metadata
        .and_then(|item| item.folders_file.as_deref())
        .unwrap_or(FOLDER_INVENTORY_FILENAME);
    let folder_inventory_path = import_dir.join(folders_file);
    if !folder_inventory_path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&folder_inventory_path)?;
    serde_json::from_str(&raw).map_err(Into::into)
}

fn load_datasource_inventory(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<Vec<DatasourceInventoryItem>> {
    let datasources_file = metadata
        .and_then(|item| item.datasources_file.as_deref())
        .unwrap_or(DATASOURCE_INVENTORY_FILENAME);
    let datasource_inventory_path = import_dir.join(datasources_file);
    if !datasource_inventory_path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&datasource_inventory_path)?;
    serde_json::from_str(&raw).map_err(Into::into)
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

fn build_compare_document(dashboard: &Map<String, Value>, folder_uid: Option<&str>) -> Value {
    let mut compare = Map::new();
    compare.insert("dashboard".to_string(), Value::Object(dashboard.clone()));
    if let Some(folder_uid) = folder_uid.filter(|value| !value.is_empty()) {
        compare.insert(
            "folderUid".to_string(),
            Value::String(folder_uid.to_string()),
        );
    }
    Value::Object(compare)
}

fn build_local_compare_document(
    document: &Value,
    folder_uid_override: Option<&str>,
) -> Result<Value> {
    let payload = build_import_payload(document, folder_uid_override, false, "")?;
    let payload_object =
        value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
    let dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let folder_uid = payload_object.get("folderUid").and_then(Value::as_str);
    Ok(build_compare_document(dashboard, folder_uid))
}

fn build_remote_compare_document(
    payload: &Value,
    folder_uid_override: Option<&str>,
) -> Result<Value> {
    let dashboard = build_preserved_web_import_document(payload)?;
    let dashboard_object =
        value_as_object(&dashboard, "Unexpected dashboard payload from Grafana.")?;
    let payload_object = value_as_object(payload, "Unexpected dashboard payload from Grafana.")?;
    let folder_uid = folder_uid_override.or_else(|| {
        object_field(payload_object, "meta")
            .and_then(|meta| meta.get("folderUid"))
            .and_then(Value::as_str)
    });
    Ok(build_compare_document(dashboard_object, folder_uid))
}

fn serialize_compare_document(document: &Value) -> Result<String> {
    Ok(serde_json::to_string(document)?)
}

fn build_compare_diff_text(
    remote_compare: &Value,
    local_compare: &Value,
    uid: &str,
    dashboard_file: &Path,
    _context_lines: usize,
) -> Result<String> {
    let remote_pretty = serde_json::to_string_pretty(remote_compare)?;
    let local_pretty = serde_json::to_string_pretty(local_compare)?;
    let mut text = String::new();
    let _ = writeln!(&mut text, "--- grafana:{uid}");
    let _ = writeln!(&mut text, "+++ {}", dashboard_file.display());
    for line in remote_pretty.lines() {
        let _ = writeln!(&mut text, "-{line}");
    }
    for line in local_pretty.lines() {
        let _ = writeln!(&mut text, "+{line}");
    }
    Ok(text)
}

fn determine_dashboard_import_action_with_request<F>(
    mut request_json: F,
    payload: &Value,
    replace_existing: bool,
    update_existing_only: bool,
) -> Result<&'static str>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload_object =
        value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let uid = string_field(dashboard, "uid", "");
    if uid.is_empty() {
        return Ok("would-create");
    }
    if fetch_dashboard_if_exists_with_request(&mut request_json, &uid)?.is_none() {
        if update_existing_only {
            return Ok("would-skip-missing");
        }
        return Ok("would-create");
    }
    if replace_existing || update_existing_only {
        Ok("would-update")
    } else {
        Ok("would-fail-existing")
    }
}

fn determine_import_folder_uid_override_with_request<F>(
    mut request_json: F,
    uid: &str,
    folder_uid_override: Option<&str>,
    preserve_existing_folder: bool,
) -> Result<Option<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(value) = folder_uid_override {
        return Ok(Some(value.to_string()));
    }
    if !preserve_existing_folder || uid.is_empty() {
        return Ok(None);
    }
    let Some(existing_payload) = fetch_dashboard_if_exists_with_request(&mut request_json, uid)?
    else {
        return Ok(None);
    };
    let object = value_as_object(
        &existing_payload,
        &format!("Unexpected dashboard payload for UID {uid}."),
    )?;
    let folder_uid = object_field(object, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok(Some(folder_uid))
}

fn describe_dashboard_import_mode(
    replace_existing: bool,
    update_existing_only: bool,
) -> &'static str {
    if update_existing_only {
        "update-or-skip-missing"
    } else if replace_existing {
        "create-or-update"
    } else {
        "create-only"
    }
}

fn describe_import_action(action: &str) -> (&'static str, &str) {
    match action {
        "would-create" => ("missing", "create"),
        "would-update" => ("exists", "update"),
        "would-skip-missing" => ("missing", "skip-missing"),
        "would-fail-existing" => ("exists", "blocked-existing"),
        _ => (DEFAULT_UNKNOWN_UID, action),
    }
}

fn resolve_dashboard_import_folder_path_with_request<F>(
    mut request_json: F,
    payload: &Value,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload_object =
        value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let folder_uid = payload_object
        .get("folderUid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        return Ok(DEFAULT_FOLDER_TITLE.to_string());
    }
    if let Some(folder) = fetch_folder_if_exists_with_request(&mut request_json, &folder_uid)? {
        let fallback_title = string_field(&folder, "title", &folder_uid);
        return Ok(build_folder_path(&folder, &fallback_title));
    }
    if let Some(folder) = folders_by_uid.get(&folder_uid) {
        if !folder.path.is_empty() {
            return Ok(folder.path.clone());
        }
        if !folder.title.is_empty() {
            return Ok(folder.title.clone());
        }
    }
    Ok(folder_uid)
}

fn build_import_dry_run_record(
    dashboard_file: &Path,
    uid: &str,
    action: &str,
    folder_path: &str,
) -> [String; 5] {
    let (destination, action_label) = describe_import_action(action);
    [
        uid.to_string(),
        destination.to_string(),
        action_label.to_string(),
        folder_path.to_string(),
        dashboard_file.display().to_string(),
    ]
}

fn render_import_dry_run_table(records: &[[String; 5]], include_header: bool) -> Vec<String> {
    let headers = ["UID", "DESTINATION", "ACTION", "FOLDER_PATH", "FILE"];
    let mut widths = headers.map(str::len);
    for row in records {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String; 5]| -> String {
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
        ];
        let divider_values = [
            "-".repeat(widths[0]),
            "-".repeat(widths[1]),
            "-".repeat(widths[2]),
            "-".repeat(widths[3]),
            "-".repeat(widths[4]),
        ];
        lines.push(format_row(&header_values));
        lines.push(format_row(&divider_values));
    }
    for row in records {
        lines.push(format_row(row));
    }
    lines
}

fn render_import_dry_run_json(
    mode: &str,
    folder_statuses: &[FolderInventoryStatus],
    dashboard_records: &[[String; 5]],
    import_dir: &Path,
    skipped_missing_count: usize,
) -> Result<String> {
    let mut folders = Vec::new();
    for status in folder_statuses {
        let (destination, status_label, reason) = match status.kind {
            FolderInventoryStatusKind::Missing => {
                ("missing", "missing", "would-create".to_string())
            }
            FolderInventoryStatusKind::Matches => ("exists", "match", String::new()),
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
                ("exists", "mismatch", reasons.join(","))
            }
        };
        folders.push(json!({
            "uid": status.uid,
            "destination": destination,
            "status": status_label,
            "reason": reason,
            "expectedPath": status.expected_path,
            "actualPath": status.actual_path.clone().unwrap_or_default(),
        }));
    }
    let dashboards = dashboard_records
        .iter()
        .map(|row| {
            json!({
                "uid": row[0],
                "destination": row[1],
                "action": row[2],
                "folderPath": row[3],
                "file": row[4],
            })
        })
        .collect::<Vec<Value>>();
    let payload = json!({
        "mode": mode,
        "folders": folders,
        "dashboards": dashboards,
        "summary": {
            "importDir": import_dir.display().to_string(),
            "folderCount": folder_statuses.len(),
            "missingFolders": folder_statuses.iter().filter(|status| status.kind == FolderInventoryStatusKind::Missing).count(),
            "mismatchedFolders": folder_statuses.iter().filter(|status| status.kind == FolderInventoryStatusKind::Mismatch).count(),
            "dashboardCount": dashboard_records.len(),
            "missingDashboards": dashboard_records.iter().filter(|row| row[1] == "missing").count(),
            "skippedMissingDashboards": skipped_missing_count,
        }
    });
    Ok(serde_json::to_string_pretty(&payload)?)
}

fn render_simple_table(
    headers: &[&str],
    rows: &[Vec<String>],
    include_header: bool,
) -> Vec<String> {
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<usize>>();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        lines.push(format_row(
            &headers
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<String>>(),
        ));
        lines.push(format_row(
            &widths
                .iter()
                .map(|width| "-".repeat(*width))
                .collect::<Vec<String>>(),
        ));
    }
    for row in rows {
        lines.push(format_row(row));
    }
    lines
}

fn render_csv(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    fn escape_csv(value: &str) -> String {
        if value.contains(',') || value.contains('"') || value.contains('\n') {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }

    let mut lines = Vec::new();
    lines.push(
        headers
            .iter()
            .map(|value| escape_csv(value))
            .collect::<Vec<String>>()
            .join(","),
    );
    for row in rows {
        lines.push(
            row.iter()
                .map(|value| escape_csv(value))
                .collect::<Vec<String>>()
                .join(","),
        );
    }
    lines
}

fn resolve_export_folder_path(
    document: &Map<String, Value>,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
) -> String {
    let folder_uid = object_field(document, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if !folder_uid.is_empty() {
        if let Some(folder) = folders_by_uid.get(&folder_uid) {
            return folder.path.clone();
        }
    }
    let relative_parent = dashboard_file
        .strip_prefix(import_dir)
        .ok()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| Path::new(""));
    let folder_name = relative_parent.display().to_string();
    if !folder_name.is_empty() && folder_name != "." && folder_name != DEFAULT_FOLDER_TITLE {
        let matches = folders_by_uid
            .values()
            .filter(|item| item.title == folder_name)
            .collect::<Vec<&FolderInventoryItem>>();
        if matches.len() == 1 {
            return matches[0].path.clone();
        }
    }
    if folder_name.is_empty() || folder_name == "." || folder_name == DEFAULT_FOLDER_TITLE {
        DEFAULT_FOLDER_TITLE.to_string()
    } else {
        folder_name
    }
}

fn collect_panel_stats(panel: &Map<String, Value>) -> (usize, usize) {
    let mut panel_count = 1usize;
    let mut query_count = panel
        .get("targets")
        .and_then(Value::as_array)
        .map(|targets| targets.len())
        .unwrap_or(0);
    if let Some(children) = panel.get("panels").and_then(Value::as_array) {
        for child in children {
            if let Some(child_object) = child.as_object() {
                let (child_panels, child_queries) = collect_panel_stats(child_object);
                panel_count += child_panels;
                query_count += child_queries;
            }
        }
    }
    (panel_count, query_count)
}

fn count_dashboard_panels_and_queries(dashboard: &Map<String, Value>) -> (usize, usize) {
    let mut panel_count = 0usize;
    let mut query_count = 0usize;
    if let Some(panels) = dashboard.get("panels").and_then(Value::as_array) {
        for panel in panels {
            if let Some(panel_object) = panel.as_object() {
                let (child_panels, child_queries) = collect_panel_stats(panel_object);
                panel_count += child_panels;
                query_count += child_queries;
            }
        }
    }
    (panel_count, query_count)
}

fn summarize_datasource_ref(reference: &Value) -> Option<String> {
    if reference.is_null() || is_builtin_datasource_ref(reference) {
        return None;
    }
    match reference {
        Value::String(text) => {
            if is_placeholder_string(text) {
                None
            } else {
                Some(text.to_string())
            }
        }
        Value::Object(object) => {
            for key in ["name", "uid", "type"] {
                if let Some(value) = object.get(key).and_then(Value::as_str) {
                    if !value.is_empty() && !is_placeholder_string(value) {
                        return Some(value.to_string());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn summarize_datasource_uid(reference: &Value) -> Option<String> {
    if reference.is_null() || is_builtin_datasource_ref(reference) {
        return None;
    }
    match reference {
        Value::String(text) => {
            if is_placeholder_string(text) {
                None
            } else {
                Some(text.to_string())
            }
        }
        Value::Object(object) => object
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value))
            .map(ToString::to_string),
        _ => None,
    }
}

fn summarize_datasource_inventory_usage(
    datasource: &DatasourceInventoryItem,
    usage_by_label: &std::collections::BTreeMap<
        String,
        (usize, std::collections::BTreeSet<String>),
    >,
) -> (usize, usize) {
    let mut labels = Vec::new();
    if !datasource.uid.is_empty() {
        labels.push(datasource.uid.as_str());
    }
    if !datasource.name.is_empty() && datasource.name != datasource.uid {
        labels.push(datasource.name.as_str());
    }
    let mut reference_count = 0usize;
    let mut dashboards = std::collections::BTreeSet::new();
    for label in labels {
        if let Some((count, dashboard_uids)) = usage_by_label.get(label) {
            reference_count += *count;
            dashboards.extend(dashboard_uids.iter().cloned());
        }
    }
    (reference_count, dashboards.len())
}

fn string_list_field(target: &Map<String, Value>, key: &str) -> Vec<String> {
    target
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn quoted_captures(text: &str, pattern: &str) -> Vec<String> {
    let regex = Regex::new(pattern).expect("invalid hard-coded query report regex");
    let mut values = std::collections::BTreeSet::new();
    for captures in regex.captures_iter(text) {
        if let Some(value) = captures.get(1).map(|item| item.as_str().trim()) {
            if !value.is_empty() {
                values.insert(value.to_string());
            }
        }
    }
    values.into_iter().collect()
}

fn extract_query_field_and_text(target: &Map<String, Value>) -> (String, String) {
    for key in ["expr", "expression", "query", "rawSql", "sql", "rawQuery"] {
        if let Some(value) = target.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return (key.to_string(), trimmed.to_string());
            }
        }
    }
    (String::new(), String::new())
}

fn extract_metric_names(query_text: &str) -> Vec<String> {
    if query_text.trim().is_empty() {
        return Vec::new();
    }
    let token_regex =
        Regex::new(r"[A-Za-z_:][A-Za-z0-9_:]*").expect("invalid hard-coded metric regex");
    let mut values = std::collections::BTreeSet::new();
    let reserved_words = [
        "and",
        "bool",
        "by",
        "group_left",
        "group_right",
        "ignoring",
        "offset",
        "on",
        "or",
        "unless",
        "without",
    ];
    for capture in quoted_captures(query_text, r#"__name__\s*=\s*"([A-Za-z_:][A-Za-z0-9_:]*)""#) {
        values.insert(capture);
    }
    for matched in token_regex.find_iter(query_text) {
        let start = matched.start();
        let end = matched.end();
        let previous = query_text[..start].chars().next_back();
        if previous
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let next = query_text[end..].chars().next();
        if next
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let token = matched.as_str();
        if reserved_words.contains(&token) {
            continue;
        }
        if query_text[end..].trim_start().starts_with('(') {
            continue;
        }
        values.insert(token.to_string());
    }
    values.into_iter().collect()
}

fn extract_query_measurements(target: &Map<String, Value>, query_text: &str) -> Vec<String> {
    let mut values = std::collections::BTreeSet::new();
    if let Some(measurement) = target.get("measurement").and_then(Value::as_str) {
        let trimmed = measurement.trim();
        if !trimmed.is_empty() {
            values.insert(trimmed.to_string());
        }
    }
    for value in string_list_field(target, "measurements") {
        values.insert(value);
    }
    for value in quoted_captures(query_text, r#"(?i)\bFROM\s+"?([A-Za-z0-9_.:-]+)"?"#) {
        values.insert(value);
    }
    for value in quoted_captures(query_text, r#"_measurement\s*==\s*"([^"]+)""#) {
        values.insert(value);
    }
    values.into_iter().collect()
}

fn extract_query_buckets(target: &Map<String, Value>, query_text: &str) -> Vec<String> {
    let mut values = std::collections::BTreeSet::new();
    if let Some(bucket) = target.get("bucket").and_then(Value::as_str) {
        let trimmed = bucket.trim();
        if !trimmed.is_empty() {
            values.insert(trimmed.to_string());
        }
    }
    for value in string_list_field(target, "buckets") {
        values.insert(value);
    }
    for value in quoted_captures(query_text, r#"from\s*\(\s*bucket\s*:\s*"([^"]+)""#) {
        values.insert(value);
    }
    values.into_iter().collect()
}

fn collect_query_report_rows(
    panels: &[Value],
    dashboard_uid: &str,
    dashboard_title: &str,
    folder_path: &str,
    rows: &mut Vec<ExportInspectionQueryRow>,
) {
    for panel in panels {
        let Some(panel_object) = panel.as_object() else {
            continue;
        };
        let panel_id = panel_object
            .get("id")
            .map(|value| match value {
                Value::Number(number) => number.to_string(),
                Value::String(text) => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();
        let panel_title = string_field(panel_object, "title", "");
        let panel_type = string_field(panel_object, "type", "");
        let panel_datasource = panel_object.get("datasource");
        if let Some(targets) = panel_object.get("targets").and_then(Value::as_array) {
            for target in targets {
                let Some(target_object) = target.as_object() else {
                    continue;
                };
                let datasource = target_object
                    .get("datasource")
                    .and_then(summarize_datasource_ref)
                    .or_else(|| panel_datasource.and_then(summarize_datasource_ref))
                    .unwrap_or_default();
                let datasource_uid = target_object
                    .get("datasource")
                    .and_then(summarize_datasource_uid)
                    .or_else(|| panel_datasource.and_then(summarize_datasource_uid))
                    .unwrap_or_default();
                let (query_field, query_text) = extract_query_field_and_text(target_object);
                let metrics = extract_metric_names(&query_text);
                let measurements = extract_query_measurements(target_object, &query_text);
                let buckets = extract_query_buckets(target_object, &query_text);
                rows.push(ExportInspectionQueryRow {
                    dashboard_uid: dashboard_uid.to_string(),
                    dashboard_title: dashboard_title.to_string(),
                    folder_path: folder_path.to_string(),
                    panel_id: panel_id.clone(),
                    panel_title: panel_title.clone(),
                    panel_type: panel_type.clone(),
                    ref_id: string_field(target_object, "refId", ""),
                    datasource,
                    datasource_uid,
                    query_field,
                    query_text,
                    metrics,
                    measurements,
                    buckets,
                });
            }
        }
        if let Some(children) = panel_object.get("panels").and_then(Value::as_array) {
            collect_query_report_rows(children, dashboard_uid, dashboard_title, folder_path, rows);
        }
    }
}

fn build_export_inspection_query_report(import_dir: &Path) -> Result<ExportInspectionQueryReport> {
    let summary = build_export_inspection_summary(import_dir)?;
    let metadata = load_export_metadata(import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let dashboard_files = discover_dashboard_files(import_dir)?;
    let folder_inventory = load_folder_inventory(import_dir, metadata.as_ref())?;
    let folders_by_uid = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect::<std::collections::BTreeMap<String, FolderInventoryItem>>();
    let mut rows = Vec::new();

    for dashboard_file in &dashboard_files {
        let document = load_json_file(dashboard_file)?;
        let document_object =
            value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let folder_path = resolve_export_folder_path(
            document_object,
            dashboard_file,
            import_dir,
            &folders_by_uid,
        );
        let dashboard_uid = string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
        let dashboard_title = string_field(dashboard, "title", DEFAULT_DASHBOARD_TITLE);
        if let Some(panels) = dashboard.get("panels").and_then(Value::as_array) {
            collect_query_report_rows(
                panels,
                &dashboard_uid,
                &dashboard_title,
                &folder_path,
                &mut rows,
            );
        }
    }

    Ok(ExportInspectionQueryReport {
        import_dir: summary.import_dir.clone(),
        summary: QueryReportSummary {
            dashboard_count: summary.dashboard_count,
            panel_count: summary.panel_count,
            query_count: summary.query_count,
            report_row_count: rows.len(),
        },
        queries: rows,
    })
}

fn resolve_report_column_ids(selected: &[String]) -> Result<Vec<String>> {
    if selected.is_empty() {
        return Ok(DEFAULT_REPORT_COLUMN_IDS
            .iter()
            .map(|value| value.to_string())
            .collect());
    }
    let mut result = Vec::new();
    for value in selected {
        let normalized = value.trim();
        if normalized.is_empty() {
            continue;
        }
        if !SUPPORTED_REPORT_COLUMN_IDS.contains(&normalized) {
            return Err(message(format!(
                "Unsupported --report-columns value {:?}. Supported columns: {}",
                normalized,
                SUPPORTED_REPORT_COLUMN_IDS.join(",")
            )));
        }
        if !result.iter().any(|item| item == normalized) {
            result.push(normalized.to_string());
        }
    }
    if result.is_empty() {
        return Err(message(format!(
            "--report-columns did not include any supported columns. Supported columns: {}",
            SUPPORTED_REPORT_COLUMN_IDS.join(",")
        )));
    }
    Ok(result)
}

fn report_column_header(column_id: &str) -> &'static str {
    match column_id {
        "dashboard_uid" => "DASHBOARD_UID",
        "dashboard_title" => "DASHBOARD_TITLE",
        "folder_path" => "FOLDER_PATH",
        "panel_id" => "PANEL_ID",
        "panel_title" => "PANEL_TITLE",
        "panel_type" => "PANEL_TYPE",
        "ref_id" => "REF_ID",
        "datasource" => "DATASOURCE",
        "datasource_uid" => "DATASOURCE_UID",
        "query_field" => "QUERY_FIELD",
        "metrics" => "METRICS",
        "measurements" => "MEASUREMENTS",
        "buckets" => "BUCKETS",
        "query" => "QUERY",
        _ => unreachable!("unsupported report column header"),
    }
}

fn render_query_report_column(row: &ExportInspectionQueryRow, column_id: &str) -> String {
    match column_id {
        "dashboard_uid" => row.dashboard_uid.clone(),
        "dashboard_title" => row.dashboard_title.clone(),
        "folder_path" => row.folder_path.clone(),
        "panel_id" => row.panel_id.clone(),
        "panel_title" => row.panel_title.clone(),
        "panel_type" => row.panel_type.clone(),
        "ref_id" => row.ref_id.clone(),
        "datasource" => row.datasource.clone(),
        "datasource_uid" => row.datasource_uid.clone(),
        "query_field" => row.query_field.clone(),
        "metrics" => row.metrics.join(","),
        "measurements" => row.measurements.join(","),
        "buckets" => row.buckets.join(","),
        "query" => row.query_text.clone(),
        _ => unreachable!("unsupported report column value"),
    }
}

fn apply_query_report_filters(
    mut report: ExportInspectionQueryReport,
    datasource_filter: Option<&str>,
    panel_id_filter: Option<&str>,
) -> ExportInspectionQueryReport {
    let datasource_filter = datasource_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let panel_id_filter = panel_id_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if datasource_filter.is_none() && panel_id_filter.is_none() {
        return report;
    }
    report.queries.retain(|row| {
        let datasource_match = datasource_filter
            .map(|value| row.datasource == value)
            .unwrap_or(true);
        let panel_match = panel_id_filter
            .map(|value| row.panel_id == value)
            .unwrap_or(true);
        datasource_match && panel_match
    });
    report.summary.dashboard_count = report
        .queries
        .iter()
        .map(|row| row.dashboard_uid.clone())
        .collect::<std::collections::BTreeSet<String>>()
        .len();
    report.summary.panel_count = report
        .queries
        .iter()
        .map(|row| {
            (
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                row.panel_title.clone(),
            )
        })
        .collect::<std::collections::BTreeSet<(String, String, String)>>()
        .len();
    report.summary.query_count = report.queries.len();
    report.summary.report_row_count = report.queries.len();
    report
}

fn validate_inspect_export_report_args(args: &InspectExportArgs) -> Result<()> {
    if args.report.is_none() {
        if !args.report_columns.is_empty() {
            return Err(message(
                "--report-columns is only supported together with --report.",
            ));
        }
        if args.report_filter_datasource.is_some() {
            return Err(message(
                "--report-filter-datasource is only supported together with --report.",
            ));
        }
        if args.report_filter_panel_id.is_some() {
            return Err(message(
                "--report-filter-panel-id is only supported together with --report.",
            ));
        }
        return Ok(());
    }
    if args.report == Some(InspectExportReportFormat::Json) && !args.report_columns.is_empty() {
        return Err(message(
            "--report-columns is only supported with table or csv --report output.",
        ));
    }
    let _ = resolve_report_column_ids(&args.report_columns)?;
    Ok(())
}

fn build_export_inspection_summary(import_dir: &Path) -> Result<ExportInspectionSummary> {
    let metadata = load_export_metadata(import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let dashboard_files = discover_dashboard_files(import_dir)?;
    let folder_inventory = load_folder_inventory(import_dir, metadata.as_ref())?;
    let datasource_inventory = load_datasource_inventory(import_dir, metadata.as_ref())?;
    let folders_by_uid = folder_inventory
        .clone()
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect::<std::collections::BTreeMap<String, FolderInventoryItem>>();

    let mut folder_order = Vec::new();
    let mut folder_counts = std::collections::BTreeMap::new();
    let mut datasource_counts =
        std::collections::BTreeMap::<String, (usize, std::collections::BTreeSet<String>)>::new();
    let mut mixed_dashboards = Vec::new();
    let mut total_panels = 0usize;
    let mut total_queries = 0usize;

    let mut inventory_paths = folder_inventory
        .iter()
        .filter_map(|item| {
            let path = item.path.trim();
            if path.is_empty() {
                None
            } else {
                Some(path.to_string())
            }
        })
        .collect::<Vec<String>>();
    inventory_paths.sort();
    inventory_paths.dedup();
    for path in inventory_paths {
        folder_order.push(path.clone());
        folder_counts.insert(path, 0usize);
    }

    for dashboard_file in &dashboard_files {
        let document = load_json_file(dashboard_file)?;
        let document_object =
            value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let uid = string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
        let title = string_field(dashboard, "title", DEFAULT_DASHBOARD_TITLE);
        let folder_path = resolve_export_folder_path(
            document_object,
            dashboard_file,
            import_dir,
            &folders_by_uid,
        );
        if !folder_counts.contains_key(&folder_path) {
            folder_order.push(folder_path.clone());
            folder_counts.insert(folder_path.clone(), 0usize);
        }
        *folder_counts.entry(folder_path.clone()).or_insert(0usize) += 1;

        let (panel_count, query_count) = count_dashboard_panels_and_queries(dashboard);
        total_panels += panel_count;
        total_queries += query_count;

        let mut refs = Vec::new();
        collect_datasource_refs(&Value::Object(dashboard.clone()), &mut refs);
        let mut unique_datasources = std::collections::BTreeSet::new();
        for reference in refs {
            if let Some(label) = summarize_datasource_ref(&reference) {
                let usage = datasource_counts
                    .entry(label.clone())
                    .or_insert_with(|| (0usize, std::collections::BTreeSet::new()));
                usage.0 += 1;
                usage.1.insert(uid.clone());
                unique_datasources.insert(label);
            }
        }
        if unique_datasources.len() > 1 {
            mixed_dashboards.push(MixedDashboardSummary {
                uid,
                title,
                folder_path,
                datasource_count: unique_datasources.len(),
                datasources: unique_datasources.into_iter().collect(),
            });
        }
    }

    let folder_paths = folder_order
        .into_iter()
        .map(|path| ExportFolderUsage {
            dashboards: *folder_counts.get(&path).unwrap_or(&0usize),
            path,
        })
        .collect::<Vec<ExportFolderUsage>>();
    let mut datasource_usage = datasource_counts
        .iter()
        .map(
            |(datasource, (reference_count, dashboards))| ExportDatasourceUsage {
                datasource: datasource.clone(),
                reference_count: *reference_count,
                dashboard_count: dashboards.len(),
            },
        )
        .collect::<Vec<ExportDatasourceUsage>>();
    datasource_usage.sort_by(|left, right| left.datasource.cmp(&right.datasource));
    let mut datasource_inventory_summary = datasource_inventory
        .iter()
        .map(|datasource| {
            let (reference_count, dashboard_count) =
                summarize_datasource_inventory_usage(datasource, &datasource_counts);
            DatasourceInventorySummary {
                uid: datasource.uid.clone(),
                name: datasource.name.clone(),
                datasource_type: datasource.datasource_type.clone(),
                access: datasource.access.clone(),
                url: datasource.url.clone(),
                is_default: datasource.is_default.clone(),
                org: datasource.org.clone(),
                org_id: datasource.org_id.clone(),
                reference_count,
                dashboard_count,
            }
        })
        .collect::<Vec<DatasourceInventorySummary>>();
    datasource_inventory_summary.sort_by(|left, right| {
        left.org_id
            .cmp(&right.org_id)
            .then(left.name.cmp(&right.name))
            .then(left.uid.cmp(&right.uid))
    });
    mixed_dashboards.sort_by(|left, right| {
        left.folder_path
            .cmp(&right.folder_path)
            .then(left.title.cmp(&right.title))
            .then(left.uid.cmp(&right.uid))
    });

    Ok(ExportInspectionSummary {
        import_dir: import_dir.display().to_string(),
        dashboard_count: dashboard_files.len(),
        folder_count: folder_paths.len(),
        panel_count: total_panels,
        query_count: total_queries,
        datasource_inventory_count: datasource_inventory_summary.len(),
        mixed_dashboard_count: mixed_dashboards.len(),
        folder_paths,
        datasource_usage,
        datasource_inventory: datasource_inventory_summary,
        mixed_dashboards,
    })
}

fn analyze_export_dir(args: &InspectExportArgs) -> Result<usize> {
    validate_inspect_export_report_args(args)?;
    if let Some(report_format) = args.report {
        let report = apply_query_report_filters(
            build_export_inspection_query_report(&args.import_dir)?,
            args.report_filter_datasource.as_deref(),
            args.report_filter_panel_id.as_deref(),
        );
        if report_format == InspectExportReportFormat::Json {
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(report.summary.dashboard_count);
        }

        let column_ids = resolve_report_column_ids(&args.report_columns)?;
        let rows = report
            .queries
            .iter()
            .map(|item| {
                column_ids
                    .iter()
                    .map(|column_id| render_query_report_column(item, column_id))
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<Vec<String>>>();
        let headers = column_ids
            .iter()
            .map(|column_id| report_column_header(column_id))
            .collect::<Vec<&str>>();

        if report_format == InspectExportReportFormat::Csv {
            for line in render_csv(&headers, &rows) {
                println!("{line}");
            }
            return Ok(report.summary.dashboard_count);
        }

        println!("Export inspection report: {}", report.import_dir);
        println!();
        println!("# Query report");
        for line in render_simple_table(&headers, &rows, !args.no_header) {
            println!("{line}");
        }
        return Ok(report.summary.dashboard_count);
    }

    let summary = build_export_inspection_summary(&args.import_dir)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(summary.dashboard_count);
    }

    println!("Export inspection: {}", summary.import_dir);
    if args.table {
        println!();
        println!("# Summary");
        let summary_rows = vec![
            vec![
                "dashboard_count".to_string(),
                summary.dashboard_count.to_string(),
            ],
            vec!["folder_count".to_string(), summary.folder_count.to_string()],
            vec!["panel_count".to_string(), summary.panel_count.to_string()],
            vec!["query_count".to_string(), summary.query_count.to_string()],
            vec![
                "datasource_inventory_count".to_string(),
                summary.datasource_inventory_count.to_string(),
            ],
            vec![
                "mixed_datasource_dashboard_count".to_string(),
                summary.mixed_dashboard_count.to_string(),
            ],
        ];
        for line in render_simple_table(&["METRIC", "VALUE"], &summary_rows, !args.no_header) {
            println!("{line}");
        }
    } else {
        println!("Dashboards: {}", summary.dashboard_count);
        println!("Folders: {}", summary.folder_count);
        println!("Panels: {}", summary.panel_count);
        println!("Queries: {}", summary.query_count);
        println!(
            "Datasource inventory: {}",
            summary.datasource_inventory_count
        );
        println!(
            "Mixed datasource dashboards: {}",
            summary.mixed_dashboard_count
        );
    }

    println!();
    println!("# Folder paths");
    let folder_rows = summary
        .folder_paths
        .iter()
        .map(|item| vec![item.path.clone(), item.dashboards.to_string()])
        .collect::<Vec<Vec<String>>>();
    for line in render_simple_table(
        &["FOLDER_PATH", "DASHBOARDS"],
        &folder_rows,
        !args.no_header,
    ) {
        println!("{line}");
    }

    println!();
    println!("# Datasource usage");
    let datasource_rows = summary
        .datasource_usage
        .iter()
        .map(|item| {
            vec![
                item.datasource.clone(),
                item.reference_count.to_string(),
                item.dashboard_count.to_string(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    for line in render_simple_table(
        &["DATASOURCE", "REFS", "DASHBOARDS"],
        &datasource_rows,
        !args.no_header,
    ) {
        println!("{line}");
    }

    if !summary.datasource_inventory.is_empty() {
        println!();
        println!("# Datasource inventory");
        let datasource_inventory_rows = summary
            .datasource_inventory
            .iter()
            .map(|item| {
                vec![
                    item.org_id.clone(),
                    item.uid.clone(),
                    item.name.clone(),
                    item.datasource_type.clone(),
                    item.access.clone(),
                    item.url.clone(),
                    item.is_default.clone(),
                    item.reference_count.to_string(),
                    item.dashboard_count.to_string(),
                ]
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(
            &[
                "ORG_ID",
                "UID",
                "NAME",
                "TYPE",
                "ACCESS",
                "URL",
                "IS_DEFAULT",
                "REFS",
                "DASHBOARDS",
            ],
            &datasource_inventory_rows,
            !args.no_header,
        ) {
            println!("{line}");
        }
    }

    if !summary.mixed_dashboards.is_empty() {
        println!();
        println!("# Mixed datasource dashboards");
        let mixed_rows = summary
            .mixed_dashboards
            .iter()
            .map(|item| {
                vec![
                    item.uid.clone(),
                    item.title.clone(),
                    item.folder_path.clone(),
                    item.datasources.join(","),
                ]
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(
            &["UID", "TITLE", "FOLDER_PATH", "DATASOURCES"],
            &mixed_rows,
            !args.no_header,
        ) {
            println!("{line}");
        }
    }
    Ok(summary.dashboard_count)
}

struct TempInspectLiveDir {
    path: PathBuf,
}

impl TempInspectLiveDir {
    fn new() -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| message(format!("Failed to build inspect-live temp path: {error}")))?
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "grafana-utils-inspect-live-{}-{timestamp}",
            process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempInspectLiveDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn build_live_export_args(args: &InspectLiveArgs, export_dir: PathBuf) -> ExportArgs {
    ExportArgs {
        common: args.common.clone(),
        export_dir,
        page_size: args.page_size,
        org_id: args.org_id,
        all_orgs: args.all_orgs,
        flat: false,
        overwrite: false,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        dry_run: false,
        progress: false,
        verbose: false,
    }
}

fn build_export_inspect_args_from_live(args: &InspectLiveArgs, import_dir: PathBuf) -> InspectExportArgs {
    InspectExportArgs {
        import_dir,
        json: args.json,
        table: args.table,
        report: args.report,
        report_columns: args.report_columns.clone(),
        report_filter_datasource: args.report_filter_datasource.clone(),
        report_filter_panel_id: args.report_filter_panel_id.clone(),
        no_header: args.no_header,
    }
}

fn inspect_live_dashboards_with_request<F>(
    mut request_json: F,
    args: &InspectLiveArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.all_orgs {
        return Err(message(
            "inspect-live does not yet support --all-orgs. Export dashboards first or inspect one org at a time.",
        ));
    }
    let temp_dir = TempInspectLiveDir::new()?;
    let export_args = build_live_export_args(args, temp_dir.path.clone());
    let _ = dashboard_export::export_dashboards_with_request(&mut request_json, &export_args)?;
    let inspect_args =
        build_export_inspect_args_from_live(args, temp_dir.path.join(RAW_EXPORT_SUBDIR));
    analyze_export_dir(&inspect_args)
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

pub(crate) fn format_import_progress_line(
    current: usize,
    total: usize,
    dashboard_target: &str,
    dry_run: bool,
    action: Option<&str>,
    folder_path: Option<&str>,
) -> String {
    if dry_run {
        let (destination, action_label) =
            describe_import_action(action.unwrap_or(DEFAULT_UNKNOWN_UID));
        let mut line = format!(
            "Dry-run dashboard {current}/{total}: {dashboard_target} dest={destination} action={action_label}"
        );
        if let Some(path) = folder_path.filter(|value| !value.is_empty()) {
            let _ = write!(&mut line, " folderPath={path}");
        }
        line
    } else {
        format!("Importing dashboard {current}/{total}: {dashboard_target}")
    }
}

pub(crate) fn format_import_verbose_line(
    dashboard_file: &Path,
    dry_run: bool,
    uid: Option<&str>,
    action: Option<&str>,
    folder_path: Option<&str>,
) -> String {
    if dry_run {
        let (destination, action_label) =
            describe_import_action(action.unwrap_or(DEFAULT_UNKNOWN_UID));
        let mut line = format!(
            "Dry-run import uid={} dest={} action={} file={}",
            uid.unwrap_or(DEFAULT_UNKNOWN_UID),
            destination,
            action_label,
            dashboard_file.display()
        );
        if let Some(path) = folder_path.filter(|value| !value.is_empty()) {
            line = format!(
                "Dry-run import uid={} dest={} action={} folderPath={} file={}",
                uid.unwrap_or(DEFAULT_UNKNOWN_UID),
                destination,
                action_label,
                path,
                dashboard_file.display()
            );
        }
        line
    } else {
        format!("Imported {}", dashboard_file.display())
    }
}

fn import_dashboards_with_request<F>(mut request_json: F, args: &ImportArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.table && !args.dry_run {
        return Err(message(
            "--table is only supported with --dry-run for import-dashboard.",
        ));
    }
    if args.json && !args.dry_run {
        return Err(message(
            "--json is only supported with --dry-run for import-dashboard.",
        ));
    }
    if args.table && args.json {
        return Err(message(
            "--table and --json are mutually exclusive for import-dashboard.",
        ));
    }
    if args.no_header && !args.table {
        return Err(message(
            "--no-header is only supported with --dry-run --table for import-dashboard.",
        ));
    }
    if args.ensure_folders && args.import_folder_uid.is_some() {
        return Err(message(
            "--ensure-folders cannot be combined with --import-folder-uid.",
        ));
    }
    let metadata = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let folder_inventory = if args.ensure_folders {
        load_folder_inventory(&args.import_dir, metadata.as_ref())?
    } else {
        Vec::new()
    };
    if args.ensure_folders && folder_inventory.is_empty() {
        let folders_file = metadata
            .as_ref()
            .and_then(|item| item.folders_file.as_deref())
            .unwrap_or(FOLDER_INVENTORY_FILENAME);
        return Err(message(format!(
            "Folder inventory file not found for --ensure-folders: {}. Re-export dashboards with raw folder inventory or omit --ensure-folders.",
            args.import_dir.join(folders_file).display()
        )));
    }
    let folder_statuses = if args.dry_run && args.ensure_folders {
        collect_folder_inventory_statuses_with_request(&mut request_json, &folder_inventory)?
    } else {
        Vec::new()
    };
    let folders_by_uid: std::collections::BTreeMap<String, FolderInventoryItem> = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect();
    let mut dashboard_files = discover_dashboard_files(&args.import_dir)?;
    dashboard_files.retain(|path| {
        path.file_name().and_then(|name| name.to_str()) != Some(FOLDER_INVENTORY_FILENAME)
    });
    let total = dashboard_files.len();
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let mut dry_run_records: Vec<[String; 5]> = Vec::new();
    let mut imported_count = 0usize;
    let mut skipped_missing_count = 0usize;
    let mode = describe_dashboard_import_mode(args.replace_existing, args.update_existing_only);
    if !args.json {
        println!("Import mode: {}", mode);
    }
    if args.dry_run && args.ensure_folders {
        let folder_dry_run_records: Vec<[String; 6]> = folder_statuses
            .iter()
            .map(build_folder_inventory_dry_run_record)
            .collect();
        if args.json {
        } else if args.table {
            for line in
                render_folder_inventory_dry_run_table(&folder_dry_run_records, !args.no_header)
            {
                println!("{line}");
            }
        } else {
            for status in &folder_statuses {
                println!("{}", format_folder_inventory_status_line(status));
            }
        }
        let missing_folder_count = folder_statuses
            .iter()
            .filter(|status| status.kind == FolderInventoryStatusKind::Missing)
            .count();
        let mismatched_folder_count = folder_statuses
            .iter()
            .filter(|status| status.kind == FolderInventoryStatusKind::Mismatch)
            .count();
        let folders_file = metadata
            .as_ref()
            .and_then(|item| item.folders_file.as_deref())
            .unwrap_or(FOLDER_INVENTORY_FILENAME);
        if !args.json {
            println!(
                "Dry-run checked {} folder(s) from {}; {} missing, {} mismatched",
                folder_statuses.len(),
                args.import_dir.join(folders_file).display(),
                missing_folder_count,
                mismatched_folder_count
            );
        }
    }
    for (index, dashboard_file) in dashboard_files.iter().enumerate() {
        if dashboard_file.file_name().and_then(|name| name.to_str())
            == Some(FOLDER_INVENTORY_FILENAME)
        {
            continue;
        }
        let document = load_json_file(dashboard_file)?;
        let document_object =
            value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let uid = string_field(dashboard, "uid", "");
        let folder_uid_override = determine_import_folder_uid_override_with_request(
            &mut request_json,
            &uid,
            args.import_folder_uid.as_deref(),
            effective_replace_existing,
        )?;
        let payload = build_import_payload(
            &document,
            folder_uid_override.as_deref(),
            effective_replace_existing,
            &args.import_message,
        )?;
        let action = if args.dry_run || args.update_existing_only || args.ensure_folders {
            Some(determine_dashboard_import_action_with_request(
                &mut request_json,
                &payload,
                args.replace_existing,
                args.update_existing_only,
            )?)
        } else {
            None
        };
        if args.dry_run {
            let folder_path = resolve_dashboard_import_folder_path_with_request(
                &mut request_json,
                &payload,
                &folders_by_uid,
            )?;
            let payload_object =
                value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
            let dashboard = payload_object
                .get("dashboard")
                .and_then(Value::as_object)
                .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
            let uid = string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
            if args.table || args.json {
                dry_run_records.push(build_import_dry_run_record(
                    dashboard_file,
                    &uid,
                    action.unwrap_or(DEFAULT_UNKNOWN_UID),
                    &folder_path,
                ));
            } else if args.verbose {
                println!(
                    "{}",
                    format_import_verbose_line(
                        dashboard_file,
                        true,
                        Some(&uid),
                        Some(action.unwrap_or(DEFAULT_UNKNOWN_UID)),
                        Some(&folder_path),
                    )
                );
            } else if args.progress {
                println!(
                    "{}",
                    format_import_progress_line(
                        index + 1,
                        total,
                        &uid,
                        true,
                        Some(action.unwrap_or(DEFAULT_UNKNOWN_UID)),
                        Some(&folder_path),
                    )
                );
            }
            continue;
        }
        if args.update_existing_only {
            let payload_object =
                value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
            let dashboard = payload_object
                .get("dashboard")
                .and_then(Value::as_object)
                .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
            let uid = string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
            if action == Some("would-skip-missing") {
                skipped_missing_count += 1;
                if args.verbose {
                    println!(
                        "Skipped import uid={} dest=missing action=skip-missing file={}",
                        uid,
                        dashboard_file.display()
                    );
                } else if args.progress {
                    println!(
                        "Skipping dashboard {}/{}: {} dest=missing action=skip-missing",
                        index + 1,
                        total,
                        uid
                    );
                }
                continue;
            }
        }
        if args.ensure_folders {
            let payload_object =
                value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
            let folder_uid = payload_object
                .get("folderUid")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !folder_uid.is_empty() && action != Some("would-fail-existing") {
                ensure_folder_inventory_entry_with_request(
                    &mut request_json,
                    &folders_by_uid,
                    folder_uid,
                )?;
            }
        }
        let _result = import_dashboard_request_with_request(&mut request_json, &payload)?;
        imported_count += 1;
        if args.verbose {
            println!(
                "{}",
                format_import_verbose_line(dashboard_file, false, None, None, None)
            );
        } else if args.progress {
            println!(
                "{}",
                format_import_progress_line(
                    index + 1,
                    total,
                    &dashboard_file.display().to_string(),
                    false,
                    None,
                    None,
                )
            );
        }
    }
    if args.dry_run {
        if args.update_existing_only {
            skipped_missing_count = dry_run_records
                .iter()
                .filter(|record| record[2] == "skip-missing")
                .count();
        }
        if args.json {
            println!(
                "{}",
                render_import_dry_run_json(
                    mode,
                    &folder_statuses,
                    &dry_run_records,
                    &args.import_dir,
                    skipped_missing_count,
                )?
            );
        } else if args.table {
            for line in render_import_dry_run_table(&dry_run_records, !args.no_header) {
                println!("{line}");
            }
        }
        if args.json {
        } else if args.update_existing_only && skipped_missing_count > 0 {
            println!(
                "Dry-run checked {} dashboard(s) from {}; would skip {} missing dashboards",
                dashboard_files.len(),
                args.import_dir.display(),
                skipped_missing_count
            );
        } else {
            println!(
                "Dry-run checked {} dashboard(s) from {}",
                dashboard_files.len(),
                args.import_dir.display()
            );
        }
        return Ok(dashboard_files.len());
    }
    if args.update_existing_only && skipped_missing_count > 0 {
        println!(
            "Imported {} dashboard files from {}; skipped {} missing dashboards",
            imported_count,
            args.import_dir.display(),
            skipped_missing_count
        );
    }
    Ok(imported_count)
}

pub fn import_dashboards_with_client(client: &JsonHttpClient, args: &ImportArgs) -> Result<usize> {
    import_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

fn diff_dashboards_with_request<F>(mut request_json: F, args: &DiffArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let _ = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let dashboard_files = discover_dashboard_files(&args.import_dir)?;
    let mut differences = 0;
    for dashboard_file in &dashboard_files {
        let document = load_json_file(dashboard_file)?;
        let payload = build_import_payload(&document, None, false, "")?;
        let payload_object =
            value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
        let dashboard = payload_object
            .get("dashboard")
            .and_then(Value::as_object)
            .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
        let uid = string_field(dashboard, "uid", "");
        let local_compare =
            build_local_compare_document(&document, args.import_folder_uid.as_deref())?;
        let Some(remote_payload) = fetch_dashboard_if_exists_with_request(&mut request_json, &uid)?
        else {
            println!(
                "Diff missing in Grafana for uid={} from {}",
                uid,
                dashboard_file.display()
            );
            differences += 1;
            continue;
        };
        let remote_compare =
            build_remote_compare_document(&remote_payload, args.import_folder_uid.as_deref())?;
        if serialize_compare_document(&local_compare)?
            != serialize_compare_document(&remote_compare)?
        {
            let diff_text = build_compare_diff_text(
                &remote_compare,
                &local_compare,
                &uid,
                dashboard_file,
                args.context_lines,
            )?;
            println!("{diff_text}");
            differences += 1;
        } else {
            println!("Diff matched uid={} for {}", uid, dashboard_file.display());
        }
    }
    println!(
        "Diff checked {} dashboard(s); {} difference(s) found.",
        dashboard_files.len(),
        differences
    );
    Ok(differences)
}

pub fn diff_dashboards_with_client(client: &JsonHttpClient, args: &DiffArgs) -> Result<usize> {
    diff_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
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
            let _ = analyze_export_dir(&inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectLive(inspect_args) => {
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
            let client = build_http_client(&import_args.common)?;
            let _ = import_dashboards_with_client(&client, &import_args)?;
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
            let _ = analyze_export_dir(&inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectLive(inspect_args) => {
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
