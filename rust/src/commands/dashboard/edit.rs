#![cfg(feature = "tui")]
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};

use super::browse_support::{
    DashboardBrowseDocument, DashboardBrowseNode, DashboardBrowseNodeKind,
};
use super::delete_support::normalize_folder_path;
use super::{
    extract_dashboard_object, fetch_dashboard_with_request, import_dashboard_request_with_request,
};

pub(crate) const BROWSE_EDIT_MESSAGE: &str = "Edited by grafana-utils dashboard browse";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DashboardEditDraft {
    pub uid: String,
    pub title: String,
    pub folder_path: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct DashboardEditUpdate {
    pub title: Option<String>,
    pub folder_path: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub(crate) fn fetch_dashboard_edit_draft_with_request<F>(
    mut request_json: F,
    node: &DashboardBrowseNode,
) -> Result<DashboardEditDraft>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if node.kind != DashboardBrowseNodeKind::Dashboard {
        return Err(message(
            "Dashboard edit is only available for dashboard rows.",
        ));
    }
    let uid = node
        .uid
        .as_deref()
        .ok_or_else(|| message("Dashboard edit requires a dashboard UID."))?;
    let payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let object = value_as_object(&payload, "Unexpected dashboard payload for edit.")?;
    let dashboard = extract_dashboard_object(object)?;
    let tags = dashboard
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(DashboardEditDraft {
        uid: uid.to_string(),
        title: string_field(dashboard, "title", &node.title),
        folder_path: node.path.clone(),
        tags,
    })
}

pub(crate) fn resolve_folder_uid_for_path(
    document: &DashboardBrowseDocument,
    path: &str,
) -> Result<String> {
    let normalized = normalize_folder_path(path);
    document
        .nodes
        .iter()
        .find(|node| {
            node.kind == DashboardBrowseNodeKind::Folder
                && normalize_folder_path(&node.path) == normalized
        })
        .and_then(|node| node.uid.clone())
        .filter(|uid| !uid.is_empty())
        .ok_or_else(|| {
            message(format!(
                "Dashboard edit requires a known destination folder UID for path: {normalized}"
            ))
        })
}

pub(crate) fn apply_dashboard_edit_with_request<F>(
    mut request_json: F,
    draft: &DashboardEditDraft,
    update: &DashboardEditUpdate,
    destination_folder_uid: Option<&str>,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload = fetch_dashboard_with_request(&mut request_json, &draft.uid)?;
    let object = value_as_object(&payload, "Unexpected dashboard payload for edit.")?;
    let dashboard = extract_dashboard_object(object)?;
    let mut dashboard = dashboard.clone();
    dashboard.insert("id".to_string(), Value::Null);

    if let Some(title) = update.title.as_ref() {
        dashboard.insert("title".to_string(), Value::String(title.clone()));
    }
    if let Some(tags) = update.tags.as_ref() {
        dashboard.insert(
            "tags".to_string(),
            Value::Array(tags.iter().cloned().map(Value::String).collect()),
        );
    }

    let current_folder_uid = object
        .get("meta")
        .and_then(Value::as_object)
        .map(|meta| string_field(meta, "folderUid", ""))
        .filter(|value| !value.is_empty());
    let folder_uid = destination_folder_uid
        .map(str::to_string)
        .or(current_folder_uid);

    let mut import_payload = Map::new();
    import_payload.insert("dashboard".to_string(), Value::Object(dashboard));
    import_payload.insert("overwrite".to_string(), Value::Bool(true));
    import_payload.insert(
        "message".to_string(),
        Value::String(BROWSE_EDIT_MESSAGE.to_string()),
    );
    if let Some(folder_uid) = folder_uid {
        import_payload.insert("folderUid".to_string(), Value::String(folder_uid));
    }
    let _ =
        import_dashboard_request_with_request(&mut request_json, &Value::Object(import_payload))?;
    Ok(())
}
