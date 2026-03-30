use std::path::Path;

use crate::common::{message, Result};

use super::super::cli_defs::{InspectExportArgs, InspectExportReportFormat, InspectOutputFormat};
use super::super::inspect_governance::build_export_inspection_governance_document;
use super::super::inspect_live::{prepare_inspect_export_import_dir, TempInspectDir};
use super::super::inspect_report::{
    refresh_filtered_query_report_summary, report_format_supports_columns,
    resolve_report_column_ids_for_format, ExportInspectionQueryReport,
};
use super::super::inspect_workbench::run_inspect_workbench;
use super::super::inspect_workbench_support::build_inspect_workbench_document;
use super::inspect_output::{
    render_export_inspection_report_output, render_export_inspection_summary_output,
};
use super::inspect_query_report::build_export_inspection_query_report;
use super::{build_export_inspection_summary, write_inspect_output};

#[cfg(feature = "tui")]
fn emit_interactive_inspect_progress(step: &str, import_dir: &Path) {
    eprintln!(
        "[inspect-export --interactive] {step}: {}",
        import_dir.display()
    );
}

fn map_output_format_to_report(
    output_format: InspectOutputFormat,
) -> Option<InspectExportReportFormat> {
    match output_format {
        InspectOutputFormat::Text | InspectOutputFormat::Table | InspectOutputFormat::Json => None,
        InspectOutputFormat::ReportTable => Some(InspectExportReportFormat::Table),
        InspectOutputFormat::ReportCsv => Some(InspectExportReportFormat::Csv),
        InspectOutputFormat::ReportJson => Some(InspectExportReportFormat::Json),
        InspectOutputFormat::ReportTree => Some(InspectExportReportFormat::Tree),
        InspectOutputFormat::ReportTreeTable => Some(InspectExportReportFormat::TreeTable),
        InspectOutputFormat::ReportDependency => Some(InspectExportReportFormat::Dependency),
        InspectOutputFormat::ReportDependencyJson => {
            Some(InspectExportReportFormat::DependencyJson)
        }
        InspectOutputFormat::Governance => Some(InspectExportReportFormat::Governance),
        InspectOutputFormat::GovernanceJson => Some(InspectExportReportFormat::GovernanceJson),
    }
}

pub(crate) fn effective_inspect_report_format(
    args: &InspectExportArgs,
) -> Option<InspectExportReportFormat> {
    args.report
        .or_else(|| args.output_format.and_then(map_output_format_to_report))
}

pub(crate) fn effective_inspect_json(args: &InspectExportArgs) -> bool {
    args.json || matches!(args.output_format, Some(InspectOutputFormat::Json))
}

pub(crate) fn effective_inspect_table(args: &InspectExportArgs) -> bool {
    args.table || matches!(args.output_format, Some(InspectOutputFormat::Table))
}

pub(crate) fn apply_query_report_filters(
    mut report: ExportInspectionQueryReport,
    datasource_filter: Option<&str>,
    panel_id_filter: Option<&str>,
) -> ExportInspectionQueryReport {
    let datasource_filter = datasource_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let panel_id_filter = panel_id_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if datasource_filter.is_none() && panel_id_filter.is_none() {
        return report;
    }
    report.queries.retain(|row| {
        let datasource_match = datasource_filter
            .map(|value| {
                row.datasource == value
                    || row.datasource_uid == value
                    || row.datasource_type == value
                    || row.datasource_family == value
            })
            .unwrap_or(true);
        let panel_match = panel_id_filter
            .map(|value| row.panel_id == value)
            .unwrap_or(true);
        datasource_match && panel_match
    });
    refresh_filtered_query_report_summary(&mut report);
    report
}

pub(crate) fn validate_inspect_export_report_args(args: &InspectExportArgs) -> Result<()> {
    let report_format = effective_inspect_report_format(args);
    if report_format.is_none() {
        if !args.report_columns.is_empty() {
            return Err(message(
                "--report-columns is only supported together with --report or report-like --output-format.",
            ));
        }
        if args.report_filter_datasource.is_some() {
            return Err(message(
                "--report-filter-datasource is only supported together with --report or report-like --output-format.",
            ));
        }
        if args.report_filter_panel_id.is_some() {
            return Err(message(
                "--report-filter-panel-id is only supported together with --report or report-like --output-format.",
            ));
        }
        return Ok(());
    }
    if report_format
        .map(|format| {
            matches!(
                format,
                InspectExportReportFormat::Governance | InspectExportReportFormat::GovernanceJson
            )
        })
        .unwrap_or(false)
        && !args.report_columns.is_empty()
    {
        return Err(message(
            "--report-columns is not supported with governance output.",
        ));
    }
    if report_format
        .map(|format| !report_format_supports_columns(format))
        .unwrap_or(false)
        && !args.report_columns.is_empty()
    {
        return Err(message(
            "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes.",
        ));
    }
    let _ = resolve_report_column_ids_for_format(report_format, &args.report_columns)?;
    Ok(())
}

fn analyze_export_dir_at_path(args: &InspectExportArgs, import_dir: &Path) -> Result<usize> {
    if args.interactive {
        return run_interactive_export_workbench(import_dir);
    }
    let write_output =
        |output: &str| -> Result<()> { write_inspect_output(output, args.output_file.as_ref()) };

    if let Some(report_format) = effective_inspect_report_format(args) {
        let report = apply_query_report_filters(
            build_export_inspection_query_report(import_dir)?,
            args.report_filter_datasource.as_deref(),
            args.report_filter_panel_id.as_deref(),
        );
        let rendered =
            render_export_inspection_report_output(args, import_dir, report_format, &report)?;
        write_output(&rendered.output)?;
        return Ok(rendered.dashboard_count);
    }

    let summary = build_export_inspection_summary(import_dir)?;
    let output = render_export_inspection_summary_output(args, &summary)?;
    write_output(&output)?;
    Ok(summary.dashboard_count)
}

#[cfg(feature = "tui")]
fn run_interactive_export_workbench(import_dir: &Path) -> Result<usize> {
    emit_interactive_inspect_progress("building summary", import_dir);
    let summary = build_export_inspection_summary(import_dir)?;
    emit_interactive_inspect_progress("building query report", import_dir);
    let report = build_export_inspection_query_report(import_dir)?;
    emit_interactive_inspect_progress("building governance review", import_dir);
    let governance = build_export_inspection_governance_document(&summary, &report);
    emit_interactive_inspect_progress("launching inspect workbench", import_dir);
    let document =
        build_inspect_workbench_document("export artifacts", &summary, &governance, &report);
    run_inspect_workbench(document)?;
    Ok(summary.dashboard_count)
}

#[cfg(not(feature = "tui"))]
fn run_interactive_export_workbench(_import_dir: &Path) -> Result<usize> {
    super::tui_not_built("inspect-export --interactive")
}

pub(crate) fn analyze_export_dir(args: &InspectExportArgs) -> Result<usize> {
    validate_inspect_export_report_args(args)?;
    let temp_dir = TempInspectDir::new("inspect-export")?;
    let import_dir = prepare_inspect_export_import_dir(&temp_dir.path, &args.import_dir)?;
    analyze_export_dir_at_path(args, &import_dir)
}
