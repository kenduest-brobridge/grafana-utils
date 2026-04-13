//! Alert sync-spec normalization for source-bundle inputs.

use super::bundle_inputs_alert_registry::{
    alert_sync_kind_spec, alert_sync_kind_spec_for_document_kind, AlertSyncKindSpec,
};
use crate::alert::{build_rule_import_payload, detect_document_kind};
use crate::common::{message, Result};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

fn add_non_empty_text_field(
    body: &mut Map<String, Value>,
    managed_fields: &mut Vec<String>,
    field: &str,
    value: &str,
) {
    let normalized = value.trim();
    if normalized.is_empty() {
        return;
    }
    body.insert(field.to_string(), Value::String(normalized.to_string()));
    managed_fields.push(field.to_string());
}

fn extract_rule_dependency_lists(
    rule: &Map<String, Value>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut datasource_uids = BTreeSet::new();
    let mut datasource_names = BTreeSet::new();
    let mut plugin_ids = BTreeSet::new();
    for item in rule
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(object) = item.as_object() else {
            continue;
        };
        for (key, sink) in [
            ("datasourceUid", &mut datasource_uids),
            ("datasourceName", &mut datasource_names),
            ("datasourceType", &mut plugin_ids),
        ] {
            if let Some(value) = object
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                sink.insert(value.to_string());
            }
        }
        if let Some(datasource) = object
            .get("model")
            .and_then(Value::as_object)
            .and_then(|model| model.get("datasource"))
            .and_then(Value::as_object)
        {
            if let Some(uid) = datasource
                .get("uid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                datasource_uids.insert(uid.to_string());
            }
            if let Some(name) = datasource
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                datasource_names.insert(name.to_string());
            }
            if let Some(ds_type) = datasource
                .get("type")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                plugin_ids.insert(ds_type.to_string());
            }
        }
    }
    (
        datasource_uids.into_iter().collect(),
        datasource_names.into_iter().collect(),
        plugin_ids.into_iter().collect(),
    )
}

fn extract_rule_contact_points(rule: &Map<String, Value>) -> Vec<String> {
    let mut contact_points = BTreeSet::new();
    if let Some(receiver) = rule
        .get("receiver")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        contact_points.insert(receiver.to_string());
    }
    if let Some(receiver) = rule
        .get("notification_settings")
        .or_else(|| rule.get("notificationSettings"))
        .and_then(Value::as_object)
        .and_then(|settings| settings.get("receiver"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        contact_points.insert(receiver.to_string());
    }
    contact_points.into_iter().collect()
}

pub(crate) fn normalize_alert_managed_fields(body: &Map<String, Value>) -> Vec<String> {
    body.keys().cloned().collect()
}

pub(crate) fn normalize_alert_resource_identity_and_title(
    sync_kind: &str,
    payload: &Map<String, Value>,
) -> Result<(String, String)> {
    let Some(spec) = alert_sync_kind_spec(sync_kind) else {
        return Err(message(format!(
            "Alert provisioning export document is missing a stable identity for {sync_kind}."
        )));
    };
    let identity = first_non_empty_field(payload, spec.identity_fields)
        .or(spec.default_identity)
        .unwrap_or("");
    if identity.is_empty() {
        return Err(message(format!(
            "Alert provisioning export document is missing a stable identity for {sync_kind}."
        )));
    }
    let title = first_non_empty_field(payload, spec.title_fields).unwrap_or(identity);
    Ok((identity.to_string(), title.to_string()))
}

