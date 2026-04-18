use crate::common::{render_json_value, requested_columns_include_all, Result};

use super::super::DatasourcePlanOutputFormat;
use super::builder::build_datasource_plan_json;
use super::model::{DatasourcePlanAction, DatasourcePlanReport, PLAN_ACTION_SAME};

pub(crate) fn datasource_plan_column_ids() -> &'static [&'static str] {
    &[
        "action_id",
        "action",
        "status",
        "uid",
        "name",
        "type",
        "match_basis",
        "source_org_id",
        "target_org_id",
        "target_uid",
        "target_version",
        "target_read_only",
        "changed_fields",
        "blocked_reason",
        "source_file",
    ]
}

pub(crate) fn print_datasource_plan_report(
    report: &DatasourcePlanReport,
    output_format: DatasourcePlanOutputFormat,
    show_same: bool,
    no_header: bool,
    selected_columns: &[String],
) -> Result<()> {
    match output_format {
        DatasourcePlanOutputFormat::Json => {
            print!(
                "{}",
                render_json_value(&build_datasource_plan_json(report)?)?
            );
        }
        DatasourcePlanOutputFormat::Table => {
            for line in render_plan_table(report, show_same, no_header, selected_columns) {
                println!("{line}");
            }
            println!("{}", plan_summary_line(report));
        }
        DatasourcePlanOutputFormat::Text => {
            println!("{}", plan_summary_line(report));
            for line in render_plan_text_details(report, show_same) {
                println!("{line}");
            }
        }
    }
    Ok(())
}

fn plan_summary_line(report: &DatasourcePlanReport) -> String {
    format!(
        "Datasource plan: checked={} same={} create={} update={} extra={} delete={} blocked={} warning={} orgs={} would-create-orgs={} prune={}",
        report.summary.checked,
        report.summary.same,
        report.summary.create,
        report.summary.update,
        report.summary.extra,
        report.summary.delete,
        report.summary.blocked,
        report.summary.warning,
        report.summary.org_count,
        report.summary.would_create_org_count,
        report.prune
    )
}

fn render_plan_text_details(report: &DatasourcePlanReport, show_same: bool) -> Vec<String> {
    report
        .actions
        .iter()
        .filter(|action| show_same || action.action != PLAN_ACTION_SAME)
        .map(|action| {
            let changed = if action.changed_fields.is_empty() {
                "-".to_string()
            } else {
                action.changed_fields.join(",")
            };
            format!(
                "{} status={} uid={} name={} type={} fields={} reason={}",
                action.action,
                action.status,
                action.uid,
                action.name,
                action.datasource_type,
                changed,
                action.blocked_reason.as_deref().unwrap_or("-")
            )
        })
        .collect()
}

fn render_plan_table(
    report: &DatasourcePlanReport,
    show_same: bool,
    include_header: bool,
    selected_columns: &[String],
) -> Vec<String> {
    let columns = resolve_plan_columns(selected_columns);
    let rows = report
        .actions
        .iter()
        .filter(|action| show_same || action.action != PLAN_ACTION_SAME)
        .map(plan_action_row)
        .collect::<Vec<Vec<String>>>();
    render_table_rows(&rows, &columns, include_header)
}

fn resolve_plan_columns(selected_columns: &[String]) -> Vec<(usize, &'static str)> {
    let all = vec![
        (0usize, "ACTION_ID"),
        (1usize, "ACTION"),
        (2usize, "STATUS"),
        (3usize, "UID"),
        (4usize, "NAME"),
        (5usize, "TYPE"),
        (6usize, "MATCH_BASIS"),
        (7usize, "SOURCE_ORG_ID"),
        (8usize, "TARGET_ORG_ID"),
        (9usize, "TARGET_UID"),
        (10usize, "TARGET_VERSION"),
        (11usize, "TARGET_READ_ONLY"),
        (12usize, "CHANGED_FIELDS"),
        (13usize, "BLOCKED_REASON"),
        (14usize, "SOURCE_FILE"),
    ];
    if selected_columns.is_empty() {
        return vec![
            (1usize, "ACTION"),
            (2usize, "STATUS"),
            (3usize, "UID"),
            (4usize, "NAME"),
            (5usize, "TYPE"),
            (12usize, "CHANGED_FIELDS"),
            (13usize, "BLOCKED_REASON"),
        ];
    }
    if requested_columns_include_all(selected_columns) {
        return all;
    }
    selected_columns
        .iter()
        .filter_map(|column| {
            datasource_plan_column_ids()
                .iter()
                .position(|item| item == column)
                .map(|index| all[index])
        })
        .collect()
}

fn plan_action_row(action: &DatasourcePlanAction) -> Vec<String> {
    vec![
        action.action_id.clone(),
        action.action.clone(),
        action.status.clone(),
        action.uid.clone(),
        action.name.clone(),
        action.datasource_type.clone(),
        action.match_basis.clone(),
        action.source_org_id.clone().unwrap_or_default(),
        action.target_org_id.clone().unwrap_or_default(),
        action.target_uid.clone().unwrap_or_default(),
        action
            .target_version
            .map(|value| value.to_string())
            .unwrap_or_default(),
        action
            .target_read_only
            .map(|value| value.to_string())
            .unwrap_or_default(),
        action.changed_fields.join(","),
        action.blocked_reason.clone().unwrap_or_default(),
        action.source_file.clone().unwrap_or_default(),
    ]
}

fn render_table_rows(
    rows: &[Vec<String>],
    columns: &[(usize, &'static str)],
    include_header: bool,
) -> Vec<String> {
    let headers = columns
        .iter()
        .map(|(_, header)| header.to_string())
        .collect::<Vec<String>>();
    let mut widths = headers
        .iter()
        .map(|item| item.len())
        .collect::<Vec<usize>>();
    for row in rows {
        for (index, (source_index, _)) in columns.iter().enumerate() {
            let value = row.get(*source_index).map(String::as_str).unwrap_or("");
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>();
    let mut lines = Vec::new();
    if include_header {
        lines.push(format_row(&headers));
        lines.push(format_row(&separator));
    }
    lines.extend(rows.iter().map(|row| {
        let values = columns
            .iter()
            .map(|(source_index, _)| row.get(*source_index).cloned().unwrap_or_default())
            .collect::<Vec<String>>();
        format_row(&values)
    }));
    lines
}
