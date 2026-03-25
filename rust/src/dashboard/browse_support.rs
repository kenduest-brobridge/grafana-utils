use std::collections::BTreeMap;

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, Result};

use super::delete_support::normalize_folder_path;
use super::{
    collect_folder_inventory_with_request, fetch_dashboard_with_request,
    list_dashboard_summaries_with_request, DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_TITLE,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DashboardBrowseSummary {
    pub root_path: Option<String>,
    pub dashboard_count: usize,
    pub folder_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DashboardBrowseNodeKind {
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

pub(crate) fn load_dashboard_browse_document_with_request<F>(
    mut request_json: F,
    page_size: usize,
    root_path: Option<&str>,
) -> Result<DashboardBrowseDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let dashboard_summaries = list_dashboard_summaries_with_request(&mut request_json, page_size)?;
    let summaries = super::list::attach_dashboard_folder_paths_with_request(
        &mut request_json,
        &dashboard_summaries,
    )?;
    let folder_inventory = collect_folder_inventory_with_request(&mut request_json, &summaries)?;
    build_dashboard_browse_document(&summaries, &folder_inventory, root_path)
}

pub(crate) fn build_dashboard_browse_document(
    summaries: &[Map<String, Value>],
    folder_inventory: &[super::FolderInventoryItem],
    root_path: Option<&str>,
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
                "Delete: press d to remove dashboards in this subtree.".to_string(),
                "Delete folders: press D to remove dashboards and folders in this subtree."
                    .to_string(),
            ],
            url: None,
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
                    "View: press v to load live dashboard details.".to_string(),
                    "Advanced edit: press E to open raw dashboard JSON in an external editor."
                        .to_string(),
                    "Delete: press d to delete this dashboard.".to_string(),
                ],
                url: (!url.is_empty()).then_some(url),
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
        },
        nodes,
    })
}

pub(crate) fn fetch_dashboard_view_lines_with_request<F>(
    mut request_json: F,
    node: &DashboardBrowseNode,
) -> Result<Vec<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if node.kind != DashboardBrowseNodeKind::Dashboard {
        return Ok(node.details.clone());
    }
    let Some(uid) = node.uid.as_deref() else {
        return Ok(node.details.clone());
    };
    let payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let dashboard = payload
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message(format!("Unexpected dashboard payload for UID {uid}.")))?;
    let meta = payload.get("meta").and_then(Value::as_object);
    let tags = dashboard
        .get("tags")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let panels = dashboard
        .get("panels")
        .and_then(Value::as_array)
        .map(|values| values.len())
        .unwrap_or(0);
    let links = dashboard
        .get("links")
        .and_then(Value::as_array)
        .map(|values| values.len())
        .unwrap_or(0);
    let version = dashboard
        .get("version")
        .map(display_value)
        .unwrap_or_else(|| "-".to_string());
    let schema_version = dashboard
        .get("schemaVersion")
        .map(display_value)
        .unwrap_or_else(|| "-".to_string());
    let editable = meta
        .and_then(|item| item.get("canEdit"))
        .map(display_value)
        .unwrap_or_else(|| "-".to_string());
    let slug = meta
        .map(|item| string_field(item, "slug", ""))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string());

    let mut lines = node.details.clone();
    lines.push(String::new());
    lines.push("Live details:".to_string());
    lines.push(format!("Slug: {slug}"));
    lines.push(format!("Version: {version}"));
    lines.push(format!("Schema version: {schema_version}"));
    lines.push(format!("Editable: {editable}"));
    lines.push(format!("Panels: {panels}"));
    lines.push(format!("Links: {links}"));
    lines.push(format!(
        "Timezone: {}",
        dashboard
            .get("timezone")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("default")
    ));
    lines.push(format!(
        "Refresh: {}",
        dashboard
            .get("refresh")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("-")
    ));
    lines.push(format!(
        "Tags: {}",
        if tags.is_empty() {
            "-".to_string()
        } else {
            tags.join(", ")
        }
    ));
    if let Some(history_lines) = fetch_dashboard_history_lines_with_request(
        &mut request_json,
        dashboard.get("id").and_then(Value::as_i64),
    )? {
        lines.push(String::new());
        lines.push("Recent versions:".to_string());
        lines.extend(history_lines);
    }
    Ok(lines)
}

fn fetch_dashboard_history_lines_with_request<F>(
    request_json: &mut F,
    dashboard_id: Option<i64>,
) -> Result<Option<Vec<String>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(dashboard_id) = dashboard_id else {
        return Ok(None);
    };
    let path = format!("/api/dashboards/id/{dashboard_id}/versions");
    let params = vec![("limit".to_string(), "5".to_string())];
    let response = match request_json(Method::GET, &path, &params, None) {
        Ok(response) => response,
        Err(error) if error.status_code() == Some(404) => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(Value::Array(items)) = response else {
        return Ok(None);
    };
    let lines = items
        .into_iter()
        .filter_map(|item| item.as_object().cloned())
        .map(|item| {
            let version = item
                .get("version")
                .map(display_value)
                .unwrap_or_else(|| "-".to_string());
            let created = item
                .get("created")
                .map(display_value)
                .unwrap_or_else(|| "-".to_string());
            let message = string_field(&item, "message", "");
            let author = string_field(&item, "createdBy", "");
            let mut parts = vec![format!("v{version}"), created];
            if !author.is_empty() {
                parts.push(author);
            }
            if !message.is_empty() {
                parts.push(message);
            }
            format!("- {}", parts.join(" | "))
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        Ok(None)
    } else {
        Ok(Some(lines))
    }
}

fn ensure_folder_path(
    folders: &mut BTreeMap<String, FolderNodeRecord>,
    path: &str,
    uid: Option<String>,
) {
    let normalized = normalize_folder_path(path);
    if normalized.is_empty() {
        return;
    }
    let mut current = Vec::new();
    for segment in normalized.split(" / ") {
        current.push(segment);
        let current_path = current.join(" / ");
        let parent_path = (current.len() > 1).then(|| current[..current.len() - 1].join(" / "));
        folders
            .entry(current_path.clone())
            .or_insert_with(|| FolderNodeRecord {
                title: segment.to_string(),
                path: current_path.clone(),
                uid: None,
                parent_path: parent_path.clone(),
            });
    }
    if let Some(folder_uid) = uid.filter(|value| !value.is_empty()) {
        if let Some(record) = folders.get_mut(&normalized) {
            record.uid = Some(folder_uid);
        }
    }
}

fn folder_ancestors(path: &str) -> Vec<String> {
    let parts = normalize_folder_path(path)
        .split(" / ")
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut ancestors = Vec::new();
    for index in 0..parts.len() {
        ancestors.push(parts[..=index].join(" / "));
    }
    ancestors
}

fn matches_root_path(path: &str, root_path: Option<&str>) -> bool {
    match root_path {
        Some(root) => path == root || path.starts_with(&format!("{root} / ")),
        None => true,
    }
}

fn folder_depth(path: &str, root_path: Option<&str>) -> usize {
    let depth = normalize_folder_path(path).matches(" / ").count();
    match root_path {
        Some(root) => depth.saturating_sub(normalize_folder_path(root).matches(" / ").count()),
        None => depth,
    }
}

fn display_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}
