//! Dashboard domain orchestrator for the unified dashboard CLI.
//!
//! This file is the boundary between unified command parsing and the lower-level
//! dashboard modules that implement export/import/live/inspect/screenshot logic.
use crate::common::{message, render_json_value, set_json_color_choice, Result};
use crate::http::JsonHttpClient;
use crate::tabular_output::render_yaml;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::path::Path;

// Keep the dashboard surface area split by concern. This file should stay focused
// on re-exports, shared constants, and top-level command dispatch.
mod authoring;
mod browse;
mod browse_actions;
mod browse_edit_dialog;
mod browse_history_dialog;
mod browse_input;
mod browse_render;
mod browse_state;
mod browse_support;
mod browse_terminal;
mod browse_tui;
mod cli_defs;
mod delete;
mod delete_interactive;
mod delete_render;
mod delete_support;
mod edit;
mod edit_external;
mod edit_live;
mod edit_prompt;
mod export;
mod facade_support;
mod files;
mod governance_gate;
mod governance_gate_rules;
mod governance_gate_tui;
mod governance_policy;
mod help;
mod history;
mod impact_tui;
mod import;
mod import_compare;
mod import_interactive;
mod import_interactive_context;
mod import_interactive_loader;
mod import_interactive_render;
mod import_interactive_review;
mod import_interactive_state;
mod import_lookup;
mod import_render;
mod import_routed;
mod import_validation;
mod inspect;
mod inspect_analyzer_flux;
mod inspect_analyzer_loki;
mod inspect_analyzer_prometheus;
mod inspect_analyzer_search;
mod inspect_analyzer_sql;
mod inspect_dependency_render;
mod inspect_family;
mod inspect_governance;
mod inspect_live;
mod inspect_live_tui;
mod inspect_query;
mod inspect_render;
mod inspect_report;
mod inspect_summary;
mod inspect_workbench;
mod inspect_workbench_render;
mod inspect_workbench_state;
mod inspect_workbench_support;
mod list;
mod live;
mod live_project_status;
mod models;
mod project_status;
mod prompt;
mod prompt_helpers;
mod raw_to_prompt;
mod raw_to_prompt_output;
mod raw_to_prompt_plan;
mod raw_to_prompt_resolution;
mod raw_to_prompt_types;
mod screenshot;
mod serve;
mod topology;
mod topology_tui;
mod validate;
mod vars;

pub(crate) use authoring::{
    clone_live_dashboard_to_file_with_client, get_live_dashboard_to_file_with_client,
    patch_dashboard_file, publish_dashboard_with_client, render_dashboard_review_csv,
    render_dashboard_review_json, render_dashboard_review_table, render_dashboard_review_text,
    render_dashboard_review_yaml, review_dashboard_file as build_dashboard_review,
};
pub(crate) use cli_defs::{build_api_client, build_http_client_for_org_from_api};
pub use cli_defs::{
    build_auth_context, build_http_client, build_http_client_for_org, normalize_dashboard_cli_args,
    parse_cli_from, BrowseArgs, CloneLiveArgs, CommonCliArgs, DashboardAuthContext,
    DashboardCliArgs, DashboardCommand, DashboardHistoryArgs, DashboardHistorySubcommand,
    DashboardImportInputFormat, DashboardServeScriptFormat, DeleteArgs, DiffArgs, EditLiveArgs,
    ExportArgs, GetArgs, GovernanceGateArgs, GovernanceGateOutputFormat, GovernancePolicySource,
    HistoryExportArgs, HistoryListArgs, HistoryOutputFormat, HistoryRestoreArgs, ImpactArgs,
    ImpactOutputFormat, ImportArgs, InspectExportArgs, InspectExportReportFormat, InspectLiveArgs,
    InspectOutputFormat, InspectVarsArgs, ListArgs, PatchFileArgs, PublishArgs, RawToPromptArgs,
    RawToPromptLogFormat, RawToPromptOutputFormat, RawToPromptResolution, ReviewArgs,
    ScreenshotArgs, ScreenshotFullPageOutput, ScreenshotOutputFormat, ScreenshotTheme, ServeArgs,
    SimpleOutputFormat, TopologyArgs, TopologyOutputFormat, ValidateExportArgs,
    ValidationOutputFormat,
};
pub use export::{build_export_variant_dirs, build_output_path, export_dashboards_with_client};
pub use help::{
    maybe_render_dashboard_help_full_from_os_args, render_inspect_export_help_full,
    render_inspect_live_help_full,
};
pub use import::{diff_dashboards_with_client, import_dashboards_with_client};
pub use list::list_dashboards_with_client;
pub use live::{
    delete_dashboard_request, delete_folder_request, fetch_dashboard, import_dashboard_request,
    list_dashboard_summaries, list_datasources,
};
pub use prompt::build_external_export_document;
pub(crate) use raw_to_prompt::run_raw_to_prompt;

