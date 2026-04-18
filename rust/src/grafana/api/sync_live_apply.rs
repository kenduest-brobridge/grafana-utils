use serde_json::{Map, Value};

use crate::common::Result;
use crate::review_contract::{
    REVIEW_ACTION_WOULD_CREATE, REVIEW_ACTION_WOULD_DELETE, REVIEW_ACTION_WOULD_UPDATE,
};
use crate::sync::live::SyncApplyOperation;

use super::sync_live_apply_alert::apply_alert_operation_with_client;
use super::sync_live_apply_datasource::{
    resolve_live_datasource_id, resolve_live_datasource_target,
};
use super::sync_live_apply_error::{
    datasource_sync_target_not_resolved, refuse_live_folder_delete,
    unsupported_datasource_sync_action, unsupported_folder_sync_action,
    unsupported_sync_resource_kind,
};
use super::sync_live_apply_phase::execute_live_apply_phase;
use super::SyncLiveClient;

#[cfg(test)]
#[path = "sync_live_apply_request.rs"]
mod sync_live_apply_request;
#[cfg(test)]
pub(crate) use sync_live_apply_request::execute_live_apply_with_request;

impl<'a> SyncLiveClient<'a> {
    pub(crate) fn create_folder(
        &self,
        title: &str,
        uid: &str,
        parent_uid: Option<&str>,
    ) -> Result<Map<String, Value>> {
        self.api
            .dashboard()
            .create_folder_entry(title, uid, parent_uid)
    }

    pub(crate) fn update_folder(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.dashboard().update_folder_request(uid, payload)
    }

    pub(crate) fn delete_folder(&self, uid: &str) -> Result<Value> {
        Ok(Value::Object(
            self.api
                .dashboard()
                .delete_folder_request(uid)?
                .into_iter()
                .collect(),
        ))
    }

    pub(crate) fn upsert_dashboard(
        &self,
        payload: &Map<String, Value>,
        overwrite: bool,
        folder_uid: Option<&str>,
    ) -> Result<Value> {
        let mut body = Map::new();
        body.insert("dashboard".to_string(), Value::Object(payload.clone()));
        body.insert("overwrite".to_string(), Value::Bool(overwrite));
        if let Some(folder_uid) = folder_uid.filter(|value: &&str| !value.is_empty()) {
            body.insert(
                "folderUid".to_string(),
                Value::String(folder_uid.to_string()),
            );
        }
        self.api
            .dashboard()
            .import_dashboard_request(&Value::Object(body))
    }

    pub(crate) fn delete_dashboard(&self, uid: &str) -> Result<Value> {
        Ok(Value::Object(
            self.api
                .dashboard()
                .delete_dashboard_request(uid)?
                .into_iter()
                .collect(),
        ))
    }

    pub(crate) fn resolve_datasource_target(
        &self,
        identity: &str,
    ) -> Result<Option<Map<String, Value>>> {
        resolve_live_datasource_target(&self.list_datasources()?, identity)
    }

    pub(crate) fn create_datasource(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.datasource().create_datasource(payload)
    }

    pub(crate) fn update_datasource(
        &self,
        datasource_id: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api
            .datasource()
            .update_datasource(datasource_id, payload)
    }

    pub(crate) fn delete_datasource(&self, datasource_id: &str) -> Result<Value> {
        self.api.datasource().delete_datasource(datasource_id)
    }

    pub(crate) fn create_alert_rule(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().create_alert_rule(payload)
    }

    pub(crate) fn update_alert_rule(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_alert_rule(uid, payload)
    }

    pub(crate) fn delete_alert_rule(&self, uid: &str) -> Result<Value> {
        self.api.alerting().delete_alert_rule(uid)
    }

    pub(crate) fn create_contact_point(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().create_contact_point(payload)
    }

    pub(crate) fn update_contact_point(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_contact_point(uid, payload)
    }

    pub(crate) fn delete_contact_point(&self, uid: &str) -> Result<Value> {
        self.api.alerting().delete_contact_point(uid)
    }

    pub(crate) fn create_mute_timing(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().create_mute_timing(payload)
    }

    pub(crate) fn update_mute_timing(
        &self,
        name: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_mute_timing(name, payload)
    }

    pub(crate) fn delete_mute_timing(&self, name: &str) -> Result<Value> {
        self.api.alerting().delete_mute_timing(name)
    }

    pub(crate) fn update_notification_policies(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_notification_policies(payload)
    }

    pub(crate) fn delete_notification_policies(&self) -> Result<Value> {
        self.api.alerting().delete_notification_policies()
    }

