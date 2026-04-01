//! Read model for dashboard and datasource listing.
//! This module keeps orchestration and org/path enrichment while summary rendering lives in a
//! focused sibling helper.
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

use crate::common::{message, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;
use crate::tabular_output::render_yaml;

use super::{
    build_datasource_catalog, build_folder_path, build_http_client, build_http_client_for_org,
    datasource_type_alias, extract_dashboard_object, fetch_dashboard_with_request,
    fetch_folder_if_exists_with_request, is_builtin_datasource_ref, is_placeholder_string,
    list_dashboard_summaries_with_request, list_datasources_with_request, lookup_datasource,
    resolve_datasource_type_alias, ListArgs, DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_TITLE,
    DEFAULT_FOLDER_UID, DEFAULT_UNKNOWN_UID,
};

#[path = "list_render.rs"]
mod list_render;

#[allow(unused_imports)]
pub(crate) use list_render::{
    format_dashboard_summary_line, render_dashboard_summary_csv, render_dashboard_summary_json,
    render_dashboard_summary_table, render_dashboard_summary_text,
};

/// attach dashboard folder paths with request.
pub(crate) fn attach_dashboard_folder_paths_with_request<F>(
    mut request_json: F,
    summaries: &[Map<String, Value>],
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut folder_paths = BTreeMap::new();
    for summary in summaries {
        let folder_uid = string_field(summary, "folderUid", "");
        let folder_title = string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE);
        if folder_uid.is_empty() || folder_paths.contains_key(&folder_uid) {
            continue;
        }
        let folder = fetch_folder_if_exists_with_request(&mut request_json, &folder_uid)?;
        let folder_path = match folder {
            Some(folder) => build_folder_path(&folder, &folder_title),
            None => folder_title,
        };
        folder_paths.insert(folder_uid, folder_path);
    }

    Ok(summaries
        .iter()
        .map(|summary| {
            let mut item = summary.clone();
            let folder_uid = string_field(summary, "folderUid", "");
            let folder_title = string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE);
            item.insert(
                "folderPath".to_string(),
                Value::String(
                    folder_paths
                        .get(&folder_uid)
                        .cloned()
                        .unwrap_or(folder_title),
                ),
            );
            item
        })
        .collect())
}

/// fetch current org with request.
pub(crate) fn fetch_current_org_with_request<F>(mut request_json: F) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, "/api/org", &[], None)? {
        Some(value) => {
            let object = value_as_object(&value, "Unexpected current-org payload from Grafana.")?;
            Ok(object.clone())
        }
        None => Err(message("Grafana did not return current-org metadata.")),
    }
}

/// Purpose: implementation note.
pub(crate) fn list_orgs_with_request<F>(mut request_json: F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, "/api/orgs", &[], None)? {
        Some(Value::Array(values)) => values
            .into_iter()
            .map(|value| {
                let object = value_as_object(&value, "Unexpected org list payload from Grafana.")?;
                Ok(object.clone())
            })
            .collect(),
        Some(_) => Err(message("Unexpected org list payload from Grafana.")),
        None => Ok(Vec::new()),
    }
}

/// attach dashboard org metadata.
pub(crate) fn attach_dashboard_org_metadata(
    summaries: &[Map<String, Value>],
    org: &Map<String, Value>,
) -> Vec<Map<String, Value>> {
    let org_name = string_field(org, "name", "");
    let org_id = org.get("id").cloned().unwrap_or(Value::Null);
    summaries
        .iter()
        .map(|summary| {
            let mut item = summary.clone();
            item.insert("orgName".to_string(), Value::String(org_name.clone()));
            item.insert("orgId".to_string(), org_id.clone());
            item
        })
        .collect()
}

/// org id value.
pub(crate) fn org_id_value(org: &Map<String, Value>) -> Result<i64> {
    org.get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Grafana org payload did not include a usable id."))
}

fn lookup_unique_datasource_name_by_type(
    datasources_by_uid: &BTreeMap<String, Map<String, Value>>,
    datasource_type: &str,
) -> Option<String> {
    let matches: BTreeSet<String> = datasources_by_uid
        .values()
        .filter(|datasource| {
            string_field(datasource, "type", "").eq_ignore_ascii_case(datasource_type)
        })
        .map(|datasource| {
            let name = string_field(datasource, "name", "");
            if name.is_empty() {
                string_field(datasource, "uid", datasource_type)
            } else {
                name
            }
        })
        .collect();
    if matches.len() == 1 {
        matches.iter().next().cloned()
    } else {
        None
    }
}

