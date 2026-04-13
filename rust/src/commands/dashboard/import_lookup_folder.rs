use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::common::{message, object_field, string_field, validation, value_as_object, Result};
use crate::grafana_api::DashboardResourceClient;

use super::super::{
    build_folder_path, FolderInventoryItem, FolderInventoryStatus, FolderInventoryStatusKind,
    DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID,
};
use super::import_lookup_cache::{fetch_folder_if_exists_cached, ImportLookupCache};

pub(crate) fn normalize_folder_path(path: Option<&str>) -> String {
    let value = path.unwrap_or("").trim();
    if value.is_empty() {
        DEFAULT_FOLDER_TITLE.to_string()
    } else {
        value.to_string()
    }
}

pub(crate) fn resolve_source_dashboard_folder_path(
    document: &Value,
    dashboard_file: &Path,
    input_dir: &Path,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
) -> Result<String> {
    let document_object = value_as_object(document, "Dashboard payload must be a JSON object.")?;
    if let Some(folder_uid) = object_field(document_object, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .map(str::trim)
    {
        if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
            return Ok(DEFAULT_FOLDER_TITLE.to_string());
        }
        if let Some(folder) = folders_by_uid.get(folder_uid) {
            if !folder.path.is_empty() {
                return Ok(folder.path.clone());
            }
            if !folder.title.is_empty() {
                return Ok(folder.title.clone());
            }
        }
    }
    let relative = dashboard_file.strip_prefix(input_dir).map_err(|error| {
        validation(format!(
            "Failed to resolve import-relative dashboard path for {}: {error}",
            dashboard_file.display()
        ))
    })?;
    let parts = relative
        .parent()
        .map(|path| {
            path.iter()
                .map(|part| part.to_string_lossy().into_owned())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if parts.is_empty() {
        Ok(DEFAULT_FOLDER_TITLE.to_string())
    } else {
        Ok(parts.join(" / "))
    }
}

pub(crate) fn determine_import_folder_uid_override_with_request<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    uid: &str,
    folder_uid_override: Option<&str>,
    preserve_existing_folder: bool,
) -> Result<Option<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(value) = folder_uid_override {
        return Ok(Some(value.to_string()));
    }
    if !preserve_existing_folder || uid.is_empty() {
        return Ok(None);
    }
    if let Some(folder_uid) =
        super::import_lookup_cache::dashboard_summary_folder_uid(request_json, cache, uid)?
    {
        if !folder_uid.is_empty() {
            return Ok(Some(folder_uid));
        }
    }
    let Some(existing_payload) =
        super::import_lookup_cache::fetch_dashboard_if_exists_cached(request_json, cache, uid)?
    else {
        return Ok(None);
    };
    let object = value_as_object(
        &existing_payload,
        &format!("Unexpected dashboard payload for UID {uid}."),
    )?;
    let folder_uid = object_field(object, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok(Some(folder_uid))
}

pub(crate) fn determine_import_folder_uid_override_with_client(
    client: &DashboardResourceClient<'_>,
    cache: &mut ImportLookupCache,
    uid: &str,
    folder_uid_override: Option<&str>,
    preserve_existing_folder: bool,
) -> Result<Option<String>> {
    if let Some(value) = folder_uid_override {
        return Ok(Some(value.to_string()));
    }
    if !preserve_existing_folder || uid.is_empty() {
        return Ok(None);
    }
    if let Some(folder_uid) =
        super::import_lookup_cache::dashboard_summary_folder_uid_with_client(client, cache, uid)?
    {
        if !folder_uid.is_empty() {
            return Ok(Some(folder_uid));
        }
    }
    let Some(existing_payload) =
        super::import_lookup_cache::fetch_dashboard_if_exists_cached_with_client(
            client, cache, uid,
        )?
    else {
        return Ok(None);
    };
    let object = value_as_object(
        &existing_payload,
        &format!("Unexpected dashboard payload for UID {uid}."),
    )?;
    let folder_uid = object_field(object, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok(Some(folder_uid))
}

pub(crate) fn resolve_dashboard_import_folder_path_with_request<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    payload: &Value,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
    prefer_live_lookup: bool,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload_object =
        value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let folder_uid = payload_object
        .get("folderUid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let cache_key = (folder_uid.clone(), prefer_live_lookup);
    if let Some(path) = cache.resolved_dashboard_import_folder_paths.get(&cache_key) {
        return Ok(path.clone());
    }
    if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        let path = DEFAULT_FOLDER_TITLE.to_string();
        cache
            .resolved_dashboard_import_folder_paths
            .insert(cache_key, path.clone());
        return Ok(path);
    }
    if prefer_live_lookup {
        if let Some(folder) = fetch_folder_if_exists_cached(request_json, cache, &folder_uid)? {
            let fallback_title = string_field(&folder, "title", &folder_uid);
            let path = build_folder_path(&folder, &fallback_title);
            cache
                .resolved_dashboard_import_folder_paths
                .insert(cache_key, path.clone());
            return Ok(path);
        }
    }
    if let Some(folder) = folders_by_uid.get(&folder_uid) {
        if !folder.path.is_empty() {
            let path = folder.path.clone();
            cache
                .resolved_dashboard_import_folder_paths
                .insert(cache_key, path.clone());
            return Ok(path);
        }
        if !folder.title.is_empty() {
            let path = folder.title.clone();
            cache
                .resolved_dashboard_import_folder_paths
                .insert(cache_key, path.clone());
            return Ok(path);
        }
    }
    if let Some(folder) = fetch_folder_if_exists_cached(request_json, cache, &folder_uid)? {
        let fallback_title = string_field(&folder, "title", &folder_uid);
        let path = build_folder_path(&folder, &fallback_title);
        cache
            .resolved_dashboard_import_folder_paths
            .insert(cache_key, path.clone());
        return Ok(path);
    }
    Ok(folder_uid)
}

pub(crate) fn resolve_dashboard_import_folder_path_with_client(
    client: &DashboardResourceClient<'_>,
    cache: &mut ImportLookupCache,
    payload: &Value,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
    prefer_live_lookup: bool,
) -> Result<String> {
    let payload_object =
        value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let folder_uid = payload_object
        .get("folderUid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let cache_key = (folder_uid.clone(), prefer_live_lookup);
    if let Some(path) = cache.resolved_dashboard_import_folder_paths.get(&cache_key) {
        return Ok(path.clone());
    }
    if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        let path = DEFAULT_FOLDER_TITLE.to_string();
        cache
            .resolved_dashboard_import_folder_paths
            .insert(cache_key, path.clone());
        return Ok(path);
    }
    if prefer_live_lookup {
        if let Some(folder) = super::import_lookup_cache::fetch_folder_if_exists_cached_with_client(
            client,
            cache,
            &folder_uid,
        )? {
            let fallback_title = string_field(&folder, "title", &folder_uid);
            let path = build_folder_path(&folder, &fallback_title);
            cache
                .resolved_dashboard_import_folder_paths
                .insert(cache_key, path.clone());
            return Ok(path);
        }
    }
    if let Some(folder) = folders_by_uid.get(&folder_uid) {
        if !folder.path.is_empty() {
            let path = folder.path.clone();
            cache
                .resolved_dashboard_import_folder_paths
                .insert(cache_key, path.clone());
            return Ok(path);
        }
        if !folder.title.is_empty() {
            let path = folder.title.clone();
            cache
                .resolved_dashboard_import_folder_paths
                .insert(cache_key, path.clone());
            return Ok(path);
        }
    }
    if let Some(folder) = super::import_lookup_cache::fetch_folder_if_exists_cached_with_client(
        client,
        cache,
        &folder_uid,
    )? {
        let fallback_title = string_field(&folder, "title", &folder_uid);
        let path = build_folder_path(&folder, &fallback_title);
        cache
            .resolved_dashboard_import_folder_paths
            .insert(cache_key, path.clone());
        return Ok(path);
    }
    Ok(folder_uid)
}

pub(crate) fn resolve_existing_dashboard_folder_path_with_request<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    uid: &str,
) -> Result<Option<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if uid.is_empty() {
        return Ok(None);
    }
    if let Some(path) = cache.resolved_existing_dashboard_folder_paths.get(uid) {
        return Ok(Some(path.clone()));
    }
    let Some(existing_payload) =
        super::import_lookup_cache::fetch_dashboard_if_exists_cached(request_json, cache, uid)?
    else {
        return Ok(None);
    };
    let object = value_as_object(
        &existing_payload,
        &format!("Unexpected dashboard payload for UID {uid}."),
    )?;
    let folder_uid = object_field(object, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        let path = DEFAULT_FOLDER_TITLE.to_string();
        cache
            .resolved_existing_dashboard_folder_paths
            .insert(uid.to_string(), path.clone());
        return Ok(Some(path));
    }
    let Some(folder) = fetch_folder_if_exists_cached(request_json, cache, &folder_uid)? else {
        return Ok(None);
    };
    let title = string_field(&folder, "title", &folder_uid);
    let path = build_folder_path(&folder, &title);
    if path.trim().is_empty() {
        Ok(None)
    } else {
        cache
            .resolved_existing_dashboard_folder_paths
            .insert(uid.to_string(), path.clone());
        Ok(Some(path))
    }
}