use browse::browse_dashboards_with_org_client;
use delete::delete_dashboards_with_org_clients;
use edit_live::run_dashboard_edit_live;
use export::export_dashboards_with_org_clients;
use history::{
    export_dashboard_history_with_request, run_dashboard_history_list,
    run_dashboard_history_restore,
};
use inspect::analyze_export_dir;
use inspect_live::inspect_live_dashboards_with_client;
use list::list_dashboards_with_org_clients;
use screenshot::capture_dashboard_screenshot;
use serve::run_dashboard_serve;
use topology::{run_dashboard_impact, run_dashboard_topology};
use validate::run_dashboard_validate_export;
use vars::inspect_dashboard_variables;

fn request_json_with_client(
    client: &JsonHttpClient,
    method: reqwest::Method,
    path: &str,
    params: &[(String, String)],
    payload: Option<&Value>,
) -> Result<Option<Value>> {
    client.request_json(method, path, params, payload)
}

pub(crate) use files::{
    build_dashboard_index_item, build_export_metadata, build_import_payload,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    discover_dashboard_files, extract_dashboard_object, load_datasource_inventory,
    load_export_metadata, load_folder_inventory, load_json_file, resolve_dashboard_import_source,
    write_dashboard, write_json_document, ResolvedDashboardImportSource,
};
#[allow(unused_imports)]
pub(crate) use facade_support::{
    build_datasource_inventory_record, build_folder_path, collect_folder_inventory_with_request,
    fetch_dashboard_if_exists_with_request, fetch_dashboard_permissions_with_request,
    fetch_dashboard_with_request, fetch_folder_if_exists_with_request,
    fetch_folder_permissions_with_request, format_folder_inventory_status_line,
    import_dashboard_request_with_request, list_dashboard_summaries_with_request,
    list_datasources_with_request, build_live_dashboard_domain_status,
    build_live_dashboard_domain_status_from_inputs,
    collect_live_dashboard_project_status_inputs_with_request, LiveDashboardProjectStatusInputs,
    load_builtin_governance_policy, load_governance_policy, load_governance_policy_file,
    load_governance_policy_source,
};
pub(crate) use inspect::build_export_inspection_summary_for_variant;
pub(crate) use inspect_live::TempInspectDir;
pub(crate) use inspect_report::ExportInspectionQueryRow;
pub(crate) use inspect_summary::{
    build_export_inspection_summary_document, ExportInspectionSummary,
};
pub(crate) use models::{
    DashboardExportRootManifest, DashboardExportRootScopeKind, DashboardIndexItem,
    DatasourceInventoryItem, ExportDatasourceUsageSummary, ExportMetadata, ExportOrgSummary,
    FolderInventoryItem, RootExportIndex, RootExportVariants, VariantIndexEntry,
};
pub(crate) use project_status::build_dashboard_domain_status;
pub(crate) use prompt::{
    build_datasource_catalog, collect_datasource_refs, datasource_type_alias,
    is_builtin_datasource_ref, is_placeholder_string, lookup_datasource,
    resolve_datasource_type_alias,
};

#[cfg(not(feature = "tui"))]
pub(crate) fn tui_not_built<T>(action: &str) -> Result<T> {
    Err(message(format!(
        "Dashboard {action} requires TUI support, but it was not built in."
    )))
}

