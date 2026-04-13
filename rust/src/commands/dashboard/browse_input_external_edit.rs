#![cfg(feature = "tui")]
use crossterm::event::KeyEvent;
use reqwest::Method;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{message, Result};

use super::browse_input_shared::{redraw_browser, scoped_org_client};
use crate::dashboard::browse_actions::{
    apply_external_dashboard_edit, begin_external_dashboard_edit, refresh_browser_document,
};
use crate::dashboard::browse_external_edit_dialog::{
    ExternalEditDialogAction, ExternalEditErrorAction, ExternalEditErrorState,
};
use crate::dashboard::browse_state::{BrowserState, CompletionNotice};
use crate::dashboard::browse_support::DashboardBrowseNodeKind;
use crate::dashboard::browse_terminal::TerminalSession;
use crate::dashboard::import::collect_import_dry_run_report_with_request;
use crate::dashboard::{BrowseArgs, CommonCliArgs, DashboardImportInputFormat, ImportArgs};

pub(super) fn handle_external_edit_error_key<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    session: &mut TerminalSession,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(action) = state
        .pending_external_edit_error
        .as_ref()
        .map(|dialog| dialog.handle_key(key))
    else {
        return Ok(());
    };
    match action {
        ExternalEditErrorAction::Continue => {}
        ExternalEditErrorAction::Close => {
            let uid = state
                .pending_external_edit_error
                .as_ref()
                .map(|dialog| dialog.uid.clone())
                .unwrap_or_else(|| "dashboard".to_string());
            state.pending_external_edit_error = None;
            state.status = format!("Aborted raw JSON edit for {}.", uid);
        }
        ExternalEditErrorAction::Retry => {
            state.pending_external_edit_error = None;
            run_selected_external_edit(request_json, args, session, state)?;
        }
    }
    Ok(())
}

pub(super) fn handle_external_edit_dialog_key<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    session: &mut TerminalSession,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(action) = state
        .pending_external_edit
        .as_mut()
        .map(|dialog| dialog.handle_key(key))
    else {
        return Ok(());
    };
    match action {
        ExternalEditDialogAction::Continue => {}
        ExternalEditDialogAction::Close => {
            let uid = state
                .pending_external_edit
                .as_ref()
                .map(|dialog| dialog.uid.clone())
                .unwrap_or_else(|| "dashboard".to_string());
            state.pending_external_edit = None;
            state.status = format!("Discarded raw JSON edit review for {}.", uid);
        }
        ExternalEditDialogAction::SaveDraft(save_path) => {
            let Some(dialog) = state.pending_external_edit.as_ref() else {
                return Ok(());
            };
            let uid = dialog.uid.clone();
            let updated_payload = dialog.updated_payload.clone();
            if let Some(dialog) = state.pending_external_edit.as_mut() {
                dialog.set_busy_message(format!(
                    "Working... writing draft file to {}.",
                    save_path.display()
                ));
            }
            state.status = format!("Writing draft file for {}...", uid);
            redraw_browser(session, state)?;
            if let Some(parent) = save_path
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
            {
                fs::create_dir_all(parent)?;
            }
            fs::write(
                &save_path,
                serde_json::to_string_pretty(&updated_payload)? + "\n",
            )?;
            state.pending_external_edit = None;
            state.status = format!(
                "Wrote raw JSON draft for {} to {}.",
                uid,
                save_path.display()
            );
        }
        ExternalEditDialogAction::PreviewDryRun => {
            let payload = state
                .pending_external_edit
                .as_ref()
                .map(|dialog| dialog.updated_payload.clone())
                .ok_or_else(|| message("Raw JSON edit review state disappeared."))?;
            let uid = state
                .pending_external_edit
                .as_ref()
                .map(|dialog| dialog.uid.clone())
                .unwrap_or_else(|| "dashboard".to_string());
            if let Some(dialog) = state.pending_external_edit.as_mut() {
                dialog.set_busy_message(format!("Working... refreshing live preview for {}.", uid));
            }
            state.status = format!("Refreshing live preview for {}...", uid);
            redraw_browser(session, state)?;
            let Some(node) = state.selected_node().cloned() else {
                return Ok(());
            };
            let preview_lines = if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                preview_external_edit_dry_run(&mut scoped, args, &payload)?
            } else {
                preview_external_edit_dry_run(request_json, args, &payload)?
            };
            if let Some(dialog) = state.pending_external_edit.as_mut() {
                dialog.clear_busy_message();
                dialog.preview_lines = Some(preview_lines);
            }
            state.status = format!("Refreshed live preview for {}.", uid);
        }
        ExternalEditDialogAction::ApplyLive => {
            let payload = state
                .pending_external_edit
                .as_ref()
                .map(|dialog| dialog.updated_payload.clone())
                .ok_or_else(|| message("Raw JSON edit review state disappeared."))?;
            let uid = state
                .pending_external_edit
                .as_ref()
                .map(|dialog| dialog.uid.clone())
                .unwrap_or_else(|| "dashboard".to_string());
            if let Some(dialog) = state.pending_external_edit.as_mut() {
                dialog.set_busy_message(format!("Working... applying live edit for {}.", uid));
            }
            state.status = format!("Applying live edit for {}...", uid);
            redraw_browser(session, state)?;
            let Some(node) = state.selected_node().cloned() else {
                return Ok(());
            };
            state.pending_external_edit = None;
            if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                apply_external_dashboard_edit(&mut scoped, &payload)?;
            } else {
                apply_external_dashboard_edit(request_json, &payload)?;
            }
            let document = refresh_browser_document(request_json, args)?;
            state.replace_document(document);
            state.status = format!("Applied live edit for dashboard {}.", uid);
            state.completion_notice = Some(CompletionNotice {
                title: "Applied".to_string(),
                body: format!("Updated live dashboard {} successfully.", uid),
            });
            super::ensure_selected_dashboard_view(request_json, args, state, false)?;
        }
    }
    Ok(())
}

