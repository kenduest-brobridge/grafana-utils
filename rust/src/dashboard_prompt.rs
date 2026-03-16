//! Dashboard prompt output transformation.
//! Translates query panels into external dashboard import payload shape and keeps datasource mapping helpers.
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

use crate::common::{message, string_field, Result};

use super::{
    build_preserved_web_import_document, BUILTIN_DATASOURCE_NAMES, BUILTIN_DATASOURCE_TYPES,
};

fn known_datasource_type(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "prom" | "prometheus" => Some("prometheus"),
        "loki" => Some("loki"),
        "elastic" | "elasticsearch" => Some("elasticsearch"),
        "opensearch" => Some("grafana-opensearch-datasource"),
        "mysql" => Some("mysql"),
        "postgres" | "postgresql" => Some("postgres"),
        "mssql" => Some("mssql"),
        "influxdb" => Some("influxdb"),
        "tempo" => Some("tempo"),
        "jaeger" => Some("jaeger"),
        "zipkin" => Some("zipkin"),
        "cloudwatch" => Some("cloudwatch"),
        _ => None,
    }
}

pub(crate) fn datasource_type_alias(value: &str) -> &str {
    known_datasource_type(value).unwrap_or(value)
}

#[derive(Clone, Debug)]
struct ResolvedDatasource {
    key: String,
    input_label: String,
    ds_type: String,
    plugin_name: String,
    plugin_version: String,
}

#[derive(Clone, Debug)]
struct InputMapping {
    input_name: String,
    label: String,
    plugin_name: String,
    ds_type: String,
    plugin_version: String,
}

pub struct DatasourceCatalog {
    pub(crate) by_uid: BTreeMap<String, Map<String, Value>>,
    pub(crate) by_name: BTreeMap<String, Map<String, Value>>,
}

const DEFAULT_GENERATED_DATASOURCE_INPUT: &str = "DATASOURCE";

pub(crate) fn build_datasource_catalog(datasources: &[Map<String, Value>]) -> DatasourceCatalog {
    let mut by_uid = BTreeMap::new();
    let mut by_name = BTreeMap::new();
    for datasource in datasources {
        let uid = string_field(datasource, "uid", "");
        if !uid.is_empty() {
            by_uid.insert(uid, datasource.clone());
        }
        let name = string_field(datasource, "name", "");
        if !name.is_empty() {
            by_name.insert(name, datasource.clone());
        }
    }
    DatasourceCatalog { by_uid, by_name }
}

pub(crate) fn is_placeholder_string(value: &str) -> bool {
    value.starts_with('$')
}

fn extract_placeholder_name(value: &str) -> String {
    if value.starts_with("${") && value.ends_with('}') && value.len() > 3 {
        return value[2..value.len() - 1].to_string();
    }
    if value.starts_with('$') && value.len() > 1 {
        return value[1..].to_string();
    }
    value.to_string()
}

fn is_generated_input_placeholder(value: &str) -> bool {
    extract_placeholder_name(value).starts_with("DS_")
}

pub(crate) fn is_builtin_datasource_ref(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            BUILTIN_DATASOURCE_NAMES.contains(&text.as_str())
                || is_generated_input_placeholder(text)
        }
        Value::Object(object) => {
            let uid = object
                .get("uid")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let name = object
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let ds_type = object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            is_generated_input_placeholder(uid)
                || is_generated_input_placeholder(name)
                || BUILTIN_DATASOURCE_NAMES.contains(&uid)
                || BUILTIN_DATASOURCE_NAMES.contains(&name)
                || BUILTIN_DATASOURCE_TYPES.contains(&uid)
                || BUILTIN_DATASOURCE_TYPES.contains(&ds_type)
        }
        _ => false,
    }
}

pub(crate) fn collect_datasource_refs(node: &Value, refs: &mut Vec<Value>) {
    match node {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "datasource" {
                    refs.push(value.clone());
                }
                collect_datasource_refs(value, refs);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_datasource_refs(item, refs);
            }
        }
        _ => {}
    }
}

