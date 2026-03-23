//! Dashboard inspection pipeline for live systems and export directories.
//! Coordinates query extraction, filtering, report assembly, and table/JSON rendering entry points.
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use regex::Regex;
use reqwest::Method;
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{message, object_field, string_field, value_as_object, Result};
use crate::dashboard_inspection_dependency_contract::build_offline_dependency_contract;

use super::inspect_analyzer_flux;
use super::inspect_analyzer_loki;
use super::inspect_analyzer_prometheus;
use super::inspect_analyzer_search;
use super::inspect_analyzer_sql;
use super::inspect_governance::{
    build_export_inspection_governance_document, normalize_family_name,
    render_governance_table_report,
};
use super::inspect_live_tui::run_inspect_live_interactive as run_inspect_live_tui;
use super::inspect_render::{
    render_csv, render_grouped_query_report, render_grouped_query_table_report, render_simple_table,
};
use super::*;

/// Constant for datasource family prometheus.
pub(crate) const DATASOURCE_FAMILY_PROMETHEUS: &str = "prometheus";
/// Constant for datasource family loki.
pub(crate) const DATASOURCE_FAMILY_LOKI: &str = "loki";
/// Constant for datasource family flux.
pub(crate) const DATASOURCE_FAMILY_FLUX: &str = "flux";
/// Constant for datasource family sql.
pub(crate) const DATASOURCE_FAMILY_SQL: &str = "sql";
/// Constant for datasource family search.
pub(crate) const DATASOURCE_FAMILY_SEARCH: &str = "search";
/// Constant for datasource family tracing.
pub(crate) const DATASOURCE_FAMILY_TRACING: &str = "tracing";
/// Constant for datasource family unknown.
pub(crate) const DATASOURCE_FAMILY_UNKNOWN: &str = "unknown";

/// Struct definition for QueryAnalysis.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct QueryAnalysis {
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
}

/// Struct definition for QueryExtractionContext.
pub(crate) struct QueryExtractionContext<'a> {
    pub(crate) panel: &'a Map<String, Value>,
    pub(crate) target: &'a Map<String, Value>,
    pub(crate) query_field: &'a str,
    pub(crate) query_text: &'a str,
    pub(crate) resolved_datasource_type: &'a str,
}

struct QueryReportContext<'a> {
    export_org: &'a str,
    export_org_id: &'a str,
    dashboard: &'a Map<String, Value>,
    dashboard_uid: &'a str,
    dashboard_title: &'a str,
    folder_path: &'a str,
    folder_uid: &'a str,
    parent_folder_uid: &'a str,
    dashboard_file_display: &'a str,
    datasource_inventory: &'a [DatasourceInventoryItem],
}

fn calculate_folder_level(folder_path: &str) -> String {
    let level = folder_path
        .split(" / ")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .count();
    if level == 0 {
        String::new()
    } else {
        level.to_string()
    }
}

fn calculate_folder_full_path(folder_path: &str) -> String {
    let segments = folder_path
        .split(" / ")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<&str>>();
    if segments.is_empty() || (segments.len() == 1 && segments[0] == DEFAULT_FOLDER_TITLE) {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

fn extract_dashboard_tags(dashboard: &Map<String, Value>) -> Vec<String> {
    dashboard
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            let mut values = Vec::new();
            for tag in tags {
                if let Some(value) = tag
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    ordered_unique_push(&mut values, value);
                }
            }
            values
        })
        .unwrap_or_default()
}

fn extract_query_variables(query_text: &str) -> Vec<String> {
    let patterns = [
        r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?::[^}]*)?\}",
        r"\$([A-Za-z_][A-Za-z0-9_]*)",
        r"\[\[([A-Za-z_][A-Za-z0-9_]*)(?::[^\]]*)?\]\]",
    ];
    let mut values = Vec::new();
    for pattern in patterns {
        let regex = Regex::new(pattern).expect("invalid hard-coded variable regex");
        for capture in regex.captures_iter(query_text) {
            let Some(value) = capture.get(1).map(|item| item.as_str().trim()) else {
                continue;
            };
            if value.is_empty() {
                continue;
            }
            ordered_unique_push(&mut values, value);
        }
    }
    values
}

fn value_is_truthy(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(boolean)) => *boolean,
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0) != 0,
        Some(Value::String(text)) => {
            matches!(
                text.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes"
            )
        }
        _ => false,
    }
}

fn target_is_hidden(target: &Map<String, Value>) -> bool {
    value_is_truthy(target.get("hide"))
}

fn target_is_disabled(target: &Map<String, Value>) -> bool {
    value_is_truthy(target.get("disabled"))
}

fn normalize_relative_dashboard_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

const INSPECT_SOURCE_ROOT_FILENAME: &str = ".inspect-source-root";

fn normalize_index_entry_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if let Some((prefix, remainder)) = normalized.split_once('/') {
        if prefix.starts_with("org_") {
            if let Some((_raw_prefix, raw_remainder)) = remainder.rsplit_once("/raw/") {
                return format!("{prefix}/{raw_remainder}");
            }
            return format!("{prefix}/{}", remainder.trim_start_matches('/'));
        }
    }
    normalized
        .rsplit_once("/raw/")
        .map(|(_, remainder)| remainder.to_string())
        .unwrap_or(normalized)
}

fn write_inspect_output(output: &str, output_file: Option<&PathBuf>) -> Result<()> {
    let normalized = output.trim_end_matches('\n');
    if normalized.is_empty() {
        return Ok(());
    }
    if let Some(output_path) = output_file {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, format!("{normalized}\n"))?;
    }
    print!("{normalized}");
    println!();
    Ok(())
}

fn load_export_identity_values(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
    field_name: &str,
) -> Result<std::collections::BTreeSet<String>> {
    let mut values = std::collections::BTreeSet::new();
    if let Some(metadata) = metadata {
        let metadata_value = match field_name {
            "org" => metadata.org.clone().unwrap_or_default(),
            "orgId" => metadata.org_id.clone().unwrap_or_default(),
            _ => String::new(),
        };
        if !metadata_value.trim().is_empty() {
            values.insert(metadata_value.trim().to_string());
        }
    }
    let index_file = metadata
        .map(|item| item.index_file.clone())
        .unwrap_or_else(|| "index.json".to_string());
    let index_path = import_dir.join(&index_file);
    if index_path.is_file() {
        let raw = fs::read_to_string(&index_path)?;
        let entries: Vec<VariantIndexEntry> = serde_json::from_str(&raw).map_err(|error| {
            message(format!(
                "Invalid dashboard export index in {}: {error}",
                index_path.display()
            ))
        })?;
        for entry in entries {
            let value = match field_name {
                "org" => entry.org.trim(),
                "orgId" => entry.org_id.trim(),
                _ => "",
            };
            if !value.is_empty() {
                values.insert(value.to_string());
            }
        }
    }
    for folder in load_folder_inventory(import_dir, metadata)? {
        let value = match field_name {
            "org" => folder.org.trim(),
            "orgId" => folder.org_id.trim(),
            _ => "",
        };
        if !value.is_empty() {
            values.insert(value.to_string());
        }
    }
    for datasource in load_datasource_inventory(import_dir, metadata)? {
        let value = match field_name {
            "org" => datasource.org.trim(),
            "orgId" => datasource.org_id.trim(),
            _ => "",
        };
        if !value.is_empty() {
            values.insert(value.to_string());
        }
    }
    Ok(values)
}

fn resolve_export_identity_field(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
    field_name: &str,
) -> Result<Option<String>> {
    let values = load_export_identity_values(import_dir, metadata, field_name)?;
    if values.is_empty() {
        return Ok(None);
    }
    if values.len() > 1 {
        return Ok(None);
    }
    Ok(values.into_iter().next())
}

fn load_variant_index_entries(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<Vec<VariantIndexEntry>> {
    let index_file = metadata
        .map(|item| item.index_file.clone())
        .unwrap_or_else(|| "index.json".to_string());
    let index_path = import_dir.join(&index_file);
    if !index_path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&index_path)?;
    serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Invalid dashboard export index in {}: {error}",
            index_path.display()
        ))
    })
}

fn load_dashboard_org_scope_by_file(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<std::collections::BTreeMap<String, (String, String)>> {
    let mut scope_by_file = std::collections::BTreeMap::new();
    for entry in load_variant_index_entries(import_dir, metadata)? {
        scope_by_file.insert(
            normalize_index_entry_path(&entry.path),
            (entry.org, entry.org_id),
        );
    }
    Ok(scope_by_file)
}

fn load_inspect_source_root(import_dir: &Path) -> Option<PathBuf> {
    let source_root_path = import_dir.join(INSPECT_SOURCE_ROOT_FILENAME);
    let raw = fs::read_to_string(source_root_path).ok()?;
    let text = raw.trim();
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    }
}

