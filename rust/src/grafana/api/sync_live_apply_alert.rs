use serde_json::Value;

use crate::alert::{
    build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_rule_import_payload, build_template_import_payload,
};
use crate::common::Result;
use crate::review_contract::{
    REVIEW_ACTION_WOULD_CREATE, REVIEW_ACTION_WOULD_DELETE, REVIEW_ACTION_WOULD_UPDATE,
};
use crate::sync::live::SyncApplyOperation;

use super::sync_live_apply_error::{
    alert_sync_delete_requires_uid, alert_sync_live_apply_requires_uid,
    unsupported_alert_sync_action, unsupported_alert_sync_kind,
};
use super::SyncLiveClient;

pub(crate) fn apply_alert_operation_with_client(
    client: &SyncLiveClient<'_>,
    operation: &SyncApplyOperation,
) -> Result<Value> {
    let kind = operation.kind.as_str();
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    let desired = &operation.desired;
    match action {
        REVIEW_ACTION_WOULD_DELETE => match kind {
            "alert" => {
                if identity.is_empty() {
                    return Err(alert_sync_delete_requires_uid());
                }
                Ok(client.delete_alert_rule(identity)?)
            }
            "alert-contact-point" => Ok(client.delete_contact_point(identity)?),
            "alert-mute-timing" => Ok(client.delete_mute_timing(identity)?),
            "alert-template" => Ok(client.delete_template(identity)?),
            "alert-policy" => Ok(client.delete_notification_policies()?),
            _ => Err(unsupported_alert_sync_kind(kind)),
        },
        REVIEW_ACTION_WOULD_CREATE | REVIEW_ACTION_WOULD_UPDATE => match kind {
            "alert" => {
                let mut payload = build_rule_import_payload(desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let uid = payload
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .ok_or_else(alert_sync_live_apply_requires_uid)?;
                let response = if action == REVIEW_ACTION_WOULD_CREATE {
                    client.create_alert_rule(&payload)?
                } else {
                    client.update_alert_rule(uid, &payload)?
                };
                Ok(Value::Object(response.into_iter().collect()))
            }
            "alert-contact-point" => {
                let mut payload = build_contact_point_import_payload(desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let response = if action == REVIEW_ACTION_WOULD_CREATE {
                    client.create_contact_point(&payload)?
                } else {
                    client.update_contact_point(identity, &payload)?
                };
                Ok(Value::Object(response.into_iter().collect()))
            }
            "alert-mute-timing" => {
                let payload = build_mute_timing_import_payload(desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(identity);
                let response = if action == REVIEW_ACTION_WOULD_CREATE {
                    client.create_mute_timing(&payload)?
                } else {
                    client.update_mute_timing(name, &payload)?
                };
                Ok(Value::Object(response.into_iter().collect()))
            }
            "alert-policy" => {
                let payload = build_policies_import_payload(desired)?;
                Ok(Value::Object(
                    client
                        .update_notification_policies(&payload)?
                        .into_iter()
                        .collect(),
                ))
            }
            "alert-template" => {
                let mut payload = build_template_import_payload(desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(identity)
                    .to_string();
                payload.remove("name");
                Ok(Value::Object(
                    client
                        .update_template(&name, &payload)?
                        .into_iter()
                        .collect(),
                ))
            }
            _ => Err(unsupported_alert_sync_kind(kind)),
        },
        _ => Err(unsupported_alert_sync_action(action)),
    }
}

#[cfg(test)]
pub(crate) fn apply_alert_operation_with_request<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
) -> Result<Value>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let kind = operation.kind.as_str();
    let action = operation.action.as_str();
    let identity = operation.identity.as_str();
    let desired = &operation.desired;
    match action {
        REVIEW_ACTION_WOULD_DELETE => match kind {
            "alert" => {
                if identity.is_empty() {
                    return Err(alert_sync_delete_requires_uid());
                }
                Ok(request_json(
                    reqwest::Method::DELETE,
                    &format!("/api/v1/provisioning/alert-rules/{identity}"),
                    &[],
                    None,
                )?
                .unwrap_or(Value::Null))
            }
            "alert-contact-point" => Ok(request_json(
                reqwest::Method::DELETE,
                &format!("/api/v1/provisioning/contact-points/{identity}"),
                &[],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-mute-timing" => Ok(request_json(
                reqwest::Method::DELETE,
                &format!("/api/v1/provisioning/mute-timings/{identity}"),
                &[("version".to_string(), String::new())],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-template" => Ok(request_json(
                reqwest::Method::DELETE,
                &format!("/api/v1/provisioning/templates/{identity}"),
                &[("version".to_string(), String::new())],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-policy" => Ok(request_json(
                reqwest::Method::DELETE,
                "/api/v1/provisioning/policies",
                &[],
                None,
            )?
            .unwrap_or(Value::Null)),
            _ => Err(unsupported_alert_sync_kind(kind)),
        },
        REVIEW_ACTION_WOULD_CREATE | REVIEW_ACTION_WOULD_UPDATE => match kind {
            "alert" => {
                let mut payload = build_rule_import_payload(desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let uid = payload
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .ok_or_else(alert_sync_live_apply_requires_uid)?;
                let method = if action == REVIEW_ACTION_WOULD_CREATE {
                    reqwest::Method::POST
                } else {
                    reqwest::Method::PUT
                };
                let path = if action == REVIEW_ACTION_WOULD_CREATE {
                    "/api/v1/provisioning/alert-rules".to_string()
                } else {
                    format!("/api/v1/provisioning/alert-rules/{uid}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-contact-point" => {
                let mut payload = build_contact_point_import_payload(desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let method = if action == REVIEW_ACTION_WOULD_CREATE {
                    reqwest::Method::POST
                } else {
                    reqwest::Method::PUT
                };
                let path = if action == REVIEW_ACTION_WOULD_CREATE {
                    "/api/v1/provisioning/contact-points".to_string()
                } else {
                    format!("/api/v1/provisioning/contact-points/{identity}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-mute-timing" => {
                let payload = build_mute_timing_import_payload(desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(identity);
                let method = if action == REVIEW_ACTION_WOULD_CREATE {
                    reqwest::Method::POST
                } else {
                    reqwest::Method::PUT
                };
                let path = if action == REVIEW_ACTION_WOULD_CREATE {
                    "/api/v1/provisioning/mute-timings".to_string()
                } else {
                    format!("/api/v1/provisioning/mute-timings/{name}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-policy" => {
                let payload = build_policies_import_payload(desired)?;
                Ok(request_json(
                    reqwest::Method::PUT,
                    "/api/v1/provisioning/policies",
                    &[],
                    Some(&Value::Object(payload)),
                )?
                .unwrap_or(Value::Null))
            }
            "alert-template" => {
                let mut payload = build_template_import_payload(desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value: &&str| !value.is_empty())
                    .unwrap_or(identity)
                    .to_string();
                payload.remove("name");
                Ok(request_json(
                    reqwest::Method::PUT,
                    &format!("/api/v1/provisioning/templates/{name}"),
                    &[],
                    Some(&Value::Object(payload)),
                )?
                .unwrap_or(Value::Null))
            }
            _ => Err(unsupported_alert_sync_kind(kind)),
        },
        _ => Err(unsupported_alert_sync_action(action)),
    }
}
