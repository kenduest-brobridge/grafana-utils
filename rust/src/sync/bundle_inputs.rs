//! Sync bundle input normalization helpers.

use super::json::{
    discover_json_files, load_json_array_file, load_json_value, require_json_object,
};
use crate::alert::{
    build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_rule_import_payload, build_template_import_payload,
    detect_document_kind, CONTACT_POINT_KIND, MUTE_TIMING_KIND, POLICIES_KIND, RULE_KIND,
    TEMPLATE_KIND,
};
use crate::common::{message, Result};
use crate::dashboard::DASHBOARD_PERMISSION_BUNDLE_FILENAME;
use crate::dashboard::{resolve_dashboard_import_source, DashboardImportInputFormat};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::iter::FromIterator;
use std::path::Path;

pub(crate) type DashboardBundleSections = (Vec<Value>, Vec<Value>, Vec<Value>, Map<String, Value>);

pub(crate) fn normalize_dashboard_bundle_item(
    document: &Value,
    source_path: &str,
) -> Result<Value> {
    let mut body = if let Some(body) = document.get("dashboard").and_then(Value::as_object) {
        body.clone()
    } else {
        require_json_object(document, "Dashboard export document")?.clone()
    };
    body.remove("id");
    let uid = body
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            message(format!(
                "Dashboard export document is missing dashboard.uid: {source_path}"
            ))
        })?;
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(uid);
    Ok(serde_json::json!({
        "kind": "dashboard",
        "uid": uid,
        "title": title,
        "body": body,
        "sourcePath": source_path,
    }))
}

pub(crate) fn normalize_folder_bundle_item(document: &Value) -> Result<Value> {
    let object = require_json_object(document, "Folder inventory record")?;
    let uid = object
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| message("Folder inventory record is missing uid."))?;
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
    if let Some(path) = object
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        body.insert("path".to_string(), Value::String(path.to_string()));
    }
    Ok(serde_json::json!({
        "kind": "folder",
        "uid": uid,
        "title": title,
        "body": body,
        "sourcePath": object.get("sourcePath").cloned().unwrap_or(Value::String(String::new())),
    }))
}

pub(crate) fn normalize_datasource_bundle_item(document: &Value) -> Result<Value> {
    let object = require_json_object(document, "Datasource inventory record")?;
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
        return Err(message("Datasource inventory record requires uid or name."));
    }
    let title = if name.is_empty() { uid } else { name };
    Ok(serde_json::json!({
        "kind": "datasource",
        "uid": uid,
        "name": title,
        "title": title,
        "body": {
            "uid": uid,
            "name": title,
            "type": object.get("type").cloned().unwrap_or(Value::String(String::new())),
            "access": object.get("access").cloned().unwrap_or(Value::String(String::new())),
            "url": object.get("url").cloned().unwrap_or(Value::String(String::new())),
            "isDefault": object.get("isDefault").cloned().unwrap_or(Value::Bool(false)),
        },
        "secureJsonDataProviders": object.get("secureJsonDataProviders").cloned().unwrap_or(Value::Object(Map::new())),
        "secureJsonDataPlaceholders": object.get("secureJsonDataPlaceholders").cloned().unwrap_or(Value::Object(Map::new())),
        "sourcePath": object.get("sourcePath").cloned().unwrap_or(Value::String(String::new())),
    }))
}

#[derive(Debug, Deserialize)]
struct DatasourceProvisioningDocument {
    #[serde(default)]
    datasources: Vec<DatasourceProvisioningRecord>,
}

#[derive(Debug, Deserialize)]
struct DatasourceProvisioningRecord {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    datasource_type: String,
    #[serde(default)]
    access: String,
    #[serde(default)]
    url: String,
    #[serde(default, rename = "isDefault")]
    is_default: bool,
    #[serde(default, rename = "orgId")]
    org_id: Option<i64>,
}

