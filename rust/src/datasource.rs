//! Datasource domain orchestrator.
//!
//! Purpose:
//! - Own datasource command flows (`list`, `add`, `delete`, `export`, `import`, `diff`).
//! - Normalize datasource contract shape across live API payloads and exported metadata.
//! - Keep output serialization (`table`/`csv`/`json`/`yaml`) selection centralized.
//!
//! Flow:
//! - Parse args from `dashboard`-shared auth/common CLI types where possible.
//! - Normalize command variants before branching by subcommand.
//! - Build client and route execution to list/export/import/diff helpers.
//!
//! Caveats:
//! - Keep API-field compatibility logic in `datasource_diff.rs` and import/export helpers.
//! - Avoid side effects in normalization helpers; keep them as pure value transforms.
use serde_json::{Map, Value};
use std::path::Path;

use crate::common::{
    build_shared_diff_document, message, print_supported_columns, render_json_value, string_field,
    write_json_file, DiffOutputFormat, Result, SharedDiffSummary,
};
use crate::dashboard::CommonCliArgs;
use crate::datasource::datasource_diff::{
    build_datasource_diff_report, normalize_export_records, normalize_live_records,
    DatasourceDiffEntry, DatasourceDiffReport, DatasourceDiffStatus,
};
#[cfg(any(feature = "tui", test))]
use crate::interactive_browser::run_interactive_browser;

#[path = "datasource_browse.rs"]
mod datasource_browse;
#[cfg(feature = "tui")]
#[path = "datasource_browse_edit_dialog.rs"]
mod datasource_browse_edit_dialog;
#[cfg(feature = "tui")]
#[path = "datasource_browse_input.rs"]
mod datasource_browse_input;
#[cfg(feature = "tui")]
#[path = "datasource_browse_render.rs"]
mod datasource_browse_render;
#[cfg(feature = "tui")]
#[path = "datasource_browse_state.rs"]
mod datasource_browse_state;
#[path = "datasource_browse_support.rs"]
mod datasource_browse_support;
#[cfg(feature = "tui")]
#[path = "datasource_browse_terminal.rs"]
mod datasource_browse_terminal;
#[cfg(feature = "tui")]
#[path = "datasource_browse_tui.rs"]
mod datasource_browse_tui;
#[path = "datasource_cli_defs.rs"]
mod datasource_cli_defs;
#[path = "datasource_diff.rs"]
mod datasource_diff;
#[path = "datasource_import_export.rs"]
mod datasource_import_export;
#[path = "datasource_inspect_export.rs"]
mod datasource_inspect_export;
#[path = "datasource_mutation_support.rs"]
mod datasource_mutation_support;
#[path = "datasource_runtime.rs"]
mod datasource_runtime;

