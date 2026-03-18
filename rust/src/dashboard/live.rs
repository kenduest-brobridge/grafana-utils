//! Live dashboard and folder/index fetch helpers.
//! Encapsulates paged Grafana API reads used by list/export/import/inspect flows.
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;

use super::{
    DatasourceInventoryItem, FolderInventoryItem, FolderInventoryStatus, FolderInventoryStatusKind,
    DEFAULT_FOLDER_TITLE, DEFAULT_ORG_ID, DEFAULT_ORG_NAME,
};

/// Purpose: implementation note.
pub(crate) fn list_dashboard_summaries_with_request<F>(
    mut request_json: F,
    page_size: usize,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut dashboards = Vec::new();
    let mut seen_uids = std::collections::BTreeSet::new();
    let mut page = 1;

    loop {
        let params = vec![
            ("type".to_string(), "dash-db".to_string()),
            ("limit".to_string(), page_size.to_string()),
            ("page".to_string(), page.to_string()),
        ];
        let response = request_json(Method::GET, "/api/search", &params, None)?;
        let batch = match response {
            Some(Value::Array(batch)) => batch,
            Some(_) => return Err(message("Unexpected search response from Grafana.")),
            None => Vec::new(),
        };

        if batch.is_empty() {
            break;
        }

        let batch_len = batch.len();
        for item in batch {
            let object =
                value_as_object(&item, "Unexpected dashboard summary payload from Grafana.")?;
            let uid = string_field(object, "uid", "");
            if uid.is_empty() || seen_uids.contains(&uid) {
                continue;
            }
            seen_uids.insert(uid);
            dashboards.push(object.clone());
        }

        if batch_len < page_size {
            break;
        }
        page += 1;
    }

    Ok(dashboards)
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn list_dashboard_summaries(
    client: &JsonHttpClient,
    page_size: usize,
) -> Result<Vec<Map<String, Value>>> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

    list_dashboard_summaries_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        page_size,
    )
}

/// fetch folder if exists with request.
pub(crate) fn fetch_folder_if_exists_with_request<F>(
    mut request_json: F,
    uid: &str,
) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, &format!("/api/folders/{uid}"), &[], None)? {
        Some(value) => {
            let object =
                value_as_object(&value, &format!("Unexpected folder payload for UID {uid}."))?;
            Ok(Some(object.clone()))
        }
        None => Ok(None),
    }
}

/// collect folder inventory with request.
pub(crate) fn collect_folder_inventory_with_request<F>(
    mut request_json: F,
    summaries: &[Map<String, Value>],
) -> Result<Vec<FolderInventoryItem>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut seen = std::collections::BTreeSet::new();
    let mut folders = Vec::new();
    for summary in summaries {
        let folder_uid = string_field(summary, "folderUid", "");
        if folder_uid.is_empty() {
            continue;
        }
        let org_id = summary
            .get("orgId")
            .map(|value| match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            })
            .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
        let key = format!("{org_id}:{folder_uid}");
        if seen.contains(&key) {
            continue;
        }
        let Some(folder) = fetch_folder_if_exists_with_request(&mut request_json, &folder_uid)?
        else {
            continue;
        };
        let org = string_field(summary, "orgName", DEFAULT_ORG_NAME);
        let mut parent_path = Vec::new();
        let mut previous_parent_uid = None;
        if let Some(parents) = folder.get("parents").and_then(Value::as_array) {
            for parent in parents {
                let Some(parent_object) = parent.as_object() else {
                    continue;
                };
                let parent_uid = string_field(parent_object, "uid", "");
                let parent_title = string_field(parent_object, "title", "");
                if parent_uid.is_empty() || parent_title.is_empty() {
                    continue;
                }
                parent_path.push(parent_title.clone());
                let parent_key = format!("{org_id}:{parent_uid}");
                if !seen.contains(&parent_key) {
                    folders.push(FolderInventoryItem {
                        uid: parent_uid.clone(),
                        title: parent_title,
                        path: parent_path.join(" / "),
                        parent_uid: previous_parent_uid.clone(),
                        org: org.clone(),
                        org_id: org_id.clone(),
                    });
                    seen.insert(parent_key);
                }
                previous_parent_uid = Some(parent_uid);
            }
        }
        let folder_title = string_field(&folder, "title", DEFAULT_FOLDER_TITLE);
        parent_path.push(folder_title.clone());
        folders.push(FolderInventoryItem {
            uid: folder_uid.clone(),
            title: folder_title,
            path: parent_path.join(" / "),
            parent_uid: previous_parent_uid,
            org,
            org_id: org_id.clone(),
        });
        seen.insert(key);
    }
    folders.sort_by(|left, right| {
        left.org_id
            .cmp(&right.org_id)
            .then(left.path.cmp(&right.path))
            .then(left.uid.cmp(&right.uid))
    });
    Ok(folders)
}