pub(super) fn run_selected_external_edit<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    session: &mut TerminalSession,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Org | DashboardBrowseNodeKind::Folder => {
            state.status = "Raw JSON edit is only available for dashboard rows.".to_string();
        }
        DashboardBrowseNodeKind::Dashboard => {
            session.suspend()?;
            let raw_result = if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                begin_external_dashboard_edit(&mut scoped, &node)
            } else {
                begin_external_dashboard_edit(request_json, &node)
            };
            session.resume()?;
            match raw_result {
                Ok(Some(dialog)) => {
                    let uid = dialog.uid.clone();
                    state.status =
                        format!("Preparing live preview for raw JSON edit on {}...", uid);
                    let preview_lines = if let Some(client) = scoped_org_client(args, &node)? {
                        let mut scoped = |method: Method,
                                          path: &str,
                                          params: &[(String, String)],
                                          payload: Option<&Value>|
                         -> Result<Option<Value>> {
                            client.request_json(method, path, params, payload)
                        };
                        preview_external_edit_dry_run(&mut scoped, args, &dialog.updated_payload)?
                    } else {
                        preview_external_edit_dry_run(request_json, args, &dialog.updated_payload)?
                    };
                    let mut dialog = dialog;
                    dialog.preview_lines = Some(preview_lines);
                    state.pending_external_edit = Some(dialog);
                    state.status = format!(
                        "Review raw JSON edit for {}. Preview is ready. a applies live, w opens draft filename input, q discards.",
                        uid
                    );
                }
                Ok(None) => {
                    state.status =
                        format!("Raw JSON edit cancelled or unchanged for {}.", node.title);
                }
                Err(error) => {
                    state.pending_external_edit_error = Some(ExternalEditErrorState::new(
                        node.uid.clone().unwrap_or_else(|| node.title.clone()),
                        node.title.clone(),
                        error.to_string(),
                    ));
                    state.status = format!(
                        "Raw JSON edit failed for {}. Use r to retry or q to abort.",
                        node.title
                    );
                }
            }
        }
    }
    Ok(())
}

pub(super) fn preview_external_edit_temp_dir(uid: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "grafana-dashboard-browse-preview-{}-{}-{}",
        process::id(),
        uid,
        timestamp
    ))
}

pub(super) fn build_external_edit_import_args(args: &BrowseArgs, input_dir: PathBuf) -> ImportArgs {
    ImportArgs {
        common: CommonCliArgs {
            color: args.common.color,
            profile: args.common.profile.clone(),
            url: args.common.url.clone(),
            api_token: args.common.api_token.clone(),
            username: args.common.username.clone(),
            password: args.common.password.clone(),
            prompt_password: args.common.prompt_password,
            prompt_token: args.common.prompt_token,
            timeout: args.common.timeout,
            verify_ssl: args.common.verify_ssl,
        },
        org_id: None,
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        input_dir,
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: true,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: false,
        strict_schema: false,
        target_schema_version: None,
        import_message: "Preview raw JSON edit from dashboard browse".to_string(),
        interactive: false,
        dry_run: true,
        table: true,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        list_columns: false,
        progress: false,
        verbose: false,
    }
}

pub(super) fn preview_external_edit_dry_run<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    updated_payload: &Value,
) -> Result<Vec<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let uid = updated_payload
        .get("dashboard")
        .and_then(Value::as_object)
        .and_then(|dashboard| dashboard.get("uid"))
        .and_then(Value::as_str)
        .unwrap_or("dashboard");
    let temp_dir = preview_external_edit_temp_dir(uid);
    fs::create_dir_all(&temp_dir)?;
    let result = (|| -> Result<Vec<String>> {
        let staged_path = temp_dir.join("dashboard.json");
        fs::write(
            &staged_path,
            serde_json::to_string_pretty(updated_payload)? + "\n",
        )?;
        let import_args = build_external_edit_import_args(args, temp_dir.clone());
        let report = collect_import_dry_run_report_with_request(&mut *request_json, &import_args)?;
        Ok(render_external_edit_preview_lines(
            &report.mode,
            &report.dashboard_records,
        ))
    })();
    let _ = fs::remove_dir_all(&temp_dir);
    result
}

pub(super) fn render_external_edit_preview_lines(
    mode: &str,
    dashboard_records: &[[String; 8]],
) -> Vec<String> {
    let mut lines = vec![
        "Preview only. Nothing has been written to Grafana yet.".to_string(),
        format!("Mode: {mode}"),
    ];
    if dashboard_records.is_empty() {
        lines.push("No dashboard changes were staged for import preview.".to_string());
        return lines;
    }
    for row in dashboard_records {
        let uid = row[0].as_str();
        let destination = row[1].as_str();
        let action = row[2].as_str();
        let folder_path = if row[3].trim().is_empty() {
            "-"
        } else {
            row[3].as_str()
        };
        lines.push(String::new());
        lines.push(format!("Dashboard: {uid}"));
        lines.push(format!("Destination: {destination}"));
        lines.push(format!("Action: {action}"));
        lines.push(format!("Folder: {folder_path}"));
        if !row[6].trim().is_empty() {
            lines.push(format!("Reason: {}", row[6]));
        }
    }
    lines
}