pub(crate) fn load_datasource_provisioning_records(provisioning_file: &Path) -> Result<Vec<Value>> {
    if !provisioning_file.is_file() {
        return Err(message(format!(
            "Datasource provisioning file does not exist: {}",
            provisioning_file.display()
        )));
    }
    let extension = provisioning_file
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !matches!(extension, "yaml" | "yml") {
        return Err(message(format!(
            "Datasource provisioning file must be YAML (.yaml or .yml): {}",
            provisioning_file.display()
        )));
    }
    let raw = fs::read_to_string(provisioning_file)?;
    let document: DatasourceProvisioningDocument = serde_yaml::from_str(&raw).map_err(|error| {
        message(format!(
            "Failed to parse datasource provisioning YAML {}: {error}",
            provisioning_file.display()
        ))
    })?;
    document
        .datasources
        .into_iter()
        .map(|datasource| {
            let uid = datasource.uid.trim().to_string();
            let name = datasource.name.trim().to_string();
            if uid.is_empty() && name.is_empty() {
                return Err(message(
                    "Datasource provisioning record requires uid or name.",
                ));
            }
            let title = if name.is_empty() {
                uid.clone()
            } else {
                name.clone()
            };
            let mut body = Map::new();
            body.insert("uid".to_string(), Value::String(uid.clone()));
            body.insert("name".to_string(), Value::String(title.clone()));
            body.insert(
                "type".to_string(),
                Value::String(datasource.datasource_type.trim().to_string()),
            );
            body.insert(
                "access".to_string(),
                Value::String(datasource.access.trim().to_string()),
            );
            body.insert(
                "url".to_string(),
                Value::String(datasource.url.trim().to_string()),
            );
            body.insert(
                "isDefault".to_string(),
                Value::String(datasource.is_default.to_string()),
            );
            let mut record = Map::from_iter([
                ("uid".to_string(), Value::String(uid)),
                ("name".to_string(), Value::String(title)),
                (
                    "type".to_string(),
                    Value::String(datasource.datasource_type.trim().to_string()),
                ),
                (
                    "access".to_string(),
                    Value::String(datasource.access.trim().to_string()),
                ),
                (
                    "url".to_string(),
                    Value::String(datasource.url.trim().to_string()),
                ),
                (
                    "isDefault".to_string(),
                    Value::String(datasource.is_default.to_string()),
                ),
                ("body".to_string(), Value::Object(body)),
                (
                    "sourcePath".to_string(),
                    Value::String(provisioning_file.display().to_string()),
                ),
            ]);
            if let Some(org_id) = datasource.org_id {
                record.insert("orgId".to_string(), Value::String(org_id.to_string()));
            }
            Ok(Value::Object(record))
        })
        .collect()
}

fn classify_alert_export_path(relative_path: &str) -> Option<&'static str> {
    let first = relative_path.split('/').next().unwrap_or("");
    match first {
        "rules" => Some("rules"),
        "contact-points" => Some("contactPoints"),
        "mute-timings" => Some("muteTimings"),
        "policies" => Some("policies"),
        "templates" => Some("templates"),
        _ => None,
    }
}

