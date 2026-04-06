//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use crate::common::{render_json_value, Result};
use crate::dashboard::cli_defs::{InspectExportArgs, InspectOutputFormat};
use crate::dashboard::inspect_render::{render_csv, render_simple_table};
use crate::dashboard::inspect_summary::{
    build_export_inspection_summary_document, build_export_inspection_summary_rows,
    DatasourceInventorySummary, ExportInspectionSummary, MixedDashboardSummary,
};
use crate::tabular_output::render_yaml;

#[path = "inspect_output_report.rs"]
mod inspect_output_report;

pub(crate) use inspect_output_report::render_export_inspection_report_output;

pub(crate) struct ExportInspectionRenderedOutput {
    pub(crate) output: String,
    pub(crate) dashboard_count: usize,
}

fn dashboard_inspect_export_summary_layer(format: InspectOutputFormat) -> &'static str {
    match format {
        InspectOutputFormat::Text | InspectOutputFormat::Table | InspectOutputFormat::Csv => {
            "operator-summary"
        }
        InspectOutputFormat::Json | InspectOutputFormat::Yaml => "full-contract",
        _ => unreachable!("summary output only uses baseline output formats"),
    }
}

pub(crate) fn render_lines_to_string(lines: Vec<String>) -> String {
    let mut output = String::new();
    for line in lines {
        output.push_str(&line);
        output.push('\n');
    }
    output
}

