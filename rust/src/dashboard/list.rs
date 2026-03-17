//! Read model for dashboard and datasource listing.
//! This module translates API responses into stable CLI summary rows and output formats.
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::common::{message, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;

use super::{
    build_datasource_catalog, build_folder_path, build_http_client, build_http_client_for_org,
    datasource_type_alias, fetch_dashboard_with_request, fetch_folder_if_exists_with_request,
    is_builtin_datasource_ref, is_placeholder_string, list_dashboard_summaries_with_request,
    list_datasources_with_request, lookup_datasource, resolve_datasource_type_alias, ListArgs,
    ListDataSourcesArgs, DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID,
    DEFAULT_UNKNOWN_UID,
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

fn dashboard_org_id_cell(summary: &Map<String, Value>) -> Option<String> {
    summary.get("orgId").and_then(|value| match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) => Some(text.clone()),
        _ => None,
    })
}

/// org id value.
pub(crate) fn org_id_value(org: &Map<String, Value>) -> Result<i64> {
    org.get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Grafana org payload did not include a usable id."))
}

/// format dashboard summary line.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn format_dashboard_summary_line(summary: &Map<String, Value>) -> String {
    let uid = string_field(summary, "uid", DEFAULT_UNKNOWN_UID);
    let folder_title = string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE);
    let folder_uid = string_field(summary, "folderUid", DEFAULT_FOLDER_UID);
    let folder_path = string_field(summary, "folderPath", &folder_title);
    let title = string_field(summary, "title", DEFAULT_DASHBOARD_TITLE);
    let mut line = format!(
        "uid={uid} name={title} folder={folder_title} folderUid={folder_uid} path={folder_path}"
    );
    if summary.contains_key("orgName") || summary.contains_key("orgId") {
        let org_name = string_field(summary, "orgName", "");
        let org_id = dashboard_org_id_cell(summary).unwrap_or_default();
        let _ = write!(&mut line, " org={org_name} orgId={org_id}");
    }
    if let Some(sources) = dashboard_sources_cell(summary) {
        let _ = write!(&mut line, " sources={sources}");
    }
    line
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashboardListColumn {
    Uid,
    Name,
    Folder,
    FolderUid,
    Path,
    Org,
    OrgId,
    Sources,
    SourceUids,
}

impl DashboardListColumn {
    fn header(self) -> &'static str {
        match self {
            DashboardListColumn::Uid => "UID",
            DashboardListColumn::Name => "NAME",
            DashboardListColumn::Folder => "FOLDER",
            DashboardListColumn::FolderUid => "FOLDER_UID",
            DashboardListColumn::Path => "FOLDER_PATH",
            DashboardListColumn::Org => "ORG",
            DashboardListColumn::OrgId => "ORG_ID",
            DashboardListColumn::Sources => "SOURCES",
            DashboardListColumn::SourceUids => "SOURCE_UIDS",
        }
    }

    fn csv_key(self) -> &'static str {
        match self {
            DashboardListColumn::Uid => "uid",
            DashboardListColumn::Name => "name",
            DashboardListColumn::Folder => "folder",
            DashboardListColumn::FolderUid => "folderUid",
            DashboardListColumn::Path => "path",
            DashboardListColumn::Org => "org",
            DashboardListColumn::OrgId => "orgId",
            DashboardListColumn::Sources => "sources",
            DashboardListColumn::SourceUids => "sourceUids",
        }
    }
}

fn parse_dashboard_list_column(column: &str) -> Option<DashboardListColumn> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: dashboard_list.rs:resolve_dashboard_list_columns
    // Downstream callees: 無

    match column {
        "uid" => Some(DashboardListColumn::Uid),
        "name" => Some(DashboardListColumn::Name),
        "folder" => Some(DashboardListColumn::Folder),
        "folder_uid" => Some(DashboardListColumn::FolderUid),
        "path" => Some(DashboardListColumn::Path),
        "org" => Some(DashboardListColumn::Org),
        "org_id" => Some(DashboardListColumn::OrgId),
        "sources" => Some(DashboardListColumn::Sources),
        "source_uids" => Some(DashboardListColumn::SourceUids),
        _ => None,
    }
}

fn resolve_dashboard_list_columns(
    summaries: &[Map<String, Value>],
    output_columns: &[String],
) -> Vec<DashboardListColumn> {
    if !output_columns.is_empty() {
        return output_columns
            .iter()
            .filter_map(|column| parse_dashboard_list_column(column))
            .collect();
    }

    let mut columns = vec![
        DashboardListColumn::Uid,
        DashboardListColumn::Name,
        DashboardListColumn::Folder,
        DashboardListColumn::FolderUid,
        DashboardListColumn::Path,
    ];
    if summaries_include_org_metadata(summaries) {
        columns.push(DashboardListColumn::Org);
        columns.push(DashboardListColumn::OrgId);
    }
    if summaries_include_sources(summaries) {
        columns.push(DashboardListColumn::Sources);
    }
    if summaries_include_source_uids(summaries) {
        columns.push(DashboardListColumn::SourceUids);
    }
    columns
}

