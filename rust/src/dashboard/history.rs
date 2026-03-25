use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};

use super::{
    fetch_dashboard_with_request, import_dashboard_request_with_request, DEFAULT_DASHBOARD_TITLE,
};

pub(crate) const BROWSE_HISTORY_RESTORE_MESSAGE: &str =
    "Restored by grafana-utils dashboard browse";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DashboardHistoryVersion {
    pub version: i64,
    pub created: String,
    pub created_by: String,
    pub message: String,
}

pub(crate) fn list_dashboard_history_versions_with_request<F>(
    mut request_json: F,
    uid: &str,
    limit: usize,
) -> Result<Vec<DashboardHistoryVersion>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let path = format!("/api/dashboards/uid/{uid}/versions");
    let params = vec![("limit".to_string(), limit.to_string())];
    let response = request_json(Method::GET, &path, &params, None)?;
    let Some(value) = response else {
        return Ok(Vec::new());
    };
    let versions = match value {
        Value::Array(items) => items,
        Value::Object(object) => object
            .get("versions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => {
            return Err(message(
                "Unexpected dashboard versions payload from Grafana.",
            ))
        }
    };
    Ok(versions
        .into_iter()
        .filter_map(|item| item.as_object().cloned())
        .map(|item| DashboardHistoryVersion {
            version: item
                .get("version")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            created: item
                .get("created")
                .map(display_value)
                .unwrap_or_else(|| "-".to_string()),
            created_by: string_field(&item, "createdBy", "-"),
            message: string_field(&item, "message", ""),
        })
        .collect())
}

pub(crate) fn restore_dashboard_history_version_with_request<F>(
    mut request_json: F,
    uid: &str,
    version: i64,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let current_payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let current_object = value_as_object(
        &current_payload,
        "Unexpected current dashboard payload for history restore.",
    )?;
    let current_folder_uid = current_object
        .get("meta")
        .and_then(Value::as_object)
        .map(|meta| string_field(meta, "folderUid", ""))
        .filter(|value| !value.is_empty());

    let version_path = format!("/api/dashboards/uid/{uid}/versions/{version}");
    let version_payload =
        request_json(Method::GET, &version_path, &[], None)?.ok_or_else(|| {
            message(format!(
                "Dashboard history version {version} was not returned."
            ))
        })?;
    let version_object = value_as_object(
        &version_payload,
        "Unexpected dashboard history version payload from Grafana.",
    )?;
    let dashboard = version_object
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            message("Dashboard history version payload did not include dashboard data.")
        })?;
    let mut dashboard = dashboard.clone();
    dashboard.insert("id".to_string(), Value::Null);
    dashboard.insert("uid".to_string(), Value::String(uid.to_string()));
    dashboard.remove("version");
    if !dashboard.contains_key("title") {
        dashboard.insert(
            "title".to_string(),
            Value::String(DEFAULT_DASHBOARD_TITLE.to_string()),
        );
    }

    let mut import_payload = Map::new();
    import_payload.insert("dashboard".to_string(), Value::Object(dashboard));
    import_payload.insert("overwrite".to_string(), Value::Bool(true));
    import_payload.insert(
        "message".to_string(),
        Value::String(format!(
            "{} to version {}",
            BROWSE_HISTORY_RESTORE_MESSAGE, version
        )),
    );
    if let Some(folder_uid) = current_folder_uid {
        import_payload.insert("folderUid".to_string(), Value::String(folder_uid));
    }
    let _ =
        import_dashboard_request_with_request(&mut request_json, &Value::Object(import_payload))?;
    Ok(())
}

fn display_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}
