//! Datasource import bundle loading and export-org routing helpers.
//!
//! Maintainer notes:
//! - This module is the contract gate for on-disk datasource bundles; reject
//!   mixed-schema or ambiguous org metadata here before import logic runs.
//! - `--use-export-org` routing prefers explicit metadata from exported files and
//!   falls back to `org_<id>_<name>` directory names only when the bundle lacks
//!   stable org fields.

use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, string_field, Result};
use crate::dashboard::DEFAULT_ORG_ID;
use crate::grafana_api::DatasourceResourceClient;
use crate::http::JsonHttpClient;

use super::datasource_export_support::{
    parse_export_metadata, DATASOURCE_PROVISIONING_FILENAME, DATASOURCE_PROVISIONING_SUBDIR,
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
const DATASOURCE_RECOVERY_IMPORT_EXTRA_FIELDS: &[&str] = &[
    "basicAuth",
    "basicAuthUser",
    "database",
    "jsonData",
    "secureJsonDataPlaceholders",
    "user",
    "withCredentials",
];

#[derive(Debug, Clone)]
pub(crate) struct DatasourceExportMetadata {
    pub(crate) schema_version: i64,
    pub(crate) kind: String,
    pub(crate) variant: String,
    pub(crate) scope_kind: Option<String>,
    pub(crate) resource: String,
    pub(crate) datasources_file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatasourceExportRootScopeKind {
    OrgRoot,
    AllOrgsRoot,
    WorkspaceRoot,
    Unknown,
}

impl DatasourceExportRootScopeKind {
    pub(crate) fn is_aggregate(self) -> bool {
        matches!(self, Self::AllOrgsRoot | Self::WorkspaceRoot)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DatasourceExportRootManifest {
    pub(crate) metadata: DatasourceExportMetadata,
    pub(crate) scope_kind: DatasourceExportRootScopeKind,
}

pub(crate) fn classify_datasource_export_root_scope_kind(
    metadata: &DatasourceExportMetadata,
) -> DatasourceExportRootScopeKind {
    match metadata.scope_kind.as_deref() {
        Some("org-root") => DatasourceExportRootScopeKind::OrgRoot,
        Some("all-orgs-root") => DatasourceExportRootScopeKind::AllOrgsRoot,
        Some("workspace-root") => DatasourceExportRootScopeKind::WorkspaceRoot,
        Some(_) => DatasourceExportRootScopeKind::Unknown,
        None if metadata.variant == "all-orgs-root" => DatasourceExportRootScopeKind::AllOrgsRoot,
        None if metadata.variant == "root" => DatasourceExportRootScopeKind::OrgRoot,
        None => DatasourceExportRootScopeKind::Unknown,
    }
}

pub(crate) fn load_datasource_export_root_manifest(
    metadata_path: &Path,
) -> Result<DatasourceExportRootManifest> {
    let metadata = parse_export_metadata(metadata_path)?;
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
    if metadata.resource != "datasource" {
        return Err(message(format!(
            "Unexpected datasource export manifest resource in {}: {:?}",
            metadata_path.display(),
            metadata.resource
        )));
    }
    Ok(DatasourceExportRootManifest {
        scope_kind: classify_datasource_export_root_scope_kind(&metadata),
        metadata,
    })
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
    pub basic_auth: Option<bool>,
    pub basic_auth_user: String,
    pub database: String,
    pub json_data: Option<Map<String, Value>>,
    pub secure_json_data_placeholders: Option<Map<String, Value>>,
    pub user: String,
    pub with_credentials: Option<bool>,
}

impl DatasourceImportRecord {
    pub(crate) fn from_inventory_record(
        record: &Map<String, Value>,
        context_label: &str,
    ) -> Result<Self> {
        validate_datasource_import_record(record, context_label)?;
        Self::from_validated_record(record, context_label)
    }

    pub(crate) fn from_generic_map(record: &Map<String, Value>) -> Self {
        Self {
            uid: string_field(record, "uid", ""),
            name: string_field(record, "name", ""),
            datasource_type: string_field(record, "type", ""),
            access: string_field(record, "access", ""),
            url: string_field(record, "url", ""),
            is_default: normalize_generic_bool_value(record.get("isDefault")).unwrap_or(false),
            org_name: string_field(record, "org", ""),
            org_id: org_id_string_from_value(record.get("orgId")),
            basic_auth: normalize_generic_bool_value(record.get("basicAuth")),
            basic_auth_user: normalize_optional_string_field(record, "basicAuthUser"),
            database: normalize_optional_string_field(record, "database"),
            json_data: record.get("jsonData").and_then(Value::as_object).cloned(),
            secure_json_data_placeholders: record
                .get("secureJsonDataPlaceholders")
                .and_then(Value::as_object)
                .cloned(),
            user: normalize_optional_string_field(record, "user"),
            with_credentials: normalize_generic_bool_value(record.get("withCredentials")),
        }
    }

    pub(crate) fn to_inventory_record(&self) -> Map<String, Value> {
        let mut object = Map::from_iter(vec![
            ("uid".to_string(), Value::String(self.uid.clone())),
            ("name".to_string(), Value::String(self.name.clone())),
            (
                "type".to_string(),
                Value::String(self.datasource_type.clone()),
            ),
            ("access".to_string(), Value::String(self.access.clone())),
            ("url".to_string(), Value::String(self.url.clone())),
            (
                "isDefault".to_string(),
                Value::String(self.is_default.to_string()),
            ),
            ("org".to_string(), Value::String(self.org_name.clone())),
            ("orgId".to_string(), Value::String(self.org_id.clone())),
        ]);
        if let Some(value) = self.basic_auth {
            object.insert("basicAuth".to_string(), Value::Bool(value));
        }
        if !self.basic_auth_user.is_empty() {
            object.insert(
                "basicAuthUser".to_string(),
                Value::String(self.basic_auth_user.clone()),
            );
        }
        if !self.database.is_empty() {
            object.insert("database".to_string(), Value::String(self.database.clone()));
        }
        if let Some(json_data) = &self.json_data {
            object.insert("jsonData".to_string(), Value::Object(json_data.clone()));
        }
        if let Some(placeholders) = &self.secure_json_data_placeholders {
            object.insert(
                "secureJsonDataPlaceholders".to_string(),
                Value::Object(placeholders.clone()),
            );
        }
        if !self.user.is_empty() {
            object.insert("user".to_string(), Value::String(self.user.clone()));
        }
        if let Some(value) = self.with_credentials {
            object.insert("withCredentials".to_string(), Value::Bool(value));
        }
        object
    }

    fn from_validated_record(record: &Map<String, Value>, context_label: &str) -> Result<Self> {
        Ok(Self {
            uid: string_field(record, "uid", ""),
            name: string_field(record, "name", ""),
            datasource_type: string_field(record, "type", ""),
            access: string_field(record, "access", ""),
            url: string_field(record, "url", ""),
            is_default: normalize_optional_bool_value(
                record.get("isDefault"),
                "isDefault",
                context_label,
            )?
            .unwrap_or(false),
            org_name: string_field(record, "org", ""),
            org_id: org_id_string_from_value(record.get("orgId")),
            basic_auth: normalize_optional_bool_value(
                record.get("basicAuth"),
                "basicAuth",
                context_label,
            )?,
            basic_auth_user: normalize_optional_string_field(record, "basicAuthUser"),
            database: normalize_optional_string_field(record, "database"),
            json_data: normalize_optional_object_value(
                record.get("jsonData"),
                "jsonData",
                context_label,
            )?,
            secure_json_data_placeholders: normalize_optional_object_value(
                record.get("secureJsonDataPlaceholders"),
                "secureJsonDataPlaceholders",
                context_label,
            )?,
            user: normalize_optional_string_field(record, "user"),
            with_credentials: normalize_optional_bool_value(
                record.get("withCredentials"),
                "withCredentials",
                context_label,
            )?,
        })
    }
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
    #[serde(default, rename = "basicAuth")]
    basic_auth: Option<bool>,
    #[serde(default, rename = "basicAuthUser")]
    basic_auth_user: Option<String>,
    #[serde(default)]
    database: Option<String>,
    #[serde(default, rename = "jsonData")]
    json_data: Option<Map<String, Value>>,
    #[serde(default, rename = "secureJsonDataPlaceholders")]
    secure_json_data_placeholders: Option<Map<String, Value>>,
    #[serde(default)]
    user: Option<String>,
    #[serde(default, rename = "withCredentials")]
    with_credentials: Option<bool>,
}

pub(crate) fn fetch_current_org(client: &JsonHttpClient) -> Result<Map<String, Value>> {
    DatasourceResourceClient::new(client).fetch_current_org()
}

pub(crate) fn list_orgs(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    DatasourceResourceClient::new(client).list_orgs()
}

pub(crate) fn create_org(client: &JsonHttpClient, org_name: &str) -> Result<Map<String, Value>> {
    DatasourceResourceClient::new(client).create_org(org_name)
}

pub(crate) fn org_id_string_from_value(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn normalize_optional_string_field(object: &Map<String, Value>, field_name: &str) -> String {
    match object.get(field_name) {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(boolean)) => boolean.to_string(),
        _ => String::new(),
    }
}

fn normalize_optional_bool_value(
    value: Option<&Value>,
    field_name: &str,
    context_label: &str,
) -> Result<Option<bool>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(boolean)) => Ok(Some(*boolean)),
        Some(Value::String(text)) => {
            let normalized = text.trim().to_ascii_lowercase();
            if matches!(normalized.as_str(), "true" | "1" | "yes" | "on") {
                Ok(Some(true))
            } else if matches!(normalized.as_str(), "false" | "0" | "no" | "off") {
                Ok(Some(false))
            } else {
                Err(message(format!(
                    "{context_label} field {field_name} must be a boolean."
                )))
            }
        }
        Some(Value::Number(number)) if number.as_i64() == Some(1) => Ok(Some(true)),
        Some(Value::Number(number)) if number.as_i64() == Some(0) => Ok(Some(false)),
        Some(_) => Err(message(format!(
            "{context_label} field {field_name} must be a boolean."
        ))),
    }
}

fn normalize_generic_bool_value(value: Option<&Value>) -> Option<bool> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::Bool(boolean)) => Some(*boolean),
        Some(Value::String(text)) => {
            let normalized = text.trim().to_ascii_lowercase();
            if matches!(normalized.as_str(), "true" | "1" | "yes" | "on") {
                Some(true)
            } else if matches!(normalized.as_str(), "false" | "0" | "no" | "off") {
                Some(false)
            } else {
                None
            }
        }
        Some(Value::Number(number)) if number.as_i64() == Some(1) => Some(true),
        Some(Value::Number(number)) if number.as_i64() == Some(0) => Some(false),
        Some(_) => None,
    }
}

