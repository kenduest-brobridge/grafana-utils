//! Shared alert resource utilities and filesystem payload handling.
//!
//! Responsibilities:
//! - Resolve alert resource directories and output paths for each alert kind.
//! - Load, sanitize, and persist alert payloads used by import/export.
//! - Centralize common constants and helper transforms used by alert command modules.

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    load_json_object_file, message, sanitize_path_component, string_field, tool_version,
    value_as_object, Result,
};

use super::{
    CONTACT_POINTS_SUBDIR, CONTACT_POINT_KIND, MUTE_TIMINGS_SUBDIR, MUTE_TIMING_KIND,
    POLICIES_KIND, POLICIES_SUBDIR, ROOT_INDEX_KIND, RULES_SUBDIR, RULE_KIND, TEMPLATES_SUBDIR,
    TEMPLATE_KIND, TOOL_API_VERSION, TOOL_SCHEMA_VERSION,
};

pub fn resource_subdir_by_kind() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (RULE_KIND, RULES_SUBDIR),
        (CONTACT_POINT_KIND, CONTACT_POINTS_SUBDIR),
        (MUTE_TIMING_KIND, MUTE_TIMINGS_SUBDIR),
        (POLICIES_KIND, POLICIES_SUBDIR),
        (TEMPLATE_KIND, TEMPLATES_SUBDIR),
    ])
}

pub fn build_rule_output_path(output_dir: &Path, rule: &Map<String, Value>, flat: bool) -> PathBuf {
    let folder_uid = sanitize_path_component(&string_field(rule, "folderUID", "general"));
    let rule_group = sanitize_path_component(&string_field(rule, "ruleGroup", "default"));
    let title = sanitize_path_component(&string_field(rule, "title", "rule"));
    let uid = sanitize_path_component(&string_field(rule, "uid", "unknown"));
    let file_name = format!("{title}__{uid}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(folder_uid).join(rule_group).join(file_name)
    }
}

pub fn build_contact_point_output_path(
    output_dir: &Path,
    contact_point: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(contact_point, "name", "contact-point"));
    let uid = sanitize_path_component(&string_field(contact_point, "uid", "unknown"));
    let file_name = format!("{name}__{uid}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub fn build_mute_timing_output_path(
    output_dir: &Path,
    mute_timing: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(mute_timing, "name", "mute-timing"));
    let file_name = format!("{name}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub fn build_policies_output_path(output_dir: &Path) -> PathBuf {
    output_dir.join("notification-policies.json")
}

pub fn build_template_output_path(
    output_dir: &Path,
    template: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(template, "name", "template"));
    let file_name = format!("{name}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub const MANAGED_ROUTE_LABEL_KEY: &str = "grafana_utils_route";

pub fn build_resource_dirs(raw_dir: &Path) -> BTreeMap<&'static str, PathBuf> {
    resource_subdir_by_kind()
        .into_iter()
        .map(|(kind, subdir)| (kind, raw_dir.join(subdir)))
        .collect()
}

pub fn discover_alert_resource_files(import_dir: &Path) -> Result<Vec<PathBuf>> {
    if !import_dir.exists() {
        return Err(message(format!(
            "Import directory does not exist: {}",
            import_dir.display()
        )));
    }
    if !import_dir.is_dir() {
        return Err(message(format!(
            "Import path is not a directory: {}",
            import_dir.display()
        )));
    }
    if import_dir.join(super::RAW_EXPORT_SUBDIR).is_dir() {
        return Err(message(format!(
            "Import path {} looks like the export root. Point --import-dir at {}.",
            import_dir.display(),
            import_dir.join(super::RAW_EXPORT_SUBDIR).display()
        )));
    }

    let mut files = Vec::new();
    collect_resource_files(import_dir, &mut files)?;
    files.retain(|path| {
        !matches!(
            path.file_name().and_then(|value| value.to_str()),
            Some("index.json" | "index.yaml" | "index.yml")
        )
    });
    files.sort();
    if files.is_empty() {
        return Err(message(format!(
            "No alerting resource JSON or YAML files found in {}",
            import_dir.display()
        )));
    }
    Ok(files)
}

fn collect_resource_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_resource_files(&path, files)?;
            continue;
        }
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("json" | "yaml" | "yml")
        ) {
            files.push(path);
        }
    }
    Ok(())
}

