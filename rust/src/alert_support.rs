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
    collect_json_files(import_dir, &mut files)?;
    files.retain(|path| path.file_name().and_then(|value| value.to_str()) != Some("index.json"));
    files.sort();
    if files.is_empty() {
        return Err(message(format!(
            "No alerting resource JSON files found in {}",
            import_dir.display()
        )));
    }
    Ok(files)
}

fn collect_json_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, files)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }
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