fn resolve_datasource_source_name(
    reference: &Value,
    datasource_catalog: &super::prompt::DatasourceCatalog,
) -> Option<String> {
    if reference.is_null() || is_builtin_datasource_ref(reference) {
        return None;
    }
    match reference {
        Value::String(text) => {
            if is_placeholder_string(text) {
                return None;
            }
            if let Some(datasource) = lookup_datasource(datasource_catalog, Some(text), Some(text))
            {
                let name = string_field(&datasource, "name", text);
                return Some(name);
            }
            resolve_datasource_type_alias(text, datasource_catalog)
                .and_then(|datasource_type| {
                    lookup_unique_datasource_name_by_type(
                        &datasource_catalog.by_uid,
                        &datasource_type,
                    )
                    .or_else(|| Some(datasource_type_alias(&datasource_type).to_string()))
                })
                .or_else(|| Some(text.to_string()))
        }
        Value::Object(object) => {
            let uid = object.get("uid").and_then(Value::as_str);
            let name = object.get("name").and_then(Value::as_str);
            let datasource_type = object.get("type").and_then(Value::as_str);
            let has_placeholder =
                uid.is_some_and(is_placeholder_string) || name.is_some_and(is_placeholder_string);
            if has_placeholder {
                return None;
            }
            if let Some(datasource) = lookup_datasource(datasource_catalog, uid, name) {
                let resolved_name = string_field(
                    &datasource,
                    "name",
                    uid.or(name)
                        .unwrap_or_else(|| datasource_type.unwrap_or("")),
                );
                if !resolved_name.is_empty() {
                    return Some(resolved_name);
                }
            }
            name.map(str::to_string)
                .or_else(|| uid.map(str::to_string))
                .or_else(|| {
                    datasource_type.and_then(|value| {
                        lookup_unique_datasource_name_by_type(&datasource_catalog.by_uid, value)
                            .or_else(|| Some(datasource_type_alias(value).to_string()))
                    })
                })
        }
        _ => None,
    }
}

fn resolve_datasource_source_uid(
    reference: &Value,
    datasource_catalog: &super::prompt::DatasourceCatalog,
) -> Option<String> {
    if reference.is_null() || is_builtin_datasource_ref(reference) {
        return None;
    }
    match reference {
        Value::String(text) => {
            if is_placeholder_string(text) {
                return None;
            }
            lookup_datasource(datasource_catalog, Some(text), Some(text))
                .map(|datasource| string_field(&datasource, "uid", ""))
                .filter(|uid| !uid.is_empty())
        }
        Value::Object(object) => {
            let uid = object.get("uid").and_then(Value::as_str);
            let name = object.get("name").and_then(Value::as_str);
            let has_placeholder =
                uid.is_some_and(is_placeholder_string) || name.is_some_and(is_placeholder_string);
            if has_placeholder {
                return None;
            }
            if let Some(datasource) = lookup_datasource(datasource_catalog, uid, name) {
                let resolved_uid = string_field(&datasource, "uid", "");
                if !resolved_uid.is_empty() {
                    return Some(resolved_uid);
                }
            }
            uid.filter(|value| !value.is_empty()).map(str::to_string)
        }
        _ => None,
    }
}

/// collect dashboard source metadata.
pub(crate) fn collect_dashboard_source_metadata(
    payload: &Value,
    datasource_catalog: &super::prompt::DatasourceCatalog,
) -> Result<(Vec<String>, Vec<String>)> {
    let payload_object = value_as_object(payload, "Unexpected dashboard payload from Grafana.")?;
    let dashboard_object = extract_dashboard_object(payload_object)?;
    let mut refs = Vec::new();
    super::collect_datasource_refs(&Value::Object(dashboard_object.clone()), &mut refs);
    let mut names = BTreeSet::new();
    let mut uids = BTreeSet::new();
    for reference in refs {
        if let Some(name) = resolve_datasource_source_name(&reference, datasource_catalog) {
            names.insert(name);
        }
        if let Some(uid) = resolve_datasource_source_uid(&reference, datasource_catalog) {
            uids.insert(uid);
        }
    }
    Ok((names.into_iter().collect(), uids.into_iter().collect()))
}

fn dashboard_list_needs_sources(args: &ListArgs) -> bool {
    args.with_sources
        || args.text
        || args.json
        || args.yaml
        || args
            .output_columns
            .iter()
            .any(|column| matches!(column.as_str(), "sources" | "source_uids"))
}

fn attach_dashboard_sources_with_request<F>(
    mut request_json: F,
    summaries: &[Map<String, Value>],
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let datasource_catalog =
        build_datasource_catalog(&list_datasources_with_request(&mut request_json)?);
    summaries
        .iter()
        .map(|summary| {
            let uid = string_field(summary, "uid", "");
            let mut item = summary.clone();
            if uid.is_empty() {
                item.insert("sources".to_string(), Value::Array(Vec::new()));
                item.insert("sourceUids".to_string(), Value::Array(Vec::new()));
                return Ok(item);
            }
            let payload = fetch_dashboard_with_request(&mut request_json, &uid)?;
            let (sources, source_uids) =
                collect_dashboard_source_metadata(&payload, &datasource_catalog)?;
            item.insert(
                "sources".to_string(),
                Value::Array(sources.into_iter().map(Value::String).collect()),
            );
            item.insert(
                "sourceUids".to_string(),
                Value::Array(source_uids.into_iter().map(Value::String).collect()),
            );
            Ok(item)
        })
        .collect()
}