pub(crate) fn resolve_existing_dashboard_folder_path_with_client(
    client: &DashboardResourceClient<'_>,
    cache: &mut ImportLookupCache,
    uid: &str,
) -> Result<Option<String>> {
    if uid.is_empty() {
        return Ok(None);
    }
    if let Some(path) = cache.resolved_existing_dashboard_folder_paths.get(uid) {
        return Ok(Some(path.clone()));
    }
    let Some(existing_payload) =
        super::import_lookup_cache::fetch_dashboard_if_exists_cached_with_client(
            client, cache, uid,
        )?
    else {
        return Ok(None);
    };
    let object = value_as_object(
        &existing_payload,
        &format!("Unexpected dashboard payload for UID {uid}."),
    )?;
    let folder_uid = object_field(object, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        let path = DEFAULT_FOLDER_TITLE.to_string();
        cache
            .resolved_existing_dashboard_folder_paths
            .insert(uid.to_string(), path.clone());
        return Ok(Some(path));
    }
    let Some(folder) = super::import_lookup_cache::fetch_folder_if_exists_cached_with_client(
        client,
        cache,
        &folder_uid,
    )?
    else {
        return Ok(None);
    };
    let title = string_field(&folder, "title", &folder_uid);
    let path = build_folder_path(&folder, &title);
    if path.trim().is_empty() {
        Ok(None)
    } else {
        cache
            .resolved_existing_dashboard_folder_paths
            .insert(uid.to_string(), path.clone());
        Ok(Some(path))
    }
}

