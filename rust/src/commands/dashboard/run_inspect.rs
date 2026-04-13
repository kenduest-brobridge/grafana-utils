//! Reusable dashboard inspect execution surface.
use crate::common::Result;
use std::path::Path;

use super::export;
use super::inspect;
use super::inspect_live;
use super::{
    DashboardImportInputFormat, ExportArgs, InspectExportArgs, InspectLiveArgs, InspectVarsArgs,
};

fn rendered_output_to_lines(output: String) -> Vec<String> {
    output
        .trim_end_matches('\n')
        .split('\n')
        .map(str::to_string)
        .collect()
}

// Inspect path dispatcher:
// validate args, build selected report/summary variants, and return a shared web output.
fn execute_dashboard_inspect_at_path(
    args: &InspectExportArgs,
    input_dir: &Path,
    expected_variant: &str,
) -> Result<super::DashboardWebRunOutput> {
    inspect::validate_inspect_export_report_args(args)?;
    if let Some(report_format) = inspect::effective_inspect_report_format(args) {
        let report = inspect::apply_query_report_filters(
            inspect::build_export_inspection_query_report_for_variant(input_dir, expected_variant)?,
            args.report_filter_datasource.as_deref(),
            args.report_filter_panel_id.as_deref(),
        );
        let rendered = inspect::render_export_inspection_report_output(
            args,
            input_dir,
            expected_variant,
            report_format,
            &report,
        )?;
        let document = match report_format {
            super::InspectExportReportFormat::Governance
            | super::InspectExportReportFormat::GovernanceJson => {
                let summary = inspect::build_export_inspection_summary_for_variant(
                    input_dir,
                    expected_variant,
                )?;
                serde_json::to_value(
                    super::inspect_governance::build_export_inspection_governance_document(
                        &summary, &report,
                    ),
                )?
            }
            super::InspectExportReportFormat::Dependency
            | super::InspectExportReportFormat::DependencyJson => {
                let metadata = super::load_export_metadata(input_dir, Some(expected_variant))?;
                let datasource_inventory =
                    super::load_datasource_inventory(input_dir, metadata.as_ref())?;
                crate::dashboard_inspection_dependency_contract::build_offline_dependency_contract_from_report_rows(
                    &report.queries,
                    &datasource_inventory,
                )
            }
            super::InspectExportReportFormat::QueriesJson
            | super::InspectExportReportFormat::Tree
            | super::InspectExportReportFormat::TreeTable
            | super::InspectExportReportFormat::Csv
            | super::InspectExportReportFormat::Table => serde_json::to_value(
                super::inspect_report::build_export_inspection_query_report_document(&report),
            )?,
        };
        return Ok(super::DashboardWebRunOutput {
            document,
            text_lines: rendered_output_to_lines(rendered.output),
        });
    }

    let summary =
        inspect::build_export_inspection_summary_for_variant(input_dir, expected_variant)?;
    let rendered = inspect::render_export_inspection_summary_output(args, &summary)?;
    Ok(super::DashboardWebRunOutput {
        document: serde_json::to_value(super::build_export_inspection_summary_document(&summary))?,
        text_lines: rendered_output_to_lines(rendered),
    })
}

// Export-backed inspect path: materialize input dir, normalize output variant, then reuse the
// shared `execute_dashboard_inspect_at_path` output path.
pub fn execute_dashboard_inspect_export(
    args: &InspectExportArgs,
) -> Result<super::DashboardWebRunOutput> {
    let temp_dir = inspect_live::TempInspectDir::new("summary-export-web")?;
    let input_dir = inspect::resolve_inspect_export_import_dir(
        &temp_dir.path,
        &args.input_dir,
        args.input_format,
        args.input_type,
        args.interactive,
    )?;
    execute_dashboard_inspect_at_path(args, &input_dir.input_dir, input_dir.expected_variant)
}

// Live inspect path: fetch dashboards into a temp export dir and convert into export-style input.
pub fn execute_dashboard_inspect_live(
    args: &InspectLiveArgs,
) -> Result<super::DashboardWebRunOutput> {
    let temp_dir = inspect_live::TempInspectDir::new("summary-live-web")?;
    let export_args = ExportArgs {
        common: args.common.clone(),
        output_dir: temp_dir.path.clone(),
        page_size: args.page_size,
        org_id: args.org_id,
        all_orgs: args.all_orgs,
        flat: false,
        overwrite: false,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        without_dashboard_provisioning: true,
        include_history: false,
        provisioning_provider_name: "grafana-utils-dashboards".to_string(),
        provisioning_provider_org_id: None,
        provisioning_provider_path: None,
        provisioning_provider_disable_deletion: false,
        provisioning_provider_allow_ui_updates: false,
        provisioning_provider_update_interval_seconds: 30,
        dry_run: false,
        progress: args.progress,
        verbose: false,
    };
    let _ = export::export_dashboards_with_org_clients(&export_args)?;
    let inspect_import_dir = inspect_live::prepare_inspect_live_import_dir(&temp_dir.path, args)?;
    let inspect_args = InspectExportArgs {
        input_dir: inspect_import_dir,
        input_type: None,
        input_format: DashboardImportInputFormat::Raw,
        text: args.text,
        csv: args.csv,
        json: args.json,
        table: args.table,
        yaml: args.yaml,
        output_format: args.output_format,
        report_columns: args.report_columns.clone(),
        list_columns: args.list_columns,
        report_filter_datasource: args.report_filter_datasource.clone(),
        report_filter_panel_id: args.report_filter_panel_id.clone(),
        help_full: args.help_full,
        no_header: args.no_header,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };
    execute_dashboard_inspect_at_path(
        &inspect_args,
        &inspect_args.input_dir,
        super::RAW_EXPORT_SUBDIR,
    )
}

// Variable-inspection execution path: render variable diagnostics into shared run output shape.
pub fn execute_dashboard_inspect_vars(
    args: &InspectVarsArgs,
) -> Result<super::DashboardWebRunOutput> {
    let document = super::vars::execute_dashboard_variable_inspection(args)?;
    let rendered = super::vars::render_dashboard_variable_output(args, &document)?;
    Ok(super::DashboardWebRunOutput {
        document: serde_json::to_value(document)?,
        text_lines: rendered_output_to_lines(rendered),
    })
}
