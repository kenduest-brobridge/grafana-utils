//! Prompt-lane preservation helpers for raw-to-prompt conversions.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::common::Result;
use crate::dashboard::prompt::{build_library_panel_export, collect_library_panel_uids};
use crate::grafana_api::DashboardResourceClient;

use super::{
    build_http_client, build_http_client_for_org, CommonCliArgs, RawToPromptArgs, DEFAULT_TIMEOUT,
    DEFAULT_URL,
};

pub(crate) fn raw_to_prompt_live_lookup_requested(args: &RawToPromptArgs) -> bool {
    args.profile.is_some()
        || args.url.is_some()
        || args.api_token.is_some()
        || args.username.is_some()
        || args.password.is_some()
        || args.prompt_password
        || args.prompt_token
        || args.org_id.is_some()
        || args.timeout.is_some()
        || args.verify_ssl
}

pub(crate) fn load_live_library_panel_exports(
    args: &RawToPromptArgs,
    dashboard_payload: &Value,
) -> Result<BTreeMap<String, Value>> {
    let common = CommonCliArgs {
        color: args.color,
        profile: args.profile.clone(),
        url: args.url.clone().unwrap_or_else(|| DEFAULT_URL.to_string()),
        api_token: args.api_token.clone(),
        username: args.username.clone(),
        password: args.password.clone(),
        prompt_password: args.prompt_password,
        prompt_token: args.prompt_token,
        timeout: args.timeout.unwrap_or(DEFAULT_TIMEOUT),
        verify_ssl: args.verify_ssl,
    };
    let client = match args.org_id {
        Some(org_id) => build_http_client_for_org(&common, org_id)?,
        None => build_http_client(&common)?,
    };
    let dashboard = DashboardResourceClient::new(&client);
    let (exports, warnings) = collect_live_library_panel_exports_with_request(
        dashboard_payload,
        |method, path, params, payload| dashboard.request_json(method, path, params, payload),
    )?;
    for warning in warnings {
        eprintln!("Dashboard raw-to-prompt warning: {warning}");
    }
    Ok(exports)
}

fn collect_live_library_panel_exports_with_request<F>(
    dashboard_payload: &Value,
    mut request_json: F,
) -> Result<(BTreeMap<String, Value>, Vec<String>)>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut exports = BTreeMap::new();
    let mut warnings = Vec::new();

    for uid in collect_library_panel_uids(dashboard_payload) {
        match request_json(
            reqwest::Method::GET,
            &format!("/api/library-elements/{uid}"),
            &[],
            None,
        ) {
            Ok(Some(value)) => match build_library_panel_export(&value) {
                Ok((export_uid, export)) => {
                    exports.insert(export_uid, export);
                }
                Err(error) => warnings.push(format!(
                    "Failed to normalize library panel {uid} for export: {error}"
                )),
            },
            Ok(None) => warnings.push(format!(
                "Library panel {uid} did not return a response from Grafana."
            )),
            Err(error) => warnings.push(format!(
                "Failed to fetch library panel {uid} for export: {error}"
            )),
        }
    }

    Ok((exports, warnings))
}

pub(crate) fn is_dashboard_v2_payload(payload: &Value) -> bool {
    let Some(object) = payload.as_object() else {
        return false;
    };
    let api_version = object
        .get("apiVersion")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if api_version.starts_with("dashboard.grafana.app/") {
        return true;
    }
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if kind != "Dashboard" {
        return false;
    }
    object
        .get("spec")
        .and_then(Value::as_object)
        .is_some_and(|spec| spec.contains_key("elements") || spec.contains_key("variables"))
}

pub(crate) fn collect_library_panel_portability_warnings(dashboard: &Value) -> Vec<String> {
    let mut count = 0usize;
    collect_library_panel_reference_count(dashboard, &mut count);
    if count == 0 {
        Vec::new()
    } else {
        vec![format!(
            "library panel external export is not fully portable yet; preserved {count} libraryPanel reference(s) without inlining models"
        )]
    }
}

fn collect_library_panel_reference_count(node: &Value, count: &mut usize) {
    match node {
        Value::Object(object) => {
            if object.contains_key("libraryPanel") {
                *count += 1;
            }
            for value in object.values() {
                collect_library_panel_reference_count(value, count);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_library_panel_reference_count(item, count);
            }
        }
        _ => {}
    }
}

pub(crate) fn collect_panel_placeholder_datasource_paths(document: &Value) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    collect_panel_placeholder_datasource_paths_recursive(document, "root", &mut paths);
    paths
}

