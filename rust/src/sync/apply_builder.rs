//! Build an executable sync apply intent from a reviewed plan document.
//! This module checks plan kind, enforces review and approval gates, and extracts the
//! runnable operations that are safe to hand to the apply executor. It rejects plans
//! that are unreviewed, unsupported, or missing explicit approval.

use super::apply_contract::{
    load_apply_intent_document, SyncApplyIntentDocument, SyncApplyOperation,
};
use super::json::{require_json_array_field, require_json_object};
use super::workbench::{SYNC_APPLY_INTENT_KIND, SYNC_APPLY_INTENT_SCHEMA_VERSION, SYNC_PLAN_KIND};
use crate::common::{message, tool_version, GrafanaCliError, Result};
use serde_json::Value;

pub fn build_sync_apply_intent_document(plan_document: &Value, approve: bool) -> Result<Value> {
    let plan = require_json_object(plan_document, "Sync plan document")?;
    if plan.get("kind").and_then(Value::as_str) != Some(SYNC_PLAN_KIND) {
        return Err(message("Sync plan document kind is not supported."));
    }
    if plan
        .get("reviewRequired")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !plan
            .get("reviewed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return Err(message(
            "Refusing local sync apply intent before the reviewable plan is marked reviewed.",
        ));
    }
    if !approve {
        return Err(message(
            "Refusing local sync apply intent without explicit approval.",
        ));
    }
    let operations = require_json_array_field(plan, "operations", "Sync plan document")?.clone();
    let executable_operations = operations
        .into_iter()
        .filter(|item| {
            matches!(
                item.get("action").and_then(Value::as_str),
                Some("would-create" | "would-update" | "would-delete")
            )
        })
        .map(serde_json::from_value::<SyncApplyOperation>)
        .collect::<serde_json::Result<Vec<_>>>()?;

    let document = serde_json::to_value(SyncApplyIntentDocument {
        kind: SYNC_APPLY_INTENT_KIND.to_string(),
        schema_version: SYNC_APPLY_INTENT_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        mode: "apply".to_string(),
        reviewed: plan
            .get("reviewed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        review_required: plan
            .get("reviewRequired")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        allow_prune: plan
            .get("allowPrune")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        approved: true,
        summary: plan.get("summary").cloned().unwrap_or(Value::Null),
        alert_assessment: plan.get("alertAssessment").cloned().unwrap_or(Value::Null),
        operations: executable_operations,
    })
    .map_err(GrafanaCliError::from)?;
    load_apply_intent_document(&document)?;
    Ok(document)
}