fn resolve_dashboard_source_file_path(
    import_dir: &Path,
    dashboard_file: &Path,
    source_root: Option<&Path>,
) -> String {
    let Some(source_root) = source_root else {
        return dashboard_file.display().to_string();
    };
    let Ok(relative_path) = dashboard_file.strip_prefix(import_dir) else {
        return dashboard_file.display().to_string();
    };
    let mut parts = relative_path.components();
    let Some(first) = parts.next() else {
        return dashboard_file.display().to_string();
    };
    let first = first.as_os_str();
    if first.to_string_lossy().starts_with("org_") {
        return source_root
            .join(first)
            .join(RAW_EXPORT_SUBDIR)
            .join(parts.as_path())
            .display()
            .to_string();
    }
    source_root.join(relative_path).display().to_string()
}

fn normalize_merged_dashboard_folder_path(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let mut parts = normalized.split('/').collect::<Vec<&str>>();
    if parts.len() >= 2 && parts[0].starts_with("org_") {
        parts.drain(0..1);
        return parts.join("/");
    }
    normalized
        .rsplit_once("/raw/")
        .map(|(_, remainder)| remainder.to_string())
        .unwrap_or(normalized)
}

fn resolve_export_folder_inventory_item(
    document: &Map<String, Value>,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
) -> Option<FolderInventoryItem> {
    let folder_uid = object_field(document, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if !folder_uid.is_empty() {
        if let Some(folder) = folders_by_uid.get(&folder_uid) {
            return Some(folder.clone());
        }
        if folder_uid == DEFAULT_FOLDER_UID {
            return Some(FolderInventoryItem {
                uid: DEFAULT_FOLDER_UID.to_string(),
                title: DEFAULT_FOLDER_TITLE.to_string(),
                path: DEFAULT_FOLDER_TITLE.to_string(),
                parent_uid: None,
                org: String::new(),
                org_id: String::new(),
            });
        }
    }
    let relative_parent = dashboard_file
        .strip_prefix(import_dir)
        .ok()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| Path::new(""));
    let folder_name = normalize_merged_dashboard_folder_path(relative_parent);
    if !folder_name.is_empty() && folder_name != "." && folder_name != DEFAULT_FOLDER_TITLE {
        let matches = folders_by_uid
            .values()
            .filter(|item| item.title == folder_name)
            .collect::<Vec<&FolderInventoryItem>>();
        if matches.len() == 1 {
            return Some((*matches[0]).clone());
        }
    }
    if folder_name.is_empty() || folder_name == "." || folder_name == DEFAULT_FOLDER_TITLE {
        return Some(FolderInventoryItem {
            uid: DEFAULT_FOLDER_UID.to_string(),
            title: DEFAULT_FOLDER_TITLE.to_string(),
            path: DEFAULT_FOLDER_TITLE.to_string(),
            parent_uid: None,
            org: String::new(),
            org_id: String::new(),
        });
    }
    None
}

fn resolve_export_folder_path(
    document: &Map<String, Value>,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
) -> String {
    if let Some(folder) =
        resolve_export_folder_inventory_item(document, dashboard_file, import_dir, folders_by_uid)
    {
        if !folder.path.trim().is_empty() {
            return folder.path;
        }
    }
    let relative_parent = dashboard_file
        .strip_prefix(import_dir)
        .ok()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| Path::new(""));
    let folder_name = normalize_merged_dashboard_folder_path(relative_parent);
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

#[derive(Clone, Copy, Debug)]
enum DatasourceReference<'a> {
    String(&'a str),
    Object(DatasourceReferenceObject<'a>),
}

#[derive(Clone, Copy, Debug)]
struct DatasourceReferenceObject<'a> {
    uid: Option<&'a str>,
    name: Option<&'a str>,
    plugin_id: Option<&'a str>,
    datasource_type: Option<&'a str>,
}

impl<'a> DatasourceReferenceObject<'a> {
    fn from_value(reference: &'a Value) -> Option<Self> {
        let object = reference.as_object()?;
        let uid = object
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value));
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value));
        let plugin_id = object
            .get("pluginId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value));
        let datasource_type = object
            .get("type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value));
        if uid.is_none() && name.is_none() && plugin_id.is_none() && datasource_type.is_none() {
            None
        } else {
            Some(Self {
                uid,
                name,
                plugin_id,
                datasource_type,
            })
        }
    }

    fn summary_label(self) -> Option<&'a str> {
        self.name
            .or(self.uid)
            .or(self.plugin_id)
            .or(self.datasource_type)
    }

    fn uid_label(self) -> Option<&'a str> {
        self.uid
    }

    fn inventory_item(
        self,
        datasource_inventory: &'a [DatasourceInventoryItem],
    ) -> Option<&'a DatasourceInventoryItem> {
        datasource_inventory.iter().find(|datasource| {
            self.uid
                .map(|value| datasource.uid == value)
                .unwrap_or(false)
                || self
                    .name
                    .map(|value| datasource.name == value)
                    .unwrap_or(false)
        })
    }

    fn name_label(self, datasource_inventory: &'a [DatasourceInventoryItem]) -> Option<String> {
        if let Some(datasource) = self.inventory_item(datasource_inventory) {
            if !datasource.name.is_empty() {
                return Some(datasource.name.clone());
            }
        }
        self.uid
            .map(str::to_string)
            .or_else(|| self.name.map(str::to_string))
    }

    fn type_label(self, datasource_inventory: &'a [DatasourceInventoryItem]) -> Option<String> {
        if let Some(datasource) = self.inventory_item(datasource_inventory) {
            if !datasource.datasource_type.is_empty() {
                return Some(datasource.datasource_type.clone());
            }
        }
        self.datasource_type
            .or(self.plugin_id)
            .map(|value| datasource_type_alias(value).to_string())
    }
}

impl<'a> DatasourceReference<'a> {
    fn parse(reference: &'a Value) -> Option<Self> {
        if reference.is_null() || is_builtin_datasource_ref(reference) {
            return None;
        }
        match reference {
            Value::String(text) => {
                let normalized = text.trim();
                if normalized.is_empty() {
                    None
                } else {
                    Some(Self::String(normalized))
                }
            }
            Value::Object(_) => DatasourceReferenceObject::from_value(reference).map(Self::Object),
            _ => None,
        }
    }

    fn summary_label(self) -> Option<String> {
        match self {
            Self::String(text) => {
                if is_placeholder_string(text) {
                    None
                } else {
                    Some(text.to_string())
                }
            }
            Self::Object(reference) => reference.summary_label().map(str::to_string),
        }
    }

    fn uid_label(self) -> Option<String> {
        match self {
            Self::String(text) => {
                if is_placeholder_string(text) {
                    None
                } else {
                    Some(text.to_string())
                }
            }
            Self::Object(reference) => reference.uid_label().map(str::to_string),
        }
    }

    fn name_label(self, datasource_inventory: &'a [DatasourceInventoryItem]) -> Option<String> {
        match self {
            Self::String(text) => {
                let normalized = text.trim();
                if normalized.is_empty() || is_placeholder_string(normalized) {
                    return None;
                }
                datasource_inventory
                    .iter()
                    .find(|datasource| {
                        datasource.uid == normalized || datasource.name == normalized
                    })
                    .map(|datasource| datasource.name.clone())
                    .or_else(|| Some(text.to_string()))
            }
            Self::Object(reference) => reference.name_label(datasource_inventory),
        }
    }

    fn type_label(self, datasource_inventory: &'a [DatasourceInventoryItem]) -> Option<String> {
        match self {
            Self::String(text) => {
                let normalized = text.trim();
                if normalized.is_empty() || is_placeholder_string(normalized) {
                    None
                } else {
                    datasource_inventory
                        .iter()
                        .find(|datasource| {
                            datasource.uid == normalized || datasource.name == normalized
                        })
                        .map(|datasource| datasource.datasource_type.clone())
                        .or_else(|| Some(datasource_type_alias(normalized).to_string()))
                }
            }
            Self::Object(reference) => reference.type_label(datasource_inventory),
        }
    }

    fn inventory_item(
        self,
        datasource_inventory: &'a [DatasourceInventoryItem],
    ) -> Option<&'a DatasourceInventoryItem> {
        match self {
            Self::String(text) => {
                let normalized = text.trim();
                if normalized.is_empty() || is_placeholder_string(normalized) {
                    None
                } else {
                    datasource_inventory.iter().find(|datasource| {
                        datasource.uid == normalized || datasource.name == normalized
                    })
                }
            }
            Self::Object(reference) => reference.inventory_item(datasource_inventory),
        }
    }
}

fn summarize_datasource_ref(reference: &Value) -> Option<String> {
    DatasourceReference::parse(reference)?.summary_label()
}

fn summarize_datasource_uid(reference: &Value) -> Option<String> {
    DatasourceReference::parse(reference)?.uid_label()
}

fn summarize_datasource_name(
    reference: &Value,
    datasource_inventory: &[DatasourceInventoryItem],
) -> Option<String> {
    DatasourceReference::parse(reference)?.name_label(datasource_inventory)
}

fn summarize_datasource_type(
    reference: &Value,
    datasource_inventory: &[DatasourceInventoryItem],
) -> Option<String> {
    DatasourceReference::parse(reference)?.type_label(datasource_inventory)
}

