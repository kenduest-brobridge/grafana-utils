//! Access plan text, table, and JSON renderers.

use std::fmt::Write as _;

use crate::access::cli_defs::{AccessPlanArgs, PlanOutputFormat};
use crate::access::render::format_table;
use crate::common::{message, render_json_value, Result};

use super::{AccessPlanAction, AccessPlanDocument};

fn plan_supported_columns() -> &'static [&'static str] {
    &[
        "action_id",
        "resource_kind",
        "identity",
        "action",
        "status",
        "changed_fields",
        "changes",
        "target",
        "blocked_reason",
        "review_hints",
        "source_path",
    ]
}

fn default_plan_columns() -> Vec<&'static str> {
    vec![
        "action_id",
        "identity",
        "action",
        "status",
        "blocked_reason",
    ]
}

fn plan_columns_label(columns: &[String]) -> Vec<String> {
    if columns.len() == 1 && columns[0] == "all" {
        return plan_supported_columns()
            .iter()
            .map(|column| (*column).to_string())
            .collect();
    }
    if columns.is_empty() {
        return default_plan_columns()
            .iter()
            .map(|column| (*column).to_string())
            .collect();
    }
    columns.to_vec()
}

fn normalize_plan_columns(columns: &[String]) -> Vec<String> {
    let mut resolved = Vec::new();
    for column in plan_columns_label(columns) {
        if !plan_supported_columns().contains(&column.as_str()) {
            continue;
        }
        if !resolved.contains(&column) {
            resolved.push(column);
        }
    }
    resolved
}

fn plan_header_text(document: &AccessPlanDocument) -> String {
    format!(
        "access plan: resources={} checked={} same={} create={} update={} extra_remote={} delete={} blocked={} warning={} prune={}",
        document.summary.resource_count,
        document.summary.checked,
        document.summary.same,
        document.summary.create,
        document.summary.update,
        document.summary.extra_remote,
        document.summary.delete,
        document.summary.blocked,
        document.summary.warning,
        document.summary.prune,
    )
}

fn render_action_row(action: &AccessPlanAction, columns: &[String]) -> Vec<String> {
    columns
        .iter()
        .map(|column| match column.as_str() {
            "action_id" => action.action_id.clone(),
            "resource_kind" => action.resource_kind.clone(),
            "identity" => action.identity.clone(),
            "action" => action.action.clone(),
            "status" => action.status.clone(),
            "changed_fields" => serde_json::to_string(&action.changed_fields).unwrap_or_default(),
            "changes" => serde_json::to_string(&action.changes).unwrap_or_default(),
            "target" => serde_json::to_string(&action.target).unwrap_or_default(),
            "blocked_reason" => action.blocked_reason.clone().unwrap_or_default(),
            "review_hints" => serde_json::to_string(&action.review_hints).unwrap_or_default(),
            "source_path" => action.source_path.clone(),
            _ => String::new(),
        })
        .collect()
}

pub(super) fn validate_plan_columns(args: &AccessPlanArgs) -> Result<()> {
    if !args.output_columns.is_empty() && matches!(args.output_format, PlanOutputFormat::Json) {
        return Err(message(
            "--output-columns is only supported with text or table output for access plan.",
        ));
    }
    Ok(())
}

pub(super) fn render_plan_text(document: &AccessPlanDocument, args: &AccessPlanArgs) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "{}", plan_header_text(document));
    for resource in &document.resources {
        let _ = writeln!(
            output,
            "- {} source={} bundle={} checked={} same={} create={} update={} extra={} delete={} blocked={} warning={}",
            resource.resource_kind,
            resource.source_path,
            if resource.bundle_present { "present" } else { "missing" },
            resource.checked,
            resource.same,
            resource.create,
            resource.update,
            resource.extra_remote,
            resource.delete,
            resource.blocked,
            resource.warning
        );
    }

    for action in document
        .actions
        .iter()
        .filter(|action| args.show_same || action.action != "same")
    {
        let _ = write!(
            output,
            "{} {} {}",
            action.status.to_uppercase(),
            action.identity,
            action.action
        );
        if !action.changed_fields.is_empty() {
            let _ = write!(output, " fields={}", action.changed_fields.join(","));
        }
        if let Some(reason) = &action.blocked_reason {
            let _ = write!(output, " blocked={reason}");
        }
        if !action.review_hints.is_empty() {
            let _ = write!(output, " hints={}", action.review_hints.join(" | "));
        }
        let _ = writeln!(output);
    }

    output
}

pub(super) fn render_plan_table(document: &AccessPlanDocument, args: &AccessPlanArgs) -> String {
    let columns = normalize_plan_columns(&args.output_columns);
    let headers = columns
        .iter()
        .map(|column| column.replace('_', " ").to_ascii_uppercase())
        .collect::<Vec<String>>();
    let header_refs = headers
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<&str>>();
    let rows = document
        .actions
        .iter()
        .filter(|action| args.show_same || action.action != "same")
        .map(|action| render_action_row(action, &columns))
        .collect::<Vec<Vec<String>>>();
    let mut rendered = String::new();
    let table = format_table(&header_refs, &rows);
    if args.no_header {
        for line in table.into_iter().skip(2) {
            let _ = writeln!(rendered, "{line}");
        }
    } else {
        for line in table {
            let _ = writeln!(rendered, "{line}");
        }
    }
    rendered
}

pub(super) fn render_plan_json(document: &AccessPlanDocument) -> Result<String> {
    render_json_value(&serde_json::to_value(document)?)
}

pub(crate) fn print_access_plan_columns() {
    println!(
        "Supported --output-columns values: all, {}",
        plan_supported_columns().join(", ")
    );
}
