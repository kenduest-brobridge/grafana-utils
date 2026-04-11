use crate::common::{message, render_json_value, string_field, value_as_object, Result};
use crate::tabular_output::render_yaml;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use reqwest::Method;
use serde_json::{Map, Value};
use std::io::{self, IsTerminal};

use super::history_live::{
    build_dashboard_restore_preview_with_request,
    fetch_dashboard_history_version_data_with_request,
    list_dashboard_history_versions_with_request,
};
use super::history_render::{
    render_dashboard_history_restore_table, render_dashboard_history_restore_text,
};
use super::history_types::{
    DashboardHistoryRestoreDocument, DashboardHistoryVersion, DashboardRestorePreview,
    HISTORY_RESTORE_PROMPT_LIMIT,
};
use super::{
    fetch_dashboard_with_request, import_dashboard_request_with_request, tool_version,
    HistoryOutputFormat, HistoryRestoreArgs, DASHBOARD_HISTORY_RESTORE_MESSAGE,
    DEFAULT_DASHBOARD_TITLE, TOOL_SCHEMA_VERSION,
};

fn prompt_dashboard_history_restore_version(
    uid: &str,
    versions: &[DashboardHistoryVersion],
) -> Result<Option<i64>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(message(
            "Dashboard history restore --prompt requires a TTY.",
        ));
    }
    if versions.is_empty() {
        return Err(message(format!(
            "Dashboard history restore --prompt did not find any versions for {uid}."
        )));
    }
    let labels = versions
        .iter()
        .map(|item| {
            let mut line = format!("v{}  {}  {}", item.version, item.created, item.created_by);
            if !item.message.is_empty() {
                line.push_str("  ");
                line.push_str(&item.message);
            }
            line
        })
        .collect::<Vec<_>>();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Select a dashboard history version to restore for {uid}"
        ))
        .items(&labels)
        .default(0)
        .interact_opt()
        .map_err(|error| message(format!("Dashboard history restore prompt failed: {error}")))?;
    Ok(selection.and_then(|index| versions.get(index).map(|item| item.version)))
}

fn confirm_dashboard_history_restore(uid: &str, version: i64) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Restore dashboard {uid} to version {version} and create a new latest revision?"
        ))
        .default(false)
        .interact_opt()
        .map(|choice| choice.unwrap_or(false))
        .map_err(|error| {
            message(format!(
                "Dashboard history restore confirmation failed: {error}"
            ))
        })
}

fn build_dashboard_history_restore_document(
    uid: &str,
    version: i64,
    preview: &DashboardRestorePreview,
    message_text: &str,
    dry_run: bool,
) -> DashboardHistoryRestoreDocument {
    DashboardHistoryRestoreDocument {
        kind: super::history_types::DASHBOARD_HISTORY_RESTORE_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        mode: if dry_run { "dry-run" } else { "live" }.to_string(),
        dashboard_uid: uid.to_string(),
        current_version: preview.current_version,
        restore_version: version,
        current_title: preview.current_title.clone(),
        restored_title: preview.restored_title.clone(),
        target_folder_uid: preview.target_folder_uid.clone(),
        creates_new_revision: true,
        message: message_text.to_string(),
    }
}

pub(crate) fn restore_dashboard_history_version_with_request_and_message<F>(
    mut request_json: F,
    uid: &str,
    version: i64,
    message_text: &str,
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
    let current_dashboard = current_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard data."))?;
    let current_id = current_dashboard
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard id."))?;
    let current_version = current_dashboard
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard version."))?;

    let mut dashboard =
        fetch_dashboard_history_version_data_with_request(&mut request_json, uid, version)?;
    dashboard.insert("id".to_string(), Value::from(current_id));
    dashboard.insert("uid".to_string(), Value::String(uid.to_string()));
    dashboard.insert("version".to_string(), Value::from(current_version));
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
        Value::String(message_text.to_string()),
    );
    if let Some(folder_uid) = current_folder_uid {
        import_payload.insert("folderUid".to_string(), Value::String(folder_uid));
    }
    let _ =
        import_dashboard_request_with_request(&mut request_json, &Value::Object(import_payload))?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn restore_dashboard_history_version_with_request<F>(
    request_json: F,
    uid: &str,
    version: i64,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    restore_dashboard_history_version_with_request_and_message(
        request_json,
        uid,
        version,
        &format!("{DASHBOARD_HISTORY_RESTORE_MESSAGE} to version {version}"),
    )
}

pub(crate) fn run_dashboard_history_restore<F>(
    mut request_json: F,
    args: &HistoryRestoreArgs,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let version = if let Some(version) = args.version {
        version
    } else if args.prompt {
        let versions = list_dashboard_history_versions_with_request(
            &mut request_json,
            &args.dashboard_uid,
            HISTORY_RESTORE_PROMPT_LIMIT,
        )?;
        let Some(version) =
            prompt_dashboard_history_restore_version(&args.dashboard_uid, &versions)?
        else {
            println!("Cancelled dashboard history restore.");
            return Ok(());
        };
        version
    } else {
        return Err(message(
            "Dashboard history restore requires --version unless --prompt is used.",
        ));
    };
    let preview = build_dashboard_restore_preview_with_request(
        &mut request_json,
        &args.dashboard_uid,
        version,
    )?;
    let message_text = args
        .message
        .clone()
        .unwrap_or_else(|| format!("{DASHBOARD_HISTORY_RESTORE_MESSAGE} to version {version}"));
    let document = build_dashboard_history_restore_document(
        &args.dashboard_uid,
        version,
        &preview,
        &message_text,
        args.dry_run,
    );
    let rendered = match args.output_format {
        HistoryOutputFormat::Text => render_dashboard_history_restore_text(&document),
        HistoryOutputFormat::Table => render_dashboard_history_restore_table(&document),
        HistoryOutputFormat::Json => render_json_value(&document)?.trim_end().to_string(),
        HistoryOutputFormat::Yaml => render_yaml(&document)?.trim_end().to_string(),
    };
    if args.dry_run {
        println!("{rendered}");
        return Ok(());
    }
    if args.prompt {
        println!("{rendered}");
        if !confirm_dashboard_history_restore(&args.dashboard_uid, version)? {
            println!("Cancelled dashboard history restore.");
            return Ok(());
        }
    } else if !args.yes {
        return Err(message(
            "Dashboard history restore requires --yes unless --dry-run or --prompt is set.",
        ));
    }
    restore_dashboard_history_version_with_request_and_message(
        &mut request_json,
        &args.dashboard_uid,
        version,
        &message_text,
    )?;
    println!("{rendered}");
    Ok(())
}
