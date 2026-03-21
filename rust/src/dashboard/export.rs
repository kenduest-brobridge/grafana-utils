//! Export orchestration for dashboards.
//! Collects org/catalog context, resolves metadata, writes raw and prompt variants, and emits
//! progress/formatting output for CLI-facing export modes.
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, sanitize_path_component, string_field, Result};
use crate::http::JsonHttpClient;

use super::list::{
    attach_dashboard_org_metadata, collect_dashboard_source_metadata,
    fetch_current_org_with_request, list_orgs_with_request, org_id_value,
};
use super::{
    build_datasource_catalog, build_datasource_inventory_record, build_export_metadata,
    build_external_export_document, build_http_client, build_http_client_for_org,
    build_preserved_web_import_document, build_root_export_index, build_variant_index,
    fetch_dashboard_permissions_with_request, fetch_dashboard_with_request,
    fetch_folder_permissions_with_request, list_dashboard_summaries_with_request,
    list_datasources_with_request, write_dashboard, write_json_document, DashboardIndexItem,
    ExportArgs, ExportDatasourceUsageSummary, ExportOrgSummary, FolderInventoryItem,
    RootExportIndex, DASHBOARD_PERMISSION_BUNDLE_FILENAME, DATASOURCE_INVENTORY_FILENAME,
    DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_TITLE, DEFAULT_UNKNOWN_UID, EXPORT_METADATA_FILENAME,
    FOLDER_INVENTORY_FILENAME, PROMPT_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR,
};

const PERMISSION_BUNDLE_KIND: &str = "grafana-utils-dashboard-permission-bundle";
const PERMISSION_BUNDLE_SCHEMA_VERSION: i64 = 1;
const PERMISSION_EXPORT_KIND: &str = "grafana-utils-dashboard-permission-export";
const PERMISSION_EXPORT_SCHEMA_VERSION: i64 = 1;

pub(crate) struct ScopeExportResult {
    exported_count: usize,
    root_index: Option<RootExportIndex>,
    org_summary: Option<ExportOrgSummary>,
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
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

fn build_all_orgs_output_dir(output_dir: &Path, org: &Map<String, Value>) -> PathBuf {
    let org_id = org
        .get("id")
        .map(|value| sanitize_path_component(&value.to_string()))
        .unwrap_or_else(|| DEFAULT_UNKNOWN_UID.to_string());
    let org_name = sanitize_path_component(&string_field(org, "name", "org"));
    output_dir.join(format!("org_{org_id}_{org_name}"))
}

fn build_used_datasource_summaries(
    datasource_inventory: &[super::DatasourceInventoryItem],
    used_names: &BTreeSet<String>,
    used_uids: &BTreeSet<String>,
) -> Vec<ExportDatasourceUsageSummary> {
    let mut used = Vec::new();
    let mut matched_names = BTreeSet::new();
    let mut matched_uids = BTreeSet::new();

    for datasource in datasource_inventory {
        if used_uids.contains(&datasource.uid) || used_names.contains(&datasource.name) {
            used.push(ExportDatasourceUsageSummary {
                name: datasource.name.clone(),
                uid: if datasource.uid.is_empty() {
                    None
                } else {
                    Some(datasource.uid.clone())
                },
                datasource_type: if datasource.datasource_type.is_empty() {
                    None
                } else {
                    Some(datasource.datasource_type.clone())
                },
            });
            if !datasource.uid.is_empty() {
                matched_uids.insert(datasource.uid.clone());
            }
            if !datasource.name.is_empty() {
                matched_names.insert(datasource.name.clone());
            }
        }
    }

    for name in used_names {
        if !matched_names.contains(name) {
            used.push(ExportDatasourceUsageSummary {
                name: name.clone(),
                uid: None,
                datasource_type: None,
            });
        }
    }
    for uid in used_uids {
        if !matched_uids.contains(uid) {
            used.push(ExportDatasourceUsageSummary {
                name: String::new(),
                uid: Some(uid.clone()),
                datasource_type: None,
            });
        }
    }

    used
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn build_export_variant_dirs(output_dir: &Path) -> (PathBuf, PathBuf) {
    (
        output_dir.join(RAW_EXPORT_SUBDIR),
        output_dir.join(PROMPT_EXPORT_SUBDIR),
    )
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

fn normalize_permission_level(record: &Map<String, Value>) -> (i64, &'static str) {
    let level_value = match record.get("permission") {
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0),
        Some(Value::String(text)) => match text.trim().to_lowercase().as_str() {
            "view" => 1,
            "edit" => 2,
            "admin" => 4,
            other => other.parse::<i64>().unwrap_or(0),
        },
        Some(value) => value.to_string().parse::<i64>().unwrap_or(0),
        None => match record.get("permissionName") {
            Some(Value::String(text)) => match text.trim().to_lowercase().as_str() {
                "view" => 1,
                "edit" => 2,
                "admin" => 4,
                other => other.parse::<i64>().unwrap_or(0),
            },
            Some(value) => value.to_string().parse::<i64>().unwrap_or(0),
            None => 0,
        },
    };
    let level_name = match level_value {
        1 => "view",
        2 => "edit",
        4 => "admin",
        _ => "unknown",
    };
    (level_value, level_name)
}

fn value_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string().trim_matches('"').to_string(),
    }
}