pub fn load_alert_resource_file(path: &Path, object_label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let value = match extension {
        "yaml" | "yml" => serde_yaml::from_str::<Value>(&raw).map_err(|error| {
            message(format!("Failed to parse YAML {}: {error}", path.display()))
        })?,
        _ => serde_json::from_str::<Value>(&raw)?,
    };
    if !value.is_object() {
        return Err(message(format!(
            "{object_label} file must contain a top-level object: {}",
            path.display()
        )));
    }
    Ok(value)
}

pub fn write_alert_resource_file(path: &Path, payload: &Value, overwrite: bool) -> Result<()> {
    if path.exists() && !overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = match path.extension().and_then(|value| value.to_str()) {
        Some("yaml" | "yml") => serde_yaml::to_string(payload).map_err(|error| {
            message(format!("Failed to write YAML {}: {error}", path.display()))
        })?,
        _ => serde_json::to_string_pretty(payload)?,
    };
    fs::write(path, format!("{serialized}\n"))?;
    Ok(())
}

pub fn derive_dashboard_slug(value: &Value) -> String {
    let mut text = value.as_str().unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return String::new();
    }
    if let Some(index) = text.find("/d/") {
        let tail = &text[index + 3..];
        let mut segments = tail.split('/');
        let _uid = segments.next();
        if let Some(slug) = segments.next() {
            return slug
                .split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_string();
        }
    }
    if text.starts_with('/') {
        text = text
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
    }
    text
}

pub fn load_string_map(path: Option<&Path>, label: &str) -> Result<BTreeMap<String, String>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let payload = load_json_object_file(path, label)?;
    let object = value_as_object(&payload, &format!("{label} must be a JSON object."))?;
    Ok(object
        .iter()
        .map(|(key, value)| (key.clone(), value_to_string(value)))
        .collect())
}

pub fn load_panel_id_map(
    path: Option<&Path>,
) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let payload = load_json_object_file(path, "Panel ID map")?;
    let object = value_as_object(&payload, "Panel ID map must be a JSON object.")?;
    let mut normalized = BTreeMap::new();
    for (dashboard_uid, mapping_value) in object {
        let mapping_object = value_as_object(
            mapping_value,
            "Panel ID map values must be JSON objects keyed by source panel ID.",
        )?;
        normalized.insert(
            dashboard_uid.clone(),
            mapping_object
                .iter()
                .map(|(panel_id, target_panel_id)| {
                    (panel_id.clone(), value_to_string(target_panel_id))
                })
                .collect(),
        );
    }
    Ok(normalized)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AlertLinkageMappings {
    dashboard_uid_map: BTreeMap<String, String>,
    panel_id_map: BTreeMap<String, BTreeMap<String, String>>,
}

impl AlertLinkageMappings {
    pub(crate) fn load(
        dashboard_uid_path: Option<&Path>,
        panel_id_path: Option<&Path>,
    ) -> Result<AlertLinkageMappings> {
        Ok(AlertLinkageMappings {
            dashboard_uid_map: load_string_map(dashboard_uid_path, "Dashboard UID map")?,
            panel_id_map: load_panel_id_map(panel_id_path)?,
        })
    }

    pub(crate) fn resolve_dashboard_uid(&self, source_dashboard_uid: &str) -> String {
        self.dashboard_uid_map
            .get(source_dashboard_uid)
            .cloned()
            .unwrap_or_else(|| source_dashboard_uid.to_string())
    }

    pub(crate) fn resolve_panel_id(
        &self,
        source_dashboard_uid: &str,
        source_panel_id: &str,
    ) -> Option<String> {
        self.panel_id_map
            .get(source_dashboard_uid)
            .and_then(|mapping| mapping.get(source_panel_id))
            .cloned()
    }
}

pub(crate) fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