/// Purpose: implementation note.
pub(crate) fn build_datasource_inventory_record(
    datasource: &Map<String, Value>,
    org: &Map<String, Value>,
) -> DatasourceInventoryItem {
    DatasourceInventoryItem {
        uid: string_field(datasource, "uid", ""),
        name: string_field(datasource, "name", ""),
        datasource_type: string_field(datasource, "type", ""),
        access: string_field(datasource, "access", ""),
        url: string_field(datasource, "url", ""),
        is_default: if datasource
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "true".to_string()
        } else {
            "false".to_string()
        },
        org: string_field(org, "name", DEFAULT_ORG_NAME),
        org_id: org
            .get("id")
            .map(|value| match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            })
            .unwrap_or_else(|| DEFAULT_ORG_ID.to_string()),
    }
}

/// Purpose: implementation note.
pub(crate) fn build_folder_path(folder: &Map<String, Value>, fallback_title: &str) -> String {
    let mut titles = Vec::new();
    if let Some(parents) = folder.get("parents").and_then(Value::as_array) {
        for parent in parents {
            if let Some(parent_object) = parent.as_object() {
                let title = string_field(parent_object, "title", "");
                if !title.is_empty() {
                    titles.push(title);
                }
            }
        }
    }
    let title = string_field(folder, "title", fallback_title);
    if !title.is_empty() {
        titles.push(title);
    }
    if titles.is_empty() {
        fallback_title.to_string()
    } else {
        titles.join(" / ")
    }
}

#[cfg(test)]
fn parent_uid_from_folder(folder: &Map<String, Value>) -> Option<String> {
    folder
        .get("parents")
        .and_then(Value::as_array)
        .and_then(|parents| parents.last())
        .and_then(Value::as_object)
        .map(|parent| string_field(parent, "uid", ""))
        .filter(|uid| !uid.is_empty())
}

/// Purpose: implementation note.
#[cfg(test)]
pub(crate) fn build_folder_inventory_status(
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
    status.actual_parent_uid = parent_uid_from_folder(destination_folder);
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

/// format folder inventory status line.
pub(crate) fn format_folder_inventory_status_line(status: &FolderInventoryStatus) -> String {
    match status.kind {
        FolderInventoryStatusKind::Missing => format!(
            "Folder inventory missing uid={} title={} parentUid={} path={}",
            status.uid,
            status.expected_title,
            status.expected_parent_uid.as_deref().unwrap_or("-"),
            status.expected_path
        ),
        FolderInventoryStatusKind::Matches => format!(
            "Folder inventory matches uid={} title={} parentUid={} path={}",
            status.uid,
            status.expected_title,
            status.expected_parent_uid.as_deref().unwrap_or("-"),
            status.expected_path
        ),
        FolderInventoryStatusKind::Mismatch => format!(
            "Folder inventory mismatch uid={} expected(title={}, parentUid={}, path={}) actual(title={}, parentUid={}, path={})",
            status.uid,
            status.expected_title,
            status.expected_parent_uid.as_deref().unwrap_or("-"),
            status.expected_path,
            status.actual_title.as_deref().unwrap_or("-"),
            status.actual_parent_uid.as_deref().unwrap_or("-"),
            status.actual_path.as_deref().unwrap_or("-")
        ),
    }
}

/// collect folder inventory statuses with request.
#[cfg(test)]
pub(crate) fn collect_folder_inventory_statuses_with_request<F>(
    request_json: &mut F,
    folder_inventory: &[FolderInventoryItem],
) -> Result<Vec<FolderInventoryStatus>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut statuses = Vec::new();
    for folder in folder_inventory {
        let destination_folder =
            fetch_folder_if_exists_with_request(&mut *request_json, &folder.uid)?;
        statuses.push(build_folder_inventory_status(
            folder,
            destination_folder.as_ref(),
        ));
    }
    Ok(statuses)
}

