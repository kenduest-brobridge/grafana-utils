//! Dashboard CLI execution and orchestration.
use crate::common::{message, print_supported_columns, set_json_color_choice, Result};
use crate::http::JsonHttpClient;
use serde_json::Value;

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
pub use super::run_inspect::{
    execute_dashboard_inspect_export, execute_dashboard_inspect_live,
    execute_dashboard_inspect_vars,
};
pub use super::run_list::execute_dashboard_list;
use super::screenshot::capture_dashboard_screenshot;
use super::serve::run_dashboard_serve;
use super::topology::{run_dashboard_impact, run_dashboard_topology};
use super::validate::run_dashboard_validate_export;
use super::vars::inspect_dashboard_variables;
#[allow(unused_imports)]
use super::{
    build_dashboard_review, build_http_client, materialize_dashboard_common_auth,
    render_inspect_export_help_full, render_inspect_live_help_full, AnalyzeArgs, DashboardCliArgs,
    DashboardCommand, DashboardHistorySubcommand, InspectExportArgs, InspectLiveArgs, ReviewArgs,
    SimpleOutputFormat,
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