pub(crate) fn render_export_inspection_summary_output(
    args: &InspectExportArgs,
    summary: &ExportInspectionSummary,
) -> Result<String> {
    let mut output = String::new();
    let requested_output_format =
        super::inspect_orchestration::effective_inspect_output_format(args);

    match requested_output_format {
        InspectOutputFormat::Json => {
            output.push_str(&format!(
                "{}\n",
                render_json_value(&build_export_inspection_summary_document(summary))?
            ));
            return Ok(output);
        }
        InspectOutputFormat::Csv => {
            let summary_rows = build_export_inspection_summary_rows(summary);
            for line in render_csv(&["NAME", "VALUE"], &summary_rows) {
                output.push_str(&line);
                output.push('\n');
            }
            return Ok(output);
        }
        InspectOutputFormat::Yaml => {
            output.push_str(&format!(
                "{}\n",
                render_yaml(&build_export_inspection_summary_document(summary))?
            ));
            return Ok(output);
        }
        InspectOutputFormat::Table => {
            if !summary.import_dir.is_empty() {
                output.push_str(&format!(
                    "Dashboard analyze-export {}: {}\n\n",
                    dashboard_inspect_export_summary_layer(requested_output_format),
                    summary.import_dir
                ));
            }
            output.push_str(&format!(
                "Layer: {}\n",
                dashboard_inspect_export_summary_layer(requested_output_format)
            ));
            output.push('\n');
            output.push_str("# Overview\n");
            let summary_rows = build_export_inspection_summary_rows(summary);
            for line in render_simple_table(&["NAME", "VALUE"], &summary_rows, !args.no_header) {
                output.push_str(&line);
                output.push('\n');
            }
        }
        InspectOutputFormat::Text => {
            output.push_str(&format!(
                "Dashboard analyze-export {}: {}\n",
                dashboard_inspect_export_summary_layer(requested_output_format),
                summary.import_dir
            ));
            output.push_str(&format!(
                "Layer: {}\n\n",
                dashboard_inspect_export_summary_layer(requested_output_format)
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
        _ => unreachable!("report formats are handled earlier"),
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
            .map(render_datasource_inventory_row)
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
            .map(render_orphaned_datasource_row)
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
            .map(render_mixed_dashboard_row)
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

fn render_datasource_inventory_row(item: &DatasourceInventorySummary) -> Vec<String> {
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
}

fn render_orphaned_datasource_row(item: &DatasourceInventorySummary) -> Vec<String> {
    vec![
        item.org_id.clone(),
        item.uid.clone(),
        item.name.clone(),
        item.datasource_type.clone(),
        item.access.clone(),
        item.url.clone(),
        item.is_default.clone(),
    ]
}

fn render_mixed_dashboard_row(item: &MixedDashboardSummary) -> Vec<String> {
    vec![
        item.uid.clone(),
        item.title.clone(),
        item.folder_path.clone(),
        item.datasources.join(","),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::cli_defs::InspectOutputFormat;
    use crate::dashboard::inspect_summary::{ExportDatasourceUsage, ExportFolderUsage};
    use std::path::PathBuf;

    fn make_summary() -> ExportInspectionSummary {
        ExportInspectionSummary {
            import_dir: "/tmp/demo".to_string(),
            export_org: Some("Main Org.".to_string()),
            export_org_id: Some("1".to_string()),
            dashboard_count: 2,
            folder_count: 1,
            panel_count: 4,
            query_count: 5,
            datasource_inventory_count: 2,
            orphaned_datasource_count: 1,
            mixed_dashboard_count: 1,
            folder_paths: vec![ExportFolderUsage {
                path: "ops/platform".to_string(),
                dashboards: 2,
            }],
            datasource_usage: vec![ExportDatasourceUsage {
                datasource: "Prometheus Main".to_string(),
                reference_count: 5,
                dashboard_count: 2,
            }],
            datasource_inventory: vec![DatasourceInventorySummary {
                org_id: "1".to_string(),
                org: "Main Org.".to_string(),
                uid: "prom-main".to_string(),
                name: "Prometheus Main".to_string(),
                datasource_type: "prometheus".to_string(),
                access: "proxy".to_string(),
                url: "http://prometheus:9090".to_string(),
                is_default: "true".to_string(),
                reference_count: 5,
                dashboard_count: 2,
            }],
            orphaned_datasources: vec![DatasourceInventorySummary {
                org_id: "1".to_string(),
                org: "Main Org.".to_string(),
                uid: "unused-main".to_string(),
                name: "Unused Main".to_string(),
                datasource_type: "postgres".to_string(),
                access: "proxy".to_string(),
                url: "postgresql://postgres:5432/unused".to_string(),
                is_default: "false".to_string(),
                reference_count: 0,
                dashboard_count: 0,
            }],
            mixed_dashboards: vec![MixedDashboardSummary {
                uid: "cpu-main".to_string(),
                title: "CPU Main".to_string(),
                folder_path: "ops/platform".to_string(),
                datasource_count: 2,
                datasources: vec!["Prometheus Main".to_string(), "Loki Main".to_string()],
            }],
        }
    }

    #[test]
    fn render_export_inspection_summary_output_renders_inventory_orphans_and_mixed_dashboards() {
        let args = InspectExportArgs {
            import_dir: PathBuf::from("/tmp/demo"),
            input_type: None,
            input_format: crate::dashboard::DashboardImportInputFormat::Raw,
            text: false,
            csv: false,
            json: false,
            table: false,
            yaml: false,
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

        let output = render_export_inspection_summary_output(&args, &make_summary()).unwrap();

        assert!(output.starts_with("Dashboard analyze-export operator-summary: /tmp/demo"));
        assert!(output.contains("Layer: operator-summary"));
        assert!(output.contains("# Datasource inventory"));
        assert!(output.contains("# Orphaned datasources"));
        assert!(output.contains("# Mixed datasource dashboards"));
        assert!(output.contains("Unused Main"));
        assert!(output.contains("Prometheus Main,Loki Main"));
    }

    #[test]
    fn render_export_inspection_summary_output_honors_table_mode() {
        let args = InspectExportArgs {
            import_dir: PathBuf::from("/tmp/demo"),
            input_type: None,
            input_format: crate::dashboard::DashboardImportInputFormat::Raw,
            text: false,
            csv: false,
            json: false,
            table: false,
            yaml: false,
                        output_format: Some(InspectOutputFormat::Table),
            report_columns: Vec::new(),
            report_filter_datasource: None,
            report_filter_panel_id: None,
            help_full: false,
            no_header: false,
            output_file: None,
            also_stdout: false,
            interactive: false,
        };

        let output = render_export_inspection_summary_output(&args, &make_summary()).unwrap();

        assert!(output.contains("Layer: operator-summary"));
        assert!(output.contains("# Overview"));
        assert!(!output.contains("Dashboards: 2\n"));
        assert!(output.contains("NAME"));
        assert!(output.contains("VALUE"));
    }

    #[test]
    fn render_export_inspection_summary_output_honors_csv_and_yaml_modes() {
        let csv_args = InspectExportArgs {
            import_dir: PathBuf::from("/tmp/demo"),
            input_type: None,
            input_format: crate::dashboard::DashboardImportInputFormat::Raw,
            text: false,
            csv: false,
            json: false,
            table: false,
            yaml: false,
                        output_format: Some(InspectOutputFormat::Csv),
            report_columns: Vec::new(),
            report_filter_datasource: None,
            report_filter_panel_id: None,
            help_full: false,
            no_header: false,
            output_file: None,
            also_stdout: false,
            interactive: false,
        };
        let yaml_args = InspectExportArgs {
            output_format: Some(InspectOutputFormat::Yaml),
            ..csv_args.clone()
        };

        let csv_output =
            render_export_inspection_summary_output(&csv_args, &make_summary()).unwrap();
        assert!(csv_output.starts_with("NAME,VALUE"));
        assert!(csv_output.contains("dashboard_count,2"));
        assert!(csv_output.contains("mixed_datasource_dashboard_count,1"));

        let yaml_output =
            render_export_inspection_summary_output(&yaml_args, &make_summary()).unwrap();
        assert!(
            yaml_output.contains("dashboardCount: 2") || yaml_output.contains("dashboard_count: 2")
        );
        assert!(
            yaml_output.contains("mixedDatasourceDashboardCount: 1")
                || yaml_output.contains("mixed_datasource_dashboard_count: 1")
        );
        assert!(
            yaml_output.contains("datasourceInventory:")
                || yaml_output.contains("datasource_inventory:")
        );
    }
}
