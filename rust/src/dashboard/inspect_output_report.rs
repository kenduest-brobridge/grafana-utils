//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use std::path::Path;

use crate::common::{render_json_value, Result};
use crate::dashboard::cli_defs::{InspectExportArgs, InspectExportReportFormat};
use crate::dashboard::files::{load_datasource_inventory, load_export_metadata};
use crate::dashboard::inspect_dependency_render::render_export_inspection_dependency_table_report;
use crate::dashboard::inspect_governance::{
    build_export_inspection_governance_document, render_governance_table_report,
    ExportInspectionGovernanceDocument,
};
use crate::dashboard::inspect_render::{
    render_csv, render_grouped_query_report, render_grouped_query_table_report, render_simple_table,
};
use crate::dashboard::inspect_report::{
    build_export_inspection_query_report_document, render_query_report_column,
    report_column_header, resolve_report_column_ids_for_format, ExportInspectionQueryReport,
};
use crate::dashboard::inspect_summary::ExportInspectionSummary;
use crate::dashboard_inspection_dependency_contract::{
    build_offline_dependency_contract_document_from_report_rows,
    build_offline_dependency_contract_from_report_rows,
};

use super::super::build_export_inspection_summary_for_variant;
use super::{render_lines_to_string, ExportInspectionRenderedOutput};

fn render_export_inspection_governance_output(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report_format: InspectExportReportFormat,
) -> Result<ExportInspectionRenderedOutput> {
    let output = if report_format == InspectExportReportFormat::GovernanceJson {
        format!("{}\n", render_json_value(governance)?)
    } else {
        render_lines_to_string(render_governance_table_report(
            &summary.import_dir,
            governance,
        ))
    };
    Ok(ExportInspectionRenderedOutput {
        output,
        dashboard_count: summary.dashboard_count,
    })
}

