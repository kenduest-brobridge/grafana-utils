//! Import orchestration for Dashboard resources, including input normalization and apply contract handling.

use reqwest::Method;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::common::{message, Result};
#[cfg(feature = "tui")]
use crate::dashboard::import_interactive;
use crate::dashboard::{
    build_http_client_for_org, build_import_payload, extract_dashboard_object,
    import_dashboard_request_with_request, load_export_metadata, load_folder_inventory,
    load_json_file, validate, DiffArgs, ExportMetadata, FolderInventoryItem, ImportArgs,
    DEFAULT_UNKNOWN_UID, FOLDER_INVENTORY_FILENAME,
};
use crate::grafana_api::DashboardResourceClient;
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::super::import_compare::diff_dashboards_with_request;
use super::super::import_lookup::{
    apply_folder_path_guard_to_action, build_folder_path_match_result,
    determine_dashboard_import_action_with_client, determine_dashboard_import_action_with_request,
    determine_import_folder_uid_override_with_client,
    determine_import_folder_uid_override_with_request, ensure_folder_inventory_entry_cached,
    ensure_folder_inventory_entry_with_client, resolve_existing_dashboard_folder_path_with_client,
    resolve_existing_dashboard_folder_path_with_request, ImportLookupCache,
};
use super::super::import_render::{
    format_import_progress_line, format_import_verbose_line, render_import_dry_run_json,
    render_import_dry_run_table,
};
use super::super::import_validation::{
    validate_dashboard_import_dependencies_with_request, validate_matching_export_org_with_request,
};
use super::import_dry_run::{
    collect_import_dry_run_report_with_client, collect_import_dry_run_report_with_request,
    folder_inventory_status_output_lines,
};

trait LiveImportBackend {
    fn validate_export_org(
        &mut self,
        cache: &mut ImportLookupCache,
        args: &ImportArgs,
        input_dir: &std::path::Path,
        metadata: Option<&ExportMetadata>,
    ) -> Result<()>;

    fn validate_dependencies(
        &mut self,
        input_dir: &std::path::Path,
        strict_schema: bool,
        target_schema_version: Option<i64>,
    ) -> Result<()>;

    fn determine_import_folder_uid_override(
        &mut self,
        cache: &mut ImportLookupCache,
        uid: &str,
        folder_uid_override: Option<&str>,
        preserve_existing_folder: bool,
    ) -> Result<Option<String>>;

    fn determine_dashboard_import_action(
        &mut self,
        cache: &mut ImportLookupCache,
        payload: &Value,
        replace_existing: bool,
        update_existing_only: bool,
    ) -> Result<&'static str>;

    fn resolve_existing_dashboard_folder_path(
        &mut self,
        cache: &mut ImportLookupCache,
        uid: &str,
    ) -> Result<Option<String>>;

    fn ensure_folder_inventory_entry(
        &mut self,
        cache: &mut ImportLookupCache,
        folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
        folder_uid: &str,
    ) -> Result<()>;

    fn import_dashboard(&mut self, payload: &Value) -> Result<()>;
}

struct RequestImportBackend<'a, F>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_json: &'a mut F,
}

impl<'a, F> RequestImportBackend<'a, F>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    fn new(request_json: &'a mut F) -> Self {
        Self { request_json }
    }
}