fn make_input_name(label: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_underscore = false;
    for character in label.chars().flat_map(|character| character.to_uppercase()) {
        if character.is_ascii_alphanumeric() {
            normalized.push(character);
            last_was_underscore = false;
        } else if !last_was_underscore {
            normalized.push('_');
            last_was_underscore = true;
        }
    }
    let normalized = normalized.trim_matches('_').to_string();
    format!(
        "DS_{}",
        if normalized.is_empty() {
            DEFAULT_GENERATED_DATASOURCE_INPUT
        } else {
            &normalized
        }
    )
}

fn format_plugin_name(datasource_type: &str) -> String {
    match datasource_type_alias(datasource_type) {
        "cloudwatch" => return "CloudWatch".to_string(),
        "grafana-opensearch-datasource" => return "OpenSearch".to_string(),
        "influxdb" => return "InfluxDB".to_string(),
        "mssql" => return "Microsoft SQL Server".to_string(),
        "mysql" => return "MySQL".to_string(),
        "postgres" => return "PostgreSQL".to_string(),
        _ => {}
    }
    datasource_type_alias(datasource_type)
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn make_input_label(datasource_type: &str, index: usize) -> String {
    let title = format_plugin_name(datasource_type);
    if index == 1 {
        format!("{title} datasource")
    } else {
        format!("{title} datasource {index}")
    }
}

fn build_resolved_datasource(
    key: String,
    ds_type: String,
    input_label: String,
) -> ResolvedDatasource {
    let plugin_name = format_plugin_name(&ds_type);
    ResolvedDatasource {
        key,
        input_label,
        ds_type,
        plugin_name,
        plugin_version: String::new(),
    }
}

fn datasource_plugin_version(datasource: &Map<String, Value>) -> String {
    if let Some(version) = datasource
        .get("pluginVersion")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return version.to_string();
    }
    if let Some(version) = datasource
        .get("version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return version.to_string();
    }
    datasource
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("info"))
        .and_then(Value::as_object)
        .and_then(|info| info.get("version"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn lookup_datasource(
    datasource_catalog: &DatasourceCatalog,
    uid: Option<&str>,
    name: Option<&str>,
) -> Option<Map<String, Value>> {
    if let Some(uid) = uid.filter(|value| !value.is_empty()) {
        if let Some(datasource) = datasource_catalog.by_uid.get(uid) {
            return Some(datasource.clone());
        }
    }
    if let Some(name) = name.filter(|value| !value.is_empty()) {
        return datasource_catalog.by_name.get(name).cloned();
    }
    None
}

pub(crate) fn resolve_datasource_type_alias(
    reference: &str,
    datasource_catalog: &DatasourceCatalog,
) -> Option<String> {
    if let Some(alias) = known_datasource_type(reference) {
        return Some(alias.to_string());
    }
    let lower = reference.to_ascii_lowercase();
    for candidate in datasource_catalog.by_uid.values() {
        let candidate_type = string_field(candidate, "type", "");
        if !candidate_type.is_empty() && candidate_type.eq_ignore_ascii_case(&lower) {
            return Some(candidate_type);
        }
    }
    None
}

fn resolve_string_datasource_ref(
    reference: &str,
    datasource_catalog: &DatasourceCatalog,
) -> Result<ResolvedDatasource> {
    if let Some(datasource) =
        lookup_datasource(datasource_catalog, Some(reference), Some(reference))
    {
        let uid = string_field(&datasource, "uid", reference);
        let ds_type = string_field(&datasource, "type", "");
        if ds_type.is_empty() {
            return Err(message(format!(
                "Datasource {reference:?} does not have a usable type."
            )));
        }
        let label = string_field(&datasource, "name", reference);
        let mut resolved = build_resolved_datasource(format!("uid:{uid}"), ds_type, label);
        resolved.plugin_version = datasource_plugin_version(&datasource);
        return Ok(resolved);
    }

    if let Some(datasource_type) = resolve_datasource_type_alias(reference, datasource_catalog) {
        return Ok(build_resolved_datasource(
            format!("type:{datasource_type}"),
            datasource_type.clone(),
            format_plugin_name(&datasource_type),
        ));
    }

    Err(message(format!(
        "Cannot resolve datasource name or uid {reference:?} for prompt export."
    )))
}

fn resolve_placeholder_object_ref(
    uid: Option<&str>,
    name: Option<&str>,
    ds_type: Option<&str>,
) -> Option<ResolvedDatasource> {
    let ds_type = ds_type.filter(|value| !value.is_empty())?;
    let placeholder_value = if uid.is_some_and(is_placeholder_string) {
        uid
    } else if name.is_some_and(is_placeholder_string) {
        name
    } else {
        None
    }?;
    let token = extract_placeholder_name(placeholder_value);
    Some(build_resolved_datasource(
        format!("var:{ds_type}:{token}"),
        ds_type.to_string(),
        format_plugin_name(ds_type),
    ))
}

fn resolve_object_datasource_ref(
    reference: &Map<String, Value>,
    datasource_catalog: &DatasourceCatalog,
) -> Result<Option<ResolvedDatasource>> {
    let uid = reference.get("uid").and_then(Value::as_str);
    let name = reference.get("name").and_then(Value::as_str);
    let ds_type = reference.get("type").and_then(Value::as_str);
    let has_placeholder =
        uid.is_some_and(is_placeholder_string) || name.is_some_and(is_placeholder_string);

    if let Some(resolved) = resolve_placeholder_object_ref(uid, name, ds_type) {
        return Ok(Some(resolved));
    }
    if has_placeholder {
        return Ok(None);
    }

    let datasource = lookup_datasource(datasource_catalog, uid, name);
    let mut resolved_type = ds_type.unwrap_or_default().to_string();
    let mut resolved_label = name.unwrap_or(uid.unwrap_or_default()).to_string();
    let mut resolved_uid = uid.unwrap_or(name.unwrap_or_default()).to_string();
    if let Some(ref datasource) = datasource {
        if resolved_type.is_empty() {
            resolved_type = string_field(datasource, "type", "");
        }
        let datasource_name = string_field(datasource, "name", "");
        if !datasource_name.is_empty() {
            resolved_label = datasource_name;
        }
        if resolved_uid.is_empty() {
            resolved_uid = string_field(datasource, "uid", "");
        }
    }

    if resolved_type.is_empty() {
        return Err(message(format!(
            "Cannot resolve datasource type from reference {:?}.",
            Value::Object(reference.clone())
        )));
    }
    if resolved_uid.is_empty() {
        resolved_uid = resolved_type.clone();
    }
    if resolved_label.is_empty() {
        resolved_label = resolved_type.clone();
    }

    let mut resolved =
        build_resolved_datasource(format!("uid:{resolved_uid}"), resolved_type, resolved_label);
    if let Some(datasource) = datasource {
        resolved.plugin_version = datasource_plugin_version(&datasource);
    }
    Ok(Some(resolved))
}

fn resolve_datasource_ref(
    reference: &Value,
    datasource_catalog: &DatasourceCatalog,
) -> Result<Option<ResolvedDatasource>> {
    if reference.is_null() || is_builtin_datasource_ref(reference) {
        return Ok(None);
    }
    match reference {
        Value::String(text) => {
            if is_placeholder_string(text) {
                Ok(None)
            } else {
                resolve_string_datasource_ref(text, datasource_catalog).map(Some)
            }
        }
        Value::Object(object) => resolve_object_datasource_ref(object, datasource_catalog),
        _ => Ok(None),
    }
}

fn allocate_input_mapping(
    resolved: &ResolvedDatasource,
    ref_mapping: &mut BTreeMap<String, InputMapping>,
    type_counts: &mut BTreeMap<String, usize>,
    key_override: Option<String>,
) -> InputMapping {
    let mapping_key = key_override.unwrap_or_else(|| resolved.key.clone());
    if let Some(mapping) = ref_mapping.get(&mapping_key) {
        return mapping.clone();
    }
    let input_label = if resolved.input_label.is_empty() {
        resolved.plugin_name.clone()
    } else {
        resolved.input_label.clone()
    };
    let input_base = make_input_name(&input_label);
    let count = type_counts.get(&input_base).copied().unwrap_or(0) + 1;
    type_counts.insert(input_base.clone(), count);
    let mapping = InputMapping {
        input_name: if count == 1 {
            input_base
        } else {
            format!("{input_base}_{count}")
        },
        label: if resolved.input_label.is_empty() {
            make_input_label(&resolved.ds_type, count)
        } else {
            resolved.input_label.clone()
        },
        plugin_name: resolved.plugin_name.clone(),
        ds_type: resolved.ds_type.clone(),
        plugin_version: resolved.plugin_version.clone(),
    };
    ref_mapping.insert(mapping_key, mapping.clone());
    mapping
}

fn rewrite_template_variable_query(
    variable: &mut Map<String, Value>,
    mapping: &InputMapping,
    datasource_var_mappings: &mut BTreeMap<String, InputMapping>,
    datasource_var_placeholders: &mut BTreeSet<String>,
) {
    if let Some(name) = variable
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        datasource_var_mappings.insert(name.to_string(), mapping.clone());
        datasource_var_placeholders.insert(format!("${name}"));
        datasource_var_placeholders.insert(format!("${{{name}}}"));
    }
    variable.insert("current".to_string(), Value::Object(Map::new()));
    variable.insert("options".to_string(), Value::Array(Vec::new()));
    variable.insert("query".to_string(), Value::String(mapping.ds_type.clone()));
    variable.insert("refresh".to_string(), Value::from(1));
    if !variable.contains_key("regex") {
        variable.insert("regex".to_string(), Value::String(String::new()));
    }
    if variable.get("hide").and_then(Value::as_i64) == Some(0) {
        variable.remove("hide");
    }
}

fn rewrite_template_variable_datasource(
    variable: &mut Map<String, Value>,
    datasource_var_mappings: &BTreeMap<String, InputMapping>,
    datasource_var_placeholders: &BTreeSet<String>,
) {
    let placeholder_value = match variable.get("datasource") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Object(object)) => object
            .get("uid")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
        _ => None,
    };
    let Some(placeholder_value) = placeholder_value else {
        return;
    };
    let mapping = datasource_var_mappings.get(&extract_placeholder_name(&placeholder_value));
    if !datasource_var_placeholders.contains(&placeholder_value) || mapping.is_none() {
        return;
    }
    let mapping = mapping.unwrap();
    variable.insert(
        "datasource".to_string(),
        Value::Object(Map::from_iter([
            ("type".to_string(), Value::String(mapping.ds_type.clone())),
            (
                "uid".to_string(),
                Value::String(format!("${{{}}}", mapping.input_name)),
            ),
        ])),
    );
    variable.insert("current".to_string(), Value::Object(Map::new()));
    variable.insert("options".to_string(), Value::Array(Vec::new()));
}