pub fn strip_server_managed_fields(kind: &str, payload: &Map<String, Value>) -> Map<String, Value> {
    let managed_fields = match kind {
        RULE_KIND => ["id", "updated", "provenance"].as_slice(),
        CONTACT_POINT_KIND => ["provenance"].as_slice(),
        MUTE_TIMING_KIND => ["version", "provenance"].as_slice(),
        POLICIES_KIND => ["provenance"].as_slice(),
        TEMPLATE_KIND => ["version", "provenance"].as_slice(),
        _ => [].as_slice(),
    };

    payload
        .iter()
        .filter(|(key, _)| !managed_fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn remove_null_field(object: &mut Map<String, Value>, key: &str) {
    if matches!(object.get(key), Some(Value::Null)) {
        object.remove(key);
    }
}

fn remove_empty_object_field(object: &mut Map<String, Value>, key: &str) {
    if object
        .get(key)
        .and_then(Value::as_object)
        .map(|value| value.is_empty())
        .unwrap_or(false)
    {
        object.remove(key);
    }
}

fn remove_string_field_when(object: &mut Map<String, Value>, key: &str, expected: &str) {
    if object.get(key).and_then(Value::as_str) == Some(expected) {
        object.remove(key);
    }
}

fn remove_bool_field_when(object: &mut Map<String, Value>, key: &str, expected: bool) {
    if object.get(key).and_then(Value::as_bool) == Some(expected) {
        object.remove(key);
    }
}

fn sort_matcher_values(matchers: &mut [Value]) {
    matchers.sort_by_key(value_to_string);
}

fn normalize_compare_value(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.into_iter().map(normalize_compare_value).collect())
        }
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, item)| (key, normalize_compare_value(item)))
                .collect(),
        ),
        Value::Number(number) => {
            if let Some(float_value) = number.as_f64() {
                if float_value.fract() == 0.0
                    && float_value >= i64::MIN as f64
                    && float_value <= i64::MAX as f64
                {
                    return Value::Number(serde_json::Number::from(float_value as i64));
                }
            }
            Value::Number(number)
        }
        other => other,
    }
}

fn normalize_rule_compare_payload(payload: &mut Map<String, Value>) {
    payload.remove("orgID");
    remove_bool_field_when(payload, "isPaused", false);
    remove_string_field_when(payload, "keep_firing_for", "0s");
    remove_null_field(payload, "notification_settings");
    remove_null_field(payload, "record");
    remove_empty_object_field(payload, "annotations");

    if let Some(data) = payload.get_mut("data").and_then(Value::as_array_mut) {
        for item in data {
            let Some(item_object) = item.as_object_mut() else {
                continue;
            };
            remove_string_field_when(item_object, "queryType", "");
        }
    }
}

fn normalize_contact_point_compare_payload(payload: &mut Map<String, Value>) {
    remove_bool_field_when(payload, "disableResolveMessage", false);
}

fn normalize_policy_route_for_compare(route: &mut Map<String, Value>) {
    remove_bool_field_when(route, "continue", false);
    if let Some(matchers) = route
        .get_mut("object_matchers")
        .and_then(Value::as_array_mut)
    {
        sort_matcher_values(matchers);
    }
}

fn normalize_policy_compare_payload(payload: &mut Map<String, Value>) {
    if let Some(routes) = payload.get_mut("routes").and_then(Value::as_array_mut) {
        for route in routes {
            let Some(route_object) = route.as_object_mut() else {
                continue;
            };
            normalize_policy_route_for_compare(route_object);
        }
    }
}

pub fn normalize_compare_payload(kind: &str, payload: &Map<String, Value>) -> Map<String, Value> {
    let mut normalized = strip_server_managed_fields(kind, payload);
    match kind {
        RULE_KIND => normalize_rule_compare_payload(&mut normalized),
        CONTACT_POINT_KIND => normalize_contact_point_compare_payload(&mut normalized),
        POLICIES_KIND => normalize_policy_compare_payload(&mut normalized),
        _ => {}
    }
    normalize_compare_value(Value::Object(normalized))
        .as_object()
        .cloned()
        .expect("normalized compare payload must remain an object")
}

#[allow(dead_code)]
pub fn stable_route_label_key() -> &'static str {
    MANAGED_ROUTE_LABEL_KEY
}

#[allow(dead_code)]
pub fn build_stable_route_label_value(name: &str) -> String {
    let value = sanitize_path_component(name);
    if value.is_empty() {
        "managed-route".to_string()
    } else {
        value
    }
}

#[allow(dead_code)]
pub fn build_stable_route_matcher(route_name: &str) -> Value {
    json!([
        stable_route_label_key(),
        "=",
        build_stable_route_label_value(route_name)
    ])
}

