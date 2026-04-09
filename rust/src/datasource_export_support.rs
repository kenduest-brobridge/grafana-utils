//! Shared helpers for datasource list/export/import orchestration.
//!
//! Responsibilities:
//! - Build typed records and export indexes from API payloads.
//! - Resolve target output directories and per-org export scopes.
//! - Serialize provisioning artifacts and metadata in supported output formats.

use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    message, requested_columns_include_all, sanitize_path_component, string_field, tool_version,
    write_json_file, Result,
};
use crate::dashboard::{
    build_auth_context, build_http_client, build_http_client_for_org, list_datasources,
    CommonCliArgs, DEFAULT_ORG_ID,
};
use crate::datasource::fetch_datasource_by_uid_if_exists;
use crate::datasource_secret::{
    build_inline_secret_placeholder_token, inline_secret_provider_contract,
    summarize_secret_provider_contract,
};
use crate::export_metadata::{
    build_export_metadata_common, export_metadata_common_map, EXPORT_BUNDLE_KIND_ROOT,
};
use crate::http::JsonHttpClient;

use super::{
    datasource_import_export_support::{
        DatasourceExportMetadata, DatasourceImportRecord, DATASOURCE_EXPORT_FILENAME,
        EXPORT_METADATA_FILENAME, ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION,
    },
    fetch_current_org,
};

pub(crate) const DATASOURCE_PROVISIONING_SUBDIR: &str = "provisioning";
pub(crate) const DATASOURCE_PROVISIONING_FILENAME: &str = "datasources.yaml";
const DATASOURCE_MASKED_RECOVERY_FORMAT: &str = "grafana-datasource-masked-recovery-v1";
const DATASOURCE_EXPORT_MODE: &str = "masked-recovery";
const DATASOURCE_SECRET_MATERIAL_MODE: &str = "placeholders-only";
const DATASOURCE_PROVISIONING_PROJECTION_MODE: &str = "derived-projection";

#[derive(Serialize)]
pub(crate) struct ProvisioningDatasource {
    name: String,
    #[serde(rename = "type")]
    datasource_type: String,
    access: String,
    #[serde(rename = "orgId")]
    org_id: i64,
    uid: String,
    url: String,
    #[serde(rename = "basicAuth", skip_serializing_if = "Option::is_none")]
    basic_auth: Option<bool>,
    #[serde(rename = "basicAuthUser", skip_serializing_if = "Option::is_none")]
    basic_auth_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    #[serde(rename = "withCredentials", skip_serializing_if = "Option::is_none")]
    with_credentials: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    database: Option<String>,
    #[serde(rename = "jsonData", skip_serializing_if = "Option::is_none")]
    json_data: Option<Map<String, Value>>,
    #[serde(
        rename = "secureJsonDataPlaceholders",
        skip_serializing_if = "Option::is_none"
    )]
    secure_json_data_placeholders: Option<Map<String, Value>>,
    #[serde(rename = "isDefault")]
    is_default: bool,
    editable: bool,
}

#[derive(Serialize)]
pub(crate) struct ProvisioningDocument {
    #[serde(rename = "apiVersion")]
    api_version: i64,
    datasources: Vec<ProvisioningDatasource>,
}

pub(crate) fn build_all_orgs_output_dir(output_dir: &Path, org: &Map<String, Value>) -> PathBuf {
    let org_id = org
        .get("id")
        .map(|value| sanitize_path_component(&value.to_string()))
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let org_name = sanitize_path_component(&string_field(org, "name", "org"));
    output_dir.join(format!("org_{org_id}_{org_name}"))
}

pub(crate) fn resolve_target_client(
    common: &CommonCliArgs,
    org_id: Option<i64>,
) -> Result<JsonHttpClient> {
    if let Some(org_id) = org_id {
        let context = build_auth_context(common)?;
        if context.auth_mode != "basic" {
            return Err(message(
                "Datasource org switching requires Basic auth (--basic-user / --basic-password).",
            ));
        }
        build_http_client_for_org(common, org_id)
    } else {
        build_http_client(common)
    }
}

pub(crate) fn validate_import_org_auth(
    common: &CommonCliArgs,
    args: &super::DatasourceImportArgs,
) -> Result<()> {
    let context = build_auth_context(common)?;
    if (args.org_id.is_some() || args.use_export_org) && context.auth_mode != "basic" {
        return Err(message(if args.use_export_org {
            "Datasource import with --use-export-org requires Basic auth (--basic-user / --basic-password)."
        } else {
            "Datasource import with --org-id requires Basic auth (--basic-user / --basic-password)."
        }));
    }
    Ok(())
}

