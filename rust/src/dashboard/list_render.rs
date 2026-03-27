//! Dashboard and datasource summary render helpers for list output.
use serde_json::{Map, Value};
use std::fmt::Write as _;

use crate::common::string_field;

use super::{
    DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID, DEFAULT_UNKNOWN_UID,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashboardListColumn {
    Uid,
    Name,
    Folder,
    FolderUid,
    Path,
    Org,
    OrgId,
    Sources,
    SourceUids,
}

impl DashboardListColumn {
    fn header(self) -> &'static str {
        match self {
            DashboardListColumn::Uid => "UID",
            DashboardListColumn::Name => "NAME",
            DashboardListColumn::Folder => "FOLDER",
            DashboardListColumn::FolderUid => "FOLDER_UID",
            DashboardListColumn::Path => "FOLDER_PATH",
            DashboardListColumn::Org => "ORG",
            DashboardListColumn::OrgId => "ORG_ID",
            DashboardListColumn::Sources => "SOURCES",
            DashboardListColumn::SourceUids => "SOURCE_UIDS",
        }
    }

    fn csv_key(self) -> &'static str {
        match self {
            DashboardListColumn::Uid => "uid",
            DashboardListColumn::Name => "name",
            DashboardListColumn::Folder => "folder",
            DashboardListColumn::FolderUid => "folderUid",
            DashboardListColumn::Path => "path",
            DashboardListColumn::Org => "org",
            DashboardListColumn::OrgId => "orgId",
            DashboardListColumn::Sources => "sources",
            DashboardListColumn::SourceUids => "sourceUids",
        }
    }
}

fn org_id_cell(summary: &Map<String, Value>) -> Option<String> {
    summary.get("orgId").and_then(|value| match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) => Some(text.clone()),
        _ => None,
    })
}

fn parse_dashboard_list_column(column: &str) -> Option<DashboardListColumn> {
    match column {
        "uid" => Some(DashboardListColumn::Uid),
        "name" => Some(DashboardListColumn::Name),
        "folder" => Some(DashboardListColumn::Folder),
        "folder_uid" => Some(DashboardListColumn::FolderUid),
        "path" => Some(DashboardListColumn::Path),
        "org" => Some(DashboardListColumn::Org),
        "org_id" => Some(DashboardListColumn::OrgId),
        "sources" => Some(DashboardListColumn::Sources),
        "source_uids" => Some(DashboardListColumn::SourceUids),
        _ => None,
    }
}

fn dashboard_sources(summary: &Map<String, Value>) -> Option<Vec<String>> {
    let values = summary.get("sources")?.as_array()?;
    Some(
        values
            .iter()
            .filter_map(Value::as_str)
            .map(|value| value.to_string())
            .collect(),
    )
}

fn dashboard_source_uids(summary: &Map<String, Value>) -> Option<Vec<String>> {
    let values = summary.get("sourceUids")?.as_array()?;
    Some(
        values
            .iter()
            .filter_map(Value::as_str)
            .map(|value| value.to_string())
            .collect(),
    )
}

fn dashboard_sources_cell(summary: &Map<String, Value>) -> Option<String> {
    let values = dashboard_sources(summary)?;
    if values.is_empty() {
        None
    } else {
        Some(values.join(","))
    }
}

fn summaries_include_sources(summaries: &[Map<String, Value>]) -> bool {
    summaries
        .iter()
        .any(|summary| summary.contains_key("sources"))
}

fn summaries_include_org_metadata(summaries: &[Map<String, Value>]) -> bool {
    summaries
        .iter()
        .any(|summary| summary.contains_key("orgName") || summary.contains_key("orgId"))
}

fn summaries_include_source_uids(summaries: &[Map<String, Value>]) -> bool {
    summaries
        .iter()
        .any(|summary| summary.contains_key("sourceUids"))
}

fn resolve_dashboard_list_columns(
    summaries: &[Map<String, Value>],
    output_columns: &[String],
) -> Vec<DashboardListColumn> {
    if !output_columns.is_empty() {
        return output_columns
            .iter()
            .filter_map(|column| parse_dashboard_list_column(column))
            .collect();
    }

    let mut columns = vec![
        DashboardListColumn::Uid,
        DashboardListColumn::Name,
        DashboardListColumn::Folder,
        DashboardListColumn::FolderUid,
        DashboardListColumn::Path,
    ];
    if summaries_include_org_metadata(summaries) {
        columns.push(DashboardListColumn::Org);
        columns.push(DashboardListColumn::OrgId);
    }
    if summaries_include_sources(summaries) {
        columns.push(DashboardListColumn::Sources);
    }
    if summaries_include_source_uids(summaries) {
        columns.push(DashboardListColumn::SourceUids);
    }
    columns
}

