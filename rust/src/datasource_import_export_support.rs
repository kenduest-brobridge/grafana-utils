use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, string_field, Result};
use crate::dashboard::DEFAULT_ORG_ID;
use crate::http::JsonHttpClient;

use super::datasource_export_support::{
    parse_export_metadata, validate_datasource_contract_record,
};
use super::DatasourceImportArgs;

pub(crate) const DATASOURCE_EXPORT_FILENAME: &str = "datasources.json";
pub(crate) const EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
pub(crate) const ROOT_INDEX_KIND: &str = "grafana-utils-datasource-export-index";
pub(crate) const TOOL_SCHEMA_VERSION: i64 = 1;
pub(crate) const DATASOURCE_CONTRACT_FIELDS: &[&str] = &[
    "uid",
    "name",
    "type",
    "access",
    "url",
    "isDefault",
    "org",
    "orgId",
];

#[derive(Debug, Clone)]
pub(crate) struct DatasourceExportMetadata {
    pub(crate) schema_version: i64,
    pub(crate) kind: String,
    pub(crate) variant: String,
    pub(crate) resource: String,
    pub(crate) datasources_file: String,
    pub(crate) index_file: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DatasourceImportRecord {
    pub uid: String,
    pub name: String,
    pub datasource_type: String,
    pub access: String,
    pub url: String,
    pub is_default: bool,
    pub org_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DatasourceExportOrgScope {
    pub(crate) source_org_id: i64,
    pub(crate) source_org_name: String,
    pub(crate) import_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct DatasourceExportOrgTargetPlan {
    pub(crate) source_org_id: i64,
    pub(crate) source_org_name: String,
    pub(crate) target_org_id: Option<i64>,
    pub(crate) org_action: &'static str,
    pub(crate) import_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceImportDryRunReport {
    pub(crate) mode: String,
    pub(crate) import_dir: PathBuf,
    pub(crate) source_org_id: String,
    pub(crate) target_org_id: String,
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) datasource_count: usize,
    pub(crate) would_create: usize,
    pub(crate) would_update: usize,
    pub(crate) would_skip: usize,
    pub(crate) would_block: usize,
}

pub(crate) fn fetch_current_org(client: &JsonHttpClient) -> Result<Map<String, Value>> {
    match client.request_json(Method::GET, "/api/org", &[], None)? {
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or_else(|| message("Unexpected current-org payload from Grafana.")),
        None => Err(message("Grafana did not return current-org metadata.")),
    }
}

pub(crate) fn list_orgs(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    match client.request_json(Method::GET, "/api/orgs", &[], None)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| {
                item.as_object()
                    .cloned()
                    .ok_or_else(|| message("Unexpected org entry in /api/orgs response."))
            })
            .collect(),
        Some(_) => Err(message("Unexpected /api/orgs payload from Grafana.")),
        None => Ok(Vec::new()),
    }
}

pub(crate) fn create_org(client: &JsonHttpClient, org_name: &str) -> Result<Map<String, Value>> {
    let payload = Value::Object(Map::from_iter(vec![(
        "name".to_string(),
        Value::String(org_name.to_string()),
    )]));
    match client.request_json(Method::POST, "/api/orgs", &[], Some(&payload))? {
        Some(Value::Object(object)) => Ok(object),
        Some(_) => Err(message("Unexpected create-org payload from Grafana.")),
        None => Err(message("Grafana did not return create-org metadata.")),
    }
}

pub(crate) fn org_id_string_from_value(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn load_import_records(
    import_dir: &Path,
) -> Result<(DatasourceExportMetadata, Vec<DatasourceImportRecord>)> {
    let metadata_path = import_dir.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Err(message(format!(
            "Datasource import directory is missing {}: {}",
            EXPORT_METADATA_FILENAME,
            metadata_path.display()
        )));
    }
    let metadata = parse_export_metadata(&metadata_path)?;
    if metadata.kind != ROOT_INDEX_KIND {
        return Err(message(format!(
            "Unexpected datasource export manifest kind in {}: {:?}",
            metadata_path.display(),
            metadata.kind
        )));
    }
    if metadata.schema_version != TOOL_SCHEMA_VERSION {
        return Err(message(format!(
            "Unsupported datasource export schemaVersion {:?} in {}. Expected {}.",
            metadata.schema_version,
            metadata_path.display(),
            TOOL_SCHEMA_VERSION
        )));
    }
    if metadata.variant != "root" || metadata.resource != "datasource" {
        return Err(message(format!(
            "Datasource export manifest {} is not a datasource export root.",
            metadata_path.display()
        )));
    }
    let datasources_path = import_dir.join(&metadata.datasources_file);
    let raw = fs::read_to_string(&datasources_path)?;
    let value: Value = serde_json::from_str(&raw)?;
    let items = value.as_array().ok_or_else(|| {
        message(format!(
            "Datasource inventory file must contain a JSON array: {}",
            datasources_path.display()
        ))
    })?;
    let mut records = Vec::new();
    for item in items {
        let object = item.as_object().ok_or_else(|| {
            message(format!(
                "Datasource inventory entry must be a JSON object: {}",
                datasources_path.display()
            ))
        })?;
        validate_datasource_contract_record(
            object,
            &format!("Datasource import entry in {}", datasources_path.display()),
        )?;
        records.push(DatasourceImportRecord {
            uid: string_field(object, "uid", ""),
            name: string_field(object, "name", ""),
            datasource_type: string_field(object, "type", ""),
            access: string_field(object, "access", ""),
            url: string_field(object, "url", ""),
            is_default: string_field(object, "isDefault", "false") == "true",
            org_id: string_field(object, "orgId", ""),
        });
    }
    Ok((metadata, records))
}

pub(crate) fn load_diff_record_values(diff_dir: &Path) -> Result<Vec<Value>> {
    let metadata_path = diff_dir.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Err(message(format!(
            "Datasource diff directory is missing {}: {}",
            EXPORT_METADATA_FILENAME,
            metadata_path.display()
        )));
    }
    let metadata = parse_export_metadata(&metadata_path)?;
    if metadata.kind != ROOT_INDEX_KIND {
        return Err(message(format!(
            "Unexpected datasource export manifest kind in {}: {:?}",
            metadata_path.display(),
            metadata.kind
        )));
    }
    if metadata.schema_version != TOOL_SCHEMA_VERSION {
        return Err(message(format!(
            "Unsupported datasource export schemaVersion {:?} in {}. Expected {}.",
            metadata.schema_version,
            metadata_path.display(),
            TOOL_SCHEMA_VERSION
        )));
    }
    if metadata.variant != "root" || metadata.resource != "datasource" {
        return Err(message(format!(
            "Datasource export manifest {} is not a datasource export root.",
            metadata_path.display()
        )));
    }
    let datasources_path = diff_dir.join(&metadata.datasources_file);
    let raw = fs::read_to_string(&datasources_path)?;
    let value: Value = serde_json::from_str(&raw)?;
    let items = value.as_array().ok_or_else(|| {
        message(format!(
            "Datasource inventory file must contain a JSON array: {}",
            datasources_path.display()
        ))
    })?;
    for item in items {
        let object = item.as_object().ok_or_else(|| {
            message(format!(
                "Datasource inventory entry must be a JSON object: {}",
                datasources_path.display()
            ))
        })?;
        validate_datasource_contract_record(
            object,
            &format!("Datasource diff entry in {}", datasources_path.display()),
        )?;
    }
    Ok(items.clone())
}

