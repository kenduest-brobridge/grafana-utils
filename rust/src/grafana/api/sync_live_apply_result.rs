use serde_json::{json, Value};

use crate::sync::live::SyncApplyOperation;

pub(crate) fn append_live_apply_result(
    results: &mut Vec<Value>,
    operation: &SyncApplyOperation,
    response: Value,
) {
    results.push(json!({
        "kind": operation.kind.as_str(),
        "identity": operation.identity.as_str(),
        "action": operation.action.as_str(),
        "response": response,
    }));
}

pub(crate) fn finish_live_apply_response(results: Vec<Value>) -> Value {
    json!({
        "mode": "live-apply",
        "appliedCount": results.len(),
        "results": results,
    })
}
