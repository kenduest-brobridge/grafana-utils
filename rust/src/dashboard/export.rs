//! Export dashboards into staged artifact trees.
//!
//! Responsibilities:
//! - Resolve dashboards from live/org metadata and normalize folder hierarchy.
//! - Render staged export variants (`raw`, `prompt`, `provisioning`) using shared helpers.
//! - Build dashboard export metadata/index artifacts and stream operator-facing progress/output.
use reqwest::Method;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, sanitize_path_component, string_field, Result};
use crate::grafana_api::DashboardResourceClient;
use crate::http::JsonHttpClient;

use super::history::build_dashboard_history_export_document_with_request;
use super::list::{
    attach_dashboard_org_metadata, collect_dashboard_source_metadata,
    fetch_current_org_with_request, list_orgs_with_request, org_id_value,
};
use super::{
    build_api_client, build_datasource_catalog, build_datasource_inventory_record,
    build_export_metadata, build_external_export_document, build_http_client,
    build_http_client_for_org, build_http_client_for_org_from_api,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    fetch_dashboard_with_request, list_dashboard_summaries_with_request,
    list_datasources_with_request, write_dashboard, write_json_document, DashboardIndexItem,
    ExportArgs, ExportOrgSummary, FolderInventoryItem, RootExportIndex,
    DASHBOARD_PERMISSION_BUNDLE_FILENAME, DATASOURCE_INVENTORY_FILENAME, DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE, DEFAULT_UNKNOWN_UID, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    PROMPT_EXPORT_SUBDIR, PROVISIONING_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR,
};
#[path = "export_support.rs"]
mod export_support;

use self::export_support::{
    build_all_orgs_output_dir, build_permission_bundle_document, build_used_datasource_summaries,
    collect_permission_export_documents,
};

pub(crate) struct ScopeExportResult {
    exported_count: usize,
    root_index: Option<RootExportIndex>,
    org_summary: Option<ExportOrgSummary>,
}

pub fn build_output_path(output_dir: &Path, summary: &Map<String, Value>, flat: bool) -> PathBuf {
    let folder_title = string_field(summary, "folderTitle", DEFAULT_FOLDER_TITLE);
    let title = string_field(summary, "title", DEFAULT_DASHBOARD_TITLE);
    let uid = string_field(summary, "uid", DEFAULT_UNKNOWN_UID);
    let file_name = format!(
        "{}__{}.json",
        sanitize_path_component(&title),
        sanitize_path_component(&uid)
    );

    if flat {
        output_dir.join(file_name)
    } else {
        output_dir
            .join(sanitize_path_component(&folder_title))
            .join(file_name)
    }
}

pub fn build_export_variant_dirs(output_dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        output_dir.join(RAW_EXPORT_SUBDIR),
        output_dir.join(PROMPT_EXPORT_SUBDIR),
        output_dir.join(PROVISIONING_EXPORT_SUBDIR),
    )
}

fn build_history_output_path(history_dir: &Path, uid: &str) -> PathBuf {
    history_dir.join(format!("{}.history.json", sanitize_path_component(uid)))
}

fn write_history_document<T: Serialize>(
    payload: &T,
    output_path: &Path,
    overwrite: bool,
) -> Result<()> {
    if output_path.exists() && !overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            output_path.display()
        )));
    }
    write_json_document(payload, output_path)
}

#[derive(Serialize)]
struct ProvisioningOptions {
    path: String,
    #[serde(rename = "foldersFromFilesStructure")]
    folders_from_files_structure: bool,
}

#[derive(Serialize)]
struct ProvisioningProvider {
    name: String,
    #[serde(rename = "orgId")]
    org_id: i64,
    #[serde(rename = "type")]
    provider_type: String,
    #[serde(rename = "disableDeletion")]
    disable_deletion: bool,
    #[serde(rename = "allowUiUpdates")]
    allow_ui_updates: bool,
    #[serde(rename = "updateIntervalSeconds")]
    update_interval_seconds: i64,
    options: ProvisioningOptions,
}

#[derive(Serialize)]
struct ProvisioningConfig {
    #[serde(rename = "apiVersion")]
    api_version: i64,
    providers: Vec<ProvisioningProvider>,
}

fn build_provisioning_config(
    org_id: i64,
    provider_name: &str,
    dashboard_path: &Path,
    disable_deletion: bool,
    allow_ui_updates: bool,
    update_interval_seconds: i64,
) -> ProvisioningConfig {
    let path = dashboard_path
        .canonicalize()
        .unwrap_or_else(|_| dashboard_path.to_path_buf())
        .display()
        .to_string();
    ProvisioningConfig {
        api_version: 1,
        providers: vec![ProvisioningProvider {
            name: provider_name.to_string(),
            org_id,
            provider_type: "file".to_string(),
            disable_deletion,
            allow_ui_updates,
            update_interval_seconds,
            options: ProvisioningOptions {
                path,
                folders_from_files_structure: true,
            },
        }],
    }
}

