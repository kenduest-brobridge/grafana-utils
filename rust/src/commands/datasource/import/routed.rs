//! Import orchestration for Core resources, including input normalization and apply contract handling.

use serde_json::{json, Value};
use std::path::Path;

use crate::common::{message, render_json_value, tool_version, Result};
use crate::http::JsonHttpClient;

use super::{
    build_api_client, build_datasource_import_dry_run_json_value,
    build_http_client_for_org_from_api, collect_datasource_import_dry_run_report, create_org,
    describe_datasource_import_mode, list_orgs, load_import_records, org_id_string_from_value,
    DatasourceExportOrgScope, DatasourceExportOrgTargetPlan, DatasourceImportArgs,
};

pub(crate) fn resolve_export_org_target_plan(
    admin_client: &JsonHttpClient,
    args: &DatasourceImportArgs,
    scope: &DatasourceExportOrgScope,
) -> Result<DatasourceExportOrgTargetPlan> {
    let orgs = list_orgs(admin_client)?;
    for org in orgs {
        let org_id_text = org_id_string_from_value(org.get("id"));
        if org_id_text == scope.source_org_id.to_string() {
            return Ok(DatasourceExportOrgTargetPlan {
                source_org_id: scope.source_org_id,
                source_org_name: scope.source_org_name.clone(),
                target_org_id: Some(scope.source_org_id),
                org_action: "exists",
                input_dir: scope.input_dir.clone(),
            });
        }
    }
    if args.dry_run && !args.create_missing_orgs {
        return Ok(DatasourceExportOrgTargetPlan {
            source_org_id: scope.source_org_id,
            source_org_name: scope.source_org_name.clone(),
            target_org_id: None,
            org_action: "missing",
            input_dir: scope.input_dir.clone(),
        });
    }
    if args.dry_run && args.create_missing_orgs {
        return Ok(DatasourceExportOrgTargetPlan {
            source_org_id: scope.source_org_id,
            source_org_name: scope.source_org_name.clone(),
            target_org_id: None,
            org_action: "would-create",
            input_dir: scope.input_dir.clone(),
        });
    }
    if !args.create_missing_orgs {
        return Err(message(format!(
            "Datasource import source orgId {} was not found in destination Grafana. Use --create-missing-orgs to create it from export metadata.",
            scope.source_org_id
        )));
    }
    if scope.source_org_name.trim().is_empty() {
        return Err(message(format!(
            "Datasource import with --create-missing-orgs could not determine an exported org name for source orgId {}.",
            scope.source_org_id
        )));
    }
    let created = create_org(admin_client, &scope.source_org_name)?;
    let created_org_id =
        org_id_string_from_value(created.get("orgId").or_else(|| created.get("id")));
    if created_org_id.is_empty() {
        return Err(message(format!(
            "Grafana did not return a usable orgId after creating destination org '{}' for exported org {}.",
            scope.source_org_name, scope.source_org_id
        )));
    }
    let parsed_org_id = created_org_id.parse::<i64>().map_err(|_| {
        message(format!(
            "Grafana returned non-numeric orgId '{}' after creating destination org '{}' for exported org {}.",
            created_org_id, scope.source_org_name, scope.source_org_id
        ))
    })?;
    Ok(DatasourceExportOrgTargetPlan {
        source_org_id: scope.source_org_id,
        source_org_name: scope.source_org_name.clone(),
        target_org_id: Some(parsed_org_id),
        org_action: "created",
        input_dir: scope.input_dir.clone(),
    })
}

pub(crate) fn format_routed_datasource_target_org_label(target_org_id: Option<i64>) -> String {
    target_org_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| "<new>".to_string())
}

pub(crate) fn format_routed_datasource_source_org_label(
    source_org_id: i64,
    source_org_name: &str,
) -> String {
    let source_org_name = source_org_name.trim();
    if source_org_name.is_empty() {
        source_org_id.to_string()
    } else {
        format!("{source_org_id}:{source_org_name}")
    }
}

#[cfg(test)]
pub(crate) fn format_routed_datasource_scope_summary_fields(
    source_org_id: i64,
    source_org_name: &str,
    org_action: &str,
    target_org_id: Option<i64>,
    input_dir: &Path,
) -> String {
    let source_org_name = if source_org_name.is_empty() {
        "-".to_string()
    } else {
        source_org_name.to_string()
    };
    let target_org_id = format_routed_datasource_target_org_label(target_org_id);
    format!(
        "export orgId={} name={} orgAction={} targetOrgId={} from {}",
        source_org_id,
        source_org_name,
        org_action,
        target_org_id,
        input_dir.display()
    )
}

