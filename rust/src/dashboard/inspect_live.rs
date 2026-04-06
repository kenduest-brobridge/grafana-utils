//! Live dashboard inspection staging and export orchestration.
//!
//! This module turns live Grafana reads into the same raw export contract consumed by the
//! offline inspect pipeline, so the exported files and the live request path can share the
//! same summary/report/governance builders.
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use reqwest::Method;
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{message, string_field, validation, value_as_object, GrafanaCliError, Result};
use crate::grafana_api::{DashboardResourceClient, DatasourceResourceClient};
use crate::http::JsonHttpClient;

use super::cli_defs::{ExportArgs, InspectExportArgs, InspectLiveArgs};
use super::export::{build_output_path, export_dashboards_with_request};
use super::files::{
    build_export_metadata, load_export_metadata, write_dashboard, write_json_document,
};
use super::inspect::analyze_export_dir;
#[cfg(feature = "tui")]
use super::inspect::build_export_inspection_query_report;
#[cfg(feature = "tui")]
use super::inspect::build_export_inspection_summary;
#[cfg(feature = "tui")]
use super::inspect_governance::build_export_inspection_governance_document;
#[cfg(feature = "tui")]
use super::inspect_live_tui::run_inspect_live_interactive as run_inspect_live_tui;
use super::live::build_datasource_inventory_record;
use super::{
    FolderInventoryItem, DATASOURCE_INVENTORY_FILENAME, DEFAULT_FOLDER_TITLE, DEFAULT_ORG_ID,
    DEFAULT_ORG_NAME, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME, RAW_EXPORT_SUBDIR,
};

const MAX_LIVE_INSPECT_CONCURRENCY: usize = 16;
const LIVE_INSPECT_RETRY_LIMIT: usize = 3;
const LIVE_INSPECT_RETRY_BACKOFF_MS: u64 = 200;

pub(crate) struct TempInspectDir {
    pub(crate) path: PathBuf,
}

impl TempInspectDir {
    pub(crate) fn new(prefix: &str) -> Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| validation(format!("Failed to build {prefix} temp path: {error}")))?
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "grafana-utils-{prefix}-{}-{timestamp}",
            process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempInspectDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn build_analysis_live_export_args(
    common: &crate::dashboard::CommonCliArgs,
    export_dir: PathBuf,
    page_size: usize,
    org_id: Option<i64>,
    all_orgs: bool,
) -> ExportArgs {
    ExportArgs {
        common: common.clone(),
        export_dir,
        page_size,
        org_id,
        all_orgs,
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
        progress: false,
        verbose: false,
    }
}

fn build_live_export_args(args: &InspectLiveArgs, export_dir: PathBuf) -> ExportArgs {
    let mut export_args = build_analysis_live_export_args(
        &args.common,
        export_dir,
        args.page_size,
        args.org_id,
        args.all_orgs,
    );
    export_args.progress = args.progress;
    export_args
}

pub(crate) fn prepare_live_analysis_import_dir(temp_root: &Path, all_orgs: bool) -> Result<PathBuf> {
    if !all_orgs {
        return Ok(temp_root.join(RAW_EXPORT_SUBDIR));
    }

    let inspect_raw_dir = temp_root
        .join("inspect-live-all-orgs")
        .join(RAW_EXPORT_SUBDIR);
    let org_raw_dirs = discover_org_variant_export_dirs(temp_root, RAW_EXPORT_SUBDIR)?;
    merge_org_variant_exports_into_dir(&org_raw_dirs, &inspect_raw_dir, None, RAW_EXPORT_SUBDIR)?;
    Ok(inspect_raw_dir)
}

fn build_live_scan_progress_bar(total: usize) -> ProgressBar {
    let bar = ProgressBar::new(total as u64);
    let style = ProgressStyle::with_template(
        "{spinner:.green} dashboards {pos}/{len} [{bar:40.cyan/blue}] {msg}",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar());
    bar.set_style(style.progress_chars("##-"));
    bar
}

fn bounded_live_inspect_concurrency(requested: usize) -> usize {
    requested.clamp(1, MAX_LIVE_INSPECT_CONCURRENCY)
}

