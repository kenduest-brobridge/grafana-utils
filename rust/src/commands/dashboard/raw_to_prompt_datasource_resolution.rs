//! Datasource mapping and rewrite helpers for raw-to-prompt conversions.

use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::common::{message, sanitize_path_component, Result};
use crate::dashboard::inspect_query::{
    resolve_query_analyzer_family_from_datasource_type,
    resolve_query_analyzer_family_from_query_signature,
};

use super::collect_datasource_refs;
use super::raw_to_prompt_prompt_paths::is_placeholder_datasource_reference;
use super::raw_to_prompt_types::{
    DashboardScanContext, DatasourceMapDocument, RawToPromptStats, ResolvedDatasourceReplacement,
    MAPPING_KIND,
};
use super::DatasourceInventoryItem;
use super::RawToPromptResolution;

pub(crate) fn load_datasource_mapping(
    mapping_path: Option<&std::path::Path>,
) -> Result<Option<DatasourceMapDocument>> {
    let Some(mapping_path) = mapping_path else {
        return Ok(None);
    };
    if !mapping_path.exists() {
        return Ok(None);
    }
    let mut document = super::load_json_file(mapping_path)?;
    if document
        .get("datasources")
        .and_then(Value::as_array)
        .is_none()
    {
        if let Some(alt) = document.get("mapping") {
            document = alt.clone();
        }
    }
    if document
        .get("datasources")
        .and_then(Value::as_array)
        .is_none()
    {
        return Err(message(
            "Datasource mapping file must contain a datasources array.",
        ));
    }
    let mut parsed: DatasourceMapDocument = serde_json::from_value(document)?;
    if parsed.kind.trim().is_empty() {
        parsed.kind = MAPPING_KIND.to_string();
    }
    Ok(Some(parsed))
}

pub(crate) fn collect_reference_families(document: &mut Value, context: &mut DashboardScanContext) {
    let Some(dashboard) = document.as_object_mut() else {
        return;
    };
    let panel_default = dashboard.get("datasource").cloned();
    if let Some(panels) = dashboard.get_mut("panels").and_then(Value::as_array_mut) {
        for panel in panels {
            collect_panel_reference_families(panel, panel_default.as_ref(), context);
        }
    }
}

fn collect_panel_reference_families(
    panel: &mut Value,
    inherited_panel_datasource: Option<&Value>,
    context: &mut DashboardScanContext,
) {
    let Some(panel_object) = panel.as_object_mut() else {
        return;
    };
    let panel_datasource = panel_object
        .get("datasource")
        .cloned()
        .or_else(|| inherited_panel_datasource.cloned());
    if let Some(targets) = panel_object
        .get_mut("targets")
        .and_then(Value::as_array_mut)
    {
        for target in targets {
            let Some(target_object) = target.as_object_mut() else {
                continue;
            };
            let reference = target_object
                .get("datasource")
                .or(panel_datasource.as_ref());
            let Some(reference) = reference else {
                continue;
            };
            let ref_key = reference_identity_key(reference);
            let Some(ref_key) = ref_key else {
                continue;
            };
            if let Some(ds_type) = datasource_type_from_reference(reference) {
                if let Some(family) = resolve_query_analyzer_family_from_datasource_type(&ds_type) {
                    context
                        .ref_families
                        .entry(ref_key.clone())
                        .or_default()
                        .insert(family.to_string());
                }
            }
            for (field_name, field_value) in target_object.iter() {
                let Some(query_text) = field_value.as_str() else {
                    continue;
                };
                if let Some(family) =
                    resolve_query_analyzer_family_from_query_signature(field_name, query_text)
                {
                    context
                        .ref_families
                        .entry(ref_key.clone())
                        .or_default()
                        .insert(family.to_string());
                }
            }
        }
    }
    if let Some(rows) = panel_object.get_mut("rows").and_then(Value::as_array_mut) {
        for nested in rows {
            collect_panel_reference_families(nested, panel_datasource.as_ref(), context);
        }
    }
    if let Some(nested_panels) = panel_object.get_mut("panels").and_then(Value::as_array_mut) {
        for nested in nested_panels {
            collect_panel_reference_families(nested, panel_datasource.as_ref(), context);
        }
    }
}

pub(crate) fn rewrite_datasource_refs(
    document: &mut Value,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    scan: &DashboardScanContext,
    resolution: RawToPromptResolution,
    warnings: &mut Vec<String>,
    stats: &mut RawToPromptStats,
) -> Result<()> {
    let Some(dashboard) = document.as_object_mut() else {
        return Ok(());
    };
    rewrite_value_datasource_fields(
        dashboard,
        datasource_inventory,
        mapping,
        scan,
        resolution,
        warnings,
        stats,
    )
}

