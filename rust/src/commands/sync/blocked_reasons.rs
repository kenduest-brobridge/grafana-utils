//! Sync staged document blocking-reason shaping helpers.
//!
//! Purpose:
//! - Pull a few concrete blocking reasons out of staged check arrays so local
//!   apply rejections can explain *why* a preflight blocked, not just how many
//!   checks failed.

use serde_json::Value;

const MAX_BLOCKING_REASONS: usize = 3;

#[derive(Clone, Copy)]
pub(crate) struct BlockingReasonSource {
    pub(crate) label: &'static str,
    pub(crate) path: &'static [&'static str],
}

pub(crate) fn format_blocking_rejection_message(
    subject: &str,
    blocking_count: i64,
    reasons: &[String],
) -> String {
    if reasons.is_empty() {
        return format!(
            "Refusing local sync apply intent because {subject} reports {blocking_count} blocking checks."
        );
    }
    let remaining = (blocking_count.max(0) as usize).saturating_sub(reasons.len());
    format!(
        "Refusing local sync apply intent because {subject} reports {blocking_count} blocking checks. Blocking reasons: {}{}",
        reasons.join("; "),
        if remaining > 0 {
            format!(" (+{} more)", remaining)
        } else {
            String::new()
        }
    )
}

pub(crate) fn collect_blocking_reasons(
    document: &Value,
    sources: &[BlockingReasonSource],
) -> Vec<String> {
    let mut reasons = Vec::new();
    for source in sources {
        let Some(checks) = value_at_path(document, source.path).and_then(Value::as_array) else {
            continue;
        };
        for check in checks {
            if check.get("blocking").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            if let Some(reason) = format_blocking_check(source.label, check) {
                reasons.push(reason);
            }
            if reasons.len() >= MAX_BLOCKING_REASONS {
                return reasons;
            }
        }
    }
    reasons
}

fn value_at_path<'a>(document: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = document;
    for key in path {
        current = current.get(key)?;
    }
    Some(current)
}

fn format_blocking_check(label: &str, check: &Value) -> Option<String> {
    let object = check.as_object()?;
    let mut parts = Vec::new();
    if !label.is_empty() {
        parts.push(format!("source={label}"));
    }
    let kind = normalize_text(object.get("kind"));
    if !kind.is_empty() {
        parts.push(format!("kind={kind}"));
    }
    let identity = normalize_text(object.get("identity"));
    if !identity.is_empty() {
        parts.push(format!("identity={identity}"));
    }
    let provider_name = normalize_text(object.get("providerName"));
    if !provider_name.is_empty() {
        parts.push(format!("providerName={provider_name}"));
    }
    let datasource_name = normalize_text(object.get("datasourceName"));
    if !datasource_name.is_empty() {
        parts.push(format!("datasourceName={datasource_name}"));
    }
    let status = normalize_text(object.get("status"));
    if !status.is_empty() {
        parts.push(format!("status={status}"));
    }
    let detail = normalize_text(object.get("detail"));
    if !detail.is_empty() {
        parts.push(format!("detail={detail}"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}
