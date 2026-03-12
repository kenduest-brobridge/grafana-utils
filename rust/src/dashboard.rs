use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    message, object_field, string_field, value_as_object, Result,
};
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
    build_auth_context, build_http_client, build_http_client_for_org, parse_cli_from, CommonCliArgs,
    DashboardAuthContext, DashboardCliArgs, DashboardCommand, DiffArgs, ExportArgs, ImportArgs,
    ListArgs, ListDataSourcesArgs,
};
pub use dashboard_export::{build_export_variant_dirs, build_output_path, export_dashboards_with_client};
pub use dashboard_list::{list_dashboards_with_client, list_data_sources_with_client};
pub use dashboard_prompt::build_external_export_document;

use dashboard_export::export_dashboards_with_org_clients;
use dashboard_list::list_dashboards_with_org_clients;

#[cfg(test)]
pub(crate) use dashboard_export::export_dashboards_with_request;
#[cfg(test)]
pub(crate) use dashboard_list::{
    attach_dashboard_folder_paths_with_request, collect_dashboard_source_metadata,
    format_dashboard_summary_line, format_data_source_line, list_dashboards_with_request,
    list_data_sources_with_request, render_dashboard_summary_csv, render_dashboard_summary_json,
    render_dashboard_summary_table, render_data_source_csv, render_data_source_json,
    render_data_source_table,
};
pub(crate) use dashboard_prompt::{
    build_datasource_catalog, collect_datasource_refs, datasource_type_alias, is_builtin_datasource_ref,
    is_placeholder_string, lookup_datasource, resolve_datasource_type_alias,
};

pub const DEFAULT_URL: &str = "http://localhost:3000";
pub const DEFAULT_TIMEOUT: u64 = 30;
pub const DEFAULT_PAGE_SIZE: usize = 500;
pub const DEFAULT_EXPORT_DIR: &str = "dashboards";
pub const RAW_EXPORT_SUBDIR: &str = "raw";
pub const PROMPT_EXPORT_SUBDIR: &str = "prompt";
pub const DEFAULT_IMPORT_MESSAGE: &str = "Imported by grafana-utils";
pub const EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
pub const TOOL_SCHEMA_VERSION: i64 = 1;
pub const ROOT_INDEX_KIND: &str = "grafana-utils-dashboard-export-index";
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
}

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
    if import_dir.join(RAW_EXPORT_SUBDIR).is_dir() && import_dir.join(PROMPT_EXPORT_SUBDIR).is_dir() {
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
        file_name != Some("index.json") && file_name != Some(EXPORT_METADATA_FILENAME)
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

fn build_export_metadata(variant: &str, dashboard_count: usize, format_name: Option<&str>) -> ExportMetadata {
    ExportMetadata {
        schema_version: TOOL_SCHEMA_VERSION,
        kind: ROOT_INDEX_KIND.to_string(),
        variant: variant.to_string(),
        dashboard_count: dashboard_count as u64,
        index_file: "index.json".to_string(),
        format: format_name.map(str::to_owned),
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

fn load_export_metadata(import_dir: &Path, expected_variant: Option<&str>) -> Result<Option<ExportMetadata>> {
    let metadata_path = import_dir.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Ok(None);
    }
    let value = load_json_file(&metadata_path)?;
    value_as_object(&value, "Dashboard export metadata must be a JSON object.")?;
    let metadata: ExportMetadata = serde_json::from_value(value)
        .map_err(|error| message(format!("Invalid dashboard export metadata in {}: {error}", metadata_path.display())))?;
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

    let folder_uid = folder_uid_override
        .map(str::to_owned)
        .or_else(|| {
            object_field(document_object, "meta")
                .and_then(|meta| meta.get("folderUid"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        });

    let mut payload = Map::new();
    payload.insert("dashboard".to_string(), Value::Object(dashboard));
    payload.insert("overwrite".to_string(), Value::Bool(replace_existing));
    payload.insert("message".to_string(), Value::String(message_text.to_string()));
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
        title: string_field(summary, "title", "dashboard"),
        folder_title: string_field(summary, "folderTitle", "General"),
        org: string_field(summary, "orgName", "Main Org."),
        org_id: summary
            .get("orgId")
            .map(|value| match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            })
            .unwrap_or_else(|| "1".to_string()),
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
) -> RootExportIndex {
    RootExportIndex {
        schema_version: TOOL_SCHEMA_VERSION,
        kind: ROOT_INDEX_KIND.to_string(),
        items: items.to_vec(),
        variants: RootExportVariants {
            raw: raw_index_path.map(|path| path.display().to_string()),
            prompt: prompt_index_path.map(|path| path.display().to_string()),
        },
    }
}

fn list_dashboard_summaries_with_request<F>(mut request_json: F, page_size: usize) -> Result<Vec<Map<String, Value>>>
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
            let object = value_as_object(&item, "Unexpected dashboard summary payload from Grafana.")?;
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

pub fn list_dashboard_summaries(client: &JsonHttpClient, page_size: usize) -> Result<Vec<Map<String, Value>>> {
    list_dashboard_summaries_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        page_size,
    )
}

fn fetch_folder_if_exists_with_request<F>(mut request_json: F, uid: &str) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, &format!("/api/folders/{uid}"), &[], None)? {
        Some(value) => {
            let object = value_as_object(&value, &format!("Unexpected folder payload for UID {uid}."))?;
            Ok(Some(object.clone()))
        }
        None => Ok(None),
    }
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

fn fetch_dashboard_with_request<F>(mut request_json: F, uid: &str) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, &format!("/api/dashboards/uid/{uid}"), &[], None)? {
        Some(value) => {
            let object = value_as_object(&value, &format!("Unexpected dashboard payload for UID {uid}."))?;
            if !object.contains_key("dashboard") {
                return Err(message(format!("Unexpected dashboard payload for UID {uid}.")));
            }
            Ok(value)
        }
        None => Err(message(format!("Unexpected empty dashboard payload for UID {uid}."))),
    }
}

