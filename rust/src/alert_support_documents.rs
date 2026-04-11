use crate::common::{message, tool_version, value_as_object, Result};
use serde_json::{json, Map, Value};

use super::super::{
    CONTACT_POINTS_SUBDIR, CONTACT_POINT_KIND, MUTE_TIMINGS_SUBDIR, MUTE_TIMING_KIND,
    POLICIES_KIND, POLICIES_SUBDIR, ROOT_INDEX_KIND, RULES_SUBDIR, RULE_KIND, TEMPLATES_SUBDIR,
    TEMPLATE_KIND, TOOL_API_VERSION, TOOL_SCHEMA_VERSION,
};
use super::alert_support_normalize::strip_server_managed_fields;
use super::alert_support_paths::resource_subdir_by_kind;

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

fn build_rule_metadata(rule: &Map<String, Value>) -> Value {
    json!({
        "uid": crate::common::string_field(rule, "uid", ""),
        "title": crate::common::string_field(rule, "title", ""),
        "folderUID": crate::common::string_field(rule, "folderUID", ""),
        "ruleGroup": crate::common::string_field(rule, "ruleGroup", ""),
    })
}

fn build_contact_point_metadata(contact_point: &Map<String, Value>) -> Value {
    json!({
        "uid": crate::common::string_field(contact_point, "uid", ""),
        "name": crate::common::string_field(contact_point, "name", ""),
        "type": crate::common::string_field(contact_point, "type", ""),
    })
}

fn build_mute_timing_metadata(mute_timing: &Map<String, Value>) -> Value {
    json!({ "name": crate::common::string_field(mute_timing, "name", "") })
}

fn build_policies_metadata(policies: &Map<String, Value>) -> Value {
    json!({ "receiver": crate::common::string_field(policies, "receiver", "") })
}

fn build_template_metadata(template: &Map<String, Value>) -> Value {
    json!({ "name": crate::common::string_field(template, "name", "") })
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
