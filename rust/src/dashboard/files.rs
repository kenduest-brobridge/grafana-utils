//! File-system contract for dashboard exports/imports.
//! Owns dashboard file discovery, index construction/parsing, and structured JSON write/read paths.
use serde::Serialize;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, object_field, string_field, tool_version, value_as_object, Result};
use crate::export_metadata::{build_export_metadata_common, EXPORT_BUNDLE_KIND_ROOT};

use super::{
    DashboardExportRootManifest, DashboardExportRootScopeKind, DashboardImportInputFormat,
    DashboardIndexItem, DatasourceInventoryItem, ExportMetadata, ExportOrgSummary,
    FolderInventoryItem, RootExportIndex, RootExportVariants, VariantIndexEntry,
    DASHBOARD_PERMISSION_BUNDLE_FILENAME, DATASOURCE_INVENTORY_FILENAME, DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID, DEFAULT_ORG_ID, DEFAULT_ORG_NAME,
    EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME, PROMPT_EXPORT_SUBDIR,
    PROVISIONING_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR, ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedDashboardImportSource {
    pub dashboard_dir: PathBuf,
    pub metadata_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedDashboardExportRoot {
    pub(crate) manifest: DashboardExportRootManifest,
    pub(crate) metadata_dir: PathBuf,
}

pub(crate) fn resolve_dashboard_import_source(
    input_dir: &Path,
    input_format: DashboardImportInputFormat,
) -> Result<ResolvedDashboardImportSource> {
    match input_format {
        DashboardImportInputFormat::Raw => Ok(ResolvedDashboardImportSource {
            dashboard_dir: input_dir.to_path_buf(),
            metadata_dir: input_dir.to_path_buf(),
        }),
        DashboardImportInputFormat::Provisioning => {
            if !input_dir.exists() {
                return Err(message(format!(
                    "Import directory does not exist: {}",
                    input_dir.display()
                )));
            }
            if !input_dir.is_dir() {
                return Err(message(format!(
                    "Import path is not a directory: {}",
                    input_dir.display()
                )));
            }
            let nested_dashboards_dir = input_dir.join("dashboards");
            if nested_dashboards_dir.is_dir() {
                return Ok(ResolvedDashboardImportSource {
                    dashboard_dir: nested_dashboards_dir,
                    metadata_dir: input_dir.to_path_buf(),
                });
            }
            if input_dir.file_name().and_then(|name| name.to_str()) == Some("dashboards") {
                let metadata_dir = input_dir.parent().ok_or_else(|| {
                    message(format!(
                        "Dashboard provisioning import expects a parent provisioning directory for {}.",
                        input_dir.display()
                    ))
                })?;
                return Ok(ResolvedDashboardImportSource {
                    dashboard_dir: input_dir.to_path_buf(),
                    metadata_dir: metadata_dir.to_path_buf(),
                });
            }
            Err(message(format!(
                "Dashboard provisioning import expects --input-dir to point at the {}/ root or its dashboards/ directory: {}",
                PROVISIONING_EXPORT_SUBDIR,
                input_dir.display()
            )))
        }
    }
}

/// discover dashboard files.
pub(crate) fn discover_dashboard_files(input_dir: &Path) -> Result<Vec<PathBuf>> {
    if !input_dir.exists() {
        return Err(message(format!(
            "Import directory does not exist: {}",
            input_dir.display()
        )));
    }
    if !input_dir.is_dir() {
        return Err(message(format!(
            "Import path is not a directory: {}",
            input_dir.display()
        )));
    }
    if input_dir.join(RAW_EXPORT_SUBDIR).is_dir() && input_dir.join(PROMPT_EXPORT_SUBDIR).is_dir() {
        return Err(message(format!(
            "Import path {} looks like the combined export root. Point --input-dir at {}.",
            input_dir.display(),
            input_dir.join(RAW_EXPORT_SUBDIR).display()
        )));
    }

    let mut files = Vec::new();
    collect_json_files(input_dir, &mut files)?;
    files.retain(|path| {
        let file_name = path.file_name().and_then(|name| name.to_str());
        file_name != Some("index.json")
            && file_name != Some(EXPORT_METADATA_FILENAME)
            && file_name != Some(FOLDER_INVENTORY_FILENAME)
            && file_name != Some(DATASOURCE_INVENTORY_FILENAME)
            && file_name != Some(DASHBOARD_PERMISSION_BUNDLE_FILENAME)
    });
    files.sort();

    if files.is_empty() {
        return Err(message(format!(
            "No dashboard JSON files found in {}",
            input_dir.display()
        )));
    }

    Ok(files)
}

/// Purpose: implementation note.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_export_metadata(
    variant: &str,
    dashboard_count: usize,
    format_name: Option<&str>,
    folders_file: Option<&str>,
    datasources_file: Option<&str>,
    permissions_file: Option<&str>,
    org_name: Option<&str>,
    org_id: Option<&str>,
    orgs: Option<Vec<ExportOrgSummary>>,
    source_kind: &str,
    source_url: Option<&str>,
    source_path: Option<&Path>,
    source_profile: Option<&str>,
    artifact_path: &Path,
    metadata_path: &Path,
) -> ExportMetadata {
    let org_count = orgs.as_ref().map(|items| items.len() as u64);
    let scope_kind = if variant == "root" {
        Some(if orgs.is_some() {
            "all-orgs-root".to_string()
        } else {
            "org-root".to_string()
        })
    } else {
        None
    };
    let org_scope = if variant == "root" {
        if orgs.is_some() {
            Some("all-orgs")
        } else {
            Some("org")
        }
    } else {
        Some("org")
    };
    let common = build_export_metadata_common(
        "dashboard",
        "dashboards",
        EXPORT_BUNDLE_KIND_ROOT,
        source_kind,
        source_url,
        source_path,
        source_profile,
        org_scope,
        org_id,
        org_name,
        artifact_path,
        metadata_path,
        dashboard_count,
    );
    ExportMetadata {
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: Some(tool_version().to_string()),
        kind: ROOT_INDEX_KIND.to_string(),
        variant: variant.to_string(),
        scope_kind,
        dashboard_count: dashboard_count as u64,
        index_file: "index.json".to_string(),
        format: format_name.map(str::to_owned),
        folders_file: folders_file.map(str::to_owned),
        datasources_file: datasources_file.map(str::to_owned),
        permissions_file: permissions_file.map(str::to_owned),
        org: org_name.map(str::to_owned),
        org_id: org_id.map(str::to_owned),
        org_count,
        orgs,
        metadata_version: Some(common.metadata_version),
        domain: Some(common.domain),
        resource_kind: Some(common.resource_kind),
        bundle_kind: Some(common.bundle_kind),
        source: Some(common.source),
        capture: Some(common.capture),
        paths: Some(common.paths),
    }
}

fn validate_export_metadata_contract(
    metadata: &ExportMetadata,
    metadata_path: &Path,
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
    Ok(())
}

fn validate_export_metadata(
    metadata: &ExportMetadata,
    metadata_path: &Path,
    expected_variant: Option<&str>,
) -> Result<()> {
    validate_export_metadata_contract(metadata, metadata_path)?;
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

pub(crate) fn load_dashboard_export_root_manifest(
    metadata_path: &Path,
) -> Result<DashboardExportRootManifest> {
    let value = load_json_file(metadata_path)?;
    value_as_object(&value, "Dashboard export metadata must be a JSON object.")?;
    let metadata: ExportMetadata = serde_json::from_value(value).map_err(|error| {
        message(format!(
            "Invalid dashboard export metadata in {}: {error}",
            metadata_path.display()
        ))
    })?;
    validate_export_metadata_contract(&metadata, metadata_path)?;
    Ok(DashboardExportRootManifest::from_metadata(metadata))
}

pub(crate) fn resolve_dashboard_export_root(
    input_dir: &Path,
) -> Result<Option<ResolvedDashboardExportRoot>> {
    let metadata_path = input_dir.join(EXPORT_METADATA_FILENAME);
    if metadata_path.is_file() {
        return Ok(Some(ResolvedDashboardExportRoot {
            manifest: load_dashboard_export_root_manifest(&metadata_path)?,
            metadata_dir: input_dir.to_path_buf(),
        }));
    }

    let dashboard_dir = input_dir.join("dashboards");
    let dashboard_metadata_path = dashboard_dir.join(EXPORT_METADATA_FILENAME);
    if dashboard_metadata_path.is_file() {
        let manifest = load_dashboard_export_root_manifest(&dashboard_metadata_path)?;
        let manifest =
            if input_dir.join("datasources").is_dir() && manifest.scope_kind.is_aggregate() {
                manifest.with_scope_kind(DashboardExportRootScopeKind::WorkspaceRoot)
            } else {
                manifest
            };
        return Ok(Some(ResolvedDashboardExportRoot {
            manifest,
            metadata_dir: dashboard_dir,
        }));
    }

    Ok(None)
}

/// load export metadata.
pub(crate) fn load_export_metadata(
    input_dir: &Path,
    expected_variant: Option<&str>,
) -> Result<Option<ExportMetadata>> {
    let metadata_path = input_dir.join(EXPORT_METADATA_FILENAME);
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
            if path.file_name().and_then(|value| value.to_str()) == Some("history") {
                continue;
            }
            collect_json_files(&path, files)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(())
}

/// load json file.
pub(crate) fn load_json_file(path: &Path) -> Result<Value> {
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

/// Purpose: implementation note.
pub(crate) fn build_import_payload(
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
    if let Some(folder_uid) = folder_uid
        .filter(|value| !value.is_empty())
        .filter(|value| value != DEFAULT_FOLDER_UID)
    {
        payload.insert("folderUid".to_string(), Value::String(folder_uid));
    }
    Ok(Value::Object(payload))
}

/// Purpose: implementation note.
pub(crate) fn build_preserved_web_import_document(payload: &Value) -> Result<Value> {
    let object = value_as_object(payload, "Unexpected dashboard payload from Grafana.")?;
    let mut dashboard = extract_dashboard_object(object)?.clone();
    dashboard.insert("id".to_string(), Value::Null);
    Ok(Value::Object(dashboard))
}

/// extract dashboard object.
pub(crate) fn extract_dashboard_object(
    document: &Map<String, Value>,
) -> Result<&Map<String, Value>> {
    match document.get("dashboard") {
        Some(value) => value_as_object(value, "Dashboard payload must be a JSON object."),
        None => Ok(document),
    }
}

/// write dashboard.
pub(crate) fn write_dashboard(payload: &Value, output_path: &Path, overwrite: bool) -> Result<()> {
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

/// write json document.
pub(crate) fn write_json_document<T: Serialize>(payload: &T, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, serde_json::to_string_pretty(payload)? + "\n")?;
    Ok(())
}

/// Purpose: implementation note.
pub(crate) fn build_dashboard_index_item(
    summary: &Map<String, Value>,
    uid: &str,
) -> DashboardIndexItem {
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
        provisioning_path: None,
    }
}

/// Purpose: implementation note.
pub(crate) fn build_variant_index(
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

/// Purpose: implementation note.
pub(crate) fn build_root_export_index(
    items: &[DashboardIndexItem],
    raw_index_path: Option<&Path>,
    prompt_index_path: Option<&Path>,
    provisioning_index_path: Option<&Path>,
    folders: &[FolderInventoryItem],
) -> RootExportIndex {
    RootExportIndex {
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: Some(tool_version().to_string()),
        kind: ROOT_INDEX_KIND.to_string(),
        items: items.to_vec(),
        variants: RootExportVariants {
            raw: raw_index_path.map(|path| path.display().to_string()),
            prompt: prompt_index_path.map(|path| path.display().to_string()),
            provisioning: provisioning_index_path.map(|path| path.display().to_string()),
        },
        folders: folders.to_vec(),
    }
}

/// load folder inventory.
pub(crate) fn load_folder_inventory(
    input_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<Vec<FolderInventoryItem>> {
    let folders_file = metadata
        .and_then(|item| item.folders_file.as_deref())
        .unwrap_or(FOLDER_INVENTORY_FILENAME);
    let folder_inventory_path = input_dir.join(folders_file);
    if !folder_inventory_path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&folder_inventory_path)?;
    serde_json::from_str(&raw).map_err(Into::into)
}

/// load datasource inventory.
pub(crate) fn load_datasource_inventory(
    input_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<Vec<DatasourceInventoryItem>> {
    let datasources_file = metadata
        .and_then(|item| item.datasources_file.as_deref())
        .unwrap_or(DATASOURCE_INVENTORY_FILENAME);
    let datasource_inventory_path = input_dir.join(datasources_file);
    if !datasource_inventory_path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&datasource_inventory_path)?;
    serde_json::from_str(&raw).map_err(Into::into)
}