pub(crate) use datasource_cli_defs::{normalize_datasource_group_command, root_command};
pub use datasource_cli_defs::{
    DatasourceAddArgs, DatasourceBrowseArgs, DatasourceCliArgs, DatasourceDeleteArgs,
    DatasourceDiffArgs, DatasourceExportArgs, DatasourceGroupCommand, DatasourceImportArgs,
    DatasourceImportInputFormat, DatasourceListArgs, DatasourceModifyArgs, DatasourceTypesArgs,
    DryRunOutputFormat, ListOutputFormat,
};
pub(crate) use datasource_import_export::{
    build_all_orgs_export_index, build_all_orgs_export_metadata, build_all_orgs_output_dir,
    build_datasource_export_metadata, build_datasource_provisioning_document, build_export_index,
    build_export_records, build_list_records, datasource_list_column_ids, fetch_current_org,
    import_datasources_by_export_org, import_datasources_with_client, list_orgs,
    load_datasource_export_root_manifest, load_datasource_inventory_records_from_export_root,
    load_diff_record_values, load_import_records, render_data_source_csv, render_data_source_json,
    render_data_source_summary_line, render_data_source_table, resolve_datasource_export_root_dir,
    resolve_target_client, validate_import_org_auth, write_yaml_file,
    DatasourceExportRootScopeKind, DatasourceImportRecord, DATASOURCE_EXPORT_FILENAME,
    DATASOURCE_PROVISIONING_FILENAME, DATASOURCE_PROVISIONING_SUBDIR, EXPORT_METADATA_FILENAME,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use datasource_import_export::{
    build_import_payload, build_import_payload_with_secret_values,
    classify_datasource_export_root_scope_kind, collect_datasource_import_dry_run_report,
    discover_export_org_import_scopes, format_routed_datasource_import_summary_line,
    format_routed_datasource_scope_summary_fields, format_routed_datasource_target_org_label,
    render_routed_datasource_import_org_table, resolve_export_org_target_plan,
    DatasourceExportOrgScope, DatasourceExportOrgTargetPlan, DatasourceImportDryRunReport,
};
#[cfg(any(feature = "tui", test))]
#[allow(unused_imports)]
pub(crate) use datasource_inspect_export::{
    build_datasource_inspect_export_browser_items, load_datasource_inspect_export_source,
    prompt_datasource_inspect_export_input_format, render_datasource_inspect_export_output,
    resolve_datasource_inspect_export_input_format, DatasourceInspectExportRenderFormat,
    DatasourceInspectExportSource,
};
#[cfg(not(any(feature = "tui", test)))]
#[allow(unused_imports)]
pub(crate) use datasource_inspect_export::{
    load_datasource_inspect_export_source, prompt_datasource_inspect_export_input_format,
    render_datasource_inspect_export_output, resolve_datasource_inspect_export_input_format,
    DatasourceInspectExportRenderFormat, DatasourceInspectExportSource,
};
#[cfg(test)]
pub(crate) use datasource_mutation_support::parse_json_object_argument;
use datasource_mutation_support::{
    build_add_payload, build_modify_payload, build_modify_updates, render_import_table,
    render_live_mutation_json, render_live_mutation_table, resolve_delete_match,
    resolve_live_mutation_match, validate_live_mutation_dry_run_args,
};
pub(crate) use datasource_mutation_support::{fetch_datasource_by_uid_if_exists, resolve_match};
pub use datasource_runtime::run_datasource_cli;
fn render_datasource_text(
    records: &[Map<String, Value>],
    selected_columns: &[String],
) -> Vec<String> {
    records
        .iter()
        .map(|record| {
            render_data_source_summary_line(
                record,
                (!selected_columns.is_empty()).then_some(selected_columns),
            )
        })
        .collect()
}

fn resolve_local_datasource_list_format(
    args: &DatasourceListArgs,
) -> DatasourceInspectExportRenderFormat {
    if args.table {
        DatasourceInspectExportRenderFormat::Table
    } else if args.csv {
        DatasourceInspectExportRenderFormat::Csv
    } else if args.json {
        DatasourceInspectExportRenderFormat::Json
    } else if args.yaml {
        DatasourceInspectExportRenderFormat::Yaml
    } else {
        DatasourceInspectExportRenderFormat::Table
    }
}

fn run_local_datasource_list(args: &DatasourceListArgs) -> Result<()> {
    if args.all_orgs || args.org_id.is_some() {
        return Err(message(
            "Datasource list with --input-dir does not support --org-id or --all-orgs.",
        ));
    }
    if args.list_columns {
        print_supported_columns(datasource_list_column_ids());
        return Ok(());
    }
    let input_dir = args
        .input_dir
        .as_ref()
        .ok_or_else(|| message("Datasource list local mode requires --input-dir."))?;
    let input_format =
        resolve_datasource_inspect_export_input_format(input_dir, args.input_format)?.ok_or_else(
            || {
                message(format!(
                    "Datasource list could not find export-metadata.json or provisioning/datasources.yaml under {}.",
                    input_dir.display()
                ))
            },
        )?;
    if args.interactive {
        #[cfg(feature = "tui")]
        {
            let source = load_datasource_inspect_export_source(input_dir, input_format)?;
            let summary_lines = vec![
                "Datasource list".to_string(),
                format!("Input: {}", source.input_path),
                format!("Mode: {}", source.input_mode),
                format!("Datasources: {}", source.records.len()),
            ];
            let items = build_datasource_inspect_export_browser_items(&source);
            return run_interactive_browser("Datasource list", &summary_lines, &items);
        }
        #[cfg(not(feature = "tui"))]
        {
            return Err(crate::common::tui(
                "Datasource list --interactive requires the `tui` feature.",
            ));
        }
    }
    let source = load_datasource_inspect_export_source(input_dir, input_format)?;
    let format = resolve_local_datasource_list_format(args);
    let rendered = render_datasource_inspect_export_output(
        &source,
        format,
        (!args.output_columns.is_empty()).then_some(args.output_columns.as_slice()),
    )?;
    print!("{rendered}");
    Ok(())
}

fn render_diff_identity(entry: &DatasourceDiffEntry) -> String {
    let diff_type = entry
        .export_record
        .as_ref()
        .map(|record| record.datasource_type.as_str())
        .or_else(|| {
            entry
                .live_record
                .as_ref()
                .map(|record| record.datasource_type.as_str())
        })
        .unwrap_or_default();
    if let Some(record) = &entry.export_record {
        if !record.uid.is_empty() {
            return if diff_type.is_empty() {
                format!("uid={} name={}", record.uid, record.name)
            } else {
                format!("uid={} name={} type={}", record.uid, record.name, diff_type)
            };
        }
        return if diff_type.is_empty() {
            format!("name={}", record.name)
        } else {
            format!("name={} type={}", record.name, diff_type)
        };
    }
    if let Some(record) = &entry.live_record {
        if !record.uid.is_empty() {
            return if diff_type.is_empty() {
                format!("uid={} name={}", record.uid, record.name)
            } else {
                format!("uid={} name={} type={}", record.uid, record.name, diff_type)
            };
        }
        return if diff_type.is_empty() {
            format!("name={}", record.name)
        } else {
            format!("name={} type={}", record.name, diff_type)
        };
    }
    entry.key.clone()
}

fn render_diff_match_basis(entry: &DatasourceDiffEntry) -> &'static str {
    if let Some(record) = &entry.export_record {
        if !record.uid.is_empty() {
            return "uid";
        }
        if !record.name.is_empty() {
            return "name";
        }
    }
    if let Some(record) = &entry.live_record {
        if !record.uid.is_empty() {
            return "uid";
        }
        if !record.name.is_empty() {
            return "name";
        }
    }
    "unknown"
}

