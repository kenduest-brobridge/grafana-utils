//! Shared helpers for datasource list/export/import orchestration.
//!
//! Responsibilities:
//! - Build typed records and export indexes from API payloads.
//! - Resolve target output directories and per-org export scopes.
//! - Serialize provisioning artifacts and metadata in supported output formats.

use serde::Serialize;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    message, sanitize_path_component, string_field, tool_version, write_json_file, Result,
};
use crate::dashboard::{
    build_auth_context, build_http_client, build_http_client_for_org, CommonCliArgs, DEFAULT_ORG_ID,
};
use crate::datasource_secret::{
    inline_secret_provider_contract, summarize_secret_provider_contract,
};
use crate::export_metadata::{
    build_export_metadata_common, export_metadata_common_map, EXPORT_BUNDLE_KIND_ROOT,
};
use crate::http::JsonHttpClient;

use super::datasource_import_export_support::{
    DatasourceExportMetadata, DATASOURCE_EXPORT_FILENAME, EXPORT_METADATA_FILENAME,
    ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION,
};

pub(crate) const DATASOURCE_PROVISIONING_SUBDIR: &str = "provisioning";
pub(crate) const DATASOURCE_PROVISIONING_FILENAME: &str = "datasources.yaml";
const DATASOURCE_MASKED_RECOVERY_FORMAT: &str = "grafana-datasource-masked-recovery-v1";
const DATASOURCE_EXPORT_MODE: &str = "masked-recovery";
const DATASOURCE_SECRET_MATERIAL_MODE: &str = "placeholders-only";
const DATASOURCE_PROVISIONING_PROJECTION_MODE: &str = "derived-projection";
#[path = "datasource_export_columns.rs"]
mod datasource_export_columns;
#[path = "datasource_export_records.rs"]
mod datasource_export_records;

pub(crate) use datasource_export_columns::{
    datasource_list_column_ids, render_data_source_csv, render_data_source_json,
    render_data_source_summary_line, render_data_source_table,
};
#[cfg(test)]
pub(crate) use datasource_export_records::build_export_record_from_datasource;
pub(crate) use datasource_export_records::{
    build_all_orgs_export_index, build_all_orgs_export_metadata,
    build_datasource_provisioning_document, build_export_index, build_export_records,
    build_list_records,
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
