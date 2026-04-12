//! Dashboard CLI execution and orchestration.
use crate::common::{
    message, print_supported_columns, render_json_value, set_json_color_choice, Result,
};
use crate::http::JsonHttpClient;
use crate::tabular_output::render_yaml;
use serde_json::{json, Map, Value};
use std::path::Path;

use super::browse;
use super::delete;
use super::edit_live::run_dashboard_edit_live;
use super::export;
use super::history::{
    export_dashboard_history_with_request, run_dashboard_history_diff, run_dashboard_history_list,
    run_dashboard_history_restore,
};
use super::import;
use super::inspect;
use super::inspect_live;
use super::inspect_report::SUPPORTED_REPORT_COLUMN_IDS;
use super::list;
use super::screenshot::capture_dashboard_screenshot;
use super::serve::run_dashboard_serve;
use super::topology::{run_dashboard_impact, run_dashboard_topology};
use super::validate::run_dashboard_validate_export;
use super::vars::inspect_dashboard_variables;
#[allow(unused_imports)]
use super::{
    build_api_client, build_dashboard_review, build_http_client, build_http_client_for_org,
    build_http_client_for_org_from_api, materialize_dashboard_common_auth,
    render_inspect_export_help_full, render_inspect_live_help_full, AnalyzeArgs, DashboardCliArgs,
    DashboardCommand, DashboardHistorySubcommand, DashboardImportInputFormat, ExportArgs,
    InspectExportArgs, InspectLiveArgs, InspectVarsArgs, ListArgs, ReviewArgs, SimpleOutputFormat,
};

const DASHBOARD_LIST_OUTPUT_COLUMNS: &[&str] = &[
    "uid",
    "name",
    "folder",
    "folder_uid",
    "path",
    "org",
    "org_id",
    "sources",
    "source_uids",
];

const DASHBOARD_IMPORT_OUTPUT_COLUMNS: &[&str] = &[
    "uid",
    "destination",
    "action",
    "folder_path",
    "source_folder_path",
    "destination_folder_path",
    "reason",
    "file",
];

fn print_supported_dashboard_report_columns() {
    print_supported_columns(SUPPORTED_REPORT_COLUMN_IDS);
}

fn rendered_output_to_lines(output: String) -> Vec<String> {
    output
        .trim_end_matches('\n')
        .split('\n')
        .map(str::to_string)
        .collect()
}

pub(crate) fn collect_dashboard_list_summaries(args: &ListArgs) -> Result<Vec<Map<String, Value>>> {
    let mut summaries = Vec::new();
    if args.all_orgs {
        let admin_api = build_api_client(&args.common)?;
        let admin_client = admin_api.http_client();
        let orgs = list::list_orgs_with_request(|method, path, params, payload| {
            admin_client.request_json(method, path, params, payload)
        })?;
        for org in orgs {
            let org_id = list::org_id_value(&org)?;
            let org_client = build_http_client_for_org_from_api(&admin_api, org_id)?;
            let mut scoped = list::collect_list_dashboards_with_request(
                &mut |method, path, params, payload| {
                    org_client.request_json(method, path, params, payload)
                },
                args,
                Some(&org),
                None,
            )?;
            summaries.append(&mut scoped);
        }
        return Ok(summaries);
    }
    if let Some(org_id) = args.org_id {
        let org_client = build_http_client_for_org(&args.common, org_id)?;
        return list::collect_list_dashboards_with_request(
            &mut |method, path, params, payload| {
                org_client.request_json(method, path, params, payload)
            },
            args,
            None,
            None,
        );
    }
    let client = build_http_client(&args.common)?;
    list::collect_list_dashboards_with_request(
        &mut |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
        None,
        None,
    )
}

// Build a single dashboard list output document used by reusable execution callers.
pub fn execute_dashboard_list(args: &ListArgs) -> Result<super::DashboardWebRunOutput> {
    let summaries = collect_dashboard_list_summaries(args)?;
    let rows = list::render_dashboard_summary_json(&summaries, &args.output_columns);
    let text_lines = if args.json {
        rendered_output_to_lines(render_json_value(&rows)?)
    } else if args.yaml {
        rendered_output_to_lines(render_yaml(&rows)?)
    } else if args.csv {
        list::render_dashboard_summary_csv(&summaries, &args.output_columns)
    } else if args.text {
        let mut lines = summaries
            .iter()
            .map(list::format_dashboard_summary_line)
            .collect::<Vec<String>>();
        lines.push(String::new());
        lines.push(format!("Listed {} dashboard(s).", summaries.len()));
        lines
    } else {
        let mut lines =
            list::render_dashboard_summary_table(&summaries, &args.output_columns, !args.no_header);
        lines.push(String::new());
        lines.push(format!("Listed {} dashboard(s).", summaries.len()));
        lines
    };
    Ok(super::DashboardWebRunOutput {
        document: json!({
            "kind": "grafana-utils-dashboard-list",
            "dashboardCount": summaries.len(),
            "rows": rows,
        }),
        text_lines,
    })
}

