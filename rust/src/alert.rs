//! Alerting domain entry and orchestration module.
//!
//! Purpose:
//! - Own the alerting command surface (`list`, `export`, `import`, `diff`).
//! - Bridge parsed CLI args to `GrafanaAlertClient` and alerting handlers.
//! - Keep response parsing and payload shaping close to alert domain types.
//!
//! Flow:
//! - Parse CLI args via `alert_cli_defs`.
//! - Normalize legacy/namespaced invocation forms before dispatch.
//! - Build client only in the concrete runtime entrypoint; keep pure routing paths testable.
//!
//! Caveats:
//! - Avoid adding transport policy here; retry/pagination behavior should stay in shared HTTP
//!   layers and alert handlers.
//! - Keep diff/import/export payload transforms next to their handlers, not in dispatcher code.

use crate::common::{string_field, write_json_file, Result};

#[path = "alert_cli_defs.rs"]
mod alert_cli_defs;
#[path = "alert_client.rs"]
mod alert_client;
#[path = "alert_compare_support.rs"]
mod alert_compare_support;
#[path = "alert_export.rs"]
mod alert_export;
#[path = "alert_import_diff.rs"]
mod alert_import_diff;
#[path = "alert_linkage_support.rs"]
mod alert_linkage_support;
#[path = "alert_list.rs"]
mod alert_list;
#[path = "alert_live_project_status.rs"]
mod alert_live_project_status;
#[path = "alert_project_status.rs"]
mod alert_project_status;
#[path = "alert_runtime_support.rs"]
mod alert_runtime_support;
#[path = "alert_support.rs"]
mod alert_support;

pub use alert_cli_defs::{
    build_auth_context, cli_args_from_common, normalize_alert_group_command,
    normalize_alert_namespace_args, parse_cli_from, root_command, AlertAuthContext, AlertCliArgs,
    AlertCommonArgs, AlertDiffArgs, AlertExportArgs, AlertGroupCommand, AlertImportArgs,
    AlertLegacyArgs, AlertListArgs, AlertListKind, AlertNamespaceArgs,
};
pub(crate) use alert_client::GrafanaAlertClient;
#[cfg(test)]
pub(crate) use alert_client::{expect_object_list, parse_template_list_response};
#[allow(unused_imports)]
pub(crate) use alert_compare_support::{
    append_root_index_item, build_compare_diff_text, build_compare_document,
    build_resource_identity, format_export_summary, serialize_compare_document,
    write_resource_indexes,
};
#[cfg(test)]
pub(crate) use alert_linkage_support::get_rule_linkage;
#[cfg(test)]
pub(crate) use alert_list::serialize_rule_list_rows;
pub use alert_live_project_status::{
    build_alert_live_project_status_domain, AlertLiveProjectStatusInputs,
};
pub(crate) use alert_project_status::build_alert_project_status_domain;
pub use alert_runtime_support::{build_alert_diff_document, build_alert_import_dry_run_document};
#[cfg(test)]
pub(crate) use alert_runtime_support::{
    determine_import_action_with_request, fetch_live_compare_document_with_request,
    import_resource_document_with_request,
};
pub use alert_support::{
    build_contact_point_export_document, build_contact_point_import_payload,
    build_contact_point_output_path, build_empty_root_index, build_import_operation,
    build_mute_timing_export_document, build_mute_timing_import_payload,
    build_mute_timing_output_path, build_policies_export_document, build_policies_import_payload,
    build_policies_output_path, build_resource_dirs, build_rule_export_document,
    build_rule_import_payload, build_rule_output_path, build_template_export_document,
    build_template_import_payload, build_template_output_path, derive_dashboard_slug,
    detect_document_kind, discover_alert_resource_files, load_panel_id_map, load_string_map,
    reject_provisioning_export, resource_subdir_by_kind, strip_server_managed_fields,
};
pub(crate) use alert_support::{value_to_string, AlertLinkageMappings};

/// Constant for default url.
pub const DEFAULT_URL: &str = "http://127.0.0.1:3000";
/// Constant for default timeout.
pub const DEFAULT_TIMEOUT: u64 = 30;
/// Constant for default output dir.
pub const DEFAULT_OUTPUT_DIR: &str = "alerts";
/// Constant for raw export subdir.
pub const RAW_EXPORT_SUBDIR: &str = "raw";
/// Constant for rules subdir.
pub const RULES_SUBDIR: &str = "rules";
/// Constant for contact points subdir.
pub const CONTACT_POINTS_SUBDIR: &str = "contact-points";
/// Constant for mute timings subdir.
pub const MUTE_TIMINGS_SUBDIR: &str = "mute-timings";
/// Constant for policies subdir.
pub const POLICIES_SUBDIR: &str = "policies";
/// Constant for templates subdir.
pub const TEMPLATES_SUBDIR: &str = "templates";
/// Constant for rule kind.
pub const RULE_KIND: &str = "grafana-alert-rule";
/// Constant for contact point kind.
pub const CONTACT_POINT_KIND: &str = "grafana-contact-point";
/// Constant for mute timing kind.
pub const MUTE_TIMING_KIND: &str = "grafana-mute-timing";
/// Constant for policies kind.
pub const POLICIES_KIND: &str = "grafana-notification-policies";
/// Constant for template kind.
pub const TEMPLATE_KIND: &str = "grafana-notification-template";
/// Constant for tool api version.
pub const TOOL_API_VERSION: i64 = 1;
/// Constant for tool schema version.
pub const TOOL_SCHEMA_VERSION: i64 = 1;
/// Constant for root index kind.
pub const ROOT_INDEX_KIND: &str = "grafana-util-alert-export-index";

/// Constant for alert help text.
pub const ALERT_HELP_TEXT: &str = "Examples:\n\n  Export alerting resources with an API token:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-util alert export --url https://grafana.example.com --output-dir ./alerts --overwrite\n\n  Import back into Grafana and update existing resources:\n    grafana-util alert import --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing\n\n  Preview alert import as structured JSON before execution:\n    grafana-util alert import --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing --dry-run --json\n\n  Compare a local alert export against Grafana as structured JSON:\n    grafana-util alert diff --url https://grafana.example.com --diff-dir ./alerts/raw --json\n\n  Import linked alert rules with dashboard and panel remapping:\n    grafana-util alert import --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json";

/// Alert domain execution entrypoint.
///
/// Dispatches by checking argument exclusivity (`list`, `import`, `diff`, else export) and
/// forwarding to the corresponding handler.
pub fn run_alert_cli(args: AlertCliArgs) -> Result<()> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: alert.rs:diff_alerting_resources, alert.rs:export_alerting_resources, alert.rs:import_alerting_resources, alert_list.rs:list_alert_resources

    if args.list_kind.is_some() {
        return alert_list::list_alert_resources(&args);
    }
    if args.import_dir.is_some() {
        return alert_import_diff::import_alerting_resources(&args);
    }
    if args.diff_dir.is_some() {
        return alert_import_diff::diff_alerting_resources(&args);
    }
    alert_export::export_alerting_resources(&args)
}

#[cfg(test)]
#[path = "alert_rust_tests.rs"]
mod alert_rust_tests;
