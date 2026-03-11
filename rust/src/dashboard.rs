use clap::{Args, Parser, Subcommand};
use reqwest::Method;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    message, object_field, resolve_auth_headers, sanitize_path_component, string_field, value_as_object,
    Result,
};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

pub const DEFAULT_URL: &str = "http://127.0.0.1:3000";
pub const DEFAULT_TIMEOUT: u64 = 30;
pub const DEFAULT_PAGE_SIZE: usize = 500;
pub const DEFAULT_EXPORT_DIR: &str = "dashboards";
pub const RAW_EXPORT_SUBDIR: &str = "raw";
pub const PROMPT_EXPORT_SUBDIR: &str = "prompt";
pub const DEFAULT_IMPORT_MESSAGE: &str = "Imported by grafana-utils";
const BUILTIN_DATASOURCE_TYPES: &[&str] = &["__expr__", "grafana"];
const BUILTIN_DATASOURCE_NAMES: &[&str] = &[
    "-- Dashboard --",
    "-- Grafana --",
    "-- Mixed --",
    "grafana",
    "expr",
    "__expr__",
];

#[derive(Debug, Clone, Args)]
pub struct CommonCliArgs {
    #[arg(long, default_value = DEFAULT_URL)]
    pub url: String,
    #[arg(long)]
    pub api_token: Option<String>,
    #[arg(long)]
    pub username: Option<String>,
    #[arg(long)]
    pub password: Option<String>,
    #[arg(long, default_value_t = DEFAULT_TIMEOUT)]
    pub timeout: u64,
    #[arg(long, default_value_t = false)]
    pub verify_ssl: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, default_value = DEFAULT_EXPORT_DIR)]
    pub export_dir: PathBuf,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE)]
    pub page_size: usize,
    #[arg(long, default_value_t = false)]
    pub flat: bool,
    #[arg(long, default_value_t = false)]
    pub overwrite: bool,
    #[arg(long, default_value_t = false)]
    pub without_dashboard_raw: bool,
    #[arg(long, default_value_t = false)]
    pub without_dashboard_prompt: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long)]
    pub import_dir: PathBuf,
    #[arg(long)]
    pub import_folder_uid: Option<String>,
    #[arg(long, default_value_t = false)]
    pub replace_existing: bool,
    #[arg(long, default_value = DEFAULT_IMPORT_MESSAGE)]
    pub import_message: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardCommand {
    Export(ExportArgs),
    Import(ImportArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(about = "Export or import Grafana dashboards.")]
pub struct DashboardCliArgs {
    #[command(subcommand)]
    pub command: DashboardCommand,
}

#[derive(Debug, Clone)]
pub struct DashboardAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub headers: Vec<(String, String)>,
}

pub fn parse_cli_from<I, T>(iter: I) -> DashboardCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    DashboardCliArgs::parse_from(iter)
}

pub fn build_auth_context(common: &CommonCliArgs) -> Result<DashboardAuthContext> {
    Ok(DashboardAuthContext {
        url: common.url.clone(),
        timeout: common.timeout,
        verify_ssl: common.verify_ssl,
        headers: resolve_auth_headers(
            common.api_token.as_deref(),
            common.username.as_deref(),
            common.password.as_deref(),
        )?,
    })
}

pub fn build_http_client(common: &CommonCliArgs) -> Result<JsonHttpClient> {
    let context = build_auth_context(common)?;
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}

pub fn build_output_path(output_dir: &Path, summary: &Map<String, Value>, flat: bool) -> PathBuf {
    let folder_title = string_field(summary, "folderTitle", "General");
    let title = string_field(summary, "title", "dashboard");
    let uid = string_field(summary, "uid", "unknown");
    let file_name = format!(
        "{}__{}.json",
        sanitize_path_component(&title),
        sanitize_path_component(&uid)
    );

    if flat {
        output_dir.join(file_name)
    } else {
        output_dir
            .join(sanitize_path_component(&folder_title))
            .join(file_name)
    }
}

pub fn build_export_variant_dirs(output_dir: &Path) -> (PathBuf, PathBuf) {
    (
        output_dir.join(RAW_EXPORT_SUBDIR),
        output_dir.join(PROMPT_EXPORT_SUBDIR),
    )
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
    files.retain(|path| path.file_name().and_then(|name| name.to_str()) != Some("index.json"));
    files.sort();

    if files.is_empty() {
        return Err(message(format!(
            "No dashboard JSON files found in {}",
            import_dir.display()
        )));
    }

    Ok(files)
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

fn known_datasource_type(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "prom" | "prometheus" => Some("prometheus"),
        "loki" => Some("loki"),
        "elastic" | "elasticsearch" => Some("elasticsearch"),
        "opensearch" => Some("grafana-opensearch-datasource"),
        "mysql" => Some("mysql"),
        "postgres" | "postgresql" => Some("postgres"),
        "mssql" => Some("mssql"),
        "influxdb" => Some("influxdb"),
        "tempo" => Some("tempo"),
        "jaeger" => Some("jaeger"),
        "zipkin" => Some("zipkin"),
        "cloudwatch" => Some("cloudwatch"),
        _ => None,
    }
}

fn datasource_type_alias(value: &str) -> &str {
    known_datasource_type(value).unwrap_or(value)
}

#[derive(Clone, Debug)]
struct ResolvedDatasource {
    key: String,
    ds_type: String,
}