fn write_yaml_document<T: Serialize>(
    payload: &T,
    output_path: &Path,
    overwrite: bool,
) -> Result<()> {
    if output_path.exists() && !overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            output_path.display()
        )));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let rendered = serde_yaml::to_string(payload).map_err(|error| {
        message(format!(
            "Failed to serialize YAML document for {}: {error}",
            output_path.display()
        ))
    })?;
    fs::write(output_path, rendered)?;
    Ok(())
}

/// format export progress line.
pub(crate) fn format_export_progress_line(
    current: usize,
    total: usize,
    uid: &str,
    dry_run: bool,
) -> String {
    format!(
        "{} dashboard {current}/{total}: {uid}",
        if dry_run { "Would export" } else { "Exporting" }
    )
}

/// format export verbose line.
pub(crate) fn format_export_verbose_line(
    kind: &str,
    uid: &str,
    path: &Path,
    dry_run: bool,
) -> String {
    format!(
        "{} {kind:<6} {uid} -> {}",
        if dry_run { "Would export" } else { "Exported" },
        path.display()
    )
}

pub(crate) fn export_dashboards_in_scope_with_request<F>(
    request_json: &mut F,
    args: &ExportArgs,
    org: Option<&Map<String, Value>>,
    org_id_override: Option<i64>,
) -> Result<ScopeExportResult>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.without_dashboard_raw
        && args.without_dashboard_prompt
        && args.without_dashboard_provisioning
    {
        return Err(message(
            "Nothing to export. Remove one of --without-dashboard-raw, --without-dashboard-prompt, or --without-dashboard-provisioning.",
        ));
    }
    let mut scoped_request = |method: Method,
                              path: &str,
                              params: &[(String, String)],
                              payload: Option<&Value>|
     -> Result<Option<Value>> {
        let mut scoped_params = params.to_vec();
        if let Some(org_id) = org_id_override {
            scoped_params.push(("orgId".to_string(), org_id.to_string()));
        }
        request_json(method, path, &scoped_params, payload)
    };
    let current_org = match org {
        Some(org) => org.clone(),
        None => fetch_current_org_with_request(&mut scoped_request)?,
    };
    let current_org_name = string_field(&current_org, "name", "org");
    let current_org_id = current_org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_UNKNOWN_UID.to_string());
    let scope_output_dir = if args.all_orgs {
        build_all_orgs_output_dir(&args.export_dir, &current_org)
    } else {
        args.export_dir.clone()
    };
    let (raw_dir, prompt_dir, provisioning_dir) = build_export_variant_dirs(&scope_output_dir);
    let history_dir = scope_output_dir.join("history");
    let provisioning_dashboards_dir = provisioning_dir.join("dashboards");
    let provisioning_config_dir = provisioning_dir.join("provisioning");
    if !args.dry_run && !args.without_dashboard_raw {
        fs::create_dir_all(&raw_dir)?;
    }
    if !args.dry_run && !args.without_dashboard_prompt {
        fs::create_dir_all(&prompt_dir)?;
    }
    if !args.dry_run && !args.without_dashboard_provisioning {
        fs::create_dir_all(&provisioning_dashboards_dir)?;
        fs::create_dir_all(&provisioning_config_dir)?;
    }
    if !args.dry_run && args.include_history {
        fs::create_dir_all(&history_dir)?;
    }
    let datasource_list = list_datasources_with_request(&mut scoped_request)?;
    let datasource_inventory = datasource_list
        .iter()
        .map(|datasource| build_datasource_inventory_record(datasource, &current_org))
        .collect::<Vec<_>>();
    let datasource_catalog = build_datasource_catalog(&datasource_list);

    let summaries = list_dashboard_summaries_with_request(&mut scoped_request, args.page_size)?;
    if summaries.is_empty() {
        return Ok(ScopeExportResult {
            exported_count: 0,
            root_index: None,
            org_summary: None,
        });
    }
    let summaries = attach_dashboard_org_metadata(&summaries, &current_org);
    let folder_inventory =
        super::collect_folder_inventory_with_request(&mut scoped_request, &summaries)?;
    let permission_documents = if args.without_dashboard_raw || args.dry_run {
        Vec::new()
    } else {
        collect_permission_export_documents(&mut scoped_request, &summaries, &folder_inventory)?
    };

    let mut exported_count = 0;
    let mut index_items = Vec::new();
    let mut used_source_names = BTreeSet::new();
    let mut used_source_uids = BTreeSet::new();
    let total = summaries.len();
    for (index, summary) in summaries.into_iter().enumerate() {
        let uid = string_field(&summary, "uid", "");
        if uid.is_empty() {
            continue;
        }
        if args.verbose {
            // Verbose mode prints per-variant details after each write, which already
            // includes status for this iteration; suppressing progress lines avoids
            // duplicated/noisy output and keeps verbose logs ordered.
        } else if args.progress {
            println!(
                "{}",
                format_export_progress_line(index + 1, total, &uid, args.dry_run)
            );
        }
        let payload = fetch_dashboard_with_request(&mut scoped_request, &uid)?;
        let (source_names, source_uids) =
            collect_dashboard_source_metadata(&payload, &datasource_catalog)?;
        used_source_names.extend(source_names);
        used_source_uids.extend(source_uids);
        if args.include_history {
            let history_document = build_dashboard_history_export_document_with_request(
                &mut scoped_request,
                &uid,
                20,
            )?;
            let history_path = build_history_output_path(&history_dir, &uid);
            if !args.dry_run {
                write_history_document(&history_document, &history_path, args.overwrite)?;
            }
            if args.verbose {
                println!(
                    "{}",
                    format_export_verbose_line("history", &uid, &history_path, args.dry_run)
                );
            }
        }
        let mut item = super::build_dashboard_index_item(&summary, &uid);
        if !args.without_dashboard_raw {
            let raw_document = build_preserved_web_import_document(&payload)?;
            let raw_path = build_output_path(&raw_dir, &summary, args.flat);
            if !args.dry_run {
                write_dashboard(&raw_document, &raw_path, args.overwrite)?;
            }
            if args.verbose {
                println!(
                    "{}",
                    format_export_verbose_line("raw", &uid, &raw_path, args.dry_run)
                );
            }
            item.raw_path = Some(raw_path.display().to_string());
        }
        if !args.without_dashboard_prompt {
            let prompt_document = build_external_export_document(&payload, &datasource_catalog)?;
            let prompt_path = build_output_path(&prompt_dir, &summary, args.flat);
            if !args.dry_run {
                write_dashboard(&prompt_document, &prompt_path, args.overwrite)?;
            }
            if args.verbose {
                println!(
                    "{}",
                    format_export_verbose_line("prompt", &uid, &prompt_path, args.dry_run)
                );
            }
            item.prompt_path = Some(prompt_path.display().to_string());
        }
        if !args.without_dashboard_provisioning {
            let provisioning_document = build_preserved_web_import_document(&payload)?;
            let provisioning_path =
                build_output_path(&provisioning_dashboards_dir, &summary, false);
            if !args.dry_run {
                write_dashboard(&provisioning_document, &provisioning_path, args.overwrite)?;
            }
            if args.verbose {
                println!(
                    "{}",
                    format_export_verbose_line(
                        "provisioning",
                        &uid,
                        &provisioning_path,
                        args.dry_run
                    )
                );
            }
            item.provisioning_path = Some(provisioning_path.display().to_string());
        }
        exported_count += 1;
        index_items.push(item);
    }

    let mut raw_index_path = None;
    if !args.without_dashboard_raw {
        let index_path = raw_dir.join("index.json");
        let metadata_path = raw_dir.join(EXPORT_METADATA_FILENAME);
        let folder_inventory_path = raw_dir.join(FOLDER_INVENTORY_FILENAME);
        let datasource_inventory_path = raw_dir.join(DATASOURCE_INVENTORY_FILENAME);
        let permission_bundle_path = raw_dir.join(DASHBOARD_PERMISSION_BUNDLE_FILENAME);
        if !args.dry_run {
            write_json_document(
                &build_variant_index(
                    &index_items,
                    |item| item.raw_path.as_deref(),
                    "grafana-web-import-preserve-uid",
                ),
                &index_path,
            )?;
            write_json_document(
                &build_export_metadata(
                    RAW_EXPORT_SUBDIR,
                    index_items
                        .iter()
                        .filter(|item| item.raw_path.is_some())
                        .count(),
                    Some("grafana-web-import-preserve-uid"),
                    Some(FOLDER_INVENTORY_FILENAME),
                    Some(DATASOURCE_INVENTORY_FILENAME),
                    Some(DASHBOARD_PERMISSION_BUNDLE_FILENAME),
                    Some(&current_org_name),
                    Some(&current_org_id),
                    None,
                    "live",
                    Some(&args.common.url),
                    None,
                    args.common.profile.as_deref(),
                    raw_dir.as_path(),
                    &metadata_path,
                ),
                &metadata_path,
            )?;
            write_json_document(&folder_inventory, &folder_inventory_path)?;
            write_json_document(&datasource_inventory, &datasource_inventory_path)?;
            write_json_document(
                &build_permission_bundle_document(&permission_documents),
                &permission_bundle_path,
            )?;
        }
        raw_index_path = Some(index_path);
    }
    let mut prompt_index_path = None;
    if !args.without_dashboard_prompt {
        let index_path = prompt_dir.join("index.json");
        let metadata_path = prompt_dir.join(EXPORT_METADATA_FILENAME);
        if !args.dry_run {
            write_json_document(
                &build_variant_index(
                    &index_items,
                    |item| item.prompt_path.as_deref(),
                    "grafana-web-import-with-datasource-inputs",
                ),
                &index_path,
            )?;
            write_json_document(
                &build_export_metadata(
                    PROMPT_EXPORT_SUBDIR,
                    index_items
                        .iter()
                        .filter(|item| item.prompt_path.is_some())
                        .count(),
                    Some("grafana-web-import-with-datasource-inputs"),
                    None,
                    None,
                    None,
                    Some(&current_org_name),
                    Some(&current_org_id),
                    None,
                    "live",
                    Some(&args.common.url),
                    None,
                    args.common.profile.as_deref(),
                    prompt_dir.as_path(),
                    &metadata_path,
                ),
                &metadata_path,
            )?;
        }
        prompt_index_path = Some(index_path);
    }
    let mut provisioning_index_path = None;
    if !args.without_dashboard_provisioning {
        let index_path = provisioning_dir.join("index.json");
        let metadata_path = provisioning_dir.join(EXPORT_METADATA_FILENAME);
        let config_path = provisioning_config_dir.join("dashboards.yaml");
        let provisioning_provider_org_id = args
            .provisioning_provider_org_id
            .unwrap_or_else(|| current_org_id.parse::<i64>().unwrap_or(1));
        let provisioning_provider_path = args
            .provisioning_provider_path
            .as_deref()
            .unwrap_or(&provisioning_dashboards_dir);
        if !args.dry_run {
            write_json_document(
                &build_variant_index(
                    &index_items,
                    |item| item.provisioning_path.as_deref(),
                    "grafana-file-provisioning-dashboard",
                ),
                &index_path,
            )?;
            write_json_document(
                &build_export_metadata(
                    PROVISIONING_EXPORT_SUBDIR,
                    index_items
                        .iter()
                        .filter(|item| item.provisioning_path.is_some())
                        .count(),
                    Some("grafana-file-provisioning-dashboard"),
                    None,
                    None,
                    None,
                    Some(&current_org_name),
                    Some(&current_org_id),
                    None,
                    "live",
                    Some(&args.common.url),
                    None,
                    args.common.profile.as_deref(),
                    provisioning_dir.as_path(),
                    &metadata_path,
                ),
                &metadata_path,
            )?;
            write_yaml_document(
                &build_provisioning_config(
                    provisioning_provider_org_id,
                    &args.provisioning_provider_name,
                    provisioning_provider_path,
                    args.provisioning_provider_disable_deletion,
                    args.provisioning_provider_allow_ui_updates,
                    args.provisioning_provider_update_interval_seconds,
                ),
                &config_path,
                args.overwrite,
            )?;
        }
        provisioning_index_path = Some(index_path);
    }
    let root_index = build_root_export_index(
        &index_items,
        raw_index_path.as_deref(),
        prompt_index_path.as_deref(),
        provisioning_index_path.as_deref(),
        &folder_inventory,
    );
    let used_datasources = build_used_datasource_summaries(
        &datasource_inventory,
        &used_source_names,
        &used_source_uids,
    );
    let org_summary = ExportOrgSummary {
        org: current_org_name.clone(),
        org_id: current_org_id.clone(),
        dashboard_count: index_items.len() as u64,
        datasource_count: Some(datasource_inventory.len() as u64),
        used_datasource_count: Some(used_datasources.len() as u64),
        used_datasources: Some(used_datasources),
        export_dir: if args.all_orgs {
            Some(scope_output_dir.display().to_string())
        } else {
            None
        },
    };
    if !args.dry_run {
        write_json_document(&root_index, &scope_output_dir.join("index.json"))?;
        write_json_document(
            &build_export_metadata(
                "root",
                index_items.len(),
                None,
                None,
                None,
                None,
                Some(&current_org_name),
                Some(&current_org_id),
                None,
                "live",
                Some(&args.common.url),
                None,
                args.common.profile.as_deref(),
                scope_output_dir.as_path(),
                &scope_output_dir.join(EXPORT_METADATA_FILENAME),
            ),
            &scope_output_dir.join(EXPORT_METADATA_FILENAME),
        )?;
    }
    Ok(ScopeExportResult {
        exported_count,
        root_index: Some(root_index),
        org_summary: Some(org_summary),
    })
}