fn dashboard_list_value(summary: &Map<String, Value>, column: DashboardListColumn) -> String {
    match column {
        DashboardListColumn::Uid => string_field(summary, "uid", DEFAULT_UNKNOWN_UID),
        DashboardListColumn::Name => string_field(summary, "title", DEFAULT_DASHBOARD_TITLE),
        DashboardListColumn::Folder => string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
        DashboardListColumn::FolderUid => string_field(summary, "folderUid", DEFAULT_FOLDER_UID),
        DashboardListColumn::Path => string_field(
            summary,
            "folderPath",
            &string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
        ),
        DashboardListColumn::Org => string_field(summary, "orgName", ""),
        DashboardListColumn::OrgId => org_id_cell(summary).unwrap_or_default(),
        DashboardListColumn::Sources => dashboard_sources_cell(summary).unwrap_or_default(),
        DashboardListColumn::SourceUids => {
            dashboard_source_uids(summary).unwrap_or_default().join(",")
        }
    }
}

fn build_dashboard_summary_row_for_columns(
    summary: &Map<String, Value>,
    columns: &[DashboardListColumn],
) -> Vec<String> {
    columns
        .iter()
        .map(|column| dashboard_list_value(summary, *column))
        .collect()
}

/// format dashboard summary line.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn format_dashboard_summary_line(summary: &Map<String, Value>) -> String {
    let uid = string_field(summary, "uid", DEFAULT_UNKNOWN_UID);
    let folder_title = string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE);
    let folder_uid = string_field(summary, "folderUid", DEFAULT_FOLDER_UID);
    let folder_path = string_field(summary, "folderPath", &folder_title);
    let title = string_field(summary, "title", DEFAULT_DASHBOARD_TITLE);
    let mut line = format!(
        "uid={uid} name={title} folder={folder_title} folderUid={folder_uid} path={folder_path}"
    );
    if summary.contains_key("orgName") || summary.contains_key("orgId") {
        let org_name = string_field(summary, "orgName", "");
        let org_id = org_id_cell(summary).unwrap_or_default();
        let _ = write!(&mut line, " org={org_name} orgId={org_id}");
    }
    if let Some(sources) = dashboard_sources_cell(summary) {
        let _ = write!(&mut line, " sources={sources}");
    }
    line
}

/// Purpose: implementation note.
pub(crate) fn render_dashboard_summary_table(
    summaries: &[Map<String, Value>],
    output_columns: &[String],
    include_header: bool,
) -> Vec<String> {
    let columns = resolve_dashboard_list_columns(summaries, output_columns);
    let headers: Vec<String> = columns
        .iter()
        .map(|column| column.header().to_string())
        .collect();
    let rows: Vec<Vec<String>> = summaries
        .iter()
        .map(|summary| build_dashboard_summary_row_for_columns(summary, &columns))
        .collect();
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
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

    let separator: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    let mut lines = Vec::new();
    if include_header {
        lines.extend([format_row(&headers), format_row(&separator)]);
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

/// Purpose: implementation note.
pub(crate) fn render_dashboard_summary_csv(
    summaries: &[Map<String, Value>],
    output_columns: &[String],
) -> Vec<String> {
    let columns = resolve_dashboard_list_columns(summaries, output_columns);
    let header: Vec<String> = columns
        .iter()
        .map(|column| column.csv_key().to_string())
        .collect();
    let mut lines = vec![header.join(",")];
    lines.extend(summaries.iter().map(|summary| {
        let row = build_dashboard_summary_row_for_columns(summary, &columns);
        row.into_iter()
            .map(|value| {
                if value.contains(',') || value.contains('"') || value.contains('\n') {
                    format!("\"{}\"", value.replace('"', "\"\""))
                } else {
                    value
                }
            })
            .collect::<Vec<String>>()
            .join(",")
    }));
    lines
}

/// Purpose: implementation note.
pub(crate) fn render_dashboard_summary_json(
    summaries: &[Map<String, Value>],
    output_columns: &[String],
) -> Value {
    let columns = resolve_dashboard_list_columns(summaries, output_columns);
    Value::Array(
        summaries
            .iter()
            .map(|summary| {
                let mut object = Map::new();
                for column in &columns {
                    match column {
                        DashboardListColumn::Sources => {
                            object.insert(
                                column.csv_key().to_string(),
                                Value::Array(
                                    dashboard_sources(summary)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(Value::String)
                                        .collect(),
                                ),
                            );
                        }
                        DashboardListColumn::SourceUids => {
                            object.insert(
                                column.csv_key().to_string(),
                                Value::Array(
                                    dashboard_source_uids(summary)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(Value::String)
                                        .collect(),
                                ),
                            );
                        }
                        _ => {
                            object.insert(
                                column.csv_key().to_string(),
                                Value::String(dashboard_list_value(summary, *column)),
                            );
                        }
                    }
                }
                Value::Object(object)
            })
            .collect(),
    )
}