pub(crate) fn load_dashboard_bundle_sections(
    dashboard_dir: &Path,
    metadata_dir: &Path,
    datasource_provisioning_file: Option<&Path>,
) -> Result<DashboardBundleSections> {
    let mut dashboards = Vec::new();
    for path in discover_json_files(
        dashboard_dir,
        &[
            "index.json",
            "export-metadata.json",
            "folders.json",
            "datasources.json",
            DASHBOARD_PERMISSION_BUNDLE_FILENAME,
        ],
    )? {
        let source_path = path
            .strip_prefix(dashboard_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        dashboards.push(normalize_dashboard_bundle_item(
            &load_json_value(&path, "Dashboard export document")?,
            &source_path,
        )?);
    }
    let folders_path = metadata_dir.join("folders.json");
    let folders = if folders_path.is_file() {
        load_json_array_file(&folders_path, "Dashboard folder inventory")?
            .into_iter()
            .map(|item| normalize_folder_bundle_item(&item))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let datasources = if let Some(provisioning_file) = datasource_provisioning_file {
        load_datasource_provisioning_records(provisioning_file)?
            .into_iter()
            .map(|item| normalize_datasource_bundle_item(&item))
            .collect::<Result<Vec<_>>>()?
    } else {
        let datasources_path = metadata_dir.join("datasources.json");
        if datasources_path.is_file() {
            load_json_array_file(&datasources_path, "Dashboard datasource inventory")?
                .into_iter()
                .map(|item| normalize_datasource_bundle_item(&item))
                .collect::<Result<Vec<_>>>()?
        } else {
            Vec::new()
        }
    };
    let mut metadata = Map::new();
    let export_metadata_path = metadata_dir.join("export-metadata.json");
    if export_metadata_path.is_file() {
        metadata.insert(
            "dashboardExport".to_string(),
            load_json_value(&export_metadata_path, "Dashboard export metadata")?,
        );
    }
    if let Some(provisioning_file) = datasource_provisioning_file {
        metadata.insert(
            "datasourceProvisioningFile".to_string(),
            Value::String(provisioning_file.display().to_string()),
        );
    }
    Ok((dashboards, datasources, folders, metadata))
}

pub(crate) fn load_dashboard_provisioning_bundle_sections(
    provisioning_dir: &Path,
    datasource_provisioning_file: Option<&Path>,
) -> Result<DashboardBundleSections> {
    let resolved = resolve_dashboard_import_source(
        provisioning_dir,
        DashboardImportInputFormat::Provisioning,
    )?;
    load_dashboard_bundle_sections(
        &resolved.dashboard_dir,
        &resolved.metadata_dir,
        datasource_provisioning_file,
    )
}

pub(crate) fn load_alerting_bundle_section(output_dir: &Path) -> Result<Value> {
    let mut alerting = Map::from_iter(vec![
        ("rules".to_string(), Value::Array(Vec::<Value>::new())),
        (
            "contactPoints".to_string(),
            Value::Array(Vec::<Value>::new()),
        ),
        ("muteTimings".to_string(), Value::Array(Vec::<Value>::new())),
        ("policies".to_string(), Value::Array(Vec::<Value>::new())),
        ("templates".to_string(), Value::Array(Vec::<Value>::new())),
    ]);
    for path in discover_json_files(output_dir, &["index.json", "export-metadata.json"])? {
        let relative_path = path
            .strip_prefix(output_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let Some(section) = classify_alert_export_path(&relative_path) else {
            continue;
        };
        let item = serde_json::json!({
            "sourcePath": relative_path,
            "document": load_json_value(&path, "Alert export document")?,
        });
        alerting
            .entry(section.to_string())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .expect("alerting section array")
            .push(item);
    }
    let summary = serde_json::json!({
        "ruleCount": alerting.get("rules").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
        "contactPointCount": alerting.get("contactPoints").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
        "muteTimingCount": alerting.get("muteTimings").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
        "policyCount": alerting.get("policies").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
        "templateCount": alerting.get("templates").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
    });
    alerting.insert("summary".to_string(), summary);
    let export_metadata_path = output_dir.join("export-metadata.json");
    if export_metadata_path.is_file() {
        alerting.insert(
            "exportMetadata".to_string(),
            load_json_value(&export_metadata_path, "Alert export metadata")?,
        );
    }
    alerting.insert(
        "exportDir".to_string(),
        Value::String(output_dir.display().to_string()),
    );
    Ok(Value::Object(alerting))
}

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
    let identity = match sync_kind {
        "alert" | "alert-contact-point" => payload
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or(""),
        "alert-mute-timing" | "alert-template" => payload
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(""),
        "alert-policy" => payload
            .get("receiver")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("root"),
        _ => "",
    };
    if identity.is_empty() {
        return Err(message(format!(
            "Alert provisioning export document is missing a stable identity for {sync_kind}."
        )));
    }
    let title = payload
        .get("name")
        .or_else(|| payload.get("title"))
        .or_else(|| payload.get("receiver"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(identity);
    Ok((identity.to_string(), title.to_string()))
}

fn map_alert_document_kind_to_sync_kind(kind: &str) -> Option<&'static str> {
    match kind {
        RULE_KIND => Some("alert"),
        CONTACT_POINT_KIND => Some("alert-contact-point"),
        MUTE_TIMING_KIND => Some("alert-mute-timing"),
        POLICIES_KIND => Some("alert-policy"),
        TEMPLATE_KIND => Some("alert-template"),
        _ => None,
    }
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
    let Some(sync_kind) = map_alert_document_kind_to_sync_kind(document_kind) else {
        return Ok(None);
    };
    if sync_kind == "alert" {
        return Ok(Some(normalize_alert_rule_sync_spec(document, source_path)?));
    }
    let body = match document_kind {
        CONTACT_POINT_KIND => build_contact_point_import_payload(document)?,
        MUTE_TIMING_KIND => build_mute_timing_import_payload(document)?,
        POLICIES_KIND => build_policies_import_payload(document)?,
        TEMPLATE_KIND => build_template_import_payload(document)?,
        _ => return Ok(None),
    };
    let (identity, title) = normalize_alert_resource_identity_and_title(sync_kind, &body)?;
    Ok(Some(serde_json::json!({
        "kind": sync_kind,
        "uid": if sync_kind == "alert-contact-point" { identity.clone() } else { String::new() },
        "name": if matches!(sync_kind, "alert-mute-timing" | "alert-template") { identity.clone() } else { String::new() },
        "title": title,
        "managedFields": normalize_alert_managed_fields(&body),
        "body": body,
        "sourcePath": source_path,
    })))
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
