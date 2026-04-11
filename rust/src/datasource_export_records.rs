//! Datasource record and provisioning document helpers.

use serde::Serialize;
use serde_json::{Map, Value};
use std::path::Path;

use crate::common::{string_field, Result};
use crate::dashboard::{list_datasources, DEFAULT_ORG_ID};
use crate::datasource::fetch_datasource_by_uid_if_exists;
use crate::datasource_secret::{
    build_inline_secret_placeholder_token, inline_secret_provider_contract,
    summarize_secret_provider_contract,
};
use crate::export_metadata::{build_export_metadata_common, export_metadata_common_map};

use super::super::datasource_import_export_support::{
    fetch_current_org, DatasourceImportRecord, DATASOURCE_EXPORT_FILENAME,
    EXPORT_METADATA_FILENAME, ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION,
};
use super::DATASOURCE_EXPORT_MODE;
use super::DATASOURCE_MASKED_RECOVERY_FORMAT;
use super::DATASOURCE_PROVISIONING_PROJECTION_MODE;
use super::DATASOURCE_SECRET_MATERIAL_MODE;
use super::{DATASOURCE_PROVISIONING_FILENAME, DATASOURCE_PROVISIONING_SUBDIR};

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
            Value::String(crate::common::tool_version().to_string()),
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
            Value::String(crate::common::tool_version().to_string()),
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
        super::EXPORT_BUNDLE_KIND_ROOT,
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
            Value::String(crate::common::tool_version().to_string()),
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

pub(crate) fn build_export_record_from_datasource(
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

pub(crate) fn build_list_records(
    client: &crate::http::JsonHttpClient,
) -> Result<Vec<Map<String, Value>>> {
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

pub(crate) fn build_export_records(
    client: &crate::http::JsonHttpClient,
) -> Result<Vec<Map<String, Value>>> {
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
