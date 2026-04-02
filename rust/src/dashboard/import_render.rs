//! Import orchestration for Dashboard resources, including input normalization and apply contract handling.

use serde::Serialize;
use serde_json::Value;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::common::{render_json_value, Result};

use super::{FolderInventoryStatus, FolderInventoryStatusKind, DEFAULT_UNKNOWN_UID};

/// Struct definition for ImportDryRunReport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ImportDryRunReport {
    pub mode: String,
    pub import_dir: PathBuf,
    pub folder_statuses: Vec<FolderInventoryStatus>,
    pub dashboard_records: Vec<[String; 8]>,
    pub skipped_missing_count: usize,
    pub skipped_folder_mismatch_count: usize,
}

fn describe_import_action(action: &str) -> (&'static str, &str) {
    match action {
        "would-create" => ("missing", "create"),
        "would-update" => ("exists", "update"),
        "would-skip-missing" => ("missing", "skip-missing"),
        "would-skip-folder-mismatch" => ("exists", "skip-folder-mismatch"),
        "would-fail-existing" => ("exists", "blocked-existing"),
        _ => (DEFAULT_UNKNOWN_UID, action),
    }
}

pub(crate) fn describe_dashboard_import_mode(
    replace_existing: bool,
    update_existing_only: bool,
) -> &'static str {
    if update_existing_only {
        "update-or-skip-missing"
    } else if replace_existing {
        "create-or-update"
    } else {
        "create-only"
    }
}

pub(crate) fn build_import_dry_run_record(
    dashboard_file: &Path,
    uid: &str,
    action: &str,
    folder_path: &str,
    source_folder_path: &str,
    destination_folder_path: Option<&str>,
    reason: &str,
) -> [String; 8] {
    let (destination, action_label) = describe_import_action(action);
    [
        uid.to_string(),
        destination.to_string(),
        action_label.to_string(),
        folder_path.to_string(),
        source_folder_path.to_string(),
        destination_folder_path.unwrap_or("").to_string(),
        reason.to_string(),
        dashboard_file.display().to_string(),
    ]
}

pub(crate) fn build_folder_inventory_dry_run_record(status: &FolderInventoryStatus) -> [String; 6] {
    let destination = match status.kind {
        FolderInventoryStatusKind::Missing => "missing",
        _ => "exists",
    };
    let reason = match status.kind {
        FolderInventoryStatusKind::Missing => "would-create".to_string(),
        FolderInventoryStatusKind::Matches => String::new(),
        FolderInventoryStatusKind::Mismatch => {
            let mut reasons = Vec::new();
            if status.actual_title.as_deref() != Some(status.expected_title.as_str()) {
                reasons.push("title");
            }
            if status.actual_parent_uid != status.expected_parent_uid {
                reasons.push("parentUid");
            }
            if status.actual_path.as_deref() != Some(status.expected_path.as_str()) {
                reasons.push("path");
            }
            reasons.join(",")
        }
    };
    [
        status.uid.clone(),
        destination.to_string(),
        match status.kind {
            FolderInventoryStatusKind::Missing => "missing",
            FolderInventoryStatusKind::Matches => "match",
            FolderInventoryStatusKind::Mismatch => "mismatch",
        }
        .to_string(),
        reason,
        status.expected_path.clone(),
        status.actual_path.clone().unwrap_or_default(),
    ]
}

pub(crate) fn render_folder_inventory_dry_run_table(
    records: &[[String; 6]],
    include_header: bool,
) -> Vec<String> {
    let headers = [
        "UID",
        "DESTINATION",
        "STATUS",
        "REASON",
        "EXPECTED_PATH",
        "ACTUAL_PATH",
    ];
    let mut widths = headers.map(str::len);
    for row in records {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String; 6]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        let header_values = [
            headers[0].to_string(),
            headers[1].to_string(),
            headers[2].to_string(),
            headers[3].to_string(),
            headers[4].to_string(),
            headers[5].to_string(),
        ];
        let divider_values = [
            "-".repeat(widths[0]),
            "-".repeat(widths[1]),
            "-".repeat(widths[2]),
            "-".repeat(widths[3]),
            "-".repeat(widths[4]),
            "-".repeat(widths[5]),
        ];
        lines.push(format_row(&header_values));
        lines.push(format_row(&divider_values));
    }
    for row in records {
        lines.push(format_row(row));
    }
    lines
}

pub(crate) fn render_import_dry_run_table(
    records: &[[String; 8]],
    include_header: bool,
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    let columns = resolve_dashboard_import_table_columns(records, selected_columns);
    let headers = columns
        .iter()
        .map(|(_, header)| *header)
        .collect::<Vec<&str>>();
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<usize>>();
    for row in records {
        let visible = columns
            .iter()
            .map(|(index, _)| row[*index].as_str())
            .collect::<Vec<&str>>();
        for (index, value) in visible.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        let header_values = headers
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>();
        let divider_values = widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<String>>();
        lines.push(format_row(&header_values));
        lines.push(format_row(&divider_values));
    }
    for row in records {
        let visible = columns
            .iter()
            .map(|(index, _)| row[*index].clone())
            .collect::<Vec<String>>();
        lines.push(format_row(&visible));
    }
    lines
}

