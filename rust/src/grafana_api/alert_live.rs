#![allow(dead_code)]

use crate::alert::{
    build_compare_document, build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_rule_import_payload, build_template_import_payload,
    normalize_compare_payload, CONTACT_POINT_KIND, MUTE_TIMING_KIND, POLICIES_KIND, RULE_KIND,
    TEMPLATE_KIND,
};
use crate::common::{message, string_field, value_as_object, Result};
use crate::grafana_api::parse_template_list_response;
use reqwest::Method;
use serde_json::{Map, Value};

#[allow(dead_code)]
#[cfg(test)]
pub(crate) fn request_object_with_request<F>(
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

pub(crate) fn request_array_with_request<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    payload: Option<&Value>,
    error_message: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    crate::grafana_api::expect_object_list(request_json(method, path, &[], payload)?, error_message)
}

pub(crate) fn request_optional_object_with_request<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    payload: Option<&Value>,
) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let value = match request_json(method, path, &[], payload) {
        Ok(value) => value,
        Err(error) if error.status_code() == Some(404) => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(value) = value else {
        return Ok(None);
    };
    Ok(Some(
        value_as_object(&value, "Unexpected alert request object response.")?.clone(),
    ))
}

fn request_template_list_with_request<F>(request_json: &mut F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    parse_template_list_response(request_json(
        Method::GET,
        "/api/v1/provisioning/templates",
        &[],
        None,
    )?)
}

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
            let uid = string_field(payload, "uid", "");
            if uid.is_empty() {
                return Ok(None);
            }
            Ok(request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                None,
            )?
            .map(|remote| build_compare_document(kind, &normalize_compare_payload(kind, &remote))))
        }
        CONTACT_POINT_KIND => {
            let uid = string_field(payload, "uid", "");
            let remote = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/contact-points",
                None,
                "Unexpected contact-point list response from Grafana.",
            )?
            .into_iter()
            .find(|item| string_field(item, "uid", "") == uid);
            Ok(remote
                .map(|item| build_compare_document(kind, &normalize_compare_payload(kind, &item))))
        }
        MUTE_TIMING_KIND => {
            let name = string_field(payload, "name", "");
            let remote = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/mute-timings",
                None,
                "Unexpected mute-timing list response from Grafana.",
            )?
            .into_iter()
            .find(|item| string_field(item, "name", "") == name);
            Ok(remote
                .map(|item| build_compare_document(kind, &normalize_compare_payload(kind, &item))))
        }
        TEMPLATE_KIND => {
            let name = string_field(payload, "name", "");
            Ok(request_optional_object_with_request(
                &mut request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?
            .map(|remote| build_compare_document(kind, &normalize_compare_payload(kind, &remote))))
        }
        POLICIES_KIND => Ok(request_optional_object_with_request(
            &mut request_json,
            Method::GET,
            "/api/v1/provisioning/policies",
            None,
        )?
        .map(|remote| build_compare_document(kind, &normalize_compare_payload(kind, &remote)))),
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
            let uid = string_field(payload, "uid", "");
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
            let uid = string_field(payload, "uid", "");
            let exists = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/contact-points",
                None,
                "Unexpected contact-point list response from Grafana.",
            )?
            .into_iter()
            .any(|item| string_field(&item, "uid", "") == uid);
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
            let name = string_field(payload, "name", "");
            let exists = request_array_with_request(
                &mut request_json,
                Method::GET,
                "/api/v1/provisioning/mute-timings",
                None,
                "Unexpected mute-timing list response from Grafana.",
            )?
            .into_iter()
            .any(|item| string_field(&item, "name", "") == name);
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
            let name = string_field(payload, "name", "");
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
            let uid = string_field(payload, "uid", "");
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
                return Ok(("updated".to_string(), string_field(&result, "uid", &uid)));
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/alert-rules",
                Some(&Value::Object(payload.clone())),
                "Unexpected alert-rule create response from Grafana.",
            )?;
            Ok(("created".to_string(), string_field(&result, "uid", &uid)))
        }
        CONTACT_POINT_KIND => {
            let uid = string_field(payload, "uid", "");
            if replace_existing && !uid.is_empty() {
                let existing: Vec<String> = request_array_with_request(
                    &mut request_json,
                    Method::GET,
                    "/api/v1/provisioning/contact-points",
                    None,
                    "Unexpected contact-point list response from Grafana.",
                )?
                .into_iter()
                .map(|item| string_field(&item, "uid", ""))
                .collect();
                if existing.iter().any(|item| item == &uid) {
                    let result = request_object_with_request(
                        &mut request_json,
                        Method::PUT,
                        &format!("/api/v1/provisioning/contact-points/{uid}"),
                        Some(&Value::Object(payload.clone())),
                        "Unexpected contact-point update response from Grafana.",
                    )?;
                    return Ok(("updated".to_string(), string_field(&result, "uid", &uid)));
                }
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/contact-points",
                Some(&Value::Object(payload.clone())),
                "Unexpected contact-point create response from Grafana.",
            )?;
            Ok(("created".to_string(), string_field(&result, "uid", &uid)))
        }
        MUTE_TIMING_KIND => {
            let name = string_field(payload, "name", "");
            if replace_existing && !name.is_empty() {
                let existing: Vec<String> = request_array_with_request(
                    &mut request_json,
                    Method::GET,
                    "/api/v1/provisioning/mute-timings",
                    None,
                    "Unexpected mute-timing list response from Grafana.",
                )?
                .into_iter()
                .map(|item| string_field(&item, "name", ""))
                .collect();
                if existing.iter().any(|item| item == &name) {
                    let result = request_object_with_request(
                        &mut request_json,
                        Method::PUT,
                        &format!("/api/v1/provisioning/mute-timings/{name}"),
                        Some(&Value::Object(payload.clone())),
                        "Unexpected mute-timing update response from Grafana.",
                    )?;
                    return Ok(("updated".to_string(), string_field(&result, "name", &name)));
                }
            }
            let result = request_object_with_request(
                &mut request_json,
                Method::POST,
                "/api/v1/provisioning/mute-timings",
                Some(&Value::Object(payload.clone())),
                "Unexpected mute-timing create response from Grafana.",
            )?;
            Ok(("created".to_string(), string_field(&result, "name", &name)))
        }
        TEMPLATE_KIND => {
            let name = string_field(payload, "name", "");
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
                    Value::String(string_field(&current, "version", "")),
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
            let status = if template_payload
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                "created"
            } else {
                "updated"
            };
            Ok((status.to_string(), string_field(&result, "name", &name)))
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
                string_field(&result, "receiver", "root"),
            ))
        }
        _ => unreachable!(),
    }
}

