#![cfg(feature = "tui")]
use std::collections::BTreeMap;

use reqwest::Method;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

use crate::common::{message, string_field, Result};
use crate::grafana_api::DashboardResourceClient;

use super::delete_support::normalize_folder_path;
use super::inspect::{resolve_export_folder_inventory_item, resolve_export_folder_path};
use super::list::{fetch_current_org_with_request, org_id_value};
use super::source_loader::{load_dashboard_source, LoadedDashboardSource};
use super::{
    build_auth_context, build_http_client, build_http_client_for_org,
    collect_folder_inventory_with_request, list_dashboard_summaries_with_request, BrowseArgs,
    DashboardImportInputFormat, DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_TITLE,
};
use crate::dashboard::files::{
    discover_dashboard_files, extract_dashboard_object, load_export_metadata,
    load_folder_inventory, load_json_file,
};

pub(crate) use super::browse_live_detail::fetch_dashboard_view_lines_with_request;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DashboardBrowseSummary {
    pub root_path: Option<String>,
    pub dashboard_count: usize,
    pub folder_count: usize,
    pub org_count: usize,
    pub scope_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DashboardBrowseNodeKind {
    Org,
    Folder,
    Dashboard,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DashboardBrowseNode {
    pub kind: DashboardBrowseNodeKind,
    pub title: String,
    pub path: String,
    pub uid: Option<String>,
    pub depth: usize,
    pub meta: String,
    pub details: Vec<String>,
    pub url: Option<String>,
    pub org_name: String,
    pub org_id: String,
    pub child_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DashboardBrowseDocument {
    pub summary: DashboardBrowseSummary,
    pub nodes: Vec<DashboardBrowseNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FolderNodeRecord {
    title: String,
    path: String,
    uid: Option<String>,
    parent_path: Option<String>,
}

pub(crate) fn load_dashboard_browse_document_for_args<F>(
    request_json: &mut F,
    args: &BrowseArgs,
) -> Result<DashboardBrowseDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(input_dir) = args.input_dir.as_deref().or(args.workspace.as_deref()) {
        return load_dashboard_browse_document_from_local_import_dir(
            input_dir,
            args.input_format,
            args.path.as_deref(),
            args.workspace.is_some(),
        );
    }
    if args.all_orgs {
        return load_dashboard_browse_document_all_orgs(request_json, args);
    }
    load_dashboard_browse_document_with_request(request_json, args.page_size, args.path.as_deref())
}

fn load_dashboard_browse_document_from_local_import_dir(
    input_dir: &Path,
    input_format: DashboardImportInputFormat,
    root_path: Option<&str>,
    strict_workspace: bool,
) -> Result<DashboardBrowseDocument> {
    let resolved = resolve_local_browse_source(input_dir, input_format, strict_workspace)?;
    let metadata = load_export_metadata(&resolved.resolved.metadata_dir, None)?;
    let folder_inventory =
        load_folder_inventory(&resolved.resolved.metadata_dir, metadata.as_ref())?;
    let dashboard_files = discover_dashboard_files(&resolved.resolved.dashboard_dir)?;
    let summaries = build_local_dashboard_summaries(
        &resolved.resolved.dashboard_dir,
        &dashboard_files,
        &folder_inventory,
        metadata.as_ref(),
    )?;
    let org_name = metadata
        .as_ref()
        .and_then(|item| item.org.as_deref())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Local export");
    let org_id = metadata
        .as_ref()
        .and_then(|item| item.org_id.as_deref())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(super::DEFAULT_ORG_ID);
    let mut document = build_dashboard_browse_document_for_org(
        &summaries,
        &folder_inventory,
        root_path,
        org_name,
        org_id,
        true,
        false,
    )?;
    document.summary.scope_label = format!(
        "Local export tree ({})",
        resolved.resolved.dashboard_dir.display()
    );
    Ok(document)
}

fn resolve_local_browse_source(
    input_dir: &Path,
    input_format: DashboardImportInputFormat,
    strict_workspace: bool,
) -> Result<LoadedDashboardSource> {
    load_dashboard_source(input_dir, input_format, None, strict_workspace)
}

fn build_local_dashboard_summaries(
    input_dir: &Path,
    dashboard_files: &[PathBuf],
    folder_inventory: &[super::FolderInventoryItem],
    metadata: Option<&crate::dashboard::models::ExportMetadata>,
) -> Result<Vec<Map<String, Value>>> {
    let mut summaries = Vec::new();
    let folder_inventory_by_uid = folder_inventory
        .iter()
        .cloned()
        .map(|item| (item.uid.clone(), item))
        .collect::<std::collections::BTreeMap<String, super::FolderInventoryItem>>();
    for dashboard_file in dashboard_files {
        if dashboard_file
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.ends_with(".history.json"))
            .unwrap_or(false)
            || dashboard_file
                .components()
                .any(|component| component.as_os_str() == "history")
        {
            continue;
        }
        let document = load_json_file(dashboard_file)?;
        let document_object = document
            .as_object()
            .ok_or_else(|| message("Dashboard file must be a JSON object."))?;
        let dashboard = extract_dashboard_object(document_object)?;
        let uid = string_field(dashboard, "uid", "");
        if uid.is_empty() {
            continue;
        }
        let folder_item = resolve_export_folder_inventory_item(
            document_object,
            dashboard_file,
            input_dir,
            &folder_inventory_by_uid,
        );
        let folder_path = resolve_export_folder_path(
            document_object,
            dashboard_file,
            input_dir,
            &folder_inventory_by_uid,
        );
        let folder_uid = folder_item
            .as_ref()
            .map(|item| item.uid.clone())
            .unwrap_or_else(|| string_field(document_object, "folderUid", ""));
        let folder_title = folder_item
            .as_ref()
            .map(|item| item.title.clone())
            .unwrap_or_else(|| string_field(document_object, "folderTitle", DEFAULT_FOLDER_TITLE));
        let mut summary = Map::new();
        summary.insert("uid".to_string(), Value::String(uid));
        summary.insert(
            "title".to_string(),
            Value::String(string_field(dashboard, "title", DEFAULT_DASHBOARD_TITLE)),
        );
        summary.insert("folderUid".to_string(), Value::String(folder_uid));
        summary.insert("folderTitle".to_string(), Value::String(folder_title));
        summary.insert("folderPath".to_string(), Value::String(folder_path));
        summary.insert("url".to_string(), Value::String(String::new()));
        summary.insert(
            "sourceFile".to_string(),
            Value::String(dashboard_file.display().to_string()),
        );
        summary.insert(
            "orgName".to_string(),
            Value::String(
                metadata
                    .and_then(|item| item.org.clone())
                    .unwrap_or_else(|| "Local export".to_string()),
            ),
        );
        summary.insert(
            "orgId".to_string(),
            Value::String(
                metadata
                    .and_then(|item| item.org_id.clone())
                    .unwrap_or_else(|| super::DEFAULT_ORG_ID.to_string()),
            ),
        );
        summaries.push(summary);
    }
    Ok(summaries)
}

pub(crate) fn load_dashboard_browse_document_with_request<F>(
    mut request_json: F,
    page_size: usize,
    root_path: Option<&str>,
) -> Result<DashboardBrowseDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let org = fetch_current_org_with_request(&mut request_json)?;
    let org_name = string_field(&org, "name", "");
    let org_id = org
        .get("id")
        .map(Value::to_string)
        .unwrap_or_else(|| super::DEFAULT_ORG_ID.to_string());
    let dashboard_summaries = list_dashboard_summaries_with_request(&mut request_json, page_size)?;
    let summaries = super::list::attach_dashboard_folder_paths_with_request(
        &mut request_json,
        &dashboard_summaries,
    )?;
    let folder_inventory = collect_folder_inventory_with_request(&mut request_json, &summaries)?;
    build_dashboard_browse_document_for_org(
        &summaries,
        &folder_inventory,
        root_path,
        &org_name,
        &org_id,
        false,
        false,
    )
}

fn load_dashboard_browse_document_all_orgs<F>(
    _request_json: &mut F,
    args: &BrowseArgs,
) -> Result<DashboardBrowseDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let context = build_auth_context(&args.common)?;
    if context.auth_mode != "basic" {
        return Err(message(
            "Dashboard browse with --all-orgs requires Basic auth (--basic-user / --basic-password).",
        ));
    }

    let client = build_http_client(&args.common)?;
    let dashboard = DashboardResourceClient::new(&client);
    let mut orgs = dashboard.list_orgs()?;
    orgs.sort_by(|left, right| {
        string_field(left, "name", "")
            .to_ascii_lowercase()
            .cmp(&string_field(right, "name", "").to_ascii_lowercase())
            .then_with(|| {
                left.get("id")
                    .map(Value::to_string)
                    .cmp(&right.get("id").map(Value::to_string))
            })
    });

    let mut nodes = Vec::new();
    let mut folder_count = 0usize;
    let mut dashboard_count = 0usize;
    let mut matched_orgs = 0usize;

    for org in &orgs {
        let org_name = string_field(org, "name", "");
        let org_id = org_id_value(org)?;
        let org_id_text = org_id.to_string();
        let client = build_http_client_for_org(&args.common, org_id)?;
        let dashboard = DashboardResourceClient::new(&client);
        let dashboard_summaries = dashboard.list_dashboard_summaries(args.page_size)?;
        let summaries = super::list::attach_dashboard_folder_paths_with_request(
            |method, path, params, payload| dashboard.request_json(method, path, params, payload),
            &dashboard_summaries,
        )?;
        let folder_inventory = collect_folder_inventory_with_request(
            |method, path, params, payload| dashboard.request_json(method, path, params, payload),
            &summaries,
        )?;
        let scoped = build_dashboard_browse_document_for_org(
            &summaries,
            &folder_inventory,
            args.path.as_deref(),
            &org_name,
            &org_id_text,
            false,
            true,
        )?;
        if scoped.summary.dashboard_count == 0 && scoped.summary.folder_count == 0 {
            continue;
        }
        matched_orgs += 1;
        folder_count += scoped.summary.folder_count;
        dashboard_count += scoped.summary.dashboard_count;
        nodes.push(build_org_node(
            &org_name,
            &org_id_text,
            scoped.summary.folder_count,
            scoped.summary.dashboard_count,
        ));
        nodes.extend(scoped.nodes.into_iter().map(|mut node| {
            node.depth += 1;
            node
        }));
    }

    if matched_orgs == 0 {
        if let Some(root) = args.path.as_deref() {
            return Err(message(format!(
                "Dashboard browser folder path did not match any dashboards across all visible orgs: {}",
                normalize_folder_path(root)
            )));
        }
    }

    Ok(DashboardBrowseDocument {
        summary: DashboardBrowseSummary {
            root_path: args.path.as_ref().map(|value| normalize_folder_path(value)),
            dashboard_count,
            folder_count,
            org_count: matched_orgs,
            scope_label: if matched_orgs > 0 {
                "All visible orgs".to_string()
            } else {
                "No matching orgs".to_string()
            },
        },
        nodes,
    })
}