fn is_retryable_live_fetch_error(error: &GrafanaCliError) -> bool {
    matches!(error.status_code(), Some(429 | 502 | 503 | 504))
}

fn fetch_live_dashboard_with_retries<F>(uid: &str, fetch_dashboard: &F) -> Result<Value>
where
    F: Fn(&str) -> Result<Value> + Sync,
{
    let mut attempt = 0usize;
    loop {
        attempt += 1;
        match fetch_dashboard(uid) {
            Ok(payload) => return Ok(payload),
            Err(error)
                if is_retryable_live_fetch_error(&error) && attempt < LIVE_INSPECT_RETRY_LIMIT =>
            {
                thread::sleep(Duration::from_millis(
                    LIVE_INSPECT_RETRY_BACKOFF_MS * attempt as u64,
                ));
            }
            Err(error) => {
                return Err(error.with_context(format!(
                    "Failed to fetch live dashboard uid={uid} during analyze-live"
                )))
            }
        }
    }
}

pub(crate) fn snapshot_live_dashboard_export_with_fetcher<F>(
    raw_dir: &Path,
    summaries: &[Map<String, Value>],
    concurrency: usize,
    progress: bool,
    fetch_dashboard: F,
) -> Result<usize>
where
    F: Fn(&str) -> Result<Value> + Sync,
{
    // Only dashboard fetches fan out; the file layout and progress semantics stay
    // deterministic so the staged export remains comparable to offline export roots.
    fs::create_dir_all(raw_dir)?;
    let progress_bar = if progress {
        Some(build_live_scan_progress_bar(summaries.len()))
    } else {
        None
    };
    let thread_count = bounded_live_inspect_concurrency(concurrency);
    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .map_err(|error| {
            message(format!(
                "Failed to build dashboard scan worker pool: {error}"
            ))
        })?;
    let results = pool.install(|| {
        summaries
            .par_iter()
            .map(|summary| {
                let uid = string_field(summary, "uid", "");
                let output_path = build_output_path(raw_dir, summary, false);
                let payload = fetch_live_dashboard_with_retries(&uid, &fetch_dashboard)?;
                write_dashboard(&payload, &output_path, false).map_err(|error| {
                    error.with_context(format!(
                        "Failed to stage live dashboard uid={uid} into {}",
                        output_path.display()
                    ))
                })?;
                if let Some(bar) = progress_bar.as_ref() {
                    bar.inc(1);
                    bar.set_message(uid);
                }
                Ok::<(), crate::common::GrafanaCliError>(())
            })
            .collect::<Vec<_>>()
    });
    if let Some(bar) = progress_bar.as_ref() {
        bar.finish_and_clear();
    }
    for result in results {
        result?;
    }
    Ok(summaries.len())
}

fn collect_folder_inventory_from_summaries(
    dashboard: &DashboardResourceClient<'_>,
    summaries: &[Map<String, Value>],
) -> Result<Vec<FolderInventoryItem>> {
    let mut seen = std::collections::BTreeSet::new();
    let mut folders = Vec::new();
    for summary in summaries {
        let folder_uid = string_field(summary, "folderUid", "");
        if folder_uid.is_empty() {
            continue;
        }
        let org_id = summary
            .get("orgId")
            .map(|value| match value {
                Value::String(text) => text.clone(),
                _ => value.to_string(),
            })
            .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
        let key = format!("{org_id}:{folder_uid}");
        if seen.contains(&key) {
            continue;
        }
        let Some(folder) = dashboard.fetch_folder_if_exists(&folder_uid)? else {
            continue;
        };
        let org = string_field(summary, "orgName", DEFAULT_ORG_NAME);
        let mut parent_path = Vec::new();
        let mut previous_parent_uid = None;
        if let Some(parents) = folder.get("parents").and_then(Value::as_array) {
            for parent in parents {
                let Some(parent_object) = parent.as_object() else {
                    continue;
                };
                let parent_uid = string_field(parent_object, "uid", "");
                let parent_title = string_field(parent_object, "title", "");
                if parent_uid.is_empty() || parent_title.is_empty() {
                    continue;
                }
                parent_path.push(parent_title.clone());
                let parent_key = format!("{org_id}:{parent_uid}");
                if !seen.contains(&parent_key) {
                    folders.push(FolderInventoryItem {
                        uid: parent_uid.clone(),
                        title: parent_title,
                        path: parent_path.join(" / "),
                        parent_uid: previous_parent_uid.clone(),
                        org: org.clone(),
                        org_id: org_id.clone(),
                    });
                    seen.insert(parent_key);
                }
                previous_parent_uid = Some(parent_uid);
            }
        }
        let folder_title = string_field(&folder, "title", DEFAULT_FOLDER_TITLE);
        parent_path.push(folder_title.clone());
        folders.push(FolderInventoryItem {
            uid: folder_uid.clone(),
            title: folder_title,
            path: parent_path.join(" / "),
            parent_uid: previous_parent_uid,
            org,
            org_id: org_id.clone(),
        });
        seen.insert(key);
    }
    folders.sort_by(|left, right| {
        left.org_id
            .cmp(&right.org_id)
            .then(left.path.cmp(&right.path))
            .then(left.uid.cmp(&right.uid))
    });
    Ok(folders)
}