fn resolve_delete_preview_type(target_id: Option<i64>, live: &[Map<String, Value>]) -> String {
    let Some(target_id) = target_id else {
        return String::new();
    };
    live.iter()
        .find(|item| item.get("id").and_then(Value::as_i64) == Some(target_id))
        .map(|item| string_field(item, "type", ""))
        .unwrap_or_default()
}

// Render the diff as an operator-summary report rather than a machine contract.
fn print_datasource_diff_summary_report(report: &DatasourceDiffReport) {
    for entry in &report.entries {
        let identity = render_diff_identity(entry);
        let match_basis = render_diff_match_basis(entry);
        match entry.status {
            DatasourceDiffStatus::Matches => {
                println!("Diff same datasource {identity} match={match_basis}");
            }
            DatasourceDiffStatus::Different => {
                let changed_fields = entry
                    .differences
                    .iter()
                    .map(|item| item.field)
                    .collect::<Vec<&str>>()
                    .join(",");
                println!(
                    "Diff different datasource {identity} match={match_basis} fields={changed_fields}"
                );
            }
            DatasourceDiffStatus::MissingInLive => {
                println!("Diff missing-live datasource {identity} match={match_basis}");
            }
            DatasourceDiffStatus::MissingInExport => {
                println!("Diff extra-live datasource {identity} match={match_basis}");
            }
            DatasourceDiffStatus::AmbiguousLiveMatch => {
                println!("Diff ambiguous-live datasource {identity} match={match_basis}");
            }
        }
    }
}

fn datasource_diff_summary(report: &DatasourceDiffReport) -> SharedDiffSummary {
    SharedDiffSummary {
        checked: report.summary.compared_count,
        same: report.summary.matches_count,
        different: report.summary.different_count,
        missing_remote: report.summary.missing_in_live_count,
        extra_remote: report.summary.missing_in_export_count,
        ambiguous: report.summary.ambiguous_live_match_count,
    }
}

fn datasource_diff_summary_line(diff_dir: &Path, report: &DatasourceDiffReport) -> String {
    let summary = &report.summary;
    let status_breakdown = format!(
        "same={} different={} missing-live={} extra-live={} ambiguous={}",
        summary.matches_count,
        summary.different_count,
        summary.missing_in_live_count,
        summary.missing_in_export_count,
        summary.ambiguous_live_match_count
    );
    let difference_count = summary.compared_count - summary.matches_count;
    if difference_count > 0 {
        format!(
            "Diff checked {} datasource(s) from {} against Grafana live datasources; {} difference(s) found ({status_breakdown}).",
            summary.compared_count,
            diff_dir.display(),
            difference_count
        )
    } else {
        format!(
            "No datasource differences across {} datasource(s) from {} against Grafana live datasources ({status_breakdown}).",
            summary.compared_count,
            diff_dir.display(),
        )
    }
}

