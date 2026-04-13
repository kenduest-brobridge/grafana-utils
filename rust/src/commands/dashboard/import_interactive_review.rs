#![cfg(feature = "tui")]

use std::collections::BTreeMap;

use reqwest::Method;
use serde_json::Value;

use crate::common::{message, string_field, value_as_object, Result};
use crate::grafana_api::DashboardResourceClient;

use super::build_preserved_web_import_document;
use super::import_interactive::{InteractiveImportDiffData, InteractiveImportReview};
use super::import_lookup::{
    apply_folder_path_guard_to_action, build_folder_path_match_result,
    determine_dashboard_import_action_with_client, determine_dashboard_import_action_with_request,
    determine_import_folder_uid_override_with_client,
    determine_import_folder_uid_override_with_request, fetch_dashboard_if_exists_cached,
    fetch_dashboard_if_exists_cached_with_client, resolve_dashboard_import_folder_path_with_client,
    resolve_dashboard_import_folder_path_with_request,
    resolve_existing_dashboard_folder_path_with_client,
    resolve_existing_dashboard_folder_path_with_request, ImportLookupCache,
};

pub(crate) fn build_interactive_import_review_with_request<F>(
    request_json: &mut F,
    lookup_cache: &mut ImportLookupCache,
    args: &super::ImportArgs,
    dashboard_file: &std::path::Path,
    uid: &str,
    source_folder_path: &str,
) -> Result<InteractiveImportReview>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let document = super::load_json_file(dashboard_file)?;
    if args.strict_schema {
        super::validate::validate_dashboard_import_document(
            &document,
            dashboard_file,
            true,
            args.target_schema_version,
        )?;
    }
    let resolved_import = super::import::resolve_import_source(args)?;
    let metadata = super::load_export_metadata(
        resolved_import.metadata_dir(),
        Some(super::import::import_metadata_variant(args)),
    )?;
    let folder_inventory =
        super::load_folder_inventory(resolved_import.metadata_dir(), metadata.as_ref())?;
    let folders_by_uid: BTreeMap<String, super::FolderInventoryItem> = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect();
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let folder_uid_override = determine_import_folder_uid_override_with_request(
        request_json,
        lookup_cache,
        uid,
        args.import_folder_uid.as_deref(),
        effective_replace_existing,
    )?;
    let payload = super::build_import_payload(
        &document,
        folder_uid_override.as_deref(),
        effective_replace_existing,
        &args.import_message,
    )?;
    let action = determine_dashboard_import_action_with_request(
        request_json,
        lookup_cache,
        &payload,
        args.replace_existing,
        args.update_existing_only,
    )?;
    let destination_folder_path = if args.require_matching_folder_path {
        resolve_existing_dashboard_folder_path_with_request(request_json, lookup_cache, uid)?
    } else {
        None
    };
    let (
        folder_paths_match,
        reason,
        normalized_source_folder_path,
        normalized_destination_folder_path,
    ) = if args.require_matching_folder_path {
        build_folder_path_match_result(
            Some(source_folder_path),
            destination_folder_path.as_deref(),
            destination_folder_path.is_some(),
            true,
        )
    } else {
        (true, "", source_folder_path.to_string(), None::<String>)
    };
    let action = apply_folder_path_guard_to_action(action, folder_paths_match);
    let prefer_live_folder_path =
        folder_uid_override.is_some() && args.import_folder_uid.is_none() && !uid.is_empty();
    let folder_path = resolve_dashboard_import_folder_path_with_request(
        request_json,
        lookup_cache,
        &payload,
        &folders_by_uid,
        prefer_live_folder_path,
    )?;
    let (destination, action_label) = match action {
        "would-create" => ("missing", "create"),
        "would-update" => ("exists", "update"),
        "would-skip-missing" => ("missing", "skip-missing"),
        "would-skip-folder-mismatch" => ("exists", "skip-folder-mismatch"),
        "would-fail-existing" => ("exists", "blocked-existing"),
        _ => ("unknown", action),
    };
    let (diff_status, diff_summary_lines, diff_structural_lines, diff_raw_lines) =
        build_interactive_import_diff_summary_with_request(
            request_json,
            lookup_cache,
            &document,
            &payload,
            uid,
        )?;
    Ok(InteractiveImportReview {
        action: action.to_string(),
        destination: destination.to_string(),
        action_label: action_label.to_string(),
        folder_path,
        source_folder_path: normalized_source_folder_path,
        destination_folder_path: normalized_destination_folder_path.unwrap_or_default(),
        reason: reason.to_string(),
        diff_status,
        diff_summary_lines,
        diff_structural_lines,
        diff_raw_lines,
    })
}