pub(crate) fn load_variant_index_entries(
    import_dir: &Path,
    metadata: Option<&super::models::ExportMetadata>,
) -> Result<Vec<super::models::VariantIndexEntry>> {
    let index_file = metadata
        .map(|item| item.index_file.clone())
        .unwrap_or_else(|| "index.json".to_string());
    let index_path = import_dir.join(&index_file);
    if !index_path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&index_path)?;
    let entries: Vec<super::models::VariantIndexEntry> = serde_json::from_str(&raw)?;
    Ok(entries)
}

#[cfg(feature = "tui")]
fn run_interactive_inspect_live_tui_from_dir(import_dir: &Path) -> Result<usize> {
    eprintln!(
        "[analyze-live --interactive] building summary: {}",
        import_dir.display()
    );
    let summary = build_export_inspection_summary(import_dir)?;
    eprintln!(
        "[analyze-live --interactive] building query report: {}",
        import_dir.display()
    );
    let report = build_export_inspection_query_report(import_dir)?;
    eprintln!(
        "[analyze-live --interactive] building governance review: {}",
        import_dir.display()
    );
    let governance = build_export_inspection_governance_document(&summary, &report);
    eprintln!(
        "[analyze-live --interactive] launching analysis workbench: {}",
        import_dir.display()
    );
    run_inspect_live_tui(&summary, &governance, &report)?;
    Ok(summary.dashboard_count)
}

#[cfg(not(feature = "tui"))]
fn run_interactive_inspect_live_tui_from_dir(_import_dir: &Path) -> Result<usize> {
    super::tui_not_built("analyze-live --interactive")
}

pub(crate) fn inspect_live_dashboards_with_client(
    client: &JsonHttpClient,
    args: &InspectLiveArgs,
) -> Result<usize> {
    // Small or multi-org scans take the request-parity path; the threaded client path is
    // only for the single-org case where live fetch fan-out is safe and useful.
    if args.all_orgs || args.concurrency <= 1 {
        return inspect_live_dashboards_with_request(
            |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
        );
    }

    let temp_dir = TempInspectDir::new("inspect-live")?;
    let raw_dir = temp_dir.path.join(RAW_EXPORT_SUBDIR);
    let dashboard = DashboardResourceClient::new(client);
    let datasource = DatasourceResourceClient::new(client);
    let summaries = dashboard.list_dashboard_summaries(args.page_size)?;
    let datasource_items = datasource.list_datasources()?;
    let org_value = dashboard
        .fetch_current_org()
        .map(Value::Object)
        .unwrap_or_else(|_| serde_json::json!({"id": 1, "name": "Main Org."}));
    let org = value_as_object(&org_value, "Unexpected current org payload from Grafana.")?;
    let datasource_inventory = datasource_items
        .iter()
        .map(|item| build_datasource_inventory_record(item, org))
        .collect::<Vec<_>>();
    snapshot_live_dashboard_export_with_fetcher(
        &raw_dir,
        &summaries,
        args.concurrency,
        args.progress,
        |uid| dashboard.fetch_dashboard(uid),
    )?;
    write_json_document(
        &datasource_inventory,
        &raw_dir.join(DATASOURCE_INVENTORY_FILENAME),
    )?;
    let folder_inventory = collect_folder_inventory_from_summaries(&dashboard, &summaries)?;
    write_json_document(&folder_inventory, &raw_dir.join(FOLDER_INVENTORY_FILENAME))?;
    if args.interactive {
        return run_interactive_inspect_live_tui_from_dir(&raw_dir);
    }
    let inspect_args = build_export_inspect_args_from_live(args, raw_dir);
    analyze_export_dir(&inspect_args)
}