fn datasource_diff_row(entry: &DatasourceDiffEntry) -> Value {
    let identity = render_diff_identity(entry);
    let match_basis = render_diff_match_basis(entry);
    let datasource_type = entry
        .export_record
        .as_ref()
        .map(|record| record.datasource_type.clone())
        .or_else(|| {
            entry
                .live_record
                .as_ref()
                .map(|record| record.datasource_type.clone())
        })
        .unwrap_or_default();
    serde_json::json!({
        "domain": "datasource",
        "resourceKind": "datasource",
        "identity": identity,
        "type": datasource_type,
        "matchBasis": match_basis,
        "status": entry.status.as_str(),
        "path": Value::Null,
        "changedFields": entry
            .differences
            .iter()
            .map(|item| item.field)
            .collect::<Vec<&str>>(),
        "changes": entry
            .differences
            .iter()
            .map(|item| serde_json::json!({
                "field": item.field,
                "before": item.expected,
                "after": item.actual,
            }))
            .collect::<Vec<Value>>(),
    })
}

/// Purpose: implementation note.
pub(crate) fn diff_datasources_with_live(
    diff_dir: &Path,
    input_format: DatasourceImportInputFormat,
    live: &[Map<String, Value>],
    output_format: DiffOutputFormat,
) -> Result<(usize, usize)> {
    let export_values = load_diff_record_values(diff_dir, input_format)?;
    let live_values = live
        .iter()
        .cloned()
        .map(Value::Object)
        .collect::<Vec<Value>>();
    let report = build_datasource_diff_report(
        &normalize_export_records(&export_values),
        &normalize_live_records(&live_values),
    );
    match output_format {
        DiffOutputFormat::Text => print_datasource_diff_summary_report(&report),
        DiffOutputFormat::Json => {
            let rows = report
                .entries
                .iter()
                .map(datasource_diff_row)
                .collect::<Vec<Value>>();
            print!(
                "{}",
                render_json_value(&build_shared_diff_document(
                    "grafana-util-datasource-diff",
                    1,
                    datasource_diff_summary(&report),
                    &rows,
                ))?
            );
        }
    }
    if matches!(output_format, DiffOutputFormat::Text) {
        println!("{}", datasource_diff_summary_line(diff_dir, &report));
    }
    let difference_count = report.summary.compared_count - report.summary.matches_count;
    Ok((report.summary.compared_count, difference_count))
}

#[cfg(test)]
mod datasource_operator_text_tests {
    use super::*;
    use crate::datasource::datasource_diff::DatasourceDiffSummary;

    #[test]
    fn diff_identity_and_row_include_datasource_type() {
        let export_record = DatasourceImportRecord::from_generic_map(
            &serde_json::json!({
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "true",
                "orgId": "1"
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let live_record = DatasourceImportRecord::from_generic_map(
            &serde_json::json!({
                "id": 7,
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "loki",
                "access": "direct",
                "url": "http://loki:3100",
                "isDefault": false,
                "orgId": 1
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let entry = DatasourceDiffEntry {
            key: "uid:prom-main".to_string(),
            status: DatasourceDiffStatus::Different,
            export_record: Some(export_record),
            live_record: Some(live_record),
            differences: vec![],
        };

        let identity = render_diff_identity(&entry);
        let row = datasource_diff_row(&entry);

        assert!(identity.contains("uid=prom-main"));
        assert!(identity.contains("name=Prometheus Main"));
        assert!(identity.contains("type=prometheus"));
        assert_eq!(row["type"], serde_json::json!("prometheus"));
        assert_eq!(row["matchBasis"], serde_json::json!("uid"));
        assert_eq!(row["identity"], serde_json::json!(identity));
    }

    #[test]
    fn datasource_diff_summary_line_includes_source_context_and_status_breakdown() {
        let report = DatasourceDiffReport {
            entries: vec![],
            summary: DatasourceDiffSummary {
                compared_count: 4,
                matches_count: 1,
                different_count: 1,
                missing_in_live_count: 1,
                missing_in_export_count: 0,
                ambiguous_live_match_count: 1,
            },
        };

        let line = datasource_diff_summary_line(Path::new("/tmp/datasources"), &report);

        assert_eq!(
            line,
            "Diff checked 4 datasource(s) from /tmp/datasources against Grafana live datasources; 3 difference(s) found (same=1 different=1 missing-live=1 extra-live=0 ambiguous=1)."
        );
    }
}

#[cfg(test)]
#[path = "datasource_rust_tests.rs"]
mod datasource_rust_tests;

#[cfg(test)]
#[path = "datasource_diff_rust_tests.rs"]
mod datasource_diff_rust_tests;
