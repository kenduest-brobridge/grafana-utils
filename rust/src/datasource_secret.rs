//! Staged secret-placeholder contract helpers for datasource imports.
//!
//! Purpose:
//! - Stage placeholder-based datasource secret requirements without wiring
//!   secret resolution yet.
//! - Keep placeholder declarations explicit, reviewable, and fail-closed.

use crate::common::{message, Result};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

/// Constant for placeholder reference prefix.
pub const SECRET_PLACEHOLDER_PREFIX: &str = "${secret:";
/// Constant for placeholder reference suffix.
pub const SECRET_PLACEHOLDER_SUFFIX: &str = "}";

/// Struct definition for SecretPlaceholderReference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecretPlaceholderReference {
    pub field_name: String,
    pub placeholder_name: String,
    pub raw_token: String,
}

/// Struct definition for DatasourceSecretPlaceholderPlan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatasourceSecretPlaceholderPlan {
    pub datasource_uid: Option<String>,
    pub datasource_name: String,
    pub datasource_type: String,
    pub placeholders: Vec<SecretPlaceholderReference>,
    pub provider_kind: String,
    pub action: String,
    pub review_required: bool,
}

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or("").trim().to_string()
}

/// parse secret placeholder.
pub fn parse_secret_placeholder(
    value: &Value,
    field_name: &str,
) -> Result<SecretPlaceholderReference> {
    let field_name = field_name.trim().to_string();
    if field_name.is_empty() {
        return Err(message("Secret field names must be non-empty strings."));
    }
    let token = value.as_str().ok_or_else(|| {
        message(format!(
            "Secret field '{field_name}' must use a placeholder string."
        ))
    })?;
    if !token.starts_with(SECRET_PLACEHOLDER_PREFIX) || !token.ends_with(SECRET_PLACEHOLDER_SUFFIX)
    {
        return Err(message(format!(
            "Secret field '{field_name}' must use ${{secret:...}} placeholders; opaque replay is not allowed."
        )));
    }
    let placeholder_name = token
        [SECRET_PLACEHOLDER_PREFIX.len()..token.len() - SECRET_PLACEHOLDER_SUFFIX.len()]
        .trim()
        .to_string();
    if placeholder_name.is_empty() {
        return Err(message(format!(
            "Secret field '{field_name}' must not use an empty placeholder name."
        )));
    }
    Ok(SecretPlaceholderReference {
        field_name,
        placeholder_name,
        raw_token: token.to_string(),
    })
}

/// collect secret placeholders.
pub fn collect_secret_placeholders(
    secure_json_data: Option<&Map<String, Value>>,
) -> Result<Vec<SecretPlaceholderReference>> {
    let Some(secure_json_data) = secure_json_data else {
        return Ok(Vec::new());
    };
    let mut field_names = secure_json_data.keys().cloned().collect::<Vec<_>>();
    field_names.sort();
    let mut placeholders = Vec::with_capacity(field_names.len());
    for field_name in field_names {
        let value = secure_json_data
            .get(&field_name)
            .expect("field name collected from same map");
        placeholders.push(parse_secret_placeholder(value, &field_name)?);
    }
    Ok(placeholders)
}

/// iter placeholder names.
pub fn iter_secret_placeholder_names(
    placeholders: &[SecretPlaceholderReference],
) -> impl Iterator<Item = &str> {
    let mut seen = HashSet::new();
    placeholders.iter().filter_map(move |item| {
        if seen.insert(item.placeholder_name.as_str()) {
            Some(item.placeholder_name.as_str())
        } else {
            None
        }
    })
}

/// resolve secret placeholders.
pub fn resolve_secret_placeholders(
    placeholders: &[SecretPlaceholderReference],
    provided_secrets: &Map<String, Value>,
) -> Result<Map<String, Value>> {
    let mut resolved = Map::new();
    let mut unresolved_placeholder_names = Vec::new();
    for placeholder in placeholders {
        let Some(secret_value) = provided_secrets
            .get(&placeholder.placeholder_name)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            unresolved_placeholder_names.push(placeholder.placeholder_name.clone());
            continue;
        };
        resolved.insert(
            placeholder.field_name.clone(),
            Value::String(secret_value.to_string()),
        );
    }
    if !unresolved_placeholder_names.is_empty() {
        unresolved_placeholder_names.sort();
        unresolved_placeholder_names.dedup();
        return Err(message(format!(
            "Datasource secret placeholders must resolve to non-empty strings before import: {}.",
            unresolved_placeholder_names.join(", ")
        )));
    }
    Ok(resolved)
}

/// build secret placeholder plan.
pub fn build_secret_placeholder_plan(
    datasource_spec: &Map<String, Value>,
) -> Result<DatasourceSecretPlaceholderPlan> {
    let datasource_name = normalize_text(datasource_spec.get("name").and_then(Value::as_str));
    let datasource_type = normalize_text(datasource_spec.get("type").and_then(Value::as_str));
    if datasource_name.is_empty() {
        return Err(message(
            "Datasource secret placeholder plan requires a datasource name.",
        ));
    }
    if datasource_type.is_empty() {
        return Err(message(
            "Datasource secret placeholder plan requires a datasource type.",
        ));
    }
    let secure_json_data = match datasource_spec.get("secureJsonDataPlaceholders") {
        None | Some(Value::Null) => None,
        Some(Value::Object(object)) => Some(object),
        Some(_) => {
            return Err(message(
                "Placeholder-backed secureJsonData input must be a JSON object.",
            ))
        }
    };
    Ok(DatasourceSecretPlaceholderPlan {
        datasource_uid: datasource_spec
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        datasource_name,
        datasource_type,
        placeholders: collect_secret_placeholders(secure_json_data)?,
        provider_kind: "inline-placeholder-map".to_string(),
        action: "inject-secrets".to_string(),
        review_required: true,
    })
}

/// summarize secret placeholder plan.
pub fn summarize_secret_placeholder_plan(plan: &DatasourceSecretPlaceholderPlan) -> Value {
    json!({
        "datasourceUid": plan.datasource_uid,
        "datasourceName": plan.datasource_name,
        "datasourceType": plan.datasource_type,
        "providerKind": plan.provider_kind,
        "action": plan.action,
        "reviewRequired": plan.review_required,
        "secretFields": plan.placeholders.iter().map(|item| item.field_name.clone()).collect::<Vec<_>>(),
        "placeholderNames": iter_secret_placeholder_names(&plan.placeholders).collect::<Vec<_>>(),
    })
}

/// render secret placeholder plan.
pub fn describe_secret_placeholder_plan(plan: &DatasourceSecretPlaceholderPlan) -> String {
    summarize_secret_placeholder_plan(plan).to_string()
}
