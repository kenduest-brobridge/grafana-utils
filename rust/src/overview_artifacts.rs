//! Overview artifact loading and builder helpers.

use super::overview_support::{
    load_access_export_records, load_json_array_value, load_json_object_value,
    load_object_from_value, object_field_count, overview_inputs, value_is_truthy,
};
use super::{
    OverviewArgs, OverviewArtifact, OverviewInputField, DATASOURCE_EXPORT_FILENAME,
    DATASOURCE_EXPORT_METADATA_FILENAME, DATASOURCE_ROOT_KIND, DATASOURCE_ROOT_SCHEMA_VERSION,
    OVERVIEW_ARTIFACT_ACCESS_ORG_EXPORT_KIND, OVERVIEW_ARTIFACT_ACCESS_SERVICE_ACCOUNT_EXPORT_KIND,
    OVERVIEW_ARTIFACT_ACCESS_TEAM_EXPORT_KIND, OVERVIEW_ARTIFACT_ACCESS_USER_EXPORT_KIND,
    OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND, OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND,
    OVERVIEW_ARTIFACT_DASHBOARD_EXPORT_KIND, OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND,
    OVERVIEW_ARTIFACT_PROMOTION_PREFLIGHT_KIND, OVERVIEW_ARTIFACT_SYNC_SUMMARY_KIND,
    OVERVIEW_SCHEMA_VERSION,
};
use crate::access::{
    ACCESS_ORG_EXPORT_FILENAME, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
    ACCESS_TEAM_EXPORT_FILENAME, ACCESS_USER_EXPORT_FILENAME,
};
use crate::common::{message, Result};
use crate::dashboard::{
    build_export_inspection_summary, build_export_inspection_summary_document,
    build_export_inspection_summary_for_variant, PROVISIONING_EXPORT_SUBDIR,
};
use crate::staged_export_scopes::resolve_datasource_export_scope_dirs;
use crate::sync::bundle_preflight::build_sync_bundle_preflight_document;
use crate::sync::load_datasource_provisioning_records;
use crate::sync::promotion_preflight::build_sync_promotion_preflight_document;
use crate::sync::workbench::build_sync_summary_document;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

fn bundle_context_required(args: &OverviewArgs) -> bool {
    args.source_bundle.is_some()
        || args.target_inventory.is_some()
        || args.mapping_file.is_some()
        || args.availability_file.is_some()
}

fn validate_bundle_context(args: &OverviewArgs) -> Result<()> {
    if !bundle_context_required(args) {
        return Ok(());
    }
    if args.source_bundle.is_none() || args.target_inventory.is_none() {
        return Err(message(
            "Overview bundle and promotion preflights require both --source-bundle and --target-inventory.",
        ));
    }
    Ok(())
}

fn build_dashboard_artifact(
    path: &Path,
    input_name: &str,
    title: &str,
    expected_variant: Option<&str>,
) -> Result<OverviewArtifact> {
    let summary = match expected_variant {
        Some(variant) => build_export_inspection_summary_for_variant(path, variant)?,
        None => build_export_inspection_summary(path)?,
    };
    let document = serde_json::to_value(build_export_inspection_summary_document(&summary))?;
    Ok(OverviewArtifact {
        kind: OVERVIEW_ARTIFACT_DASHBOARD_EXPORT_KIND.to_string(),
        title: title.to_string(),
        inputs: overview_inputs(&[(input_name, path.display().to_string())]),
        document,
    })
}

fn build_access_export_artifact(
    path: &Path,
    payload_filename: &str,
    kind: &str,
    title: &str,
) -> Result<OverviewArtifact> {
    let records = load_access_export_records(
        path,
        payload_filename,
        kind,
        "Overview access export bundle",
    )?;
    let document = Value::Object(Map::from_iter([
        ("kind".to_string(), Value::String(kind.to_string())),
        (
            "schemaVersion".to_string(),
            Value::Number(OVERVIEW_SCHEMA_VERSION.into()),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter([(
                "recordCount".to_string(),
                Value::Number((records.len() as i64).into()),
            )])),
        ),
        (
            "records".to_string(),
            Value::Array(records.into_iter().map(Value::Object).collect()),
        ),
    ]));
    Ok(OverviewArtifact {
        kind: kind.to_string(),
        title: title.to_string(),
        inputs: overview_inputs(&[("exportDir", path.display().to_string())]),
        document,
    })
}