#[derive(Clone, Debug)]
struct InputMapping {
    input_name: String,
    label: String,
    ds_type: String,
}

fn build_datasource_catalog(
    datasources: &[Map<String, Value>],
) -> (BTreeMap<String, Map<String, Value>>, BTreeMap<String, Map<String, Value>>) {
    let mut by_uid = BTreeMap::new();
    let mut by_name = BTreeMap::new();
    for datasource in datasources {
        let uid = string_field(datasource, "uid", "");
        if !uid.is_empty() {
            by_uid.insert(uid, datasource.clone());
        }
        let name = string_field(datasource, "name", "");
        if !name.is_empty() {
            by_name.insert(name, datasource.clone());
        }
    }
    (by_uid, by_name)
}

fn is_placeholder_string(value: &str) -> bool {
    value.starts_with('$')
}

fn extract_placeholder_name(value: &str) -> String {
    if value.starts_with("${") && value.ends_with('}') && value.len() > 3 {
        return value[2..value.len() - 1].to_string();
    }
    if value.starts_with('$') && value.len() > 1 {
        return value[1..].to_string();
    }
    value.to_string()
}

fn is_generated_input_placeholder(value: &str) -> bool {
    extract_placeholder_name(value).starts_with("DS_")
}

fn is_builtin_datasource_ref(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            BUILTIN_DATASOURCE_NAMES.contains(&text.as_str()) || is_generated_input_placeholder(text)
        }
        Value::Object(object) => {
            let uid = object.get("uid").and_then(Value::as_str).unwrap_or_default();
            let name = object.get("name").and_then(Value::as_str).unwrap_or_default();
            let ds_type = object.get("type").and_then(Value::as_str).unwrap_or_default();
            is_generated_input_placeholder(uid)
                || is_generated_input_placeholder(name)
                || BUILTIN_DATASOURCE_TYPES.contains(&uid)
                || BUILTIN_DATASOURCE_TYPES.contains(&ds_type)
        }
        _ => false,
    }
}

fn collect_datasource_refs(node: &Value, refs: &mut Vec<Value>) {
    match node {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "datasource" {
                    refs.push(value.clone());
                }
                collect_datasource_refs(value, refs);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_datasource_refs(item, refs);
            }
        }
        _ => {}
    }
}

fn make_input_name(label: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_underscore = false;
    for character in label.chars().flat_map(|character| character.to_uppercase()) {
        if character.is_ascii_alphanumeric() {
            normalized.push(character);
            last_was_underscore = false;
        } else if !last_was_underscore {
            normalized.push('_');
            last_was_underscore = true;
        }
    }
    let normalized = normalized.trim_matches('_').to_string();
    format!("DS_{}", if normalized.is_empty() { "DATASOURCE" } else { &normalized })
}

fn make_type_input_base(datasource_type: &str) -> String {
    make_input_name(datasource_type_alias(datasource_type))
}

fn make_input_label(datasource_type: &str, index: usize) -> String {
    let title = datasource_type_alias(datasource_type)
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ");
    if index == 1 {
        format!("{title} datasource")
    } else {
        format!("{title} datasource {index}")
    }
}

fn build_resolved_datasource(key: String, ds_type: String) -> ResolvedDatasource {
    ResolvedDatasource { key, ds_type }
}

fn lookup_datasource(
    datasources_by_uid: &BTreeMap<String, Map<String, Value>>,
    datasources_by_name: &BTreeMap<String, Map<String, Value>>,
    uid: Option<&str>,
    name: Option<&str>,
) -> Option<Map<String, Value>> {
    if let Some(uid) = uid.filter(|value| !value.is_empty()) {
        if let Some(datasource) = datasources_by_uid.get(uid) {
            return Some(datasource.clone());
        }
    }
    if let Some(name) = name.filter(|value| !value.is_empty()) {
        return datasources_by_name.get(name).cloned();
    }
    None
}

fn resolve_datasource_type_alias(
    reference: &str,
    datasources_by_uid: &BTreeMap<String, Map<String, Value>>,
) -> Option<String> {
    if let Some(alias) = known_datasource_type(reference) {
        return Some(alias.to_string());
    }
    let lower = reference.to_ascii_lowercase();
    for candidate in datasources_by_uid.values() {
        let candidate_type = string_field(candidate, "type", "");
        if !candidate_type.is_empty() && candidate_type.eq_ignore_ascii_case(&lower) {
            return Some(candidate_type);
        }
    }
    None
}

fn resolve_string_datasource_ref(
    reference: &str,
    datasources_by_uid: &BTreeMap<String, Map<String, Value>>,
    datasources_by_name: &BTreeMap<String, Map<String, Value>>,
) -> Result<ResolvedDatasource> {
    if let Some(datasource) =
        lookup_datasource(datasources_by_uid, datasources_by_name, Some(reference), Some(reference))
    {
        let uid = string_field(&datasource, "uid", reference);
        let ds_type = string_field(&datasource, "type", "");
        if ds_type.is_empty() {
            return Err(message(format!(
                "Datasource {reference:?} does not have a usable type."
            )));
        }
        return Ok(build_resolved_datasource(format!("uid:{uid}"), ds_type));
    }

    if let Some(datasource_type) = resolve_datasource_type_alias(reference, datasources_by_uid) {
        return Ok(build_resolved_datasource(
            format!("type:{datasource_type}"),
            datasource_type,
        ));
    }

    Err(message(format!(
        "Cannot resolve datasource name or uid {reference:?} for prompt export."
    )))
}

