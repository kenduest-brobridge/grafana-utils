use serde_json::{json, Value};

#[cfg(test)]
use super::{
    build_compare_document, strip_server_managed_fields, CONTACT_POINT_KIND, MUTE_TIMING_KIND,
    POLICIES_KIND, RULE_KIND, TEMPLATE_KIND,
};
#[cfg(test)]
use crate::common::{message, value_as_object, Result};
#[cfg(test)]
use reqwest::Method;
#[cfg(test)]
use serde_json::Map;

#[cfg(test)]
fn request_object_with_request<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    payload: Option<&Value>,
    error_message: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let value = request_json(method, path, &[], payload)?
        .ok_or_else(|| message(error_message.to_string()))?;
    Ok(value_as_object(&value, error_message)?.clone())
}

#[cfg(test)]
fn request_array_with_request<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    payload: Option<&Value>,
    error_message: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    super::expect_object_list(request_json(method, path, &[], payload)?, error_message)
}

#[cfg(test)]
fn request_optional_object_with_request<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    payload: Option<&Value>,
) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(value) = request_json(method, path, &[], payload)? else {
        return Ok(None);
    };
    Ok(Some(
        value_as_object(&value, "Unexpected alert request object response.")?.clone(),
    ))
}

#[cfg(test)]
pub(crate) fn fetch_live_compare_document_with_request<F>(
    mut request_json: F,
    kind: &str,
    payload: &Map<String, Value>,
) -> Result<Option<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if uid.is_empty() {
                return Ok(None);
            }
            Ok(request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                None,
            )?
            .map(|remote| {
                build_compare_document(kind, &strip_server_managed_fields(kind, &remote))
            }))
        }
        CONTACT_POINT_KIND => {
            let uid = super::string_field(payload, "uid", "");
            let remote = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/contact-points",
                None,
                "Unexpected contact-point list response from Grafana.",
            )?
            .into_iter()
            .find(|item| super::string_field(item, "uid", "") == uid);
            Ok(remote.map(|item| {
                build_compare_document(kind, &strip_server_managed_fields(kind, &item))
            }))
        }
        MUTE_TIMING_KIND => {
            let name = super::string_field(payload, "name", "");
            let remote = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/mute-timings",
                None,
                "Unexpected mute-timing list response from Grafana.",
            )?
            .into_iter()
            .find(|item| super::string_field(item, "name", "") == name);
            Ok(remote.map(|item| {
                build_compare_document(kind, &strip_server_managed_fields(kind, &item))
            }))
        }
        TEMPLATE_KIND => {
            let name = super::string_field(payload, "name", "");
            Ok(request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?
            .map(|remote| {
                build_compare_document(kind, &strip_server_managed_fields(kind, &remote))
            }))
        }
        POLICIES_KIND => Ok(request_optional_object_with_request(
            &mut request_json,
            Method::GET,
            "/api/v1/provisioning/policies",
            None,
        )?
        .map(|remote| build_compare_document(kind, &strip_server_managed_fields(kind, &remote)))),
        _ => unreachable!(),
    }
}

#[cfg(test)]
pub(crate) fn determine_import_action_with_request<F>(
    mut request_json: F,
    kind: &str,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if uid.is_empty() {
                return Ok("would-create");
            }
            if request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                None,
            )?
            .is_some()
            {
                if replace_existing {
                    Ok("would-update")
                } else {
                    Ok("would-fail-existing")
                }
            } else {
                Ok("would-create")
            }
        }
        CONTACT_POINT_KIND => {
            let uid = super::string_field(payload, "uid", "");
            let exists = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/contact-points",
                None,
                "Unexpected contact-point list response from Grafana.",
            )?
            .into_iter()
            .any(|item| super::string_field(&item, "uid", "") == uid);
            if exists {
                if replace_existing {
                    Ok("would-update")
                } else {
                    Ok("would-fail-existing")
                }
            } else {
                Ok("would-create")
            }
        }
        MUTE_TIMING_KIND => {
            let name = super::string_field(payload, "name", "");
            let exists = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/mute-timings",
                None,
                "Unexpected mute-timing list response from Grafana.",
            )?
            .into_iter()
            .any(|item| super::string_field(&item, "name", "") == name);
            if exists {
                if replace_existing {
                    Ok("would-update")
                } else {
                    Ok("would-fail-existing")
                }
            } else {
                Ok("would-create")
            }
        }
        TEMPLATE_KIND => {
            let name = super::string_field(payload, "name", "");
            let exists = request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?
            .is_some();
            if exists {
                if replace_existing {
                    Ok("would-update")
                } else {
                    Ok("would-fail-existing")
                }
            } else {
                Ok("would-create")
            }
        }
        POLICIES_KIND => Ok("would-update"),
        _ => unreachable!(),
    }
}