fn build_export_inspect_args_from_live(
    args: &InspectLiveArgs,
    import_dir: PathBuf,
) -> InspectExportArgs {
    InspectExportArgs {
        import_dir,
        input_type: None,
        input_format: super::DashboardImportInputFormat::Raw,
        text: args.text,
        csv: args.csv,
        json: args.json,
        table: args.table,
        yaml: args.yaml,
        output_format: args.output_format,
        output_file: args.output_file.clone(),
        also_stdout: args.also_stdout,
        report_columns: args.report_columns.clone(),
        report_filter_datasource: args.report_filter_datasource.clone(),
        report_filter_panel_id: args.report_filter_panel_id.clone(),
        help_full: args.help_full,
        no_header: args.no_header,
        interactive: args.interactive,
    }
}

fn load_json_array_file(path: &Path, error_context: &str) -> Result<Vec<Value>> {
    let raw = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&raw)?;
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(message(format!(
            "{error_context} must be a JSON array: {}",
            path.display()
        ))),
    }
}

fn discover_org_variant_export_dirs(
    import_dir: &Path,
    expected_variant: &'static str,
) -> Result<Vec<(String, PathBuf)>> {
    if !import_dir.exists() {
        return Err(message(format!(
            "Import directory does not exist: {}",
            import_dir.display()
        )));
    }
    if !import_dir.is_dir() {
        return Err(message(format!(
            "Import path is not a directory: {}",
            import_dir.display()
        )));
    }
    let mut org_raw_dirs = Vec::new();
    for entry in fs::read_dir(import_dir)? {
        let entry = entry?;
        let org_root = entry.path();
        if !org_root.is_dir() {
            continue;
        }
        let org_name = entry.file_name().to_string_lossy().to_string();
        if !org_name.starts_with("org_") {
            continue;
        }
        let org_variant_dir = org_root.join(expected_variant);
        if org_variant_dir.is_dir() {
            org_raw_dirs.push((org_name, org_variant_dir));
        }
    }
    org_raw_dirs.sort_by(|left, right| left.0.cmp(&right.0));
    if org_raw_dirs.is_empty() {
        return Err(message(format!(
            "Import path {} does not contain any org-scoped {expected_variant}/ exports. Point --import-dir at a combined multi-org dashboard export root that includes that variant.",
            import_dir.display(),
        )));
    }
    Ok(org_raw_dirs)
}

