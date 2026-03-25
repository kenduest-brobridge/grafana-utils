use serde_json::json;

use crate::common::Result;

use super::delete_support::{DashboardDeleteTarget, DeletePlan, FolderDeleteTarget};

pub(crate) fn render_delete_dry_run_text(plan: &DeletePlan) -> Vec<String> {
    let mut lines = Vec::new();
    for item in &plan.dashboards {
        lines.push(format!(
            "Dry-run dashboard delete uid={} name={} folderPath={} action=delete",
            item.uid, item.title, item.folder_path
        ));
    }
    for item in &plan.folders {
        lines.push(format!(
            "Dry-run folder delete uid={} title={} path={} action=delete-folder",
            item.uid, item.title, item.path
        ));
    }
    lines.push(format!(
        "Dry-run matched {} dashboard(s){}",
        plan.dashboards.len(),
        if plan.folders.is_empty() {
            String::new()
        } else {
            format!(" and {} folder(s)", plan.folders.len())
        }
    ));
    lines
}

pub(crate) fn render_delete_dry_run_json(plan: &DeletePlan) -> Result<String> {
    let items = plan
        .dashboards
        .iter()
        .map(|item| {
            json!({
                "kind": "dashboard",
                "uid": item.uid,
                "name": item.title,
                "folderPath": item.folder_path,
                "action": "delete",
            })
        })
        .chain(plan.folders.iter().map(|item| {
            json!({
                "kind": "folder",
                "uid": item.uid,
                "name": item.title,
                "folderPath": item.path,
                "action": "delete-folder",
            })
        }))
        .collect::<Vec<_>>();
    Ok(serde_json::to_string_pretty(&json!({
        "selector": {
            "uid": plan.selector_uid,
            "path": plan.selector_path,
        },
        "deleteFolders": plan.delete_folders,
        "items": items,
        "summary": {
            "dashboardCount": plan.dashboards.len(),
            "folderCount": plan.folders.len(),
        }
    }))?)
}

pub(crate) fn render_delete_dry_run_table(plan: &DeletePlan, include_header: bool) -> Vec<String> {
    let mut rows = Vec::new();
    for item in &plan.dashboards {
        rows.push(vec![
            "dashboard".to_string(),
            item.uid.clone(),
            item.title.clone(),
            item.folder_path.clone(),
            "delete".to_string(),
        ]);
    }
    for item in &plan.folders {
        rows.push(vec![
            "folder".to_string(),
            item.uid.clone(),
            item.title.clone(),
            item.path.clone(),
            "delete-folder".to_string(),
        ]);
    }
    render_table(
        &["KIND", "UID", "NAME", "FOLDER_PATH", "ACTION"],
        &rows,
        include_header,
    )
}

fn render_table(headers: &[&str], rows: &[Vec<String>], include_header: bool) -> Vec<String> {
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .fold(header.len(), |width, value| width.max(value.len()))
        })
        .collect();
    let render_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };
    let mut lines = Vec::new();
    if include_header {
        lines.push(render_row(
            &headers
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>(),
        ));
        lines.push(render_row(
            &widths
                .iter()
                .map(|width| "-".repeat(*width))
                .collect::<Vec<_>>(),
        ));
    }
    for row in rows {
        lines.push(render_row(row));
    }
    lines
}

pub(crate) fn format_live_dashboard_delete_line(item: &DashboardDeleteTarget) -> String {
    format!(
        "Deleted dashboard uid={} name={} folderPath={}",
        item.uid, item.title, item.folder_path
    )
}

pub(crate) fn format_live_folder_delete_line(item: &FolderDeleteTarget) -> String {
    format!(
        "Deleted folder uid={} title={} path={}",
        item.uid, item.title, item.path
    )
}
