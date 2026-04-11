//! Datasource import dry-run reporting helpers.

use serde_json::{Map, Value};
use std::path::Path;

use crate::common::{render_json_value, tool_version, Result};
use crate::dashboard::DEFAULT_ORG_ID;
use crate::datasource::resolve_match;
use crate::datasource_secret::{
    build_secret_placeholder_plan, inline_secret_provider_contract,
    summarize_secret_placeholder_plan, summarize_secret_provider_contract,
};
use crate::grafana_api::DatasourceResourceClient;

use super::datasource_import_export_support::DatasourceImportDryRunReport;
use super::render_import_table;
use super::{
    describe_datasource_import_mode, fetch_current_org, load_import_records,
    validate_matching_export_org, DatasourceImportArgs, DatasourceImportInputFormat,
};

fn build_import_secret_visibility_entries(
    input_dir: &Path,
    input_format: DatasourceImportInputFormat,
) -> Vec<Value> {
    let Ok((_, records)) = load_import_records(input_dir, input_format) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for record in records {
        let Some(placeholders) = &record.secure_json_data_placeholders else {
            continue;
        };
        let datasource_spec = Map::from_iter(vec![
            ("uid".to_string(), Value::String(record.uid.clone())),
            ("name".to_string(), Value::String(record.name.clone())),
            (
                "type".to_string(),
                Value::String(record.datasource_type.clone()),
            ),
            (
                "secureJsonDataPlaceholders".to_string(),
                Value::Object(placeholders.clone()),
            ),
        ]);
        match build_secret_placeholder_plan(&datasource_spec) {
            Ok(plan) => entries.push(summarize_secret_placeholder_plan(&plan)),
            Err(error) => entries.push(Value::Object(Map::from_iter(vec![
                (
                    "provider".to_string(),
                    summarize_secret_provider_contract(&inline_secret_provider_contract()),
                ),
                (
                    "datasourceUid".to_string(),
                    Value::String(record.uid.clone()),
                ),
                (
                    "datasourceName".to_string(),
                    Value::String(record.name.clone()),
                ),
                (
                    "datasourceType".to_string(),
                    Value::String(record.datasource_type.clone()),
                ),
                (
                    "providerKind".to_string(),
                    Value::String(inline_secret_provider_contract().kind),
                ),
                (
                    "action".to_string(),
                    Value::String("secret-plan-error".to_string()),
                ),
                ("reviewRequired".to_string(), Value::Bool(true)),
                ("error".to_string(), Value::String(error.to_string())),
            ]))),
        }
    }
    entries
}

pub(crate) fn format_datasource_import_dry_run_line(row: &[String]) -> String {
    format!(
        "Dry-run datasource uid={} name={} type={} match={} dest={} action={} file={}",
        row[0], row[1], row[2], row[3], row[4], row[5], row[7]
    )
}

pub(crate) fn collect_datasource_import_dry_run_report(
    client: &crate::http::JsonHttpClient,
    args: &DatasourceImportArgs,
) -> Result<DatasourceImportDryRunReport> {
    let replace_existing = args.replace_existing || args.update_existing_only;
    let (metadata, records) = load_import_records(&args.input_dir, args.input_format)?;
    validate_matching_export_org(client, args, &records)?;
    let live = DatasourceResourceClient::new(client).list_datasources()?;
    let target_org = fetch_current_org(client)?;
    let target_org_id = target_org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let mode = describe_datasource_import_mode(args.replace_existing, args.update_existing_only);
    let mut rows = Vec::new();
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut blocked = 0usize;
    for (index, record) in records.iter().enumerate() {
        let matching = resolve_match(record, &live, replace_existing, args.update_existing_only);
        let file_ref = format!("{}#{}", metadata.datasources_file, index);
        rows.push(vec![
            record.uid.clone(),
            record.name.clone(),
            record.datasource_type.clone(),
            matching.match_basis.to_string(),
            matching.destination.to_string(),
            matching.action.to_string(),
            target_org_id.clone(),
            file_ref,
        ]);
        match matching.action {
            "would-create" => created += 1,
            "would-update" => updated += 1,
            "would-skip-missing" => skipped += 1,
            _ => blocked += 1,
        }
    }
    Ok(DatasourceImportDryRunReport {
        mode: mode.to_string(),
        input_dir: args.input_dir.clone(),
        input_format: args.input_format,
        source_org_id: records
            .iter()
            .find(|item| !item.org_id.is_empty())
            .map(|item| item.org_id.clone())
            .unwrap_or_default(),
        target_org_id,
        rows,
        datasource_count: records.len(),
        would_create: created,
        would_update: updated,
        would_skip: skipped,
        would_block: blocked,
    })
}

