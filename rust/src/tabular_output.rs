//! Shared table/csv/yaml/text rendering primitives.
//!
//! Responsibilities:
//! - Build aligned CLI rows for summary, list, and report command outputs.
//! - Serialize simple values consistently for machine-readable and human-readable outputs.
//! - Keep formatting behavior centralized and reused by dashboard/datasource/alert modules.

use crate::common::Result;

pub(crate) fn render_csv(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    let mut lines = vec![headers
        .iter()
        .map(|value| escape_csv(value))
        .collect::<Vec<_>>()
        .join(",")];
    for row in rows {
        lines.push(
            row.iter()
                .map(|value| escape_csv(value))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    lines
}

pub(crate) fn render_table(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<usize>>();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if index >= widths.len() {
                widths.push(value.len());
            } else {
                widths[index] = widths[index].max(value.len());
            }
        }
    }

    let mut lines = Vec::new();
    lines.push(render_row(
        &headers
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
        &widths,
    ));
    lines.push(
        widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  "),
    );
    for row in rows {
        lines.push(render_row(row, &widths));
    }
    lines
}

pub(crate) fn print_lines(lines: &[String]) {
    for line in lines {
        println!("{line}");
    }
}

pub(crate) fn render_yaml<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_yaml::to_string(value)
        .map_err(|error| crate::common::message(format!("YAML rendering failed: {error}")))
}

fn summary_rows_to_cells(rows: &[(&str, String)]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|(field, value)| vec![(*field).to_string(), value.clone()])
        .collect()
}

pub(crate) fn render_summary_table(rows: &[(&str, String)]) -> Vec<String> {
    render_table(&["field", "value"], &summary_rows_to_cells(rows))
}

pub(crate) fn render_summary_csv(rows: &[(&str, String)]) -> Vec<String> {
    render_csv(&["field", "value"], &summary_rows_to_cells(rows))
}

fn render_row(values: &[String], widths: &[usize]) -> String {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
        .collect::<Vec<_>>()
        .join("  ")
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
