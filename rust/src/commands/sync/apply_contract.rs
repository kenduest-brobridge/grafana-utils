//! Typed sync apply-intent envelope shared by local builders and live execution.

use super::workbench::SYNC_APPLY_INTENT_KIND;
use crate::common::{message, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SyncApplyOperation {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub identity: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub desired: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncApplyIntentDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    pub mode: String,
    pub reviewed: bool,
    pub review_required: bool,
    pub allow_prune: bool,
    pub approved: bool,
    #[serde(default)]
    pub summary: Value,
    #[serde(default)]
    pub alert_assessment: Value,
    #[serde(default)]
    pub operations: Vec<SyncApplyOperation>,
}

pub(crate) fn load_apply_intent_document(document: &Value) -> Result<SyncApplyIntentDocument> {
    let object = document
        .as_object()
        .ok_or_else(|| message("Sync apply intent document must be a JSON object."))?;
    if object.get("kind").and_then(Value::as_str) != Some(SYNC_APPLY_INTENT_KIND) {
        return Err(message("Sync apply intent document kind is not supported."));
    }
    if !object.contains_key("operations") {
        return Err(message("Sync apply intent document is missing operations."));
    }
    let parsed: SyncApplyIntentDocument =
        serde_json::from_value(document.clone()).map_err(|error| {
            message(format!(
                "Sync apply intent document is not valid JSON: {error}"
            ))
        })?;
    Ok(parsed)
}

pub(crate) fn load_apply_intent_operations(document: &Value) -> Result<Vec<SyncApplyOperation>> {
    let object = document
        .as_object()
        .ok_or_else(|| message("Sync apply intent document must be a JSON object."))?;
    if let Some(kind) = object.get("kind").and_then(Value::as_str) {
        if kind != SYNC_APPLY_INTENT_KIND {
            return Err(message("Sync apply intent document kind is not supported."));
        }
    }
    let operations = object
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync apply intent document is missing operations."))?;
    operations
        .iter()
        .cloned()
        .map(serde_json::from_value::<SyncApplyOperation>)
        .collect::<serde_json::Result<Vec<_>>>()
        .map_err(Into::into)
}
