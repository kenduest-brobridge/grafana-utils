use crate::common::{render_json_value, Result};
use serde_json::Value;

use super::AlertCommandOutputFormat;

pub(crate) fn render_alert_action_text(title: &str, document: &Value) -> Vec<String> {
    let mut lines = vec![title.to_string()];
    if document
        .get("reviewRequired")
        .and_then(Value::as_bool)
        .is_some()
        || document.get("reviewed").and_then(Value::as_bool).is_some()
    {
        let review_required = document
            .get("reviewRequired")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let reviewed = document
            .get("reviewed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        lines.push(format!(
            "Review: required={review_required} reviewed={reviewed}"
        ));
    }
    if let Some(summary) = document.get("summary").and_then(Value::as_object) {
        let summary_line = summary
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" ");
        if !summary_line.is_empty() {
            lines.push(format!("Summary: {summary_line}"));
        }
    }
    if let Some(rows) = document.get("rows").and_then(Value::as_array) {
        lines.push("Rows:".to_string());
        for row in rows.iter().take(20) {
            let kind = row.get("kind").and_then(Value::as_str).unwrap_or("unknown");
            let identity = row
                .get("identity")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let action = row
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let reason = row.get("reason").and_then(Value::as_str).unwrap_or("");
            if reason.is_empty() {
                lines.push(format!("- {kind} {identity} action={action}"));
            } else {
                lines.push(format!(
                    "- {kind} {identity} action={action} reason={reason}"
                ));
            }
        }
        if rows.len() > 20 {
            lines.push(format!("- ... {} more rows", rows.len() - 20));
        }
    }
    if let Some(results) = document.get("results").and_then(Value::as_array) {
        lines.push("Results:".to_string());
        for result in results.iter().take(20) {
            let kind = result
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let identity = result
                .get("identity")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let action = result
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            lines.push(format!("- {kind} {identity} action={action}"));
        }
        if results.len() > 20 {
            lines.push(format!("- ... {} more results", results.len() - 20));
        }
    }
    lines
}

pub(super) fn print_alert_action_document(
    title: &str,
    document: &Value,
    output: AlertCommandOutputFormat,
) -> Result<()> {
    match output {
        AlertCommandOutputFormat::Json => {
            println!("{}", render_json_value(document)?);
            Ok(())
        }
        AlertCommandOutputFormat::Text => {
            for line in render_alert_action_text(title, document) {
                println!("{line}");
            }
            Ok(())
        }
    }
}