pub(crate) fn describe_datasource_import_mode(
    replace_existing: bool,
    update_existing_only: bool,
) -> &'static str {
    if update_existing_only {
        "update-or-skip-missing"
    } else if replace_existing {
        "create-or-update"
    } else {
        "create-only"
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_datasource_export_metadata(
    source_url: &str,
    source_profile: Option<&str>,
    org_scope: Option<&str>,
    org_id: Option<&str>,
    org_name: Option<&str>,
    artifact_path: &Path,
    count: usize,
) -> Value {
    let common = build_export_metadata_common(
        "datasource",
        "datasources",
        EXPORT_BUNDLE_KIND_ROOT,
        "live",
        Some(source_url),
        None,
        source_profile,
        org_scope,
        org_id,
        org_name,
        artifact_path,
        &artifact_path.join(EXPORT_METADATA_FILENAME),
        count,
    );
    let mut metadata = Map::from_iter(vec![
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "toolVersion".to_string(),
            Value::String(tool_version().to_string()),
        ),
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        ("variant".to_string(), Value::String("root".to_string())),
        (
            "scopeKind".to_string(),
            Value::String("org-root".to_string()),
        ),
        (
            "resource".to_string(),
            Value::String("datasource".to_string()),
        ),
        (
            "datasourceCount".to_string(),
            Value::Number((count as i64).into()),
        ),
        (
            "datasourcesFile".to_string(),
            Value::String(DATASOURCE_EXPORT_FILENAME.to_string()),
        ),
        (
            "indexFile".to_string(),
            Value::String("index.json".to_string()),
        ),
        (
            "format".to_string(),
            Value::String(DATASOURCE_MASKED_RECOVERY_FORMAT.to_string()),
        ),
        (
            "exportMode".to_string(),
            Value::String(DATASOURCE_EXPORT_MODE.to_string()),
        ),
        ("masked".to_string(), Value::Bool(true)),
        ("recoveryCapable".to_string(), Value::Bool(true)),
        (
            "secretMaterial".to_string(),
            Value::String(DATASOURCE_SECRET_MATERIAL_MODE.to_string()),
        ),
        (
            "secretPlaceholderProvider".to_string(),
            summarize_secret_provider_contract(&inline_secret_provider_contract()),
        ),
        (
            "provisioningProjection".to_string(),
            Value::String(DATASOURCE_PROVISIONING_PROJECTION_MODE.to_string()),
        ),
        (
            "provisioningFile".to_string(),
            Value::String(
                Path::new(DATASOURCE_PROVISIONING_SUBDIR)
                    .join(DATASOURCE_PROVISIONING_FILENAME)
                    .display()
                    .to_string(),
            ),
        ),
    ]);
    metadata.extend(export_metadata_common_map(&common));
    Value::Object(metadata)
}

fn data_source_rows_include_org_scope(datasources: &[Map<String, Value>]) -> bool {
    datasources
        .iter()
        .map(DatasourceImportRecord::from_generic_map)
        .any(|record| !record.org_name.is_empty() || !record.org_id.is_empty())
}

const DATASOURCE_LIST_DEFAULT_COLUMNS: [&str; 5] = ["uid", "name", "type", "url", "is_default"];
const DATASOURCE_LIST_ORG_COLUMNS: [&str; 2] = ["org", "org_id"];
const DATASOURCE_LIST_DISCOVERABLE_COLUMNS: [&str; 14] = [
    "uid",
    "name",
    "type",
    "access",
    "url",
    "is_default",
    "basicAuth",
    "basicAuthUser",
    "database",
    "user",
    "withCredentials",
    "org",
    "org_id",
    "jsonData.<key>",
];

pub(crate) fn datasource_list_column_ids() -> &'static [&'static str] {
    &DATASOURCE_LIST_DISCOVERABLE_COLUMNS
}

fn normalize_datasource_column_id(id: &str) -> String {
    match id {
        "isDefault" => "is_default".to_string(),
        "orgId" => "org_id".to_string(),
        other => other.to_string(),
    }
}