fn collect_panel_placeholder_datasource_paths_recursive(
    node: &Value,
    current_path: &str,
    paths: &mut BTreeSet<String>,
) {
    match node {
        Value::Object(object) => {
            for (key, value) in object {
                let next_path = format!("{current_path}.{key}");
                if key == "datasource"
                    && current_path.contains(".panels[")
                    && is_placeholder_datasource_reference(value)
                {
                    paths.insert(next_path.clone());
                }
                collect_panel_placeholder_datasource_paths_recursive(value, &next_path, paths);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_panel_placeholder_datasource_paths_recursive(
                    item,
                    &format!("{current_path}[{index}]"),
                    paths,
                );
            }
        }
        _ => {}
    }
}

pub(crate) fn rewrite_prompt_panel_placeholder_paths(
    document: &mut Value,
    paths: &BTreeSet<String>,
) {
    rewrite_prompt_panel_placeholder_paths_recursive(document, "root", paths);
}

fn rewrite_prompt_panel_placeholder_paths_recursive(
    node: &mut Value,
    current_path: &str,
    paths: &BTreeSet<String>,
) {
    match node {
        Value::Object(object) => {
            if let Some(datasource) = object.get_mut("datasource") {
                let datasource_path = format!("{current_path}.datasource");
                if paths.contains(&datasource_path) {
                    *datasource = serde_json::json!({"uid": "$datasource"});
                }
            }
            for (key, value) in object {
                rewrite_prompt_panel_placeholder_paths_recursive(
                    value,
                    &format!("{current_path}.{key}"),
                    paths,
                );
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                rewrite_prompt_panel_placeholder_paths_recursive(
                    item,
                    &format!("{current_path}[{index}]"),
                    paths,
                );
            }
        }
        _ => {}
    }
}

pub(crate) fn is_placeholder_datasource_reference(reference: &Value) -> bool {
    match reference {
        Value::String(text) => text.starts_with('$'),
        Value::Object(object) => {
            object
                .get("uid")
                .and_then(Value::as_str)
                .is_some_and(|value| value.starts_with('$'))
                || object
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.starts_with('$'))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collect_live_library_panel_exports_with_request_returns_exports_for_models() {
        let dashboard = json!({
            "panels": [{
                "id": 1,
                "libraryPanel": {"uid": "shared-panel", "name": "Shared Panel"}
            }]
        });
        let (exports, warnings) =
            collect_live_library_panel_exports_with_request(&dashboard, |_, path, _, _| {
                assert_eq!(path, "/api/library-elements/shared-panel");
                Ok(Some(json!({
                    "result": {
                        "uid": "shared-panel",
                        "name": "Shared Panel",
                        "kind": 1,
                        "type": "graph",
                        "model": {
                            "type": "graph",
                            "datasource": {"uid": "prom-main", "type": "prometheus"}
                        }
                    }
                })))
            })
            .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(exports.len(), 1);
        assert_eq!(exports["shared-panel"]["model"]["type"], "graph");
    }

    #[test]
    fn collect_live_library_panel_exports_with_request_warns_when_model_is_missing() {
        let dashboard = json!({
            "panels": [{
                "id": 1,
                "libraryPanel": {"uid": "shared-panel", "name": "Shared Panel"}
            }]
        });
        let (exports, warnings) =
            collect_live_library_panel_exports_with_request(&dashboard, |_, path, _, _| {
                assert_eq!(path, "/api/library-elements/shared-panel");
                Ok(Some(json!({
                    "result": {
                        "uid": "shared-panel",
                        "name": "Shared Panel",
                        "kind": 1,
                        "type": "graph"
                    }
                })))
            })
            .unwrap();
        assert!(exports.is_empty());
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("Unexpected library panel payload")));
    }

    #[test]
    fn rewrite_prompt_panel_placeholder_paths_preserves_placeholder_datasource_slots() {
        let mut document = json!({
            "panels": [{
                "datasource": {"uid": "$datasource"}
            }]
        });
        let paths = collect_panel_placeholder_datasource_paths(&document);
        assert!(paths.contains("root.panels[0].datasource"));
        rewrite_prompt_panel_placeholder_paths(&mut document, &paths);
        assert_eq!(document["panels"][0]["datasource"]["uid"], "$datasource");
    }

    #[test]
    fn dashboard_v2_payload_is_detected_from_grafana_api_version() {
        let payload = json!({
            "apiVersion": "dashboard.grafana.app/v1alpha1",
            "kind": "Dashboard"
        });
        assert!(is_dashboard_v2_payload(&payload));
    }
}
