use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

use crate::common::{message, sanitize_path_component, string_field, write_json_file, Result};
use crate::dashboard::{
    build_auth_context, build_http_client, build_http_client_for_org, list_datasources,
    CommonCliArgs, DEFAULT_ORG_ID,
};
use crate::http::JsonHttpClient;

use super::{
    datasource_import_export_support::{
        DATASOURCE_CONTRACT_FIELDS, DATASOURCE_EXPORT_FILENAME, EXPORT_METADATA_FILENAME,
        ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION,
    },
    fetch_current_org, DatasourceExportMetadata,
};

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

pub(crate) fn build_datasource_export_metadata(count: usize) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        ("variant".to_string(), Value::String("root".to_string())),
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
            Value::String("grafana-datasource-inventory-v1".to_string()),
        ),
    ]))
}

fn data_source_rows_include_org_scope(datasources: &[Map<String, Value>]) -> bool {
    datasources.iter().any(|datasource| {
        !string_field(datasource, "org", "").is_empty()
            || !string_field(datasource, "orgId", "").is_empty()
    })
}

fn build_data_source_record(
    datasource: &Map<String, Value>,
    include_org_scope: bool,
) -> Vec<String> {
    let mut row = vec![
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
    ];
    if include_org_scope {
        row.push(string_field(datasource, "org", ""));
        row.push(string_field(datasource, "orgId", ""));
    }
    row
}

pub(crate) fn render_data_source_table(
    datasources: &[Map<String, Value>],
    include_header: bool,
) -> Vec<String> {
    let include_org_scope = data_source_rows_include_org_scope(datasources);
    let mut headers = vec![
        "UID".to_string(),
        "NAME".to_string(),
        "TYPE".to_string(),
        "URL".to_string(),
        "IS_DEFAULT".to_string(),
    ];
    if include_org_scope {
        headers.push("ORG".to_string());
        headers.push("ORG_ID".to_string());
    }
    let rows: Vec<Vec<String>> = datasources
        .iter()
        .map(|datasource| build_data_source_record(datasource, include_org_scope))
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

pub(crate) fn render_data_source_csv(datasources: &[Map<String, Value>]) -> Vec<String> {
    let include_org_scope = data_source_rows_include_org_scope(datasources);
    let mut lines = vec![if include_org_scope {
        "uid,name,type,url,isDefault,org,orgId".to_string()
    } else {
        "uid,name,type,url,isDefault".to_string()
    }];
    lines.extend(datasources.iter().map(|datasource| {
        build_data_source_record(datasource, include_org_scope)
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

pub(crate) fn render_data_source_json(datasources: &[Map<String, Value>]) -> Value {
    let include_org_scope = data_source_rows_include_org_scope(datasources);
    Value::Array(
        datasources
            .iter()
            .map(|datasource| {
                let row = build_data_source_record(datasource, include_org_scope);
                let mut object = Map::from_iter(vec![
                    ("uid".to_string(), Value::String(row[0].clone())),
                    ("name".to_string(), Value::String(row[1].clone())),
                    ("type".to_string(), Value::String(row[2].clone())),
                    ("url".to_string(), Value::String(row[3].clone())),
                    ("isDefault".to_string(), Value::String(row[4].clone())),
                ]);
                if include_org_scope {
                    object.insert("org".to_string(), Value::String(row[5].clone()));
                    object.insert("orgId".to_string(), Value::String(row[6].clone()));
                }
                Value::Object(object)
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
            "datasourcesFile".to_string(),
            Value::String(DATASOURCE_EXPORT_FILENAME.to_string()),
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
            "variant".to_string(),
            Value::String("all-orgs-root".to_string()),
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

pub(crate) fn build_all_orgs_export_metadata(org_count: usize, datasource_count: usize) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
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
            Value::String("grafana-datasource-inventory-v1".to_string()),
        ),
    ]))
}

pub(crate) fn build_export_records(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    let org = fetch_current_org(client)?;
    let org_name = string_field(&org, "name", "");
    let org_id = org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let datasources = list_datasources(client)?;
    Ok(datasources
        .into_iter()
        .map(|datasource| {
            let mut record = Map::new();
            record.insert(
                "uid".to_string(),
                Value::String(string_field(&datasource, "uid", "")),
            );
            record.insert(
                "name".to_string(),
                Value::String(string_field(&datasource, "name", "")),
            );
            record.insert(
                "type".to_string(),
                Value::String(string_field(&datasource, "type", "")),
            );
            record.insert(
                "access".to_string(),
                Value::String(string_field(&datasource, "access", "")),
            );
            record.insert(
                "url".to_string(),
                Value::String(string_field(&datasource, "url", "")),
            );
            record.insert(
                "isDefault".to_string(),
                Value::String(
                    if datasource
                        .get("isDefault")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        "true"
                    } else {
                        "false"
                    }
                    .to_string(),
                ),
            );
            record.insert("org".to_string(), Value::String(org_name.clone()));
            record.insert("orgId".to_string(), Value::String(org_id.clone()));
            record
        })
        .collect())
}

pub(crate) fn export_datasource_scope(
    client: &JsonHttpClient,
    output_dir: &Path,
    overwrite: bool,
    dry_run: bool,
) -> Result<usize> {
    let records = build_export_records(client)?;
    let datasources_path = output_dir.join(DATASOURCE_EXPORT_FILENAME);
    let index_path = output_dir.join("index.json");
    let metadata_path = output_dir.join(EXPORT_METADATA_FILENAME);
    if !dry_run {
        write_json_file(
            &datasources_path,
            &Value::Array(records.clone().into_iter().map(Value::Object).collect()),
            overwrite,
        )?;
        write_json_file(&index_path, &build_export_index(&records), overwrite)?;
        write_json_file(
            &metadata_path,
            &build_datasource_export_metadata(records.len()),
            overwrite,
        )?;
    }
    let summary_verb = if dry_run { "Would export" } else { "Exported" };
    println!(
        "{summary_verb} {} datasource(s). Datasources: {} Index: {} Manifest: {}",
        records.len(),
        datasources_path.display(),
        index_path.display(),
        metadata_path.display()
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
        resource: string_field(object, "resource", ""),
        datasources_file: string_field(object, "datasourcesFile", DATASOURCE_EXPORT_FILENAME),
        index_file: string_field(object, "indexFile", "index.json"),
    })
}

pub(crate) fn validate_datasource_contract_record(
    record: &Map<String, Value>,
    context_label: &str,
) -> Result<()> {
    let mut extra_fields = record
        .keys()
        .filter(|key| !DATASOURCE_CONTRACT_FIELDS.contains(&key.as_str()))
        .cloned()
        .collect::<Vec<String>>();
    extra_fields.sort();
    if extra_fields.is_empty() {
        return Ok(());
    }
    Err(message(format!(
        "{context_label} contains unsupported datasource field(s): {}. Supported fields: {}.",
        extra_fields.join(", "),
        DATASOURCE_CONTRACT_FIELDS.join(", ")
    )))
}