#[allow(dead_code)]
fn value_list(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).cloned().unwrap_or_default()
}

#[allow(dead_code)]
fn route_matcher_entries(route: &Map<String, Value>) -> Vec<Vec<String>> {
    route
        .get("object_matchers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|matcher| {
            let items = matcher.as_array()?;
            if items.len() != 3 {
                return None;
            }
            Some(items.iter().map(value_to_string).collect::<Vec<String>>())
        })
        .collect()
}

#[allow(dead_code)]
fn normalize_route_matchers(route: &mut Map<String, Value>, route_name: &str) {
    let managed_value = build_stable_route_label_value(route_name);
    let mut matchers = route_matcher_entries(route)
        .into_iter()
        .filter(|matcher| {
            !(matcher.first().map(String::as_str) == Some(stable_route_label_key())
                && matcher.get(1).map(String::as_str) == Some("="))
        })
        .map(|matcher| Value::Array(matcher.into_iter().map(Value::String).collect()))
        .collect::<Vec<Value>>();
    matchers.push(json!([stable_route_label_key(), "=", managed_value]));
    route.insert("object_matchers".to_string(), Value::Array(matchers));
}

#[allow(dead_code)]
pub fn route_matches_stable_label(route: &Map<String, Value>, route_name: &str) -> bool {
    let expected_value = build_stable_route_label_value(route_name);
    route_matcher_entries(route).into_iter().any(|matcher| {
        matcher.first().map(String::as_str) == Some(stable_route_label_key())
            && matcher.get(1).map(String::as_str) == Some("=")
            && matcher.get(2).map(String::as_str) == Some(expected_value.as_str())
    })
}

#[allow(dead_code)]
pub fn build_route_preview(route: &Map<String, Value>) -> Value {
    json!({
        "receiver": string_field(route, "receiver", ""),
        "continue": route.get("continue").and_then(Value::as_bool).unwrap_or(false),
        "groupBy": value_list(route.get("group_by")),
        "matchers": value_list(route.get("object_matchers")),
        "childRouteCount": route.get("routes").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
    })
}