impl<F> LiveImportBackend for RequestImportBackend<'_, F>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    fn validate_export_org(
        &mut self,
        cache: &mut ImportLookupCache,
        args: &ImportArgs,
        input_dir: &std::path::Path,
        metadata: Option<&ExportMetadata>,
    ) -> Result<()> {
        validate_matching_export_org_with_request(
            &mut *self.request_json,
            cache,
            args,
            input_dir,
            metadata,
            None,
        )
    }

    fn validate_dependencies(
        &mut self,
        input_dir: &std::path::Path,
        strict_schema: bool,
        target_schema_version: Option<i64>,
    ) -> Result<()> {
        validate_dashboard_import_dependencies_with_request(
            &mut *self.request_json,
            input_dir,
            strict_schema,
            target_schema_version,
        )
    }

    fn determine_import_folder_uid_override(
        &mut self,
        cache: &mut ImportLookupCache,
        uid: &str,
        folder_uid_override: Option<&str>,
        preserve_existing_folder: bool,
    ) -> Result<Option<String>> {
        determine_import_folder_uid_override_with_request(
            &mut *self.request_json,
            cache,
            uid,
            folder_uid_override,
            preserve_existing_folder,
        )
    }

    fn determine_dashboard_import_action(
        &mut self,
        cache: &mut ImportLookupCache,
        payload: &Value,
        replace_existing: bool,
        update_existing_only: bool,
    ) -> Result<&'static str> {
        determine_dashboard_import_action_with_request(
            &mut *self.request_json,
            cache,
            payload,
            replace_existing,
            update_existing_only,
        )
    }

    fn resolve_existing_dashboard_folder_path(
        &mut self,
        cache: &mut ImportLookupCache,
        uid: &str,
    ) -> Result<Option<String>> {
        resolve_existing_dashboard_folder_path_with_request(&mut *self.request_json, cache, uid)
    }

    fn ensure_folder_inventory_entry(
        &mut self,
        cache: &mut ImportLookupCache,
        folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
        folder_uid: &str,
    ) -> Result<()> {
        ensure_folder_inventory_entry_cached(
            &mut *self.request_json,
            cache,
            folders_by_uid,
            folder_uid,
        )
    }

    fn import_dashboard(&mut self, payload: &Value) -> Result<()> {
        let _ = import_dashboard_request_with_request(&mut *self.request_json, payload)?;
        Ok(())
    }
}

struct ClientImportBackend<'a> {
    dashboard: DashboardResourceClient<'a>,
}

impl<'a> ClientImportBackend<'a> {
    fn new(client: &'a JsonHttpClient) -> Self {
        Self {
            dashboard: DashboardResourceClient::new(client),
        }
    }
}

impl LiveImportBackend for ClientImportBackend<'_> {
    fn validate_export_org(
        &mut self,
        _cache: &mut ImportLookupCache,
        args: &ImportArgs,
        input_dir: &std::path::Path,
        metadata: Option<&ExportMetadata>,
    ) -> Result<()> {
        super::super::import_validation::validate_matching_export_org_with_client(
            &self.dashboard,
            args,
            input_dir,
            metadata,
            None,
        )
    }

    fn validate_dependencies(
        &mut self,
        input_dir: &std::path::Path,
        strict_schema: bool,
        target_schema_version: Option<i64>,
    ) -> Result<()> {
        super::super::import_validation::validate_dashboard_import_dependencies_with_client(
            &self.dashboard,
            input_dir,
            strict_schema,
            target_schema_version,
        )
    }

    fn determine_import_folder_uid_override(
        &mut self,
        cache: &mut ImportLookupCache,
        uid: &str,
        folder_uid_override: Option<&str>,
        preserve_existing_folder: bool,
    ) -> Result<Option<String>> {
        determine_import_folder_uid_override_with_client(
            &self.dashboard,
            cache,
            uid,
            folder_uid_override,
            preserve_existing_folder,
        )
    }

    fn determine_dashboard_import_action(
        &mut self,
        cache: &mut ImportLookupCache,
        payload: &Value,
        replace_existing: bool,
        update_existing_only: bool,
    ) -> Result<&'static str> {
        determine_dashboard_import_action_with_client(
            &self.dashboard,
            cache,
            payload,
            replace_existing,
            update_existing_only,
        )
    }

    fn resolve_existing_dashboard_folder_path(
        &mut self,
        cache: &mut ImportLookupCache,
        uid: &str,
    ) -> Result<Option<String>> {
        resolve_existing_dashboard_folder_path_with_client(&self.dashboard, cache, uid)
    }

    fn ensure_folder_inventory_entry(
        &mut self,
        cache: &mut ImportLookupCache,
        folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
        folder_uid: &str,
    ) -> Result<()> {
        ensure_folder_inventory_entry_with_client(
            &self.dashboard,
            cache,
            folders_by_uid,
            folder_uid,
        )
    }

    fn import_dashboard(&mut self, payload: &Value) -> Result<()> {
        let _ = self.dashboard.import_dashboard_request(payload)?;
        Ok(())
    }
}