fn collect_source_org_ids(
    import_dir: &Path,
    metadata: &DatasourceExportMetadata,
) -> Result<BTreeSet<String>> {
    let mut org_ids = BTreeSet::new();
    let datasources_path = import_dir.join(&metadata.datasources_file);
    if datasources_path.is_file() {
        let raw = fs::read_to_string(&datasources_path)?;
        let value: Value = serde_json::from_str(&raw)?;
        if let Some(items) = value.as_array() {
            for item in items {
                if let Some(object) = item.as_object() {
                    let org_id = string_field(object, "orgId", "");
                    if !org_id.is_empty() {
                        org_ids.insert(org_id);
                    }
                }
            }
        }
    }
    let index_path = import_dir.join(&metadata.index_file);
    if index_path.is_file() {
        let raw = fs::read_to_string(&index_path)?;
        let value: Value = serde_json::from_str(&raw)?;
        if let Some(items) = value.get("items").and_then(Value::as_array) {
            for item in items {
                if let Some(object) = item.as_object() {
                    let org_id = string_field(object, "orgId", "");
                    if !org_id.is_empty() {
                        org_ids.insert(org_id);
                    }
                }
            }
        }
    }
    Ok(org_ids)
}

fn collect_source_org_names(
    import_dir: &Path,
    metadata: &DatasourceExportMetadata,
) -> Result<BTreeSet<String>> {
    let mut org_names = BTreeSet::new();
    let datasources_path = import_dir.join(&metadata.datasources_file);
    if datasources_path.is_file() {
        let raw = fs::read_to_string(&datasources_path)?;
        let value: Value = serde_json::from_str(&raw)?;
        if let Some(items) = value.as_array() {
            for item in items {
                if let Some(object) = item.as_object() {
                    let org_name = string_field(object, "org", "");
                    if !org_name.is_empty() {
                        org_names.insert(org_name);
                    }
                }
            }
        }
    }
    let index_path = import_dir.join(&metadata.index_file);
    if index_path.is_file() {
        let raw = fs::read_to_string(&index_path)?;
        let value: Value = serde_json::from_str(&raw)?;
        if let Some(items) = value.get("items").and_then(Value::as_array) {
            for item in items {
                if let Some(object) = item.as_object() {
                    let org_name = string_field(object, "org", "");
                    if !org_name.is_empty() {
                        org_names.insert(org_name);
                    }
                }
            }
        }
    }
    Ok(org_names)
}

