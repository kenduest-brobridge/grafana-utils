use std::path::Path;

use crate::common::{message, Result};

use super::super::cli_defs::{InspectExportArgs, InspectExportReportFormat, InspectOutputFormat};
use super::super::inspect_governance::build_export_inspection_governance_document;
use super::super::inspect_live::{prepare_inspect_export_import_dir, TempInspectDir};
use super::super::inspect_render::render_simple_table;
use super::super::inspect_report::{
    refresh_filtered_query_report_summary, report_format_supports_columns,
    resolve_report_column_ids_for_format, ExportInspectionQueryReport,
};
use super::super::inspect_summary::{
    build_export_inspection_summary_document, build_export_inspection_summary_rows,
    ExportInspectionSummary,
};
use super::super::inspect_workbench::run_inspect_workbench;
use super::super::inspect_workbench_support::build_inspect_workbench_document;
use super::inspect_output::render_export_inspection_report_output;
use super::inspect_query_report::build_export_inspection_query_report;
use super::{build_export_inspection_summary, write_inspect_output};

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

fn render_export_inspection_summary_output(
    args: &InspectExportArgs,
    summary: &ExportInspectionSummary,
) -> Result<String> {
    let mut output = String::new();
    if effective_inspect_json(args) {
        output.push_str(&format!(
            "{}\n",
            serde_json::to_string_pretty(&build_export_inspection_summary_document(summary))?
        ));
        return Ok(output);
    }

    if effective_inspect_table(args) {
        if !summary.import_dir.is_empty() {
            output.push_str(&format!(
                "Export inspection report: {}\n\n",
                summary.import_dir
            ));
        }
        output.push_str("# Overview\n");
        let summary_rows = build_export_inspection_summary_rows(summary);
        for line in render_simple_table(&["NAME", "VALUE"], &summary_rows, !args.no_header) {
            output.push_str(&line);
            output.push('\n');
        }
    } else {
        output.push_str(&format!(
            "Export inspection report: {}\n\n",
            summary.import_dir
        ));
        if let Some(export_org) = &summary.export_org {
            output.push_str(&format!("Export org: {}\n", export_org));
        }
        if let Some(export_org_id) = &summary.export_org_id {
            output.push_str(&format!("Export orgId: {}\n", export_org_id));
        }
        output.push_str(&format!("Dashboards: {}\n", summary.dashboard_count));
        output.push_str(&format!("Folders: {}\n", summary.folder_count));
        output.push_str(&format!("Panels: {}\n", summary.panel_count));
        output.push_str(&format!("Queries: {}\n", summary.query_count));
        output.push_str(&format!(
            "Datasource inventory: {}\n",
            summary.datasource_inventory_count
        ));
        output.push_str(&format!(
            "Orphaned datasources: {}\n",
            summary.orphaned_datasource_count
        ));
        output.push_str(&format!(
            "Mixed datasource dashboards: {}\n",
            summary.mixed_dashboard_count
        ));
    }

    output.push('\n');
    output.push_str("# Folder paths\n");
    let folder_rows = summary
        .folder_paths
        .iter()
        .map(|item| vec![item.path.clone(), item.dashboards.to_string()])
        .collect::<Vec<Vec<String>>>();
    for line in render_simple_table(
        &["FOLDER_PATH", "DASHBOARDS"],
        &folder_rows,
        !args.no_header,
    ) {
        output.push_str(&line);
        output.push('\n');
    }

    output.push('\n');
    output.push_str("# Datasource usage\n");
    let datasource_rows = summary
        .datasource_usage
        .iter()
        .map(|item| {
            vec![
                item.datasource.clone(),
                item.reference_count.to_string(),
                item.dashboard_count.to_string(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    for line in render_simple_table(
        &["DATASOURCE", "REFS", "DASHBOARDS"],
        &datasource_rows,
        !args.no_header,
    ) {
        output.push_str(&line);
        output.push('\n');
    }

    if !summary.datasource_inventory.is_empty() {
        output.push('\n');
        output.push_str("# Datasource inventory\n");
        let datasource_inventory_rows = summary
            .datasource_inventory
            .iter()
            .map(|item| {
                vec![
                    item.org_id.clone(),
                    item.uid.clone(),
                    item.name.clone(),
                    item.datasource_type.clone(),
                    item.access.clone(),
                    item.url.clone(),
                    item.is_default.clone(),
                    item.reference_count.to_string(),
                    item.dashboard_count.to_string(),
                ]
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(
            &[
                "ORG_ID",
                "UID",
                "NAME",
                "TYPE",
                "ACCESS",
                "URL",
                "IS_DEFAULT",
                "REFS",
                "DASHBOARDS",
            ],
            &datasource_inventory_rows,
            !args.no_header,
        ) {
            output.push_str(&line);
            output.push('\n');
        }
    }

    if !summary.orphaned_datasources.is_empty() {
        output.push('\n');
        output.push_str("# Orphaned datasources\n");
        let orphaned_rows = summary
            .orphaned_datasources
            .iter()
            .map(|item| {
                vec![
                    item.org_id.clone(),
                    item.uid.clone(),
                    item.name.clone(),
                    item.datasource_type.clone(),
                    item.access.clone(),
                    item.url.clone(),
                    item.is_default.clone(),
                ]
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(
            &[
                "ORG_ID",
                "UID",
                "NAME",
                "TYPE",
                "ACCESS",
                "URL",
                "IS_DEFAULT",
            ],
            &orphaned_rows,
            !args.no_header,
        ) {
            output.push_str(&line);
            output.push('\n');
        }
    }

    if !summary.mixed_dashboards.is_empty() {
        output.push('\n');
        output.push_str("# Mixed datasource dashboards\n");
        let mixed_rows = summary
            .mixed_dashboards
            .iter()
            .map(|item| {
                vec![
                    item.uid.clone(),
                    item.title.clone(),
                    item.folder_path.clone(),
                    item.datasources.join(","),
                ]
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(
            &["UID", "TITLE", "FOLDER_PATH", "DATASOURCES"],
            &mixed_rows,
            !args.no_header,
        ) {
            output.push_str(&line);
            output.push('\n');
        }
    }
    Ok(output)
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
    let summary = build_export_inspection_summary(import_dir)?;
    let report = build_export_inspection_query_report(import_dir)?;
    let governance = build_export_inspection_governance_document(&summary, &report);
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