fn normalize_permission_subject(record: &Map<String, Value>) -> (String, String, String, String) {
    let user_id = value_text(record.get("userId"));
    if !user_id.is_empty() {
        let user_name = value_text(record.get("user").or_else(|| record.get("userLogin")));
        let subject_name = if user_name.is_empty() {
            user_id.clone()
        } else {
            user_name
        };
        return (
            "user".to_string(),
            format!("user:{user_id}"),
            user_id,
            subject_name,
        );
    }
    let team_id = value_text(record.get("teamId"));
    if !team_id.is_empty() {
        let team_name = value_text(record.get("team").or_else(|| record.get("teamName")));
        let subject_name = if team_name.is_empty() {
            team_id.clone()
        } else {
            team_name
        };
        return (
            "team".to_string(),
            format!("team:{team_id}"),
            team_id,
            subject_name,
        );
    }
    let service_account_id = value_text(record.get("serviceAccountId"));
    if !service_account_id.is_empty() {
        let service_account_name = value_text(
            record
                .get("serviceAccount")
                .or_else(|| record.get("serviceAccountName")),
        );
        let subject_name = if service_account_name.is_empty() {
            service_account_id.clone()
        } else {
            service_account_name
        };
        return (
            "service-account".to_string(),
            format!("service-account:{service_account_id}"),
            service_account_id,
            subject_name,
        );
    }
    let role = value_text(record.get("role"));
    if !role.is_empty() {
        return (
            "role".to_string(),
            format!("role:{role}"),
            role.clone(),
            role,
        );
    }
    (
        "unknown".to_string(),
        "unknown".to_string(),
        String::new(),
        "unknown".to_string(),
    )
}

