//! Datasource domain orchestrator.
//!
//! Purpose:
//! - Own datasource command flows (`list`, `add`, `delete`, `export`, `import`, `diff`).
//! - Normalize datasource contract shape across live API payloads and exported metadata.
//! - Keep output serialization (`table`/`csv`/`json`/`yaml`) selection centralized.
//!
//! Flow:
//! - Parse args from `dashboard`-shared auth/common CLI types where possible.
//! - Normalize command variants before branching by subcommand.
//! - Build client and route execution to list/export/import/diff helpers.
//!
//! Caveats:
//! - Keep API-field compatibility logic in `datasource_diff.rs` and import/export helpers.
//! - Avoid side effects in normalization helpers; keep them as pure value transforms.
pub(crate) use crate::common::write_json_file;
pub use crate::dashboard::CommonCliArgs;

#[path = "browse/mod.rs"]
mod datasource_browse;
#[cfg(feature = "tui")]
#[path = "browse/edit_dialog.rs"]
mod datasource_browse_edit_dialog;
#[cfg(feature = "tui")]
#[path = "browse/input.rs"]
mod datasource_browse_input;
#[cfg(feature = "tui")]
#[path = "browse/render.rs"]
mod datasource_browse_render;
#[cfg(feature = "tui")]
#[path = "browse/state.rs"]
mod datasource_browse_state;
#[path = "browse/support.rs"]
mod datasource_browse_support;
#[cfg(feature = "tui")]
#[path = "browse/terminal.rs"]
mod datasource_browse_terminal;
#[cfg(feature = "tui")]
#[path = "browse/tui.rs"]
mod datasource_browse_tui;
#[path = "cli/defs.rs"]
mod datasource_cli_defs;
#[path = "diff/mod.rs"]
mod datasource_diff;
#[path = "diff/render.rs"]
mod datasource_diff_render;
#[path = "import_export.rs"]
mod datasource_import_export;
#[path = "inspect/export.rs"]
mod datasource_inspect_export;
#[path = "list/local.rs"]
mod datasource_local_list;
#[path = "mutation/support.rs"]
mod datasource_mutation_support;
#[path = "runtime.rs"]
mod datasource_runtime;

