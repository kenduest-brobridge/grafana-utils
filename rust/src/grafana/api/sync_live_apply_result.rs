use serde_json::{json, Value};

use crate::sync::live::SyncApplyOperation;

pub(crate) fn normalize_live_apply_result(
    operation: &SyncApplyOperation,
    response: Value,
) -> Value {
    json!({
        "kind": operation.kind.as_str(),
        "identity": operation.identity.as_str(),
        "action": operation.action.as_str(),
        "response": response,
    })
}

pub(crate) fn append_live_apply_result(results: &mut Vec<Value>, result: Value) {
    results.push(result);
}

pub(crate) fn finish_live_apply_response(results: Vec<Value>) -> Value {
    json!({
        "mode": "live-apply",
        "appliedCount": results.len(),
        "results": results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review_contract::REVIEW_ACTION_WOULD_UPDATE;
    use crate::sync::live::SyncApplyOperation;
    use serde_json::json;

    #[test]
    fn normalize_live_apply_result_preserves_operation_identity_and_response() {
        let operation = SyncApplyOperation {
            kind: "dashboard".to_string(),
            identity: "dash-uid".to_string(),
            action: REVIEW_ACTION_WOULD_UPDATE.to_string(),
            desired: serde_json::Map::new(),
        };
        let result = normalize_live_apply_result(&operation, json!({"status":"ok"}));

        assert_eq!(result["kind"], json!("dashboard"));
        assert_eq!(result["identity"], json!("dash-uid"));
        assert_eq!(result["action"], json!("would-update"));
        assert_eq!(result["response"]["status"], json!("ok"));
    }
}