pub(crate) fn collect_list_dashboards_with_request<F>(
    request_json: &mut F,
    args: &ListArgs,
    org: Option<&Map<String, Value>>,
    org_id_override: Option<i64>,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut scoped_request = |method: Method,
                              path: &str,
                              params: &[(String, String)],
                              payload: Option<&Value>|
     -> Result<Option<Value>> {
        let mut scoped_params = params.to_vec();
        if let Some(org_id) = org_id_override {
            scoped_params.push(("orgId".to_string(), org_id.to_string()));
        }
        request_json(method, path, &scoped_params, payload)
    };

    let dashboard_summaries =
        list_dashboard_summaries_with_request(&mut scoped_request, args.page_size)?;
    let current_org = match org {
        Some(org) => org.clone(),
        None => fetch_current_org_with_request(&mut scoped_request)?,
    };
    let summaries =
        attach_dashboard_folder_paths_with_request(&mut scoped_request, &dashboard_summaries)?;
    let summaries = attach_dashboard_org_metadata(&summaries, &current_org);
    let summaries = if dashboard_list_needs_sources(args) && !summaries.is_empty() {
        attach_dashboard_sources_with_request(&mut scoped_request, &summaries)?
    } else {
        summaries
    };
    Ok(summaries)
}

fn render_dashboard_list_output(
    summaries: &[Map<String, Value>],
    args: &ListArgs,
) -> Result<usize> {
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render_dashboard_summary_json(
                summaries,
                &args.output_columns,
            ))?
        );
    } else if args.yaml {
        print!(
            "{}",
            render_yaml(&render_dashboard_summary_json(
                summaries,
                &args.output_columns
            ))?
        );
    } else if args.csv {
        for line in render_dashboard_summary_csv(summaries, &args.output_columns) {
            println!("{line}");
        }
    } else if args.text {
        for line in render_dashboard_summary_text(summaries) {
            println!("{line}");
        }
    } else {
        for line in render_dashboard_summary_table(summaries, &args.output_columns, !args.no_header)
        {
            println!("{line}");
        }
    }
    if !args.csv && !args.json && !args.yaml {
        println!();
        println!("Listed {} dashboard(s).", summaries.len());
    }
    Ok(summaries.len())
}

/// Purpose: implementation note.
pub(crate) fn list_dashboards_with_request<F>(mut request_json: F, args: &ListArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut summaries = Vec::new();
    if args.all_orgs {
        for org in list_orgs_with_request(&mut request_json)? {
            let org_id = org_id_value(&org)?;
            let mut scoped = collect_list_dashboards_with_request(
                &mut request_json,
                args,
                Some(&org),
                Some(org_id),
            )?;
            summaries.append(&mut scoped);
        }
    } else {
        summaries =
            collect_list_dashboards_with_request(&mut request_json, args, None, args.org_id)?;
    }
    render_dashboard_list_output(&summaries, args)
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn list_dashboards_with_client(client: &JsonHttpClient, args: &ListArgs) -> Result<usize> {
    list_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

/// Purpose: implementation note.
pub(crate) fn list_dashboards_with_org_clients(args: &ListArgs) -> Result<usize> {
    let admin_client = build_http_client(&args.common)?;
    let orgs = if args.all_orgs {
        list_orgs_with_request(|method, path, params, payload| {
            admin_client.request_json(method, path, params, payload)
        })?
    } else {
        Vec::new()
    };
    let mut summaries = Vec::new();
    if args.all_orgs {
        for org in orgs {
            let org_id = org_id_value(&org)?;
            let org_client = build_http_client_for_org(&args.common, org_id)?;
            let mut scoped = collect_list_dashboards_with_request(
                &mut |method, path, params, payload| {
                    org_client.request_json(method, path, params, payload)
                },
                args,
                Some(&org),
                None,
            )?;
            summaries.append(&mut scoped);
        }
    } else if let Some(org_id) = args.org_id {
        let org_client = build_http_client_for_org(&args.common, org_id)?;
        summaries = collect_list_dashboards_with_request(
            &mut |method, path, params, payload| {
                org_client.request_json(method, path, params, payload)
            },
            args,
            None,
            None,
        )?;
    } else {
        let client = build_http_client(&args.common)?;
        summaries = collect_list_dashboards_with_request(
            &mut |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
            None,
            None,
        )?;
    }
    render_dashboard_list_output(&summaries, args)
}
