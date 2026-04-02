#![cfg(feature = "tui")]
use serde_json::Value;

use crate::common::{message, Result};
use reqwest::Method;

use super::browse_edit_dialog::EditDialogState;
use super::browse_history_dialog::HistoryDialogState;
use super::browse_support::{
    fetch_dashboard_view_lines_with_request, load_dashboard_browse_document_for_args,
    DashboardBrowseDocument, DashboardBrowseNode, DashboardBrowseNodeKind,
};
use super::delete_support::{build_delete_plan_with_request, DeletePlan};
use super::edit::{
    apply_dashboard_edit_with_request, fetch_dashboard_edit_draft_with_request,
    resolve_folder_uid_for_path, DashboardEditUpdate,
};
use super::edit_external::{
    apply_external_dashboard_edit_with_request, fetch_external_dashboard_edit_draft_with_request,
    open_dashboard_in_external_editor, review_external_dashboard_edit,
};
use super::edit_prompt::prompt_yes_no;
use super::history::{
    list_dashboard_history_versions_with_request, restore_dashboard_history_version_with_request,
};
use super::live::{delete_dashboard_request_with_request, delete_folder_request_with_request};
use super::{BrowseArgs, CommonCliArgs, DeleteArgs};

pub(crate) fn refresh_browser_document<F>(
    request_json: &mut F,
    args: &BrowseArgs,
) -> Result<DashboardBrowseDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    load_dashboard_browse_document_for_args(request_json, args)
}

pub(crate) fn load_live_detail_lines<F>(
    request_json: &mut F,
    node: &DashboardBrowseNode,
) -> Result<Vec<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    fetch_dashboard_view_lines_with_request(request_json, node)
}

pub(crate) fn begin_dashboard_edit<F>(
    request_json: &mut F,
    document: &DashboardBrowseDocument,
    node: &DashboardBrowseNode,
) -> Result<EditDialogState>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let draft = fetch_dashboard_edit_draft_with_request(request_json, node)?;
    Ok(EditDialogState::from_draft(draft, document))
}

pub(crate) fn begin_dashboard_history<F>(
    request_json: &mut F,
    node: &DashboardBrowseNode,
) -> Result<HistoryDialogState>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if node.kind != DashboardBrowseNodeKind::Dashboard {
        return Err(message(
            "Dashboard history is only available for dashboard rows.",
        ));
    }
    let uid = node
        .uid
        .as_deref()
        .ok_or_else(|| message("Dashboard history requires a dashboard UID."))?;
    let versions = list_dashboard_history_versions_with_request(request_json, uid, 20)?;
    Ok(HistoryDialogState::new(
        uid.to_string(),
        node.title.clone(),
        versions,
    ))
}

pub(crate) fn restore_dashboard_history_version<F>(
    request_json: &mut F,
    uid: &str,
    version: i64,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    restore_dashboard_history_version_with_request(request_json, uid, version)
}

pub(crate) fn apply_dashboard_edit_save<F>(
    request_json: &mut F,
    document: &DashboardBrowseDocument,
    draft: &super::edit::DashboardEditDraft,
    update: &DashboardEditUpdate,
) -> Result<bool>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if *update == DashboardEditUpdate::default() {
        return Ok(false);
    }
    let destination_folder_uid = update
        .folder_path
        .as_deref()
        .map(|path| resolve_folder_uid_for_path(document, path))
        .transpose()?;
    apply_dashboard_edit_with_request(
        request_json,
        draft,
        update,
        destination_folder_uid.as_deref(),
    )?;
    Ok(true)
}

pub(crate) fn run_external_dashboard_edit<F>(
    request_json: &mut F,
    node: &DashboardBrowseNode,
) -> Result<(String, bool)>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let draft = fetch_external_dashboard_edit_draft_with_request(
        &mut *request_json,
        node.uid.as_deref().unwrap_or_default(),
    )?;
    let edited = open_dashboard_in_external_editor(&draft)?;
    let Some(review) = review_external_dashboard_edit(&draft, &edited)? else {
        println!("No raw JSON changes detected for {}.", draft.uid);
        return Ok((draft.uid, false));
    };
    for line in &review.summary_lines {
        println!("{line}");
    }
    if !prompt_yes_no("Confirm raw JSON apply [y/N]: ")? {
        return Ok((draft.uid, false));
    }
    apply_external_dashboard_edit_with_request(&mut *request_json, &review.updated_payload)?;
    Ok((draft.uid, true))
}

pub(crate) fn build_delete_preview<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    node: &DashboardBrowseNode,
    delete_folders: bool,
) -> Result<DeletePlan>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    build_delete_plan_with_request(request_json, &build_delete_args(args, node, delete_folders))
}

pub(crate) fn execute_delete_plan_with_request<F>(
    request_json: &mut F,
    plan: &DeletePlan,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    for item in &plan.dashboards {
        let _ = delete_dashboard_request_with_request(&mut *request_json, &item.uid)?;
    }
    for item in &plan.folders {
        let _ = delete_folder_request_with_request(&mut *request_json, &item.uid)?;
    }
    Ok(plan.dashboards.len() + plan.folders.len())
}

pub(crate) fn delete_status_message(node: &DashboardBrowseNode, delete_folders: bool) -> String {
    match node.kind {
        DashboardBrowseNodeKind::Org => {
            "Org header rows are browse-only. Select a folder or dashboard row.".to_string()
        }
        DashboardBrowseNodeKind::Dashboard => {
            format!(
                "Previewing dashboard delete for {}. Press y to confirm.",
                node.title
            )
        }
        DashboardBrowseNodeKind::Folder if delete_folders => format!(
            "Previewing subtree delete for {} including folders. Press y to confirm.",
            node.path
        ),
        DashboardBrowseNodeKind::Folder => format!(
            "Previewing dashboard-only subtree delete for {}. Press y to confirm.",
            node.path
        ),
    }
}

fn build_delete_args(
    args: &BrowseArgs,
    node: &DashboardBrowseNode,
    delete_folders: bool,
) -> DeleteArgs {
    DeleteArgs {
        common: CommonCliArgs {
            color: args.common.color,
            profile: None,
            url: args.common.url.clone(),
            api_token: args.common.api_token.clone(),
            username: args.common.username.clone(),
            password: args.common.password.clone(),
            prompt_password: args.common.prompt_password,
            prompt_token: args.common.prompt_token,
            timeout: args.common.timeout,
            verify_ssl: args.common.verify_ssl,
        },
        page_size: args.page_size,
        org_id: if args.all_orgs {
            node.org_id.parse::<i64>().ok()
        } else {
            args.org_id
        },
        uid: (node.kind == DashboardBrowseNodeKind::Dashboard)
            .then(|| node.uid.as_deref().unwrap_or_default().to_string()),
        path: (node.kind == DashboardBrowseNodeKind::Folder).then(|| node.path.clone()),
        delete_folders,
        yes: false,
        interactive: false,
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    }
}
