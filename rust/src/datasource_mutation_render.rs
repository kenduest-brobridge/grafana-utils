//! Renderers for datasource mutation dry-run and import result payloads.
//!
//! Responsibilities:
//! - Convert mutation rows into structured table/json output.
//! - Validate dry-run arguments for command-line consistency.
//! - Provide shared formatting between `mutation` and `import` workflows.

use serde_json::{Map, Value};

use crate::common::{message, requested_columns_include_all, Result};

pub(crate) fn render_live_mutation_table(
    rows: &[Vec<String>],
    include_header: bool,
) -> Vec<String> {
    let headers = vec![
        "OPERATION".to_string(),
        "UID".to_string(),
        "NAME".to_string(),
        "TYPE".to_string(),
        "MATCH".to_string(),
        "ACTION".to_string(),
        "TARGET_ID".to_string(),
    ];
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
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
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>();
    let mut lines = Vec::new();
    if include_header {
        lines.push(format_row(&headers));
        lines.push(format_row(&separator));
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

pub(crate) fn render_live_mutation_json(rows: &[Vec<String>]) -> Value {
    let create_count = rows.iter().filter(|row| row[5] == "would-create").count();
    let update_count = rows.iter().filter(|row| row[5] == "would-update").count();
    let delete_count = rows.iter().filter(|row| row[5] == "would-delete").count();
    let blocked_count = rows
        .iter()
        .filter(|row| row[5].starts_with("would-fail-"))
        .count();
    Value::Object(Map::from_iter(vec![
        (
            "items".to_string(),
            Value::Array(
                rows.iter()
                    .map(|row| {
                        Value::Object(Map::from_iter(vec![
                            ("operation".to_string(), Value::String(row[0].clone())),
                            ("uid".to_string(), Value::String(row[1].clone())),
                            ("name".to_string(), Value::String(row[2].clone())),
                            ("type".to_string(), Value::String(row[3].clone())),
                            ("match".to_string(), Value::String(row[4].clone())),
                            ("action".to_string(), Value::String(row[5].clone())),
                            ("targetId".to_string(), Value::String(row[6].clone())),
                        ]))
                    })
                    .collect(),
            ),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "itemCount".to_string(),
                    Value::Number((rows.len() as i64).into()),
                ),
                (
                    "createCount".to_string(),
                    Value::Number((create_count as i64).into()),
                ),
                (
                    "updateCount".to_string(),
                    Value::Number((update_count as i64).into()),
                ),
                (
                    "deleteCount".to_string(),
                    Value::Number((delete_count as i64).into()),
                ),
                (
                    "blockedCount".to_string(),
                    Value::Number((blocked_count as i64).into()),
                ),
            ])),
        ),
    ]))
}

pub(crate) fn validate_live_mutation_dry_run_args(
    table: bool,
    json: bool,
    dry_run: bool,
    no_header: bool,
    verb: &str,
) -> Result<()> {
    if table && !dry_run {
        return Err(message(format!(
            "--table is only supported with --dry-run for datasource {verb}."
        )));
    }
    if json && !dry_run {
        return Err(message(format!(
            "--json is only supported with --dry-run for datasource {verb}."
        )));
    }
    if table && json {
        return Err(message(format!(
            "--table and --json are mutually exclusive for datasource {verb}."
        )));
    }
    if no_header && !table {
        return Err(message(format!(
            "--no-header is only supported with --dry-run --table for datasource {verb}."
        )));
    }
    Ok(())
}

pub(crate) fn render_import_table(
    rows: &[Vec<String>],
    include_header: bool,
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    let columns = if let Some(selected) = selected_columns {
        if requested_columns_include_all(selected) {
            vec![
                (0usize, "UID"),
                (1usize, "NAME"),
                (2usize, "TYPE"),
                (3usize, "DESTINATION"),
                (4usize, "ACTION"),
                (5usize, "ORG_ID"),
                (6usize, "FILE"),
            ]
        } else {
            selected
                .iter()
                .map(|column| match column.as_str() {
                    "uid" => (0usize, "UID"),
                    "name" => (1usize, "NAME"),
                    "type" => (2usize, "TYPE"),
                    "destination" => (3usize, "DESTINATION"),
                    "action" => (4usize, "ACTION"),
                    "org_id" => (5usize, "ORG_ID"),
                    "file" => (6usize, "FILE"),
                    _ => unreachable!("validated datasource import output column"),
                })
                .collect::<Vec<(usize, &str)>>()
        }
    } else {
        vec![
            (0usize, "UID"),
            (1usize, "NAME"),
            (2usize, "TYPE"),
            (3usize, "DESTINATION"),
            (4usize, "ACTION"),
            (5usize, "ORG_ID"),
            (6usize, "FILE"),
        ]
    };
    let headers = columns
        .iter()
        .map(|(_, header)| header.to_string())
        .collect::<Vec<String>>();
    let mut widths: Vec<usize> = headers.iter().map(|item| item.len()).collect();
    for row in rows {
        for (index, (source_index, _)) in columns.iter().enumerate() {
            let value = &row[*source_index];
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
            .map(|(source_index, _)| row[*source_index].clone())
            .collect::<Vec<String>>();
        format_row(&values)
    }));
    lines
}