pub(crate) fn format_routed_import_target_org_label(target_org_id: Option<i64>) -> String {
    target_org_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<new>".to_string())
}

pub(crate) fn build_routed_import_org_row(
    plan: &super::import_validation::ExportOrgTargetPlan,
    dashboard_count: usize,
) -> [String; 5] {
    [
        plan.source_org_id.to_string(),
        if plan.source_org_name.is_empty() {
            "-".to_string()
        } else {
            plan.source_org_name.clone()
        },
        plan.org_action.to_string(),
        format_routed_import_target_org_label(plan.target_org_id),
        dashboard_count.to_string(),
    ]
}

pub(crate) fn format_routed_import_scope_summary_fields(
    source_org_id: i64,
    source_org_name: &str,
    org_action: &str,
    target_org_id: Option<i64>,
    import_dir: &Path,
) -> String {
    let source_org_name = if source_org_name.is_empty() {
        "-".to_string()
    } else {
        source_org_name.to_string()
    };
    let target_org_id = format_routed_import_target_org_label(target_org_id);
    format!(
        "export orgId={} name={} orgAction={} targetOrgId={} from {}",
        source_org_id,
        source_org_name,
        org_action,
        target_org_id,
        import_dir.display()
    )
}

pub(crate) fn render_routed_import_org_table(
    rows: &[[String; 5]],
    include_header: bool,
) -> Vec<String> {
    let headers = [
        "SOURCE_ORG_ID",
        "SOURCE_ORG_NAME",
        "ORG_ACTION",
        "TARGET_ORG_ID",
        "DASHBOARD_COUNT",
    ];
    let mut widths = headers.map(str::len);
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String; 5]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        let header_values = [
            headers[0].to_string(),
            headers[1].to_string(),
            headers[2].to_string(),
            headers[3].to_string(),
            headers[4].to_string(),
        ];
        let divider_values = [
            "-".repeat(widths[0]),
            "-".repeat(widths[1]),
            "-".repeat(widths[2]),
            "-".repeat(widths[3]),
            "-".repeat(widths[4]),
        ];
        lines.push(format_row(&header_values));
        lines.push(format_row(&divider_values));
    }
    for row in rows {
        lines.push(format_row(row));
    }
    lines
}

fn resolve_dashboard_import_table_columns(
    records: &[[String; 8]],
    selected_columns: Option<&[String]>,
) -> Vec<(usize, &'static str)> {
    if let Some(columns) = selected_columns {
        return columns
            .iter()
            .map(|column| match column.as_str() {
                "uid" => (0usize, "UID"),
                "destination" => (1usize, "DESTINATION"),
                "action" => (2usize, "ACTION"),
                "folder_path" => (3usize, "FOLDER_PATH"),
                "source_folder_path" => (4usize, "SOURCE_FOLDER_PATH"),
                "destination_folder_path" => (5usize, "DESTINATION_FOLDER_PATH"),
                "reason" => (6usize, "REASON"),
                "file" => (7usize, "FILE"),
                _ => unreachable!("validated dashboard import output column"),
            })
            .collect();
    }
    let include_source_folder = records.iter().any(|row| !row[4].is_empty());
    let include_destination_folder = records.iter().any(|row| !row[5].is_empty());
    let include_reason = records.iter().any(|row| !row[6].is_empty());
    let mut columns = vec![
        (0usize, "UID"),
        (1usize, "DESTINATION"),
        (2usize, "ACTION"),
        (3usize, "FOLDER_PATH"),
    ];
    if include_source_folder {
        columns.push((4usize, "SOURCE_FOLDER_PATH"));
    }
    if include_destination_folder {
        columns.push((5usize, "DESTINATION_FOLDER_PATH"));
    }
    if include_reason {
        columns.push((6usize, "REASON"));
    }
    columns.push((7usize, "FILE"));
    columns
}

