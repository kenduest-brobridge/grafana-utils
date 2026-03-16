//! Staged bundle-level sync preflight helpers.
//!
//! Purpose:
//! - Aggregate staged sync preflight checks and datasource provider assessments
//!   into one reviewable bundle document.
//! - Keep Rust-side bundle planning pure and import-safe before any CLI wiring.

use crate::common::{message, Result};
use crate::datasource_provider::{build_provider_plan, iter_provider_names, summarize_provider_plan};
use crate::sync_preflight::build_sync_preflight_document;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub const SYNC_BUNDLE_PREFLIGHT_KIND: &str = "grafana-utils-sync-bundle-preflight";
pub const SYNC_BUNDLE_PREFLIGHT_SCHEMA_VERSION: i64 = 1;

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn require_object(value: Option<&Value>, label: &str) -> Result<Map<String, Value>> {
    match value {
        None => Ok(Map::new()),
        Some(Value::Object(object)) => Ok(object.clone()),
        Some(_) => Err(message(format!("{label} must be a JSON object."))),
    }
}

fn require_array<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a Vec<Value>> {
    match value {
        None => Err(message(format!("{label} must be a JSON array."))),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(message(format!("{label} must be a JSON array."))),
    }
}

fn require_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| message(format!("{label} must be a list.")))?;
    let mut result = Vec::new();
    for item in items {
        let text = normalize_text(Some(item));
        if !text.is_empty() {
            result.push(text);
        }
    }
    Ok(result)
}

fn build_provider_assessment(
    datasource_specs: &[Value],
    availability: &Map<String, Value>,
) -> Result<Value> {
    let provider_names = require_string_list(availability.get("providerNames"), "providerNames")?
        .into_iter()
        .collect::<BTreeSet<String>>();
    let mut plans = Vec::new();
    let mut checks = Vec::new();
    for datasource in datasource_specs {
        let Some(object) = datasource.as_object() else {
            continue;
        };
        let Some(Value::Object(_)) = object.get("secureJsonDataProviders") else {
            continue;
        };
        let mut datasource_spec = object.clone();
        let body = object
            .get("body")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if !datasource_spec.contains_key("uid") {
            if let Some(value) = body.get("uid") {
                datasource_spec.insert("uid".to_string(), value.clone());
            }
        }
        if !datasource_spec.contains_key("name") {
            if let Some(value) = body.get("name") {
                datasource_spec.insert("name".to_string(), value.clone());
            }
        }
        if !datasource_spec.contains_key("type") {
            if let Some(value) = body.get("type") {
                datasource_spec.insert("type".to_string(), value.clone());
            }
        }
        let plan = build_provider_plan(&datasource_spec)?;
        let plan_summary = summarize_provider_plan(&plan);
        for provider_name in iter_provider_names(&plan.references) {
            let missing = !provider_names.contains(provider_name);
            checks.push(Value::Object(Map::from_iter(vec![
                ("kind".to_string(), Value::String("secret-provider".to_string())),
                (
                    "datasourceName".to_string(),
                    Value::String(plan.datasource_name.clone()),
                ),
                (
                    "identity".to_string(),
                    Value::String(format!(
                        "{}->{}",
                        plan.datasource_uid
                            .clone()
                            .unwrap_or_else(|| plan.datasource_name.clone()),
                        provider_name
                    )),
                ),
                (
                    "providerName".to_string(),
                    Value::String(provider_name.to_string()),
                ),
                (
                    "status".to_string(),
                    Value::String(if missing { "missing" } else { "ok" }.to_string()),
                ),
                ("blocking".to_string(), Value::Bool(missing)),
            ])));
        }
        plans.push(plan_summary);
    }
    let reference_count = plans
        .iter()
        .map(|plan| {
            plan.get("providers")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0)
        })
        .sum::<usize>();
    let blocking_count = checks
        .iter()
        .filter(|item| item.get("blocking").and_then(Value::as_bool) == Some(true))
        .count();
    Ok(Value::Object(Map::from_iter(vec![
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "datasourceCount".to_string(),
                    Value::Number((plans.len() as i64).into()),
                ),
                (
                    "referenceCount".to_string(),
                    Value::Number((reference_count as i64).into()),
                ),
                (
                    "blockingCount".to_string(),
                    Value::Number((blocking_count as i64).into()),
                ),
            ])),
        ),
        ("plans".to_string(), Value::Array(plans)),
        ("checks".to_string(), Value::Array(checks)),
    ])))
}

pub fn build_sync_bundle_preflight_document(
    source_bundle: &Value,
    target_inventory: &Value,
    availability: Option<&Value>,
) -> Result<Value> {
    let source_bundle = require_object(Some(source_bundle), "source bundle")?;
    let _target_inventory = require_object(Some(target_inventory), "target inventory")?;
    let availability = require_object(availability, "availability")?;
    let mut desired_specs = Vec::new();
    for key in ["dashboards", "datasources", "folders", "alerts"] {
        let Some(items) = source_bundle.get(key) else {
            continue;
        };
        for item in require_array(Some(items), key)? {
            desired_specs.push(item.clone());
        }
    }
    let sync_preflight = build_sync_preflight_document(&desired_specs, Some(&Value::Object(availability.clone())))?;
    let provider_assessment = build_provider_assessment(
        source_bundle
            .get("datasources")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        &availability,
    )?;
    let sync_blocking_count = sync_preflight
        .get("summary")
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("blockingCount"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let provider_blocking_count = provider_assessment
        .get("summary")
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("blockingCount"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Ok(Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(SYNC_BUNDLE_PREFLIGHT_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(SYNC_BUNDLE_PREFLIGHT_SCHEMA_VERSION.into()),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "resourceCount".to_string(),
                    Value::Number((desired_specs.len() as i64).into()),
                ),
                (
                    "syncBlockingCount".to_string(),
                    Value::Number(sync_blocking_count.into()),
                ),
                (
                    "providerBlockingCount".to_string(),
                    Value::Number(provider_blocking_count.into()),
                ),
            ])),
        ),
        ("syncPreflight".to_string(), sync_preflight),
        ("providerAssessment".to_string(), provider_assessment),
    ])))
}

pub fn render_sync_bundle_preflight_text(document: &Value) -> Result<Vec<String>> {
    let kind = normalize_text(document.get("kind"));
    if kind != SYNC_BUNDLE_PREFLIGHT_KIND {
        return Err(message(
            "Sync bundle preflight document kind is not supported.",
        ));
    }
    let summary = require_object(document.get("summary"), "summary")?;
    Ok(vec![
        "Sync bundle preflight summary".to_string(),
        format!(
            "Resources: {} total",
            summary
                .get("resourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Sync blocking: {}",
            summary
                .get("syncBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Provider blocking: {}",
            summary
                .get("providerBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
    ])
}

#[cfg(test)]
#[path = "sync_bundle_rust_tests.rs"]
mod sync_bundle_rust_tests;
