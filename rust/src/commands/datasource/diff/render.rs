use serde_json::{Map, Value};
use std::path::Path;

use crate::common::{
    build_shared_diff_document, render_json_value, string_field, DiffOutputFormat, Result,
    SharedDiffSummary,
};

use super::datasource_diff::{
    build_datasource_diff_report, normalize_export_records, normalize_live_records,
    DatasourceDiffEntry, DatasourceDiffReport, DatasourceDiffStatus,
};
use super::{load_diff_record_values, DatasourceImportInputFormat};

pub(crate) fn render_diff_identity(entry: &DatasourceDiffEntry) -> String {
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

pub(crate) fn resolve_delete_preview_type(
    target_id: Option<i64>,
    live: &[Map<String, Value>],
) -> String {
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

pub(crate) fn datasource_diff_summary_line(
    diff_dir: &Path,
    report: &DatasourceDiffReport,
) -> String {
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

pub(crate) fn datasource_diff_row(entry: &DatasourceDiffEntry) -> Value {
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