fn dashboard_list_value(summary: &Map<String, Value>, column: DashboardListColumn) -> String {
    match column {
        DashboardListColumn::Uid => string_field(summary, "uid", DEFAULT_UNKNOWN_UID),
        DashboardListColumn::Name => string_field(summary, "title", DEFAULT_DASHBOARD_TITLE),
        DashboardListColumn::Folder => string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
        DashboardListColumn::FolderUid => string_field(summary, "folderUid", DEFAULT_FOLDER_UID),
        DashboardListColumn::Path => string_field(
            summary,
            "folderPath",
            &string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE),
        ),
        DashboardListColumn::Org => string_field(summary, "orgName", ""),
        DashboardListColumn::OrgId => dashboard_org_id_cell(summary).unwrap_or_default(),
        DashboardListColumn::Sources => dashboard_sources_cell(summary).unwrap_or_default(),
        DashboardListColumn::SourceUids => {
            dashboard_source_uids(summary).unwrap_or_default().join(",")
        }
    }
}

fn build_dashboard_summary_row_for_columns(
    summary: &Map<String, Value>,
    columns: &[DashboardListColumn],
) -> Vec<String> {
    columns
        .iter()
        .map(|column| dashboard_list_value(summary, *column))
        .collect()
}

fn dashboard_list_needs_sources(args: &ListArgs) -> bool {
    args.with_sources
        || args.json
        || args
            .output_columns
            .iter()
            .any(|column| matches!(column.as_str(), "sources" | "source_uids"))
}

fn dashboard_sources(summary: &Map<String, Value>) -> Option<Vec<String>> {
    let values = summary.get("sources")?.as_array()?;
    Some(
        values
            .iter()
            .filter_map(Value::as_str)
            .map(|value| value.to_string())
            .collect(),
    )
}

fn dashboard_source_uids(summary: &Map<String, Value>) -> Option<Vec<String>> {
    let values = summary.get("sourceUids")?.as_array()?;
    Some(
        values
            .iter()
            .filter_map(Value::as_str)
            .map(|value| value.to_string())
            .collect(),
    )
}

fn dashboard_sources_cell(summary: &Map<String, Value>) -> Option<String> {
    let values = dashboard_sources(summary)?;
    if values.is_empty() {
        None
    } else {
        Some(values.join(","))
    }
}

fn summaries_include_sources(summaries: &[Map<String, Value>]) -> bool {
    summaries
        .iter()
        .any(|summary| summary.contains_key("sources"))
}

fn summaries_include_org_metadata(summaries: &[Map<String, Value>]) -> bool {
    summaries
        .iter()
        .any(|summary| summary.contains_key("orgName") || summary.contains_key("orgId"))
}

fn summaries_include_source_uids(summaries: &[Map<String, Value>]) -> bool {
    summaries
        .iter()
        .any(|summary| summary.contains_key("sourceUids"))
}

/// Purpose: implementation note.
pub(crate) fn render_dashboard_summary_table(
    summaries: &[Map<String, Value>],
    output_columns: &[String],
    include_header: bool,
) -> Vec<String> {
    let columns = resolve_dashboard_list_columns(summaries, output_columns);
    let headers: Vec<String> = columns
        .iter()
        .map(|column| column.header().to_string())
        .collect();
    let rows: Vec<Vec<String>> = summaries
        .iter()
        .map(|summary| build_dashboard_summary_row_for_columns(summary, &columns))
        .collect();
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }

    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };

    let separator: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    let mut lines = Vec::new();
    if include_header {
        lines.extend([format_row(&headers), format_row(&separator)]);
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

/// Purpose: implementation note.
pub(crate) fn render_dashboard_summary_csv(
    summaries: &[Map<String, Value>],
    output_columns: &[String],
) -> Vec<String> {
    let columns = resolve_dashboard_list_columns(summaries, output_columns);
    let header: Vec<String> = columns
        .iter()
        .map(|column| column.csv_key().to_string())
        .collect();
    let mut lines = vec![header.join(",")];
    lines.extend(summaries.iter().map(|summary| {
        let row = build_dashboard_summary_row_for_columns(summary, &columns);
        row.into_iter()
            .map(|value| {
                if value.contains(',') || value.contains('"') || value.contains('\n') {
                    format!("\"{}\"", value.replace('"', "\"\""))
                } else {
                    value
                }
            })
            .collect::<Vec<String>>()
            .join(",")
    }));
    lines
}

/// Purpose: implementation note.
pub(crate) fn render_dashboard_summary_json(
    summaries: &[Map<String, Value>],
    output_columns: &[String],
) -> Value {
    let columns = resolve_dashboard_list_columns(summaries, output_columns);
    Value::Array(
        summaries
            .iter()
            .map(|summary| {
                let mut object = Map::new();
                for column in &columns {
                    match column {
                        DashboardListColumn::Sources => {
                            object.insert(
                                column.csv_key().to_string(),
                                Value::Array(
                                    dashboard_sources(summary)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(Value::String)
                                        .collect(),
                                ),
                            );
                        }
                        DashboardListColumn::SourceUids => {
                            object.insert(
                                column.csv_key().to_string(),
                                Value::Array(
                                    dashboard_source_uids(summary)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(Value::String)
                                        .collect(),
                                ),
                            );
                        }
                        _ => {
                            object.insert(
                                column.csv_key().to_string(),
                                Value::String(dashboard_list_value(summary, *column)),
                            );
                        }
                    }
                }
                Value::Object(object)
            })
            .collect(),
    )
}

fn build_data_source_record(datasource: &Map<String, Value>) -> Vec<String> {
    vec![
        string_field(datasource, "uid", ""),
        string_field(datasource, "name", ""),
        string_field(datasource, "type", ""),
        string_field(datasource, "url", ""),
        if datasource
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "true".to_string()
        } else {
            "false".to_string()
        },
    ]
}

