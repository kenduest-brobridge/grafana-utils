//! Datasource import bundle loading and export-org routing helpers.
//!
//! Maintainer notes:
//! - This module is the contract gate for on-disk datasource bundles; reject
//!   mixed-schema or ambiguous org metadata here before import logic runs.
//! - `--use-export-org` routing prefers explicit metadata from exported files and
//!   falls back to `org_<id>_<name>` directory names only when the bundle lacks
//!   stable org fields.

use reqwest::Method;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, string_field, Result};
use crate::dashboard::DEFAULT_ORG_ID;
use crate::http::JsonHttpClient;

use super::datasource_export_support::{
    parse_export_metadata, validate_datasource_contract_record, DATASOURCE_PROVISIONING_FILENAME,
    DATASOURCE_PROVISIONING_SUBDIR,
};
use super::{DatasourceImportArgs, DatasourceImportInputFormat};

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
    "secureJsonDataPlaceholders",
];

#[derive(Debug, Clone)]
pub(crate) struct DatasourceExportMetadata {
    pub(crate) schema_version: i64,
    pub(crate) kind: String,
    pub(crate) variant: String,
    pub(crate) resource: String,
    pub(crate) datasources_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceImportRecord {
    pub uid: String,
    pub name: String,
    pub datasource_type: String,
    pub access: String,
    pub url: String,
    pub is_default: bool,
    pub org_name: String,
    pub org_id: String,
    pub secure_json_data_placeholders: Option<Map<String, Value>>,
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
    pub(crate) input_format: DatasourceImportInputFormat,
    pub(crate) source_org_id: String,
    pub(crate) target_org_id: String,
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) datasource_count: usize,
    pub(crate) would_create: usize,
    pub(crate) would_update: usize,
    pub(crate) would_skip: usize,
    pub(crate) would_block: usize,
}

#[derive(Debug, Deserialize)]
struct ProvisioningImportDocument {
    #[serde(rename = "apiVersion")]
    _api_version: Option<i64>,
    #[serde(default)]
    datasources: Vec<ProvisioningImportDatasource>,
}

#[derive(Debug, Deserialize)]
struct ProvisioningImportDatasource {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    datasource_type: String,
    #[serde(default)]
    access: String,
    #[serde(default)]
    url: String,
    #[serde(default, rename = "isDefault")]
    is_default: bool,
    #[serde(default, rename = "orgId")]
    org_id: Option<i64>,
    #[serde(default, rename = "secureJsonDataPlaceholders")]
    secure_json_data_placeholders: Option<Map<String, Value>>,
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

fn load_inventory_import_records(
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
    // Treat export-metadata.json as the root contract. Import should fail here
    // rather than guessing at newer or unrelated bundle layouts.
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
            org_name: string_field(object, "org", ""),
            org_id: string_field(object, "orgId", ""),
            secure_json_data_placeholders: object
                .get("secureJsonDataPlaceholders")
                .and_then(Value::as_object)
                .cloned(),
        });
    }
    Ok((metadata, records))
}

fn relative_import_source_label(import_path: &Path, resolved_path: &Path) -> String {
    if import_path.is_file() {
        return import_path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| resolved_path.to_string_lossy().into_owned());
    }
    resolved_path
        .strip_prefix(import_path)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| {
            resolved_path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.to_string())
                .unwrap_or_else(|| resolved_path.to_string_lossy().into_owned())
        })
}