fn datasource_record_path_segments(column: &str) -> Vec<String> {
    column
        .split('.')
        .map(|segment| match segment {
            "is_default" => "isDefault".to_string(),
            "org_id" => "orgId".to_string(),
            other => other.to_string(),
        })
        .collect()
}

fn datasource_column_header(column: &str) -> String {
    column.to_ascii_uppercase()
}

fn datasource_json_scalar_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn lookup_datasource_column_value<'a>(
    datasource: &'a Map<String, Value>,
    column: &str,
) -> Option<&'a Value> {
    let path = datasource_record_path_segments(column);
    let mut current = datasource.get(path.first()?)?;
    for segment in path.iter().skip(1) {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

fn lookup_datasource_column_text(datasource: &Map<String, Value>, column: &str) -> String {
    lookup_datasource_column_value(datasource, column)
        .map(datasource_json_scalar_text)
        .unwrap_or_default()
}

fn insert_projected_datasource_value(
    target: &mut Map<String, Value>,
    path: &[String],
    value: Value,
) {
    if path.is_empty() {
        return;
    }
    if path.len() == 1 {
        target.insert(path[0].clone(), value);
        return;
    }
    let entry = target
        .entry(path[0].clone())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    if let Some(object) = entry.as_object_mut() {
        insert_projected_datasource_value(object, &path[1..], value);
    }
}

fn project_datasource_record(
    datasource: &Map<String, Value>,
    selected_columns: &[String],
) -> Map<String, Value> {
    let mut projected = Map::new();
    for column in selected_columns {
        let path = datasource_record_path_segments(column);
        if let Some(value) = lookup_datasource_column_value(datasource, column).cloned() {
            insert_projected_datasource_value(&mut projected, &path, value);
        }
    }
    projected
}

fn collect_datasource_leaf_columns_from_value(
    prefix: &str,
    value: &Value,
    columns: &mut BTreeSet<String>,
) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let normalized_key = normalize_datasource_column_id(key);
                let child_prefix = if prefix.is_empty() {
                    normalized_key
                } else {
                    format!("{prefix}.{normalized_key}")
                };
                collect_datasource_leaf_columns_from_value(&child_prefix, child, columns);
            }
        }
        _ => {
            if !prefix.is_empty() {
                columns.insert(prefix.to_string());
            }
        }
    }
}

fn discover_all_datasource_columns(
    datasources: &[Map<String, Value>],
    include_org_scope: bool,
) -> Vec<String> {
    let mut discovered = BTreeSet::new();
    for datasource in datasources {
        for (key, value) in datasource {
            let normalized_key = normalize_datasource_column_id(key);
            collect_datasource_leaf_columns_from_value(&normalized_key, value, &mut discovered);
        }
    }
    let mut columns = DATASOURCE_LIST_DEFAULT_COLUMNS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<String>>();
    if include_org_scope {
        columns.extend(
            DATASOURCE_LIST_ORG_COLUMNS
                .iter()
                .map(|value| value.to_string()),
        );
    }
    for column in discovered {
        if !columns.iter().any(|item| item == &column) {
            columns.push(column);
        }
    }
    columns
}

fn resolve_datasource_list_columns(
    datasources: &[Map<String, Value>],
    include_org_scope: bool,
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    match selected_columns {
        None => {
            let mut columns = DATASOURCE_LIST_DEFAULT_COLUMNS
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<String>>();
            if include_org_scope {
                columns.extend(
                    DATASOURCE_LIST_ORG_COLUMNS
                        .iter()
                        .map(|value| value.to_string()),
                );
            }
            columns
        }
        Some(selected) if requested_columns_include_all(selected) => {
            discover_all_datasource_columns(datasources, include_org_scope)
        }
        Some(selected) => selected.to_vec(),
    }
}

pub(crate) fn render_data_source_summary_line(
    datasource: &Map<String, Value>,
    selected_columns: Option<&[String]>,
) -> String {
    let include_org_scope = !string_field(datasource, "org", "").is_empty()
        || !string_field(datasource, "orgId", "").is_empty();
    let columns = resolve_datasource_list_columns(
        std::slice::from_ref(datasource),
        include_org_scope,
        selected_columns,
    );
    let values = columns
        .iter()
        .filter_map(|column| {
            let value = lookup_datasource_column_text(datasource, column);
            if value.is_empty() {
                None
            } else {
                Some(format!("{column}={value}"))
            }
        })
        .collect::<Vec<String>>();
    format!("- {}", values.join(" "))
}