fn resolve_datasource_inventory_item<'a>(
    reference: &'a Value,
    datasource_inventory: &'a [DatasourceInventoryItem],
) -> Option<&'a DatasourceInventoryItem> {
    DatasourceReference::parse(reference)?.inventory_item(datasource_inventory)
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

/// Purpose: implementation note.
pub(crate) fn resolve_query_analyzer_family(context: &QueryExtractionContext<'_>) -> &'static str {
    if let Some(family) = resolve_query_analyzer_family_from_datasource_type(datasource_type_alias(
        context.resolved_datasource_type,
    )) {
        return family;
    }
    for reference in [
        context.target.get("datasource"),
        context.panel.get("datasource"),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(datasource_type) = datasource_type_from_reference(reference) {
            if let Some(family) =
                resolve_query_analyzer_family_from_datasource_type(datasource_type.as_str())
            {
                return family;
            }
        }
    }
    if let Some(family) =
        resolve_query_analyzer_family_from_query_signature(context.query_field, context.query_text)
    {
        return family;
    }
    DATASOURCE_FAMILY_UNKNOWN
}

/// Purpose: implementation note.
pub(crate) fn dispatch_query_analysis(context: &QueryExtractionContext<'_>) -> QueryAnalysis {
    match resolve_query_analyzer_family(context) {
        DATASOURCE_FAMILY_PROMETHEUS => inspect_analyzer_prometheus::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_LOKI => inspect_analyzer_loki::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_FLUX => inspect_analyzer_flux::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_SQL => inspect_analyzer_sql::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_SEARCH => inspect_analyzer_search::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_TRACING => analyze_tracing_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        _ => QueryAnalysis {
            metrics: extract_metric_names(context.query_text),
            functions: Vec::new(),
            measurements: extract_query_measurements(context.target, context.query_text),
            buckets: extract_query_buckets(context.target, context.query_text),
        },
    }
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

fn extract_template_variable_names_from_text(text: &str) -> Vec<String> {
    let regex = Regex::new(
        r#"(?m)(?:^|[^A-Za-z0-9_])\$(?:\{([A-Za-z_][A-Za-z0-9_]*)\}|([A-Za-z_][A-Za-z0-9_]*))"#,
    )
    .expect("invalid hard-coded dashboard template variable regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(text) {
        let value = captures
            .get(1)
            .or_else(|| captures.get(2))
            .map(|item| item.as_str())
            .unwrap_or_default();
        ordered_unique_push(&mut values, value);
    }
    values
}

fn extract_template_variable_names_from_value(value: &Value, skip_keys: &[&str]) -> Vec<String> {
    fn visit(value: &Value, skip_keys: &[&str], values: &mut Vec<String>) {
        match value {
            Value::String(text) => {
                for variable in extract_template_variable_names_from_text(text) {
                    ordered_unique_push(values, &variable);
                }
            }
            Value::Array(items) => {
                for item in items {
                    visit(item, skip_keys, values);
                }
            }
            Value::Object(object) => {
                for (key, item) in object {
                    if skip_keys.iter().any(|skip_key| skip_key == &key.as_str()) {
                        continue;
                    }
                    visit(item, skip_keys, values);
                }
            }
            _ => {}
        }
    }

    let mut values = Vec::new();
    visit(value, skip_keys, &mut values);
    values
}

fn summarize_panel_datasource_key(reference: &Value) -> Option<String> {
    if reference.is_null() {
        return None;
    }
    match reference {
        Value::String(text) => {
            let normalized = text.trim();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized.to_string())
            }
        }
        Value::Object(object) => {
            for key in ["uid", "name", "type"] {
                if let Some(value) = object.get(key).and_then(Value::as_str) {
                    let normalized = value.trim();
                    if !normalized.is_empty() && !is_placeholder_string(normalized) {
                        return Some(normalized.to_string());
                    }
                }
            }
            None
        }
        _ => None,
    }
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

pub(crate) fn ordered_unique_push(values: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }
    if !values.iter().any(|value| value == trimmed) {
        values.push(trimmed.to_string());
    }
}

fn datasource_type_from_reference(reference: &Value) -> Option<String> {
    DatasourceReference::parse(reference)?.type_label(&[])
}

pub(crate) fn resolve_query_analyzer_family_from_datasource_type(
    datasource_type: &str,
) -> Option<&'static str> {
    match canonicalize_query_analyzer_datasource_type(datasource_type) {
        "loki" => Some(DATASOURCE_FAMILY_LOKI),
        "prometheus" => Some(DATASOURCE_FAMILY_PROMETHEUS),
        "tempo" | "jaeger" | "zipkin" => Some(DATASOURCE_FAMILY_TRACING),
        "influxdb" | "flux" => Some(DATASOURCE_FAMILY_FLUX),
        "mysql" | "postgres" | "postgresql" | "mssql" => Some(DATASOURCE_FAMILY_SQL),
        "elasticsearch" | "opensearch" => Some(DATASOURCE_FAMILY_SEARCH),
        _ => None,
    }
}

fn canonicalize_query_analyzer_datasource_type(datasource_type: &str) -> &str {
    let datasource_type = datasource_type_alias(datasource_type);
    if let Some(normalized) = datasource_type
        .strip_prefix("grafana-")
        .and_then(|value| value.strip_suffix("-datasource"))
    {
        return normalized;
    }
    if let Some(normalized) = datasource_type.strip_suffix("-datasource") {
        return normalized;
    }
    datasource_type
}

pub(crate) fn resolve_query_analyzer_family_from_query_signature(
    query_field: &str,
    query_text: &str,
) -> Option<&'static str> {
    if matches!(query_field, "rawSql" | "sql") {
        return Some(DATASOURCE_FAMILY_SQL);
    }
    if query_field == "logql" {
        return Some(DATASOURCE_FAMILY_LOKI);
    }
    if query_field == "expr" {
        return Some(DATASOURCE_FAMILY_PROMETHEUS);
    }
    let trimmed = query_text.trim_start();
    if trimmed.starts_with("from(") || trimmed.starts_with("from (") || trimmed.contains("|>") {
        return Some(DATASOURCE_FAMILY_FLUX);
    }
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("select ")
        || lowered.starts_with("with ")
        || lowered.starts_with("insert ")
        || lowered.starts_with("update ")
        || lowered.starts_with("delete ")
    {
        return Some(DATASOURCE_FAMILY_SQL);
    }
    if inspect_analyzer_search::query_text_looks_like_search(query_text) {
        return Some(DATASOURCE_FAMILY_SEARCH);
    }
    None
}

fn tracing_field_hint_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?i)(?:^|[^A-Za-z0-9_.-])((?:service\.name|span\.name|resource\.service\.name|trace\.id|traceID|traceId))\s*(?:=|:)\s*(?:"(?:\\.|[^"\\])*"|\[[^\]]*\]|\{[^}]*\}|\([^\)]*\)|[A-Za-z0-9_*?.-]+)"#,
        )
        .expect("invalid hard-coded tracing field regex")
    })
}

fn extract_tracing_measurements(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for captures in tracing_field_hint_regex().captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
}

/// Trace inspection is intentionally narrow: only explicit field-shaped hints are retained.
pub(crate) fn analyze_tracing_query(
    _panel: &Map<String, Value>,
    _target: &Map<String, Value>,
    _query_field: &str,
    query_text: &str,
) -> QueryAnalysis {
    QueryAnalysis {
        metrics: Vec::new(),
        functions: Vec::new(),
        measurements: extract_tracing_measurements(query_text),
        buckets: Vec::new(),
    }
}

/// extract query field and text.
pub(crate) fn extract_query_field_and_text(target: &Map<String, Value>) -> (String, String) {
    for key in [
        "expr",
        "expression",
        "query",
        "logql",
        "rawSql",
        "sql",
        "rawQuery",
    ] {
        if let Some(value) = target.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return (key.to_string(), trimmed.to_string());
            }
        }
    }
    let synthesized = synthesize_influx_builder_query(target);
    if !synthesized.is_empty() {
        return ("builder".to_string(), synthesized);
    }
    (String::new(), String::new())
}

