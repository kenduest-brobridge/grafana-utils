use serde_json::Value;

use crate::common::Result;
use crate::review_contract::REVIEW_ACTION_WOULD_DELETE;
use crate::sync::live::SyncApplyOperation;

use super::sync_live_apply_error::refuse_live_policy_reset;
use super::sync_live_apply_result::{
    append_live_apply_result, finish_live_apply_response, normalize_live_apply_result,
};

pub(crate) fn execute_live_apply_phase<F>(
    operations: &[SyncApplyOperation],
    allow_policy_reset: bool,
    mut apply_operation: F,
) -> Result<Value>
where
    F: FnMut(&SyncApplyOperation) -> Result<Value>,
{
    let mut results = Vec::new();
    for operation in operations {
        if operation.kind == "alert-policy"
            && operation.action == REVIEW_ACTION_WOULD_DELETE
            && !allow_policy_reset
        {
            return Err(refuse_live_policy_reset());
        }
        let response = apply_operation(operation)?;
        let normalized = normalize_live_apply_result(operation, response);
        append_live_apply_result(&mut results, normalized);
    }
    Ok(finish_live_apply_response(results))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review_contract::{REVIEW_ACTION_WOULD_DELETE, REVIEW_ACTION_WOULD_UPDATE};
    use serde_json::json;

    fn operation(kind: &str, action: &str, identity: &str) -> SyncApplyOperation {
        SyncApplyOperation {
            kind: kind.to_string(),
            identity: identity.to_string(),
            action: action.to_string(),
            desired: serde_json::Map::new(),
        }
    }

    #[test]
    fn phase_preserves_operation_order_and_results() {
        let operations = vec![operation("dashboard", REVIEW_ACTION_WOULD_UPDATE, "dash-a")];
        let result = execute_live_apply_phase(&operations, false, |op| {
            Ok(json!({
                "kind": op.kind,
                "identity": op.identity,
            }))
        })
        .unwrap();

        assert_eq!(result["mode"], json!("live-apply"));
        assert_eq!(result["appliedCount"], json!(1));
        assert_eq!(result["results"][0]["kind"], json!("dashboard"));
        assert_eq!(result["results"][0]["identity"], json!("dash-a"));
    }

    #[test]
    fn phase_blocks_policy_reset_when_not_allowed() {
        let operations = vec![operation(
            "alert-policy",
            REVIEW_ACTION_WOULD_DELETE,
            "policies",
        )];
        let result =
            execute_live_apply_phase(&operations, false, |_| Ok(json!({"should_not_run": true})));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Refusing live notification policy reset without --allow-policy-reset."
        );
    }
}