fn prepare_templating_for_external_import(
    dashboard: &mut Map<String, Value>,
    ref_mapping: &mut BTreeMap<String, InputMapping>,
    type_counts: &mut BTreeMap<String, usize>,
    datasource_catalog: &DatasourceCatalog,
) {
    let Some(templating) = dashboard
        .get_mut("templating")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let Some(variables) = templating.get_mut("list").and_then(Value::as_array_mut) else {
        return;
    };

    let mut datasource_var_mappings = BTreeMap::new();
    let mut datasource_var_placeholders = BTreeSet::new();

    for variable in variables.iter_mut() {
        let Some(variable_object) = variable.as_object_mut() else {
            continue;
        };
        if variable_object.get("type").and_then(Value::as_str) != Some("datasource") {
            continue;
        }
        let Some(query) = variable_object
            .get("query")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(resolved) =
            resolve_datasource_ref(&Value::String(query.to_string()), datasource_catalog)
                .ok()
                .flatten()
        else {
            continue;
        };
        let variable_name = variable_object
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(&resolved.key);
        let mapping = allocate_input_mapping(
            &resolved,
            ref_mapping,
            type_counts,
            Some(format!("templating:{variable_name}")),
        );
        rewrite_template_variable_query(
            variable_object,
            &mapping,
            &mut datasource_var_mappings,
            &mut datasource_var_placeholders,
        );
    }

    for variable in variables.iter_mut() {
        if let Some(variable_object) = variable.as_object_mut() {
            rewrite_template_variable_datasource(
                variable_object,
                &datasource_var_mappings,
                &datasource_var_placeholders,
            );
        }
    }
}

