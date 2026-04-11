use crate::common::{sanitize_path_component, Result};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::alert_support_documents::{
    build_contact_point_export_document, build_rule_export_document, build_template_export_document,
};
use super::alert_support_paths::resource_subdir_by_kind;
use super::alert_support_policy::{build_stable_route_label_value, stable_route_label_key};

fn scaffold_identity(name: &str, fallback: &str) -> String {
    let identity = sanitize_path_component(name);
    if identity.is_empty() {
        fallback.to_string()
    } else {
        identity
    }
}

pub fn init_alert_managed_dir(root: &Path) -> Result<Vec<PathBuf>> {
    let mut created = Vec::new();
    fs::create_dir_all(root)?;
    created.push(root.to_path_buf());
    for subdir in resource_subdir_by_kind().into_values() {
        let path = root.join(subdir);
        fs::create_dir_all(&path)?;
        created.push(path);
    }
    Ok(created)
}

pub fn build_folder_resolution_contract(folder_uid: &str, folder_title: Option<&str>) -> Value {
    let title = folder_title.unwrap_or_default().trim().to_string();
    json!({
        "folderUid": folder_uid,
        "folderTitle": title,
        "resolution": if title.is_empty() { "uid-only" } else { "uid-or-title" },
    })
}

pub fn build_simple_rule_body(
    title: &str,
    folder_uid: &str,
    rule_group: &str,
    route_name: &str,
) -> Map<String, Value> {
    let mut labels = Map::new();
    labels.insert(
        stable_route_label_key().to_string(),
        Value::String(build_stable_route_label_value(route_name)),
    );
    [
        (
            "uid".to_string(),
            Value::String(scaffold_identity(title, "rule")),
        ),
        ("title".to_string(), Value::String(title.to_string())),
        (
            "folderUID".to_string(),
            Value::String(if folder_uid.trim().is_empty() {
                "general".to_string()
            } else {
                folder_uid.trim().to_string()
            }),
        ),
        (
            "ruleGroup".to_string(),
            Value::String(if rule_group.trim().is_empty() {
                "default".to_string()
            } else {
                rule_group.trim().to_string()
            }),
        ),
        ("condition".to_string(), Value::String("A".to_string())),
        ("data".to_string(), Value::Array(Vec::new())),
        ("for".to_string(), Value::String("5m".to_string())),
        (
            "noDataState".to_string(),
            Value::String("NoData".to_string()),
        ),
        (
            "execErrState".to_string(),
            Value::String("Alerting".to_string()),
        ),
        ("annotations".to_string(), Value::Object(Map::new())),
        ("labels".to_string(), Value::Object(labels)),
    ]
    .into_iter()
    .collect()
}

pub fn build_new_rule_scaffold_document(name: &str) -> Value {
    build_new_rule_scaffold_document_with_route(name, "general", "default", name)
}

pub fn build_new_contact_point_scaffold_document(name: &str) -> Value {
    build_contact_point_scaffold_document(name, "webhook")
}

pub fn build_new_template_scaffold_document(name: &str) -> Value {
    build_template_export_document(
        json!({
            "name": name,
            "template": format!("{{{{ define \"{name}\" }}}}replace me{{{{ end }}}}")
        })
        .as_object()
        .expect("template scaffold must be an object"),
    )
}

pub fn build_new_rule_scaffold_document_with_route(
    name: &str,
    folder_uid: &str,
    rule_group: &str,
    route_name: &str,
) -> Value {
    let body = build_simple_rule_body(name, folder_uid, rule_group, route_name);
    let mut document = build_rule_export_document(&body);
    if let Some(metadata) = document.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.insert(
            "folder".to_string(),
            build_folder_resolution_contract(folder_uid, None),
        );
        metadata.insert(
            "route".to_string(),
            json!({
                "labelKey": stable_route_label_key(),
                "labelValue": build_stable_route_label_value(route_name),
            }),
        );
    }
    document
}

pub fn build_contact_point_scaffold_document(name: &str, channel_type: &str) -> Value {
    let identity = scaffold_identity(name, "contact-point");
    let (normalized_type, settings) = match channel_type {
        "email" => (
            "email",
            json!({
                "addresses": ["alerts@example.com"],
                "singleEmail": false,
            }),
        ),
        "slack" => (
            "slack",
            json!({
                "recipient": "#alerts",
                "token": "${SLACK_BOT_TOKEN}",
                "text": "{{ template \"slack.default\" . }}",
            }),
        ),
        _ => (
            "webhook",
            json!({
                "url": "http://127.0.0.1:9000/notify"
            }),
        ),
    };
    let mut document = build_contact_point_export_document(
        json!({
            "uid": identity,
            "name": name,
            "type": normalized_type,
            "settings": settings,
        })
        .as_object()
        .expect("contact-point scaffold must be an object"),
    );
    let settings_keys = metadata_from_settings(document.get("spec").and_then(Value::as_object));
    if let Some(metadata) = document.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.insert(
            "authoring".to_string(),
            json!({
                "channelType": normalized_type,
                "settingsKeys": settings_keys,
            }),
        );
    }
    document
}

fn metadata_from_settings(spec: Option<&Map<String, Value>>) -> Value {
    let mut keys = spec
        .and_then(|item| item.get("settings"))
        .and_then(Value::as_object)
        .map(|settings| settings.keys().cloned().collect::<Vec<String>>())
        .unwrap_or_default();
    keys.sort();
    Value::Array(keys.into_iter().map(Value::String).collect())
}