fn analyze_args_to_export_args(args: AnalyzeArgs) -> Result<InspectExportArgs> {
    let input_dir = args
        .input_dir
        .ok_or_else(|| message("dashboard summary local mode requires --input-dir."))?;
    Ok(InspectExportArgs {
        input_dir,
        input_type: args.input_type,
        input_format: args.input_format,
        text: args.text,
        table: args.table,
        csv: args.csv,
        json: args.json,
        yaml: args.yaml,
        output_format: args.output_format,
        report_columns: args.report_columns,
        list_columns: args.list_columns,
        report_filter_datasource: args.report_filter_datasource,
        report_filter_panel_id: args.report_filter_panel_id,
        help_full: args.help_full,
        no_header: args.no_header,
        output_file: args.output_file,
        also_stdout: args.also_stdout,
        interactive: args.interactive,
    })
}

fn analyze_args_to_live_args(args: AnalyzeArgs) -> InspectLiveArgs {
    InspectLiveArgs {
        common: args.common,
        page_size: args.page_size,
        concurrency: args.concurrency,
        org_id: args.org_id,
        all_orgs: args.all_orgs,
        text: args.text,
        table: args.table,
        csv: args.csv,
        json: args.json,
        yaml: args.yaml,
        output_format: args.output_format,
        report_columns: args.report_columns,
        list_columns: args.list_columns,
        report_filter_datasource: args.report_filter_datasource,
        report_filter_panel_id: args.report_filter_panel_id,
        progress: args.progress,
        help_full: args.help_full,
        no_header: args.no_header,
        output_file: args.output_file,
        also_stdout: args.also_stdout,
        interactive: args.interactive,
    }
}