fn parse_export_org_scope(
    import_root: &Path,
    scope_dir: &Path,
) -> Result<DatasourceExportOrgScope> {
    let metadata = parse_export_metadata(&scope_dir.join(EXPORT_METADATA_FILENAME))?;
    let export_org_ids = collect_source_org_ids(scope_dir, &metadata)?;
    let (source_org_id, source_org_name_from_dir) = if export_org_ids.is_empty() {
        let scope_name = scope_dir
            .file_name()
            .and_then(|item| item.to_str())
            .unwrap_or_default();
        if let Some(rest) = scope_name.strip_prefix("org_") {
            let mut parts = rest.splitn(2, '_');
            let source_org_id_text = parts.next().unwrap_or_default();
            let source_org_name = parts
                .next()
                .unwrap_or_default()
                .replace('_', " ")
                .trim()
                .to_string();
            let source_org_id = source_org_id_text.parse::<i64>().map_err(|_| {
                message(format!(
                    "Cannot route datasource import by export org for {}: export orgId '{}' from the org directory name is not a valid integer.",
                    scope_dir.display(),
                    source_org_id_text
                ))
            })?;
            (source_org_id, source_org_name)
        } else {
            return Err(message(format!(
                "Cannot route datasource import by export org for {}: export orgId metadata was not found in datasources.json or index.json.",
                scope_dir.display()
            )));
        }
    } else {
        if export_org_ids.len() > 1 {
            return Err(message(format!(
                "Cannot route datasource import by export org for {}: found multiple export orgIds ({}).",
                scope_dir.display(),
                export_org_ids.into_iter().collect::<Vec<String>>().join(", ")
            )));
        }
        let source_org_id_text = export_org_ids.into_iter().next().unwrap_or_default();
        let source_org_id = source_org_id_text.parse::<i64>().map_err(|_| {
            message(format!(
                "Cannot route datasource import by export org for {}: export orgId '{}' is not a valid integer.",
                scope_dir.display(),
                source_org_id_text
            ))
        })?;
        (source_org_id, String::new())
    };
    let org_names = collect_source_org_names(scope_dir, &metadata)?;
    if org_names.len() > 1 {
        return Err(message(format!(
            "Cannot route datasource import by export org for {}: found multiple export org names ({}).",
            scope_dir.display(),
            org_names.into_iter().collect::<Vec<String>>().join(", ")
        )));
    }
    let source_org_name = org_names
        .into_iter()
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            if !source_org_name_from_dir.is_empty() {
                source_org_name_from_dir
            } else {
                import_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("org")
                    .to_string()
            }
        });
    Ok(DatasourceExportOrgScope {
        source_org_id,
        source_org_name,
        import_dir: scope_dir.to_path_buf(),
    })
}