fn placeholder_identity(datasource: &Map<String, Value>) -> String {
    let uid = string_field(datasource, "uid", "");
    if !uid.is_empty() {
        return uid;
    }
    let name = string_field(datasource, "name", "");
    if !name.is_empty() {
        return name;
    }
    let datasource_type = string_field(datasource, "type", "");
    if !datasource_type.is_empty() {
        return datasource_type;
    }
    "datasource".to_string()
}

fn build_secure_json_data_placeholders(
    datasource: &Map<String, Value>,
) -> Option<Map<String, Value>> {
    let secure_json_fields = datasource
        .get("secureJsonFields")
        .and_then(Value::as_object)?;
    let mut field_names = secure_json_fields
        .iter()
        .filter_map(|(field_name, value)| {
            value
                .as_bool()
                .filter(|enabled| *enabled)
                .map(|_| field_name)
        })
        .cloned()
        .collect::<Vec<String>>();
    field_names.sort();
    if field_names.is_empty() {
        return None;
    }
    Some(Map::from_iter(field_names.into_iter().map(|field_name| {
        (
            field_name.clone(),
            Value::String(build_inline_secret_placeholder_token(
                &placeholder_identity(datasource),
                &field_name,
            )),
        )
    })))
}

fn build_export_record_from_datasource(
    datasource: &Map<String, Value>,
    org_name: &str,
    org_id: &str,
) -> DatasourceImportRecord {
    let mut record = DatasourceImportRecord::from_generic_map(datasource);
    record.org_name = org_name.to_string();
    record.org_id = org_id.to_string();
    if let Some(placeholders) = build_secure_json_data_placeholders(datasource) {
        record.secure_json_data_placeholders = Some(placeholders);
    }
    record
}