pub fn fetch_dashboard(client: &JsonHttpClient, uid: &str) -> Result<Value> {
    fetch_dashboard_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        uid,
    )
}

fn fetch_dashboard_if_exists_with_request<F>(mut request_json: F, uid: &str) -> Result<Option<Value>>
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
        None => Err(message("Unexpected empty dashboard import response from Grafana.")),
    }
}

pub fn import_dashboard_request(client: &JsonHttpClient, payload: &Value) -> Result<Value> {
    import_dashboard_request_with_request(
        |method, path, params, request_payload| client.request_json(method, path, params, request_payload),
        payload,
    )
}

fn build_compare_document(dashboard: &Map<String, Value>, folder_uid: Option<&str>) -> Value {
    let mut compare = Map::new();
    compare.insert("dashboard".to_string(), Value::Object(dashboard.clone()));
    if let Some(folder_uid) = folder_uid.filter(|value| !value.is_empty()) {
        compare.insert("folderUid".to_string(), Value::String(folder_uid.to_string()));
    }
    Value::Object(compare)
}

fn build_local_compare_document(document: &Value, folder_uid_override: Option<&str>) -> Result<Value> {
    let payload = build_import_payload(document, folder_uid_override, false, "")?;
    let payload_object = value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
    let dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let folder_uid = payload_object.get("folderUid").and_then(Value::as_str);
    Ok(build_compare_document(dashboard, folder_uid))
}

fn build_remote_compare_document(payload: &Value, folder_uid_override: Option<&str>) -> Result<Value> {
    let dashboard = build_preserved_web_import_document(payload)?;
    let dashboard_object = value_as_object(&dashboard, "Unexpected dashboard payload from Grafana.")?;
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
) -> Result<&'static str>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload_object = value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let uid = string_field(dashboard, "uid", "");
    if uid.is_empty() {
        return Ok("would-create");
    }
    if fetch_dashboard_if_exists_with_request(&mut request_json, &uid)?.is_none() {
        return Ok("would-create");
    }
    if replace_existing {
        Ok("would-update")
    } else {
        Ok("would-fail-existing")
    }
}

fn list_datasources_with_request<F>(mut request_json: F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| {
                value_as_object(&item, "Unexpected datasource payload from Grafana.")
                    .map(|object| object.clone())
            })
            .collect(),
        Some(_) => Err(message("Unexpected datasource list response from Grafana.")),
        None => Ok(Vec::new()),
    }
}

pub fn list_datasources(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    list_datasources_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
    )
}

fn import_dashboards_with_request<F>(mut request_json: F, args: &ImportArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let _ = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let dashboard_files = discover_dashboard_files(&args.import_dir)?;
    for dashboard_file in &dashboard_files {
        let document = load_json_file(dashboard_file)?;
        let payload = build_import_payload(
            &document,
            args.import_folder_uid.as_deref(),
            args.replace_existing,
            &args.import_message,
        )?;
        if args.dry_run {
            let action = determine_dashboard_import_action_with_request(
                &mut request_json,
                &payload,
                args.replace_existing,
            )?;
            if args.progress {
                println!(
                    "Dry-run import {} -> {}",
                    dashboard_file.display(),
                    action
                );
            }
            continue;
        }
        let _result = import_dashboard_request_with_request(&mut request_json, &payload)?;
        if args.progress {
            println!("Imported {}", dashboard_file.display());
        }
    }
    if args.dry_run {
        println!(
            "Dry-run checked {} dashboard(s) from {}",
            dashboard_files.len(),
            args.import_dir.display()
        );
    }
    Ok(dashboard_files.len())
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
        let payload_object = value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
        let dashboard = payload_object
            .get("dashboard")
            .and_then(Value::as_object)
            .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
        let uid = string_field(dashboard, "uid", "");
        let local_compare = build_local_compare_document(&document, args.import_folder_uid.as_deref())?;
        let Some(remote_payload) = fetch_dashboard_if_exists_with_request(&mut request_json, &uid)? else {
            println!(
                "Diff missing in Grafana for uid={} from {}",
                uid,
                dashboard_file.display()
            );
            differences += 1;
            continue;
        };
        let remote_compare = build_remote_compare_document(&remote_payload, args.import_folder_uid.as_deref())?;
        if serialize_compare_document(&local_compare)? != serialize_compare_document(&remote_compare)? {
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

pub fn run_dashboard_cli_with_client(client: &JsonHttpClient, args: DashboardCliArgs) -> Result<()> {
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
    }
}

#[cfg(test)]
#[path = "dashboard_rust_tests.rs"]
mod dashboard_rust_tests;
