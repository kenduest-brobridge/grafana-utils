use serde::Deserialize;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, Result};

use super::super::datasource_export_support::{
    DATASOURCE_PROVISIONING_FILENAME, DATASOURCE_PROVISIONING_SUBDIR,
};
use super::{
    load_datasource_export_root_manifest, validate_datasource_import_record,
    DatasourceExportMetadata, DatasourceExportRootManifest, DatasourceExportRootScopeKind,
    DatasourceImportInputFormat, DatasourceImportRecord, EXPORT_METADATA_FILENAME, ROOT_INDEX_KIND,
    TOOL_SCHEMA_VERSION,
};

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

fn load_inventory_import_records(
    input_dir: &Path,
) -> Result<(DatasourceExportMetadata, Vec<DatasourceImportRecord>)> {
    let metadata_path = input_dir.join(EXPORT_METADATA_FILENAME);
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
    let datasources_path = input_dir.join(&metadata.datasources_file);
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

pub(crate) fn resolve_provisioning_import_source_path(import_path: &Path) -> Result<PathBuf> {
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
        "Datasource provisioning import did not find datasources.yaml under {}. Point --input-dir at the export root, provisioning directory, or concrete YAML file.",
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

fn datasource_import_record_to_diff_value(record: &DatasourceImportRecord) -> Value {
    let mut object = record.to_inventory_record();
    object.insert("isDefault".to_string(), Value::Bool(record.is_default));
    Value::Object(object)
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
                "Datasource list local mode only supports datasource org-root, all-orgs-root, or workspace-root manifests: {}",
                metadata_path.display()
            )));
        }
    };
    Ok((root_manifest, records))
}

pub(crate) fn load_diff_record_values(
    diff_dir: &Path,
    input_format: DatasourceImportInputFormat,
) -> Result<Vec<Value>> {
    match input_format {
        DatasourceImportInputFormat::Inventory => {
            let resolved_diff_dir = resolve_datasource_export_root_dir(diff_dir)?;
            let metadata_path = resolved_diff_dir.join(EXPORT_METADATA_FILENAME);
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
            let datasources_path = resolved_diff_dir.join(&metadata.datasources_file);
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

pub(crate) fn resolve_datasource_export_root_dir(input_dir: &Path) -> Result<PathBuf> {
    let datasource_dir = input_dir.join("datasources");
    let dashboard_dir = input_dir.join("dashboards");
    if input_dir.is_file() || input_dir.join(EXPORT_METADATA_FILENAME).is_file() {
        return Ok(input_dir.to_path_buf());
    }
    if datasource_dir.join(EXPORT_METADATA_FILENAME).is_file() {
        return Ok(datasource_dir);
    }
    if datasource_dir.is_dir() && dashboard_dir.is_dir() {
        return Err(message(format!(
            "Input path {} looks like a snapshot/workspace root containing dashboards/ and datasources/, but datasources/export-metadata.json is missing. Point --input-dir at {} or at an org-scoped datasource export directory.",
            input_dir.display(),
            datasource_dir.display()
        )));
    }
    Ok(input_dir.to_path_buf())
}