fn build_permission_export_document(
    resource_kind: &str,
    resource_uid: &str,
    resource_title: &str,
    permissions: &[Map<String, Value>],
    org_name: &str,
    org_id: &str,
) -> Value {
    let mut rows = permissions
        .iter()
        .map(|record| {
            let (subject_type, subject_key, subject_id, subject_name) =
                normalize_permission_subject(record);
            let (permission, permission_name) = normalize_permission_level(record);
            let inherited = record
                .get("inherited")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let mut row = Map::new();
            row.insert(
                "resourceKind".to_string(),
                Value::String(resource_kind.to_string()),
            );
            row.insert(
                "resourceUid".to_string(),
                Value::String(resource_uid.to_string()),
            );
            row.insert(
                "resourceTitle".to_string(),
                Value::String(resource_title.to_string()),
            );
            row.insert("subjectType".to_string(), Value::String(subject_type));
            row.insert("subjectKey".to_string(), Value::String(subject_key));
            row.insert("subjectId".to_string(), Value::String(subject_id));
            row.insert("subjectName".to_string(), Value::String(subject_name));
            row.insert("permission".to_string(), Value::from(permission));
            row.insert(
                "permissionName".to_string(),
                Value::String(permission_name.to_string()),
            );
            row.insert("inherited".to_string(), Value::Bool(inherited));
            row
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (
            value_text(left.get("resourceKind")),
            value_text(left.get("resourceUid")),
            value_text(left.get("subjectType")),
            value_text(left.get("subjectName")),
            left.get("permission")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
        )
            .cmp(&(
                value_text(right.get("resourceKind")),
                value_text(right.get("resourceUid")),
                value_text(right.get("subjectType")),
                value_text(right.get("subjectName")),
                right
                    .get("permission")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
            ))
    });

    let permission_count = rows.len() as i64;
    let user_count = rows
        .iter()
        .filter(|row| value_text(row.get("subjectType")) == "user")
        .count() as i64;
    let team_count = rows
        .iter()
        .filter(|row| value_text(row.get("subjectType")) == "team")
        .count() as i64;
    let service_account_count = rows
        .iter()
        .filter(|row| value_text(row.get("subjectType")) == "service-account")
        .count() as i64;
    let role_count = rows
        .iter()
        .filter(|row| value_text(row.get("subjectType")) == "role")
        .count() as i64;

    let mut summary = Map::new();
    summary.insert("permissionCount".to_string(), Value::from(permission_count));
    summary.insert("userCount".to_string(), Value::from(user_count));
    summary.insert("teamCount".to_string(), Value::from(team_count));
    summary.insert(
        "serviceAccountCount".to_string(),
        Value::from(service_account_count),
    );
    summary.insert("roleCount".to_string(), Value::from(role_count));

    let mut resource = Map::new();
    resource.insert("kind".to_string(), Value::String(resource_kind.to_string()));
    resource.insert("uid".to_string(), Value::String(resource_uid.to_string()));
    resource.insert(
        "title".to_string(),
        Value::String(resource_title.to_string()),
    );

    let mut document = Map::new();
    document.insert(
        "kind".to_string(),
        Value::String(PERMISSION_EXPORT_KIND.to_string()),
    );
    document.insert(
        "schemaVersion".to_string(),
        Value::from(PERMISSION_EXPORT_SCHEMA_VERSION),
    );
    document.insert("resource".to_string(), Value::Object(resource));
    document.insert("summary".to_string(), Value::Object(summary));
    document.insert(
        "permissions".to_string(),
        Value::Array(rows.into_iter().map(Value::Object).collect()),
    );
    document.insert("org".to_string(), Value::String(org_name.to_string()));
    document.insert("orgId".to_string(), Value::String(org_id.to_string()));
    Value::Object(document)
}

fn collect_permission_export_documents<F>(
    request_json: &mut F,
    summaries: &[Map<String, Value>],
    folder_inventory: &[FolderInventoryItem],
) -> Result<Vec<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut documents = Vec::new();
    let mut seen_folders = std::collections::BTreeSet::new();
    for folder in folder_inventory {
        if folder.uid.trim().is_empty() || !seen_folders.insert(folder.uid.clone()) {
            continue;
        }
        let permissions = fetch_folder_permissions_with_request(&mut *request_json, &folder.uid)?;
        documents.push(build_permission_export_document(
            "folder",
            &folder.uid,
            &folder.title,
            &permissions,
            &folder.org,
            &folder.org_id,
        ));
    }
    for summary in summaries {
        let uid = string_field(summary, "uid", "");
        if uid.is_empty() {
            continue;
        }
        let permissions = fetch_dashboard_permissions_with_request(&mut *request_json, &uid)?;
        documents.push(build_permission_export_document(
            "dashboard",
            &uid,
            &string_field(summary, "title", DEFAULT_DASHBOARD_TITLE),
            &permissions,
            &string_field(summary, "orgName", "org"),
            &{
                let raw_org_id = value_text(summary.get("orgId"));
                if raw_org_id.is_empty() {
                    DEFAULT_UNKNOWN_UID.to_string()
                } else {
                    raw_org_id
                }
            },
        ));
    }
    Ok(documents)
}