pub(crate) fn render_data_source_table(
    datasources: &[Map<String, Value>],
    include_header: bool,
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    let include_org_scope = data_source_rows_include_org_scope(datasources);
    let columns = resolve_datasource_list_columns(datasources, include_org_scope, selected_columns);
    let headers = columns
        .iter()
        .map(|column| datasource_column_header(column))
        .collect::<Vec<String>>();
    let rows: Vec<Vec<String>> = datasources
        .iter()
        .map(|datasource| {
            columns
                .iter()
                .map(|column| lookup_datasource_column_text(datasource, column))
                .collect::<Vec<String>>()
        })
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

pub(crate) fn render_data_source_csv(
    datasources: &[Map<String, Value>],
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    let include_org_scope = data_source_rows_include_org_scope(datasources);
    let columns = resolve_datasource_list_columns(datasources, include_org_scope, selected_columns);
    let mut lines = vec![columns
        .iter()
        .map(|column| match column.as_str() {
            "is_default" => "isDefault".to_string(),
            "org_id" => "orgId".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<String>>()
        .join(",")];
    lines.extend(datasources.iter().map(|datasource| {
        columns
            .iter()
            .map(|column| lookup_datasource_column_text(datasource, column))
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

pub(crate) fn render_data_source_json(
    datasources: &[Map<String, Value>],
    selected_columns: Option<&[String]>,
) -> Value {
    if selected_columns.is_none() || requested_columns_include_all(selected_columns.unwrap_or(&[]))
    {
        return Value::Array(
            datasources
                .iter()
                .cloned()
                .map(Value::Object)
                .collect::<Vec<Value>>(),
        );
    }
    let selected_columns = selected_columns.unwrap_or(&[]);
    Value::Array(
        datasources
            .iter()
            .map(|datasource| {
                Value::Object(project_datasource_record(datasource, selected_columns))
            })
            .collect(),
    )
}

pub(crate) fn build_list_records(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    let org = fetch_current_org(client)?;
    let org_name = string_field(&org, "name", "");
    let org_id = org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let datasources = list_datasources(client)?;
    Ok(datasources
        .into_iter()
        .map(|mut datasource| {
            datasource.insert("org".to_string(), Value::String(org_name.clone()));
            datasource.insert("orgId".to_string(), Value::String(org_id.clone()));
            datasource
        })
        .collect())
}

pub(crate) fn build_export_index(records: &[Map<String, Value>]) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "toolVersion".to_string(),
            Value::String(tool_version().to_string()),
        ),
        (
            "datasourcesFile".to_string(),
            Value::String(DATASOURCE_EXPORT_FILENAME.to_string()),
        ),
        (
            "primaryFile".to_string(),
            Value::String(DATASOURCE_EXPORT_FILENAME.to_string()),
        ),
        (
            "exportMode".to_string(),
            Value::String(DATASOURCE_EXPORT_MODE.to_string()),
        ),
        ("masked".to_string(), Value::Bool(true)),
        ("recoveryCapable".to_string(), Value::Bool(true)),
        (
            "secretMaterial".to_string(),
            Value::String(DATASOURCE_SECRET_MATERIAL_MODE.to_string()),
        ),
        (
            "variants".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "inventory".to_string(),
                    Value::String(DATASOURCE_EXPORT_FILENAME.to_string()),
                ),
                (
                    "provisioning".to_string(),
                    Value::String(
                        Path::new(DATASOURCE_PROVISIONING_SUBDIR)
                            .join(DATASOURCE_PROVISIONING_FILENAME)
                            .display()
                            .to_string(),
                    ),
                ),
            ])),
        ),
        (
            "count".to_string(),
            Value::Number((records.len() as i64).into()),
        ),
        (
            "items".to_string(),
            Value::Array(
                records
                    .iter()
                    .map(|record| {
                        Value::Object(Map::from_iter(vec![
                            (
                                "uid".to_string(),
                                Value::String(string_field(record, "uid", "")),
                            ),
                            (
                                "name".to_string(),
                                Value::String(string_field(record, "name", "")),
                            ),
                            (
                                "type".to_string(),
                                Value::String(string_field(record, "type", "")),
                            ),
                            (
                                "org".to_string(),
                                Value::String(string_field(record, "org", "")),
                            ),
                            (
                                "orgId".to_string(),
                                Value::String(string_field(record, "orgId", "")),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        ),
    ]))
}

pub(crate) fn build_all_orgs_export_index(items: &[Map<String, Value>]) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "toolVersion".to_string(),
            Value::String(tool_version().to_string()),
        ),
        (
            "exportMode".to_string(),
            Value::String(DATASOURCE_EXPORT_MODE.to_string()),
        ),
        ("masked".to_string(), Value::Bool(true)),
        ("recoveryCapable".to_string(), Value::Bool(true)),
        (
            "secretMaterial".to_string(),
            Value::String(DATASOURCE_SECRET_MATERIAL_MODE.to_string()),
        ),
        (
            "variant".to_string(),
            Value::String("all-orgs-root".to_string()),
        ),
        (
            "scopeKind".to_string(),
            Value::String("all-orgs-root".to_string()),
        ),
        (
            "variants".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "inventory".to_string(),
                    Value::String(DATASOURCE_EXPORT_FILENAME.to_string()),
                ),
                (
                    "provisioning".to_string(),
                    Value::String(
                        Path::new(DATASOURCE_PROVISIONING_SUBDIR)
                            .join(DATASOURCE_PROVISIONING_FILENAME)
                            .display()
                            .to_string(),
                    ),
                ),
            ])),
        ),
        (
            "count".to_string(),
            Value::Number((items.len() as i64).into()),
        ),
        (
            "items".to_string(),
            Value::Array(items.iter().cloned().map(Value::Object).collect()),
        ),
    ]))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_all_orgs_export_metadata(
    source_url: &str,
    source_profile: Option<&str>,
    artifact_path: &Path,
    org_count: usize,
    datasource_count: usize,
) -> Value {
    let common = build_export_metadata_common(
        "datasource",
        "datasources",
        EXPORT_BUNDLE_KIND_ROOT,
        "live",
        Some(source_url),
        None,
        source_profile,
        Some("all-orgs"),
        None,
        None,
        artifact_path,
        &artifact_path.join(EXPORT_METADATA_FILENAME),
        org_count,
    );
    let mut metadata = Map::from_iter(vec![
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "toolVersion".to_string(),
            Value::String(tool_version().to_string()),
        ),
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        (
            "variant".to_string(),
            Value::String("all-orgs-root".to_string()),
        ),
        (
            "scopeKind".to_string(),
            Value::String("all-orgs-root".to_string()),
        ),
        (
            "resource".to_string(),
            Value::String("datasource".to_string()),
        ),
        (
            "orgCount".to_string(),
            Value::Number((org_count as i64).into()),
        ),
        (
            "datasourceCount".to_string(),
            Value::Number((datasource_count as i64).into()),
        ),
        (
            "indexFile".to_string(),
            Value::String("index.json".to_string()),
        ),
        (
            "format".to_string(),
            Value::String(DATASOURCE_MASKED_RECOVERY_FORMAT.to_string()),
        ),
        (
            "exportMode".to_string(),
            Value::String(DATASOURCE_EXPORT_MODE.to_string()),
        ),
        ("masked".to_string(), Value::Bool(true)),
        ("recoveryCapable".to_string(), Value::Bool(true)),
        (
            "secretMaterial".to_string(),
            Value::String(DATASOURCE_SECRET_MATERIAL_MODE.to_string()),
        ),
        (
            "secretPlaceholderProvider".to_string(),
            summarize_secret_provider_contract(&inline_secret_provider_contract()),
        ),
        (
            "provisioningProjection".to_string(),
            Value::String(DATASOURCE_PROVISIONING_PROJECTION_MODE.to_string()),
        ),
        (
            "provisioningFile".to_string(),
            Value::String(
                Path::new(DATASOURCE_PROVISIONING_SUBDIR)
                    .join(DATASOURCE_PROVISIONING_FILENAME)
                    .display()
                    .to_string(),
            ),
        ),
    ]);
    metadata.extend(export_metadata_common_map(&common));
    Value::Object(metadata)
}

