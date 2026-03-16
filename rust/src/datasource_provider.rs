//! Unwired secret-provider contract helpers for datasource imports.
//!
//! Purpose:
//! - Stage an external secret-provider contract without wiring provider I/O yet.
//! - Keep provider references explicit, reviewable, and fail-closed.
//!
//! Caveats:
//! - This module does not fetch secrets from any remote system.
//! - It only validates provider references and shapes a later resolution plan.

use crate::common::{message, Result};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

pub const PROVIDER_REFERENCE_PREFIX: &str = "${provider:";
pub const PROVIDER_REFERENCE_SUFFIX: &str = "}";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecretProviderReference {
    pub field_name: String,
    pub provider_name: String,
    pub secret_path: String,
    pub raw_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatasourceSecretProviderPlan {
    pub datasource_uid: Option<String>,
    pub datasource_name: String,
    pub datasource_type: String,
    pub references: Vec<SecretProviderReference>,
    pub provider_kind: String,
    pub action: String,
    pub review_required: bool,
}

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or("").trim().to_string()
}

pub fn parse_provider_reference(
    value: &Value,
    field_name: &str,
) -> Result<SecretProviderReference> {
    let field_name = field_name.trim().to_string();
    if field_name.is_empty() {
        return Err(message(
            "Secret provider field names must be non-empty strings.",
        ));
    }
    let token = value.as_str().ok_or_else(|| {
        message(format!(
            "Secret provider field '{field_name}' must use a placeholder string."
        ))
    })?;
    if !token.starts_with(PROVIDER_REFERENCE_PREFIX) || !token.ends_with(PROVIDER_REFERENCE_SUFFIX)
    {
        return Err(message(format!(
            "Secret provider field '{field_name}' must use ${{provider:NAME:PATH}} references; opaque replay is not allowed."
        )));
    }
    let inner =
        &token[PROVIDER_REFERENCE_PREFIX.len()..token.len() - PROVIDER_REFERENCE_SUFFIX.len()];
    let (provider_name_raw, secret_path_raw) = inner.split_once(':').ok_or_else(|| {
        message(format!(
            "Secret provider field '{field_name}' must use ${{provider:NAME:PATH}} references."
        ))
    })?;
    let provider_name = provider_name_raw.trim().to_string();
    let secret_path = secret_path_raw.trim().to_string();
    if provider_name.is_empty() || secret_path.is_empty() {
        return Err(message(format!(
            "Secret provider field '{field_name}' must include both provider name and secret path."
        )));
    }
    Ok(SecretProviderReference {
        field_name,
        provider_name,
        secret_path,
        raw_token: token.to_string(),
    })
}

pub fn collect_provider_references(
    secure_json_data: Option<&Map<String, Value>>,
) -> Result<Vec<SecretProviderReference>> {
    let Some(secure_json_data) = secure_json_data else {
        return Ok(Vec::new());
    };
    let mut field_names = secure_json_data.keys().cloned().collect::<Vec<_>>();
    field_names.sort();
    let mut references = Vec::with_capacity(field_names.len());
    for field_name in field_names {
        let value = secure_json_data
            .get(&field_name)
            .expect("field name collected from same map");
        references.push(parse_provider_reference(value, &field_name)?);
    }
    Ok(references)
}

pub fn build_provider_plan(
    datasource_spec: &Map<String, Value>,
) -> Result<DatasourceSecretProviderPlan> {
    let datasource_name = normalize_text(datasource_spec.get("name").and_then(Value::as_str));
    let datasource_type = normalize_text(datasource_spec.get("type").and_then(Value::as_str));
    if datasource_name.is_empty() {
        return Err(message(
            "Datasource provider plan requires a datasource name.",
        ));
    }
    if datasource_type.is_empty() {
        return Err(message(
            "Datasource provider plan requires a datasource type.",
        ));
    }

    let secure_json_data = match datasource_spec.get("secureJsonDataProviders") {
        None | Some(Value::Null) => None,
        Some(Value::Object(object)) => Some(object),
        Some(_) => {
            return Err(message(
                "Provider-backed secureJsonData input must be a JSON object.",
            ))
        }
    };

    Ok(DatasourceSecretProviderPlan {
        datasource_uid: datasource_spec
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        datasource_name,
        datasource_type,
        references: collect_provider_references(secure_json_data)?,
        provider_kind: "external-provider-reference".to_string(),
        action: "resolve-provider-secrets".to_string(),
        review_required: true,
    })
}

pub fn summarize_provider_plan(plan: &DatasourceSecretProviderPlan) -> Value {
    json!({
        "datasourceUid": plan.datasource_uid,
        "datasourceName": plan.datasource_name,
        "datasourceType": plan.datasource_type,
        "providerKind": plan.provider_kind,
        "action": plan.action,
        "reviewRequired": plan.review_required,
        "providers": plan.references.iter().map(|item| {
            json!({
                "fieldName": item.field_name,
                "providerName": item.provider_name,
                "secretPath": item.secret_path,
            })
        }).collect::<Vec<_>>(),
    })
}

pub fn iter_provider_names(references: &[SecretProviderReference]) -> impl Iterator<Item = &str> {
    let mut seen = HashSet::new();
    references.iter().filter_map(move |item| {
        if seen.insert(item.provider_name.as_str()) {
            Some(item.provider_name.as_str())
        } else {
            None
        }
    })
}
