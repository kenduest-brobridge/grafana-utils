//! Merge live sync state fetched from Grafana into local sync documents.
//! This module normalizes alert resources, merges availability and metadata records,
//! and preserves the live fields needed by the sync bundle and apply builders. It is
//! the read-side translation layer, not the final plan or write executor.

use super::super::{
    append_unique_strings, normalize_alert_managed_fields,
    normalize_alert_resource_identity_and_title, require_json_object,
};
use crate::common::{message, Result};
use reqwest::Method;
use serde_json::{Map, Value};
use std::iter::FromIterator;

fn build_live_alert_resource_spec(sync_kind: &str, body: Map<String, Value>) -> Result<Value> {
    let (identity, title) = normalize_alert_resource_identity_and_title(sync_kind, &body)?;
    Ok(serde_json::json!({
        "kind": sync_kind,
        "uid": if sync_kind == "alert-contact-point" { identity.clone() } else { String::new() },
        "name": if matches!(sync_kind, "alert-mute-timing" | "alert-template") { identity.clone() } else { String::new() },
        "title": title,
        "managedFields": normalize_alert_managed_fields(&body),
        "body": body,
    }))
}

pub(crate) fn merge_availability(base: Option<Value>, extra: &Value) -> Result<Value> {
    let mut merged = match base {
        Some(Value::Object(object)) => object,
        Some(_) => {
            return Err(message(
                "Sync availability input file must contain a JSON object.",
            ))
        }
        None => Map::new(),
    };
    let extra_object = require_json_object(extra, "Live availability document")?;
    for (key, value) in extra_object {
        if matches!(
            key.as_str(),
            "datasourceUids" | "datasourceNames" | "pluginIds" | "contactPoints"
        ) {
            let existing = merged
                .remove(key)
                .and_then(|item| item.as_array().cloned())
                .unwrap_or_default();
            let mut combined = existing;
            let extra_items = value
                .as_array()
                .ok_or_else(|| message(format!("Live availability field {key} must be a list.")))?;
            let strings = extra_items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            append_unique_strings(&mut combined, &strings);
            merged.insert(key.clone(), Value::Array(combined));
        } else {
            merged.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(merged))
}

pub(crate) fn fetch_live_resource_specs_with_request<F>(
    mut request_json: F,
    page_size: usize,
) -> Result<Vec<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    // Keep the request-backed fetch isolated here so the higher-level planner
    // can treat live Grafana state as just another normalized spec array.
    let mut specs = Vec::new();
    match request_json(Method::GET, "/api/folders", &[], None)? {
        Some(Value::Array(folders)) => {
            for folder in folders {
                let object = require_json_object(&folder, "Grafana folder payload")?;
                let uid = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                if uid.is_empty() {
                    continue;
                }
                let title = object
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(uid);
                let mut body = Map::new();
                body.insert("title".to_string(), Value::String(title.to_string()));
                if let Some(parent_uid) = object
                    .get("parentUid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    body.insert(
                        "parentUid".to_string(),
                        Value::String(parent_uid.to_string()),
                    );
                }
                specs.push(serde_json::json!({
                    "kind": "folder",
                    "uid": uid,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected folder list response from Grafana.")),
        None => {}
    }

    let mut page = 1usize;
    loop {
        let params = vec![
            ("type".to_string(), "dash-db".to_string()),
            ("limit".to_string(), page_size.to_string()),
            ("page".to_string(), page.to_string()),
        ];
        let batch = match request_json(Method::GET, "/api/search", &params, None)? {
            Some(Value::Array(items)) => items,
            Some(_) => return Err(message("Unexpected search response from Grafana.")),
            None => Vec::new(),
        };
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        for item in batch {
            let summary = require_json_object(&item, "Grafana dashboard summary")?;
            let uid = summary
                .get("uid")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if uid.is_empty() {
                continue;
            }
            let dashboard_wrapper = match request_json(
                Method::GET,
                &format!("/api/dashboards/uid/{uid}"),
                &[],
                None,
            )? {
                Some(value) => value,
                None => continue,
            };
            let wrapper = require_json_object(&dashboard_wrapper, "Grafana dashboard payload")?;
            let dashboard = wrapper
                .get("dashboard")
                .ok_or_else(|| message(format!("Unexpected dashboard payload for UID {uid}.")))?;
            let body = require_json_object(dashboard, "Grafana dashboard body")?;
            let mut normalized = body.clone();
            normalized.remove("id");
            let title = normalized
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(uid);
            specs.push(serde_json::json!({
                "kind": "dashboard",
                "uid": uid,
                "title": title,
                "body": normalized,
            }));
        }
        if batch_len < page_size {
            break;
        }
        page += 1;
    }

    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(datasources)) => {
            for datasource in datasources {
                let object = require_json_object(&datasource, "Grafana datasource payload")?;
                let uid = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                let name = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                if uid.is_empty() && name.is_empty() {
                    continue;
                }
                let title = if name.is_empty() { uid } else { name };
                let mut body = Map::new();
                body.insert("uid".to_string(), Value::String(uid.to_string()));
                body.insert("name".to_string(), Value::String(title.to_string()));
                body.insert(
                    "type".to_string(),
                    object
                        .get("type")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "access".to_string(),
                    object
                        .get("access")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "url".to_string(),
                    object
                        .get("url")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "isDefault".to_string(),
                    object
                        .get("isDefault")
                        .cloned()
                        .unwrap_or(Value::Bool(false)),
                );
                if let Some(json_data) = object.get("jsonData").and_then(Value::as_object) {
                    if !json_data.is_empty() {
                        body.insert("jsonData".to_string(), Value::Object(json_data.clone()));
                    }
                }
                specs.push(serde_json::json!({
                    "kind": "datasource",
                    "uid": uid,
                    "name": title,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/alert-rules", &[], None)? {
        Some(Value::Array(rules)) => {
            for rule in rules {
                let object = require_json_object(&rule, "Grafana alert-rule payload")?;
                let body = crate::alert::build_rule_import_payload(object)?;
                let uid = body
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| message("Live alert rule payload is missing uid."))?;
                let title = body
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(uid);
                specs.push(serde_json::json!({
                    "kind": "alert",
                    "uid": uid,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected alert-rule list response from Grafana.")),
        None => {}
    }

    match request_json(
        Method::GET,
        "/api/v1/provisioning/contact-points",
        &[],
        None,
    )? {
        Some(Value::Array(contact_points)) => {
            for contact_point in contact_points {
                let object = require_json_object(&contact_point, "Grafana contact-point payload")?;
                specs.push(build_live_alert_resource_spec(
                    "alert-contact-point",
                    object.clone(),
                )?);
            }
        }
        Some(_) => {
            return Err(message(
                "Unexpected contact-point list response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/mute-timings", &[], None)? {
        Some(Value::Array(mute_timings)) => {
            for mute_timing in mute_timings {
                let object = require_json_object(&mute_timing, "Grafana mute-timing payload")?;
                specs.push(build_live_alert_resource_spec(
                    "alert-mute-timing",
                    object.clone(),
                )?);
            }
        }
        Some(_) => {
            return Err(message(
                "Unexpected mute-timing list response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/policies", &[], None)? {
        Some(Value::Object(policies)) => {
            specs.push(build_live_alert_resource_spec(
                "alert-policy",
                policies.clone(),
            )?);
        }
        Some(_) => {
            return Err(message(
                "Unexpected notification policy response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/templates", &[], None)? {
        Some(Value::Array(templates)) => {
            for template in templates {
                let object = require_json_object(&template, "Grafana template summary payload")?;
                let name = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| message("Live template payload is missing name."))?;
                let template_payload = match request_json(
                    Method::GET,
                    &format!("/api/v1/provisioning/templates/{name}"),
                    &[],
                    None,
                )? {
                    Some(Value::Object(template_object)) => template_object,
                    Some(_) => return Err(message("Unexpected template payload from Grafana.")),
                    None => continue,
                };
                specs.push(build_live_alert_resource_spec(
                    "alert-template",
                    template_payload,
                )?);
            }
        }
        Some(_) => return Err(message("Unexpected template list response from Grafana.")),
        None => {}
    }

    Ok(specs)
}

pub(crate) fn fetch_live_availability_with_request<F>(mut request_json: F) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    // Build a compact availability snapshot that only tracks the identifiers
    // preflight needs to validate, not the full live payloads.
    let mut availability = Map::from_iter(vec![
        ("datasourceUids".to_string(), Value::Array(Vec::new())),
        ("datasourceNames".to_string(), Value::Array(Vec::new())),
        ("pluginIds".to_string(), Value::Array(Vec::new())),
        ("contactPoints".to_string(), Value::Array(Vec::new())),
    ]);

    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(datasources)) => {
            let mut uids = Vec::new();
            let mut names = Vec::new();
            for datasource in datasources {
                let object = require_json_object(&datasource, "Grafana datasource payload")?;
                if let Some(uid) = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    uids.push(uid.to_string());
                }
                if let Some(name) = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    names.push(name.to_string());
                }
            }
            append_unique_strings(
                availability
                    .get_mut("datasourceUids")
                    .and_then(Value::as_array_mut)
                    .expect("datasourceUids should be array"),
                &uids,
            );
            append_unique_strings(
                availability
                    .get_mut("datasourceNames")
                    .and_then(Value::as_array_mut)
                    .expect("datasourceNames should be array"),
                &names,
            );
        }
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => {}
    }

    match request_json(Method::GET, "/api/plugins", &[], None)? {
        Some(Value::Array(plugins)) => {
            let ids = plugins
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|plugin| plugin.get("id").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            append_unique_strings(
                availability
                    .get_mut("pluginIds")
                    .and_then(Value::as_array_mut)
                    .expect("pluginIds should be array"),
                &ids,
            );
        }
        Some(_) => return Err(message("Unexpected plugin list response from Grafana.")),
        None => {}
    }

    match request_json(
        Method::GET,
        "/api/v1/provisioning/contact-points",
        &[],
        None,
    )? {
        Some(Value::Array(contact_points)) => {
            let mut names = Vec::new();
            for item in contact_points {
                let object = require_json_object(&item, "Grafana contact-point payload")?;
                if let Some(name) = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    names.push(name.to_string());
                }
                if let Some(uid) = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    names.push(uid.to_string());
                }
            }
            append_unique_strings(
                availability
                    .get_mut("contactPoints")
                    .and_then(Value::as_array_mut)
                    .expect("contactPoints should be array"),
                &names,
            );
        }
        Some(_) => {
            return Err(message(
                "Unexpected contact-point list response from Grafana.",
            ))
        }
        None => {}
    }

    Ok(Value::Object(availability))
}