#[allow(dead_code)]
pub(crate) fn build_interactive_import_review_with_client(
    client: &DashboardResourceClient<'_>,
    lookup_cache: &mut ImportLookupCache,
    args: &super::ImportArgs,
    dashboard_file: &std::path::Path,
    uid: &str,
    source_folder_path: &str,
) -> Result<InteractiveImportReview> {
    let document = super::load_json_file(dashboard_file)?;
    if args.strict_schema {
        super::validate::validate_dashboard_import_document(
            &document,
            dashboard_file,
            true,
            args.target_schema_version,
        )?;
    }
    let resolved_import = super::import::resolve_import_source(args)?;
    let metadata = super::load_export_metadata(
        resolved_import.metadata_dir(),
        Some(super::import::import_metadata_variant(args)),
    )?;
    let folder_inventory =
        super::load_folder_inventory(resolved_import.metadata_dir(), metadata.as_ref())?;
    let folders_by_uid: BTreeMap<String, super::FolderInventoryItem> = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect();
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let folder_uid_override = determine_import_folder_uid_override_with_client(
        client,
        lookup_cache,
        uid,
        args.import_folder_uid.as_deref(),
        effective_replace_existing,
    )?;
    let payload = super::build_import_payload(
        &document,
        folder_uid_override.as_deref(),
        effective_replace_existing,
        &args.import_message,
    )?;
    let action = determine_dashboard_import_action_with_client(
        client,
        lookup_cache,
        &payload,
        args.replace_existing,
        args.update_existing_only,
    )?;
    let destination_folder_path = if args.require_matching_folder_path {
        resolve_existing_dashboard_folder_path_with_client(client, lookup_cache, uid)?
    } else {
        None
    };
    let (
        folder_paths_match,
        reason,
        normalized_source_folder_path,
        normalized_destination_folder_path,
    ) = if args.require_matching_folder_path {
        build_folder_path_match_result(
            Some(source_folder_path),
            destination_folder_path.as_deref(),
            destination_folder_path.is_some(),
            true,
        )
    } else {
        (true, "", source_folder_path.to_string(), None::<String>)
    };
    let action = apply_folder_path_guard_to_action(action, folder_paths_match);
    let prefer_live_folder_path =
        folder_uid_override.is_some() && args.import_folder_uid.is_none() && !uid.is_empty();
    let folder_path = resolve_dashboard_import_folder_path_with_client(
        client,
        lookup_cache,
        &payload,
        &folders_by_uid,
        prefer_live_folder_path,
    )?;
    let (destination, action_label) = match action {
        "would-create" => ("missing", "create"),
        "would-update" => ("exists", "update"),
        "would-skip-missing" => ("missing", "skip-missing"),
        "would-skip-folder-mismatch" => ("exists", "skip-folder-mismatch"),
        "would-fail-existing" => ("exists", "blocked-existing"),
        _ => ("unknown", action),
    };
    let (diff_status, diff_summary_lines, diff_structural_lines, diff_raw_lines) =
        build_interactive_import_diff_summary_with_client(
            client,
            lookup_cache,
            &document,
            &payload,
            uid,
        )?;
    Ok(InteractiveImportReview {
        action: action.to_string(),
        destination: destination.to_string(),
        action_label: action_label.to_string(),
        folder_path,
        source_folder_path: normalized_source_folder_path,
        destination_folder_path: normalized_destination_folder_path.unwrap_or_default(),
        reason: reason.to_string(),
        diff_status,
        diff_summary_lines,
        diff_structural_lines,
        diff_raw_lines,
    })
}