#[allow(dead_code)]
pub(crate) fn build_dashboard_browse_document(
    summaries: &[Map<String, Value>],
    folder_inventory: &[super::FolderInventoryItem],
    root_path: Option<&str>,
) -> Result<DashboardBrowseDocument> {
    build_dashboard_browse_document_for_org(
        summaries,
        folder_inventory,
        root_path,
        super::DEFAULT_ORG_NAME,
        super::DEFAULT_ORG_ID,
        false,
        false,
    )
}

fn build_dashboard_browse_document_for_org(
    summaries: &[Map<String, Value>],
    folder_inventory: &[super::FolderInventoryItem],
    root_path: Option<&str>,
    org_name: &str,
    org_id: &str,
    local_mode: bool,
    allow_empty_root: bool,
) -> Result<DashboardBrowseDocument> {
    let normalized_root = root_path
        .map(normalize_folder_path)
        .filter(|value| !value.is_empty());
    let filtered_summaries = summaries
        .iter()
        .filter(|summary| {
            let folder_path = normalize_folder_path(&string_field(
                summary,
                "folderPath",
                &string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
            ));
            matches_root_path(&folder_path, normalized_root.as_deref())
        })
        .cloned()
        .collect::<Vec<_>>();

    if !allow_empty_root {
        if let Some(root) = normalized_root.as_deref() {
            let has_folder = folder_inventory
                .iter()
                .any(|folder| matches_root_path(&normalize_folder_path(&folder.path), Some(root)));
            let has_dashboard = filtered_summaries.iter().any(|summary| {
                let folder_path = normalize_folder_path(&string_field(
                    summary,
                    "folderPath",
                    &string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
                ));
                matches_root_path(&folder_path, Some(root))
            });
            if !has_folder && !has_dashboard {
                return Err(message(format!(
                    "Dashboard browser folder path did not match any dashboards: {root}"
                )));
            }
        }
    }

    let mut folders = BTreeMap::<String, FolderNodeRecord>::new();
    for folder in folder_inventory {
        ensure_folder_path(
            &mut folders,
            &normalize_folder_path(&folder.path),
            Some(folder.uid.clone()),
        );
    }
    for summary in &filtered_summaries {
        let folder_path = normalize_folder_path(&string_field(
            summary,
            "folderPath",
            &string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
        ));
        let folder_uid = string_field(summary, "folderUid", "");
        ensure_folder_path(
            &mut folders,
            &folder_path,
            (!folder_uid.is_empty()).then_some(folder_uid),
        );
    }

    let folder_keys = folders.keys().cloned().collect::<Vec<_>>();
    let mut folder_dashboard_counts = BTreeMap::<String, usize>::new();
    for folder_path in &folder_keys {
        folder_dashboard_counts.insert(folder_path.clone(), 0);
    }
    for summary in &filtered_summaries {
        let folder_path = normalize_folder_path(&string_field(
            summary,
            "folderPath",
            &string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
        ));
        for ancestor in folder_ancestors(&folder_path) {
            if let Some(count) = folder_dashboard_counts.get_mut(&ancestor) {
                *count += 1;
            }
        }
    }

    let mut folder_child_counts = BTreeMap::<String, usize>::new();
    for record in folders.values() {
        if let Some(parent_path) = record.parent_path.as_ref() {
            *folder_child_counts.entry(parent_path.clone()).or_insert(0) += 1;
        }
    }

    let mut nodes = Vec::new();
    for folder_path in &folder_keys {
        if !matches_root_path(folder_path, normalized_root.as_deref()) {
            continue;
        }
        let Some(record) = folders.get(folder_path) else {
            continue;
        };
        nodes.push(DashboardBrowseNode {
            kind: DashboardBrowseNodeKind::Folder,
            title: record.title.clone(),
            path: record.path.clone(),
            uid: record.uid.clone(),
            depth: folder_depth(folder_path, normalized_root.as_deref()),
            meta: format!(
                "{} folder(s) | {} dashboard(s)",
                folder_child_counts.get(folder_path).copied().unwrap_or(0),
                folder_dashboard_counts
                    .get(folder_path)
                    .copied()
                    .unwrap_or(0)
            ),
            details: vec![
                "Type: Folder".to_string(),
                format!("Org: {}", org_name),
                format!("Org ID: {}", org_id),
                format!("Title: {}", record.title),
                format!("Path: {}", record.path),
                format!("UID: {}", record.uid.as_deref().unwrap_or("-")),
                format!(
                    "Parent path: {}",
                    record.parent_path.as_deref().unwrap_or("-")
                ),
                format!(
                    "Child folders: {}",
                    folder_child_counts.get(folder_path).copied().unwrap_or(0)
                ),
                format!(
                    "Dashboards in subtree: {}",
                    folder_dashboard_counts
                        .get(folder_path)
                        .copied()
                        .unwrap_or(0)
                ),
                if local_mode {
                    "Local browse: read-only file tree.".to_string()
                } else {
                    "Delete: press d to remove dashboards in this subtree.".to_string()
                },
                if local_mode {
                    "Local browse: delete actions are unavailable.".to_string()
                } else {
                    "Delete folders: press D to remove dashboards and folders in this subtree."
                        .to_string()
                },
            ],
            url: None,
            org_name: org_name.to_string(),
            org_id: org_id.to_string(),
            child_count: folder_child_counts.get(folder_path).copied().unwrap_or(0),
        });

        let mut dashboards = filtered_summaries
            .iter()
            .filter(|summary| {
                normalize_folder_path(&string_field(
                    summary,
                    "folderPath",
                    &string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
                )) == *folder_path
            })
            .collect::<Vec<_>>();
        dashboards.sort_by(|left, right| {
            string_field(left, "title", DEFAULT_DASHBOARD_TITLE)
                .cmp(&string_field(right, "title", DEFAULT_DASHBOARD_TITLE))
                .then_with(|| string_field(left, "uid", "").cmp(&string_field(right, "uid", "")))
        });
        for summary in dashboards {
            let title = string_field(summary, "title", DEFAULT_DASHBOARD_TITLE);
            let uid = string_field(summary, "uid", "");
            let url = string_field(summary, "url", "");
            nodes.push(DashboardBrowseNode {
                kind: DashboardBrowseNodeKind::Dashboard,
                title: title.clone(),
                path: folder_path.clone(),
                uid: Some(uid.clone()),
                depth: folder_depth(folder_path, normalized_root.as_deref()) + 1,
                meta: format!("uid={uid}"),
                details: vec![
                    "Type: Dashboard".to_string(),
                    format!("Org: {}", org_name),
                    format!("Org ID: {}", org_id),
                    format!("Title: {title}"),
                    format!("UID: {uid}"),
                    format!("Folder path: {folder_path}"),
                    format!("Folder UID: {}", {
                        let value = string_field(summary, "folderUid", "");
                        if value.is_empty() {
                            "-".to_string()
                        } else {
                            value
                        }
                    }),
                    format!(
                        "URL: {}",
                        if url.is_empty() {
                            "-".to_string()
                        } else {
                            url.clone()
                        }
                    ),
                    if local_mode {
                        "Local browse: live details are unavailable.".to_string()
                    } else {
                        "View: press v to load live dashboard details.".to_string()
                    },
                    if local_mode {
                        format!("Source file: {}", string_field(summary, "sourceFile", "-"))
                    } else {
                        "Advanced edit: press E to open raw dashboard JSON, then review/apply/save it back in the TUI."
                            .to_string()
                    },
                    if local_mode {
                        "Local browse: delete actions are unavailable.".to_string()
                    } else {
                        "Delete: press d to delete this dashboard.".to_string()
                    },
                ],
                url: (!url.is_empty()).then_some(url),
                org_name: org_name.to_string(),
                org_id: org_id.to_string(),
                child_count: 0,
            });
        }
    }

    Ok(DashboardBrowseDocument {
        summary: DashboardBrowseSummary {
            root_path: normalized_root,
            dashboard_count: filtered_summaries.len(),
            folder_count: nodes
                .iter()
                .filter(|node| node.kind == DashboardBrowseNodeKind::Folder)
                .count(),
            org_count: 1,
            scope_label: format!("Org {} ({})", org_name, org_id),
        },
        nodes,
    })
}