fn write_all_orgs_root_export_bundle(
    export_dir: &Path,
    root_items: &[DashboardIndexItem],
    root_folders: &[FolderInventoryItem],
    org_summaries: Vec<ExportOrgSummary>,
    source_url: &str,
    source_profile: Option<&str>,
) -> Result<()> {
    write_json_document(
        &build_root_export_index(root_items, None, None, None, root_folders),
        &export_dir.join("index.json"),
    )?;
    write_json_document(
        &build_export_metadata(
            "root",
            root_items.len(),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(org_summaries),
            "live",
            Some(source_url),
            None,
            source_profile,
            export_dir,
            &export_dir.join(EXPORT_METADATA_FILENAME),
        ),
        &export_dir.join(EXPORT_METADATA_FILENAME),
    )?;
    Ok(())
}

pub(crate) fn export_dashboards_with_request<F>(
    mut request_json: F,
    args: &ExportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.all_orgs {
        let mut total = 0usize;
        let mut root_items = Vec::new();
        let mut root_folders = Vec::new();
        let mut org_summaries = Vec::new();
        for org in list_orgs_with_request(&mut request_json)? {
            let org_id = org_id_value(&org)?;
            let scope_result = export_dashboards_in_scope_with_request(
                &mut request_json,
                args,
                Some(&org),
                Some(org_id),
            )?;
            total += scope_result.exported_count;
            if let Some(root_index) = scope_result.root_index {
                root_items.extend(root_index.items);
                root_folders.extend(root_index.folders);
            }
            if let Some(org_summary) = scope_result.org_summary {
                org_summaries.push(org_summary);
            }
        }
        if !args.dry_run && !root_items.is_empty() {
            write_all_orgs_root_export_bundle(
                &args.export_dir,
                &root_items,
                &root_folders,
                org_summaries,
                &args.common.url,
                args.common.profile.as_deref(),
            )?;
        }
        Ok(total)
    } else {
        Ok(
            export_dashboards_in_scope_with_request(&mut request_json, args, None, args.org_id)?
                .exported_count,
        )
    }
}

