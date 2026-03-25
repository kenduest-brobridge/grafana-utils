use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};

use super::edit::BROWSE_EDIT_MESSAGE;
use super::{
    extract_dashboard_object, fetch_dashboard_with_request, import_dashboard_request_with_request,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExternalDashboardEditDraft {
    pub uid: String,
    pub title: String,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExternalDashboardEditReview {
    pub updated_payload: Value,
    pub summary_lines: Vec<String>,
}

pub(crate) fn fetch_external_dashboard_edit_draft_with_request<F>(
    mut request_json: F,
    uid: &str,
) -> Result<ExternalDashboardEditDraft>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let live_payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let object = value_as_object(&live_payload, "Unexpected dashboard payload for raw edit.")?;
    let dashboard = extract_dashboard_object(object)?.clone();
    let title = string_field(&dashboard, "title", uid);
    let folder_uid = object
        .get("meta")
        .and_then(Value::as_object)
        .map(|meta| string_field(meta, "folderUid", ""))
        .filter(|value| !value.is_empty());

    let mut payload = Map::new();
    payload.insert("dashboard".to_string(), Value::Object(dashboard));
    if let Some(folder_uid) = folder_uid {
        payload.insert("folderUid".to_string(), Value::String(folder_uid));
    }
    Ok(ExternalDashboardEditDraft {
        uid: uid.to_string(),
        title,
        payload: Value::Object(payload),
    })
}

pub(crate) fn open_dashboard_in_external_editor(
    draft: &ExternalDashboardEditDraft,
) -> Result<Value> {
    let path = temp_edit_path(&draft.uid);
    fs::write(&path, serde_json::to_string_pretty(&draft.payload)? + "\n")?;
    let result = (|| -> Result<Value> {
        run_editor_command(&path)?;
        let edited = fs::read_to_string(&path)?;
        let value: Value = serde_json::from_str(&edited).map_err(|error| {
            message(format!(
                "Edited dashboard JSON is invalid in {}: {error}",
                path.display()
            ))
        })?;
        validate_external_dashboard_edit_value(&value)?;
        Ok(value)
    })();
    let _ = fs::remove_file(&path);
    result
}

fn temp_edit_path(uid: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    env::temp_dir().join(format!("grafana-dashboard-{uid}-{timestamp}.json"))
}

pub(crate) fn review_external_dashboard_edit(
    original: &ExternalDashboardEditDraft,
    edited: &Value,
) -> Result<Option<ExternalDashboardEditReview>> {
    validate_external_dashboard_edit_value(edited)?;
    if edited == &original.payload {
        return Ok(None);
    }
    let summary_lines = build_external_dashboard_edit_summary(&original.payload, edited)?;
    Ok(Some(ExternalDashboardEditReview {
        updated_payload: edited.clone(),
        summary_lines,
    }))
}

pub(crate) fn apply_external_dashboard_edit_with_request<F>(
    mut request_json: F,
    updated_payload: &Value,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_external_dashboard_edit_value(updated_payload)?;
    let object = value_as_object(
        updated_payload,
        "Dashboard raw edit payload must be an object.",
    )?;
    let mut payload = object.clone();
    payload.insert("overwrite".to_string(), Value::Bool(true));
    payload.insert(
        "message".to_string(),
        Value::String(BROWSE_EDIT_MESSAGE.to_string()),
    );
    let _ = import_dashboard_request_with_request(&mut request_json, &Value::Object(payload))?;
    Ok(())
}

pub(crate) fn validate_external_dashboard_edit_value(value: &Value) -> Result<()> {
    let object = value_as_object(value, "Dashboard raw edit payload must be a JSON object.")?;
    let dashboard = extract_dashboard_object(object)?;
    if dashboard
        .get("uid")
        .and_then(Value::as_str)
        .filter(|uid| !uid.is_empty())
        .is_none()
    {
        return Err(message(
            "Dashboard raw edit payload must include dashboard.uid.",
        ));
    }
    if object.contains_key("overwrite") {
        return Err(message(
            "Dashboard raw edit payload must not include overwrite; the browser adds that automatically.",
        ));
    }
    if let Some(folder_uid) = object.get("folderUid") {
        if !folder_uid.is_string() {
            return Err(message(
                "Dashboard raw edit payload folderUid must be a string when present.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn build_external_dashboard_edit_summary(
    original: &Value,
    edited: &Value,
) -> Result<Vec<String>> {
    let original_object = value_as_object(
        original,
        "Original dashboard raw edit payload must be an object.",
    )?;
    let edited_object = value_as_object(
        edited,
        "Edited dashboard raw edit payload must be an object.",
    )?;
    let original_dashboard = extract_dashboard_object(original_object)?;
    let edited_dashboard = extract_dashboard_object(edited_object)?;
    let original_title = string_field(original_dashboard, "title", "");
    let edited_title = string_field(edited_dashboard, "title", "");
    let original_uid = string_field(original_dashboard, "uid", "");
    let edited_uid = string_field(edited_dashboard, "uid", "");
    let original_tags = join_tags(original_dashboard.get("tags"));
    let edited_tags = join_tags(edited_dashboard.get("tags"));
    let original_folder_uid = original_object
        .get("folderUid")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let edited_folder_uid = edited_object
        .get("folderUid")
        .and_then(Value::as_str)
        .unwrap_or("-");

    let mut lines = vec![
        format!("Raw JSON edit review for dashboard uid={edited_uid}"),
        format!(
            "Title: {} -> {}",
            display_text(&original_title),
            display_text(&edited_title)
        ),
        format!(
            "UID: {} -> {}",
            display_text(&original_uid),
            display_text(&edited_uid)
        ),
        format!(
            "Folder UID: {} -> {}",
            display_text(original_folder_uid),
            display_text(edited_folder_uid)
        ),
        format!(
            "Tags: {} -> {}",
            display_text(&original_tags),
            display_text(&edited_tags)
        ),
    ];
    if original != edited {
        lines.push("Apply this raw JSON change to Grafana? [y/N]".to_string());
    }
    Ok(lines)
}

fn join_tags(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn display_text(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn run_editor_command(path: &Path) -> Result<()> {
    let editor = env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".to_string());
    let mut parts = editor.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| message("Could not resolve an external editor command."))?;
    let mut command = Command::new(program);
    for part in parts {
        command.arg(part);
    }
    let status = command.arg(PathBuf::from(path)).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(message(format!(
            "External editor exited with status {status}."
        )))
    }
}
