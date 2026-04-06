//! Route combined dashboard exports back into their destination orgs.
//!
//! Each source org is resolved independently, then the import args are rebound
//! to the matching destination org so dry-run and live execution stay isolated.

use reqwest::Method;
use serde_json::Value;
use std::path::Path;

use crate::common::{message, Result};

use super::import_lookup::ImportLookupCache;
use super::import_render::{
    build_import_dry_run_json_value, build_routed_import_dry_run_json_document,
    build_routed_import_org_row, describe_dashboard_import_mode,
    format_routed_import_scope_summary_fields, render_routed_import_org_table, ImportDryRunReport,
};
use super::import_validation::{
    discover_export_org_import_scopes, resolve_target_org_plan_for_export_scope_with_request,
};

fn count_dashboard_files(input_dir: &Path) -> Result<usize> {
    let mut dashboard_files = super::discover_dashboard_files(input_dir)?;
    dashboard_files.retain(|path| {
        path.file_name().and_then(|name| name.to_str()) != Some(super::FOLDER_INVENTORY_FILENAME)
    });
    Ok(dashboard_files.len())
}

pub(crate) fn build_routed_import_dry_run_json_with_request<F, G>(
    mut request_json: F,
    mut collect_preview_for_org: G,
    args: &super::ImportArgs,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
    G: FnMut(i64, &super::ImportArgs) -> Result<ImportDryRunReport>,
{
    let scopes = discover_export_org_import_scopes(args)?;
    let mut lookup_cache = ImportLookupCache::default();
    let mut orgs = Vec::new();
    let mut imports = Vec::new();
    for scope in scopes {
        let target_plan = resolve_target_org_plan_for_export_scope_with_request(
            &mut request_json,
            &mut lookup_cache,
            args,
            &scope,
        )?;
        let dashboard_count = count_dashboard_files(&target_plan.input_dir)?;
        orgs.push(serde_json::json!({
            "sourceOrgId": target_plan.source_org_id,
            "sourceOrgName": target_plan.source_org_name,
            "orgAction": target_plan.org_action,
            "targetOrgId": target_plan.target_org_id,
            "dashboardCount": dashboard_count,
            "importDir": target_plan.input_dir.display().to_string(),
        }));
        let preview = if let Some(target_org_id) = target_plan.target_org_id {
            let mut scoped_args = args.clone();
            scoped_args.org_id = Some(target_org_id);
            scoped_args.use_export_org = false;
            scoped_args.only_org_id = Vec::new();
            scoped_args.create_missing_orgs = false;
            scoped_args.input_dir = target_plan.input_dir.clone();
            let report = collect_preview_for_org(target_org_id, &scoped_args)?;
            build_import_dry_run_json_value(&report)
        } else {
            serde_json::json!({
                "mode": describe_dashboard_import_mode(args.replace_existing, args.update_existing_only),
                "folders": [],
                "dashboards": [],
                "summary": {
                    "importDir": target_plan.input_dir.display().to_string(),
                    "folderCount": 0,
                    "missingFolders": 0,
                    "mismatchedFolders": 0,
                    "dashboardCount": dashboard_count,
                    "missingDashboards": 0,
                    "skippedMissingDashboards": 0,
                    "skippedFolderMismatchDashboards": 0,
                }
            })
        };
        let mut import_entry = serde_json::Map::new();
        import_entry.insert(
            "sourceOrgId".to_string(),
            Value::from(target_plan.source_org_id),
        );
        import_entry.insert(
            "sourceOrgName".to_string(),
            Value::from(target_plan.source_org_name.clone()),
        );
        import_entry.insert("orgAction".to_string(), Value::from(target_plan.org_action));
        import_entry.insert(
            "targetOrgId".to_string(),
            target_plan
                .target_org_id
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        if let Some(preview_object) = preview.as_object() {
            for (key, value) in preview_object {
                import_entry.insert(key.clone(), value.clone());
            }
        }
        imports.push(Value::Object(import_entry));
    }
    build_routed_import_dry_run_json_document(&orgs, &imports)
}

pub(crate) fn import_dashboards_by_export_org_with_request<F, G, H>(
    mut request_json: F,
    mut import_for_org: G,
    collect_preview_for_org: H,
    args: &super::ImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
    G: FnMut(i64, &super::ImportArgs) -> Result<usize>,
    H: FnMut(i64, &super::ImportArgs) -> Result<ImportDryRunReport>,
{
    let scopes = discover_export_org_import_scopes(args)?;
    let mut lookup_cache = ImportLookupCache::default();
    // Dry-run JSON is the most structured preview, so emit it before any
    // table rendering or live import work.
    if args.dry_run && args.json {
        print!(
            "{}",
            build_routed_import_dry_run_json_with_request(
                request_json,
                collect_preview_for_org,
                args,
            )?
        );
        return Ok(0);
    }
    let mut imported_count = 0;
    let mut org_rows = Vec::new();
    let mut resolved_plans = Vec::new();
    for scope in scopes {
        let target_plan = resolve_target_org_plan_for_export_scope_with_request(
            &mut request_json,
            &mut lookup_cache,
            args,
            &scope,
        )?;
        let dashboard_count = count_dashboard_files(&target_plan.input_dir)?;
        org_rows.push(build_routed_import_org_row(&target_plan, dashboard_count));
        resolved_plans.push(target_plan);
    }
    // Dry-run table output uses the same resolved plan but intentionally stops
    // short of any destination org write.
    if args.dry_run && args.table {
        for line in render_routed_import_org_table(&org_rows, !args.no_header) {
            println!("{line}");
        }
        return Ok(0);
    }
    for target_plan in resolved_plans {
        if !args.table {
            println!(
                "Importing {}",
                format_routed_import_scope_summary_fields(
                    target_plan.source_org_id,
                    &target_plan.source_org_name,
                    target_plan.org_action,
                    target_plan.target_org_id,
                    &target_plan.input_dir,
                )
            );
        }
        // Unresolved export-org scopes are still listed in preview output, but
        // only resolved destination orgs are eligible for live writes.
        let Some(target_org_id) = target_plan.target_org_id else {
            continue;
        };
        // Rebind the args to the destination org and disable export-org
        // expansion so each routed import stays scoped to one org at a time.
        let mut scoped_args = args.clone();
        scoped_args.org_id = Some(target_org_id);
        scoped_args.use_export_org = false;
        scoped_args.only_org_id = Vec::new();
        scoped_args.create_missing_orgs = false;
        scoped_args.input_dir = target_plan.input_dir.clone();
        imported_count += import_for_org(target_org_id, &scoped_args).map_err(|error| {
            message(format!(
                "Dashboard routed import failed for {}: {}",
                format_routed_import_scope_summary_fields(
                    target_plan.source_org_id,
                    &target_plan.source_org_name,
                    target_plan.org_action,
                    target_plan.target_org_id,
                    &target_plan.input_dir,
                ),
                error
            ))
        })?;
    }
    Ok(imported_count)
}