fn rewrite_value_datasource_fields(
    node: &mut Map<String, Value>,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    scan: &DashboardScanContext,
    resolution: RawToPromptResolution,
    warnings: &mut Vec<String>,
    stats: &mut RawToPromptStats,
) -> Result<()> {
    if let Some(reference) = node.get_mut("datasource") {
        rewrite_datasource_ref_value(
            reference,
            datasource_inventory,
            mapping,
            scan,
            resolution,
            warnings,
            stats,
        )?;
    }
    for value in node.values_mut() {
        match value {
            Value::Object(object) => rewrite_value_datasource_fields(
                object,
                datasource_inventory,
                mapping,
                scan,
                resolution,
                warnings,
                stats,
            )?,
            Value::Array(items) => {
                for item in items {
                    if let Value::Object(object) = item {
                        rewrite_value_datasource_fields(
                            object,
                            datasource_inventory,
                            mapping,
                            scan,
                            resolution,
                            warnings,
                            stats,
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn rewrite_datasource_ref_value(
    reference: &mut Value,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    scan: &DashboardScanContext,
    resolution: RawToPromptResolution,
    warnings: &mut Vec<String>,
    stats: &mut RawToPromptStats,
) -> Result<()> {
    if super::is_builtin_datasource_ref(reference)
        || reference.is_null()
        || is_placeholder_datasource_reference(reference)
    {
        return Ok(());
    }
    let Some(resolved) =
        resolve_replacement(reference, datasource_inventory, mapping, scan, resolution)?
    else {
        return Ok(());
    };
    if resolved.exact {
        stats.exact += 1;
    } else {
        stats.inferred += 1;
    }
    if let Some(warning) = resolved.warning.clone() {
        warnings.push(warning);
    }
    *reference = serde_json::json!({
        "uid": resolved.uid,
        "name": resolved.name,
        "type": resolved.datasource_type,
    });
    Ok(())
}

fn resolve_replacement(
    reference: &Value,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    scan: &DashboardScanContext,
    resolution: RawToPromptResolution,
) -> Result<Option<ResolvedDatasourceReplacement>> {
    if let Some(resolved) = resolve_from_mapping(reference, mapping)? {
        return Ok(Some(resolved));
    }
    if let Some(resolved) = resolve_from_inventory(reference, datasource_inventory) {
        return Ok(Some(resolved));
    }
    if let Some(resolved) = resolve_from_embedded_reference(reference) {
        return Ok(Some(resolved));
    }
    match resolution {
        RawToPromptResolution::Exact => Err(message(format!(
            "Cannot resolve datasource reference exactly: {}",
            reference_identity_key(reference).unwrap_or_else(|| reference.to_string())
        ))),
        RawToPromptResolution::Strict => Err(message(format!(
            "Strict datasource resolution failed for reference: {}",
            reference_identity_key(reference).unwrap_or_else(|| reference.to_string())
        ))),
        RawToPromptResolution::InferFamily => {
            if let Some(resolved) = resolve_from_family_inference(reference, scan) {
                return Ok(Some(resolved));
            }
            Err(message(format!(
                "Cannot infer datasource family for reference: {}",
                reference_identity_key(reference).unwrap_or_else(|| reference.to_string())
            )))
        }
    }
}

fn resolve_from_mapping(
    reference: &Value,
    mapping: Option<&DatasourceMapDocument>,
) -> Result<Option<ResolvedDatasourceReplacement>> {
    let Some(mapping) = mapping else {
        return Ok(None);
    };
    let reference_key = reference_identity_key(reference);
    let object = reference.as_object();
    let id_value = object
        .and_then(|item| item.get("id"))
        .map(|value| match value {
            Value::String(text) => text.clone(),
            _ => value.to_string(),
        });
    let uid_value = object
        .and_then(|item| item.get("uid"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let name_value = object
        .and_then(|item| item.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);

    for entry in &mapping.datasources {
        let is_match = entry
            .r#match
            .reference
            .as_ref()
            .is_some_and(|value| reference_key.as_ref().is_some_and(|key| key == value))
            || entry.r#match.id.as_ref().is_some_and(|value| {
                id_value
                    .as_ref()
                    .is_some_and(|candidate| candidate == value)
            })
            || entry.r#match.uid.as_ref().is_some_and(|value| {
                uid_value
                    .as_ref()
                    .is_some_and(|candidate| candidate == value)
            })
            || entry.r#match.name.as_ref().is_some_and(|value| {
                name_value
                    .as_ref()
                    .is_some_and(|candidate| candidate == value)
                    || matches!(reference, Value::String(text) if text == value)
            });
        if !is_match {
            continue;
        }
        if entry.replace.datasource_type.trim().is_empty() {
            return Err(message("Datasource mapping replace.type cannot be empty."));
        }
        let uid = entry
            .replace
            .uid
            .clone()
            .unwrap_or_else(|| synthetic_uid_from_reference(reference));
        let name = entry
            .replace
            .name
            .clone()
            .unwrap_or_else(|| synthetic_name(&entry.replace.datasource_type, &uid));
        return Ok(Some(ResolvedDatasourceReplacement {
            key: reference_key.unwrap_or_else(|| uid.clone()),
            uid,
            name,
            datasource_type: entry.replace.datasource_type.clone(),
            exact: true,
            warning: None,
        }));
    }
    Ok(None)
}

fn resolve_from_inventory(
    reference: &Value,
    datasource_inventory: &[DatasourceInventoryItem],
) -> Option<ResolvedDatasourceReplacement> {
    let object = reference.as_object();
    let reference_uid = object
        .and_then(|item| item.get("uid"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let reference_name = object
        .and_then(|item| item.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| match reference {
            Value::String(text) => Some(text.trim().to_string()),
            _ => None,
        });
    let matched = datasource_inventory.iter().find(|item| {
        reference_uid
            .as_ref()
            .is_some_and(|value| item.uid == *value)
            || reference_name
                .as_ref()
                .is_some_and(|value| item.name == *value || item.uid == *value)
    })?;
    Some(ResolvedDatasourceReplacement {
        key: reference_identity_key(reference).unwrap_or_else(|| matched.uid.clone()),
        uid: matched.uid.clone(),
        name: matched.name.clone(),
        datasource_type: matched.datasource_type.clone(),
        exact: true,
        warning: None,
    })
}

fn resolve_from_embedded_reference(reference: &Value) -> Option<ResolvedDatasourceReplacement> {
    let object = reference.as_object()?;
    let ds_type = object
        .get("type")
        .or_else(|| object.get("pluginId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let uid = object
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| synthetic_uid_from_reference(reference));
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| synthetic_name(ds_type, &uid));
    Some(ResolvedDatasourceReplacement {
        key: reference_identity_key(reference).unwrap_or_else(|| uid.clone()),
        uid,
        name,
        datasource_type: ds_type.to_string(),
        exact: true,
        warning: None,
    })
}

fn resolve_from_family_inference(
    reference: &Value,
    scan: &DashboardScanContext,
) -> Option<ResolvedDatasourceReplacement> {
    let key = reference_identity_key(reference)?;
    let families = scan.ref_families.get(&key)?;
    if families.len() != 1 {
        return None;
    }
    let family = families.iter().next()?.as_str();
    let datasource_type = match family {
        "prometheus" => "prometheus",
        "loki" => "loki",
        "flux" => "influxdb",
        _ => return None,
    };
    let uid = format!(
        "prompt-{}-{}",
        datasource_type,
        sanitize_path_component(&key).replace('_', "-")
    );
    Some(ResolvedDatasourceReplacement {
        key,
        uid: uid.clone(),
        name: synthetic_name(datasource_type, &uid),
        datasource_type: datasource_type.to_string(),
        exact: false,
        warning: Some(format!(
            "inferred datasource family {family} for unresolved dashboard reference"
        )),
    })
}

pub(crate) fn build_synthetic_catalog(document: &Value) -> Vec<Map<String, Value>> {
    let mut refs = Vec::new();
    collect_datasource_refs(document, &mut refs);
    let mut catalog = BTreeMap::<String, Map<String, Value>>::new();
    for reference in refs {
        let Some(object) = reference.as_object() else {
            continue;
        };
        let Some(ds_type) = object.get("type").and_then(Value::as_str) else {
            continue;
        };
        let uid = object
            .get("uid")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(ds_type);
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(uid);
        catalog.entry(uid.to_string()).or_insert_with(|| {
            Map::from_iter([
                ("uid".to_string(), Value::String(uid.to_string())),
                ("name".to_string(), Value::String(name.to_string())),
                ("type".to_string(), Value::String(ds_type.to_string())),
            ])
        });
    }
    catalog.into_values().collect()
}

fn reference_identity_key(reference: &Value) -> Option<String> {
    if let Value::String(text) = reference {
        let trimmed = text.trim();
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    let object = reference.as_object()?;
    object
        .get("uid")
        .or_else(|| object.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn datasource_type_from_reference(reference: &Value) -> Option<String> {
    let object = reference.as_object()?;
    object
        .get("type")
        .or_else(|| object.get("pluginId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn synthetic_uid_from_reference(reference: &Value) -> String {
    sanitize_path_component(
        &reference_identity_key(reference).unwrap_or_else(|| "datasource".into()),
    )
    .replace('_', "-")
}

fn synthetic_name(datasource_type: &str, uid: &str) -> String {
    format!("{datasource_type} ({uid})")
}
