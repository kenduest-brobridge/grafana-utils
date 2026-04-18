//! Workspace preview contract adapter.
//!
//! This facade keeps the public preview JSON shape stable while the internal
//! review view model lives in `workspace_preview_review_view.rs`.

use crate::common::{message, Result};
use serde_json::Value;

use super::workspace_preview_review_view::build_workspace_review_view;

pub(crate) fn enrich_workspace_preview_document(document: &Value) -> Result<Value> {
    let object = document
        .as_object()
        .ok_or_else(|| message("Sync plan document is not a JSON object."))?;
    let mut enriched = object.clone();
    let review_view = build_workspace_review_view(document)?;
    if !enriched.contains_key("actions") {
        enriched.insert(
            "actions".to_string(),
            Value::Array(
                review_view
                    .actions
                    .iter()
                    .map(|action| action.raw.clone())
                    .collect(),
            ),
        );
    }
    if !enriched.contains_key("operations") {
        enriched.insert(
            "operations".to_string(),
            Value::Array(
                review_view
                    .actions
                    .iter()
                    .map(|action| action.raw.clone())
                    .collect(),
            ),
        );
    }
    enriched.insert(
        "domains".to_string(),
        Value::Array(
            review_view
                .domains
                .iter()
                .map(|domain| domain.raw.clone())
                .collect(),
        ),
    );
    enriched.insert(
        "blockedReasons".to_string(),
        Value::Array(
            review_view
                .blocked_reasons
                .iter()
                .map(|reason| Value::String(reason.clone()))
                .collect(),
        ),
    );
    if let Some(summary) = enriched.get_mut("summary").and_then(Value::as_object_mut) {
        summary
            .entry("actionCount".to_string())
            .or_insert(Value::Number(
                (review_view.summary.action_count as i64).into(),
            ));
        summary
            .entry("domainCount".to_string())
            .or_insert(Value::Number(
                (review_view.summary.domain_count as i64).into(),
            ));
        summary
            .entry("sameCount".to_string())
            .or_insert(Value::Number(
                (review_view.summary.same_count as i64).into(),
            ));
        summary
            .entry("blockedCount".to_string())
            .or_insert(Value::Number(
                (review_view.summary.blocked_count as i64).into(),
            ));
        summary
            .entry("warningCount".to_string())
            .or_insert(Value::Number(
                (review_view.summary.warning_count as i64).into(),
            ));
        if !summary.contains_key("blocked_reasons") {
            summary.insert(
                "blocked_reasons".to_string(),
                Value::Array(
                    review_view
                        .blocked_reasons
                        .iter()
                        .map(|reason| Value::String(reason.clone()))
                        .collect(),
                ),
            );
        }
    }
    Ok(Value::Object(enriched))
}