pub(crate) fn build_datasource_import_dry_run_json_value(
    report: &DatasourceImportDryRunReport,
) -> Value {
    let secret_visibility =
        build_import_secret_visibility_entries(&report.input_dir, report.input_format);
    Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String("grafana-util-datasource-import-dry-run".to_string()),
        ),
        ("schemaVersion".to_string(), Value::Number(1.into())),
        (
            "toolVersion".to_string(),
            Value::String(tool_version().to_string()),
        ),
        ("reviewRequired".to_string(), Value::Bool(true)),
        ("reviewed".to_string(), Value::Bool(false)),
        ("mode".to_string(), Value::String(report.mode.clone())),
        (
            "sourceOrgId".to_string(),
            Value::String(report.source_org_id.clone()),
        ),
        (
            "targetOrgId".to_string(),
            Value::String(report.target_org_id.clone()),
        ),
        (
            "datasources".to_string(),
            Value::Array(
                report
                    .rows
                    .iter()
                    .map(|row| {
                        Value::Object(Map::from_iter(vec![
                            ("uid".to_string(), Value::String(row[0].clone())),
                            ("name".to_string(), Value::String(row[1].clone())),
                            ("type".to_string(), Value::String(row[2].clone())),
                            ("matchBasis".to_string(), Value::String(row[3].clone())),
                            ("destination".to_string(), Value::String(row[4].clone())),
                            ("action".to_string(), Value::String(row[5].clone())),
                            ("orgId".to_string(), Value::String(row[6].clone())),
                            ("file".to_string(), Value::String(row[7].clone())),
                        ]))
                    })
                    .collect(),
            ),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "datasourceCount".to_string(),
                    Value::Number((report.datasource_count as i64).into()),
                ),
                (
                    "wouldCreate".to_string(),
                    Value::Number((report.would_create as i64).into()),
                ),
                (
                    "wouldUpdate".to_string(),
                    Value::Number((report.would_update as i64).into()),
                ),
                (
                    "wouldSkip".to_string(),
                    Value::Number((report.would_skip as i64).into()),
                ),
                (
                    "wouldBlock".to_string(),
                    Value::Number((report.would_block as i64).into()),
                ),
                (
                    "secretVisibilityCount".to_string(),
                    Value::Number((secret_visibility.len() as i64).into()),
                ),
            ])),
        ),
        (
            "secretVisibility".to_string(),
            Value::Array(secret_visibility),
        ),
    ]))
}

pub(crate) fn print_datasource_import_dry_run_report(
    report: &DatasourceImportDryRunReport,
    args: &DatasourceImportArgs,
) -> Result<()> {
    if args.json {
        print!(
            "{}",
            render_json_value(&build_datasource_import_dry_run_json_value(report))?
        );
    } else if args.table {
        for line in render_import_table(
            &report.rows,
            !args.no_header,
            if args.output_columns.is_empty() {
                None
            } else {
                Some(args.output_columns.as_slice())
            },
        ) {
            println!("{line}");
        }
        println!(
            "Dry-run checked {} datasource(s) from {}",
            report.datasource_count,
            report.input_dir.display()
        );
        let secret_visibility =
            build_import_secret_visibility_entries(&report.input_dir, report.input_format);
        if !secret_visibility.is_empty() {
            println!(
                "Secret placeholder visibility: {}",
                Value::Array(secret_visibility)
            );
        }
    } else {
        println!("Import mode: {}", report.mode);
        for row in &report.rows {
            println!("{}", format_datasource_import_dry_run_line(row));
        }
        println!(
            "Dry-run checked {} datasource(s) from {}",
            report.datasource_count,
            report.input_dir.display()
        );
        let secret_visibility =
            build_import_secret_visibility_entries(&report.input_dir, report.input_format);
        if !secret_visibility.is_empty() {
            println!(
                "Secret placeholder visibility: {}",
                Value::Array(secret_visibility)
            );
        }
    }
    Ok(())
}