pub(crate) fn discover_export_org_import_scopes(
    args: &DatasourceImportArgs,
) -> Result<Vec<DatasourceExportOrgScope>> {
    if !args.use_export_org {
        return Ok(Vec::new());
    }
    let selected_org_ids: BTreeSet<i64> = args.only_org_id.iter().copied().collect();
    let mut scopes = Vec::new();
    let mut matched_source_org_ids = BTreeSet::new();
    for entry in fs::read_dir(&args.import_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|item| item.to_str()) else {
            continue;
        };
        if !name.starts_with("org_") {
            continue;
        }
        if !path.join(EXPORT_METADATA_FILENAME).is_file() {
            continue;
        }
        let scope = parse_export_org_scope(&path, &path)?;
        if !selected_org_ids.is_empty() && !selected_org_ids.contains(&scope.source_org_id) {
            continue;
        }
        matched_source_org_ids.insert(scope.source_org_id);
        scopes.push(scope);
    }
    scopes.sort_by(|left, right| left.source_org_id.cmp(&right.source_org_id));
    if !selected_org_ids.is_empty() {
        let missing: Vec<String> = selected_org_ids
            .difference(&matched_source_org_ids)
            .map(|item| item.to_string())
            .collect();
        if !missing.is_empty() {
            return Err(message(format!(
                "Selected exported org IDs were not found in {}: {}",
                args.import_dir.display(),
                missing.join(", ")
            )));
        }
    }
    if scopes.is_empty() {
        if args.import_dir.join(EXPORT_METADATA_FILENAME).is_file() {
            return Err(message(
                "Datasource import with --use-export-org expects the combined export root, not one org export directory.",
            ));
        }
        if !selected_org_ids.is_empty() {
            return Err(message(format!(
                "Datasource import with --use-export-org did not find the selected exported org IDs ({}) under {}.",
                selected_org_ids
                    .into_iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                args.import_dir.display()
            )));
        }
        return Err(message(format!(
            "Datasource import with --use-export-org did not find any org-scoped datasource exports under {}.",
            args.import_dir.display()
        )));
    }
    let found_org_ids: BTreeSet<i64> = scopes.iter().map(|scope| scope.source_org_id).collect();
    let missing_org_ids: Vec<String> = selected_org_ids
        .difference(&found_org_ids)
        .map(|id| id.to_string())
        .collect();
    if !missing_org_ids.is_empty() {
        return Err(message(format!(
            "Datasource import with --use-export-org did not find the selected exported org IDs ({}).",
            missing_org_ids.join(", ")
        )));
    }
    Ok(scopes)
}

pub(crate) fn validate_matching_export_org(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
    import_dir: &Path,
    metadata: &DatasourceExportMetadata,
) -> Result<()> {
    if !args.require_matching_export_org {
        return Ok(());
    }
    let source_org_ids = collect_source_org_ids(import_dir, metadata)?;
    if source_org_ids.is_empty() {
        return Err(message(
            "Cannot verify datasource export org: no stable orgId metadata found in datasources.json or index.json.",
        ));
    }
    if source_org_ids.len() > 1 {
        return Err(message(format!(
            "Cannot verify datasource export org: found multiple export orgIds ({}).",
            source_org_ids
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        )));
    }
    let source_org_id = source_org_ids.into_iter().next().unwrap_or_default();
    let target_org = fetch_current_org(client)?;
    let target_org_id = target_org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    if source_org_id != target_org_id {
        return Err(message(format!(
            "Datasource import export org mismatch: raw export orgId {source_org_id} does not match target org {target_org_id}. Use matching credentials/org selection or omit --require-matching-export-org."
        )));
    }
    Ok(())
}