pub(crate) use datasource_cli_defs::{normalize_datasource_group_command, root_command};
pub use datasource_cli_defs::{
    DatasourceAddArgs, DatasourceBrowseArgs, DatasourceCliArgs, DatasourceDeleteArgs,
    DatasourceDiffArgs, DatasourceExportArgs, DatasourceGroupCommand, DatasourceImportArgs,
    DatasourceImportInputFormat, DatasourceListArgs, DatasourceModifyArgs, DatasourceTypesArgs,
    DryRunOutputFormat, ListOutputFormat,
};
#[cfg(test)]
pub(crate) use datasource_diff_render::{
    datasource_diff_row, datasource_diff_summary_line, render_diff_identity,
};
pub(crate) use datasource_diff_render::{diff_datasources_with_live, resolve_delete_preview_type};
pub(crate) use datasource_import_export::{
    build_all_orgs_export_index, build_all_orgs_export_metadata, build_all_orgs_output_dir,
    build_datasource_export_metadata, build_datasource_provisioning_document, build_export_index,
    build_export_records, build_list_records, datasource_list_column_ids, fetch_current_org,
    import_datasources_by_export_org, import_datasources_with_client, list_orgs,
    load_datasource_export_root_manifest, load_datasource_inventory_records_from_export_root,
    load_diff_record_values, load_import_records, render_data_source_csv, render_data_source_json,
    render_data_source_summary_line, render_data_source_table, resolve_datasource_export_root_dir,
    resolve_target_client, validate_import_org_auth, write_yaml_file,
    DatasourceExportRootScopeKind, DatasourceImportRecord, DATASOURCE_EXPORT_FILENAME,
    DATASOURCE_PROVISIONING_FILENAME, DATASOURCE_PROVISIONING_SUBDIR, EXPORT_METADATA_FILENAME,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use datasource_import_export::{
    build_import_payload, build_import_payload_with_secret_values,
    classify_datasource_export_root_scope_kind, collect_datasource_import_dry_run_report,
    discover_export_org_import_scopes, format_routed_datasource_import_summary_line,
    format_routed_datasource_scope_summary_fields, format_routed_datasource_target_org_label,
    render_routed_datasource_import_org_table, resolve_export_org_target_plan,
    DatasourceExportOrgScope, DatasourceExportOrgTargetPlan, DatasourceImportDryRunReport,
};
#[cfg(any(feature = "tui", test))]
#[allow(unused_imports)]
pub(crate) use datasource_inspect_export::{
    build_datasource_inspect_export_browser_items, load_datasource_inspect_export_source,
    prompt_datasource_inspect_export_input_format, render_datasource_inspect_export_output,
    resolve_datasource_inspect_export_input_format, DatasourceInspectExportRenderFormat,
    DatasourceInspectExportSource,
};
#[cfg(not(any(feature = "tui", test)))]
#[allow(unused_imports)]
pub(crate) use datasource_inspect_export::{
    load_datasource_inspect_export_source, prompt_datasource_inspect_export_input_format,
    render_datasource_inspect_export_output, resolve_datasource_inspect_export_input_format,
    DatasourceInspectExportRenderFormat, DatasourceInspectExportSource,
};
pub(crate) use datasource_local_list::{render_datasource_text, run_local_datasource_list};
#[cfg(test)]
pub(crate) use datasource_mutation_support::parse_json_object_argument;
use datasource_mutation_support::{
    build_add_payload, build_modify_payload, build_modify_updates, render_import_table,
    render_live_mutation_json, render_live_mutation_table, resolve_delete_match,
    resolve_live_mutation_match, validate_live_mutation_dry_run_args,
};
pub(crate) use datasource_mutation_support::{fetch_datasource_by_uid_if_exists, resolve_match};
pub use datasource_runtime::run_datasource_cli;

#[cfg(test)]
mod datasource_operator_text_tests {
    use super::*;
    use crate::datasource::datasource_diff::DatasourceDiffSummary;
    use crate::datasource::datasource_diff::{
        DatasourceDiffEntry, DatasourceDiffReport, DatasourceDiffStatus,
    };
    use std::path::Path;

    #[test]
    fn diff_identity_and_row_include_datasource_type() {
        let export_record = DatasourceImportRecord::from_generic_map(
            &serde_json::json!({
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "true",
                "orgId": "1"
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let live_record = DatasourceImportRecord::from_generic_map(
            &serde_json::json!({
                "id": 7,
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "loki",
                "access": "direct",
                "url": "http://loki:3100",
                "isDefault": false,
                "orgId": 1
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let entry = DatasourceDiffEntry {
            key: "uid:prom-main".to_string(),
            status: DatasourceDiffStatus::Different,
            export_record: Some(export_record),
            live_record: Some(live_record),
            differences: vec![],
        };

        let identity = render_diff_identity(&entry);
        let row = datasource_diff_row(&entry);

        assert!(identity.contains("uid=prom-main"));
        assert!(identity.contains("name=Prometheus Main"));
        assert!(identity.contains("type=prometheus"));
        assert_eq!(row["type"], serde_json::json!("prometheus"));
        assert_eq!(row["matchBasis"], serde_json::json!("uid"));
        assert_eq!(row["identity"], serde_json::json!(identity));
    }

    #[test]
    fn datasource_diff_summary_line_includes_source_context_and_status_breakdown() {
        let report = DatasourceDiffReport {
            entries: vec![],
            summary: DatasourceDiffSummary {
                compared_count: 4,
                matches_count: 1,
                different_count: 1,
                missing_in_live_count: 1,
                missing_in_export_count: 0,
                ambiguous_live_match_count: 1,
            },
        };

        let line = datasource_diff_summary_line(Path::new("/tmp/datasources"), &report);

        assert_eq!(
            line,
            "Diff checked 4 datasource(s) from /tmp/datasources against Grafana live datasources; 3 difference(s) found (same=1 different=1 missing-live=1 extra-live=0 ambiguous=1)."
        );
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod datasource_rust_tests;

#[cfg(test)]
#[path = "tests/diff.rs"]
mod datasource_diff_rust_tests;