fn cached_parent_uid_from_folder(folder: &Map<String, Value>) -> Option<String> {
    folder
        .get("parents")
        .and_then(Value::as_array)
        .and_then(|parents| parents.last())
        .and_then(Value::as_object)
        .map(|parent| string_field(parent, "uid", ""))
        .filter(|uid| !uid.is_empty())
}

fn build_cached_folder_inventory_status(
    folder: &FolderInventoryItem,
    destination_folder: Option<&Map<String, Value>>,
) -> FolderInventoryStatus {
    let expected_parent_uid = folder.parent_uid.clone();
    let mut status = FolderInventoryStatus {
        uid: folder.uid.clone(),
        expected_title: folder.title.clone(),
        expected_parent_uid,
        expected_path: folder.path.clone(),
        actual_title: None,
        actual_parent_uid: None,
        actual_path: None,
        kind: FolderInventoryStatusKind::Missing,
    };
    let Some(destination_folder) = destination_folder else {
        return status;
    };

    status.actual_title = Some(string_field(destination_folder, "title", ""));
    status.actual_parent_uid = cached_parent_uid_from_folder(destination_folder);
    status.actual_path = Some(build_folder_path(destination_folder, &folder.title));
    let title_matches = status.actual_title.as_deref() == Some(folder.title.as_str());
    let parent_matches = status.actual_parent_uid == folder.parent_uid;
    let path_matches = status.actual_path.as_deref() == Some(folder.path.as_str());
    status.kind = if title_matches && parent_matches && path_matches {
        FolderInventoryStatusKind::Matches
    } else {
        FolderInventoryStatusKind::Mismatch
    };
    status
}