pub(crate) fn render_routed_datasource_import_org_table(
    rows: &[Vec<String>],
    include_header: bool,
) -> Vec<String> {
    let headers = vec![
        "SOURCE_ORG_ID".to_string(),
        "SOURCE_ORG_NAME".to_string(),
        "ORG_ACTION".to_string(),
        "TARGET_ORG_ID".to_string(),
        "DATASOURCE_COUNT".to_string(),
        "IMPORT_DIR".to_string(),
    ];
    let mut widths: Vec<usize> = headers.iter().map(|item| item.len()).collect();
    for row in rows {
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
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>();
    let mut lines = Vec::new();
    if include_header {
        lines.push(format_row(&headers));
        lines.push(format_row(&separator));
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

pub(crate) fn format_routed_datasource_import_summary_line(
    org_count: usize,
    source_org_labels: &[String],
    existing_org_count: usize,
    missing_org_count: usize,
    would_create_org_count: usize,
    datasource_count: usize,
    input_dir: &Path,
) -> String {
    let source_org_labels = if source_org_labels.is_empty() {
        "<none>".to_string()
    } else {
        format!("[{}]", source_org_labels.join(", "))
    };
    format!(
        "Routed datasource import summary: orgs={} sources={} existing={} missing={} would-create={} datasources={} from {}",
        org_count,
        source_org_labels,
        existing_org_count,
        missing_org_count,
        would_create_org_count,
        datasource_count,
        input_dir.display()
    )
}

pub(crate) fn build_routed_datasource_import_dry_run_json(
    args: &DatasourceImportArgs,
) -> Result<String> {
    let admin_api = build_api_client(&args.common)?;
    let admin_client = admin_api.http_client();
    let scopes = super::discover_export_org_import_scopes(args)?;
    let mut orgs = Vec::new();
    let mut imports = Vec::new();
    for scope in scopes {
        let plan = resolve_export_org_target_plan(admin_client, args, &scope)?;
        let datasource_count = load_import_records(&plan.input_dir, args.input_format)?
            .1
            .len();
        orgs.push(json!({
            "sourceOrgId": plan.source_org_id,
            "sourceOrgName": plan.source_org_name,
            "orgAction": plan.org_action,
            "targetOrgId": plan.target_org_id,
            "datasourceCount": datasource_count,
            "importDir": plan.input_dir.display().to_string(),
        }));
        let preview = if let Some(target_org_id) = plan.target_org_id {
            let mut scoped_args = args.clone();
            scoped_args.org_id = Some(target_org_id);
            scoped_args.use_export_org = false;
            scoped_args.only_org_id = Vec::new();
            scoped_args.create_missing_orgs = false;
            scoped_args.input_dir = plan.input_dir.clone();
            let scoped_client = build_http_client_for_org_from_api(&admin_api, target_org_id)?;
            build_datasource_import_dry_run_json_value(&collect_datasource_import_dry_run_report(
                &scoped_client,
                &scoped_args,
            )?)
        } else {
            json!({
                "mode": describe_datasource_import_mode(args.replace_existing, args.update_existing_only),
                "sourceOrgId": plan.source_org_id.to_string(),
                "targetOrgId": Value::Null,
                "datasources": [],
                "summary": {
                    "datasourceCount": datasource_count,
                    "wouldCreate": 0,
                    "wouldUpdate": 0,
                    "wouldSkip": 0,
                    "wouldBlock": 0
                }
            })
        };
        let mut import_entry = serde_json::Map::new();
        import_entry.insert("sourceOrgId".to_string(), Value::from(plan.source_org_id));
        import_entry.insert(
            "sourceOrgName".to_string(),
            Value::from(plan.source_org_name.clone()),
        );
        import_entry.insert("orgAction".to_string(), Value::from(plan.org_action));
        import_entry.insert(
            "targetOrgId".to_string(),
            plan.target_org_id.map(Value::from).unwrap_or(Value::Null),
        );
        if let Some(object) = preview.as_object() {
            for (key, value) in object {
                import_entry.insert(key.clone(), value.clone());
            }
        }
        imports.push(Value::Object(import_entry));
    }
    let summary = json!({
        "orgCount": orgs.len(),
        "sourceOrgLabels": orgs.iter().map(|entry| {
            format_routed_datasource_source_org_label(
                entry.get("sourceOrgId").and_then(Value::as_i64).unwrap_or_default(),
                entry.get("sourceOrgName").and_then(Value::as_str).unwrap_or_default(),
            )
        }).collect::<Vec<String>>(),
        "existingOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("exists".to_string()))).count(),
        "missingOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("missing".to_string()))).count(),
        "wouldCreateOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("would-create".to_string()))).count(),
        "datasourceCount": imports.iter().filter_map(|entry| entry.get("summary").and_then(|summary| summary.get("datasourceCount")).and_then(Value::as_i64)).sum::<i64>(),
    });
    render_json_value(&json!({
        "kind": "grafana-util-datasource-import-dry-run-routed",
        "schemaVersion": 1,
        "toolVersion": tool_version(),
        "reviewRequired": true,
        "reviewed": false,
        "mode": describe_datasource_import_mode(args.replace_existing, args.update_existing_only),
        "orgs": orgs,
        "imports": imports,
        "summary": summary,
    }))
}