struct PreparedImportRun {
    resolved_import: super::ResolvedDashboardImportSource,
    metadata: Option<ExportMetadata>,
    folders_by_uid: BTreeMap<String, FolderInventoryItem>,
    dashboard_files: Vec<PathBuf>,
}

fn validate_import_args(args: &ImportArgs) -> Result<()> {
    if args.table && !args.dry_run {
        return Err(message(
            "--table is only supported with --dry-run for import-dashboard.",
        ));
    }
    if args.json && !args.dry_run {
        return Err(message(
            "--json is only supported with --dry-run for import-dashboard.",
        ));
    }
    if args.table && args.json {
        return Err(message(
            "--table and --json are mutually exclusive for import-dashboard.",
        ));
    }
    if args.no_header && !args.table {
        return Err(message(
            "--no-header is only supported with --dry-run --table for import-dashboard.",
        ));
    }
    if !args.output_columns.is_empty() && !args.table {
        return Err(message(
            "--output-columns is only supported with --dry-run --table or table-like --output-format for import-dashboard.",
        ));
    }
    if args.require_matching_folder_path && args.import_folder_uid.is_some() {
        return Err(message(
            "--require-matching-folder-path cannot be combined with --import-folder-uid.",
        ));
    }
    if args.ensure_folders && args.import_folder_uid.is_some() {
        return Err(message(
            "--ensure-folders cannot be combined with --import-folder-uid.",
        ));
    }
    Ok(())
}

fn prepare_import_run<F>(
    select_dashboard_files: &mut F,
    args: &ImportArgs,
) -> Result<PreparedImportRun>
where
    F: FnMut(Vec<PathBuf>) -> Result<Option<Vec<PathBuf>>>,
{
    let resolved_import = super::resolve_import_source(args)?;
    let metadata = load_export_metadata(
        &resolved_import.metadata_dir,
        Some(super::import_metadata_variant(args)),
    )?;
    let folder_inventory = if args.ensure_folders || args.dry_run {
        load_folder_inventory(&resolved_import.metadata_dir, metadata.as_ref())?
    } else {
        Vec::new()
    };
    if args.ensure_folders && folder_inventory.is_empty() {
        let folders_file = metadata
            .as_ref()
            .and_then(|item| item.folders_file.as_deref())
            .unwrap_or(FOLDER_INVENTORY_FILENAME);
        return Err(message(format!(
            "Folder inventory file not found for --ensure-folders: {}. Re-export dashboards with raw folder inventory or omit --ensure-folders.",
            resolved_import.metadata_dir.join(folders_file).display()
        )));
    }
    let folders_by_uid = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect::<BTreeMap<String, FolderInventoryItem>>();
    let discovered_dashboard_files =
        super::dashboard_files_for_import(&resolved_import.dashboard_dir)?;
    let dashboard_files = match select_dashboard_files(discovered_dashboard_files.clone())? {
        Some(selected) => selected,
        None if args.interactive => {
            println!(
                "{} cancelled.",
                if args.dry_run {
                    "Interactive dry-run"
                } else {
                    "Import"
                }
            );
            return Ok(PreparedImportRun {
                resolved_import,
                metadata,
                folders_by_uid,
                dashboard_files: Vec::new(),
            });
        }
        None => discovered_dashboard_files,
    };
    Ok(PreparedImportRun {
        resolved_import,
        metadata,
        folders_by_uid,
        dashboard_files,
    })
}