pub fn export_dashboards_with_client(client: &JsonHttpClient, args: &ExportArgs) -> Result<usize> {
    export_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

pub(crate) fn export_dashboards_with_org_clients(args: &ExportArgs) -> Result<usize> {
    if args.all_orgs {
        let admin_api = build_api_client(&args.common)?;
        let admin_dashboard = DashboardResourceClient::new(admin_api.http_client());
        let mut total = 0usize;
        let mut root_items = Vec::new();
        let mut root_folders = Vec::new();
        let mut org_summaries = Vec::new();
        for org in admin_dashboard.list_orgs()? {
            let org_id = org_id_value(&org)?;
            let org_client = build_http_client_for_org_from_api(&admin_api, org_id)?;
            let scope_result = export_dashboards_in_scope_with_request(
                &mut |method, path, params, payload| {
                    org_client.request_json(method, path, params, payload)
                },
                args,
                Some(&org),
                None,
            )?;
            total += scope_result.exported_count;
            if let Some(root_index) = scope_result.root_index {
                root_items.extend(root_index.items);
                root_folders.extend(root_index.folders);
            }
            if let Some(org_summary) = scope_result.org_summary {
                org_summaries.push(org_summary);
            }
        }
        if !args.dry_run && !root_items.is_empty() {
            write_all_orgs_root_export_bundle(
                &args.export_dir,
                &root_items,
                &root_folders,
                org_summaries,
                &args.common.url,
                args.common.profile.as_deref(),
            )?;
        }
        Ok(total)
    } else if let Some(org_id) = args.org_id {
        let org_client = build_http_client_for_org(&args.common, org_id)?;
        Ok(export_dashboards_in_scope_with_request(
            &mut |method, path, params, payload| {
                org_client.request_json(method, path, params, payload)
            },
            args,
            None,
            None,
        )?
        .exported_count)
    } else {
        let client = build_http_client(&args.common)?;
        let dashboard = DashboardResourceClient::new(&client);
        let current_org = dashboard.fetch_current_org()?;
        Ok(export_dashboards_in_scope_with_request(
            &mut |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
            Some(&current_org),
            None,
        )?
        .exported_count)
    }
}
