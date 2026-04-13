//! Shared table/csv/yaml/text rendering primitives.
//!
//! Responsibilities:
//! - Build aligned CLI rows for summary, list, and report command outputs.
//! - Serialize simple values consistently for machine-readable and human-readable outputs.
//! - Keep formatting behavior centralized and reused by dashboard/datasource/alert modules.

use std::io::IsTerminal;

use crate::common::{json_color_choice, json_color_enabled, Result};

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_TABLE_HEADER: &str = "\x1b[1;36m";
const ANSI_TABLE_TRUE: &str = "\x1b[1;32m";
const ANSI_TABLE_FALSE: &str = "\x1b[1;31m";
const ANSI_TABLE_WARN: &str = "\x1b[1;33m";
const ANSI_TABLE_INFO: &str = "\x1b[1;34m";
const ANSI_TABLE_MUTED: &str = "\x1b[2;90m";
const ANSI_TABLE_SEPARATOR: &str = "\x1b[2;90m";
const ANSI_TABLE_PRIMARY: &str = "\x1b[1;97m";
const ANSI_TABLE_LINK: &str = "\x1b[36m";
const ANSI_YAML_KEY: &str = "\x1b[1;36m";
const ANSI_YAML_STRING: &str = "\x1b[32m";
const ANSI_YAML_NUMBER: &str = "\x1b[33m";
const ANSI_YAML_BOOL: &str = "\x1b[35m";
const ANSI_YAML_NULL: &str = "\x1b[2;90m";

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
    let colorize = json_color_enabled(json_color_choice(), std::io::stdout().is_terminal());
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
    lines.push(render_header_row(headers, &widths, colorize));
    lines.push(render_separator_row(&widths, colorize));
    for row in rows {
        lines.push(render_table_data_row(headers, row, &widths, colorize));
    }
    lines
}

pub(crate) fn print_lines(lines: &[String]) {
    for line in lines {
        println!("{line}");
    }
}

pub(crate) fn render_yaml<T: serde::Serialize>(value: &T) -> Result<String> {
    let rendered = serde_yaml::to_string(value)
        .map_err(|error| crate::common::message(format!("YAML rendering failed: {error}")))?;
    if json_color_enabled(json_color_choice(), std::io::stdout().is_terminal()) {
        Ok(colorize_yaml(&rendered))
    } else {
        Ok(rendered)
    }
}

fn colorize_yaml(rendered: &str) -> String {
    let mut output = String::with_capacity(rendered.len() + 32);
    for line in rendered.lines() {
        output.push_str(&colorize_yaml_line(line));
        output.push('\n');
    }
    output
}

fn colorize_yaml_line(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return line.to_string();
    }

    let indent_len = line.len().saturating_sub(trimmed.len());
    let indent = &line[..indent_len];

    if let Some(rest) = trimmed.strip_prefix("- ") {
        let mut out = String::from(indent);
        out.push_str("- ");
        out.push_str(&colorize_yaml_scalar(rest));
        return out;
    }

    if let Some((key, value)) = trimmed.split_once(':') {
        let mut out = String::from(indent);
        out.push_str(ANSI_YAML_KEY);
        out.push_str(key);
        out.push_str(ANSI_RESET);
        out.push(':');
        if !value.is_empty() {
            out.push_str(&colorize_yaml_value_segment(value));
        }
        return out;
    }

    format!("{indent}{}", colorize_yaml_scalar(trimmed))
}

fn colorize_yaml_value_segment(value: &str) -> String {
    let trimmed_len = value.trim_start().len();
    let leading_ws_len = value.len().saturating_sub(trimmed_len);
    let leading = &value[..leading_ws_len];
    let scalar = value[leading_ws_len..].trim_end();
    let trailing_len = value[leading_ws_len..].len().saturating_sub(scalar.len());
    let trailing = &value[value.len().saturating_sub(trailing_len)..];
    format!("{leading}{}{}", colorize_yaml_scalar(scalar), trailing)
}

fn colorize_yaml_scalar(scalar: &str) -> String {
    if scalar.is_empty() {
        return String::new();
    }
    let color = match scalar {
        "true" | "false" => Some(ANSI_YAML_BOOL),
        "null" | "~" => Some(ANSI_YAML_NULL),
        _ if scalar
            .chars()
            .next()
            .is_some_and(|first| first == '-' || first.is_ascii_digit()) =>
        {
            Some(ANSI_YAML_NUMBER)
        }
        _ => Some(ANSI_YAML_STRING),
    };
    match color {
        Some(color) => format!("{color}{scalar}{ANSI_RESET}"),
        None => scalar.to_string(),
    }
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

fn render_table_data_row(
    headers: &[&str],
    values: &[String],
    widths: &[usize],
    colorize: bool,
) -> String {
    if !colorize {
        return render_row(values, widths);
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let padded = format!("{value:<width$}", width = widths[index]);
            match table_cell_color(headers.get(index).copied(), value) {
                Some(color) => format!("{color}{padded}{ANSI_RESET}"),
                None => padded,
            }
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn render_header_row(headers: &[&str], widths: &[usize], colorize: bool) -> String {
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let padded = format!("{header:<width$}", width = widths[index]);
            if colorize {
                format!("{ANSI_TABLE_HEADER}{padded}{ANSI_RESET}")
            } else {
                padded
            }
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn render_separator_row(widths: &[usize], colorize: bool) -> String {
    widths
        .iter()
        .map(|width| {
            let separator = "-".repeat(*width);
            if colorize {
                format!("{ANSI_TABLE_SEPARATOR}{separator}{ANSI_RESET}")
            } else {
                separator
            }
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn table_cell_color(header: Option<&str>, value: &str) -> Option<&'static str> {
    if let Some(header) = header.map(|item| item.trim().to_ascii_lowercase()) {
        match header.as_str() {
            "uid" | "id" | "org_id" | "orgid" | "datasource_uid" | "dashboard_uid" => {
                return Some(ANSI_TABLE_MUTED);
            }
            "name" | "title" => return Some(ANSI_TABLE_PRIMARY),
            "url" | "path" | "file" => return Some(ANSI_TABLE_LINK),
            _ => {}
        }
    }
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "enabled" | "done" | "same" | "matches" | "ok" | "passed" => {
            Some(ANSI_TABLE_TRUE)
        }
        "false" | "no" | "disabled" | "missing" | "different" | "failed" | "error" => {
            Some(ANSI_TABLE_FALSE)
        }
        "warn" | "warning" | "changed" | "would-update" | "would-create" | "provisioning" => {
            Some(ANSI_TABLE_WARN)
        }
        "raw" | "source" | "inventory" | "json" | "yaml" | "table" | "csv" | "text" => {
            Some(ANSI_TABLE_INFO)
        }
        "none" | "null" | "wait" | "unknown" => Some(ANSI_TABLE_MUTED),
        _ => None,
    }
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