fn render_export_inspection_column_report_output(
    args: &InspectExportArgs,
    report: &ExportInspectionQueryReport,
    report_format: InspectExportReportFormat,
) -> Result<ExportInspectionRenderedOutput> {
    let column_ids =
        resolve_report_column_ids_for_format(Some(report_format), &args.report_columns)?;
    let output = if report_format == InspectExportReportFormat::TreeTable {
        render_lines_to_string(render_grouped_query_table_report(
            report,
            &column_ids,
            !args.no_header,
        ))
    } else if report_format == InspectExportReportFormat::Csv {
        let headers = column_ids
            .iter()
            .map(|column_id| report_column_header(column_id))
            .collect::<Vec<&str>>();
        let rows = report
            .queries
            .iter()
            .map(|item| {
                column_ids
                    .iter()
                    .map(|column_id| render_query_report_column(item, column_id))
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<Vec<String>>>();
        render_lines_to_string(render_csv(&headers, &rows))
    } else {
        let rows = report
            .queries
            .iter()
            .map(|item| {
                column_ids
                    .iter()
                    .map(|column_id| render_query_report_column(item, column_id))
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<Vec<String>>>();
        let headers = column_ids
            .iter()
            .map(|column_id| report_column_header(column_id))
            .collect::<Vec<&str>>();
        let mut output = String::new();
        output.push_str(&format!(
            "Export inspection report: {}\n\n",
            report.import_dir
        ));
        output.push_str("# Query report\n");
        for line in render_simple_table(&headers, &rows, !args.no_header) {
            output.push_str(&line);
            output.push('\n');
        }
        output
    };
    Ok(ExportInspectionRenderedOutput {
        output,
        dashboard_count: report.summary.dashboard_count,
    })
}

fn render_export_inspection_tree_output(
    report: &ExportInspectionQueryReport,
) -> ExportInspectionRenderedOutput {
    ExportInspectionRenderedOutput {
        output: render_lines_to_string(render_grouped_query_report(report)),
        dashboard_count: report.summary.dashboard_count,
    }
}

fn render_export_inspection_json_output(
    report: &ExportInspectionQueryReport,
) -> Result<ExportInspectionRenderedOutput> {
    let document = build_export_inspection_query_report_document(report);
    Ok(ExportInspectionRenderedOutput {
        output: format!("{}\n", render_json_value(&document)?),
        dashboard_count: report.summary.dashboard_count,
    })
}

fn render_export_inspection_dependency_output(
    import_dir: &Path,
    expected_variant: &str,
    report_format: InspectExportReportFormat,
    report: &ExportInspectionQueryReport,
) -> Result<ExportInspectionRenderedOutput> {
    let metadata = load_export_metadata(import_dir, Some(expected_variant))?;
    let datasource_inventory = load_datasource_inventory(import_dir, metadata.as_ref())?;
    let output = if report_format == InspectExportReportFormat::DependencyJson {
        format!(
            "{}\n",
            render_json_value(&build_offline_dependency_contract_from_report_rows(
                &report.queries,
                &datasource_inventory,
            ))?
        )
    } else {
        let document = build_offline_dependency_contract_document_from_report_rows(
            &report.queries,
            &datasource_inventory,
        );
        render_lines_to_string(render_export_inspection_dependency_table_report(
            &report.import_dir,
            &document,
        ))
    };
    Ok(ExportInspectionRenderedOutput {
        output,
        dashboard_count: report.summary.dashboard_count,
    })
}

pub(crate) fn render_export_inspection_report_output(
    args: &InspectExportArgs,
    import_dir: &Path,
    expected_variant: &str,
    report_format: InspectExportReportFormat,
    report: &ExportInspectionQueryReport,
) -> Result<ExportInspectionRenderedOutput> {
    match report_format {
        InspectExportReportFormat::Governance | InspectExportReportFormat::GovernanceJson => {
            let summary =
                build_export_inspection_summary_for_variant(import_dir, expected_variant)?;
            let governance = build_export_inspection_governance_document(&summary, report);
            render_export_inspection_governance_output(&summary, &governance, report_format)
        }
        InspectExportReportFormat::Json => render_export_inspection_json_output(report),
        InspectExportReportFormat::Dependency | InspectExportReportFormat::DependencyJson => {
            render_export_inspection_dependency_output(
                import_dir,
                expected_variant,
                report_format,
                report,
            )
        }
        InspectExportReportFormat::Tree => Ok(render_export_inspection_tree_output(report)),
        InspectExportReportFormat::TreeTable
        | InspectExportReportFormat::Csv
        | InspectExportReportFormat::Table => {
            render_export_inspection_column_report_output(args, report, report_format)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::render_export_inspection_report_output;
    use crate::dashboard::cli_defs::{InspectExportArgs, InspectExportReportFormat};
    use crate::dashboard::files::build_export_metadata;
    use crate::dashboard::inspect_report::{ExportInspectionQueryReport, QueryReportSummary};
    use crate::dashboard::test_support::make_core_family_report_row;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    fn make_report(import_dir: &str) -> ExportInspectionQueryReport {
        ExportInspectionQueryReport {
            import_dir: import_dir.to_string(),
            summary: QueryReportSummary {
                dashboard_count: 1,
                panel_count: 1,
                query_count: 1,
                report_row_count: 1,
            },
            queries: vec![make_core_family_report_row(
                "cpu-main",
                "7",
                "A",
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "prometheus",
                "sum(rate(up[5m]))",
                &["job=\"api\""],
            )],
        }
    }

    #[test]
    fn render_export_inspection_report_output_renders_dependency_text_and_json_distinctly() {
        let temp = tempdir().unwrap();
        let import_dir = temp.path();
        let metadata = build_export_metadata(
            "raw",
            1,
            None,
            None,
            Some("datasources.json"),
            None,
            None,
            None,
            None,
        );
        fs::write(
            import_dir.join("export-metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap() + "\n",
        )
        .unwrap();
        fs::write(
            import_dir.join("datasources.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "database": "",
                    "defaultBucket": "",
                    "organization": "",
                    "indexPattern": "",
                    "isDefault": "true",
                    "org": "Main Org.",
                    "orgId": "1"
                },
                {
                    "uid": "unused-main",
                    "name": "Unused Main",
                    "type": "postgres",
                    "access": "proxy",
                    "url": "postgresql://postgres:5432/unused",
                    "database": "metrics",
                    "defaultBucket": "",
                    "organization": "",
                    "indexPattern": "",
                    "isDefault": "false",
                    "org": "Main Org.",
                    "orgId": "1"
                }
            ]))
            .unwrap()
                + "\n",
        )
        .unwrap();

        let args = InspectExportArgs {
            import_dir: import_dir.to_path_buf(),
            input_type: None,
            input_format: crate::dashboard::DashboardImportInputFormat::Raw,
            text: false,
            csv: false,
            json: false,
            table: false,
            yaml: false,
            report: Some(InspectExportReportFormat::Dependency),
            output_format: None,
            report_columns: Vec::new(),
            report_filter_datasource: None,
            report_filter_panel_id: None,
            help_full: false,
            no_header: false,
            output_file: None,
            also_stdout: false,
            interactive: false,
        };
        let report = make_report(&import_dir.display().to_string());

        let dependency_output = render_export_inspection_report_output(
            &args,
            import_dir,
            "raw",
            InspectExportReportFormat::Dependency,
            &report,
        )
        .unwrap();
        assert!(dependency_output
            .output
            .starts_with("Export inspection dependency: "));
        assert!(dependency_output.output.contains("# Datasource usage"));
        assert!(dependency_output
            .output
            .contains("# Dashboard dependencies"));
        assert!(dependency_output.output.contains("# Orphaned datasources"));
        assert!(dependency_output.output.contains("cpu-main"));
        assert!(dependency_output.output.contains("Prometheus Main"));
        assert!(dependency_output.output.contains("Unused Main"));
        assert!(!dependency_output.output.trim_start().starts_with('{'));

        let dependency_json_output = render_export_inspection_report_output(
            &args,
            import_dir,
            "raw",
            InspectExportReportFormat::DependencyJson,
            &report,
        )
        .unwrap();
        assert!(dependency_json_output.output.trim_start().starts_with('{'));
        assert!(dependency_json_output
            .output
            .contains("\"datasourceUid\": \"prom-main\""));
        assert!(dependency_json_output
            .output
            .contains("\"dashboardDependencies\""));
        assert!(dependency_json_output
            .output
            .contains("\"orphanedDatasources\""));
    }
}