pub(crate) fn request_live_resources_by_kind_with_request<F>(
    request_json: &mut F,
    kind: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => request_array_with_request(
            request_json,
            Method::GET,
            "/api/v1/provisioning/alert-rules",
            None,
            "Unexpected alert-rule list response from Grafana.",
        ),
        CONTACT_POINT_KIND => request_array_with_request(
            request_json,
            Method::GET,
            "/api/v1/provisioning/contact-points",
            None,
            "Unexpected contact-point list response from Grafana.",
        ),
        MUTE_TIMING_KIND => request_array_with_request(
            request_json,
            Method::GET,
            "/api/v1/provisioning/mute-timings",
            None,
            "Unexpected mute-timing list response from Grafana.",
        ),
        TEMPLATE_KIND => request_template_list_with_request(request_json),
        POLICIES_KIND => Ok(request_optional_object_with_request(
            request_json,
            Method::GET,
            "/api/v1/provisioning/policies",
            None,
        )?
        .into_iter()
        .collect()),
        _ => unreachable!(),
    }
}

pub(crate) fn apply_create_with_request<F>(
    request_json: &mut F,
    kind: &str,
    payload: &Map<String, Value>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => Ok(request_json(
            Method::POST,
            "/api/v1/provisioning/alert-rules",
            &[],
            Some(&Value::Object(build_rule_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        CONTACT_POINT_KIND => Ok(request_json(
            Method::POST,
            "/api/v1/provisioning/contact-points",
            &[],
            Some(&Value::Object(build_contact_point_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        MUTE_TIMING_KIND => Ok(request_json(
            Method::POST,
            "/api/v1/provisioning/mute-timings",
            &[],
            Some(&Value::Object(build_mute_timing_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        TEMPLATE_KIND => {
            let mut template_payload = build_template_import_payload(payload)?;
            let name = string_field(&template_payload, "name", "");
            template_payload.insert("version".to_string(), Value::String(String::new()));
            template_payload.remove("name");
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[],
                Some(&Value::Object(template_payload)),
            )?
            .unwrap_or(Value::Null))
        }
        POLICIES_KIND => Ok(request_json(
            Method::PUT,
            "/api/v1/provisioning/policies",
            &[],
            Some(&Value::Object(build_policies_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        _ => Err(message(format!("Unsupported alert create kind {kind}."))),
    }
}

pub(crate) fn apply_update_with_request<F>(
    request_json: &mut F,
    kind: &str,
    identity: &str,
    payload: &Map<String, Value>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => {
            let mut body = build_rule_import_payload(payload)?;
            if !body.contains_key("uid") && !identity.is_empty() {
                body.insert("uid".to_string(), Value::String(identity.to_string()));
            }
            let uid = string_field(&body, "uid", identity);
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        CONTACT_POINT_KIND => {
            let mut body = build_contact_point_import_payload(payload)?;
            if !body.contains_key("uid") && !identity.is_empty() {
                body.insert("uid".to_string(), Value::String(identity.to_string()));
            }
            let uid = string_field(&body, "uid", identity);
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/contact-points/{uid}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        MUTE_TIMING_KIND => {
            let body = build_mute_timing_import_payload(payload)?;
            let name = string_field(&body, "name", identity);
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/mute-timings/{name}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        TEMPLATE_KIND => {
            let mut body = build_template_import_payload(payload)?;
            let name = string_field(&body, "name", identity);
            let existing = request_optional_object_with_request(
                &mut *request_json,
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                None,
            )?;
            body.insert(
                "version".to_string(),
                Value::String(
                    existing
                        .as_ref()
                        .map(|item| string_field(item, "version", ""))
                        .unwrap_or_default(),
                ),
            );
            body.remove("name");
            Ok(request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        POLICIES_KIND => Ok(request_json(
            Method::PUT,
            "/api/v1/provisioning/policies",
            &[],
            Some(&Value::Object(build_policies_import_payload(payload)?)),
        )?
        .unwrap_or(Value::Null)),
        _ => Err(message(format!("Unsupported alert update kind {kind}."))),
    }
}

pub(crate) fn apply_delete_with_request<F>(
    request_json: &mut F,
    kind: &str,
    identity: &str,
    allow_policy_reset: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match kind {
        RULE_KIND => Ok(request_json(
            Method::DELETE,
            &format!("/api/v1/provisioning/alert-rules/{identity}"),
            &[],
            None,
        )?
        .unwrap_or(Value::Null)),
        CONTACT_POINT_KIND => Ok(request_json(
            Method::DELETE,
            &format!("/api/v1/provisioning/contact-points/{identity}"),
            &[],
            None,
        )?
        .unwrap_or(Value::Null)),
        MUTE_TIMING_KIND => Ok(request_json(
            Method::DELETE,
            &format!("/api/v1/provisioning/mute-timings/{identity}"),
            &[("version".to_string(), String::new())],
            None,
        )?
        .unwrap_or(Value::Null)),
        TEMPLATE_KIND => Ok(request_json(
            Method::DELETE,
            &format!("/api/v1/provisioning/templates/{identity}"),
            &[("version".to_string(), String::new())],
            None,
        )?
        .unwrap_or(Value::Null)),
        POLICIES_KIND => {
            if !allow_policy_reset {
                return Err(message(
                    "Refusing live notification policy reset without --allow-policy-reset.",
                ));
            }
            Ok(
                request_json(Method::DELETE, "/api/v1/provisioning/policies", &[], None)?
                    .unwrap_or(Value::Null),
            )
        }
        _ => Err(message(format!("Unsupported alert delete kind {kind}."))),
    }
}