fn first_step_param(step: &Map<String, Value>) -> String {
    step.get("params")
        .and_then(Value::as_array)
        .and_then(|params| params.first())
        .map(|value| match value {
            Value::String(text) => text.trim().to_string(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn render_influx_select_chain(chain: &Value) -> String {
    let Some(steps) = chain.as_array() else {
        return String::new();
    };
    let mut expression = String::new();
    for step in steps {
        let Some(step_object) = step.as_object() else {
            continue;
        };
        let step_type = string_field(step_object, "type", "");
        let param = first_step_param(step_object);
        match step_type.as_str() {
            "field" => {
                if !param.is_empty() {
                    expression = format!("\"{param}\"");
                }
            }
            "math" => {
                if !param.is_empty() {
                    if expression.is_empty() {
                        expression = param;
                    } else {
                        expression.push_str(&param);
                    }
                }
            }
            "alias" => {}
            "" => {}
            _ => {
                if !expression.is_empty() {
                    expression = format!("{step_type}({expression})");
                } else if !param.is_empty() {
                    expression = format!("{step_type}({param})");
                } else {
                    expression = format!("{step_type}()");
                }
            }
        }
    }
    expression.trim().to_string()
}

fn render_influx_group_by_clause(group_by: Option<&Value>) -> String {
    let Some(items) = group_by.and_then(Value::as_array) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for item in items {
        let Some(group_object) = item.as_object() else {
            continue;
        };
        let group_type = string_field(group_object, "type", "");
        let param = first_step_param(group_object);
        let rendered = match group_type.as_str() {
            "time" if !param.is_empty() => format!("time({param})"),
            "fill" if !param.is_empty() => format!("fill({param})"),
            "tag" if !param.is_empty() => format!("\"{param}\""),
            _ if !group_type.is_empty() && !param.is_empty() => format!("{group_type}({param})"),
            _ if !group_type.is_empty() => group_type,
            _ => String::new(),
        };
        if !rendered.is_empty() {
            parts.push(rendered);
        }
    }
    parts.join(", ")
}

fn render_influx_where_clause(tags: Option<&Value>) -> String {
    let Some(items) = tags.and_then(Value::as_array) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for item in items {
        let Some(tag_object) = item.as_object() else {
            continue;
        };
        let key = string_field(tag_object, "key", "");
        let operator = string_field(tag_object, "operator", "=");
        let value = string_field(tag_object, "value", "");
        if key.is_empty() || value.is_empty() {
            continue;
        }
        let condition = string_field(tag_object, "condition", "").to_ascii_uppercase();
        if !parts.is_empty() && (condition == "AND" || condition == "OR") {
            parts.push(condition);
        }
        parts.push(format!("\"{key}\" {operator} {value}"));
    }
    parts.join(" ")
}

fn synthesize_influx_builder_query(target: &Map<String, Value>) -> String {
    let measurement = string_field(target, "measurement", "");
    let select_parts = target
        .get("select")
        .and_then(Value::as_array)
        .map(|chains| {
            chains
                .iter()
                .map(render_influx_select_chain)
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if measurement.is_empty() && select_parts.is_empty() {
        return String::new();
    }
    let mut query = format!(
        "SELECT {}",
        if select_parts.is_empty() {
            "*".to_string()
        } else {
            select_parts.join(", ")
        }
    );
    if !measurement.is_empty() {
        query.push_str(&format!(" FROM \"{measurement}\""));
    }
    let where_clause = render_influx_where_clause(target.get("tags"));
    if !where_clause.is_empty() {
        query.push_str(&format!(" WHERE {where_clause}"));
    }
    let group_by_clause = render_influx_group_by_clause(target.get("groupBy"));
    if !group_by_clause.is_empty() {
        query.push_str(&format!(" GROUP BY {group_by_clause}"));
    }
    query
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

/// extract prometheus metric names.
pub(crate) fn extract_prometheus_metric_names(query_text: &str) -> Vec<String> {
    if query_text.trim().is_empty() {
        return Vec::new();
    }
    let token_regex =
        Regex::new(r"[A-Za-z_:][A-Za-z0-9_:]*").expect("invalid hard-coded metric regex");
    let quoted_regex =
        Regex::new(r#""(?:\\.|[^"\\])*""#).expect("invalid hard-coded quoted string regex");
    let vector_matching_regex = Regex::new(r"\b(?:by|without|on|ignoring)\s*\(\s*[^)]*\)")
        .expect("invalid hard-coded promql vector matching regex");
    let group_modifier_regex = Regex::new(r"\b(?:group_left|group_right)\s*(?:\(\s*[^)]*\))?")
        .expect("invalid hard-coded promql group modifier regex");
    let matcher_regex = Regex::new(r"\{[^{}]*\}").expect("invalid hard-coded promql matcher regex");
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
        "sum",
        "min",
        "max",
        "avg",
        "count",
        "stddev",
        "stdvar",
        "bottomk",
        "topk",
        "quantile",
        "count_values",
        "rate",
        "irate",
        "increase",
        "delta",
        "idelta",
        "deriv",
        "predict_linear",
        "holt_winters",
        "sort",
        "sort_desc",
        "label_replace",
        "label_join",
        "histogram_quantile",
        "clamp_max",
        "clamp_min",
        "abs",
        "absent",
        "ceil",
        "floor",
        "ln",
        "log2",
        "log10",
        "round",
        "scalar",
        "vector",
        "year",
        "month",
        "day_of_month",
        "day_of_week",
        "hour",
        "minute",
        "time",
    ];
    for capture in quoted_captures(query_text, r#"__name__\s*=\s*"([A-Za-z_:][A-Za-z0-9_:]*)""#) {
        values.insert(capture);
    }
    let sanitized_query = quoted_regex.replace_all(query_text, "\"\"");
    let sanitized_query = vector_matching_regex.replace_all(&sanitized_query, " ");
    let sanitized_query = group_modifier_regex.replace_all(&sanitized_query, " ");
    let sanitized_query = matcher_regex.replace_all(&sanitized_query, "{}");
    for matched in token_regex.find_iter(&sanitized_query) {
        let start = matched.start();
        let end = matched.end();
        let previous = sanitized_query[..start].chars().next_back();
        if previous
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let next = sanitized_query[end..].chars().next();
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
        let trailing = sanitized_query[end..].trim_start();
        if trailing.starts_with('(') {
            continue;
        }
        if ["=", "!=", "=~", "!~"]
            .iter()
            .any(|operator| trailing.starts_with(operator))
        {
            continue;
        }
        values.insert(token.to_string());
    }
    values.into_iter().collect()
}

/// extract query measurements.
pub(crate) fn extract_query_measurements(
    target: &Map<String, Value>,
    query_text: &str,
) -> Vec<String> {
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

/// extract query buckets.
pub(crate) fn extract_query_buckets(target: &Map<String, Value>, query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    if let Some(bucket) = target.get("bucket").and_then(Value::as_str) {
        let trimmed = bucket.trim();
        if !trimmed.is_empty() {
            ordered_unique_push(&mut values, trimmed);
        }
    }
    for value in string_list_field(target, "buckets") {
        ordered_unique_push(&mut values, &value);
    }
    for value in quoted_captures(query_text, r#"from\s*\(\s*bucket\s*:\s*"([^"]+)""#) {
        ordered_unique_push(&mut values, &value);
    }
    for value in extract_influxql_time_windows(query_text) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

/// extract Prometheus-style range windows used in query functions.
pub(crate) fn extract_prometheus_range_windows(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in quoted_captures(query_text, r#"\[([^\[\]]+)\]"#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

/// extract InfluxQL GROUP BY time windows used for dashboard aggregation.
pub(crate) fn extract_influxql_time_windows(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    if !query_text.to_ascii_lowercase().contains("group by") {
        return values;
    }
    for value in quoted_captures(query_text, r#"(?i)\btime\s*\(\s*([^)]+?)\s*\)"#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn extract_influxql_select_clause(query_text: &str) -> Option<String> {
    let query_text = strip_sql_comments(query_text);
    let regex = Regex::new(r#"(?is)^\s*select\s+(.*?)\s+\bfrom\b"#)
        .expect("invalid hard-coded influxql select regex");
    regex
        .captures(&query_text)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
}

/// extract InfluxQL field references from the SELECT clause.
pub(crate) fn extract_influxql_select_metrics(query_text: &str) -> Vec<String> {
    let Some(select_clause) = extract_influxql_select_clause(query_text) else {
        return Vec::new();
    };
    let select_clause = Regex::new(r#"(?i)\bas\s+"[^"]+""#)
        .expect("invalid hard-coded influxql alias regex")
        .replace_all(&select_clause, "")
        .into_owned();
    let mut values = Vec::new();
    for value in quoted_captures(&select_clause, r#""([^"]+)""#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

/// extract InfluxQL functions from the SELECT clause.
pub(crate) fn extract_influxql_select_functions(query_text: &str) -> Vec<String> {
    let Some(select_clause) = extract_influxql_select_clause(query_text) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for value in quoted_captures(&select_clause, r#"\b([A-Za-z_][A-Za-z0-9_]*)\s*\("#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

/// extract Prometheus function and aggregation names.
pub(crate) fn extract_prometheus_functions(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .expect("invalid hard-coded promql function regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            let name = value.as_str();
            if matches!(
                name,
                "by" | "without" | "on" | "ignoring" | "group_left" | "group_right"
            ) {
                continue;
            }
            ordered_unique_push(&mut values, name);
        }
    }
    values
}

/// extract flux pipeline functions.
pub(crate) fn extract_flux_pipeline_functions(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    if let Some(value) = quoted_captures(query_text, r#"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\("#)
        .into_iter()
        .next()
    {
        ordered_unique_push(&mut values, &value);
    }
    for value in quoted_captures(query_text, r#"\|>\s*([A-Za-z_][A-Za-z0-9_]*)\s*\("#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn strip_sql_comments(query_text: &str) -> String {
    let block_regex = Regex::new(r"(?s)/\*.*?\*/").expect("invalid hard-coded sql comment regex");
    let line_regex = Regex::new(r"--[^\n]*").expect("invalid hard-coded sql line comment regex");
    let without_blocks = block_regex.replace_all(query_text, " ");
    line_regex.replace_all(&without_blocks, " ").into_owned()
}

// Normalize SQL identifiers into a stable dot-qualified form for dedup and
// cross-query matching.
fn normalize_sql_identifier(value: &str) -> String {
    value
        .split('.')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }
            let normalized = if trimmed.len() >= 2
                && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
                    || (trimmed.starts_with('`') && trimmed.ends_with('`'))
                    || (trimmed.starts_with('[') && trimmed.ends_with(']')))
            {
                &trimmed[1..trimmed.len() - 1]
            } else {
                trimmed
            };
            let normalized = normalized.trim();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized.to_string())
            }
        })
        .collect::<Vec<String>>()
        .join(".")
}

/// extract sql source references.
pub(crate) fn extract_sql_source_references(query_text: &str) -> Vec<String> {
    let query_text = strip_sql_comments(query_text);
    if query_text.trim().is_empty() {
        return Vec::new();
    }
    let cte_names = quoted_captures(
        &query_text,
        r#"(?i)\bwith\s+([A-Za-z_][A-Za-z0-9_$]*)\s+as\s*\("#,
    )
    .into_iter()
    .map(|value| value.to_ascii_lowercase())
    .collect::<std::collections::BTreeSet<String>>();
    let mut values = Vec::new();
    for value in quoted_captures(
        &query_text,
        r#"(?i)\b(?:from|join|update|into|delete\s+from)\s+((?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+"|`[^`]+`|\[[^\]]+\])(?:\s*\.\s*(?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+"|`[^`]+`|\[[^\]]+\])){0,2})"#,
    ) {
        let normalized = normalize_sql_identifier(&value);
        if !normalized.is_empty() && !cte_names.contains(&normalized.to_ascii_lowercase()) {
            ordered_unique_push(&mut values, &normalized);
        }
    }
    values
}

/// extract sql query shape hints.
pub(crate) fn extract_sql_query_shape_hints(query_text: &str) -> Vec<String> {
    let lowered = strip_sql_comments(query_text).to_ascii_lowercase();
    let patterns = [
        ("with", r"\bwith\b"),
        ("select", r"\bselect\b"),
        ("insert", r"\binsert\s+into\b"),
        ("update", r"\bupdate\b"),
        ("delete", r"\bdelete\s+from\b"),
        ("distinct", r"\bdistinct\b"),
        ("join", r"\bjoin\b"),
        ("where", r"\bwhere\b"),
        ("group_by", r"\bgroup\s+by\b"),
        ("having", r"\bhaving\b"),
        ("order_by", r"\border\s+by\b"),
        ("limit", r"\blimit\b"),
        ("top", r"\btop\s+\d+\b"),
        ("union", r"\bunion(?:\s+all)?\b"),
        ("window", r"\bover\s*\("),
        ("subquery", r"\b(?:from|join)\s*\("),
    ];
    let mut values = Vec::new();
    for (name, pattern) in patterns {
        let regex = Regex::new(pattern).expect("invalid hard-coded sql shape regex");
        if regex.is_match(&lowered) {
            values.push(name.to_string());
        }
    }
    values
}

fn collect_query_report_rows(
    panels: &[Value],
    context: &QueryReportContext<'_>,
    rows: &mut Vec<ExportInspectionQueryRow>,
) {
    for panel in panels {
        let Some(panel_object) = panel.as_object() else {
            continue;
        };
        let dashboard_tags = extract_dashboard_tags(context.dashboard);
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
        let panel_datasource_value = panel_object.get("datasource");
        let panel_variables = extract_template_variable_names_from_value(
            &Value::Object(panel_object.clone()),
            &["targets", "panels"],
        );
        if let Some(targets) = panel_object.get("targets").and_then(Value::as_array) {
            let panel_target_count = targets
                .iter()
                .filter(|target| target.as_object().is_some())
                .count();
            let panel_query_count = targets
                .iter()
                .filter_map(Value::as_object)
                .filter(|target_object| {
                    !target_is_disabled(target_object)
                        && !extract_query_field_and_text(target_object)
                            .1
                            .trim()
                            .is_empty()
                })
                .count();
            let mut panel_datasource_keys = std::collections::BTreeSet::new();
            for target in targets {
                let Some(target_object) = target.as_object() else {
                    continue;
                };
                if target_is_disabled(target_object) {
                    continue;
                }
                let panel_datasource_label = target_object
                    .get("datasource")
                    .and_then(summarize_panel_datasource_key)
                    .or_else(|| panel_datasource_value.and_then(summarize_panel_datasource_key))
                    .unwrap_or_default();
                if !panel_datasource_label.is_empty() {
                    panel_datasource_keys.insert(panel_datasource_label);
                }
            }
            let panel_datasource_count = panel_datasource_keys.len();
            for target in targets {
                let Some(target_object) = target.as_object() else {
                    continue;
                };
                let datasource = target_object
                    .get("datasource")
                    .and_then(summarize_datasource_ref)
                    .or_else(|| panel_datasource_value.and_then(summarize_datasource_ref))
                    .unwrap_or_default();
                let datasource_name = target_object
                    .get("datasource")
                    .and_then(|value| {
                        summarize_datasource_name(value, context.datasource_inventory)
                    })
                    .or_else(|| {
                        panel_datasource_value.and_then(|value| {
                            summarize_datasource_name(value, context.datasource_inventory)
                        })
                    })
                    .unwrap_or_default();
                let datasource_uid = target_object
                    .get("datasource")
                    .and_then(summarize_datasource_uid)
                    .or_else(|| panel_datasource_value.and_then(summarize_datasource_uid))
                    .unwrap_or_default();
                let datasource_type = target_object
                    .get("datasource")
                    .and_then(|value| {
                        summarize_datasource_type(value, context.datasource_inventory)
                    })
                    .or_else(|| {
                        panel_datasource_value.and_then(|value| {
                            summarize_datasource_type(value, context.datasource_inventory)
                        })
                    })
                    .unwrap_or_default();
                let datasource_inventory_item = target_object
                    .get("datasource")
                    .and_then(|value| {
                        resolve_datasource_inventory_item(value, context.datasource_inventory)
                    })
                    .or_else(|| {
                        panel_datasource_value.and_then(|value| {
                            resolve_datasource_inventory_item(value, context.datasource_inventory)
                        })
                    });
                let (query_field, query_text) = extract_query_field_and_text(target_object);
                let mut query_variables = extract_query_variables(&query_text);
                if query_variables.is_empty() {
                    query_variables = extract_template_variable_names_from_value(
                        &Value::Object(target_object.clone()),
                        &[],
                    );
                }
                let analysis = dispatch_query_analysis(&QueryExtractionContext {
                    panel: panel_object,
                    target: target_object,
                    query_field: &query_field,
                    query_text: &query_text,
                    resolved_datasource_type: &datasource_type,
                });
                rows.push(ExportInspectionQueryRow {
                    org: context.export_org.to_string(),
                    org_id: context.export_org_id.to_string(),
                    dashboard_uid: context.dashboard_uid.to_string(),
                    dashboard_title: context.dashboard_title.to_string(),
                    dashboard_tags: dashboard_tags.clone(),
                    folder_path: context.folder_path.to_string(),
                    folder_full_path: calculate_folder_full_path(context.folder_path),
                    folder_level: calculate_folder_level(context.folder_path),
                    folder_uid: context.folder_uid.to_string(),
                    parent_folder_uid: context.parent_folder_uid.to_string(),
                    panel_id: panel_id.clone(),
                    panel_title: panel_title.clone(),
                    panel_type: panel_type.clone(),
                    panel_target_count,
                    panel_query_count,
                    panel_datasource_count,
                    panel_variables: panel_variables.clone(),
                    ref_id: string_field(target_object, "refId", ""),
                    datasource,
                    datasource_name,
                    datasource_uid,
                    datasource_org: datasource_inventory_item
                        .map(|item| item.org.clone())
                        .unwrap_or_default(),
                    datasource_org_id: datasource_inventory_item
                        .map(|item| item.org_id.clone())
                        .unwrap_or_default(),
                    datasource_database: datasource_inventory_item
                        .map(|item| item.database.clone())
                        .unwrap_or_default(),
                    datasource_bucket: datasource_inventory_item
                        .map(|item| item.default_bucket.clone())
                        .unwrap_or_default(),
                    datasource_organization: datasource_inventory_item
                        .map(|item| item.organization.clone())
                        .unwrap_or_default(),
                    datasource_index_pattern: datasource_inventory_item
                        .map(|item| item.index_pattern.clone())
                        .unwrap_or_default(),
                    datasource_family: normalize_family_name(&datasource_type),
                    datasource_type,
                    query_field,
                    target_hidden: target_is_hidden(target_object).to_string(),
                    target_disabled: target_is_disabled(target_object).to_string(),
                    query_text,
                    query_variables,
                    metrics: analysis.metrics,
                    functions: analysis.functions,
                    measurements: analysis.measurements,
                    buckets: analysis.buckets,
                    file_path: context.dashboard_file_display.to_string(),
                });
            }
        }
        if let Some(children) = panel_object.get("panels").and_then(Value::as_array) {
            collect_query_report_rows(children, context, rows);
        }
    }
}

/// Purpose: implementation note.
pub(crate) fn build_export_inspection_query_report(
    import_dir: &Path,
) -> Result<ExportInspectionQueryReport> {
    let summary = build_export_inspection_summary(import_dir)?;
    let metadata = load_export_metadata(import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let dashboard_org_scope = load_dashboard_org_scope_by_file(import_dir, metadata.as_ref())?;
    let source_root = load_inspect_source_root(import_dir);
    let dashboard_files = discover_dashboard_files(import_dir)?;
    let datasource_inventory = load_datasource_inventory(import_dir, metadata.as_ref())?;
    let folder_inventory = load_folder_inventory(import_dir, metadata.as_ref())?;
    let folders_by_uid = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect::<std::collections::BTreeMap<String, FolderInventoryItem>>();
    let mut rows = Vec::new();
    let export_org = summary.export_org.clone().unwrap_or_default();
    let export_org_id = summary.export_org_id.clone().unwrap_or_default();

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
        let relative_dashboard_path = dashboard_file
            .strip_prefix(import_dir)
            .map(normalize_relative_dashboard_path)
            .unwrap_or_else(|_| normalize_relative_dashboard_path(dashboard_file));
        let (dashboard_org, dashboard_org_id) = dashboard_org_scope
            .get(&relative_dashboard_path)
            .cloned()
            .unwrap_or_else(|| (export_org.clone(), export_org_id.clone()));
        let mut folder_record = resolve_export_folder_inventory_item(
            document_object,
            dashboard_file,
            import_dir,
            &folders_by_uid,
        );
        if folder_record
            .as_ref()
            .map(|item| item.uid.is_empty())
            .unwrap_or(true)
        {
            let scoped_matches = folders_by_uid
                .values()
                .filter(|item| {
                    item.path == folder_path
                        && ((!dashboard_org_id.is_empty() && item.org_id == dashboard_org_id)
                            || (!dashboard_org.is_empty() && item.org == dashboard_org))
                })
                .collect::<Vec<&FolderInventoryItem>>();
            if scoped_matches.len() == 1 {
                folder_record = Some((*scoped_matches[0]).clone());
            }
        }
        let folder_uid = folder_record
            .as_ref()
            .map(|item| item.uid.trim().to_string())
            .unwrap_or_default();
        let parent_folder_uid = folder_record
            .as_ref()
            .and_then(|item| item.parent_uid.clone())
            .unwrap_or_default();
        let dashboard_file_display =
            resolve_dashboard_source_file_path(import_dir, dashboard_file, source_root.as_deref());
        let context = QueryReportContext {
            export_org: &dashboard_org,
            export_org_id: &dashboard_org_id,
            dashboard,
            dashboard_uid: &dashboard_uid,
            dashboard_title: &dashboard_title,
            folder_path: &folder_path,
            folder_uid: &folder_uid,
            parent_folder_uid: &parent_folder_uid,
            dashboard_file_display: &dashboard_file_display,
            datasource_inventory: &datasource_inventory,
        };
        if let Some(panels) = dashboard.get("panels").and_then(Value::as_array) {
            collect_query_report_rows(panels, &context, &mut rows);
        }
    }

    Ok(build_query_report(
        source_root
            .as_ref()
            .map_or(import_dir, |value| value.as_path())
            .display()
            .to_string(),
        summary.dashboard_count,
        summary.panel_count,
        summary.query_count,
        rows,
    ))
}

/// apply query report filters.
pub(crate) fn apply_query_report_filters(
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
            .map(|value| {
                row.datasource == value
                    || row.datasource_uid == value
                    || row.datasource_type == value
                    || row.datasource_family == value
            })
            .unwrap_or(true);
        let panel_match = panel_id_filter
            .map(|value| row.panel_id == value)
            .unwrap_or(true);
        datasource_match && panel_match
    });
    refresh_filtered_query_report_summary(&mut report);
    report
}

/// validate inspect export report args.
pub(crate) fn validate_inspect_export_report_args(args: &InspectExportArgs) -> Result<()> {
    let report_format = effective_inspect_report_format(args);
    if report_format.is_none() {
        if !args.report_columns.is_empty() {
            return Err(message(
                "--report-columns is only supported together with --report or report-like --output-format.",
            ));
        }
        if args.report_filter_datasource.is_some() {
            return Err(message(
                "--report-filter-datasource is only supported together with --report or report-like --output-format.",
            ));
        }
        if args.report_filter_panel_id.is_some() {
            return Err(message(
                "--report-filter-panel-id is only supported together with --report or report-like --output-format.",
            ));
        }
        return Ok(());
    }
    if report_format
        .map(|format| {
            matches!(
                format,
                InspectExportReportFormat::Governance | InspectExportReportFormat::GovernanceJson
            )
        })
        .unwrap_or(false)
        && !args.report_columns.is_empty()
    {
        return Err(message(
            "--report-columns is not supported with governance output.",
        ));
    }
    if report_format
        .map(|format| !report_format_supports_columns(format))
        .unwrap_or(false)
        && !args.report_columns.is_empty()
    {
        return Err(message(
            "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes.",
        ));
    }
    let _ = resolve_report_column_ids_for_format(report_format, &args.report_columns)?;
    Ok(())
}

fn map_output_format_to_report(
    output_format: InspectOutputFormat,
) -> Option<InspectExportReportFormat> {
    match output_format {
        InspectOutputFormat::Text | InspectOutputFormat::Table | InspectOutputFormat::Json => None,
        InspectOutputFormat::ReportTable => Some(InspectExportReportFormat::Table),
        InspectOutputFormat::ReportCsv => Some(InspectExportReportFormat::Csv),
        InspectOutputFormat::ReportJson => Some(InspectExportReportFormat::Json),
        InspectOutputFormat::ReportTree => Some(InspectExportReportFormat::Tree),
        InspectOutputFormat::ReportTreeTable => Some(InspectExportReportFormat::TreeTable),
        InspectOutputFormat::ReportDependency => Some(InspectExportReportFormat::Dependency),
        InspectOutputFormat::ReportDependencyJson => {
            Some(InspectExportReportFormat::DependencyJson)
        }
        InspectOutputFormat::Governance => Some(InspectExportReportFormat::Governance),
        InspectOutputFormat::GovernanceJson => Some(InspectExportReportFormat::GovernanceJson),
    }
}

fn effective_inspect_report_format(args: &InspectExportArgs) -> Option<InspectExportReportFormat> {
    args.report
        .or_else(|| args.output_format.and_then(map_output_format_to_report))
}

fn effective_inspect_json(args: &InspectExportArgs) -> bool {
    args.json || matches!(args.output_format, Some(InspectOutputFormat::Json))
}

fn effective_inspect_table(args: &InspectExportArgs) -> bool {
    args.table || matches!(args.output_format, Some(InspectOutputFormat::Table))
}

/// Purpose: implementation note.
pub(crate) fn build_export_inspection_summary(
    import_dir: &Path,
) -> Result<ExportInspectionSummary> {
    let metadata = load_export_metadata(import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let source_root = load_inspect_source_root(import_dir);
    let export_org = resolve_export_identity_field(import_dir, metadata.as_ref(), "org")?;
    let export_org_id = resolve_export_identity_field(import_dir, metadata.as_ref(), "orgId")?;
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
    let orphaned_datasource_summary = datasource_inventory_summary
        .iter()
        .filter(|item| item.reference_count == 0 && item.dashboard_count == 0)
        .cloned()
        .collect::<Vec<DatasourceInventorySummary>>();
    mixed_dashboards.sort_by(|left, right| {
        left.folder_path
            .cmp(&right.folder_path)
            .then(left.title.cmp(&right.title))
            .then(left.uid.cmp(&right.uid))
    });

    Ok(ExportInspectionSummary {
        import_dir: source_root
            .as_ref()
            .map_or(import_dir, |value| value.as_path())
            .display()
            .to_string(),
        export_org,
        export_org_id,
        dashboard_count: dashboard_files.len(),
        folder_count: folder_paths.len(),
        panel_count: total_panels,
        query_count: total_queries,
        datasource_inventory_count: datasource_inventory_summary.len(),
        orphaned_datasource_count: orphaned_datasource_summary.len(),
        mixed_dashboard_count: mixed_dashboards.len(),
        folder_paths,
        datasource_usage,
        datasource_inventory: datasource_inventory_summary,
        orphaned_datasources: orphaned_datasource_summary,
        mixed_dashboards,
    })
}

/// Purpose: implementation note.
pub(crate) fn build_export_inspection_summary_rows(
    summary: &ExportInspectionSummary,
) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    if let Some(export_org) = &summary.export_org {
        rows.push(vec!["export_org".to_string(), export_org.clone()]);
    }
    if let Some(export_org_id) = &summary.export_org_id {
        rows.push(vec!["export_org_id".to_string(), export_org_id.clone()]);
    }
    rows.extend([
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
            "orphaned_datasource_count".to_string(),
            summary.orphaned_datasource_count.to_string(),
        ],
        vec![
            "mixed_datasource_dashboard_count".to_string(),
            summary.mixed_dashboard_count.to_string(),
        ],
    ]);
    rows
}

fn analyze_export_dir_at_path(args: &InspectExportArgs, import_dir: &Path) -> Result<usize> {
    let write_output =
        |output: &str| -> Result<()> { write_inspect_output(output, args.output_file.as_ref()) };

    if let Some(report_format) = effective_inspect_report_format(args) {
        let report = apply_query_report_filters(
            build_export_inspection_query_report(import_dir)?,
            args.report_filter_datasource.as_deref(),
            args.report_filter_panel_id.as_deref(),
        );
        if report_format == InspectExportReportFormat::Governance
            || report_format == InspectExportReportFormat::GovernanceJson
        {
            let summary = build_export_inspection_summary(import_dir)?;
            let governance = build_export_inspection_governance_document(&summary, &report);
            let mut output = String::new();
            if report_format == InspectExportReportFormat::GovernanceJson {
                output.push_str(&serde_json::to_string_pretty(&governance)?);
                output.push('\n');
            } else {
                for line in render_governance_table_report(&summary.import_dir, &governance) {
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            write_output(&output)?;
            return Ok(summary.dashboard_count);
        }
        if report_format == InspectExportReportFormat::Json {
            let document = build_export_inspection_query_report_document(&report);
            let output = format!("{}\n", serde_json::to_string_pretty(&document)?);
            write_output(&output)?;
            return Ok(report.summary.dashboard_count);
        }
        if report_format == InspectExportReportFormat::Dependency
            || report_format == InspectExportReportFormat::DependencyJson
        {
            let metadata = load_export_metadata(import_dir, Some(RAW_EXPORT_SUBDIR))?;
            let datasource_inventory = load_datasource_inventory(import_dir, metadata.as_ref())?;
            let report_document = build_export_inspection_query_report_document(&report);
            let query_rows = report_document
                .queries
                .iter()
                .map(|row| {
                    serde_json::to_value(row).map_err(|error| {
                        message(format!("failed to serialize dependency query row: {error}"))
                    })
                })
                .collect::<Result<Vec<Value>>>()?;
            let payload = build_offline_dependency_contract(&query_rows, &datasource_inventory);
            let output = format!("{}\n", serde_json::to_string_pretty(&payload)?);
            write_output(&output)?;
            return Ok(report.summary.dashboard_count);
        }
        if report_format == InspectExportReportFormat::Tree {
            let mut output = String::new();
            for line in render_grouped_query_report(&report) {
                output.push_str(&line);
                output.push('\n');
            }
            write_output(&output)?;
            return Ok(report.summary.dashboard_count);
        }

        let column_ids =
            resolve_report_column_ids_for_format(Some(report_format), &args.report_columns)?;
        if report_format == InspectExportReportFormat::TreeTable {
            let mut output = String::new();
            for line in render_grouped_query_table_report(&report, &column_ids, !args.no_header) {
                output.push_str(&line);
                output.push('\n');
            }
            write_output(&output)?;
            return Ok(report.summary.dashboard_count);
        }
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
            let mut output = String::new();
            for line in render_csv(&headers, &rows) {
                output.push_str(&line);
                output.push('\n');
            }
            write_output(&output)?;
            return Ok(report.summary.dashboard_count);
        }

        let mut output = String::new();
        output.push_str(&format!(
            "Export inspection report: {}\n\n",
            report.import_dir
        ));
        output.push_str("# Query report\n");
        for line in render_simple_table(&headers, &rows, !args.no_header) {
            output.push_str(&line);
            output.push('\n');
        }
        write_output(&output)?;
        return Ok(report.summary.dashboard_count);
    }

    let summary = build_export_inspection_summary(import_dir)?;
    if effective_inspect_json(args) {
        let document = build_export_inspection_summary_document(&summary);
        let output = format!("{}\n", serde_json::to_string_pretty(&document)?);
        write_output(&output)?;
        return Ok(summary.dashboard_count);
    }

    let mut output = String::new();
    output.push_str(&format!("Export inspection: {}\n", summary.import_dir));
    if effective_inspect_table(args) {
        output.push('\n');
        output.push_str("# Summary\n");
        let summary_rows = build_export_inspection_summary_rows(&summary);
        for line in render_simple_table(&["METRIC", "VALUE"], &summary_rows, !args.no_header) {
            output.push_str(&line);
            output.push('\n');
        }
    } else {
        if let Some(export_org) = &summary.export_org {
            output.push_str(&format!("Export org: {}\n", export_org));
        }
        if let Some(export_org_id) = &summary.export_org_id {
            output.push_str(&format!("Export orgId: {}\n", export_org_id));
        }
        output.push_str(&format!("Dashboards: {}\n", summary.dashboard_count));
        output.push_str(&format!("Folders: {}\n", summary.folder_count));
        output.push_str(&format!("Panels: {}\n", summary.panel_count));
        output.push_str(&format!("Queries: {}\n", summary.query_count));
        output.push_str(&format!(
            "Datasource inventory: {}\n",
            summary.datasource_inventory_count
        ));
        output.push_str(&format!(
            "Orphaned datasources: {}\n",
            summary.orphaned_datasource_count
        ));
        output.push_str(&format!(
            "Mixed datasource dashboards: {}\n",
            summary.mixed_dashboard_count
        ));
    }

    output.push('\n');
    output.push_str("# Folder paths\n");
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
        output.push_str(&line);
        output.push('\n');
    }

    output.push('\n');
    output.push_str("# Datasource usage\n");
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
        output.push_str(&line);
        output.push('\n');
    }

    if !summary.datasource_inventory.is_empty() {
        output.push('\n');
        output.push_str("# Datasource inventory\n");
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
            output.push_str(&line);
            output.push('\n');
        }
    }

    if !summary.orphaned_datasources.is_empty() {
        output.push('\n');
        output.push_str("# Orphaned datasources\n");
        let orphaned_rows = summary
            .orphaned_datasources
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
            ],
            &orphaned_rows,
            !args.no_header,
        ) {
            output.push_str(&line);
            output.push('\n');
        }
    }

    if !summary.mixed_dashboards.is_empty() {
        output.push('\n');
        output.push_str("# Mixed datasource dashboards\n");
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
            output.push_str(&line);
            output.push('\n');
        }
    }
    write_output(&output)?;
    Ok(summary.dashboard_count)
}

fn run_interactive_inspect_live_tui_from_dir(import_dir: &Path) -> Result<usize> {
    let summary = build_export_inspection_summary(import_dir)?;
    let report = build_export_inspection_query_report(import_dir)?;
    let governance = build_export_inspection_governance_document(&summary, &report);
    run_inspect_live_tui(&summary, &governance, &report)?;
    Ok(summary.dashboard_count)
}

/// analyze export dir.
pub(crate) fn analyze_export_dir(args: &InspectExportArgs) -> Result<usize> {
    validate_inspect_export_report_args(args)?;
    let temp_dir = TempInspectDir::new("inspect-export")?;
    let import_dir = prepare_inspect_export_import_dir(&temp_dir.path, &args.import_dir)?;
    analyze_export_dir_at_path(args, &import_dir)
}

pub(crate) struct TempInspectDir {
    pub(crate) path: PathBuf,
}

impl TempInspectDir {
    pub(crate) fn new(prefix: &str) -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| message(format!("Failed to build {prefix} temp path: {error}")))?
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "grafana-utils-{prefix}-{}-{timestamp}",
            process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempInspectDir {
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
        progress: args.progress,
        verbose: false,
    }
}

fn build_live_scan_progress_bar(total: usize) -> ProgressBar {
    let bar = ProgressBar::new(total as u64);
    let style = ProgressStyle::with_template(
        "{spinner:.green} dashboards {pos}/{len} [{bar:40.cyan/blue}] {msg}",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar());
    bar.set_style(style.progress_chars("##-"));
    bar
}

pub(crate) fn snapshot_live_dashboard_export_with_fetcher<F>(
    raw_dir: &Path,
    summaries: &[Map<String, Value>],
    concurrency: usize,
    progress: bool,
    fetch_dashboard: F,
) -> Result<usize>
where
    F: Fn(&str) -> Result<Value> + Sync,
{
    fs::create_dir_all(raw_dir)?;
    let progress_bar = if progress {
        Some(build_live_scan_progress_bar(summaries.len()))
    } else {
        None
    };
    let thread_count = concurrency.max(1);
    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .map_err(|error| {
            message(format!(
                "Failed to build dashboard scan worker pool: {error}"
            ))
        })?;
    let results = pool.install(|| {
        summaries
            .par_iter()
            .map(|summary| {
                let uid = string_field(summary, "uid", "");
                let output_path = build_output_path(raw_dir, summary, false);
                let payload = fetch_dashboard(&uid)?;
                write_dashboard(&payload, &output_path, false)?;
                if let Some(bar) = progress_bar.as_ref() {
                    bar.inc(1);
                    bar.set_message(uid);
                }
                Ok::<(), crate::common::GrafanaCliError>(())
            })
            .collect::<Vec<_>>()
    });
    if let Some(bar) = progress_bar.as_ref() {
        bar.finish_and_clear();
    }
    for result in results {
        result?;
    }
    Ok(summaries.len())
}

pub(crate) fn inspect_live_dashboards_with_client(
    client: &JsonHttpClient,
    args: &InspectLiveArgs,
) -> Result<usize> {
    if args.all_orgs || args.concurrency <= 1 {
        return inspect_live_dashboards_with_request(
            |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
        );
    }

    let temp_dir = TempInspectDir::new("inspect-live")?;
    let raw_dir = temp_dir.path.join(RAW_EXPORT_SUBDIR);
    let summaries = list_dashboard_summaries(client, args.page_size)?;
    let datasource_items = list_datasources(client)?;
    let org_value = client
        .request_json(Method::GET, "/api/org", &[], None)?
        .unwrap_or_else(|| serde_json::json!({"id": 1, "name": "Main Org."}));
    let org = value_as_object(&org_value, "Unexpected current org payload from Grafana.")?;
    let datasource_inventory = datasource_items
        .iter()
        .map(|item| build_datasource_inventory_record(item, org))
        .collect::<Vec<_>>();
    snapshot_live_dashboard_export_with_fetcher(
        &raw_dir,
        &summaries,
        args.concurrency,
        args.progress,
        |uid| fetch_dashboard(client, uid),
    )?;
    write_json_document(
        &datasource_inventory,
        &raw_dir.join(DATASOURCE_INVENTORY_FILENAME),
    )?;
    let folder_inventory = collect_folder_inventory_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        &summaries,
    )?;
    write_json_document(&folder_inventory, &raw_dir.join(FOLDER_INVENTORY_FILENAME))?;
    if args.interactive {
        return run_interactive_inspect_live_tui_from_dir(&raw_dir);
    }
    let inspect_args = build_export_inspect_args_from_live(args, raw_dir);
    analyze_export_dir(&inspect_args)
}

fn build_export_inspect_args_from_live(
    args: &InspectLiveArgs,
    import_dir: PathBuf,
) -> InspectExportArgs {
    InspectExportArgs {
        import_dir,
        json: args.json,
        table: args.table,
        report: args.report,
        output_format: args.output_format,
        output_file: args.output_file.clone(),
        report_columns: args.report_columns.clone(),
        report_filter_datasource: args.report_filter_datasource.clone(),
        report_filter_panel_id: args.report_filter_panel_id.clone(),
        help_full: args.help_full,
        no_header: args.no_header,
    }
}

fn load_json_array_file(path: &Path, error_context: &str) -> Result<Vec<Value>> {
    let raw = fs::read_to_string(path).map_err(|error| {
        message(format!(
            "Failed to read {error_context} {}: {error}",
            path.display()
        ))
    })?;
    let value: Value = serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Invalid JSON in {error_context} {}: {error}",
            path.display()
        ))
    })?;
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(message(format!(
            "{error_context} must be a JSON array: {}",
            path.display()
        ))),
    }
}

