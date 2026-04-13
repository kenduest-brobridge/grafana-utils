//! Datasource import bundle loading and export-org routing helpers.
//!
//! Maintainer notes:
//! - This module is the contract gate for on-disk datasource bundles; reject
//!   mixed-schema or ambiguous org metadata here before import logic runs.
//! - `--use-export-org` routing prefers explicit metadata from exported files and
//!   falls back to `org_<id>_<name>` directory names only when the bundle lacks
//!   stable org fields.

use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

use crate::common::{message, string_field, Result};
use crate::grafana_api::DatasourceResourceClient;
use crate::http::JsonHttpClient;

use super::datasource_export_support::parse_export_metadata;
use super::{DatasourceImportArgs, DatasourceImportInputFormat};

#[path = "support_io.rs"]
mod datasource_import_export_support_io;
#[path = "support_orgs.rs"]
mod datasource_import_export_support_orgs;

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

pub(crate) use datasource_import_export_support_io::{
    discover_datasource_inventory_scope_dirs, load_datasource_inventory_records_from_export_root,
    load_diff_record_values, load_import_records, resolve_datasource_export_root_dir,
};
pub(crate) use datasource_import_export_support_orgs::{
    discover_export_org_import_scopes, validate_matching_export_org,
};

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
    pub(crate) input_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct DatasourceExportOrgTargetPlan {
    pub(crate) source_org_id: i64,
    pub(crate) source_org_name: String,
    pub(crate) target_org_id: Option<i64>,
    pub(crate) org_action: &'static str,
    pub(crate) input_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceImportDryRunReport {
    pub(crate) mode: String,
    pub(crate) input_dir: PathBuf,
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