// Shared dashboard defaults and export filenames used across export/import/live flows.
pub const DEFAULT_URL: &str = "http://localhost:3000";
pub const DEFAULT_TIMEOUT: u64 = 30;
pub const DEFAULT_PAGE_SIZE: usize = 500;
pub const DEFAULT_EXPORT_DIR: &str = "dashboards";
pub const RAW_EXPORT_SUBDIR: &str = "raw";
pub const PROMPT_EXPORT_SUBDIR: &str = "prompt";
pub const PROVISIONING_EXPORT_SUBDIR: &str = "provisioning";
pub const DEFAULT_IMPORT_MESSAGE: &str = "Imported by grafana-utils";
pub const DEFAULT_DASHBOARD_TITLE: &str = "dashboard";
pub const DEFAULT_FOLDER_TITLE: &str = "General";
pub const DEFAULT_FOLDER_UID: &str = "general";
pub const DEFAULT_ORG_ID: &str = "1";
pub const DEFAULT_ORG_NAME: &str = "Main Org.";
pub const DEFAULT_UNKNOWN_UID: &str = "unknown";
pub const EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
pub const TOOL_SCHEMA_VERSION: i64 = 1;
pub const ROOT_INDEX_KIND: &str = "grafana-utils-dashboard-export-index";
pub const FOLDER_INVENTORY_FILENAME: &str = "folders.json";
pub const DATASOURCE_INVENTORY_FILENAME: &str = "datasources.json";
pub const DASHBOARD_PERMISSION_BUNDLE_FILENAME: &str = "permissions.json";
const BUILTIN_DATASOURCE_TYPES: &[&str] = &["__expr__", "grafana"];
const BUILTIN_DATASOURCE_NAMES: &[&str] = &[
    "-- Dashboard --",
    "-- Grafana --",
    "-- Mixed --",
    "grafana",
    "expr",
    "__expr__",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) enum FolderInventoryStatusKind {
    Missing,
    Matches,
    Mismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct FolderInventoryStatus {
    pub uid: String,
    pub expected_title: String,
    pub expected_parent_uid: Option<String>,
    pub expected_path: String,
    pub actual_title: Option<String>,
    pub actual_parent_uid: Option<String>,
    pub actual_path: Option<String>,
    pub kind: FolderInventoryStatusKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DashboardWebRunOutput {
    pub document: Value,
    pub text_lines: Vec<String>,
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

pub fn execute_dashboard_list(args: &ListArgs) -> Result<DashboardWebRunOutput> {
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
    Ok(DashboardWebRunOutput {
        document: json!({
            "kind": "grafana-utils-dashboard-list",
            "dashboardCount": summaries.len(),
            "rows": rows,
        }),
        text_lines,
    })
}

fn execute_dashboard_inspect_at_path(
    args: &InspectExportArgs,
    import_dir: &Path,
    expected_variant: &str,
) -> Result<DashboardWebRunOutput> {
    inspect::validate_inspect_export_report_args(args)?;
    if let Some(report_format) = inspect::effective_inspect_report_format(args) {
        let report = inspect::apply_query_report_filters(
            inspect::build_export_inspection_query_report_for_variant(
                import_dir,
                expected_variant,
            )?,
            args.report_filter_datasource.as_deref(),
            args.report_filter_panel_id.as_deref(),
        );
        let rendered = inspect::render_export_inspection_report_output(
            args,
            import_dir,
            expected_variant,
            report_format,
            &report,
        )?;
        let document = match report_format {
            InspectExportReportFormat::Governance | InspectExportReportFormat::GovernanceJson => {
                let summary = inspect::build_export_inspection_summary_for_variant(
                    import_dir,
                    expected_variant,
                )?;
                serde_json::to_value(
                    inspect_governance::build_export_inspection_governance_document(
                        &summary, &report,
                    ),
                )?
            }
            InspectExportReportFormat::Dependency | InspectExportReportFormat::DependencyJson => {
                let metadata = load_export_metadata(import_dir, Some(expected_variant))?;
                let datasource_inventory =
                    load_datasource_inventory(import_dir, metadata.as_ref())?;
                crate::dashboard_inspection_dependency_contract::build_offline_dependency_contract_from_report_rows(
                    &report.queries,
                    &datasource_inventory,
                )
            }
            InspectExportReportFormat::Json
            | InspectExportReportFormat::Tree
            | InspectExportReportFormat::TreeTable
            | InspectExportReportFormat::Csv
            | InspectExportReportFormat::Table => serde_json::to_value(
                inspect_report::build_export_inspection_query_report_document(&report),
            )?,
        };
        return Ok(DashboardWebRunOutput {
            document,
            text_lines: rendered_output_to_lines(rendered.output),
        });
    }

    let summary =
        inspect::build_export_inspection_summary_for_variant(import_dir, expected_variant)?;
    let rendered = inspect::render_export_inspection_summary_output(args, &summary)?;
    Ok(DashboardWebRunOutput {
        document: serde_json::to_value(build_export_inspection_summary_document(&summary))?,
        text_lines: rendered_output_to_lines(rendered),
    })
}

pub fn execute_dashboard_inspect_export(args: &InspectExportArgs) -> Result<DashboardWebRunOutput> {
    let temp_dir = inspect_live::TempInspectDir::new("inspect-export-web")?;
    let import_dir = inspect::resolve_inspect_export_import_dir(
        &temp_dir.path,
        &args.import_dir,
        args.input_format,
        args.input_type,
        args.interactive,
    )?;
    execute_dashboard_inspect_at_path(args, &import_dir.import_dir, import_dir.expected_variant)
}

pub fn execute_dashboard_inspect_live(args: &InspectLiveArgs) -> Result<DashboardWebRunOutput> {
    let temp_dir = inspect_live::TempInspectDir::new("inspect-live-web")?;
    let export_args = ExportArgs {
        common: args.common.clone(),
        export_dir: temp_dir.path.clone(),
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
    let _ = export_dashboards_with_org_clients(&export_args)?;
    let inspect_import_dir = inspect_live::prepare_inspect_live_import_dir(&temp_dir.path, args)?;
    let inspect_args = InspectExportArgs {
        import_dir: inspect_import_dir,
        input_type: None,
        input_format: DashboardImportInputFormat::Raw,
        text: args.text,
        csv: args.csv,
        json: args.json,
        table: args.table,
        yaml: args.yaml,
        report: args.report,
        output_format: args.output_format,
        report_columns: args.report_columns.clone(),
        report_filter_datasource: args.report_filter_datasource.clone(),
        report_filter_panel_id: args.report_filter_panel_id.clone(),
        help_full: args.help_full,
        no_header: args.no_header,
        output_file: None,
        also_stdout: false,
        interactive: false,
    };
    execute_dashboard_inspect_at_path(&inspect_args, &inspect_args.import_dir, RAW_EXPORT_SUBDIR)
}

pub fn execute_dashboard_inspect_vars(args: &InspectVarsArgs) -> Result<DashboardWebRunOutput> {
    let document = vars::execute_dashboard_variable_inspection(args)?;
    let rendered = vars::render_dashboard_variable_output(args, &document)?;
    Ok(DashboardWebRunOutput {
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
            println!("{}", render_dashboard_review_text(&review).join("\n"));
        }
        SimpleOutputFormat::Table => {
            for line in render_dashboard_review_table(&review) {
                println!("{line}");
            }
        }
        SimpleOutputFormat::Csv => {
            for line in render_dashboard_review_csv(&review) {
                println!("{line}");
            }
        }
        SimpleOutputFormat::Json => {
            print!("{}", render_dashboard_review_json(&review)?);
        }
        SimpleOutputFormat::Yaml => {
            print!("{}", render_dashboard_review_yaml(&review)?);
        }
    }
    Ok(())
}

/// Run the dashboard CLI with an already configured client.
pub fn run_dashboard_cli_with_client(
    client: &JsonHttpClient,
    args: DashboardCliArgs,
) -> Result<()> {
    // Use this path when callers already resolved auth/org/client concerns and
    // only need dashboard command execution.
    match args.command {
        DashboardCommand::Browse(browse_args) => {
            let _ = browse::browse_dashboards_with_client(client, &browse_args)?;
            Ok(())
        }
        DashboardCommand::List(list_args) => {
            let _ = list_dashboards_with_client(client, &list_args)?;
            Ok(())
        }
        DashboardCommand::Export(export_args) => {
            let _ = export_dashboards_with_client(client, &export_args)?;
            Ok(())
        }
        DashboardCommand::RawToPrompt(raw_to_prompt_args) => {
            set_json_color_choice(raw_to_prompt_args.color);
            run_raw_to_prompt(&raw_to_prompt_args)
        }
        DashboardCommand::Get(get_args) => {
            get_live_dashboard_to_file_with_client(client, &get_args)
        }
        DashboardCommand::CloneLive(clone_args) => {
            clone_live_dashboard_to_file_with_client(client, &clone_args)
        }
        DashboardCommand::Serve(serve_args) => run_dashboard_serve(&serve_args),
        DashboardCommand::EditLive(edit_live_args) => {
            run_dashboard_edit_live(Some(client), &edit_live_args)
        }
        DashboardCommand::Import(import_args) => {
            let _ = import_dashboards_with_client(client, &import_args)?;
            Ok(())
        }
        DashboardCommand::PatchFile(patch_args) => patch_dashboard_file(&patch_args),
        DashboardCommand::Review(review_args) => review_dashboard_file(&review_args),
        DashboardCommand::Publish(publish_args) => {
            publish_dashboard_with_client(client, &publish_args)
        }
        DashboardCommand::Delete(delete_args) => {
            let _ = delete::delete_dashboards_with_client(client, &delete_args)?;
            Ok(())
        }
        DashboardCommand::Diff(diff_args) => {
            // Diff is treated as a failing command when changes are found so shell
            // automation can gate on a non-zero exit status.
            let differences = diff_dashboards_with_client(client, &diff_args)?;
            if differences > 0 {
                return Err(message(format!(
                    "Dashboard diff found {} differing item(s).",
                    differences
                )));
            }
            Ok(())
        }
        DashboardCommand::InspectExport(inspect_args) => {
            // `--help-full` is handled here because inspect-export can be executed
            // both from the unified CLI and from direct dashboard parser paths.
            if inspect_args.help_full {
                print!("{}", render_inspect_export_help_full());
                return Ok(());
            }
            let _ = analyze_export_dir(&inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectLive(inspect_args) => {
            if inspect_args.help_full {
                print!("{}", render_inspect_live_help_full());
                return Ok(());
            }
            let _ = inspect_live_dashboards_with_client(client, &inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectVars(inspect_vars_args) => {
            inspect_dashboard_variables(&inspect_vars_args)
        }
        DashboardCommand::GovernanceGate(governance_gate_args) => {
            governance_gate::run_dashboard_governance_gate(&governance_gate_args)
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
pub fn run_dashboard_cli(args: DashboardCliArgs) -> Result<()> {
    // This is the main runtime entrypoint for dashboard commands from the binary:
    // normalize CLI args first, then decide whether each branch needs org fan-out,
    // one live client, or no client at all.
    set_json_color_choice(args.color);
    let args = normalize_dashboard_cli_args(args);
    match args.command {
        DashboardCommand::Browse(browse_args) => {
            let _ = browse_dashboards_with_org_client(&browse_args)?;
            Ok(())
        }
        DashboardCommand::List(list_args) => {
            let _ = list_dashboards_with_org_clients(&list_args)?;
            Ok(())
        }
        DashboardCommand::Export(export_args) => {
            // Reject the "export nothing" shape early so lower layers can assume at
            // least one artifact variant will be written.
            if export_args.without_dashboard_raw
                && export_args.without_dashboard_prompt
                && export_args.without_dashboard_provisioning
            {
                return Err(message(
                    "At least one export variant must stay enabled. Remove --without-dashboard-raw, --without-dashboard-prompt, or --without-dashboard-provisioning.",
                ));
            }
            let _ = export_dashboards_with_org_clients(&export_args)?;
            Ok(())
        }
        DashboardCommand::RawToPrompt(raw_to_prompt_args) => {
            set_json_color_choice(raw_to_prompt_args.color);
            run_raw_to_prompt(&raw_to_prompt_args)
        }
        DashboardCommand::Get(get_args) => {
            let client = build_http_client(&get_args.common)?;
            get_live_dashboard_to_file_with_client(&client, &get_args)
        }
        DashboardCommand::CloneLive(clone_args) => {
            let client = build_http_client(&clone_args.common)?;
            clone_live_dashboard_to_file_with_client(&client, &clone_args)
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
        DashboardCommand::PatchFile(patch_args) => patch_dashboard_file(&patch_args),
        DashboardCommand::Review(review_args) => review_dashboard_file(&review_args),
        DashboardCommand::Publish(publish_args) => {
            let client = build_http_client(&publish_args.common)?;
            publish_dashboard_with_client(&client, &publish_args)
        }
        DashboardCommand::Delete(delete_args) => {
            let _ = delete_dashboards_with_org_clients(&delete_args)?;
            Ok(())
        }
        DashboardCommand::Diff(diff_args) => {
            let client = build_http_client(&diff_args.common)?;
            let differences = diff_dashboards_with_client(&client, &diff_args)?;
            if differences > 0 {
                return Err(message(format!(
                    "Dashboard diff found {} differing item(s).",
                    differences
                )));
            }
            Ok(())
        }
        DashboardCommand::InspectExport(inspect_args) => {
            if inspect_args.help_full {
                print!("{}", render_inspect_export_help_full());
                return Ok(());
            }
            let _ = analyze_export_dir(&inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectLive(inspect_args) => {
            if inspect_args.help_full {
                print!("{}", render_inspect_live_help_full());
                return Ok(());
            }
            let client = build_http_client(&inspect_args.common)?;
            let _ = inspect_live_dashboards_with_client(&client, &inspect_args)?;
            Ok(())
        }
        DashboardCommand::InspectVars(inspect_vars_args) => {
            inspect_dashboard_variables(&inspect_vars_args)
        }
        DashboardCommand::GovernanceGate(governance_gate_args) => {
            governance_gate::run_dashboard_governance_gate(&governance_gate_args)
        }
        DashboardCommand::Topology(topology_args) => run_dashboard_topology(&topology_args),
        DashboardCommand::Impact(impact_args) => run_dashboard_impact(&impact_args),
        DashboardCommand::History(history_args) => match history_args.command {
            DashboardHistorySubcommand::List(list_args) => {
                let client = build_http_client(&list_args.common)?;
                run_dashboard_history_list(
                    |method, path, params, payload| {
                        request_json_with_client(&client, method, path, params, payload)
                    },
                    &list_args,
                )
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

#[cfg(test)]
#[path = "authoring_rust_tests.rs"]
mod authoring_rust_tests;
#[cfg(test)]
#[path = "dashboard_cli_rust_tests.rs"]
mod dashboard_cli_rust_tests;
#[cfg(test)]
#[path = "rust_tests.rs"]
mod dashboard_rust_tests;
#[cfg(test)]
#[path = "history_cli_rust_tests.rs"]
mod history_cli_rust_tests;
#[cfg(test)]
#[path = "import_rust_tests.rs"]
mod import_rust_tests;
#[cfg(test)]
#[path = "inspect_export_rust_tests.rs"]
mod inspect_export_rust_tests;
#[cfg(test)]
#[path = "inspect_governance_document_rust_tests.rs"]
mod inspect_governance_document_rust_tests;
#[cfg(test)]
#[path = "inspect_governance_rust_tests.rs"]
mod inspect_governance_rust_tests;
#[cfg(test)]
#[path = "inspect_live_rust_tests.rs"]
mod inspect_live_rust_tests;
#[cfg(test)]
#[path = "inspect_vars_rust_tests.rs"]
mod inspect_vars_rust_tests;
#[cfg(test)]
#[path = "raw_to_prompt_rust_tests.rs"]
mod raw_to_prompt_rust_tests;
#[cfg(test)]
#[path = "screenshot_rust_tests.rs"]
mod screenshot_rust_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
#[path = "topology_impact_document_rust_tests.rs"]
mod topology_impact_document_rust_tests;
#[cfg(test)]
#[path = "topology_impact_rust_tests.rs"]
mod topology_impact_rust_tests;