fn resolve_placeholder_object_ref(
    uid: Option<&str>,
    name: Option<&str>,
    ds_type: Option<&str>,
) -> Option<ResolvedDatasource> {
    let ds_type = ds_type.filter(|value| !value.is_empty())?;
    let placeholder_value = if uid.is_some_and(is_placeholder_string) {
        uid
    } else if name.is_some_and(is_placeholder_string) {
        name
    } else {
        None
    }?;
    let token = extract_placeholder_name(placeholder_value);
    Some(build_resolved_datasource(
        format!("var:{ds_type}:{token}"),
        ds_type.to_string(),
    ))
}

fn resolve_object_datasource_ref(
    reference: &Map<String, Value>,
    datasources_by_uid: &BTreeMap<String, Map<String, Value>>,
    datasources_by_name: &BTreeMap<String, Map<String, Value>>,
) -> Result<Option<ResolvedDatasource>> {
    let uid = reference.get("uid").and_then(Value::as_str);
    let name = reference.get("name").and_then(Value::as_str);
    let ds_type = reference.get("type").and_then(Value::as_str);
    let has_placeholder = uid.is_some_and(is_placeholder_string) || name.is_some_and(is_placeholder_string);

    if let Some(resolved) = resolve_placeholder_object_ref(uid, name, ds_type) {
        return Ok(Some(resolved));
    }
    if has_placeholder {
        return Ok(None);
    }

    let datasource = lookup_datasource(datasources_by_uid, datasources_by_name, uid, name);
    let mut resolved_type = ds_type.unwrap_or_default().to_string();
    let mut resolved_uid = uid.unwrap_or(name.unwrap_or_default()).to_string();
    if let Some(datasource) = datasource {
        if resolved_type.is_empty() {
            resolved_type = string_field(&datasource, "type", "");
        }
        if resolved_uid.is_empty() {
            resolved_uid = string_field(&datasource, "uid", "");
        }
    }

    if resolved_type.is_empty() {
        return Err(message(format!(
            "Cannot resolve datasource type from reference {:?}.",
            Value::Object(reference.clone())
        )));
    }
    if resolved_uid.is_empty() {
        resolved_uid = resolved_type.clone();
    }

    Ok(Some(build_resolved_datasource(
        format!("uid:{resolved_uid}"),
        resolved_type,
    )))
}

fn resolve_datasource_ref(
    reference: &Value,
    datasources_by_uid: &BTreeMap<String, Map<String, Value>>,
    datasources_by_name: &BTreeMap<String, Map<String, Value>>,
) -> Result<Option<ResolvedDatasource>> {
    if reference.is_null() || is_builtin_datasource_ref(reference) {
        return Ok(None);
    }
    match reference {
        Value::String(text) => {
            if is_placeholder_string(text) {
                Ok(None)
            } else {
                resolve_string_datasource_ref(text, datasources_by_uid, datasources_by_name).map(Some)
            }
        }
        Value::Object(object) => resolve_object_datasource_ref(object, datasources_by_uid, datasources_by_name),
        _ => Ok(None),
    }
}

fn allocate_input_mapping(
    resolved: &ResolvedDatasource,
    ref_mapping: &mut BTreeMap<String, InputMapping>,
    type_counts: &mut BTreeMap<String, usize>,
    key_override: Option<String>,
) -> InputMapping {
    let mapping_key = key_override.unwrap_or_else(|| resolved.key.clone());
    if let Some(mapping) = ref_mapping.get(&mapping_key) {
        return mapping.clone();
    }
    let count = type_counts.get(&resolved.ds_type).copied().unwrap_or(0) + 1;
    type_counts.insert(resolved.ds_type.clone(), count);
    let mapping = InputMapping {
        input_name: format!("{}_{}", make_type_input_base(&resolved.ds_type), count),
        label: make_input_label(&resolved.ds_type, count),
        ds_type: resolved.ds_type.clone(),
    };
    ref_mapping.insert(mapping_key, mapping.clone());
    mapping
}

fn rewrite_template_variable_query(
    variable: &mut Map<String, Value>,
    mapping: &InputMapping,
    datasource_var_mappings: &mut BTreeMap<String, InputMapping>,
    datasource_var_placeholders: &mut BTreeSet<String>,
) {
    if let Some(name) = variable.get("name").and_then(Value::as_str).filter(|value| !value.is_empty()) {
        datasource_var_mappings.insert(name.to_string(), mapping.clone());
        datasource_var_placeholders.insert(format!("${name}"));
        datasource_var_placeholders.insert(format!("${{{name}}}"));
    }
    variable.insert("current".to_string(), Value::Object(Map::new()));
    variable.insert("options".to_string(), Value::Array(Vec::new()));
    variable.insert("query".to_string(), Value::String(mapping.ds_type.clone()));
    variable.insert("refresh".to_string(), Value::from(1));
    if !variable.contains_key("regex") {
        variable.insert("regex".to_string(), Value::String(String::new()));
    }
    if variable.get("hide").and_then(Value::as_i64) == Some(0) {
        variable.remove("hide");
    }
}