fn first_non_empty_field<'a>(
    payload: &'a Map<String, Value>,
    fields: &[&'static str],
) -> Option<&'a str> {
    fields.iter().find_map(|field| {
        payload
            .get(*field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn normalize_rule_group_rule_document(
    group: &Map<String, Value>,
    rule: &Map<String, Value>,
) -> Map<String, Value> {
    let mut normalized = rule.clone();
    if !normalized.contains_key("folderUID") {
        if let Some(folder_uid) = group
            .get("folderUID")
            .or_else(|| group.get("folderUid"))
            .cloned()
        {
            normalized.insert("folderUID".to_string(), folder_uid);
        }
    }
    if !normalized.contains_key("ruleGroup") {
        if let Some(rule_group) = group.get("name").cloned() {
            normalized.insert("ruleGroup".to_string(), rule_group);
        }
    }
    if !normalized.contains_key("notificationSettings") {
        if let Some(notification_settings) = normalized.remove("notification_settings") {
            normalized.insert("notificationSettings".to_string(), notification_settings);
        }
    }
    normalized
}

fn normalize_alert_rule_sync_spec(
    document: &Map<String, Value>,
    source_path: &str,
) -> Result<Value> {
    let rule = build_rule_import_payload(document)?;
    let uid = rule
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            message(format!(
                "Alert rule export document is missing uid: {source_path}"
            ))
        })?;
    let title = rule
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(uid);

    let mut body = Map::new();
    let mut managed_fields = Vec::new();
    add_non_empty_text_field(
        &mut body,
        &mut managed_fields,
        "condition",
        rule.get("condition").and_then(Value::as_str).unwrap_or(""),
    );
    if let Some(annotations) = rule
        .get("annotations")
        .and_then(Value::as_object)
        .cloned()
        .filter(|value| !value.is_empty())
    {
        body.insert("annotations".to_string(), Value::Object(annotations));
        managed_fields.push("annotations".to_string());
    }
    let contact_points = extract_rule_contact_points(&rule);
    if !contact_points.is_empty() {
        body.insert(
            "contactPoints".to_string(),
            Value::Array(contact_points.into_iter().map(Value::String).collect()),
        );
        managed_fields.push("contactPoints".to_string());
    }
    let (datasource_uids, datasource_names, plugin_ids) = extract_rule_dependency_lists(&rule);
    if !datasource_uids.is_empty() {
        body.insert(
            "datasourceUids".to_string(),
            Value::Array(datasource_uids.into_iter().map(Value::String).collect()),
        );
        managed_fields.push("datasourceUids".to_string());
    }
    if !datasource_names.is_empty() {
        body.insert(
            "datasourceNames".to_string(),
            Value::Array(datasource_names.into_iter().map(Value::String).collect()),
        );
        managed_fields.push("datasourceNames".to_string());
    }
    if !plugin_ids.is_empty() {
        body.insert(
            "pluginIds".to_string(),
            Value::Array(plugin_ids.into_iter().map(Value::String).collect()),
        );
        managed_fields.push("pluginIds".to_string());
    }
    if let Some(data) = rule
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .filter(|value| !value.is_empty())
    {
        body.insert("data".to_string(), Value::Array(data));
        managed_fields.push("data".to_string());
    }

    Ok(serde_json::json!({
        "kind": "alert",
        "uid": uid,
        "title": title,
        "managedFields": managed_fields,
        "body": body,
        "sourcePath": source_path,
    }))
}

fn normalize_alert_resource_sync_spec(
    document: &Map<String, Value>,
    source_path: &str,
) -> Result<Option<Value>> {
    let document_kind = detect_document_kind(document)?;
    let Some(spec) = alert_sync_kind_spec_for_document_kind(document_kind) else {
        return Ok(None);
    };
    if spec.sync_kind == "alert" {
        return Ok(Some(normalize_alert_rule_sync_spec(document, source_path)?));
    }
    normalize_non_rule_alert_resource_sync_spec(spec, document, source_path).map(Some)
}

fn normalize_non_rule_alert_resource_sync_spec(
    spec: AlertSyncKindSpec,
    document: &Map<String, Value>,
    source_path: &str,
) -> Result<Value> {
    let Some(payload_builder) = spec.payload_builder else {
        return Ok(Value::Null);
    };
    let body = payload_builder(document)?;
    let (identity, title) = normalize_alert_resource_identity_and_title(spec.sync_kind, &body)?;
    Ok(serde_json::json!({
        "kind": spec.sync_kind,
        "uid": if spec.uid_from_identity { identity.clone() } else { String::new() },
        "name": if spec.name_from_identity { identity.clone() } else { String::new() },
        "title": title,
        "managedFields": normalize_alert_managed_fields(&body),
        "body": body,
        "sourcePath": source_path,
    }))
}

pub(crate) fn build_alert_sync_specs(alerting: &Value) -> Result<Vec<Value>> {
    let mut alerts = Vec::new();
    let Some(alerting_object) = alerting.as_object() else {
        return Ok(alerts);
    };
    for (section, items) in alerting_object {
        let Some(items) = items.as_array() else {
            continue;
        };
        for item in items {
            let Some(object) = item.as_object() else {
                continue;
            };
            let source_path = object
                .get("sourcePath")
                .and_then(Value::as_str)
                .unwrap_or("");
            let Some(document) = object.get("document").and_then(Value::as_object) else {
                continue;
            };
            if section == "rules" {
                if let Some(groups) = document.get("groups").and_then(Value::as_array) {
                    for group in groups {
                        let Some(group_object) = group.as_object() else {
                            continue;
                        };
                        let Some(group_rules) = group_object.get("rules").and_then(Value::as_array)
                        else {
                            continue;
                        };
                        for rule in group_rules {
                            let Some(rule_object) = rule.as_object() else {
                                continue;
                            };
                            let normalized_rule =
                                normalize_rule_group_rule_document(group_object, rule_object);
                            alerts.push(normalize_alert_rule_sync_spec(
                                &normalized_rule,
                                source_path,
                            )?);
                        }
                    }
                    continue;
                }
            }
            if let Some(resource) = normalize_alert_resource_sync_spec(document, source_path)? {
                alerts.push(resource);
            }
        }
    }
    Ok(alerts)
}