pub(crate) fn build_folder_path_match_result(
    source_folder_path: Option<&str>,
    destination_folder_path: Option<&str>,
    destination_exists: bool,
    require_matching_folder_path: bool,
) -> (bool, &'static str, String, Option<String>) {
    let normalized_source = normalize_folder_path(source_folder_path);
    let normalized_destination =
        destination_folder_path.map(|path| normalize_folder_path(Some(path)));
    if !require_matching_folder_path || !destination_exists {
        return (true, "", normalized_source, normalized_destination);
    }
    let Some(ref destination_path) = normalized_destination else {
        return (
            false,
            "folder-path-unknown",
            normalized_source,
            normalized_destination,
        );
    };
    if normalized_source == *destination_path {
        (true, "", normalized_source, normalized_destination)
    } else {
        (
            false,
            "folder-path-mismatch",
            normalized_source,
            normalized_destination,
        )
    }
}

pub(crate) fn apply_folder_path_guard_to_action(
    action: &'static str,
    matches: bool,
) -> &'static str {
    if action == "would-update" && !matches {
        "would-skip-folder-mismatch"
    } else {
        action
    }
}

pub(crate) fn collect_folder_inventory_statuses_cached<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    folder_inventory: &[FolderInventoryItem],
) -> Result<Vec<FolderInventoryStatus>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut statuses = Vec::new();
    for folder in folder_inventory {
        let destination_folder = fetch_folder_if_exists_cached(request_json, cache, &folder.uid)?;
        statuses.push(build_cached_folder_inventory_status(
            folder,
            destination_folder.as_ref(),
        ));
    }
    Ok(statuses)
}

pub(crate) fn collect_folder_inventory_statuses_with_client(
    client: &DashboardResourceClient<'_>,
    cache: &mut ImportLookupCache,
    folder_inventory: &[FolderInventoryItem],
) -> Result<Vec<FolderInventoryStatus>> {
    let mut statuses = Vec::new();
    for folder in folder_inventory {
        let destination_folder =
            super::import_lookup_cache::fetch_folder_if_exists_cached_with_client(
                client,
                cache,
                &folder.uid,
            )?;
        statuses.push(build_cached_folder_inventory_status(
            folder,
            destination_folder.as_ref(),
        ));
    }
    Ok(statuses)
}

pub(crate) fn ensure_folder_inventory_entry_cached<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
    folder_uid: &str,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if folder_uid.is_empty() {
        return Ok(());
    }
    if cache.ensured_folder_uids.contains(folder_uid) {
        return Ok(());
    }
    let mut create_chain = Vec::new();
    let mut current_uid: String = folder_uid.to_string();
    let mut existing_ancestor_uid = None;
    loop {
        if fetch_folder_if_exists_cached(&mut *request_json, cache, &current_uid)?.is_some() {
            existing_ancestor_uid = Some(current_uid.clone());
            break;
        }
        let folder = folders_by_uid.get(&current_uid).ok_or_else(|| {
            message(format!(
                "Missing exported folder inventory for folderUid {current_uid}."
            ))
        })?;
        create_chain.push(folder.clone());
        let Some(parent_uid) = folder.parent_uid.as_deref() else {
            break;
        };
        current_uid = parent_uid.to_string();
    }
    for folder in create_chain.into_iter().rev() {
        if fetch_folder_if_exists_cached(&mut *request_json, cache, &folder.uid)?.is_some() {
            continue;
        }
        super::super::live::create_folder_entry_with_request(
            &mut *request_json,
            &folder.title,
            &folder.uid,
            folder.parent_uid.as_deref(),
        )?;
        let mut created = Map::new();
        created.insert("uid".to_string(), Value::String(folder.uid.clone()));
        created.insert("title".to_string(), Value::String(folder.title.clone()));
        if let Some(parent_uid) = folder.parent_uid.as_ref() {
            let parents = if let Some(parent_folder) =
                fetch_folder_if_exists_cached(&mut *request_json, cache, parent_uid)?
            {
                let parent_title = string_field(&parent_folder, "title", parent_uid);
                vec![Value::Object(Map::from_iter(vec![
                    ("uid".to_string(), Value::String(parent_uid.clone())),
                    ("title".to_string(), Value::String(parent_title)),
                ]))]
            } else {
                vec![Value::Object(Map::from_iter(vec![
                    ("uid".to_string(), Value::String(parent_uid.to_string())),
                    ("title".to_string(), Value::String(parent_uid.to_string())),
                ]))]
            };
            created.insert("parents".to_string(), Value::Array(parents));
        } else {
            created.insert("parents".to_string(), Value::Array(Vec::new()));
        }
        cache
            .folders_by_uid
            .insert(folder.uid.clone(), Some(created));
        cache.ensured_folder_uids.insert(folder.uid.clone());
    }
    if let Some(existing_ancestor_uid) = existing_ancestor_uid {
        cache.ensured_folder_uids.insert(existing_ancestor_uid);
    }
    cache.ensured_folder_uids.insert(folder_uid.to_string());
    Ok(())
}