fn merge_org_variant_exports_into_dir(
    org_variant_dirs: &[(String, PathBuf)],
    inspect_variant_dir: &Path,
    source_root: Option<&Path>,
    expected_variant: &'static str,
) -> Result<()> {
    // Preserve each org's raw export content and rewrite the index paths so the merged
    // tree still looks like one export root to the downstream inspect readers.
    fs::create_dir_all(inspect_variant_dir)?;

    let mut folder_inventory = Vec::new();
    let mut datasource_inventory = Vec::new();
    let mut index_entries = Vec::new();
    let mut dashboard_count = 0usize;

    for (org_name, org_variant_dir) in org_variant_dirs {
        let relative_target = inspect_variant_dir.join(org_name);
        copy_dir_recursive(org_variant_dir, &relative_target)?;

        let metadata = load_export_metadata(org_variant_dir, Some(expected_variant))?;
        let org_index_entries = load_variant_index_entries(org_variant_dir, metadata.as_ref())?;
        for mut entry in org_index_entries.clone() {
            entry.path = format!("{org_name}/{}", entry.path);
            index_entries.push(entry);
        }

        let folder_path = org_variant_dir.join(FOLDER_INVENTORY_FILENAME);
        if folder_path.is_file() {
            folder_inventory.extend(load_json_array_file(
                &folder_path,
                "Dashboard folder inventory",
            )?);
        }
        let datasource_path = org_variant_dir.join(DATASOURCE_INVENTORY_FILENAME);
        if datasource_path.is_file() {
            datasource_inventory.extend(load_json_array_file(
                &datasource_path,
                "Dashboard datasource inventory",
            )?);
        }

        dashboard_count += metadata
            .as_ref()
            .map(|item| item.dashboard_count as usize)
            .unwrap_or(org_index_entries.len());
    }

    write_json_document(
        &build_export_metadata(
            expected_variant,
            dashboard_count,
            Some("grafana-web-import-preserve-uid"),
            Some(FOLDER_INVENTORY_FILENAME),
            Some(DATASOURCE_INVENTORY_FILENAME),
            None,
            None,
            None,
            None,
        ),
        &inspect_variant_dir.join(EXPORT_METADATA_FILENAME),
    )?;
    write_json_document(&index_entries, &inspect_variant_dir.join("index.json"))?;
    write_json_document(
        &folder_inventory,
        &inspect_variant_dir.join(FOLDER_INVENTORY_FILENAME),
    )?;
    write_json_document(
        &datasource_inventory,
        &inspect_variant_dir.join(DATASOURCE_INVENTORY_FILENAME),
    )?;
    if let Some(source_root) = source_root {
        fs::write(
            inspect_variant_dir.join(".inspect-source-root"),
            source_root.display().to_string(),
        )?;
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

pub(crate) fn prepare_inspect_live_import_dir(
    temp_root: &Path,
    args: &InspectLiveArgs,
) -> Result<PathBuf> {
    prepare_live_analysis_import_dir(temp_root, args.all_orgs)
}

#[cfg(test)]
pub(crate) fn prepare_inspect_export_import_dir(
    temp_root: &Path,
    import_dir: &Path,
) -> Result<PathBuf> {
    prepare_inspect_export_import_dir_for_variant(temp_root, import_dir, RAW_EXPORT_SUBDIR)
}

pub(crate) fn prepare_inspect_export_import_dir_for_variant(
    temp_root: &Path,
    import_dir: &Path,
    expected_variant: &'static str,
) -> Result<PathBuf> {
    // Root exports need one synthetic raw tree so offline inspect sees the same layout
    // whether the source was a live all-org fetch or a prebuilt export archive.
    let root_manifest = super::files::resolve_dashboard_export_root(import_dir)?;
    if root_manifest
        .as_ref()
        .map(|resolved| resolved.manifest.scope_kind.is_root())
        .unwrap_or(false)
    {
        let inspect_variant_dir = temp_root
            .join("inspect-export-all-orgs")
            .join(expected_variant);
        let org_variant_dirs = discover_org_variant_export_dirs(import_dir, expected_variant)?;
        merge_org_variant_exports_into_dir(
            &org_variant_dirs,
            &inspect_variant_dir,
            Some(import_dir),
            expected_variant,
        )?;
        return Ok(inspect_variant_dir);
    }
    Ok(import_dir.to_path_buf())
}

pub(crate) fn inspect_live_dashboards_with_request<F>(
    mut request_json: F,
    args: &InspectLiveArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    // Live inspect first stages a raw export tree, then reuses the offline analyzer so the
    // live and export code paths stay behaviorally aligned.
    let temp_dir = TempInspectDir::new("inspect-live")?;
    let export_args = build_live_export_args(args, temp_dir.path.clone());
    let _ = export_dashboards_with_request(&mut request_json, &export_args)?;
    let inspect_import_dir = prepare_inspect_live_import_dir(&temp_dir.path, args)?;
    if args.interactive {
        return run_interactive_inspect_live_tui_from_dir(&inspect_import_dir);
    }
    let inspect_args = build_export_inspect_args_from_live(args, inspect_import_dir);
    analyze_export_dir(&inspect_args)
}
