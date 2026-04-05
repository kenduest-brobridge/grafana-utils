//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    message, object_field, should_print_stdout, write_plain_output_file, Result,
};

use super::super::files::{load_datasource_inventory, load_folder_inventory};
use super::super::inspect_live::load_variant_index_entries;
use super::super::models::{ExportMetadata, FolderInventoryItem, VariantIndexEntry};
use super::super::{DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID, RAW_EXPORT_SUBDIR};

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

pub(crate) fn write_inspect_output(
    output: &str,
    output_file: Option<&PathBuf>,
    also_stdout: bool,
) -> Result<()> {
    let normalized = output.trim_end_matches('\n');
    if normalized.is_empty() {
        return Ok(());
    }
    if let Some(output_path) = output_file {
        write_plain_output_file(output_path, normalized)?;
    }
    if should_print_stdout(output_file.map(PathBuf::as_path), also_stdout) {
        print!("{normalized}");
        println!();
    }
    Ok(())
}

fn load_export_identity_values(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
    field_name: &str,
) -> Result<BTreeSet<String>> {
    let mut values = BTreeSet::new();
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

pub(crate) fn resolve_export_identity_field(
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

pub(crate) fn load_dashboard_org_scope_by_file(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<BTreeMap<String, (String, String)>> {
    let mut scope_by_file = BTreeMap::new();
    for entry in load_variant_index_entries(import_dir, metadata)? {
        scope_by_file.insert(
            normalize_index_entry_path(&entry.path),
            (entry.org, entry.org_id),
        );
    }
    Ok(scope_by_file)
}

pub(crate) fn load_inspect_source_root(import_dir: &Path) -> Option<PathBuf> {
    let source_root_path = import_dir.join(".inspect-source-root");
    let raw = fs::read_to_string(source_root_path).ok()?;
    let text = raw.trim();
    if text.is_empty() {
        None
    } else {
        Some(PathBuf::from(text))
    }
}

pub(crate) fn resolve_dashboard_source_file_path(
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

pub(crate) fn resolve_export_folder_inventory_item(
    document: &Map<String, Value>,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
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

pub(crate) fn resolve_export_folder_path(
    document: &Map<String, Value>,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
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