pub(crate) fn build_export_records(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    let org = fetch_current_org(client)?;
    let org_name = string_field(&org, "name", "");
    let org_id = org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let datasources = list_datasources(client)?;
    let mut records = Vec::with_capacity(datasources.len());
    for datasource in datasources {
        let resolved = datasource
            .get("uid")
            .and_then(Value::as_str)
            .filter(|uid| !uid.trim().is_empty())
            .map(|uid| fetch_datasource_by_uid_if_exists(client, uid))
            .transpose()?
            .flatten()
            .unwrap_or(datasource);
        records.push(
            build_export_record_from_datasource(&resolved, &org_name, &org_id)
                .to_inventory_record(),
        );
    }
    records.sort_by_key(|record| string_field(record, "uid", ""));
    Ok(records)
}

pub(crate) fn build_datasource_provisioning_document(
    records: &[Map<String, Value>],
) -> ProvisioningDocument {
    ProvisioningDocument {
        api_version: 1,
        datasources: records
            .iter()
            .map(|record| {
                let record = DatasourceImportRecord::from_generic_map(record);
                ProvisioningDatasource {
                    name: record.name,
                    datasource_type: record.datasource_type,
                    access: record.access,
                    org_id: if record.org_id.trim().is_empty() {
                        DEFAULT_ORG_ID.to_string()
                    } else {
                        record.org_id
                    }
                    .parse::<i64>()
                    .unwrap_or(1),
                    uid: record.uid,
                    url: record.url,
                    basic_auth: record.basic_auth,
                    basic_auth_user: (!record.basic_auth_user.is_empty())
                        .then_some(record.basic_auth_user),
                    user: (!record.user.is_empty()).then_some(record.user),
                    with_credentials: record.with_credentials,
                    database: (!record.database.is_empty()).then_some(record.database),
                    json_data: record.json_data,
                    secure_json_data_placeholders: record.secure_json_data_placeholders,
                    is_default: record.is_default,
                    editable: false,
                }
            })
            .collect(),
    }
}