fn build_interactive_import_diff_summary_with_request<F>(
    request_json: &mut F,
    lookup_cache: &mut ImportLookupCache,
    local_document: &Value,
    payload: &Value,
    uid: &str,
) -> Result<InteractiveImportDiffData>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if uid.is_empty() {
        return Ok((
            "new dashboard".to_string(),
            vec!["No live dashboard exists yet; import would create a new item.".to_string()],
            vec!["No live dashboard exists yet.".to_string()],
            vec![
                "REMOTE <missing>".to_string(),
                "LOCAL <new dashboard payload>".to_string(),
            ],
        ));
    }
    let Some(remote_payload) = fetch_dashboard_if_exists_cached(request_json, lookup_cache, uid)?
    else {
        return Ok((
            "new dashboard".to_string(),
            vec!["No live dashboard exists yet; import would create a new item.".to_string()],
            vec!["No live dashboard exists yet.".to_string()],
            vec![
                "REMOTE <missing>".to_string(),
                "LOCAL <new dashboard payload>".to_string(),
            ],
        ));
    };
    let payload_object =
        value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let local_dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let remote_dashboard_value = build_preserved_web_import_document(&remote_payload)?;
    let remote_dashboard = value_as_object(
        &remote_dashboard_value,
        "Unexpected dashboard payload from Grafana.",
    )?;
    let local_title = string_field(local_dashboard, "title", uid);
    let remote_title = string_field(remote_dashboard, "title", uid);
    let local_folder_uid = payload_object
        .get("folderUid")
        .and_then(Value::as_str)
        .unwrap_or("");
    let remote_folder_uid = value_as_object(
        &remote_payload,
        "Unexpected dashboard payload from Grafana.",
    )?
    .get("meta")
    .and_then(Value::as_object)
    .map(|meta| string_field(meta, "folderUid", ""))
    .unwrap_or_default();
    let local_tags = join_tags(local_dashboard.get("tags"));
    let remote_tags = join_tags(remote_dashboard.get("tags"));
    let local_panels = panel_count(local_document);
    let remote_panels = panel_count(&remote_dashboard_value);
    let local_variables = variable_count(local_document);
    let remote_variables = variable_count(&remote_dashboard_value);

    let mut summary_lines = Vec::new();
    if local_title != remote_title {
        summary_lines.push(format!(
            "Title: {} -> {}",
            display_text(&remote_title),
            display_text(&local_title)
        ));
    }
    if local_folder_uid != remote_folder_uid {
        summary_lines.push(format!(
            "Folder UID: {} -> {}",
            display_text(&remote_folder_uid),
            display_text(local_folder_uid)
        ));
    }
    if local_tags != remote_tags {
        summary_lines.push(format!(
            "Tags: {} -> {}",
            display_text(&remote_tags),
            display_text(&local_tags)
        ));
    }
    if local_panels != remote_panels {
        summary_lines.push(format!("Panels: {} -> {}", remote_panels, local_panels));
    }
    let mut structural_lines = summary_lines.clone();
    if local_variables != remote_variables {
        structural_lines.push(format!(
            "Variables: {} -> {}",
            remote_variables, local_variables
        ));
    }
    let raw_lines = build_raw_diff_lines(&remote_payload, payload)?;

    if summary_lines.is_empty() {
        Ok((
            "matches live".to_string(),
            vec!["Import payload already matches the live dashboard shape.".to_string()],
            vec![
                "Title, folder, tags, panels, and variables match the live dashboard.".to_string(),
            ],
            raw_lines,
        ))
    } else {
        Ok((
            "changed".to_string(),
            summary_lines,
            structural_lines,
            raw_lines,
        ))
    }
}