fn rewrite_template_variable_datasource(
    variable: &mut Map<String, Value>,
    datasource_var_mappings: &BTreeMap<String, InputMapping>,
    datasource_var_placeholders: &BTreeSet<String>,
) {
    let placeholder_value = match variable.get("datasource") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Object(object)) => object.get("uid").and_then(Value::as_str).map(|value| value.to_string()),
        _ => None,
    };
    let Some(placeholder_value) = placeholder_value else {
        return;
    };
    let mapping = datasource_var_mappings.get(&extract_placeholder_name(&placeholder_value));
    if !datasource_var_placeholders.contains(&placeholder_value) || mapping.is_none() {
        return;
    }
    let mapping = mapping.unwrap();
    variable.insert(
        "datasource".to_string(),
        Value::Object(Map::from_iter([
            ("type".to_string(), Value::String(mapping.ds_type.clone())),
            (
                "uid".to_string(),
                Value::String(format!("${{{}}}", mapping.input_name)),
            ),
        ])),
    );
    variable.insert("current".to_string(), Value::Object(Map::new()));
    variable.insert("options".to_string(), Value::Array(Vec::new()));
}

fn prepare_templating_for_external_import(
    dashboard: &mut Map<String, Value>,
    ref_mapping: &mut BTreeMap<String, InputMapping>,
    type_counts: &mut BTreeMap<String, usize>,
    datasources_by_uid: &BTreeMap<String, Map<String, Value>>,
    datasources_by_name: &BTreeMap<String, Map<String, Value>>,
) {
    let Some(templating) = dashboard.get_mut("templating").and_then(Value::as_object_mut) else {
        return;
    };
    let Some(variables) = templating.get_mut("list").and_then(Value::as_array_mut) else {
        return;
    };

    let mut datasource_var_mappings = BTreeMap::new();
    let mut datasource_var_placeholders = BTreeSet::new();

    for variable in variables.iter_mut() {
        let Some(variable_object) = variable.as_object_mut() else {
            continue;
        };
        if variable_object.get("type").and_then(Value::as_str) != Some("datasource") {
            continue;
        }
        let Some(query) = variable_object.get("query").and_then(Value::as_str).filter(|value| !value.is_empty()) else {
            continue;
        };
        let Some(resolved) = resolve_datasource_ref(
            &Value::String(query.to_string()),
            datasources_by_uid,
            datasources_by_name,
        ).ok().flatten() else {
            continue;
        };
        let variable_name = variable_object.get("name").and_then(Value::as_str).unwrap_or(&resolved.key);
        let mapping = allocate_input_mapping(
            &resolved,
            ref_mapping,
            type_counts,
            Some(format!("templating:{variable_name}")),
        );
        rewrite_template_variable_query(
            variable_object,
            &mapping,
            &mut datasource_var_mappings,
            &mut datasource_var_placeholders,
        );
    }

    for variable in variables.iter_mut() {
        if let Some(variable_object) = variable.as_object_mut() {
            rewrite_template_variable_datasource(
                variable_object,
                &datasource_var_mappings,
                &datasource_var_placeholders,
            );
        }
    }
}