/// fetch dashboard with request.
pub(crate) fn fetch_dashboard_with_request<F>(mut request_json: F, uid: &str) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(
        Method::GET,
        &format!("/api/dashboards/uid/{uid}"),
        &[],
        None,
    )? {
        Some(value) => {
            let object = value_as_object(
                &value,
                &format!("Unexpected dashboard payload for UID {uid}."),
            )?;
            if !object.contains_key("dashboard") {
                return Err(message(format!(
                    "Unexpected dashboard payload for UID {uid}."
                )));
            }
            Ok(value)
        }
        None => Err(message(format!(
            "Unexpected empty dashboard payload for UID {uid}."
        ))),
    }
}

/// fetch dashboard.
pub fn fetch_dashboard(client: &JsonHttpClient, uid: &str) -> Result<Value> {
    fetch_dashboard_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        uid,
    )
}

/// fetch dashboard permissions with request.
pub(crate) fn fetch_dashboard_permissions_with_request<F>(
    mut request_json: F,
    uid: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let path = format!("/api/dashboards/uid/{uid}/permissions");
    match request_json(Method::GET, &path, &[], None)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| {
                value_as_object(
                    &item,
                    &format!("Unexpected dashboard permissions payload for UID {uid}."),
                )
                .cloned()
            })
            .collect(),
        Some(_) => Err(message(format!(
            "Unexpected dashboard permissions payload for UID {uid}."
        ))),
        None => Ok(Vec::new()),
    }
}

/// fetch folder permissions with request.
pub(crate) fn fetch_folder_permissions_with_request<F>(
    mut request_json: F,
    uid: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let path = format!("/api/folders/{uid}/permissions");
    match request_json(Method::GET, &path, &[], None)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| {
                value_as_object(
                    &item,
                    &format!("Unexpected folder permissions payload for UID {uid}."),
                )
                .cloned()
            })
            .collect(),
        Some(_) => Err(message(format!(
            "Unexpected folder permissions payload for UID {uid}."
        ))),
        None => Ok(Vec::new()),
    }
}

/// fetch dashboard if exists with request.
pub(crate) fn fetch_dashboard_if_exists_with_request<F>(
    mut request_json: F,
    uid: &str,
) -> Result<Option<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match fetch_dashboard_with_request(&mut request_json, uid) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.status_code() == Some(404) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Purpose: implementation note.
pub(crate) fn import_dashboard_request_with_request<F>(
    mut request_json: F,
    payload: &Value,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::POST, "/api/dashboards/db", &[], Some(payload))? {
        Some(value) => {
            value_as_object(&value, "Unexpected dashboard import response from Grafana.")?;
            Ok(value)
        }
        None => Err(message(
            "Unexpected empty dashboard import response from Grafana.",
        )),
    }
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn import_dashboard_request(client: &JsonHttpClient, payload: &Value) -> Result<Value> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

    import_dashboard_request_with_request(
        |method, path, params, request_payload| {
            client.request_json(method, path, params, request_payload)
        },
        payload,
    )
}

/// Purpose: implementation note.
pub(crate) fn list_datasources_with_request<F>(
    mut request_json: F,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| {
                value_as_object(&item, "Unexpected datasource payload from Grafana.").cloned()
            })
            .collect(),
        Some(_) => Err(message("Unexpected datasource list response from Grafana.")),
        None => Ok(Vec::new()),
    }
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn list_datasources(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    list_datasources_with_request(|method, path, params, payload| {
        client.request_json(method, path, params, payload)
    })
}