fn discover_org_raw_export_dirs(import_dir: &Path) -> Result<Vec<(String, PathBuf)>> {
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
    let mut org_raw_dirs = Vec::new();
    for entry in fs::read_dir(import_dir)? {
        let entry = entry?;
        let org_root = entry.path();
        if !org_root.is_dir() {
            continue;
        }
        let org_name = entry.file_name().to_string_lossy().to_string();
        if !org_name.starts_with("org_") {
            continue;
        }
        let org_raw_dir = org_root.join(RAW_EXPORT_SUBDIR);
        if org_raw_dir.is_dir() {
            org_raw_dirs.push((org_name, org_raw_dir));
        }
    }
    org_raw_dirs.sort_by(|left, right| left.0.cmp(&right.0));
    if org_raw_dirs.is_empty() {
        return Err(message(format!(
            "Import path {} does not contain any org-scoped {}/ exports. Point --import-dir at a combined multi-org export root created with --all-orgs.",
            import_dir.display(),
            RAW_EXPORT_SUBDIR
        )));
    }
    Ok(org_raw_dirs)
}

fn merge_org_raw_exports_into_dir(
    org_raw_dirs: &[(String, PathBuf)],
    inspect_raw_dir: &Path,
    source_root: Option<&Path>,
) -> Result<()> {
    fs::create_dir_all(inspect_raw_dir)?;

    let mut folder_inventory = Vec::new();
    let mut datasource_inventory = Vec::new();
    let mut index_entries = Vec::new();
    let mut dashboard_count = 0usize;

    for (org_name, org_raw_dir) in org_raw_dirs {
        let relative_target = inspect_raw_dir.join(org_name);
        copy_dir_recursive(org_raw_dir, &relative_target)?;

        let metadata = load_export_metadata(org_raw_dir, Some(RAW_EXPORT_SUBDIR))?;
        let org_index_entries = load_variant_index_entries(org_raw_dir, metadata.as_ref())?;
        for mut entry in org_index_entries.clone() {
            entry.path = format!("{org_name}/{}", entry.path);
            index_entries.push(entry);
        }

        let folder_path = org_raw_dir.join(FOLDER_INVENTORY_FILENAME);
        if folder_path.is_file() {
            folder_inventory.extend(load_json_array_file(
                &folder_path,
                "Dashboard folder inventory",
            )?);
        }
        let datasource_path = org_raw_dir.join(DATASOURCE_INVENTORY_FILENAME);
        if datasource_path.is_file() {
            datasource_inventory.extend(load_json_array_file(
                &datasource_path,
                "Dashboard datasource inventory",
            )?);
        }

        dashboard_count += metadata
            .as_ref()
            .map(|item| item.dashboard_count as usize)
            .unwrap_or(org_index_entries.len());
    }

    write_json_document(
        &build_export_metadata(
            RAW_EXPORT_SUBDIR,
            dashboard_count,
            Some("grafana-web-import-preserve-uid"),
            Some(FOLDER_INVENTORY_FILENAME),
            Some(DATASOURCE_INVENTORY_FILENAME),
            None,
            None,
            None,
            None,
        ),
        &inspect_raw_dir.join(EXPORT_METADATA_FILENAME),
    )?;
    write_json_document(&index_entries, &inspect_raw_dir.join("index.json"))?;
    write_json_document(
        &folder_inventory,
        &inspect_raw_dir.join(FOLDER_INVENTORY_FILENAME),
    )?;
    write_json_document(
        &datasource_inventory,
        &inspect_raw_dir.join(DATASOURCE_INVENTORY_FILENAME),
    )?;
    if let Some(source_root) = source_root {
        fs::write(
            inspect_raw_dir.join(INSPECT_SOURCE_ROOT_FILENAME),
            source_root.display().to_string(),
        )?;
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path).map_err(|error| {
                message(format!(
                    "Failed to copy {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                ))
            })?;
        }
    }
    Ok(())
}

