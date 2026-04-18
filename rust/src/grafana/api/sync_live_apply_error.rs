use crate::common::{message, GrafanaCliError};

pub(crate) fn refuse_live_policy_reset() -> GrafanaCliError {
    message("Refusing live notification policy reset without --allow-policy-reset.")
}

pub(crate) fn refuse_live_folder_delete(identity: &str) -> GrafanaCliError {
    message(format!(
        "Refusing live folder delete for {identity} without --allow-folder-delete."
    ))
}

pub(crate) fn unsupported_sync_resource_kind(kind: &str) -> GrafanaCliError {
    message(format!("Unsupported sync resource kind {kind}."))
}

pub(crate) fn unsupported_folder_sync_action(action: &str) -> GrafanaCliError {
    message(format!("Unsupported folder sync action {action}."))
}

pub(crate) fn unsupported_datasource_sync_action(action: &str) -> GrafanaCliError {
    message(format!("Unsupported datasource sync action {action}."))
}

pub(crate) fn unsupported_alert_sync_kind(kind: &str) -> GrafanaCliError {
    message(format!("Unsupported alert sync kind {kind}."))
}

pub(crate) fn unsupported_alert_sync_action(action: &str) -> GrafanaCliError {
    message(format!("Unsupported alert sync action {action}."))
}

pub(crate) fn datasource_sync_target_not_resolved(identity: &str) -> GrafanaCliError {
    message(format!(
        "Could not resolve live datasource target {identity} during sync apply."
    ))
}

pub(crate) fn datasource_sync_requires_live_id(action: &str) -> GrafanaCliError {
    message(format!(
        "Datasource sync {action} requires a live datasource id."
    ))
}

pub(crate) fn alert_sync_delete_requires_uid() -> GrafanaCliError {
    message("Alert sync delete requires a stable uid identity for live apply.")
}

pub(crate) fn alert_sync_live_apply_requires_uid() -> GrafanaCliError {
    message("Alert sync live apply requires alert rule payloads with a uid.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_and_classification_messages_remain_stable() {
        assert_eq!(
            refuse_live_policy_reset().to_string(),
            "Refusing live notification policy reset without --allow-policy-reset."
        );
        assert_eq!(
            refuse_live_folder_delete("folder-1").to_string(),
            "Refusing live folder delete for folder-1 without --allow-folder-delete."
        );
        assert_eq!(
            unsupported_sync_resource_kind("widget").to_string(),
            "Unsupported sync resource kind widget."
        );
        assert_eq!(
            unsupported_folder_sync_action("would-move").to_string(),
            "Unsupported folder sync action would-move."
        );
        assert_eq!(
            unsupported_datasource_sync_action("would-clone").to_string(),
            "Unsupported datasource sync action would-clone."
        );
        assert_eq!(
            unsupported_alert_sync_kind("alert-bundle").to_string(),
            "Unsupported alert sync kind alert-bundle."
        );
        assert_eq!(
            unsupported_alert_sync_action("would-sync").to_string(),
            "Unsupported alert sync action would-sync."
        );
        assert_eq!(
            datasource_sync_target_not_resolved("ds-uid").to_string(),
            "Could not resolve live datasource target ds-uid during sync apply."
        );
        assert_eq!(
            datasource_sync_requires_live_id("update").to_string(),
            "Datasource sync update requires a live datasource id."
        );
        assert_eq!(
            alert_sync_delete_requires_uid().to_string(),
            "Alert sync delete requires a stable uid identity for live apply."
        );
        assert_eq!(
            alert_sync_live_apply_requires_uid().to_string(),
            "Alert sync live apply requires alert rule payloads with a uid."
        );
    }
}