#[allow(dead_code)]
pub fn build_folder_resolution_contract(folder_uid: &str, folder_title: Option<&str>) -> Value {
    let title = folder_title.unwrap_or_default().trim().to_string();
    json!({
        "folderUid": folder_uid,
        "folderTitle": title,
        "resolution": if title.is_empty() { "uid-only" } else { "uid-or-title" },
    })
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn normalize_managed_policy_route(
    route_name: &str,
    route: &Map<String, Value>,
) -> Map<String, Value> {
    let mut normalized = route.clone();
    normalize_route_matchers(&mut normalized, route_name);
    normalized
}

#[allow(dead_code)]
fn route_list_with_indexes(policy: &Map<String, Value>) -> Vec<(usize, Map<String, Value>)> {
    policy
        .get("routes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, route)| route.as_object().cloned().map(|item| (index, item)))
        .collect()
}

#[allow(dead_code)]
pub fn upsert_managed_policy_subtree(
    policy: &Map<String, Value>,
    route_name: &str,
    route: &Map<String, Value>,
) -> Result<(Map<String, Value>, &'static str)> {
    let normalized_route = normalize_managed_policy_route(route_name, route);
    let mut next_policy = policy.clone();
    let routes = route_list_with_indexes(policy);
    let matching = routes
        .iter()
        .filter(|(_, item)| route_matches_stable_label(item, route_name))
        .map(|(index, _)| *index)
        .collect::<Vec<usize>>();
    if matching.len() > 1 {
        return Err(message(format!(
            "Managed route label {:?} is not unique in notification policies.",
            build_stable_route_label_value(route_name)
        )));
    }

    let mut next_routes = policy
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let action = if let Some(index) = matching.first().copied() {
        if next_routes
            .get(index)
            .and_then(Value::as_object)
            .map(|item| item == &normalized_route)
            .unwrap_or(false)
        {
            "noop"
        } else {
            next_routes[index] = Value::Object(normalized_route);
            "updated"
        }
    } else {
        next_routes.push(Value::Object(normalized_route));
        "created"
    };
    next_policy.insert("routes".to_string(), Value::Array(next_routes));
    Ok((next_policy, action))
}

#[allow(dead_code)]
pub fn remove_managed_policy_subtree(
    policy: &Map<String, Value>,
    route_name: &str,
) -> Result<(Map<String, Value>, &'static str)> {
    let routes = route_list_with_indexes(policy);
    let matching = routes
        .iter()
        .filter(|(_, item)| route_matches_stable_label(item, route_name))
        .map(|(index, _)| *index)
        .collect::<Vec<usize>>();
    if matching.len() > 1 {
        return Err(message(format!(
            "Managed route label {:?} is not unique in notification policies.",
            build_stable_route_label_value(route_name)
        )));
    }
    let Some(index) = matching.first().copied() else {
        return Ok((policy.clone(), "noop"));
    };

    let next_routes = policy
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter_map(|(position, value)| (position != index).then_some(value))
        .collect::<Vec<Value>>();
    let mut next_policy = policy.clone();
    next_policy.insert("routes".to_string(), Value::Array(next_routes));
    Ok((next_policy, "deleted"))
}

#[allow(dead_code)]
pub fn build_managed_policy_route_preview(
    current_policy: &Map<String, Value>,
    route_name: &str,
    desired_route: Option<&Map<String, Value>>,
) -> Result<Value> {
    let current_route = route_list_with_indexes(current_policy)
        .into_iter()
        .find(|(_, route)| route_matches_stable_label(route, route_name))
        .map(|(_, route)| route);
    let (next_policy, action) = match desired_route {
        Some(route) => upsert_managed_policy_subtree(current_policy, route_name, route)?,
        None => remove_managed_policy_subtree(current_policy, route_name)?,
    };
    let next_route = route_list_with_indexes(&next_policy)
        .into_iter()
        .find(|(_, route)| route_matches_stable_label(route, route_name))
        .map(|(_, route)| route);
    Ok(json!({
        "action": action,
        "managedRouteKey": stable_route_label_key(),
        "managedRouteValue": build_stable_route_label_value(route_name),
        "currentRoute": current_route.map(|route| build_route_preview(&route)).unwrap_or(Value::Null),
        "nextRoute": next_route.map(|route| build_route_preview(&route)).unwrap_or(Value::Null),
        "nextPolicyRouteCount": next_policy.get("routes").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
    }))
}

fn build_rule_metadata(rule: &Map<String, Value>) -> Value {
    json!({
        "uid": string_field(rule, "uid", ""),
        "title": string_field(rule, "title", ""),
        "folderUID": string_field(rule, "folderUID", ""),
        "ruleGroup": string_field(rule, "ruleGroup", ""),
    })
}

fn build_contact_point_metadata(contact_point: &Map<String, Value>) -> Value {
    json!({
        "uid": string_field(contact_point, "uid", ""),
        "name": string_field(contact_point, "name", ""),
        "type": string_field(contact_point, "type", ""),
    })
}

fn build_mute_timing_metadata(mute_timing: &Map<String, Value>) -> Value {
    json!({ "name": string_field(mute_timing, "name", "") })
}

fn build_policies_metadata(policies: &Map<String, Value>) -> Value {
    json!({ "receiver": string_field(policies, "receiver", "") })
}

fn build_template_metadata(template: &Map<String, Value>) -> Value {
    json!({ "name": string_field(template, "name", "") })
}

fn build_tool_document(kind: &str, spec: Map<String, Value>, metadata: Value) -> Value {
    json!({
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "apiVersion": TOOL_API_VERSION,
        "kind": kind,
        "metadata": metadata,
        "spec": Value::Object(spec),
    })
}

pub fn build_rule_export_document(rule: &Map<String, Value>) -> Value {
    let mut normalized = strip_server_managed_fields(RULE_KIND, rule);
    let linked_dashboard = normalized.remove("__linkedDashboardMetadata__");
    let mut document = build_tool_document(
        RULE_KIND,
        normalized.clone(),
        build_rule_metadata(&normalized),
    );
    if let Some(Value::Object(linked_dashboard)) = linked_dashboard {
        if let Some(metadata) = document.get_mut("metadata").and_then(Value::as_object_mut) {
            metadata.insert(
                "linkedDashboard".to_string(),
                Value::Object(linked_dashboard),
            );
        }
    }
    document
}

pub fn build_contact_point_export_document(contact_point: &Map<String, Value>) -> Value {
    let normalized = strip_server_managed_fields(CONTACT_POINT_KIND, contact_point);
    build_tool_document(
        CONTACT_POINT_KIND,
        normalized.clone(),
        build_contact_point_metadata(&normalized),
    )
}

pub fn build_mute_timing_export_document(mute_timing: &Map<String, Value>) -> Value {
    let normalized = strip_server_managed_fields(MUTE_TIMING_KIND, mute_timing);
    build_tool_document(
        MUTE_TIMING_KIND,
        normalized.clone(),
        build_mute_timing_metadata(&normalized),
    )
}

pub fn build_policies_export_document(policies: &Map<String, Value>) -> Value {
    let normalized = strip_server_managed_fields(POLICIES_KIND, policies);
    build_tool_document(
        POLICIES_KIND,
        normalized.clone(),
        build_policies_metadata(&normalized),
    )
}

pub fn build_template_export_document(template: &Map<String, Value>) -> Value {
    let normalized = strip_server_managed_fields(TEMPLATE_KIND, template);
    build_tool_document(
        TEMPLATE_KIND,
        normalized.clone(),
        build_template_metadata(&normalized),
    )
}

pub fn reject_provisioning_export(document: &Map<String, Value>) -> Result<()> {
    if document.contains_key("groups")
        || document.contains_key("contactPoints")
        || document.contains_key("policies")
        || document.contains_key("templates")
    {
        return Err(message(
            "Grafana provisioning export format is not supported for API import. Use files exported by grafana-util alert export.",
        ));
    }
    Ok(())
}

pub fn detect_document_kind(document: &Map<String, Value>) -> Result<&'static str> {
    if let Some(kind) = document.get("kind").and_then(Value::as_str) {
        if resource_subdir_by_kind().contains_key(kind) {
            return Ok(match kind {
                RULE_KIND => RULE_KIND,
                CONTACT_POINT_KIND => CONTACT_POINT_KIND,
                MUTE_TIMING_KIND => MUTE_TIMING_KIND,
                POLICIES_KIND => POLICIES_KIND,
                TEMPLATE_KIND => TEMPLATE_KIND,
                _ => unreachable!(),
            });
        }
    }

    if document.contains_key("condition") && document.contains_key("data") {
        return Ok(RULE_KIND);
    }
    if document.contains_key("time_intervals") && document.contains_key("name") {
        return Ok(MUTE_TIMING_KIND);
    }
    if document.contains_key("type")
        && document.contains_key("settings")
        && document.contains_key("name")
    {
        return Ok(CONTACT_POINT_KIND);
    }
    if document.contains_key("name") && document.contains_key("template") {
        return Ok(TEMPLATE_KIND);
    }
    if document.contains_key("receiver")
        || document.contains_key("routes")
        || document.contains_key("group_by")
    {
        return Ok(POLICIES_KIND);
    }

    Err(message(
        "Cannot determine alerting resource kind from import document.",
    ))
}

fn extract_tool_spec(
    document: &Map<String, Value>,
    expected_kind: &str,
) -> Result<Map<String, Value>> {
    let spec = if document.get("kind").and_then(Value::as_str) == Some(expected_kind) {
        if let Some(api_version) = document.get("apiVersion").and_then(Value::as_i64) {
            if api_version != TOOL_API_VERSION {
                return Err(message(format!(
                    "Unsupported {expected_kind} export version: {:?}",
                    document.get("apiVersion")
                )));
            }
        }
        if let Some(schema_version) = document.get("schemaVersion").and_then(Value::as_i64) {
            if schema_version != TOOL_SCHEMA_VERSION {
                return Err(message(format!(
                    "Unsupported {expected_kind} schema version: {:?}",
                    document.get("schemaVersion")
                )));
            }
        }
        if document.get("apiVersion").is_none() && document.get("schemaVersion").is_none() {
            return Err(message(format!(
                "Unsupported {expected_kind} export version: {:?}",
                document.get("apiVersion")
            )));
        }
        document.get("spec").cloned().ok_or_else(|| {
            message(format!(
                "{expected_kind} import document is missing a valid spec object."
            ))
        })?
    } else {
        Value::Object(document.clone())
    };

    match spec {
        Value::Object(object) => Ok(object),
        _ => Err(message(format!(
            "{expected_kind} import document is missing a valid spec object."
        ))),
    }
}

pub fn build_rule_import_payload(document: &Map<String, Value>) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    let payload = strip_server_managed_fields(RULE_KIND, &extract_tool_spec(document, RULE_KIND)?);
    for field in ["title", "folderUID", "ruleGroup", "condition", "data"] {
        if !payload.contains_key(field) {
            return Err(message(format!(
                "Alert-rule import document is missing required fields: {field}"
            )));
        }
    }
    if !payload.get("data").map(Value::is_array).unwrap_or(false) {
        return Err(message("Alert-rule field 'data' must be a list."));
    }
    Ok(payload)
}

