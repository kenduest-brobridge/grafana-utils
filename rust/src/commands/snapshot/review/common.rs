//! Shared validation and field access for snapshot review documents.

use serde_json::{Map, Value};

use crate::common::{message, Result};

pub(super) fn review_summary(document: &Value) -> Result<&Map<String, Value>> {
    if document.get("kind").and_then(Value::as_str) != Some(super::super::SNAPSHOT_REVIEW_KIND) {
        return Err(message("Snapshot review document kind is not supported."));
    }
    document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Snapshot review document is missing summary."))
}

pub(super) fn review_warnings(document: &Value) -> Vec<Value> {
    document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