fn request_json_with_client(
    client: &JsonHttpClient,
    method: reqwest::Method,
    path: &str,
    params: &[(String, String)],
    payload: Option<&Value>,
) -> Result<Option<Value>> {
    client.request_json(method, path, params, payload)
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

pub(crate) fn review_dashboard_file(args: &ReviewArgs) -> Result<()> {
    let review = build_dashboard_review(&args.input)?;
    let output_format = args.output_format.unwrap_or({
        if args.json {
            SimpleOutputFormat::Json
        } else if args.table {
            SimpleOutputFormat::Table
        } else if args.csv {
            SimpleOutputFormat::Csv
        } else if args.yaml {
            SimpleOutputFormat::Yaml
        } else {
            SimpleOutputFormat::Text
        }
    });
    match output_format {
        SimpleOutputFormat::Text => {
            println!(
                "{}",
                super::render_dashboard_review_text(&review).join("\n")
            );
        }
        SimpleOutputFormat::Table => {
            for line in super::render_dashboard_review_table(&review) {
                println!("{line}");
            }
        }
        SimpleOutputFormat::Csv => {
            for line in super::render_dashboard_review_csv(&review) {
                println!("{line}");
            }
        }
        SimpleOutputFormat::Json => {
            print!("{}", super::render_dashboard_review_json(&review)?);
        }
        SimpleOutputFormat::Yaml => {
            print!("{}", super::render_dashboard_review_yaml(&review)?);
        }
    }
    Ok(())
}

/// Run the dashboard CLI with an already configured client.
/// This is the narrow execution path for callers that already resolved auth/client setup.
pub fn run_dashboard_cli_with_client(
    client: &JsonHttpClient,
    args: DashboardCliArgs,
) -> Result<()> {
    match args.command {
        DashboardCommand::Browse(browse_args) => {
            let _ = browse::browse_dashboards_with_client(client, &browse_args)?;
            Ok(())
        }
        DashboardCommand::List(list_args) => {
            if list_args.list_columns {
                print_supported_columns(DASHBOARD_LIST_OUTPUT_COLUMNS);
                return Ok(());
            }
            let _ = list::list_dashboards_with_client(client, &list_args)?;
            Ok(())
        }
        DashboardCommand::Export(export_args) => {
            let _ = export::export_dashboards_with_client(client, &export_args)?;
            Ok(())
        }
        DashboardCommand::Get(get_args) => {
            super::get_live_dashboard_to_file_with_client(client, &get_args)
        }
        DashboardCommand::CloneLive(clone_args) => {
            super::clone_live_dashboard_to_file_with_client(client, &clone_args)
        }
        DashboardCommand::Serve(serve_args) => run_dashboard_serve(&serve_args),
        DashboardCommand::EditLive(edit_live_args) => {
            run_dashboard_edit_live(Some(client), &edit_live_args)
        }
        DashboardCommand::Import(import_args) => {
            if import_args.list_columns {
                print_supported_columns(DASHBOARD_IMPORT_OUTPUT_COLUMNS);
                return Ok(());
            }
            let _ = import::import_dashboards_with_client(client, &import_args)?;
            Ok(())
        }
        DashboardCommand::PatchFile(patch_args) => super::patch_dashboard_file(&patch_args),
        DashboardCommand::Review(review_args) => review_dashboard_file(&review_args),
        DashboardCommand::Publish(publish_args) => {
            super::publish_dashboard_with_client(client, &publish_args)
        }
        DashboardCommand::Analyze(analyze_args) => {
            if analyze_args.input_dir.is_some() {
                let inspect_args = analyze_args_to_export_args(analyze_args)?;
                if inspect_args.list_columns {
                    print_supported_dashboard_report_columns();
                    return Ok(());
                }
                if inspect_args.help_full {
                    print!("{}", render_inspect_export_help_full());
                    return Ok(());
                }
                let _ = inspect::analyze_export_dir(&inspect_args)?;
                Ok(())
            } else {
                let inspect_args = analyze_args_to_live_args(analyze_args);
                if inspect_args.list_columns {
                    print_supported_dashboard_report_columns();
                    return Ok(());
                }
                if inspect_args.help_full {
                    print!("{}", render_inspect_live_help_full());
                    return Ok(());
                }
                let _ = inspect_live::inspect_live_dashboards_with_client(client, &inspect_args)?;
                Ok(())
            }
        }
        DashboardCommand::Delete(delete_args) => {
            let _ = delete::delete_dashboards_with_client(client, &delete_args)?;
            Ok(())
        }
        DashboardCommand::Diff(diff_args) => {
            let differences = super::diff_dashboards_with_client(client, &diff_args)?;
            if differences > 0 {
                return Err(message(format!(
                    "Dashboard diff found {} differing item(s).",
                    differences
                )));
            }
            Ok(())
        }
        DashboardCommand::InspectExport(inspect_args) => {
            if inspect_args.list_columns {
                print_supported_dashboard_report_columns();
                return Ok(());
            }
            if inspect_args.help_full {
                print!("{}", render_inspect_export_help_full());
                return Ok(());
            }
            let _ = inspect::analyze_export_dir(&inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectLive(inspect_args) => {
            if inspect_args.list_columns {
                print_supported_dashboard_report_columns();
                return Ok(());
            }
            if inspect_args.help_full {
                print!("{}", render_inspect_live_help_full());
                return Ok(());
            }
            let _ = inspect_live::inspect_live_dashboards_with_client(client, &inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectVars(inspect_vars_args) => {
            inspect_dashboard_variables(&inspect_vars_args)
        }
        DashboardCommand::GovernanceGate(governance_gate_args) => {
            super::governance_gate::run_dashboard_governance_gate(&governance_gate_args)
        }
        DashboardCommand::Topology(topology_args) => run_dashboard_topology(&topology_args),
        DashboardCommand::Impact(impact_args) => run_dashboard_impact(&impact_args),
        DashboardCommand::History(history_args) => match history_args.command {
            DashboardHistorySubcommand::List(list_args) => run_dashboard_history_list(
                |method, path, params, payload| {
                    request_json_with_client(client, method, path, params, payload)
                },
                &list_args,
            ),
            DashboardHistorySubcommand::Diff(diff_args) => run_dashboard_history_diff(
                |method, path, params, payload| {
                    request_json_with_client(client, method, path, params, payload)
                },
                &diff_args,
            )
            .map(|_| ()),
            DashboardHistorySubcommand::Restore(restore_args) => run_dashboard_history_restore(
                |method, path, params, payload| {
                    request_json_with_client(client, method, path, params, payload)
                },
                &restore_args,
            ),
            DashboardHistorySubcommand::Export(export_args) => {
                export_dashboard_history_with_request(
                    |method, path, params, payload| {
                        request_json_with_client(client, method, path, params, payload)
                    },
                    &export_args,
                )
            }
        },
        DashboardCommand::ValidateExport(validate_args) => {
            run_dashboard_validate_export(&validate_args)
        }
        DashboardCommand::Screenshot(screenshot_args) => {
            capture_dashboard_screenshot(&screenshot_args)
        }
    }
}

/// Run the dashboard CLI after normalizing args and creating clients as needed.
/// This is the top-level dashboard runtime boundary for the Rust CLI surface.
pub fn run_dashboard_cli(args: DashboardCliArgs) -> Result<()> {
    set_json_color_choice(args.color);
    let mut args = super::normalize_dashboard_cli_args(args);
    match &args.command {
        DashboardCommand::List(list_args) if list_args.list_columns => {
            print_supported_columns(DASHBOARD_LIST_OUTPUT_COLUMNS);
            return Ok(());
        }
        DashboardCommand::Import(import_args) if import_args.list_columns => {
            print_supported_columns(DASHBOARD_IMPORT_OUTPUT_COLUMNS);
            return Ok(());
        }
        _ => {}
    }
    materialize_dashboard_command_auth(&mut args)?;
    match args.command {
        DashboardCommand::Browse(browse_args) => {
            let _ = browse::browse_dashboards_with_org_client(&browse_args)?;
            Ok(())
        }
        DashboardCommand::List(list_args) => {
            let _ = list::list_dashboards_with_org_clients(&list_args)?;
            Ok(())
        }
        DashboardCommand::Export(export_args) => {
            if export_args.without_dashboard_raw
                && export_args.without_dashboard_prompt
                && export_args.without_dashboard_provisioning
            {
                return Err(message(
                    "At least one export variant must stay enabled. Remove --without-raw, --without-prompt, or --without-provisioning.",
                ));
            }
            let _ = export::export_dashboards_with_org_clients(&export_args)?;
            Ok(())
        }
        DashboardCommand::Get(get_args) => {
            let client = build_http_client(&get_args.common)?;
            super::get_live_dashboard_to_file_with_client(&client, &get_args)
        }
        DashboardCommand::CloneLive(clone_args) => {
            let client = build_http_client(&clone_args.common)?;
            super::clone_live_dashboard_to_file_with_client(&client, &clone_args)
        }
        DashboardCommand::Serve(serve_args) => run_dashboard_serve(&serve_args),
        DashboardCommand::EditLive(edit_live_args) => {
            let client = build_http_client(&edit_live_args.common)?;
            run_dashboard_edit_live(Some(&client), &edit_live_args)
        }
        DashboardCommand::Import(import_args) => {
            let _ = import::import_dashboards_with_org_clients(&import_args)?;
            Ok(())
        }
        DashboardCommand::PatchFile(patch_args) => super::patch_dashboard_file(&patch_args),
        DashboardCommand::Review(review_args) => review_dashboard_file(&review_args),
        DashboardCommand::Publish(publish_args) => {
            let client = build_http_client(&publish_args.common)?;
            super::publish_dashboard_with_client(&client, &publish_args)
        }
        DashboardCommand::Analyze(analyze_args) => {
            if analyze_args.input_dir.is_some() {
                let inspect_args = analyze_args_to_export_args(analyze_args)?;
                if inspect_args.list_columns {
                    print_supported_dashboard_report_columns();
                    return Ok(());
                }
                if inspect_args.help_full {
                    print!("{}", render_inspect_export_help_full());
                    return Ok(());
                }
                let _ = inspect::analyze_export_dir(&inspect_args)?;
                Ok(())
            } else {
                let inspect_args = analyze_args_to_live_args(analyze_args);
                if inspect_args.list_columns {
                    print_supported_dashboard_report_columns();
                    return Ok(());
                }
                if inspect_args.help_full {
                    print!("{}", render_inspect_live_help_full());
                    return Ok(());
                }
                let client = build_http_client(&inspect_args.common)?;
                let _ = inspect_live::inspect_live_dashboards_with_client(&client, &inspect_args)?;
                Ok(())
            }
        }
        DashboardCommand::Delete(delete_args) => {
            let _ = delete::delete_dashboards_with_org_clients(&delete_args)?;
            Ok(())
        }
        DashboardCommand::Diff(diff_args) => {
            let client = build_http_client(&diff_args.common)?;
            let differences = super::diff_dashboards_with_client(&client, &diff_args)?;
            if differences > 0 {
                return Err(message(format!(
                    "Dashboard diff found {} differing item(s).",
                    differences
                )));
            }
            Ok(())
        }
        DashboardCommand::InspectExport(inspect_args) => {
            if inspect_args.list_columns {
                print_supported_dashboard_report_columns();
                return Ok(());
            }
            if inspect_args.help_full {
                print!("{}", render_inspect_export_help_full());
                return Ok(());
            }
            let _ = inspect::analyze_export_dir(&inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectLive(inspect_args) => {
            if inspect_args.list_columns {
                print_supported_dashboard_report_columns();
                return Ok(());
            }
            if inspect_args.help_full {
                print!("{}", render_inspect_live_help_full());
                return Ok(());
            }
            let client = build_http_client(&inspect_args.common)?;
            let _ = inspect_live::inspect_live_dashboards_with_client(&client, &inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectVars(inspect_vars_args) => {
            inspect_dashboard_variables(&inspect_vars_args)
        }
        DashboardCommand::GovernanceGate(governance_gate_args) => {
            super::governance_gate::run_dashboard_governance_gate(&governance_gate_args)
        }
        DashboardCommand::Topology(topology_args) => run_dashboard_topology(&topology_args),
        DashboardCommand::Impact(impact_args) => run_dashboard_impact(&impact_args),
        DashboardCommand::History(history_args) => match history_args.command {
            DashboardHistorySubcommand::List(list_args) => {
                if list_args.input.is_some() || list_args.input_dir.is_some() {
                    run_dashboard_history_list(
                        |_method, _path, _params, _payload| {
                            Err(message(
                                "dashboard history list local mode should not call Grafana",
                            ))
                        },
                        &list_args,
                    )
                } else {
                    let client = build_http_client(&list_args.common)?;
                    run_dashboard_history_list(
                        |method, path, params, payload| {
                            request_json_with_client(&client, method, path, params, payload)
                        },
                        &list_args,
                    )
                }
            }
            DashboardHistorySubcommand::Diff(diff_args) => {
                if diff_args.base_input.is_none() && diff_args.base_input_dir.is_none()
                    || diff_args.new_input.is_none() && diff_args.new_input_dir.is_none()
                {
                    let client = build_http_client(&diff_args.common)?;
                    run_dashboard_history_diff(
                        |method, path, params, payload| {
                            request_json_with_client(&client, method, path, params, payload)
                        },
                        &diff_args,
                    )
                    .map(|_| ())
                } else {
                    run_dashboard_history_diff(
                        |_method, _path, _params, _payload| {
                            Err(message(
                                "dashboard history diff local mode should not call Grafana",
                            ))
                        },
                        &diff_args,
                    )
                    .map(|_| ())
                }
            }
            DashboardHistorySubcommand::Restore(restore_args) => {
                let client = build_http_client(&restore_args.common)?;
                run_dashboard_history_restore(
                    |method, path, params, payload| {
                        request_json_with_client(&client, method, path, params, payload)
                    },
                    &restore_args,
                )
            }
            DashboardHistorySubcommand::Export(export_args) => {
                let client = build_http_client(&export_args.common)?;
                export_dashboard_history_with_request(
                    |method, path, params, payload| {
                        request_json_with_client(&client, method, path, params, payload)
                    },
                    &export_args,
                )
            }
        },
        DashboardCommand::ValidateExport(validate_args) => {
            run_dashboard_validate_export(&validate_args)
        }
        DashboardCommand::Screenshot(screenshot_args) => {
            capture_dashboard_screenshot(&screenshot_args)
        }
    }
}

pub(crate) fn materialize_dashboard_command_auth(args: &mut DashboardCliArgs) -> Result<()> {
    match &mut args.command {
        DashboardCommand::Browse(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::List(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::Export(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::Get(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::CloneLive(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::EditLive(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::Import(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::InspectLive(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::Diff(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::Screenshot(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::Delete(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::Publish(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?
        }
        DashboardCommand::History(history_args) => match &mut history_args.command {
            DashboardHistorySubcommand::List(inner) => {
                inner.common = materialize_dashboard_common_auth(inner.common.clone())?
            }
            DashboardHistorySubcommand::Restore(inner) => {
                inner.common = materialize_dashboard_common_auth(inner.common.clone())?
            }
            DashboardHistorySubcommand::Export(inner) => {
                inner.common = materialize_dashboard_common_auth(inner.common.clone())?
            }
            DashboardHistorySubcommand::Diff(_) => {}
        },
        DashboardCommand::Review(_)
        | DashboardCommand::PatchFile(_)
        | DashboardCommand::Serve(_)
        | DashboardCommand::Analyze(_)
        | DashboardCommand::GovernanceGate(_)
        | DashboardCommand::Topology(_)
        | DashboardCommand::Impact(_)
        | DashboardCommand::ValidateExport(_)
        | DashboardCommand::InspectExport(_)
        | DashboardCommand::InspectVars(_) => {}
    }
    Ok(())
}