fn replace_datasource_refs_in_dashboard(
    node: &mut Value,
    ref_mapping: &BTreeMap<String, InputMapping>,
    datasources_by_uid: &BTreeMap<String, Map<String, Value>>,
    datasources_by_name: &BTreeMap<String, Map<String, Value>>,
) -> Result<()> {
    match node {
        Value::Object(object) => {
            if let Some(datasource_value) = object.get_mut("datasource") {
                if let Some(resolved) =
                    resolve_datasource_ref(datasource_value, datasources_by_uid, datasources_by_name)?
                {
                    let mapping = ref_mapping.get(&resolved.key).ok_or_else(|| {
                        message(format!("Missing datasource input mapping for {}", resolved.key))
                    })?;
                    let placeholder = format!("${{{}}}", mapping.input_name);
                    let replacement = if datasource_value.is_object() {
                        let mut replacement = Map::new();
                        replacement.insert("uid".to_string(), Value::String(placeholder));
                        if !resolved.ds_type.is_empty() {
                            replacement.insert("type".to_string(), Value::String(resolved.ds_type));
                        }
                        Value::Object(replacement)
                    } else {
                        Value::String(placeholder)
                    };
                    *datasource_value = replacement;
                }
            }
            let keys = object.keys().cloned().collect::<Vec<String>>();
            for key in keys {
                if key == "datasource" {
                    continue;
                }
                if let Some(value) = object.get_mut(&key) {
                    replace_datasource_refs_in_dashboard(
                        value,
                        ref_mapping,
                        datasources_by_uid,
                        datasources_by_name,
                    )?;
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                replace_datasource_refs_in_dashboard(
                    item,
                    ref_mapping,
                    datasources_by_uid,
                    datasources_by_name,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn ensure_datasource_template_variable(dashboard: &mut Map<String, Value>, datasource_type: &str) {
    let templating = dashboard
        .entry("templating".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(templating_object) = templating.as_object_mut() else {
        return;
    };
    let variables = templating_object
        .entry("list".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(variables_array) = variables.as_array_mut() else {
        return;
    };

    if variables_array.iter().any(|variable| {
        variable
            .as_object()
            .and_then(|object| object.get("type"))
            .and_then(Value::as_str)
            == Some("datasource")
    }) {
        return;
    }

    variables_array.insert(
        0,
        json!({
            "current": {},
            "label": "Data source",
            "name": "datasource",
            "options": [],
            "query": datasource_type,
            "refresh": 1,
            "regex": "",
            "type": "datasource",
        }),
    );
}

fn rewrite_panel_datasources_to_template_variable(
    panels: &mut [Value],
    placeholder_names: &BTreeSet<String>,
) {
    for panel in panels {
        let Some(panel_object) = panel.as_object_mut() else {
            continue;
        };
        if let Some(datasource) = panel_object.get_mut("datasource") {
            match datasource {
                Value::String(text)
                    if placeholder_names.contains(text)
                        || text == "$datasource"
                        || text == "${datasource}" =>
                {
                    *datasource = json!({"uid": "$datasource"});
                }
                Value::Object(object) => {
                    let uid = object.get("uid").and_then(Value::as_str).unwrap_or_default();
                    if placeholder_names.contains(uid)
                        || uid == "$datasource"
                        || uid == "${datasource}"
                    {
                        *datasource = json!({"uid": "$datasource"});
                    }
                }
                _ => {}
            }
        }

        if let Some(nested) = panel_object.get_mut("panels").and_then(Value::as_array_mut) {
            rewrite_panel_datasources_to_template_variable(nested, placeholder_names);
        }
    }
}

fn collect_panel_types(panels: &[Value], panel_types: &mut BTreeSet<String>) {
    for panel in panels {
        let Some(panel_object) = panel.as_object() else {
            continue;
        };
        let panel_type = string_field(panel_object, "type", "");
        if !panel_type.is_empty() {
            panel_types.insert(panel_type);
        }
        if let Some(nested) = panel_object.get("panels").and_then(Value::as_array) {
            collect_panel_types(nested, panel_types);
        }
    }
}

fn build_input_definitions(ref_mapping: &BTreeMap<String, InputMapping>) -> Value {
    let mut mappings = ref_mapping.values().cloned().collect::<Vec<InputMapping>>();
    mappings.sort_by(|left, right| left.input_name.cmp(&right.input_name));
    Value::Array(
        mappings
            .into_iter()
            .map(|mapping| {
                json!({
                    "name": mapping.input_name,
                    "label": mapping.label,
                    "description": "",
                    "type": "datasource",
                    "pluginId": mapping.ds_type,
                    "pluginName": mapping.ds_type,
                })
            })
            .collect(),
    )
}

fn build_requires_block(
    ref_mapping: &BTreeMap<String, InputMapping>,
    panel_types: &BTreeSet<String>,
) -> Value {
    let mut requires = vec![json!({
        "type": "grafana",
        "id": "grafana",
        "name": "Grafana",
        "version": "",
    })];
    let mut mappings = ref_mapping.values().cloned().collect::<Vec<InputMapping>>();
    mappings.sort_by(|left, right| left.input_name.cmp(&right.input_name));
    for mapping in mappings {
        requires.push(json!({
            "type": "datasource",
            "id": mapping.ds_type,
            "name": mapping.ds_type,
            "version": "",
        }));
    }
    for panel_type in panel_types {
        requires.push(json!({
            "type": "panel",
            "id": panel_type,
            "name": panel_type,
            "version": "",
        }));
    }
    Value::Array(requires)
}

pub fn build_external_export_document(
    payload: &Value,
    datasource_catalog: &(BTreeMap<String, Map<String, Value>>, BTreeMap<String, Map<String, Value>>),
) -> Result<Value> {
    let mut dashboard = build_preserved_web_import_document(payload)?;
    let dashboard_object = dashboard
        .as_object_mut()
        .ok_or_else(|| message("Unexpected dashboard payload from Grafana."))?;

    let (datasources_by_uid, datasources_by_name) = datasource_catalog;
    let mut refs = Vec::new();
    collect_datasource_refs(&Value::Object(dashboard_object.clone()), &mut refs);

    let mut ref_mapping = BTreeMap::new();
    let mut type_counts = BTreeMap::new();
    prepare_templating_for_external_import(
        dashboard_object,
        &mut ref_mapping,
        &mut type_counts,
        datasources_by_uid,
        datasources_by_name,
    );
    for reference in refs {
        let Some(resolved) =
            resolve_datasource_ref(&reference, datasources_by_uid, datasources_by_name)?
        else {
            continue;
        };
        if ref_mapping.contains_key(&resolved.key) {
            continue;
        }
        allocate_input_mapping(&resolved, &mut ref_mapping, &mut type_counts, None);
    }

    replace_datasource_refs_in_dashboard(
        &mut dashboard,
        &ref_mapping,
        datasources_by_uid,
        datasources_by_name,
    )?;

    let datasource_types = ref_mapping
        .values()
        .map(|mapping| mapping.ds_type.clone())
        .collect::<BTreeSet<String>>();
    if datasource_types.len() == 1 {
        let datasource_type = datasource_types.iter().next().cloned().unwrap_or_default();
        let dashboard_object = dashboard
            .as_object_mut()
            .ok_or_else(|| message("Unexpected dashboard payload from Grafana."))?;
        ensure_datasource_template_variable(dashboard_object, &datasource_type);
        let placeholder_names = ref_mapping
            .values()
            .map(|mapping| format!("${{{}}}", mapping.input_name))
            .collect::<BTreeSet<String>>();
        if let Some(panels) = dashboard_object.get_mut("panels").and_then(Value::as_array_mut) {
            rewrite_panel_datasources_to_template_variable(panels, &placeholder_names);
        }
    }

    let mut panel_types = BTreeSet::new();
    if let Some(panels) = dashboard.get("panels").and_then(Value::as_array) {
        collect_panel_types(panels, &mut panel_types);
    }
    let dashboard_object = dashboard
        .as_object_mut()
        .ok_or_else(|| message("Unexpected dashboard payload from Grafana."))?;
    dashboard_object.insert("__inputs".to_string(), build_input_definitions(&ref_mapping));
    dashboard_object.insert("__requires".to_string(), build_requires_block(&ref_mapping, &panel_types));
    dashboard_object.insert("__elements".to_string(), Value::Object(Map::new()));
    Ok(dashboard)
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

fn write_json_document(payload: &Value, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, serde_json::to_string_pretty(payload)? + "\n")?;
    Ok(())
}

fn build_dashboard_index_item(summary: &Map<String, Value>, uid: &str) -> Map<String, Value> {
    let mut item = Map::new();
    item.insert("uid".to_string(), Value::String(uid.to_string()));
    item.insert(
        "title".to_string(),
        Value::String(string_field(summary, "title", "dashboard")),
    );
    item.insert(
        "folderTitle".to_string(),
        Value::String(string_field(summary, "folderTitle", "General")),
    );
    item
}

fn build_variant_index(
    items: &[Map<String, Value>],
    path_key: &str,
    export_format: &str,
) -> Value {
    Value::Array(
        items
            .iter()
            .filter_map(|item| {
                item.get(path_key).map(|path| {
                    Value::Object(Map::from_iter([
                        (
                            "uid".to_string(),
                            Value::String(string_field(item, "uid", "unknown")),
                        ),
                        (
                            "title".to_string(),
                            Value::String(string_field(item, "title", "dashboard")),
                        ),
                        ("path".to_string(), path.clone()),
                        (
                            "format".to_string(),
                            Value::String(export_format.to_string()),
                        ),
                    ]))
                })
            })
            .collect(),
    )
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

fn export_dashboards_with_request<F>(mut request_json: F, args: &ExportArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.without_dashboard_raw && args.without_dashboard_prompt {
        return Err(message(
            "Nothing to export. Remove one of --without-dashboard-raw or --without-dashboard-prompt.",
        ));
    }
    fs::create_dir_all(&args.export_dir)?;
    let (raw_dir, prompt_dir) = build_export_variant_dirs(&args.export_dir);
    if !args.without_dashboard_raw {
        fs::create_dir_all(&raw_dir)?;
    }
    if !args.without_dashboard_prompt {
        fs::create_dir_all(&prompt_dir)?;
    }
    let datasource_catalog = if args.without_dashboard_prompt {
        None
    } else {
        Some(build_datasource_catalog(&list_datasources_with_request(&mut request_json)?))
    };

    let summaries = list_dashboard_summaries_with_request(&mut request_json, args.page_size)?;
    if summaries.is_empty() {
        return Ok(0);
    }

    let mut exported_count = 0;
    let mut index_items = Vec::new();
    for summary in summaries {
        let uid = string_field(&summary, "uid", "");
        if uid.is_empty() {
            continue;
        }
        let payload = fetch_dashboard_with_request(&mut request_json, &uid)?;
        let mut item = build_dashboard_index_item(&summary, &uid);
        if !args.without_dashboard_raw {
            let raw_document = build_preserved_web_import_document(&payload)?;
            let raw_path = build_output_path(&raw_dir, &summary, args.flat);
            write_dashboard(&raw_document, &raw_path, args.overwrite)?;
            item.insert(
                "raw_path".to_string(),
                Value::String(raw_path.display().to_string()),
            );
        }
        if !args.without_dashboard_prompt {
            let prompt_document = build_external_export_document(
                &payload,
                datasource_catalog
                    .as_ref()
                    .ok_or_else(|| message("Prompt export requires datasource catalog."))?,
            )?;
            let prompt_path = build_output_path(&prompt_dir, &summary, args.flat);
            write_dashboard(&prompt_document, &prompt_path, args.overwrite)?;
            item.insert(
                "prompt_path".to_string(),
                Value::String(prompt_path.display().to_string()),
            );
        }
        exported_count += 1;
        index_items.push(item);
    }

    if !args.without_dashboard_raw {
        write_json_document(
            &build_variant_index(
                &index_items,
                "raw_path",
                "grafana-web-import-preserve-uid",
            ),
            &raw_dir.join("index.json"),
        )?;
    }
    if !args.without_dashboard_prompt {
        write_json_document(
            &build_variant_index(
                &index_items,
                "prompt_path",
                "grafana-web-import-with-datasource-inputs",
            ),
            &prompt_dir.join("index.json"),
        )?;
    }
    write_json_document(
        &Value::Array(index_items.into_iter().map(Value::Object).collect()),
        &args.export_dir.join("index.json"),
    )?;
    Ok(exported_count)
}

pub fn export_dashboards_with_client(client: &JsonHttpClient, args: &ExportArgs) -> Result<usize> {
    export_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

fn import_dashboards_with_request<F>(mut request_json: F, args: &ImportArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let dashboard_files = discover_dashboard_files(&args.import_dir)?;
    for dashboard_file in &dashboard_files {
        let document = load_json_file(dashboard_file)?;
        let payload = build_import_payload(
            &document,
            args.import_folder_uid.as_deref(),
            args.replace_existing,
            &args.import_message,
        )?;
        let _result = import_dashboard_request_with_request(&mut request_json, &payload)?;
    }
    Ok(dashboard_files.len())
}

pub fn import_dashboards_with_client(client: &JsonHttpClient, args: &ImportArgs) -> Result<usize> {
    import_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

pub fn run_dashboard_cli_with_client(client: &JsonHttpClient, args: DashboardCliArgs) -> Result<()> {
    match args.command {
        DashboardCommand::Export(export_args) => {
            let _ = export_dashboards_with_client(client, &export_args)?;
            Ok(())
        }
        DashboardCommand::Import(import_args) => {
            let _ = import_dashboards_with_client(client, &import_args)?;
            Ok(())
        }
    }
}

pub fn run_dashboard_cli(args: DashboardCliArgs) -> Result<()> {
    match args.command {
        DashboardCommand::Export(export_args) => {
            let client = build_http_client(&export_args.common)?;
            if export_args.without_dashboard_raw && export_args.without_dashboard_prompt {
                return Err(message(
                    "At least one export variant must stay enabled. Remove --without-dashboard-raw or --without-dashboard-prompt.",
                ));
            }
            let _ = export_dashboards_with_client(&client, &export_args)?;
            Ok(())
        }
        DashboardCommand::Import(import_args) => {
            let client = build_http_client(&import_args.common)?;
            let _ = import_dashboards_with_client(&client, &import_args)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_export_variant_dirs, build_external_export_document, build_import_payload, build_output_path,
        build_preserved_web_import_document, discover_dashboard_files, export_dashboards_with_request,
        import_dashboards_with_request, CommonCliArgs, ExportArgs, ImportArgs,
    };
    use serde_json::{json, Value};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn make_common_args(base_url: String) -> CommonCliArgs {
        CommonCliArgs {
            url: base_url,
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            timeout: 30,
            verify_ssl: false,
        }
    }

    #[test]
    fn build_output_path_keeps_folder_structure() {
        let summary = json!({
            "folderTitle": "Infra Team",
            "title": "Cluster Health",
            "uid": "abc",
        });
        let path = build_output_path(Path::new("out"), summary.as_object().unwrap(), false);
        assert_eq!(path, Path::new("out/Infra_Team/Cluster_Health__abc.json"));
    }

    #[test]
    fn build_export_variant_dirs_returns_raw_and_prompt_dirs() {
        let (raw_dir, prompt_dir) = build_export_variant_dirs(Path::new("dashboards"));
        assert_eq!(raw_dir, Path::new("dashboards/raw"));
        assert_eq!(prompt_dir, Path::new("dashboards/prompt"));
    }

    #[test]
    fn discover_dashboard_files_rejects_combined_export_root() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("raw")).unwrap();
        fs::create_dir_all(temp.path().join("prompt")).unwrap();
        let error = discover_dashboard_files(temp.path()).unwrap_err();
        assert!(error.to_string().contains("combined export root"));
    }

    #[test]
    fn build_import_payload_accepts_wrapped_document() {
        let payload = build_import_payload(
            &json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                "meta": {"folderUid": "old-folder"}
            }),
            Some("new-folder"),
            true,
            "sync dashboards",
        )
        .unwrap();

        assert_eq!(payload["dashboard"]["id"], Value::Null);
        assert_eq!(payload["folderUid"], "new-folder");
        assert_eq!(payload["overwrite"], true);
        assert_eq!(payload["message"], "sync dashboards");
    }

    #[test]
    fn build_preserved_web_import_document_clears_numeric_id() {
        let document = build_preserved_web_import_document(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"}
        }))
        .unwrap();

        assert_eq!(document["id"], Value::Null);
        assert_eq!(document["uid"], "abc");
    }

    #[test]
    fn export_dashboards_with_client_writes_raw_variant_and_indexes() {
        let temp = tempdir().unwrap();
        let args = ExportArgs {
            common: make_common_args("http://127.0.0.1:3000".to_string()),
            export_dir: temp.path().join("dashboards"),
            page_size: 500,
            flat: false,
            overwrite: true,
            without_dashboard_raw: false,
            without_dashboard_prompt: true,
        };
        let mut calls = Vec::new();
        let count = export_dashboards_with_request(
            |method, path, params, payload| {
                calls.push((method.to_string(), path.to_string(), params.to_vec(), payload.cloned()));
                if path == "/api/search" {
                    return Ok(Some(json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }])));
                }
                if path == "/api/dashboards/uid/abc" {
                    return Ok(Some(json!({"dashboard": {"id": 7, "uid": "abc", "title": "CPU"}})));
                }
                Err(super::message(format!("unexpected path {path}")))
            },
            &args,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert!(args.export_dir.join("raw/Infra/CPU__abc.json").is_file());
        assert!(args.export_dir.join("raw/index.json").is_file());
        assert!(args.export_dir.join("index.json").is_file());
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn build_external_export_document_adds_datasource_inputs() {
        let payload = json!({
            "dashboard": {
                "id": 9,
                "title": "Infra",
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": {"type": "prometheus", "uid": "prom_uid"},
                        "targets": [
                            {
                                "datasource": {"type": "prometheus", "uid": "prom_uid"},
                                "expr": "up"
                            }
                        ]
                    },
                    {
                        "type": "stat",
                        "datasource": "Loki Logs"
                    }
                ]
            }
        });
        let catalog = super::build_datasource_catalog(&vec![
            json!({"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"})
                .as_object()
                .unwrap()
                .clone(),
            json!({"uid": "loki_uid", "name": "Loki Logs", "type": "loki"})
                .as_object()
                .unwrap()
                .clone(),
        ]);

        let document = build_external_export_document(&payload, &catalog).unwrap();

        assert_eq!(document["panels"][0]["datasource"]["uid"], "${DS_PROMETHEUS_1}");
        assert_eq!(document["panels"][0]["targets"][0]["datasource"]["uid"], "${DS_PROMETHEUS_1}");
        assert_eq!(document["panels"][1]["datasource"], "${DS_LOKI_1}");
        assert_eq!(document["__inputs"][0]["name"], "DS_LOKI_1");
        assert_eq!(document["__inputs"][1]["name"], "DS_PROMETHEUS_1");
        assert_eq!(document["__elements"], json!({}));
    }

    #[test]
    fn build_external_export_document_creates_input_from_datasource_template_variable() {
        let payload = json!({
            "dashboard": {
                "id": 15,
                "title": "Prometheus / Overview",
                "templating": {
                    "list": [
                        {
                            "current": {"text": "default", "value": "default"},
                            "hide": 0,
                            "label": "Data source",
                            "name": "datasource",
                            "options": [],
                            "query": "prometheus",
                            "refresh": 1,
                            "regex": "",
                            "type": "datasource"
                        },
                        {
                            "allValue": ".+",
                            "current": {"selected": true, "text": "All", "value": "$__all"},
                            "datasource": "$datasource",
                            "includeAll": true,
                            "label": "job",
                            "multi": true,
                            "name": "job",
                            "options": [],
                            "query": "label_values(prometheus_build_info, job)",
                            "refresh": 1,
                            "regex": "",
                            "sort": 2,
                            "type": "query"
                        }
                    ]
                },
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": "$datasource",
                        "targets": [{"refId": "A", "expr": "up"}]
                    }
                ]
            }
        });

        let document = build_external_export_document(&payload, &(BTreeMap::new(), BTreeMap::new())).unwrap();
        assert_eq!(document["__inputs"][0]["name"], "DS_PROMETHEUS_1");
        assert_eq!(document["templating"]["list"][0]["current"], json!({}));
        assert_eq!(document["templating"]["list"][0]["query"], "prometheus");
        assert_eq!(document["templating"]["list"][1]["datasource"]["uid"], "${DS_PROMETHEUS_1}");
        assert_eq!(document["panels"][0]["datasource"]["uid"], "$datasource");
    }

    #[test]
    fn export_dashboards_with_client_writes_prompt_variant_and_indexes() {
        let temp = tempdir().unwrap();
        let args = ExportArgs {
            common: make_common_args("http://127.0.0.1:3000".to_string()),
            export_dir: temp.path().join("dashboards"),
            page_size: 500,
            flat: false,
            overwrite: true,
            without_dashboard_raw: false,
            without_dashboard_prompt: false,
        };

        let count = export_dashboards_with_request(
            |_method, path, _params, _payload| match path {
                "/api/datasources" => Ok(Some(json!([
                    {"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"}
                ]))),
                "/api/search" => Ok(Some(json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }]))),
                "/api/dashboards/uid/abc" => Ok(Some(json!({
                    "dashboard": {
                        "id": 7,
                        "uid": "abc",
                        "title": "CPU",
                        "panels": [
                            {"type": "timeseries", "datasource": {"type": "prometheus", "uid": "prom_uid"}}
                        ]
                    }
                }))),
                _ => Err(super::message(format!("unexpected path {path}"))),
            },
            &args,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert!(args.export_dir.join("prompt/Infra/CPU__abc.json").is_file());
        assert!(args.export_dir.join("prompt/index.json").is_file());
    }

    #[test]
    fn import_dashboards_with_client_imports_discovered_files() {
        let temp = tempdir().unwrap();
        let raw_dir = temp.path().join("raw");
        fs::create_dir_all(&raw_dir).unwrap();
        fs::write(
            raw_dir.join("dash.json"),
            serde_json::to_string_pretty(&json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                "meta": {"folderUid": "old-folder"}
            }))
            .unwrap(),
        )
        .unwrap();
        let args = ImportArgs {
            common: make_common_args("http://127.0.0.1:3000".to_string()),
            import_dir: raw_dir,
            import_folder_uid: Some("new-folder".to_string()),
            replace_existing: true,
            import_message: "sync dashboards".to_string(),
        };
        let mut posted_payloads = Vec::new();
        let count = import_dashboards_with_request(
            |_method, path, _params, payload| {
                assert_eq!(path, "/api/dashboards/db");
                posted_payloads.push(payload.cloned().unwrap());
                Ok(Some(json!({"status": "success"})))
            },
            &args,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(posted_payloads.len(), 1);
        assert_eq!(posted_payloads[0]["folderUid"], "new-folder");
        assert_eq!(posted_payloads[0]["dashboard"]["id"], Value::Null);
    }
}