fn build_datasource_artifact(
    title: &str,
    input_name: &str,
    input_value: &Path,
    raw_datasources: Vec<Value>,
) -> Result<OverviewArtifact> {
    let datasource_count = raw_datasources.len();
    let mut org_ids = BTreeSet::new();
    let mut types = BTreeSet::new();
    let mut default_count = 0usize;
    for item in &raw_datasources {
        let datasource = item.as_object().cloned().ok_or_else(|| {
            message(format!(
                "Overview datasource inventory entry must be a JSON object: {}",
                input_value.display()
            ))
        })?;
        let org_id = datasource
            .get("orgId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let org_name = datasource
            .get("org")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let org_key = if !org_id.is_empty() { org_id } else { org_name };
        if !org_key.is_empty() {
            org_ids.insert(org_key);
        }
        let datasource_type = datasource
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if !datasource_type.is_empty() {
            types.insert(datasource_type);
        }
        if value_is_truthy(datasource.get("isDefault")) {
            default_count += 1;
        }
    }

    let document = Value::Object(Map::from_iter([
        (
            "kind".to_string(),
            Value::String(OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(OVERVIEW_SCHEMA_VERSION.into()),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter([
                (
                    "datasourceCount".to_string(),
                    Value::Number((datasource_count as i64).into()),
                ),
                (
                    "orgCount".to_string(),
                    Value::Number((org_ids.len() as i64).into()),
                ),
                (
                    "defaultCount".to_string(),
                    Value::Number((default_count as i64).into()),
                ),
                (
                    "typeCount".to_string(),
                    Value::Number((types.len() as i64).into()),
                ),
            ])),
        ),
        ("datasources".to_string(), Value::Array(raw_datasources)),
    ]));

    Ok(OverviewArtifact {
        kind: OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND.to_string(),
        title: title.to_string(),
        inputs: overview_inputs(&[(input_name, input_value.display().to_string())]),
        document,
    })
}

fn build_datasource_export_artifact(path: &Path) -> Result<OverviewArtifact> {
    let metadata_path = path.join(DATASOURCE_EXPORT_METADATA_FILENAME);
    let metadata = load_object_from_value(&metadata_path, "Overview datasource export metadata")?;
    let schema_version = metadata.get("schemaVersion").and_then(Value::as_i64);
    let kind = metadata.get("kind").and_then(Value::as_str);
    let variant = metadata
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let resource = metadata.get("resource").and_then(Value::as_str);
    if schema_version != Some(DATASOURCE_ROOT_SCHEMA_VERSION)
        || kind != Some(DATASOURCE_ROOT_KIND)
        || resource != Some("datasource")
    {
        return Err(message(format!(
            "Overview datasource export root is not supported: {}",
            metadata_path.display()
        )));
    }

    if !matches!(variant, "root" | "all-orgs-root")
        && resolve_datasource_export_scope_dirs(path)
            .iter()
            .all(|scope| scope != path)
    {
        return Err(message(format!(
            "Overview datasource export root is not supported: {}",
            metadata_path.display()
        )));
    }

    let datasources_file = metadata
        .get("datasourcesFile")
        .and_then(Value::as_str)
        .unwrap_or(DATASOURCE_EXPORT_FILENAME);
    let datasources_path = path.join(datasources_file);
    let raw_datasources =
        load_json_array_value(&datasources_path, "Overview datasource inventory")?;
    build_datasource_artifact("Datasource export", "exportDir", path, raw_datasources)
}

fn build_datasource_provisioning_artifact(path: &Path) -> Result<OverviewArtifact> {
    build_datasource_artifact(
        "Datasource provisioning",
        "datasourceProvisioningFile",
        path,
        load_datasource_provisioning_records(path)?,
    )
}

fn build_sync_summary_artifact(path: &Path) -> Result<OverviewArtifact> {
    let raw_specs = load_json_array_value(path, "Overview desired sync input")?;
    let document = build_sync_summary_document(&raw_specs)?;
    Ok(OverviewArtifact {
        kind: OVERVIEW_ARTIFACT_SYNC_SUMMARY_KIND.to_string(),
        title: "Sync summary".to_string(),
        inputs: overview_inputs(&[("desiredFile", path.display().to_string())]),
        document,
    })
}

fn build_access_user_export_artifact(path: &Path) -> Result<OverviewArtifact> {
    build_access_export_artifact(
        path,
        ACCESS_USER_EXPORT_FILENAME,
        OVERVIEW_ARTIFACT_ACCESS_USER_EXPORT_KIND,
        "Access user export",
    )
}

fn build_access_team_export_artifact(path: &Path) -> Result<OverviewArtifact> {
    build_access_export_artifact(
        path,
        ACCESS_TEAM_EXPORT_FILENAME,
        OVERVIEW_ARTIFACT_ACCESS_TEAM_EXPORT_KIND,
        "Access team export",
    )
}

fn build_access_org_export_artifact(path: &Path) -> Result<OverviewArtifact> {
    build_access_export_artifact(
        path,
        ACCESS_ORG_EXPORT_FILENAME,
        OVERVIEW_ARTIFACT_ACCESS_ORG_EXPORT_KIND,
        "Access org export",
    )
}

fn build_access_service_account_export_artifact(path: &Path) -> Result<OverviewArtifact> {
    build_access_export_artifact(
        path,
        ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
        OVERVIEW_ARTIFACT_ACCESS_SERVICE_ACCOUNT_EXPORT_KIND,
        "Access service-account export",
    )
}

fn build_dashboard_export_artifact(path: &Path) -> Result<OverviewArtifact> {
    build_dashboard_artifact(path, "exportDir", "Dashboard export", None)
}

fn build_dashboard_provisioning_artifact(path: &Path) -> Result<OverviewArtifact> {
    build_dashboard_artifact(
        path,
        "dashboardProvisioningDir",
        "Dashboard provisioning",
        Some(PROVISIONING_EXPORT_SUBDIR),
    )
}

fn build_alert_export_artifact(path: &Path) -> Result<OverviewArtifact> {
    let index_path = path.join("index.json");
    let root_index = load_object_from_value(&index_path, "Overview alert export index")?;
    if root_index.get("schemaVersion").and_then(Value::as_i64)
        != Some(crate::alert::TOOL_SCHEMA_VERSION)
        || root_index.get("apiVersion").and_then(Value::as_i64)
            != Some(crate::alert::TOOL_API_VERSION)
        || root_index.get("kind").and_then(Value::as_str) != Some(crate::alert::ROOT_INDEX_KIND)
    {
        return Err(message(format!(
            "Overview alert export root is not supported: {}",
            index_path.display()
        )));
    }

    let document = Value::Object(Map::from_iter([
        (
            "kind".to_string(),
            Value::String(OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(OVERVIEW_SCHEMA_VERSION.into()),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter([
                (
                    "ruleCount".to_string(),
                    Value::Number((object_field_count(&root_index, "rules") as i64).into()),
                ),
                (
                    "contactPointCount".to_string(),
                    Value::Number(
                        (object_field_count(&root_index, "contact-points") as i64).into(),
                    ),
                ),
                (
                    "muteTimingCount".to_string(),
                    Value::Number((object_field_count(&root_index, "mute-timings") as i64).into()),
                ),
                (
                    "policyCount".to_string(),
                    Value::Number((object_field_count(&root_index, "policies") as i64).into()),
                ),
                (
                    "templateCount".to_string(),
                    Value::Number((object_field_count(&root_index, "templates") as i64).into()),
                ),
            ])),
        ),
        (
            "rules".to_string(),
            Value::Array(
                root_index
                    .get("rules")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            ),
        ),
        (
            "contactPoints".to_string(),
            Value::Array(
                root_index
                    .get("contact-points")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            ),
        ),
        (
            "muteTimings".to_string(),
            Value::Array(
                root_index
                    .get("mute-timings")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            ),
        ),
        (
            "policies".to_string(),
            Value::Array(
                root_index
                    .get("policies")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            ),
        ),
        (
            "templates".to_string(),
            Value::Array(
                root_index
                    .get("templates")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            ),
        ),
    ]));

    Ok(OverviewArtifact {
        kind: OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND.to_string(),
        title: "Alert export".to_string(),
        inputs: overview_inputs(&[("exportDir", path.display().to_string())]),
        document,
    })
}

fn build_bundle_preflight_artifact(
    source_bundle: &Path,
    target_inventory: &Path,
    availability_file: Option<&Path>,
) -> Result<OverviewArtifact> {
    let source_bundle_document =
        load_json_object_value(source_bundle, "Overview source bundle input")?;
    let target_inventory_document =
        load_json_object_value(target_inventory, "Overview target inventory input")?;
    let availability_document = match availability_file {
        None => None,
        Some(path) => Some(load_json_object_value(path, "Overview availability input")?),
    };
    let document = build_sync_bundle_preflight_document(
        &source_bundle_document,
        &target_inventory_document,
        availability_document.as_ref(),
    )?;
    let mut inputs = vec![
        OverviewInputField {
            name: "sourceBundle".to_string(),
            value: source_bundle.display().to_string(),
        },
        OverviewInputField {
            name: "targetInventory".to_string(),
            value: target_inventory.display().to_string(),
        },
    ];
    if let Some(path) = availability_file {
        inputs.push(OverviewInputField {
            name: "availabilityFile".to_string(),
            value: path.display().to_string(),
        });
    }
    Ok(OverviewArtifact {
        kind: OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND.to_string(),
        title: "Sync bundle preflight".to_string(),
        inputs,
        document,
    })
}

fn build_promotion_preflight_artifact(
    source_bundle: &Path,
    target_inventory: &Path,
    mapping_file: Option<&Path>,
    availability_file: Option<&Path>,
) -> Result<OverviewArtifact> {
    let source_bundle_document =
        load_json_object_value(source_bundle, "Overview source bundle input")?;
    let target_inventory_document =
        load_json_object_value(target_inventory, "Overview target inventory input")?;
    let mapping_document = match mapping_file {
        None => None,
        Some(path) => Some(load_json_object_value(
            path,
            "Overview promotion mapping input",
        )?),
    };
    let availability_document = match availability_file {
        None => None,
        Some(path) => Some(load_json_object_value(path, "Overview availability input")?),
    };
    let document = build_sync_promotion_preflight_document(
        &source_bundle_document,
        &target_inventory_document,
        availability_document.as_ref(),
        mapping_document.as_ref(),
    )?;
    let mut inputs = vec![
        OverviewInputField {
            name: "sourceBundle".to_string(),
            value: source_bundle.display().to_string(),
        },
        OverviewInputField {
            name: "targetInventory".to_string(),
            value: target_inventory.display().to_string(),
        },
    ];
    if let Some(path) = mapping_file {
        inputs.push(OverviewInputField {
            name: "mappingFile".to_string(),
            value: path.display().to_string(),
        });
    }
    if let Some(path) = availability_file {
        inputs.push(OverviewInputField {
            name: "availabilityFile".to_string(),
            value: path.display().to_string(),
        });
    }
    Ok(OverviewArtifact {
        kind: OVERVIEW_ARTIFACT_PROMOTION_PREFLIGHT_KIND.to_string(),
        title: "Sync promotion preflight".to_string(),
        inputs,
        document,
    })
}

fn has_any_inputs(args: &OverviewArgs) -> bool {
    args.dashboard_export_dir.is_some()
        || args.dashboard_provisioning_dir.is_some()
        || args.datasource_export_dir.is_some()
        || args.datasource_provisioning_file.is_some()
        || args.access_user_export_dir.is_some()
        || args.access_team_export_dir.is_some()
        || args.access_org_export_dir.is_some()
        || args.access_service_account_export_dir.is_some()
        || args.desired_file.is_some()
        || args.alert_export_dir.is_some()
        || args.source_bundle.is_some()
        || args.target_inventory.is_some()
        || args.availability_file.is_some()
        || args.mapping_file.is_some()
}

pub(crate) fn build_overview_artifacts(args: &OverviewArgs) -> Result<Vec<OverviewArtifact>> {
    if !has_any_inputs(args) {
        return Err(message("Overview requires at least one input artifact."));
    }
    validate_bundle_context(args)?;

    let mut artifacts = Vec::new();
    if let Some(path) = args.dashboard_export_dir.as_deref() {
        artifacts.push(build_dashboard_export_artifact(path)?);
    }
    if let Some(path) = args.dashboard_provisioning_dir.as_deref() {
        artifacts.push(build_dashboard_provisioning_artifact(path)?);
    }
    if let Some(path) = args.datasource_provisioning_file.as_deref() {
        artifacts.push(build_datasource_provisioning_artifact(path)?);
    } else if let Some(path) = args.datasource_export_dir.as_deref() {
        artifacts.push(build_datasource_export_artifact(path)?);
    }
    if let Some(path) = args.access_user_export_dir.as_deref() {
        artifacts.push(build_access_user_export_artifact(path)?);
    }
    if let Some(path) = args.access_team_export_dir.as_deref() {
        artifacts.push(build_access_team_export_artifact(path)?);
    }
    if let Some(path) = args.access_org_export_dir.as_deref() {
        artifacts.push(build_access_org_export_artifact(path)?);
    }
    if let Some(path) = args.access_service_account_export_dir.as_deref() {
        artifacts.push(build_access_service_account_export_artifact(path)?);
    }
    if let Some(path) = args.desired_file.as_deref() {
        artifacts.push(build_sync_summary_artifact(path)?);
    }
    if let Some(path) = args.alert_export_dir.as_deref() {
        artifacts.push(build_alert_export_artifact(path)?);
    }
    if let (Some(source_bundle), Some(target_inventory)) = (
        args.source_bundle.as_deref(),
        args.target_inventory.as_deref(),
    ) {
        artifacts.push(build_bundle_preflight_artifact(
            source_bundle,
            target_inventory,
            args.availability_file.as_deref(),
        )?);
        if args.mapping_file.is_some() {
            artifacts.push(build_promotion_preflight_artifact(
                source_bundle,
                target_inventory,
                args.mapping_file.as_deref(),
                args.availability_file.as_deref(),
            )?);
        }
    }
    Ok(artifacts)
}