#[cfg(test)]
pub(crate) fn import_resource_document_with_request<F>(
    mut request_json: F,
    kind: &str,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if replace_existing
                && !uid.is_empty()
                && request_optional_object_with_request(
                    &mut request_json,
                    Method::GET,
                    &format!("/api/v1/provisioning/alert-rules/{uid}"),
                    None,
                )?
                .is_some()
            {
                let result = request_object_with_request(
                    &mut request_json,
                    Method::PUT,
                    &format!("/api/v1/provisioning/alert-rules/{uid}"),
                    Some(&Value::Object(payload.clone())),
                    "Unexpected alert-rule update response from Grafana.",
                )?;
                return Ok((
                    "updated".to_string(),
                    super::string_field(&result, "uid", &uid),
                ));
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/alert-rules",
                Some(&Value::Object(payload.clone())),
                "Unexpected alert-rule create response from Grafana.",
            )?;
            Ok((
                "created".to_string(),
                super::string_field(&result, "uid", &uid),
            ))
        }
        CONTACT_POINT_KIND => {
            let uid = super::string_field(payload, "uid", "");
            if replace_existing && !uid.is_empty() {
                let existing: Vec<String> = request_array_with_request(
                    &mut request_json,
                    Method::GET,
                    "/api/v1/provisioning/contact-points",
                    None,
                    "Unexpected contact-point list response from Grafana.",
                )?
                .into_iter()
                .map(|item| super::string_field(&item, "uid", ""))
                .collect();
                if existing.iter().any(|item| item == &uid) {
                    let result = request_object_with_request(
                        &mut request_json,
                        Method::PUT,
                        &format!("/api/v1/provisioning/contact-points/{uid}"),
                        Some(&Value::Object(payload.clone())),
                        "Unexpected contact-point update response from Grafana.",
                    )?;
                    return Ok((
                        "updated".to_string(),
                        super::string_field(&result, "uid", &uid),
                    ));
                }
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/contact-points",
                Some(&Value::Object(payload.clone())),
                "Unexpected contact-point create response from Grafana.",
            )?;
            Ok((
                "created".to_string(),
                super::string_field(&result, "uid", &uid),
            ))
        }
        MUTE_TIMING_KIND => {
            let name = super::string_field(payload, "name", "");
            if replace_existing && !name.is_empty() {
                let existing: Vec<String> = request_array_with_request(
                    &mut request_json,
                    Method::GET,
                    "/api/v1/provisioning/mute-timings",
                    None,
                    "Unexpected mute-timing list response from Grafana.",
                )?
                .into_iter()
                .map(|item| super::string_field(&item, "name", ""))
                .collect();
                if existing.iter().any(|item| item == &name) {
                    let result = request_object_with_request(
                        &mut request_json,
                        Method::PUT,
                        &format!("/api/v1/provisioning/mute-timings/{name}"),
                        Some(&Value::Object(payload.clone())),
                        "Unexpected mute-timing update response from Grafana.",
                    )?;
                    return Ok((
                        "updated".to_string(),
                        super::string_field(&result, "name", &name),
                    ));
                }
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/mute-timings",
                Some(&Value::Object(payload.clone())),
                "Unexpected mute-timing create response from Grafana.",
            )?;
            Ok((
                "created".to_string(),
                super::string_field(&result, "name", &name),
            ))
        }
        TEMPLATE_KIND => {
            let name = super::string_field(payload, "name", "");
            let existing = request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?;
            if existing.is_some() && !replace_existing {
                return Err(message(format!(
                    "Template {name:?} already exists. Use --replace-existing."
                )));
            }
            let mut template_payload = payload.clone();
            if let Some(current) = existing {
                template_payload.insert(
                    "version".to_string(),
                    Value::String(super::string_field(&current, "version", "")),
                );
            } else {
                template_payload.insert("version".to_string(), Value::String(String::new()));
            }
            let mut body = template_payload.clone();
            body.remove("name");
            let result = request_object_with_request(
                &mut request_json,
                Method::PUT,
                &format!("/api/v1/provisioning/templates/{name}"),
                Some(&Value::Object(body)),
                "Unexpected template update response from Grafana.",
            )?;
            Ok((
                (if template_payload
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .is_empty()
                {
                    "created"
                } else {
                    "updated"
                })
                .to_string(),
                super::string_field(&result, "name", &name),
            ))
        }
        POLICIES_KIND => {
            let result = request_object_with_request(
                &mut request_json,
                Method::PUT,
                "/api/v1/provisioning/policies",
                Some(&Value::Object(payload.clone())),
                "Unexpected notification policy update response from Grafana.",
            )?;
            Ok((
                "updated".to_string(),
                super::string_field(&result, "receiver", "root"),
            ))
        }
        _ => unreachable!(),
    }
}

pub fn build_alert_import_dry_run_document(rows: &[Value]) -> Value {
    let processed = rows.len();
    let would_create = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-create"))
        .count();
    let would_update = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-update"))
        .count();
    let would_fail_existing = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-fail-existing"))
        .count();

    json!({
        "summary": {
            "processed": processed,
            "wouldCreate": would_create,
            "wouldUpdate": would_update,
            "wouldFailExisting": would_fail_existing,
        },
        "rows": rows,
    })
}

pub fn build_alert_diff_document(rows: &[Value]) -> Value {
    let checked = rows.len();
    let same = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("same"))
        .count();
    let different = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("different"))
        .count();
    let missing_remote = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("missing-remote"))
        .count();

    json!({
        "summary": {
            "checked": checked,
            "same": same,
            "different": different,
            "missingRemote": missing_remote,
        },
        "rows": rows,
    })
}