fn replace_datasource_refs_in_dashboard(
    node: &mut Value,
    ref_mapping: &BTreeMap<String, InputMapping>,
    datasource_catalog: &DatasourceCatalog,
) -> Result<()> {
    match node {
        Value::Object(object) => {
            if let Some(datasource_value) = object.get_mut("datasource") {
                if let Some(resolved) =
                    resolve_datasource_ref(datasource_value, datasource_catalog)?
                {
                    let mapping = ref_mapping.get(&resolved.key).ok_or_else(|| {
                        message(format!(
                            "Missing datasource input mapping for {}",
                            resolved.key
                        ))
                    })?;
                    let placeholder = format!("${{{}}}", mapping.input_name);
                    let replacement = if datasource_value.is_object() {
                        let mut replacement = Map::new();
                        replacement.insert("uid".to_string(), Value::String(placeholder));
                        if !resolved.ds_type.is_empty() {
                            replacement.insert("type".to_string(), Value::String(resolved.ds_type));
                        }
                        Value::Object(replacement)
                    } else {
                        Value::String(placeholder)
                    };
                    *datasource_value = replacement;
                }
            }
            let keys = object.keys().cloned().collect::<Vec<String>>();
            for key in keys {
                if key == "datasource" {
                    continue;
                }
                if let Some(value) = object.get_mut(&key) {
                    replace_datasource_refs_in_dashboard(value, ref_mapping, datasource_catalog)?;
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                replace_datasource_refs_in_dashboard(item, ref_mapping, datasource_catalog)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn ensure_datasource_template_variable(dashboard: &mut Map<String, Value>, datasource_type: &str) {
    let templating = dashboard
        .entry("templating".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(templating_object) = templating.as_object_mut() else {
        return;
    };
    let variables = templating_object
        .entry("list".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(variables_array) = variables.as_array_mut() else {
        return;
    };

    if variables_array.iter().any(|variable| {
        variable
            .as_object()
            .and_then(|object| object.get("type"))
            .and_then(Value::as_str)
            == Some("datasource")
    }) {
        return;
    }

    variables_array.insert(
        0,
        json!({
            "current": {},
            "label": "Data source",
            "name": "datasource",
            "options": [],
            "query": datasource_type,
            "refresh": 1,
            "regex": "",
            "type": "datasource",
        }),
    );
}

fn rewrite_panel_datasources_to_template_variable(
    panels: &mut [Value],
    placeholder_names: &BTreeSet<String>,
) {
    for panel in panels {
        let Some(panel_object) = panel.as_object_mut() else {
            continue;
        };
        if let Some(datasource) = panel_object.get_mut("datasource") {
            match datasource {
                Value::String(text)
                    if placeholder_names.contains(text)
                        || text == "$datasource"
                        || text == "${datasource}" =>
                {
                    *datasource = json!({"uid": "$datasource"});
                }
                Value::Object(object) => {
                    let uid = object
                        .get("uid")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if placeholder_names.contains(uid)
                        || uid == "$datasource"
                        || uid == "${datasource}"
                    {
                        *datasource = json!({"uid": "$datasource"});
                    }
                }
                _ => {}
            }
        }

        if let Some(nested) = panel_object.get_mut("panels").and_then(Value::as_array_mut) {
            rewrite_panel_datasources_to_template_variable(nested, placeholder_names);
        }
    }
}

fn collect_panel_types(panels: &[Value], panel_types: &mut BTreeSet<String>) {
    for panel in panels {
        let Some(panel_object) = panel.as_object() else {
            continue;
        };
        let panel_type = string_field(panel_object, "type", "");
        if !panel_type.is_empty() {
            panel_types.insert(panel_type);
        }
        if let Some(nested) = panel_object.get("panels").and_then(Value::as_array) {
            collect_panel_types(nested, panel_types);
        }
    }
}

fn build_input_definitions(ref_mapping: &BTreeMap<String, InputMapping>) -> Value {
    let mut mappings = ref_mapping.values().cloned().collect::<Vec<InputMapping>>();
    mappings.sort_by(|left, right| left.input_name.cmp(&right.input_name));
    Value::Array(
        mappings
            .into_iter()
            .map(|mapping| {
                json!({
                    "name": mapping.input_name,
                    "label": mapping.label,
                    "description": "",
                    "type": "datasource",
                    "pluginId": mapping.ds_type,
                    "pluginName": mapping.plugin_name,
                })
            })
            .collect(),
    )
}

fn build_requires_block(
    ref_mapping: &BTreeMap<String, InputMapping>,
    panel_types: &BTreeSet<String>,
) -> Value {
    let mut requires = vec![json!({
        "type": "grafana",
        "id": "grafana",
        "name": "Grafana",
        "version": "",
    })];
    let mut datasource_plugins = BTreeMap::new();
    for mapping in ref_mapping.values() {
        let entry = datasource_plugins
            .entry(mapping.ds_type.clone())
            .or_insert_with(|| (mapping.plugin_name.clone(), mapping.plugin_version.clone()));
        if entry.1.is_empty() && !mapping.plugin_version.is_empty() {
            *entry = (mapping.plugin_name.clone(), mapping.plugin_version.clone());
        }
    }
    for (plugin_id, (plugin_name, plugin_version)) in datasource_plugins {
        requires.push(json!({
            "type": "datasource",
            "id": plugin_id,
            "name": plugin_name,
            "version": plugin_version,
        }));
    }
    for panel_type in panel_types {
        requires.push(json!({
            "type": "panel",
            "id": panel_type,
            "name": panel_type,
            "version": "",
        }));
    }
    Value::Array(requires)
}

pub fn build_external_export_document(
    payload: &Value,
    datasource_catalog: &DatasourceCatalog,
) -> Result<Value> {
    let mut dashboard = build_preserved_web_import_document(payload)?;
    let dashboard_object = dashboard
        .as_object_mut()
        .ok_or_else(|| message("Unexpected dashboard payload from Grafana."))?;

    let mut refs = Vec::new();
    collect_datasource_refs(&Value::Object(dashboard_object.clone()), &mut refs);

    let mut ref_mapping = BTreeMap::new();
    let mut type_counts = BTreeMap::new();
    prepare_templating_for_external_import(
        dashboard_object,
        &mut ref_mapping,
        &mut type_counts,
        datasource_catalog,
    );
    for reference in refs {
        let Some(resolved) = resolve_datasource_ref(&reference, datasource_catalog)? else {
            continue;
        };
        if ref_mapping.contains_key(&resolved.key) {
            continue;
        }
        allocate_input_mapping(&resolved, &mut ref_mapping, &mut type_counts, None);
    }

    replace_datasource_refs_in_dashboard(&mut dashboard, &ref_mapping, datasource_catalog)?;

    let datasource_types = ref_mapping
        .values()
        .map(|mapping| mapping.ds_type.clone())
        .collect::<BTreeSet<String>>();
    if datasource_types.len() == 1 && ref_mapping.len() == 1 {
        let datasource_type = datasource_types.iter().next().cloned().unwrap_or_default();
        let dashboard_object = dashboard
            .as_object_mut()
            .ok_or_else(|| message("Unexpected dashboard payload from Grafana."))?;
        ensure_datasource_template_variable(dashboard_object, &datasource_type);
        let placeholder_names = ref_mapping
            .values()
            .map(|mapping| format!("${{{}}}", mapping.input_name))
            .collect::<BTreeSet<String>>();
        if let Some(panels) = dashboard_object
            .get_mut("panels")
            .and_then(Value::as_array_mut)
        {
            rewrite_panel_datasources_to_template_variable(panels, &placeholder_names);
        }
    }

    let mut panel_types = BTreeSet::new();
    if let Some(panels) = dashboard.get("panels").and_then(Value::as_array) {
        collect_panel_types(panels, &mut panel_types);
    }
    let dashboard_object = dashboard
        .as_object_mut()
        .ok_or_else(|| message("Unexpected dashboard payload from Grafana."))?;
    dashboard_object.insert(
        "__inputs".to_string(),
        build_input_definitions(&ref_mapping),
    );
    dashboard_object.insert(
        "__requires".to_string(),
        build_requires_block(&ref_mapping, &panel_types),
    );
    dashboard_object.insert("__elements".to_string(), Value::Object(Map::new()));
    Ok(dashboard)
}
