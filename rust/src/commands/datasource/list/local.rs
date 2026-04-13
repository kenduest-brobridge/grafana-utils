use serde_json::{Map, Value};

use crate::common::{message, print_supported_columns, Result};

use super::{
    build_datasource_inspect_export_browser_items, datasource_list_column_ids,
    load_datasource_inspect_export_source, render_datasource_inspect_export_output,
    resolve_datasource_inspect_export_input_format, DatasourceInspectExportRenderFormat,
    DatasourceListArgs,
};

#[cfg(any(feature = "tui", test))]
use crate::interactive_browser::run_interactive_browser;

pub(crate) fn render_datasource_text(
    records: &[Map<String, Value>],
    selected_columns: &[String],
) -> Vec<String> {
    records
        .iter()
        .map(|record| {
            super::render_data_source_summary_line(
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

// Local list mode executes without live API calls:
// resolve staged/local datasource artifacts and render output in the chosen format.
pub(crate) fn run_local_datasource_list(args: &DatasourceListArgs) -> Result<()> {
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