pub(crate) fn write_yaml_file<T: Serialize>(
    output_path: &Path,
    payload: &T,
    overwrite: bool,
) -> Result<()> {
    if output_path.exists() && !overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}",
            output_path.display()
        )));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = serde_yaml::to_string(payload).map_err(|error| {
        message(format!(
            "Failed to serialize YAML document for {}: {error}",
            output_path.display()
        ))
    })?;
    fs::write(output_path, rendered)?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn export_datasource_scope(
    client: &JsonHttpClient,
    output_dir: &Path,
    overwrite: bool,
    dry_run: bool,
    write_provisioning: bool,
    source_url: &str,
    source_profile: Option<&str>,
) -> Result<usize> {
    let records = build_export_records(client)?;
    let datasources_path = output_dir.join(DATASOURCE_EXPORT_FILENAME);
    let index_path = output_dir.join("index.json");
    let metadata_path = output_dir.join(EXPORT_METADATA_FILENAME);
    let provisioning_path = output_dir
        .join(DATASOURCE_PROVISIONING_SUBDIR)
        .join(DATASOURCE_PROVISIONING_FILENAME);
    if !dry_run {
        write_json_file(
            &datasources_path,
            &Value::Array(records.clone().into_iter().map(Value::Object).collect()),
            overwrite,
        )?;
        write_json_file(&index_path, &build_export_index(&records), overwrite)?;
        write_json_file(
            &metadata_path,
            &build_datasource_export_metadata(
                source_url,
                source_profile,
                Some("org"),
                None,
                None,
                output_dir,
                records.len(),
            ),
            overwrite,
        )?;
        if write_provisioning {
            write_yaml_file(
                &provisioning_path,
                &build_datasource_provisioning_document(&records),
                overwrite,
            )?;
        }
    }
    let summary_verb = if dry_run { "Would export" } else { "Exported" };
    println!(
        "{summary_verb} {} datasource(s). Datasources: {} Index: {} Manifest: {}{}",
        records.len(),
        datasources_path.display(),
        index_path.display(),
        metadata_path.display(),
        if write_provisioning {
            format!(" Provisioning: {}", provisioning_path.display())
        } else {
            String::new()
        }
    );
    Ok(records.len())
}