fn run_live_import<B: LiveImportBackend>(
    backend: &mut B,
    args: &ImportArgs,
    prepared: &PreparedImportRun,
    lookup_cache: &mut ImportLookupCache,
) -> Result<usize> {
    let total = prepared.dashboard_files.len();
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let mut imported_count = 0usize;
    let mut skipped_missing_count = 0usize;
    let mut skipped_folder_mismatch_count = 0usize;
    let mode = super::import_render::describe_dashboard_import_mode(
        args.replace_existing,
        args.update_existing_only,
    );
    if !args.json {
        println!("Import mode: {}", mode);
    }
    for (index, dashboard_file) in prepared.dashboard_files.iter().enumerate() {
        let document = load_json_file(dashboard_file)?;
        if args.strict_schema {
            validate::validate_dashboard_import_document(
                &document,
                dashboard_file,
                true,
                args.target_schema_version,
            )?;
        }
        let document_object =
            crate::common::value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let uid = crate::common::string_field(dashboard, "uid", "");
        let source_folder_path = if args.require_matching_folder_path {
            Some(
                super::super::import_lookup::resolve_source_dashboard_folder_path(
                    &document,
                    dashboard_file,
                    &prepared.resolved_import.metadata_dir,
                    &prepared.folders_by_uid,
                )?,
            )
        } else {
            None
        };
        let folder_uid_override = backend.determine_import_folder_uid_override(
            lookup_cache,
            &uid,
            args.import_folder_uid.as_deref(),
            effective_replace_existing,
        )?;
        let payload = build_import_payload(
            &document,
            folder_uid_override.as_deref(),
            effective_replace_existing,
            &args.import_message,
        )?;
        let action = if args.update_existing_only
            || args.ensure_folders
            || args.require_matching_folder_path
        {
            Some(backend.determine_dashboard_import_action(
                lookup_cache,
                &payload,
                args.replace_existing,
                args.update_existing_only,
            )?)
        } else {
            None
        };
        let destination_folder_path = if args.require_matching_folder_path {
            backend.resolve_existing_dashboard_folder_path(lookup_cache, &uid)?
        } else {
            None
        };
        let (
            folder_paths_match,
            _folder_match_reason,
            normalized_source_folder_path,
            normalized_destination_folder_path,
        ) = if args.require_matching_folder_path {
            build_folder_path_match_result(
                source_folder_path.as_deref(),
                destination_folder_path.as_deref(),
                destination_folder_path.is_some(),
                true,
            )
        } else {
            (true, "", String::new(), None::<String>)
        };
        let action =
            action.map(|value| apply_folder_path_guard_to_action(value, folder_paths_match));
        if args.update_existing_only || args.require_matching_folder_path {
            let payload_object = crate::common::value_as_object(
                &payload,
                "Dashboard import payload must be a JSON object.",
            )?;
            let dashboard = payload_object
                .get("dashboard")
                .and_then(Value::as_object)
                .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
            let uid = crate::common::string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
            if action == Some("would-skip-missing") {
                skipped_missing_count += 1;
                if args.verbose {
                    println!(
                        "Skipped import uid={} dest=missing action=skip-missing file={}",
                        uid,
                        dashboard_file.display()
                    );
                } else if args.progress {
                    println!(
                        "Skipping dashboard {}/{}: {} dest=missing action=skip-missing",
                        index + 1,
                        total,
                        uid
                    );
                }
                continue;
            }
            if action == Some("would-skip-folder-mismatch") {
                skipped_folder_mismatch_count += 1;
                if args.verbose {
                    println!(
                        "Skipped import uid={} dest=exists action=skip-folder-mismatch sourceFolderPath={} destinationFolderPath={} file={}",
                        uid,
                        normalized_source_folder_path,
                        normalized_destination_folder_path.as_deref().unwrap_or("-"),
                        dashboard_file.display()
                    );
                } else if args.progress {
                    println!(
                        "Skipping dashboard {}/{}: {} dest=exists action=skip-folder-mismatch",
                        index + 1,
                        total,
                        uid
                    );
                }
                continue;
            }
        }
        if args.ensure_folders {
            let payload_object = crate::common::value_as_object(
                &payload,
                "Dashboard import payload must be a JSON object.",
            )?;
            let folder_uid = payload_object
                .get("folderUid")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !folder_uid.is_empty() && action != Some("would-fail-existing") {
                backend.ensure_folder_inventory_entry(
                    lookup_cache,
                    &prepared.folders_by_uid,
                    folder_uid,
                )?;
            }
        }
        backend.import_dashboard(&payload)?;
        imported_count += 1;
        if args.verbose {
            println!(
                "{}",
                format_import_verbose_line(dashboard_file, false, None, None, None)
            );
        } else if args.progress {
            println!(
                "{}",
                format_import_progress_line(
                    index + 1,
                    total,
                    &dashboard_file.display().to_string(),
                    false,
                    None,
                    None,
                )
            );
        }
    }
    if args.update_existing_only && skipped_missing_count > 0 && skipped_folder_mismatch_count > 0 {
        println!(
            "Imported {} dashboard files from {}; skipped {} missing dashboards and {} folder-mismatched dashboards",
            imported_count,
            args.input_dir.display(),
            skipped_missing_count,
            skipped_folder_mismatch_count
        );
    } else if args.update_existing_only && skipped_missing_count > 0 {
        println!(
            "Imported {} dashboard files from {}; skipped {} missing dashboards",
            imported_count,
            args.input_dir.display(),
            skipped_missing_count
        );
    } else if skipped_folder_mismatch_count > 0 {
        println!(
            "Imported {} dashboard files from {}; skipped {} folder-mismatched dashboards",
            imported_count,
            args.input_dir.display(),
            skipped_folder_mismatch_count
        );
    } else {
        println!(
            "Imported {} dashboard files from {}",
            imported_count,
            args.input_dir.display()
        );
    }
    Ok(imported_count)
}