pub(crate) fn ensure_folder_inventory_entry_with_client(
    client: &DashboardResourceClient<'_>,
    cache: &mut ImportLookupCache,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
    folder_uid: &str,
) -> Result<()> {
    if folder_uid.is_empty() {
        return Ok(());
    }
    if cache.ensured_folder_uids.contains(folder_uid) {
        return Ok(());
    }
    let mut create_chain = Vec::new();
    let mut current_uid: String = folder_uid.to_string();
    let mut existing_ancestor_uid = None;
    loop {
        if super::import_lookup_cache::fetch_folder_if_exists_cached_with_client(
            client,
            cache,
            &current_uid,
        )?
        .is_some()
        {
            existing_ancestor_uid = Some(current_uid.clone());
            break;
        }
        let folder = folders_by_uid.get(&current_uid).ok_or_else(|| {
            message(format!(
                "Missing exported folder inventory for folderUid {current_uid}."
            ))
        })?;
        create_chain.push(folder.clone());
        let Some(parent_uid) = folder.parent_uid.as_deref() else {
            break;
        };
        current_uid = parent_uid.to_string();
    }
    for folder in create_chain.into_iter().rev() {
        if super::import_lookup_cache::fetch_folder_if_exists_cached_with_client(
            client,
            cache,
            &folder.uid,
        )?
        .is_some()
        {
            continue;
        }
        let mut created = Map::new();
        created.insert("uid".to_string(), Value::String(folder.uid.clone()));
        created.insert("title".to_string(), Value::String(folder.title.clone()));
        if let Some(parent_uid) = folder.parent_uid.as_ref() {
            let parents = if let Some(parent_folder) =
                super::import_lookup_cache::fetch_folder_if_exists_cached_with_client(
                    client, cache, parent_uid,
                )? {
                let parent_title = string_field(&parent_folder, "title", parent_uid);
                vec![Value::Object(Map::from_iter(vec![
                    ("uid".to_string(), Value::String(parent_uid.clone())),
                    ("title".to_string(), Value::String(parent_title)),
                ]))]
            } else {
                vec![Value::Object(Map::from_iter(vec![
                    ("uid".to_string(), Value::String(parent_uid.to_string())),
                    ("title".to_string(), Value::String(parent_uid.to_string())),
                ]))]
            };
            created.insert("parents".to_string(), Value::Array(parents));
        } else {
            created.insert("parents".to_string(), Value::Array(Vec::new()));
        }
        cache
            .folders_by_uid
            .insert(folder.uid.clone(), Some(created));
        cache.ensured_folder_uids.insert(folder.uid.clone());
    }
    if let Some(existing_ancestor_uid) = existing_ancestor_uid {
        cache.ensured_folder_uids.insert(existing_ancestor_uid);
    }
    cache.ensured_folder_uids.insert(folder_uid.to_string());
    Ok(())
}