/// format data source line.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn format_data_source_line(datasource: &Map<String, Value>) -> String {
    let row = build_data_source_record(datasource);
    format!(
        "uid={} name={} type={} url={} isDefault={}",
        row[0], row[1], row[2], row[3], row[4]
    )
}

/// Purpose: implementation note.
pub(crate) fn render_data_source_table(
    datasources: &[Map<String, Value>],
    include_header: bool,
) -> Vec<String> {
    let headers = vec![
        "UID".to_string(),
        "NAME".to_string(),
        "TYPE".to_string(),
        "URL".to_string(),
        "IS_DEFAULT".to_string(),
    ];
    let rows: Vec<Vec<String>> = datasources.iter().map(build_data_source_record).collect();
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let separator: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    let mut lines = Vec::new();
    if include_header {
        lines.extend([format_row(&headers), format_row(&separator)]);
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

/// Purpose: implementation note.
pub(crate) fn render_data_source_csv(datasources: &[Map<String, Value>]) -> Vec<String> {
    let mut lines = vec!["uid,name,type,url,isDefault".to_string()];
    lines.extend(datasources.iter().map(|datasource| {
        build_data_source_record(datasource)
            .into_iter()
            .map(|value| {
                if value.contains(',') || value.contains('"') || value.contains('\n') {
                    format!("\"{}\"", value.replace('"', "\"\""))
                } else {
                    value
                }
            })
            .collect::<Vec<String>>()
            .join(",")
    }));
    lines
}

/// Purpose: implementation note.
pub(crate) fn render_data_source_json(datasources: &[Map<String, Value>]) -> Value {
    Value::Array(
        datasources
            .iter()
            .map(|datasource| {
                let row = build_data_source_record(datasource);
                Value::Object(Map::from_iter(vec![
                    ("uid".to_string(), Value::String(row[0].clone())),
                    ("name".to_string(), Value::String(row[1].clone())),
                    ("type".to_string(), Value::String(row[2].clone())),
                    ("url".to_string(), Value::String(row[3].clone())),
                    ("isDefault".to_string(), Value::String(row[4].clone())),
                ]))
            })
            .collect(),
    )
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
    let dashboard_object = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Unexpected dashboard payload from Grafana."))?;
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

fn collect_list_dashboards_with_request<F>(
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
    } else if args.csv {
        for line in render_dashboard_summary_csv(summaries, &args.output_columns) {
            println!("{line}");
        }
    } else {
        for line in render_dashboard_summary_table(summaries, &args.output_columns, !args.no_header)
        {
            println!("{line}");
        }
    }
    if !args.csv && !args.json {
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

/// Purpose: implementation note.
pub(crate) fn list_data_sources_with_request<F>(
    mut request_json: F,
    args: &ListDataSourcesArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let datasources = list_datasources_with_request(&mut request_json)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render_data_source_json(&datasources))?
        );
    } else if args.csv {
        for line in render_data_source_csv(&datasources) {
            println!("{line}");
        }
    } else {
        for line in render_data_source_table(&datasources, !args.no_header) {
            println!("{line}");
        }
    }
    if !args.csv && !args.json {
        println!();
        println!("Listed {} data source(s).", datasources.len());
    }
    Ok(datasources.len())
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn list_data_sources_with_client(
    client: &JsonHttpClient,
    args: &ListDataSourcesArgs,
) -> Result<usize> {
    list_data_sources_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}
