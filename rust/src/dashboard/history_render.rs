use crate::common::{render_json_value, Result};
use crate::tabular_output::{render_table, render_yaml};
use serde_json::Value;

use super::history_types::{
    DashboardHistoryDiffDocument, DashboardHistoryInventoryDocument, DashboardHistoryListDocument,
    DashboardHistoryRestoreDocument, ResolvedHistoryDiffSide,
};
use super::HistoryOutputFormat;

pub(crate) fn render_dashboard_history_list_text(
    document: &DashboardHistoryListDocument,
) -> String {
    let mut lines = vec![format!(
        "Dashboard history: {} versions={}",
        document.dashboard_uid, document.version_count
    )];
    for item in &document.versions {
        let summary = if item.message.is_empty() {
            format!("  v{} {} {}", item.version, item.created, item.created_by)
        } else {
            format!(
                "  v{} {} {} {}",
                item.version, item.created, item.created_by, item.message
            )
        };
        lines.push(summary);
    }
    lines.join("\n")
}

pub(crate) fn render_dashboard_history_list_table(
    document: &DashboardHistoryListDocument,
) -> String {
    render_table(
        &["version", "created", "createdBy", "message"],
        &document
            .versions
            .iter()
            .map(|item| {
                vec![
                    item.version.to_string(),
                    item.created.clone(),
                    item.created_by.clone(),
                    item.message.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    )
    .join("\n")
}

pub(crate) fn render_dashboard_history_inventory_text(
    document: &DashboardHistoryInventoryDocument,
) -> String {
    let mut lines = vec![format!(
        "Dashboard history artifacts: count={}",
        document.artifact_count
    )];
    for item in &document.artifacts {
        let scope = item.scope.as_deref().unwrap_or("current");
        lines.push(format!(
            "  {} title={} current-version={} versions={} scope={} path={}",
            item.dashboard_uid,
            item.current_title,
            item.current_version,
            item.version_count,
            scope,
            item.path
        ));
    }
    lines.join("\n")
}

pub(crate) fn render_dashboard_history_inventory_table(
    document: &DashboardHistoryInventoryDocument,
) -> String {
    render_table(
        &[
            "dashboardUid",
            "currentTitle",
            "currentVersion",
            "versionCount",
            "scope",
            "path",
        ],
        &document
            .artifacts
            .iter()
            .map(|item| {
                vec![
                    item.dashboard_uid.clone(),
                    item.current_title.clone(),
                    item.current_version.to_string(),
                    item.version_count.to_string(),
                    item.scope.clone().unwrap_or_else(|| "current".to_string()),
                    item.path.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    )
    .join("\n")
}

pub(crate) fn render_dashboard_history_restore_text(
    document: &DashboardHistoryRestoreDocument,
) -> String {
    let mut lines = vec![format!(
        "Dashboard history restore: {} current-version={} restore-version={} mode={} creates-new-revision={}",
        document.dashboard_uid,
        document.current_version,
        document.restore_version,
        document.mode,
        document.creates_new_revision
    )];
    lines.push(format!("Current title: {}", document.current_title));
    lines.push(format!("Restored title: {}", document.restored_title));
    if let Some(folder_uid) = &document.target_folder_uid {
        lines.push(format!("Target folder UID: {folder_uid}"));
    }
    lines.push(format!("Message: {}", document.message));
    lines.join("\n")
}

pub(crate) fn render_dashboard_history_restore_table(
    document: &DashboardHistoryRestoreDocument,
) -> String {
    let mut rows = vec![
        ("dashboardUid", document.dashboard_uid.clone()),
        ("mode", document.mode.clone()),
        ("currentVersion", document.current_version.to_string()),
        ("restoreVersion", document.restore_version.to_string()),
        ("currentTitle", document.current_title.clone()),
        ("restoredTitle", document.restored_title.clone()),
        (
            "createsNewRevision",
            document.creates_new_revision.to_string(),
        ),
        ("message", document.message.clone()),
    ];
    if let Some(folder_uid) = &document.target_folder_uid {
        rows.push(("targetFolderUid", folder_uid.clone()));
    }
    render_table(
        &["field", "value"],
        &rows
            .into_iter()
            .map(|(field, value)| vec![field.to_string(), value])
            .collect::<Vec<_>>(),
    )
    .join("\n")
}

pub(crate) fn render_dashboard_history_diff_text(
    base: &ResolvedHistoryDiffSide,
    new: &ResolvedHistoryDiffSide,
    document: &DashboardHistoryDiffDocument,
) -> String {
    let row = &document.rows[0];
    let status = row
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("different");
    let mut lines = vec![format!(
        "Dashboard history diff: {} status={} base-version={} new-version={}",
        if base.dashboard_uid == new.dashboard_uid {
            base.dashboard_uid.clone()
        } else {
            format!("{} -> {}", base.dashboard_uid, new.dashboard_uid)
        },
        status,
        base.version,
        new.version
    )];
    lines.push(format!("Base source: {}", base.source_label));
    lines.push(format!("New source: {}", new.source_label));
    if let Some(path) = row.get("path").and_then(Value::as_str) {
        lines.push(format!("Path: {path}"));
    }
    lines.push(format!("Base title: {}", base.title));
    lines.push(format!("New title: {}", new.title));
    if let Some(diff_text) = row.get("diffText").and_then(Value::as_str) {
        lines.push(String::new());
        lines.push(diff_text.trim_end().to_string());
    }
    lines.join("\n")
}

pub(crate) fn render_dashboard_history_list_output(
    document: &DashboardHistoryListDocument,
    output_format: HistoryOutputFormat,
) -> Result<()> {
    let rendered = match output_format {
        HistoryOutputFormat::Text => render_dashboard_history_list_text(document),
        HistoryOutputFormat::Table => render_dashboard_history_list_table(document),
        HistoryOutputFormat::Json => render_json_value(document)?.trim_end().to_string(),
        HistoryOutputFormat::Yaml => render_yaml(document)?.trim_end().to_string(),
    };
    println!("{rendered}");
    Ok(())
}

pub(crate) fn render_dashboard_history_inventory_output(
    document: &DashboardHistoryInventoryDocument,
    output_format: HistoryOutputFormat,
) -> Result<()> {
    let rendered = match output_format {
        HistoryOutputFormat::Text => render_dashboard_history_inventory_text(document),
        HistoryOutputFormat::Table => render_dashboard_history_inventory_table(document),
        HistoryOutputFormat::Json => render_json_value(document)?.trim_end().to_string(),
        HistoryOutputFormat::Yaml => render_yaml(document)?.trim_end().to_string(),
    };
    println!("{rendered}");
    Ok(())
}