fn build_permission_bundle_document(permission_documents: &[Value]) -> Value {
    let resource_count = permission_documents.len() as i64;
    let dashboard_count = permission_documents
        .iter()
        .filter(|item| {
            item.get("resource")
                .and_then(Value::as_object)
                .and_then(|resource| resource.get("kind"))
                .and_then(Value::as_str)
                == Some("dashboard")
        })
        .count() as i64;
    let folder_count = permission_documents
        .iter()
        .filter(|item| {
            item.get("resource")
                .and_then(Value::as_object)
                .and_then(|resource| resource.get("kind"))
                .and_then(Value::as_str)
                == Some("folder")
        })
        .count() as i64;
    let permission_count = permission_documents
        .iter()
        .map(|item| {
            item.get("summary")
                .and_then(Value::as_object)
                .and_then(|summary| summary.get("permissionCount"))
                .and_then(Value::as_i64)
                .unwrap_or_default()
        })
        .sum::<i64>();

    let mut summary = Map::new();
    summary.insert("resourceCount".to_string(), Value::from(resource_count));
    summary.insert("dashboardCount".to_string(), Value::from(dashboard_count));
    summary.insert("folderCount".to_string(), Value::from(folder_count));
    summary.insert("permissionCount".to_string(), Value::from(permission_count));

    let mut bundle = Map::new();
    bundle.insert(
        "kind".to_string(),
        Value::String(PERMISSION_BUNDLE_KIND.to_string()),
    );
    bundle.insert(
        "schemaVersion".to_string(),
        Value::from(PERMISSION_BUNDLE_SCHEMA_VERSION),
    );
    bundle.insert("summary".to_string(), Value::Object(summary));
    bundle.insert(
        "resources".to_string(),
        Value::Array(permission_documents.to_vec()),
    );
    Value::Object(bundle)
}

/// Purpose: implementation note.
pub(crate) fn export_dashboards_in_scope_with_request<F>(
    request_json: &mut F,
    args: &ExportArgs,
    org: Option<&Map<String, Value>>,
    org_id_override: Option<i64>,
) -> Result<ScopeExportResult>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.without_dashboard_raw && args.without_dashboard_prompt {
        return Err(message(
            "Nothing to export. Remove one of --without-dashboard-raw or --without-dashboard-prompt.",
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
    let (raw_dir, prompt_dir) = build_export_variant_dirs(&scope_output_dir);
    if !args.dry_run && !args.without_dashboard_raw {
        fs::create_dir_all(&raw_dir)?;
    }
    if !args.dry_run && !args.without_dashboard_prompt {
        fs::create_dir_all(&prompt_dir)?;
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
                ),
                &metadata_path,
            )?;
        }
        prompt_index_path = Some(index_path);
    }
    let root_index = build_root_export_index(
        &index_items,
        raw_index_path.as_deref(),
        prompt_index_path.as_deref(),
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
) -> Result<()> {
    write_json_document(
        &build_root_export_index(root_items, None, None, root_folders),
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
        ),
        &export_dir.join(EXPORT_METADATA_FILENAME),
    )?;
    Ok(())
}

/// Purpose: implementation note.
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

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn export_dashboards_with_client(client: &JsonHttpClient, args: &ExportArgs) -> Result<usize> {
    export_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

/// Purpose: implementation note.
pub(crate) fn export_dashboards_with_org_clients(args: &ExportArgs) -> Result<usize> {
    if args.all_orgs {
        let admin_client = build_http_client(&args.common)?;
        let mut total = 0usize;
        let mut root_items = Vec::new();
        let mut root_folders = Vec::new();
        let mut org_summaries = Vec::new();
        for org in list_orgs_with_request(|method, path, params, payload| {
            admin_client.request_json(method, path, params, payload)
        })? {
            let org_id = org_id_value(&org)?;
            let org_client = build_http_client_for_org(&args.common, org_id)?;
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
        Ok(export_dashboards_in_scope_with_request(
            &mut |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
            None,
            None,
        )?
        .exported_count)
    }
}