    pub(crate) fn update_template(
        &self,
        name: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.api.alerting().update_template(name, payload)
    }

    pub(crate) fn delete_template(&self, name: &str) -> Result<Value> {
        self.api.alerting().delete_template(name)
    }

    pub(crate) fn execute_live_apply(
        &self,
        operations: &[SyncApplyOperation],
        allow_folder_delete: bool,
        allow_policy_reset: bool,
    ) -> Result<Value> {
        execute_live_apply_phase(operations, allow_policy_reset, |operation| {
            apply_live_operation_with_client(self, operation, allow_folder_delete)
        })
    }
}

pub(crate) fn execute_live_apply_with_client(
    client: &SyncLiveClient<'_>,
    operations: &[SyncApplyOperation],
    allow_folder_delete: bool,
    allow_policy_reset: bool,
) -> Result<Value> {
    client.execute_live_apply(operations, allow_folder_delete, allow_policy_reset)
}

fn apply_live_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
    allow_folder_delete: bool,
) -> Result<Value> {
    let kind = operation.kind.as_str();
    match kind {
        "folder" => apply_folder_operation_with_client(client, operation, allow_folder_delete),
        "dashboard" => apply_dashboard_operation_with_client(client, operation),
        "datasource" => apply_datasource_operation_with_client(client, operation),
        "alert"
        | "alert-contact-point"
        | "alert-mute-timing"
        | "alert-policy"
        | "alert-template" => apply_alert_operation_with_client(client, operation),
        _ => Err(unsupported_sync_resource_kind(kind)),
    }
}

fn apply_folder_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
    allow_folder_delete: bool,
) -> Result<Value> {
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    let desired = &operation.desired;
    match action {
        REVIEW_ACTION_WOULD_CREATE => {
            let title = desired
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value: &&str| !value.is_empty())
                .unwrap_or(identity);
            let parent_uid = desired
                .get("parentUid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value: &&str| !value.is_empty());
            Ok(Value::Object(
                client
                    .create_folder(title, identity, parent_uid)?
                    .into_iter()
                    .collect(),
            ))
        }
        REVIEW_ACTION_WOULD_UPDATE => Ok(Value::Object(
            client
                .update_folder(identity, desired)?
                .into_iter()
                .collect(),
        )),
        REVIEW_ACTION_WOULD_DELETE => {
            if !allow_folder_delete {
                return Err(refuse_live_folder_delete(identity));
            }
            Ok(client.delete_folder(identity)?)
        }
        _ => Err(unsupported_folder_sync_action(action)),
    }
}

fn apply_dashboard_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
) -> Result<Value> {
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    if action == REVIEW_ACTION_WOULD_DELETE {
        return client.delete_dashboard(identity);
    }
    let mut body = operation.desired.clone();
    body.insert("uid".to_string(), Value::String(identity.to_string()));
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .unwrap_or(identity);
    body.insert("title".to_string(), Value::String(title.to_string()));
    body.remove("id");
    let folder_uid = body
        .get("folderUid")
        .or_else(|| body.get("folderUID"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty());
    client.upsert_dashboard(&body, action == REVIEW_ACTION_WOULD_UPDATE, folder_uid)
}

fn apply_datasource_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
) -> Result<Value> {
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    let mut body = operation.desired.clone();
    if !identity.is_empty() {
        body.entry("uid".to_string())
            .or_insert_with(|| Value::String(identity.to_string()));
    }
    let title = body
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value: &&str| !value.is_empty())
        .unwrap_or(identity);
    body.insert("name".to_string(), Value::String(title.to_string()));
    match action {
        REVIEW_ACTION_WOULD_CREATE => Ok(Value::Object(
            client.create_datasource(&body)?.into_iter().collect(),
        )),
        REVIEW_ACTION_WOULD_UPDATE => {
            let target = client
                .resolve_datasource_target(identity)?
                .ok_or_else(|| datasource_sync_target_not_resolved(identity))?;
            let datasource_id = resolve_live_datasource_id(&target, "update")?;
            Ok(Value::Object(
                client
                    .update_datasource(&datasource_id, &body)?
                    .into_iter()
                    .collect(),
            ))
        }
        REVIEW_ACTION_WOULD_DELETE => {
            let target = client
                .resolve_datasource_target(identity)?
                .ok_or_else(|| datasource_sync_target_not_resolved(identity))?;
            let datasource_id = resolve_live_datasource_id(&target, "delete")?;
            Ok(client.delete_datasource(&datasource_id)?)
        }
        _ => Err(unsupported_datasource_sync_action(action)),
    }
}