fn build_interactive_import_diff_summary_with_client(
    client: &DashboardResourceClient<'_>,
    lookup_cache: &mut ImportLookupCache,
    local_document: &Value,
    payload: &Value,
    uid: &str,
) -> Result<InteractiveImportDiffData> {
    if uid.is_empty() {
        return Ok((
            "new dashboard".to_string(),
            vec!["No live dashboard exists yet; import would create a new item.".to_string()],
            vec!["No live dashboard exists yet.".to_string()],
            vec![
                "REMOTE <missing>".to_string(),
                "LOCAL <new dashboard payload>".to_string(),
            ],
        ));
    }
    let Some(remote_payload) =
        fetch_dashboard_if_exists_cached_with_client(client, lookup_cache, uid)?
    else {
        return Ok((
            "new dashboard".to_string(),
            vec!["No live dashboard exists yet; import would create a new item.".to_string()],
            vec!["No live dashboard exists yet.".to_string()],
            vec![
                "REMOTE <missing>".to_string(),
                "LOCAL <new dashboard payload>".to_string(),
            ],
        ));
    };
    let payload_object =
        value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let local_dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let remote_dashboard_value = build_preserved_web_import_document(&remote_payload)?;
    let remote_dashboard = value_as_object(
        &remote_dashboard_value,
        "Unexpected dashboard payload from Grafana.",
    )?;
    let local_title = string_field(local_dashboard, "title", uid);
    let remote_title = string_field(remote_dashboard, "title", uid);
    let local_folder_uid = payload_object
        .get("folderUid")
        .and_then(Value::as_str)
        .unwrap_or("");
    let remote_folder_uid = value_as_object(
        &remote_payload,
        "Unexpected dashboard payload from Grafana.",
    )?
    .get("meta")
    .and_then(Value::as_object)
    .map(|meta| string_field(meta, "folderUid", ""))
    .unwrap_or_default();
    let local_tags = join_tags(local_dashboard.get("tags"));
    let remote_tags = join_tags(remote_dashboard.get("tags"));
    let local_panels = panel_count(local_document);
    let remote_panels = panel_count(&remote_dashboard_value);
    let local_variables = variable_count(local_document);
    let remote_variables = variable_count(&remote_dashboard_value);

    let mut summary_lines = Vec::new();
    if local_title != remote_title {
        summary_lines.push(format!(
            "Title: {} -> {}",
            display_text(&remote_title),
            display_text(&local_title)
        ));
    }
    if local_folder_uid != remote_folder_uid {
        summary_lines.push(format!(
            "Folder UID: {} -> {}",
            display_text(&remote_folder_uid),
            display_text(local_folder_uid)
        ));
    }
    if local_tags != remote_tags {
        summary_lines.push(format!(
            "Tags: {} -> {}",
            display_text(&remote_tags),
            display_text(&local_tags)
        ));
    }
    if local_panels != remote_panels {
        summary_lines.push(format!("Panels: {} -> {}", remote_panels, local_panels));
    }
    let mut structural_lines = summary_lines.clone();
    if local_variables != remote_variables {
        structural_lines.push(format!(
            "Variables: {} -> {}",
            remote_variables, local_variables
        ));
    }
    let raw_lines = build_raw_diff_lines(&remote_payload, payload)?;

    if summary_lines.is_empty() {
        Ok((
            "matches live".to_string(),
            vec!["Import payload already matches the live dashboard shape.".to_string()],
            vec![
                "Title, folder, tags, panels, and variables match the live dashboard.".to_string(),
            ],
            raw_lines,
        ))
    } else {
        Ok((
            "changed".to_string(),
            summary_lines,
            structural_lines,
            raw_lines,
        ))
    }
}

fn join_tags(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn display_text(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn panel_count(document: &Value) -> usize {
    let Ok(object) = value_as_object(document, "Dashboard payload must be a JSON object.") else {
        return 0;
    };
    let Ok(dashboard) = super::extract_dashboard_object(object) else {
        return 0;
    };
    dashboard
        .get("panels")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

fn variable_count(document: &Value) -> usize {
    let Ok(object) = value_as_object(document, "Dashboard payload must be a JSON object.") else {
        return 0;
    };
    let Ok(dashboard) = super::extract_dashboard_object(object) else {
        return 0;
    };
    dashboard
        .get("templating")
        .and_then(Value::as_object)
        .and_then(|templating| templating.get("list"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

fn build_raw_diff_lines(remote_payload: &Value, payload: &Value) -> Result<Vec<String>> {
    let remote = serde_json::to_string_pretty(remote_payload)?;
    let local = serde_json::to_string_pretty(payload)?;
    let mut lines = vec!["REMOTE".to_string()];
    lines.extend(remote.lines().take(12).map(|line| format!("- {line}")));
    lines.push("LOCAL".to_string());
    lines.extend(local.lines().take(12).map(|line| format!("+ {line}")));
    Ok(lines)
}