fn render_dry_run_report(
    report: &super::super::import_render::ImportDryRunReport,
    args: &ImportArgs,
) -> Result<usize> {
    if args.json {
        print!(
            "{}",
            render_import_dry_run_json(
                &report.mode,
                &report.folder_statuses,
                &report.dashboard_records,
                &report.input_dir,
                report.skipped_missing_count,
                report.skipped_folder_mismatch_count,
            )?
        );
    } else {
        folder_inventory_status_output_lines(
            &report.folder_statuses,
            args.no_header,
            args.json,
            args.table,
        );
        if args.table {
            for line in render_import_dry_run_table(
                &report.dashboard_records,
                !args.no_header,
                if args.output_columns.is_empty() {
                    None
                } else {
                    Some(args.output_columns.as_slice())
                },
            ) {
                println!("{line}");
            }
        } else if args.verbose {
            for record in &report.dashboard_records {
                if record[3].is_empty() {
                    println!(
                        "Dry-run import uid={} dest={} action={} file={}",
                        record[0], record[1], record[2], record[7]
                    );
                } else {
                    println!(
                        "Dry-run import uid={} dest={} action={} folderPath={} file={}",
                        record[0], record[1], record[2], record[3], record[7]
                    );
                }
            }
        } else if args.progress {
            for (index, record) in report.dashboard_records.iter().enumerate() {
                if record[3].is_empty() {
                    println!(
                        "Dry-run dashboard {}/{}: {} dest={} action={}",
                        index + 1,
                        report.dashboard_records.len(),
                        record[0],
                        record[1],
                        record[2]
                    );
                } else {
                    println!(
                        "Dry-run dashboard {}/{}: {} dest={} action={} folderPath={}",
                        index + 1,
                        report.dashboard_records.len(),
                        record[0],
                        record[1],
                        record[2],
                        record[3]
                    );
                }
            }
        }
        if args.update_existing_only
            && report.skipped_missing_count > 0
            && report.skipped_folder_mismatch_count > 0
        {
            println!(
                "Dry-run checked {} dashboard(s) from {}; would skip {} missing dashboards and {} folder-mismatched dashboards",
                report.dashboard_records.len(),
                report.input_dir.display(),
                report.skipped_missing_count,
                report.skipped_folder_mismatch_count
            );
        } else if args.update_existing_only && report.skipped_missing_count > 0 {
            println!(
                "Dry-run checked {} dashboard(s) from {}; would skip {} missing dashboards",
                report.dashboard_records.len(),
                report.input_dir.display(),
                report.skipped_missing_count
            );
        } else if report.skipped_folder_mismatch_count > 0 {
            println!(
                "Dry-run checked {} dashboard(s) from {}; would skip {} folder-mismatched dashboards",
                report.dashboard_records.len(),
                report.input_dir.display(),
                report.skipped_folder_mismatch_count
            );
        } else {
            println!(
                "Dry-run checked {} dashboard(s) from {}",
                report.dashboard_records.len(),
                report.input_dir.display()
            );
        }
    }
    Ok(report.dashboard_records.len())
}