pub fn build_contact_point_import_payload(
    document: &Map<String, Value>,
) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    let payload = strip_server_managed_fields(
        CONTACT_POINT_KIND,
        &extract_tool_spec(document, CONTACT_POINT_KIND)?,
    );
    for field in ["name", "type", "settings"] {
        if !payload.contains_key(field) {
            return Err(message(format!(
                "Contact-point import document is missing required fields: {field}"
            )));
        }
    }
    if !payload
        .get("settings")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        return Err(message("Contact-point field 'settings' must be an object."));
    }
    Ok(payload)
}

pub fn build_mute_timing_import_payload(
    document: &Map<String, Value>,
) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    let payload = strip_server_managed_fields(
        MUTE_TIMING_KIND,
        &extract_tool_spec(document, MUTE_TIMING_KIND)?,
    );
    for field in ["name", "time_intervals"] {
        if !payload.contains_key(field) {
            return Err(message(format!(
                "Mute-timing import document is missing required fields: {field}"
            )));
        }
    }
    if !payload
        .get("time_intervals")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        return Err(message(
            "Mute-timing field 'time_intervals' must be a list.",
        ));
    }
    Ok(payload)
}

pub fn build_policies_import_payload(document: &Map<String, Value>) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    extract_tool_spec(document, POLICIES_KIND)
}

