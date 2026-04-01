//! Import orchestration for dashboards.
//! Loads local export artifacts, computes target orgs, and applies idempotent upsert behavior
//! through the shared dashboard HTTP/auth context.

#[path = "import_apply.rs"]
mod import_apply;
#[path = "import_dry_run.rs"]
mod import_dry_run;

use crate::common::{message, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[cfg(feature = "tui")]
use reqwest::Method;
#[cfg(feature = "tui")]
use serde_json::Value;

#[allow(unused_imports)]
pub(crate) use super::import_compare::diff_dashboards_with_request;
#[cfg(test)]
pub(crate) use super::import_render::format_routed_import_target_org_label;
#[allow(unused_imports)]
pub(crate) use super::import_render::{
    describe_dashboard_import_mode, format_import_progress_line, format_import_verbose_line,
    format_routed_import_scope_summary_fields, render_folder_inventory_dry_run_table,
    render_import_dry_run_json, render_import_dry_run_table, render_routed_import_org_table,
    ImportDryRunReport,
};
#[allow(unused_imports)]
pub(crate) use super::import_routed::{
    build_routed_import_dry_run_json_with_request, import_dashboards_by_export_org_with_request,
};
pub(crate) use super::import_validation::build_import_auth_context;
#[allow(unused_imports)]
use super::{
    build_http_client_for_org, build_import_payload, discover_dashboard_files,
    extract_dashboard_object, import_dashboard_request_with_request, load_export_metadata,
    load_folder_inventory, load_json_file, resolve_dashboard_import_source, validate, DiffArgs,
    FolderInventoryItem, FolderInventoryStatus, FolderInventoryStatusKind, ImportArgs,
    ResolvedDashboardImportSource, DEFAULT_UNKNOWN_UID, FOLDER_INVENTORY_FILENAME,
    PROVISIONING_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR,
};
#[allow(unused_imports)]
use super::{
    format_folder_inventory_status_line, import_compare, import_lookup, import_render,
    import_routed, import_validation,
};
pub use import_apply::{diff_dashboards_with_client, import_dashboards_with_client};
#[allow(unused_imports)]
pub(crate) use import_apply::{import_dashboards_with_org_clients, import_dashboards_with_request};
#[allow(unused_imports)]
pub(crate) use import_dry_run::collect_import_dry_run_report_with_request;

pub(crate) fn resolve_import_source(
    args: &super::ImportArgs,
) -> Result<ResolvedDashboardImportSource> {
    resolve_dashboard_import_source(&args.import_dir, args.input_format)
}

pub(crate) fn resolve_diff_source(args: &super::DiffArgs) -> Result<ResolvedDashboardImportSource> {
    resolve_dashboard_import_source(&args.import_dir, args.input_format)
}

pub(crate) fn import_metadata_variant(args: &super::ImportArgs) -> &'static str {
    match args.input_format {
        super::DashboardImportInputFormat::Raw => RAW_EXPORT_SUBDIR,
        super::DashboardImportInputFormat::Provisioning => PROVISIONING_EXPORT_SUBDIR,
    }
}

pub(crate) fn dashboard_files_for_import(import_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut dashboard_files = super::discover_dashboard_files(import_dir)?;
    dashboard_files.retain(|path| {
        path.file_name().and_then(|name| name.to_str()) != Some(super::FOLDER_INVENTORY_FILENAME)
    });
    Ok(dashboard_files)
}

fn selected_dashboard_files(
    #[cfg(feature = "tui")] request_json: &mut impl FnMut(
        Method,
        &str,
        &[(String, String)],
        Option<&Value>,
    ) -> Result<Option<Value>>,
    #[cfg(feature = "tui")] lookup_cache: &mut super::import_lookup::ImportLookupCache,
    args: &super::ImportArgs,
    dashboard_files: Vec<PathBuf>,
) -> Result<Option<Vec<PathBuf>>> {
    #[cfg(feature = "tui")]
    {
        let Some(selected_files) = super::import_interactive::select_import_dashboard_files(
            request_json,
            lookup_cache,
            args,
        )?
        else {
            return Ok(None);
        };
        let known_files: BTreeSet<PathBuf> = dashboard_files.iter().cloned().collect();
        let filtered: Vec<PathBuf> = selected_files
            .into_iter()
            .filter(|path| known_files.contains(path))
            .collect();
        if filtered.is_empty() {
            return Err(message(
                "Dashboard import interactive selection did not pick any valid dashboard files.",
            ));
        }
        Ok(Some(filtered))
    }
    #[cfg(not(feature = "tui"))]
    {
        if args.interactive {
            return super::tui_not_built("import --interactive");
        }
        let _ = args;
        let _ = dashboard_files;
        Ok(None)
    }
}