fn build_org_node(
    org_name: &str,
    org_id: &str,
    folder_count: usize,
    dashboard_count: usize,
) -> DashboardBrowseNode {
    DashboardBrowseNode {
        kind: DashboardBrowseNodeKind::Org,
        title: org_name.to_string(),
        path: org_name.to_string(),
        uid: None,
        depth: 0,
        meta: format!("{folder_count} folder(s) | {dashboard_count} dashboard(s)"),
        details: vec![
            "Type: Org".to_string(),
            format!("Org: {org_name}"),
            format!("Org ID: {org_id}"),
            format!("Folder count: {folder_count}"),
            format!("Dashboard count: {dashboard_count}"),
            "Browse: select folder or dashboard rows below this org.".to_string(),
        ],
        url: None,
        org_name: org_name.to_string(),
        org_id: org_id.to_string(),
        child_count: folder_count,
    }
}

fn ensure_folder_path(
    folders: &mut BTreeMap<String, FolderNodeRecord>,
    folder_path: &str,
    uid: Option<String>,
) {
    let normalized = normalize_folder_path(folder_path);
    if normalized.is_empty() {
        return;
    }
    let ancestors = folder_ancestors(&normalized);
    for path in ancestors {
        let title = path
            .split(" / ")
            .last()
            .unwrap_or(DEFAULT_FOLDER_TITLE)
            .to_string();
        let parent_path = parent_folder_path(&path);
        let record = folders
            .entry(path.clone())
            .or_insert_with(|| FolderNodeRecord {
                title: title.clone(),
                path: path.clone(),
                uid: None,
                parent_path: parent_path.clone(),
            });
        if path == normalized {
            if let Some(folder_uid) = uid.as_ref().filter(|value| !value.is_empty()) {
                record.uid = Some(folder_uid.clone());
            }
            record.title = title;
            record.parent_path = parent_path;
        }
    }
}

fn folder_ancestors(path: &str) -> Vec<String> {
    let mut ancestors = Vec::new();
    let mut parts = Vec::new();
    for part in path.split(" / ") {
        parts.push(part);
        ancestors.push(parts.join(" / "));
    }
    ancestors
}

fn parent_folder_path(path: &str) -> Option<String> {
    let mut parts = path.split(" / ").collect::<Vec<_>>();
    if parts.len() <= 1 {
        None
    } else {
        parts.pop();
        Some(parts.join(" / "))
    }
}

fn matches_root_path(path: &str, root_path: Option<&str>) -> bool {
    match root_path {
        Some(root) => path == root || path.starts_with(&format!("{root} / ")),
        None => true,
    }
}

fn folder_depth(path: &str, root_path: Option<&str>) -> usize {
    let depth = path.split(" / ").count().saturating_sub(1);
    match root_path {
        Some(root) => depth.saturating_sub(root.split(" / ").count().saturating_sub(1)),
        None => depth,
    }
}

#[cfg(test)]
#[cfg(test)]
#[path = "browse_support_rust_tests.rs"]
mod tests;