fn normalize_optional_object_value(
    value: Option<&Value>,
    field_name: &str,
    context_label: &str,
) -> Result<Option<Map<String, Value>>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(object)) => Ok(Some(object.clone())),
        Some(_) => Err(message(format!(
            "{context_label} field {field_name} must be a JSON object."
        ))),
    }
}

fn validate_datasource_import_record(
    record: &Map<String, Value>,
    context_label: &str,
) -> Result<()> {
    let mut extra_fields = record
        .keys()
        .filter(|key| {
            !DATASOURCE_CONTRACT_FIELDS.contains(&key.as_str())
                && !DATASOURCE_RECOVERY_IMPORT_EXTRA_FIELDS.contains(&key.as_str())
        })
        .cloned()
        .collect::<Vec<String>>();
    extra_fields.sort();
    if !extra_fields.is_empty() {
        let supported_fields = DATASOURCE_CONTRACT_FIELDS
            .iter()
            .chain(DATASOURCE_RECOVERY_IMPORT_EXTRA_FIELDS.iter())
            .copied()
            .collect::<Vec<&str>>();
        return Err(message(format!(
            "{context_label} contains unsupported datasource field(s): {}. Supported fields: {}.",
            extra_fields.join(", "),
            supported_fields.join(", ")
        )));
    }
    normalize_optional_bool_value(record.get("basicAuth"), "basicAuth", context_label)?;
    normalize_optional_bool_value(
        record.get("withCredentials"),
        "withCredentials",
        context_label,
    )?;
    normalize_optional_object_value(record.get("jsonData"), "jsonData", context_label)?;
    normalize_optional_object_value(
        record.get("secureJsonDataPlaceholders"),
        "secureJsonDataPlaceholders",
        context_label,
    )?;
    Ok(())
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
    let root_manifest = load_datasource_export_root_manifest(&metadata_path)?;
    if root_manifest.scope_kind != DatasourceExportRootScopeKind::OrgRoot {
        return Err(message(format!(
            "Datasource export manifest {} is not a datasource export root.",
            metadata_path.display()
        )));
    }
    let metadata = root_manifest.metadata;
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
        let context_label = format!("Datasource import entry in {}", datasources_path.display());
        records.push(DatasourceImportRecord::from_inventory_record(
            object,
            &context_label,
        )?);
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
            basic_auth: datasource.basic_auth,
            basic_auth_user: datasource.basic_auth_user.unwrap_or_default(),
            database: datasource.database.unwrap_or_default(),
            json_data: datasource.json_data,
            secure_json_data_placeholders: datasource.secure_json_data_placeholders,
            user: datasource.user.unwrap_or_default(),
            with_credentials: datasource.with_credentials,
        })
        .collect::<Vec<DatasourceImportRecord>>();
    Ok((
        DatasourceExportMetadata {
            schema_version: TOOL_SCHEMA_VERSION,
            kind: ROOT_INDEX_KIND.to_string(),
            variant: "provisioning".to_string(),
            scope_kind: None,
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

pub(crate) fn discover_datasource_inventory_scope_dirs(input_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut scope_dirs = Vec::new();
    if !input_dir.is_dir() {
        return Ok(scope_dirs);
    }
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|item| item.to_str()) else {
            continue;
        };
        if name.starts_with("org_") && path.join(EXPORT_METADATA_FILENAME).is_file() {
            scope_dirs.push(path);
        }
    }
    scope_dirs.sort();
    Ok(scope_dirs)
}

pub(crate) fn load_datasource_inventory_records_from_export_root(
    input_dir: &Path,
) -> Result<(DatasourceExportRootManifest, Vec<DatasourceImportRecord>)> {
    let metadata_path = input_dir.join(EXPORT_METADATA_FILENAME);
    let root_manifest = load_datasource_export_root_manifest(&metadata_path)?;
    let records = match root_manifest.scope_kind {
        DatasourceExportRootScopeKind::OrgRoot => {
            load_import_records(input_dir, DatasourceImportInputFormat::Inventory)?.1
        }
        DatasourceExportRootScopeKind::AllOrgsRoot
        | DatasourceExportRootScopeKind::WorkspaceRoot => {
            let scope_dirs = discover_datasource_inventory_scope_dirs(input_dir)?;
            if scope_dirs.is_empty() {
                return Err(message(format!(
                    "Datasource export root {} declares an aggregate datasource scope, but no org-scoped datasource exports were found.",
                    input_dir.display()
                )));
            }
            let mut combined = Vec::new();
            for scope_dir in scope_dirs {
                let records =
                    load_import_records(&scope_dir, DatasourceImportInputFormat::Inventory)?.1;
                combined.extend(records);
            }
            combined
        }
        DatasourceExportRootScopeKind::Unknown => {
            return Err(message(format!(
                "Datasource inspect-export only supports datasource org-root, all-orgs-root, or workspace-root manifests: {}",
                metadata_path.display()
            )));
        }
    };
    Ok((root_manifest, records))
}

fn datasource_import_record_to_diff_value(record: &DatasourceImportRecord) -> Value {
    let mut object = record.to_inventory_record();
    object.insert("isDefault".to_string(), Value::Bool(record.is_default));
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
            let root_manifest = load_datasource_export_root_manifest(&metadata_path)?;
            if root_manifest.scope_kind != DatasourceExportRootScopeKind::OrgRoot {
                return Err(message(format!(
                    "Datasource export manifest {} is not a datasource export root.",
                    metadata_path.display()
                )));
            }
            let metadata = root_manifest.metadata;
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
                validate_datasource_import_record(
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

pub(crate) fn resolve_datasource_export_root_dir(import_dir: &Path) -> Result<PathBuf> {
    let datasource_dir = import_dir.join("datasources");
    let dashboard_dir = import_dir.join("dashboards");
    if import_dir.is_file() || import_dir.join(EXPORT_METADATA_FILENAME).is_file() {
        return Ok(import_dir.to_path_buf());
    }
    if datasource_dir.join(EXPORT_METADATA_FILENAME).is_file() {
        return Ok(datasource_dir);
    }
    if datasource_dir.is_dir() && dashboard_dir.is_dir() {
        return Err(message(format!(
            "Input path {} looks like a snapshot/workspace root containing dashboards/ and datasources/, but datasources/export-metadata.json is missing. Point --import-dir at {} or at an org-scoped datasource export directory.",
            import_dir.display(),
            datasource_dir.display()
        )));
    }
    Ok(import_dir.to_path_buf())
}

pub(crate) fn discover_export_org_import_scopes(
    args: &DatasourceImportArgs,
) -> Result<Vec<DatasourceExportOrgScope>> {
    if !args.use_export_org {
        return Ok(Vec::new());
    }
    let import_root = resolve_datasource_export_root_dir(&args.import_dir)?;
    let metadata_path = import_root.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Err(message(format!(
            "Datasource import with --use-export-org requires export-metadata.json at the combined datasource export root: {}",
            metadata_path.display()
        )));
    }
    let root_manifest = load_datasource_export_root_manifest(&metadata_path)?;
    if !root_manifest.scope_kind.is_aggregate() {
        return Err(message(format!(
            "Datasource import with --use-export-org expects a combined datasource export root with scopeKind all-orgs-root or workspace-root: {}",
            metadata_path.display()
        )));
    }
    let selected_org_ids: BTreeSet<i64> = args.only_org_id.iter().copied().collect();
    let mut scopes = Vec::new();
    let mut matched_source_org_ids = BTreeSet::new();
    for entry in fs::read_dir(&import_root)? {
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
                import_root.display()
            )));
        }
        return Err(message(format!(
            "Datasource import with --use-export-org did not find any org-scoped datasource exports under {}.",
            import_root.display()
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
