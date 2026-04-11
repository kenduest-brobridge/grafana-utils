use crate::common::{message, value_as_object, Result};
use reqwest::Method;
use serde_json::{Map, Value};

use super::history_artifacts::{
    build_dashboard_history_list_document_from_export_document,
    ensure_history_artifact_uid_matches, load_dashboard_history_export_document,
    run_dashboard_history_list_from_import_dir,
};
use super::history_render::render_dashboard_history_list_output;
use super::history_types::{
    DashboardHistoryExportDocument, DashboardHistoryExportVersion, DashboardHistoryListDocument,
    DashboardHistoryVersion, DashboardRestorePreview,
};
use super::{
    fetch_dashboard_with_request, import_dashboard_request_with_request, string_field,
    tool_version, write_json_document, HistoryExportArgs, HistoryListArgs, DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_UID, TOOL_SCHEMA_VERSION,
};

fn display_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
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

pub(crate) fn build_dashboard_history_list_document_with_request<F>(
    mut request_json: F,
    uid: &str,
    limit: usize,
) -> Result<DashboardHistoryListDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let versions = list_dashboard_history_versions_with_request(&mut request_json, uid, limit)?;
    Ok(DashboardHistoryListDocument {
        kind: super::history_types::DASHBOARD_HISTORY_LIST_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        dashboard_uid: uid.to_string(),
        version_count: versions.len(),
        versions,
    })
}

pub(crate) fn fetch_dashboard_history_version_data_with_request<F>(
    mut request_json: F,
    uid: &str,
    version: i64,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
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
    version_object
        .get("data")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Dashboard history version payload did not include dashboard data."))
}

pub(crate) fn build_dashboard_restore_preview_with_request<F>(
    mut request_json: F,
    uid: &str,
    version: i64,
) -> Result<DashboardRestorePreview>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let current_payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let current_object = value_as_object(
        &current_payload,
        "Unexpected current dashboard payload for history restore.",
    )?;
    let current_dashboard = current_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard data."))?;
    let current_version = current_dashboard
        .get("version")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let current_title = string_field(current_dashboard, "title", DEFAULT_DASHBOARD_TITLE);
    let target_folder_uid = current_object
        .get("meta")
        .and_then(Value::as_object)
        .map(|meta| string_field(meta, "folderUid", DEFAULT_FOLDER_UID))
        .filter(|value| !value.is_empty() && value != DEFAULT_FOLDER_UID);
    let restored_dashboard =
        fetch_dashboard_history_version_data_with_request(&mut request_json, uid, version)?;
    let restored_title = string_field(&restored_dashboard, "title", DEFAULT_DASHBOARD_TITLE);
    Ok(DashboardRestorePreview {
        current_version,
        current_title,
        restored_title,
        target_folder_uid,
    })
}

pub(crate) fn build_dashboard_history_export_document_with_request<F>(
    mut request_json: F,
    uid: &str,
    limit: usize,
) -> Result<DashboardHistoryExportDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let current_payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let current_object = value_as_object(
        &current_payload,
        "Unexpected current dashboard payload for history export.",
    )?;
    let current_dashboard = current_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard data."))?;
    let current_version = current_dashboard
        .get("version")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let current_title = string_field(current_dashboard, "title", DEFAULT_DASHBOARD_TITLE);
    let versions = list_dashboard_history_versions_with_request(&mut request_json, uid, limit)?;
    let versions = versions
        .into_iter()
        .map(|version| {
            let dashboard = Value::Object(fetch_dashboard_history_version_data_with_request(
                &mut request_json,
                uid,
                version.version,
            )?);
            Ok(DashboardHistoryExportVersion {
                version: version.version,
                created: version.created,
                created_by: version.created_by,
                message: version.message,
                dashboard,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(DashboardHistoryExportDocument {
        kind: super::history_types::DASHBOARD_HISTORY_EXPORT_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        dashboard_uid: uid.to_string(),
        current_version,
        current_title,
        version_count: versions.len(),
        versions,
    })
}

pub(crate) fn export_dashboard_history_with_request<F>(
    mut request_json: F,
    args: &HistoryExportArgs,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.output.exists() && !args.overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            args.output.display()
        )));
    }
    let document = build_dashboard_history_export_document_with_request(
        &mut request_json,
        &args.dashboard_uid,
        args.limit,
    )?;
    write_json_document(&document, &args.output)?;
    Ok(())
}

pub(crate) fn run_dashboard_history_list<F>(
    mut request_json: F,
    args: &HistoryListArgs,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(input_path) = &args.input {
        let document = load_dashboard_history_export_document(input_path)?;
        if let Some(uid) = &args.dashboard_uid {
            ensure_history_artifact_uid_matches(uid, &document, input_path)?;
        }
        let list_document = build_dashboard_history_list_document_from_export_document(&document);
        return render_dashboard_history_list_output(&list_document, args.output_format);
    }

    if let Some(input_dir) = &args.input_dir {
        return run_dashboard_history_list_from_import_dir(input_dir, args);
    }

    let dashboard_uid = args.dashboard_uid.as_deref().ok_or_else(|| {
        message(
            "Dashboard history list requires --dashboard-uid unless --input or --input-dir is set.",
        )
    })?;
    let document = build_dashboard_history_list_document_with_request(
        &mut request_json,
        dashboard_uid,
        args.limit,
    )?;
    render_dashboard_history_list_output(&document, args.output_format)
}