pub fn build_template_import_payload(document: &Map<String, Value>) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    let payload =
        strip_server_managed_fields(TEMPLATE_KIND, &extract_tool_spec(document, TEMPLATE_KIND)?);
    for field in ["name", "template"] {
        if !payload.contains_key(field) {
            return Err(message(format!(
                "Template import document is missing required fields: {field}"
            )));
        }
    }
    Ok(payload)
}

pub fn build_import_operation(document: &Value) -> Result<(String, Map<String, Value>)> {
    let object = value_as_object(document, "Alerting import document must be a JSON object.")?;
    let kind = detect_document_kind(object)?;
    let payload = match kind {
        RULE_KIND => build_rule_import_payload(object)?,
        CONTACT_POINT_KIND => build_contact_point_import_payload(object)?,
        MUTE_TIMING_KIND => build_mute_timing_import_payload(object)?,
        POLICIES_KIND => build_policies_import_payload(object)?,
        TEMPLATE_KIND => build_template_import_payload(object)?,
        _ => unreachable!(),
    };
    Ok((kind.to_string(), payload))
}

pub fn build_empty_root_index() -> Map<String, Value> {
    [
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "toolVersion".to_string(),
            Value::String(tool_version().to_string()),
        ),
        (
            "apiVersion".to_string(),
            Value::Number(TOOL_API_VERSION.into()),
        ),
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        (RULES_SUBDIR.to_string(), Value::Array(Vec::new())),
        (CONTACT_POINTS_SUBDIR.to_string(), Value::Array(Vec::new())),
        (MUTE_TIMINGS_SUBDIR.to_string(), Value::Array(Vec::new())),
        (POLICIES_SUBDIR.to_string(), Value::Array(Vec::new())),
        (TEMPLATES_SUBDIR.to_string(), Value::Array(Vec::new())),
    ]
    .into_iter()
    .collect()
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

fn scaffold_identity(name: &str, fallback: &str) -> String {
    let identity = sanitize_path_component(name);
    if identity.is_empty() {
        fallback.to_string()
    } else {
        identity
    }
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