fn prepare_inspect_live_import_dir(temp_root: &Path, args: &InspectLiveArgs) -> Result<PathBuf> {
    if !args.all_orgs {
        return Ok(temp_root.join(RAW_EXPORT_SUBDIR));
    }

    let inspect_raw_dir = temp_root
        .join("inspect-live-all-orgs")
        .join(RAW_EXPORT_SUBDIR);
    let org_raw_dirs = discover_org_raw_export_dirs(temp_root)?;
    merge_org_raw_exports_into_dir(&org_raw_dirs, &inspect_raw_dir, None)?;
    Ok(inspect_raw_dir)
}

pub(crate) fn prepare_inspect_export_import_dir(
    temp_root: &Path,
    import_dir: &Path,
) -> Result<PathBuf> {
    let metadata = load_export_metadata(import_dir, None)?;
    if metadata
        .as_ref()
        .map(|item| item.variant.as_str() == "root")
        .unwrap_or(false)
    {
        let inspect_raw_dir = temp_root
            .join("inspect-export-all-orgs")
            .join(RAW_EXPORT_SUBDIR);
        let org_raw_dirs = discover_org_raw_export_dirs(import_dir)?;
        merge_org_raw_exports_into_dir(&org_raw_dirs, &inspect_raw_dir, Some(import_dir))?;
        return Ok(inspect_raw_dir);
    }
    Ok(import_dir.to_path_buf())
}

/// inspect live dashboards with request.
pub(crate) fn inspect_live_dashboards_with_request<F>(
    mut request_json: F,
    args: &InspectLiveArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let temp_dir = TempInspectDir::new("inspect-live")?;
    let export_args = build_live_export_args(args, temp_dir.path.clone());
    let _ = export::export_dashboards_with_request(&mut request_json, &export_args)?;
    let inspect_import_dir = prepare_inspect_live_import_dir(&temp_dir.path, args)?;
    if args.interactive {
        return run_interactive_inspect_live_tui_from_dir(&inspect_import_dir);
    }
    let inspect_args = build_export_inspect_args_from_live(args, inspect_import_dir);
    analyze_export_dir(&inspect_args)
}