pub(crate) fn parse_export_metadata(path: &Path) -> Result<DatasourceExportMetadata> {
    let value = crate::common::load_json_object_file(path, "Datasource export metadata")?;
    let object = value
        .as_object()
        .ok_or_else(|| message("Datasource export metadata must be a JSON object."))?;
    let schema_version = object
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Datasource export metadata is missing schemaVersion."))?;
    object
        .get("datasourceCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Datasource export metadata is missing datasourceCount."))?;
    Ok(DatasourceExportMetadata {
        schema_version,
        kind: string_field(object, "kind", ""),
        variant: string_field(object, "variant", ""),
        scope_kind: object
            .get("scopeKind")
            .and_then(Value::as_str)
            .map(str::to_string),
        resource: string_field(object, "resource", ""),
        datasources_file: string_field(object, "datasourcesFile", DATASOURCE_EXPORT_FILENAME),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_export_record_preserves_recovery_fields_and_masks_secure_json_data() {
        let datasource = json!({
            "uid": "loki-main",
            "name": "Loki Logs",
            "type": "loki",
            "access": "proxy",
            "url": "http://loki:3100",
            "isDefault": false,
            "database": "logs",
            "basicAuth": true,
            "basicAuthUser": "loki-user",
            "user": "query-user",
            "withCredentials": true,
            "jsonData": {
                "maxLines": 1000,
                "timeout": 60
            },
            "secureJsonData": {
                "basicAuthPassword": "super-secret"
            },
            "secureJsonFields": {
                "basicAuthPassword": true,
                "httpHeaderValue1": true,
                "unused": false
            }
        })
        .as_object()
        .unwrap()
        .clone();

        let record = build_export_record_from_datasource(&datasource, "Observability", "7");

        assert_eq!(record.database, "logs");
        assert_eq!(record.basic_auth, Some(true));
        assert_eq!(record.basic_auth_user, "loki-user");
        assert_eq!(record.user, "query-user");
        assert_eq!(record.with_credentials, Some(true));
        assert_eq!(
            record.json_data,
            Some(
                json!({"maxLines": 1000, "timeout": 60})
                    .as_object()
                    .unwrap()
                    .clone()
            )
        );
        assert_eq!(
            record.secure_json_data_placeholders,
            Some(
                json!({
                    "basicAuthPassword": "${secret:loki-main-basicauthpassword}",
                    "httpHeaderValue1": "${secret:loki-main-httpheadervalue1}"
                })
                .as_object()
                .unwrap()
                .clone()
            )
        );
    }

    #[test]
    fn build_datasource_provisioning_document_projects_expected_shape() {
        let records = vec![json!({
            "uid": "prom-main",
            "name": "Prometheus Main",
            "type": "prometheus",
            "access": "proxy",
            "url": "http://prometheus:9090",
            "isDefault": "true",
            "orgId": "7",
            "basicAuth": true,
            "basicAuthUser": "prom-user",
            "withCredentials": true,
            "jsonData": {
                "httpMethod": "POST",
                "timeInterval": "30s"
            },
            "secureJsonDataPlaceholders": {
                "httpHeaderValue1": "${secret:prom-main-httpheadervalue1}"
            }
        })
        .as_object()
        .unwrap()
        .clone()];

        let value = serde_json::to_value(build_datasource_provisioning_document(&records)).unwrap();

        assert_eq!(
            value,
            json!({
                "apiVersion": 1,
                "datasources": [{
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "orgId": 7,
                    "uid": "prom-main",
                    "url": "http://prometheus:9090",
                    "basicAuth": true,
                    "basicAuthUser": "prom-user",
                    "withCredentials": true,
                    "jsonData": {
                        "httpMethod": "POST",
                        "timeInterval": "30s"
                    },
                    "secureJsonDataPlaceholders": {
                        "httpHeaderValue1": "${secret:prom-main-httpheadervalue1}"
                    },
                    "isDefault": true,
                    "editable": false
                }]
            })
        );
    }

    #[test]
    fn build_export_index_includes_provisioning_variant_pointer() {
        let records = vec![json!({
            "uid": "prom-main",
            "name": "Prometheus Main",
            "type": "prometheus",
            "org": "Main Org",
            "orgId": "1"
        })
        .as_object()
        .unwrap()
        .clone()];

        let value = build_export_index(&records);

        assert_eq!(
            value["variants"]["inventory"],
            Value::String("datasources.json".to_string())
        );
        assert_eq!(
            value["variants"]["provisioning"],
            Value::String("provisioning/datasources.yaml".to_string())
        );
        assert_eq!(
            value["exportMode"],
            Value::String("masked-recovery".to_string())
        );
        assert_eq!(value["masked"], Value::Bool(true));
        assert_eq!(value["recoveryCapable"], Value::Bool(true));
    }

    #[test]
    fn build_export_metadata_marks_masked_recovery_contract() {
        let metadata = build_datasource_export_metadata(
            "http://127.0.0.1:3000",
            Some("dev"),
            Some("org"),
            Some("1"),
            Some("Main Org"),
            Path::new("/tmp/export"),
            2,
        );

        assert_eq!(
            metadata["format"],
            Value::String("grafana-datasource-masked-recovery-v1".to_string())
        );
        assert_eq!(
            metadata["exportMode"],
            Value::String("masked-recovery".to_string())
        );
        assert_eq!(metadata["masked"], Value::Bool(true));
        assert_eq!(metadata["recoveryCapable"], Value::Bool(true));
        assert_eq!(
            metadata["provisioningProjection"],
            Value::String("derived-projection".to_string())
        );
        assert_eq!(metadata["metadataVersion"], Value::Number(2.into()));
        assert_eq!(metadata["domain"], Value::String("datasource".to_string()));
        assert_eq!(
            metadata["resourceKind"],
            Value::String("datasources".to_string())
        );
        assert_eq!(
            metadata["bundleKind"],
            Value::String("export-root".to_string())
        );
        assert_eq!(
            metadata["source"]["kind"],
            Value::String("live".to_string())
        );
        assert_eq!(
            metadata["source"]["url"],
            Value::String("http://127.0.0.1:3000".to_string())
        );
        assert_eq!(metadata["capture"]["recordCount"], Value::Number(2.into()));
        assert_eq!(
            metadata["secretPlaceholderProvider"]["kind"],
            Value::String("inline-placeholder-map".to_string())
        );
        assert_eq!(
            metadata["secretPlaceholderProvider"]["inputFlag"],
            Value::String("--secret-values".to_string())
        );
    }

    #[test]
    fn build_all_orgs_export_index_marks_masked_recovery_contract() {
        let items = vec![json!({
            "uid": "prom-main",
            "name": "Prometheus Main",
            "type": "prometheus",
            "org": "Main Org",
            "orgId": "1",
            "exportDir": "/tmp/export/org_1_Main_Org"
        })
        .as_object()
        .unwrap()
        .clone()];

        let value = build_all_orgs_export_index(&items);

        assert_eq!(value["variant"], Value::String("all-orgs-root".to_string()));
        assert_eq!(
            value["exportMode"],
            Value::String("masked-recovery".to_string())
        );
        assert_eq!(value["masked"], Value::Bool(true));
        assert_eq!(value["recoveryCapable"], Value::Bool(true));
        assert_eq!(
            value["variants"]["inventory"],
            Value::String("datasources.json".to_string())
        );
    }
}
