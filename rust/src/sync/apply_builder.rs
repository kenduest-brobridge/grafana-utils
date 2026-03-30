use super::json::{require_json_array_field, require_json_object};
use super::workbench::{SYNC_APPLY_INTENT_KIND, SYNC_APPLY_INTENT_SCHEMA_VERSION, SYNC_PLAN_KIND};
use crate::common::{message, tool_version, Result};
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
        .collect::<Vec<Value>>();
    Ok(serde_json::json!({
        "kind": SYNC_APPLY_INTENT_KIND,
        "schemaVersion": SYNC_APPLY_INTENT_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "mode": "apply",
        "reviewed": plan.get("reviewed").cloned().unwrap_or(Value::Bool(false)),
        "reviewRequired": plan.get("reviewRequired").cloned().unwrap_or(Value::Bool(true)),
        "allowPrune": plan.get("allowPrune").cloned().unwrap_or(Value::Bool(false)),
        "approved": true,
        "summary": plan.get("summary").cloned().unwrap_or(Value::Null),
        "alertAssessment": plan.get("alertAssessment").cloned().unwrap_or(Value::Null),
        "operations": executable_operations,
    }))
}
