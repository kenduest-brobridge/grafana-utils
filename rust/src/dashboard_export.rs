use reqwest::Method;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, sanitize_path_component, string_field, Result};
use crate::http::JsonHttpClient;

use super::{
    build_datasource_catalog, build_export_metadata, build_external_export_document, build_http_client,
    build_http_client_for_org, build_preserved_web_import_document, build_root_export_index,
    build_variant_index, fetch_dashboard_with_request, list_dashboard_summaries_with_request,
    list_datasources_with_request, write_dashboard, write_json_document, ExportArgs,
    EXPORT_METADATA_FILENAME, PROMPT_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR,
};
use super::dashboard_list::{
    attach_dashboard_org_metadata, fetch_current_org_with_request, list_orgs_with_request, org_id_value,
};

pub fn build_output_path(output_dir: &Path, summary: &Map<String, Value>, flat: bool) -> PathBuf {
    let folder_title = string_field(summary, "folderTitle", "General");
    let title = string_field(summary, "title", "dashboard");
    let uid = string_field(summary, "uid", "unknown");
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
        .unwrap_or_else(|| "unknown".to_string());
    let org_name = sanitize_path_component(&string_field(org, "name", "org"));
    output_dir.join(format!("org_{org_id}_{org_name}"))
}

pub fn build_export_variant_dirs(output_dir: &Path) -> (PathBuf, PathBuf) {
    (
        output_dir.join(RAW_EXPORT_SUBDIR),
        output_dir.join(PROMPT_EXPORT_SUBDIR),
    )
}

pub(crate) fn export_dashboards_in_scope_with_request<F>(
    request_json: &mut F,
    args: &ExportArgs,
    org: Option<&Map<String, Value>>,
    org_id_override: Option<i64>,
) -> Result<usize>
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
    let datasource_catalog = if args.without_dashboard_prompt {
        None
    } else {
        Some(build_datasource_catalog(&list_datasources_with_request(&mut scoped_request)?))
    };

    let summaries = list_dashboard_summaries_with_request(&mut scoped_request, args.page_size)?;
    if summaries.is_empty() {
        return Ok(0);
    }
    let summaries = attach_dashboard_org_metadata(&summaries, &current_org);

    let mut exported_count = 0;
    let mut index_items = Vec::new();
    for summary in summaries {
        let uid = string_field(&summary, "uid", "");
        if uid.is_empty() {
            continue;
        }
        let payload = fetch_dashboard_with_request(&mut scoped_request, &uid)?;
        let mut item = super::build_dashboard_index_item(&summary, &uid);
        if !args.without_dashboard_raw {
            let raw_document = build_preserved_web_import_document(&payload)?;
            let raw_path = build_output_path(&raw_dir, &summary, args.flat);
            if !args.dry_run {
                write_dashboard(&raw_document, &raw_path, args.overwrite)?;
            }
            if args.progress {
                println!(
                    "{} raw    {} -> {}",
                    if args.dry_run { "Would export" } else { "Exported" },
                    uid,
                    raw_path.display()
                );
            }
            item.raw_path = Some(raw_path.display().to_string());
        }
        if !args.without_dashboard_prompt {
            let prompt_document = build_external_export_document(
                &payload,
                datasource_catalog
                    .as_ref()
                    .ok_or_else(|| message("Prompt export requires datasource catalog."))?,
            )?;
            let prompt_path = build_output_path(&prompt_dir, &summary, args.flat);
            if !args.dry_run {
                write_dashboard(&prompt_document, &prompt_path, args.overwrite)?;
            }
            if args.progress {
                println!(
                    "{} prompt {} -> {}",
                    if args.dry_run { "Would export" } else { "Exported" },
                    uid,
                    prompt_path.display()
                );
            }
            item.prompt_path = Some(prompt_path.display().to_string());
        }
        exported_count += 1;
        index_items.push(item);
    }

    let mut raw_index_path = None;
    if !args.without_dashboard_raw {
        let variant_dir = if args.all_orgs {
            args.export_dir.join(RAW_EXPORT_SUBDIR)
        } else {
            raw_dir.clone()
        };
        let index_path = variant_dir.join("index.json");
        let metadata_path = variant_dir.join(EXPORT_METADATA_FILENAME);
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
                    index_items.iter().filter(|item| item.raw_path.is_some()).count(),
                    Some("grafana-web-import-preserve-uid"),
                ),
                &metadata_path,
            )?;
        }
        raw_index_path = Some(index_path);
    }
    let mut prompt_index_path = None;
    if !args.without_dashboard_prompt {
        let variant_dir = if args.all_orgs {
            args.export_dir.join(PROMPT_EXPORT_SUBDIR)
        } else {
            prompt_dir.clone()
        };
        let index_path = variant_dir.join("index.json");
        let metadata_path = variant_dir.join(EXPORT_METADATA_FILENAME);
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
                ),
                &metadata_path,
            )?;
        }
        prompt_index_path = Some(index_path);
    }
    if !args.dry_run {
        write_json_document(
            &build_root_export_index(&index_items, raw_index_path.as_deref(), prompt_index_path.as_deref()),
            &args.export_dir.join("index.json"),
        )?;
        write_json_document(
            &build_export_metadata("root", index_items.len(), None),
            &args.export_dir.join(EXPORT_METADATA_FILENAME),
        )?;
    }
    Ok(exported_count)
}

pub(crate) fn export_dashboards_with_request<F>(mut request_json: F, args: &ExportArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.all_orgs {
        let mut total = 0usize;
        for org in list_orgs_with_request(&mut request_json)? {
            let org_id = org_id_value(&org)?;
            total += export_dashboards_in_scope_with_request(&mut request_json, args, Some(&org), Some(org_id))?;
        }
        Ok(total)
    } else {
        export_dashboards_in_scope_with_request(&mut request_json, args, None, args.org_id)
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
        let admin_client = build_http_client(&args.common)?;
        let mut total = 0usize;
        for org in list_orgs_with_request(|method, path, params, payload| {
            admin_client.request_json(method, path, params, payload)
        })? {
            let org_id = org_id_value(&org)?;
            let org_client = build_http_client_for_org(&args.common, org_id)?;
            total += export_dashboards_in_scope_with_request(
                &mut |method, path, params, payload| org_client.request_json(method, path, params, payload),
                args,
                Some(&org),
                None,
            )?;
        }
        Ok(total)
    } else if let Some(org_id) = args.org_id {
        let org_client = build_http_client_for_org(&args.common, org_id)?;
        export_dashboards_in_scope_with_request(
            &mut |method, path, params, payload| org_client.request_json(method, path, params, payload),
            args,
            None,
            None,
        )
    } else {
        let client = build_http_client(&args.common)?;
        export_dashboards_in_scope_with_request(
            &mut |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
            None,
            None,
        )
    }
}