/// Purpose: implementation note.
pub fn diff_dashboards_with_client(client: &JsonHttpClient, args: &DiffArgs) -> Result<usize> {
    diff_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

/// Purpose: implementation note.
pub(crate) fn import_dashboards_with_request<F>(
    mut request_json: F,
    args: &ImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if !args.dry_run {
        validate_import_args(args)?;
        let mut lookup_cache = ImportLookupCache::default();
        let prepared = prepare_import_run(
            &mut |dashboard_files| {
                #[cfg(feature = "tui")]
                {
                    super::selected_dashboard_files(
                        &mut request_json,
                        &mut lookup_cache,
                        args,
                        dashboard_files,
                    )
                }
                #[cfg(not(feature = "tui"))]
                {
                    super::selected_dashboard_files(args, dashboard_files)
                }
            },
            args,
        )?;
        if prepared.dashboard_files.is_empty() {
            return Ok(0);
        }
        let mut backend = RequestImportBackend::new(&mut request_json);
        backend.validate_export_org(
            &mut lookup_cache,
            args,
            &prepared.resolved_import.metadata_dir,
            prepared.metadata.as_ref(),
        )?;
        backend.validate_dependencies(
            &prepared.resolved_import.dashboard_dir,
            args.strict_schema,
            args.target_schema_version,
        )?;
        return run_live_import(&mut backend, args, &prepared, &mut lookup_cache);
    }
    validate_import_args(args)?;
    let report = collect_import_dry_run_report_with_request(&mut request_json, args)?;
    render_dry_run_report(&report, args)
}

/// Purpose: implementation note.
pub fn import_dashboards_with_client(client: &JsonHttpClient, args: &ImportArgs) -> Result<usize> {
    if args.dry_run {
        let report = collect_import_dry_run_report_with_client(client, args)?;
        return render_dry_run_report(&report, args);
    }
    validate_import_args(args)?;
    let mut backend = ClientImportBackend::new(client);
    let mut lookup_cache = ImportLookupCache::default();
    let prepared = prepare_import_run(
        &mut |_dashboard_files| {
            #[cfg(feature = "tui")]
            {
                import_interactive::select_import_dashboard_files_with_client(
                    &backend.dashboard,
                    &mut lookup_cache,
                    args,
                )
            }
            #[cfg(not(feature = "tui"))]
            {
                let _ = _dashboard_files;
                if args.interactive {
                    return super::super::tui_not_built("import --interactive");
                }
                Ok(None)
            }
        },
        args,
    )?;
    if prepared.dashboard_files.is_empty() {
        return Ok(0);
    }
    backend.validate_export_org(
        &mut lookup_cache,
        args,
        &prepared.resolved_import.metadata_dir,
        prepared.metadata.as_ref(),
    )?;
    backend.validate_dependencies(
        &prepared.resolved_import.dashboard_dir,
        args.strict_schema,
        args.target_schema_version,
    )?;
    run_live_import(&mut backend, args, &prepared, &mut lookup_cache)
}

/// Purpose: implementation note.
pub(crate) fn import_dashboards_with_org_clients(args: &ImportArgs) -> Result<usize> {
    let context = super::build_import_auth_context(args)?;
    let client = JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url.clone(),
        headers: context.headers.clone(),
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })?;
    if !args.use_export_org {
        return import_dashboards_with_request(
            |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
        );
    }
    super::super::import_routed::import_dashboards_by_export_org_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        |target_org_id, scoped_args| {
            let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
            import_dashboards_with_client(&scoped_client, scoped_args)
        },
        |target_org_id, scoped_args| {
            let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
            collect_import_dry_run_report_with_request(
                |method, path, params, payload| {
                    scoped_client.request_json(method, path, params, payload)
                },
                scoped_args,
            )
        },
        args,
    )
}