pub(crate) fn build_import_dry_run_json_value(report: &ImportDryRunReport) -> Value {
    let folders = report
        .folder_statuses
        .iter()
        .map(|status| {
            let (destination, status_label, reason) = match status.kind {
                FolderInventoryStatusKind::Missing => {
                    ("missing", "missing", "would-create".to_string())
                }
                FolderInventoryStatusKind::Matches => ("exists", "match", String::new()),
                FolderInventoryStatusKind::Mismatch => {
                    let mut reasons = Vec::new();
                    if status.actual_title.as_deref() != Some(status.expected_title.as_str()) {
                        reasons.push("title");
                    }
                    if status.actual_parent_uid != status.expected_parent_uid {
                        reasons.push("parentUid");
                    }
                    if status.actual_path.as_deref() != Some(status.expected_path.as_str()) {
                        reasons.push("path");
                    }
                    ("exists", "mismatch", reasons.join(","))
                }
            };
            serde_json::json!({
                "uid": status.uid,
                "destination": destination,
                "status": status_label,
                "reason": reason,
                "expectedPath": status.expected_path,
                "actualPath": status.actual_path.clone().unwrap_or_default(),
            })
        })
        .collect::<Vec<Value>>();
    let dashboards = report
        .dashboard_records
        .iter()
        .map(|row| {
            serde_json::json!({
                "uid": row[0],
                "destination": row[1],
                "action": row[2],
                "folderPath": row[3],
                "sourceFolderPath": row[4],
                "destinationFolderPath": row[5],
                "reason": row[6],
                "file": row[7],
            })
        })
        .collect::<Vec<Value>>();
    serde_json::json!({
        "mode": report.mode,
        "folders": folders,
        "dashboards": dashboards,
        "summary": {
            "importDir": report.import_dir.display().to_string(),
            "folderCount": report.folder_statuses.len(),
            "missingFolders": report.folder_statuses.iter().filter(|status| status.kind == FolderInventoryStatusKind::Missing).count(),
            "mismatchedFolders": report.folder_statuses.iter().filter(|status| status.kind == FolderInventoryStatusKind::Mismatch).count(),
            "dashboardCount": report.dashboard_records.len(),
            "missingDashboards": report.dashboard_records.iter().filter(|row| row[1] == "missing").count(),
            "skippedMissingDashboards": report.skipped_missing_count,
            "skippedFolderMismatchDashboards": report.skipped_folder_mismatch_count,
        }
    })
}

pub(crate) fn build_routed_import_dry_run_json_document(
    orgs: &[Value],
    imports: &[Value],
) -> Result<String> {
    let payload = serde_json::json!({
        "mode": "routed-import-preview",
        "orgs": orgs,
        "imports": imports,
        "summary": {
            "orgCount": orgs.len(),
            "existingOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("exists".to_string()))).count(),
            "missingOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("missing".to_string()))).count(),
            "wouldCreateOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("would-create".to_string()))).count(),
            "dashboardCount": orgs.iter().map(|entry| entry.get("dashboardCount").and_then(Value::as_u64).unwrap_or(0)).sum::<u64>(),
        }
    });
    render_json_value(&payload)
}

pub(crate) fn render_import_dry_run_json(
    mode: &str,
    folder_statuses: &[FolderInventoryStatus],
    dashboard_records: &[[String; 8]],
    import_dir: &Path,
    skipped_missing_count: usize,
    skipped_folder_mismatch_count: usize,
) -> Result<String> {
    let report = ImportDryRunReport {
        mode: mode.to_string(),
        import_dir: import_dir.to_path_buf(),
        folder_statuses: folder_statuses.to_vec(),
        dashboard_records: dashboard_records.to_vec(),
        skipped_missing_count,
        skipped_folder_mismatch_count,
    };
    render_json_value(&build_import_dry_run_json_value(&report))
}

pub(crate) fn format_import_progress_line(
    current: usize,
    total: usize,
    dashboard_target: &str,
    dry_run: bool,
    action: Option<&str>,
    folder_path: Option<&str>,
) -> String {
    if dry_run {
        let (destination, action_label) =
            describe_import_action(action.unwrap_or(DEFAULT_UNKNOWN_UID));
        let mut line = format!(
            "Dry-run dashboard {current}/{total}: {dashboard_target} dest={destination} action={action_label}"
        );
        if let Some(path) = folder_path.filter(|value| !value.is_empty()) {
            let _ = write!(&mut line, " folderPath={path}");
        }
        line
    } else {
        format!("Importing dashboard {current}/{total}: {dashboard_target}")
    }
}

pub(crate) fn format_import_verbose_line(
    dashboard_file: &Path,
    dry_run: bool,
    uid: Option<&str>,
    action: Option<&str>,
    folder_path: Option<&str>,
) -> String {
    if dry_run {
        let (destination, action_label) =
            describe_import_action(action.unwrap_or(DEFAULT_UNKNOWN_UID));
        let mut line = format!(
            "Dry-run import uid={} dest={} action={} file={}",
            uid.unwrap_or(DEFAULT_UNKNOWN_UID),
            destination,
            action_label,
            dashboard_file.display()
        );
        if let Some(path) = folder_path.filter(|value| !value.is_empty()) {
            line = format!(
                "Dry-run import uid={} dest={} action={} folderPath={} file={}",
                uid.unwrap_or(DEFAULT_UNKNOWN_UID),
                destination,
                action_label,
                path,
                dashboard_file.display()
            );
        }
        line
    } else {
        format!("Imported {}", dashboard_file.display())
    }
}
