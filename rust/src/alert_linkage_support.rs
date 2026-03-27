use serde_json::{Map, Value};

use crate::common::{message, string_field, Result};

use super::{
    alert_client::GrafanaAlertClient, derive_dashboard_slug, value_to_string, AlertLinkageMappings,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleLinkage {
    pub(crate) dashboard_uid: String,
    pub(crate) panel_id: Option<String>,
}

pub(crate) fn get_rule_linkage(rule: &Map<String, Value>) -> Option<RuleLinkage> {
    let annotations = rule.get("annotations")?.as_object()?;
    let dashboard_uid = annotations
        .get("__dashboardUid__")
        .map(value_to_string)
        .unwrap_or_default()
        .trim()
        .to_string();
    if dashboard_uid.is_empty() {
        return None;
    }
    Some(RuleLinkage {
        dashboard_uid,
        panel_id: annotations.get("__panelId__").map(value_to_string),
    })
}

pub(crate) fn find_panel_by_id(
    panels: Option<&Vec<Value>>,
    panel_id: &str,
) -> Option<Map<String, Value>> {
    let panels = panels?;
    for panel in panels {
        let object = panel.as_object()?;
        if object.get("id").map(value_to_string).as_deref() == Some(panel_id) {
            return Some(object.clone());
        }
        let nested = object.get("panels").and_then(Value::as_array);
        if let Some(found) = find_panel_by_id(nested, panel_id) {
            return Some(found);
        }
    }
    None
}

pub(crate) fn build_linked_dashboard_metadata(
    client: &GrafanaAlertClient,
    rule: &Map<String, Value>,
) -> Result<Option<Map<String, Value>>> {
    let Some(linkage) = get_rule_linkage(rule) else {
        return Ok(None);
    };

    let mut metadata = Map::new();
    metadata.insert(
        "dashboardUid".to_string(),
        Value::String(linkage.dashboard_uid.clone()),
    );
    if let Some(panel_id) = linkage.panel_id.clone() {
        metadata.insert("panelId".to_string(), Value::String(panel_id));
    }

    let dashboard_payload = match client.get_dashboard(&linkage.dashboard_uid) {
        Ok(payload) => payload,
        Err(error) if error.status_code() == Some(404) => return Ok(Some(metadata)),
        Err(error) => return Err(error),
    };

    if let Some(dashboard) = dashboard_payload
        .get("dashboard")
        .and_then(Value::as_object)
    {
        metadata.insert(
            "dashboardTitle".to_string(),
            Value::String(string_field(dashboard, "title", "")),
        );
        if let Some(panel_id) = metadata.get("panelId").and_then(Value::as_str) {
            if let Some(panel) =
                find_panel_by_id(dashboard.get("panels").and_then(Value::as_array), panel_id)
            {
                metadata.insert(
                    "panelTitle".to_string(),
                    Value::String(string_field(&panel, "title", "")),
                );
                metadata.insert(
                    "panelType".to_string(),
                    Value::String(string_field(&panel, "type", "")),
                );
            }
        }
    }

    if let Some(meta) = dashboard_payload.get("meta").and_then(Value::as_object) {
        metadata.insert(
            "folderTitle".to_string(),
            Value::String(string_field(meta, "folderTitle", "")),
        );
        metadata.insert(
            "folderUid".to_string(),
            Value::String(string_field(meta, "folderUid", "")),
        );
        let slug_source = meta
            .get("url")
            .or_else(|| meta.get("slug"))
            .cloned()
            .unwrap_or(Value::String(String::new()));
        metadata.insert(
            "dashboardSlug".to_string(),
            Value::String(derive_dashboard_slug(&slug_source)),
        );
    }

    Ok(Some(metadata))
}

pub(crate) fn filter_dashboard_search_matches(
    candidates: Vec<Map<String, Value>>,
    linked_dashboard: &Map<String, Value>,
) -> Vec<Map<String, Value>> {
    let dashboard_title = string_field(linked_dashboard, "dashboardTitle", "");
    let mut filtered: Vec<Map<String, Value>> = candidates
        .into_iter()
        .filter(|item| string_field(item, "title", "") == dashboard_title)
        .collect();

    let folder_title = string_field(linked_dashboard, "folderTitle", "");
    if !folder_title.is_empty() {
        let folder_matches: Vec<Map<String, Value>> = filtered
            .iter()
            .filter(|item| string_field(item, "folderTitle", "") == folder_title)
            .cloned()
            .collect();
        if !folder_matches.is_empty() {
            filtered = folder_matches;
        }
    }

    let slug = derive_dashboard_slug(
        linked_dashboard
            .get("dashboardSlug")
            .unwrap_or(&Value::Null),
    );
    if !slug.is_empty() {
        let slug_matches: Vec<Map<String, Value>> = filtered
            .iter()
            .filter(|item| {
                derive_dashboard_slug(
                    item.get("url")
                        .or_else(|| item.get("slug"))
                        .unwrap_or(&Value::Null),
                ) == slug
            })
            .cloned()
            .collect();
        if !slug_matches.is_empty() {
            filtered = slug_matches;
        }
    }

    filtered
}

pub(crate) fn resolve_dashboard_uid_fallback(
    client: &GrafanaAlertClient,
    linked_dashboard: &Map<String, Value>,
) -> Result<String> {
    let dashboard_title = string_field(linked_dashboard, "dashboardTitle", "");
    if dashboard_title.is_empty() {
        return Err(message(
            "Alert rule references a dashboard UID that does not exist on the target Grafana, and the export file does not include dashboard title metadata for fallback matching.",
        ));
    }

    let filtered = filter_dashboard_search_matches(
        client.search_dashboards(&dashboard_title)?,
        linked_dashboard,
    );
    if filtered.len() == 1 {
        let resolved_uid = string_field(&filtered[0], "uid", "");
        if !resolved_uid.is_empty() {
            return Ok(resolved_uid);
        }
    }

    let folder_title = string_field(linked_dashboard, "folderTitle", "");
    let slug = derive_dashboard_slug(
        linked_dashboard
            .get("dashboardSlug")
            .unwrap_or(&Value::Null),
    );
    if filtered.is_empty() {
        return Err(message(format!(
            "Cannot resolve linked dashboard for alert rule. No dashboard matched title={dashboard_title:?}, folderTitle={folder_title:?}, slug={slug:?}.",
        )));
    }

    Err(message(format!(
        "Cannot resolve linked dashboard for alert rule. Multiple dashboards matched title={dashboard_title:?}, folderTitle={folder_title:?}, slug={slug:?}.",
    )))
}

pub(crate) fn rewrite_rule_dashboard_linkage(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    document: &Value,
    linkage_mappings: &AlertLinkageMappings,
) -> Result<Map<String, Value>> {
    let Some(linkage) = get_rule_linkage(payload) else {
        return Ok(payload.clone());
    };

    let source_dashboard_uid = linkage.dashboard_uid.clone();
    let source_panel_id = linkage.panel_id.clone().unwrap_or_default();
    let dashboard_uid = linkage_mappings.resolve_dashboard_uid(&source_dashboard_uid);
    let mapped_panel_id =
        linkage_mappings.resolve_panel_id(&source_dashboard_uid, &source_panel_id);

    let mut normalized = payload.clone();
    let annotations = normalized
        .entry("annotations".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| message("Alert-rule annotations must be an object."))?;
    annotations.insert(
        "__dashboardUid__".to_string(),
        Value::String(dashboard_uid.clone()),
    );
    if let Some(panel_id) = mapped_panel_id {
        annotations.insert("__panelId__".to_string(), Value::String(panel_id));
    }

    match client.get_dashboard(&dashboard_uid) {
        Ok(_) => return Ok(normalized),
        Err(error) if error.status_code() != Some(404) => return Err(error),
        Err(_) => {}
    }

    let linked_dashboard = document
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("linkedDashboard"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            message(format!(
                "Alert rule references dashboard UID {dashboard_uid:?}, but that dashboard does not exist on the target Grafana and the export file has no linked dashboard metadata for fallback matching.",
            ))
        })?;
    let replacement_uid = resolve_dashboard_uid_fallback(client, linked_dashboard)?;
    annotations.insert(
        "__dashboardUid__".to_string(),
        Value::String(replacement_uid),
    );
    Ok(normalized)
}