fn resolve_provisioning_import_source_path(import_path: &Path) -> Result<PathBuf> {
    if !import_path.exists() {
        return Err(message(format!(
            "Datasource provisioning import path does not exist: {}",
            import_path.display()
        )));
    }
    if import_path.is_file() {
        let extension = import_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if matches!(extension, "yaml" | "yml") {
            return Ok(import_path.to_path_buf());
        }
        return Err(message(format!(
            "Datasource provisioning import file must be YAML (.yaml or .yml): {}",
            import_path.display()
        )));
    }
    let candidates = [
        import_path.join(DATASOURCE_PROVISIONING_FILENAME),
        import_path.join("datasources.yml"),
        import_path
            .join(DATASOURCE_PROVISIONING_SUBDIR)
            .join(DATASOURCE_PROVISIONING_FILENAME),
        import_path
            .join(DATASOURCE_PROVISIONING_SUBDIR)
            .join("datasources.yml"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(message(format!(
        "Datasource provisioning import did not find datasources.yaml under {}. Point --import-dir at the export root, provisioning directory, or concrete YAML file.",
        import_path.display()
    )))
}

fn load_provisioning_import_records(
    import_path: &Path,
) -> Result<(DatasourceExportMetadata, Vec<DatasourceImportRecord>)> {
    let provisioning_path = resolve_provisioning_import_source_path(import_path)?;
    let raw = fs::read_to_string(&provisioning_path)?;
    let document: ProvisioningImportDocument = serde_yaml::from_str(&raw).map_err(|error| {
        message(format!(
            "Failed to parse datasource provisioning YAML {}: {error}",
            provisioning_path.display()
        ))
    })?;
    let records = document
        .datasources
        .into_iter()
        .map(|datasource| DatasourceImportRecord {
            uid: datasource.uid,
            name: datasource.name,
            datasource_type: datasource.datasource_type,
            access: datasource.access,
            url: datasource.url,
            is_default: datasource.is_default,
            org_name: String::new(),
            org_id: datasource
                .org_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            secure_json_data_placeholders: datasource.secure_json_data_placeholders,
        })
        .collect::<Vec<DatasourceImportRecord>>();
    Ok((
        DatasourceExportMetadata {
            schema_version: TOOL_SCHEMA_VERSION,
            kind: ROOT_INDEX_KIND.to_string(),
            variant: "provisioning".to_string(),
            resource: "datasource".to_string(),
            datasources_file: relative_import_source_label(import_path, &provisioning_path),
        },
        records,
    ))
}

pub(crate) fn load_import_records(
    import_path: &Path,
    input_format: DatasourceImportInputFormat,
) -> Result<(DatasourceExportMetadata, Vec<DatasourceImportRecord>)> {
    match input_format {
        DatasourceImportInputFormat::Inventory => load_inventory_import_records(import_path),
        DatasourceImportInputFormat::Provisioning => load_provisioning_import_records(import_path),
    }
}

fn datasource_import_record_to_diff_value(record: &DatasourceImportRecord) -> Value {
    let mut object = Map::from_iter(vec![
        ("uid".to_string(), Value::String(record.uid.clone())),
        ("name".to_string(), Value::String(record.name.clone())),
        (
            "type".to_string(),
            Value::String(record.datasource_type.clone()),
        ),
        ("access".to_string(), Value::String(record.access.clone())),
        ("url".to_string(), Value::String(record.url.clone())),
        ("isDefault".to_string(), Value::Bool(record.is_default)),
        ("org".to_string(), Value::String(record.org_name.clone())),
        ("orgId".to_string(), Value::String(record.org_id.clone())),
    ]);
    if let Some(placeholders) = &record.secure_json_data_placeholders {
        object.insert(
            "secureJsonDataPlaceholders".to_string(),
            Value::Object(placeholders.clone()),
        );
    }
    Value::Object(object)
}

pub(crate) fn load_diff_record_values(
    diff_dir: &Path,
    input_format: DatasourceImportInputFormat,
) -> Result<Vec<Value>> {
    match input_format {
        DatasourceImportInputFormat::Inventory => {
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
        DatasourceImportInputFormat::Provisioning => {
            let (_, records) = load_import_records(diff_dir, input_format)?;
            Ok(records
                .iter()
                .map(datasource_import_record_to_diff_value)
                .collect())
        }
    }
}

fn collect_source_org_ids(records: &[DatasourceImportRecord]) -> BTreeSet<String> {
    records
        .iter()
        .filter(|record| !record.org_id.is_empty())
        .map(|record| record.org_id.clone())
        .collect()
}

fn collect_source_org_names(records: &[DatasourceImportRecord]) -> BTreeSet<String> {
    records
        .iter()
        .filter(|record| !record.org_name.is_empty())
        .map(|record| record.org_name.clone())
        .collect()
}

fn parse_export_org_scope(
    scope_dir: &Path,
    input_format: DatasourceImportInputFormat,
) -> Result<DatasourceExportOrgScope> {
    let (_, records) = load_import_records(scope_dir, input_format)?;
    let export_org_ids = collect_source_org_ids(&records);
    let (source_org_id, source_org_name_from_dir) = if export_org_ids.is_empty() {
        // Older or minimized exports may omit org metadata inside the payloads.
        // In that case the directory name is the last fallback for routed import.
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
    let org_names = collect_source_org_names(&records);
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
                scope_dir
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
        let is_scope_dir = match args.input_format {
            DatasourceImportInputFormat::Inventory => path.join(EXPORT_METADATA_FILENAME).is_file(),
            DatasourceImportInputFormat::Provisioning => {
                resolve_provisioning_import_source_path(&path).is_ok()
            }
        };
        if !is_scope_dir {
            continue;
        }
        // Each child scope must be self-contained; do not infer org routing from
        // partial directories or siblings without an importable datasource bundle.
        let scope = parse_export_org_scope(&path, args.input_format)?;
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
        match args.input_format {
            DatasourceImportInputFormat::Inventory => {
                if args.import_dir.join(EXPORT_METADATA_FILENAME).is_file() {
                    return Err(message(
                        "Datasource import with --use-export-org expects the combined export root, not one org export directory.",
                    ));
                }
            }
            DatasourceImportInputFormat::Provisioning => {
                if resolve_provisioning_import_source_path(&args.import_dir).is_ok() {
                    return Err(message(
                        "Datasource import with --use-export-org expects the combined export root, not one org provisioning directory or YAML file.",
                    ));
                }
            }
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
    records: &[DatasourceImportRecord],
) -> Result<()> {
    if !args.require_matching_export_org {
        return Ok(());
    }
    // This guardrail is intentionally strict: one import bundle must map to one
    // target org, otherwise a mismatched client/org selection can mutate the
    // wrong Grafana org with valid-looking datasource records.
    let source_org_ids = collect_source_org_ids(records);
    if source_org_ids.is_empty() {
        return Err(message(
            "Cannot verify datasource export org: no stable orgId metadata found in the selected datasource import input.",
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
