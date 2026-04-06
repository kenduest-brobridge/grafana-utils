//! Live dashboard authoring helpers.
//!
//! These commands fetch one live dashboard, clear the numeric ID, and write a local
//! draft that can be edited and later imported with the existing dashboard import flow.
use reqwest::Method;
use serde_json::{Map, Value};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{message, render_json_value, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;
use crate::tabular_output::{render_summary_csv, render_summary_table, render_yaml};

#[cfg(test)]
use super::fetch_dashboard_with_request;
use super::validate::validate_dashboard_import_document;
use super::{
    extract_dashboard_object, fetch_dashboard, load_json_file, write_dashboard,
    write_json_document, CloneLiveArgs, GetArgs, ImportArgs, PatchFileArgs, PublishArgs,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardAuthoringReviewResult {
    pub(crate) input_file: String,
    pub(crate) document_kind: String,
    pub(crate) title: String,
    pub(crate) uid: String,
    pub(crate) folder_uid: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) dashboard_id_is_null: bool,
    pub(crate) meta_message_present: bool,
    pub(crate) blocking_issues: Vec<String>,
    pub(crate) suggested_next_action: String,
}

const STDIN_PATH_TOKEN: &str = "-";
const STDIN_DISPLAY_LABEL: &str = "<stdin>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardInputDocument {
    pub(crate) document: Value,
    pub(crate) display_label: String,
    pub(crate) validation_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileWatchFingerprint {
    modified_millis: u128,
    len: u64,
}

fn is_stdin_path(path: &Path) -> bool {
    path.to_str() == Some(STDIN_PATH_TOKEN)
}

fn parse_dashboard_input_json(raw: &str, source_label: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(raw).map_err(|error| {
        message(format!(
            "Failed to parse dashboard JSON from {source_label}: {error}"
        ))
    })?;
    if !value.is_object() {
        return Err(message(format!(
            "Dashboard input must contain a JSON object: {source_label}"
        )));
    }
    Ok(value)
}

pub(crate) fn load_dashboard_input_document_from_reader<R: Read>(
    mut reader: R,
    source_label: &str,
    validation_path: PathBuf,
) -> Result<DashboardInputDocument> {
    let mut raw = String::new();
    reader.read_to_string(&mut raw)?;
    let document = parse_dashboard_input_json(&raw, source_label)?;
    Ok(DashboardInputDocument {
        document,
        display_label: source_label.to_string(),
        validation_path,
    })
}

fn load_dashboard_input_document(input: &Path) -> Result<DashboardInputDocument> {
    if is_stdin_path(input) {
        return load_dashboard_input_document_from_reader(
            io::stdin(),
            STDIN_DISPLAY_LABEL,
            PathBuf::from(STDIN_DISPLAY_LABEL),
        );
    }

    Ok(DashboardInputDocument {
        document: load_json_file(input)?,
        display_label: input.display().to_string(),
        validation_path: input.to_path_buf(),
    })
}

fn validate_publish_args(args: &PublishArgs) -> Result<()> {
    if args.watch && is_stdin_path(&args.input) {
        return Err(message(
            "--watch cannot be combined with --input -. Point --input at a local file.",
        ));
    }
    Ok(())
}

fn validate_patch_file_args(args: &PatchFileArgs) -> Result<()> {
    if is_stdin_path(&args.input) && args.output.is_none() {
        return Err(message(
            "patch-file --input - requires --output because standard input cannot be overwritten in place.",
        ));
    }
    Ok(())
}

fn current_file_watch_fingerprint(path: &Path) -> Result<Option<FileWatchFingerprint>> {
    match fs::metadata(path) {
        Ok(metadata) => {
            let modified = metadata
                .modified()
                .map_err(|error| {
                    message(format!(
                        "Failed to read file modification time for {}: {error}",
                        path.display()
                    ))
                })?
                .duration_since(UNIX_EPOCH)
                .map_err(|error| {
                    message(format!(
                        "File modification time is before UNIX_EPOCH for {}: {error}",
                        path.display()
                    ))
                })?;
            Ok(Some(FileWatchFingerprint {
                modified_millis: modified.as_millis(),
                len: metadata.len(),
            }))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn watch_start_message(path: &Path) -> String {
    format!(
        "Watching {} for dashboard publish changes. Press Ctrl-C to stop.",
        path.display()
    )
}

pub(crate) fn watch_change_detected_message(path: &Path) -> String {
    format!(
        "Detected dashboard input change for {}; waiting for a stable save.",
        path.display()
    )
}

pub(crate) fn watch_change_unstable_message(path: &Path) -> String {
    format!(
        "Dashboard input changed again before it stabilized; still watching {}.",
        path.display()
    )
}

fn set_meta_folder_uid(document: &mut Map<String, Value>, folder_uid: &str) -> Result<()> {
    if folder_uid.is_empty() {
        return Ok(());
    }
    let meta = document
        .entry("meta".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let meta_object = meta.as_object_mut().ok_or_else(|| {
        message("Unexpected dashboard payload from Grafana: meta must be a JSON object.")
    })?;
    meta_object.insert(
        "folderUid".to_string(),
        Value::String(folder_uid.to_string()),
    );
    Ok(())
}

fn build_authoring_document(
    payload: &Value,
    title_override: Option<&str>,
    uid_override: Option<&str>,
    folder_uid_override: Option<&str>,
) -> Result<Value> {
    let object = value_as_object(payload, "Unexpected dashboard payload from Grafana.")?;
    let mut document = object.clone();
    let mut dashboard = extract_dashboard_object(&document)?.clone();
    dashboard.insert("id".to_string(), Value::Null);
    if let Some(title) = title_override {
        dashboard.insert("title".to_string(), Value::String(title.to_string()));
    }
    if let Some(uid) = uid_override {
        dashboard.insert("uid".to_string(), Value::String(uid.to_string()));
    }
    document.insert("dashboard".to_string(), Value::Object(dashboard));
    if let Some(folder_uid) = folder_uid_override {
        set_meta_folder_uid(&mut document, folder_uid)?;
    }
    Ok(Value::Object(document))
}

fn patch_dashboard_document(document: &mut Value, args: &PatchFileArgs) -> Result<()> {
    let object = document.as_object_mut().ok_or_else(|| {
        message("Dashboard patch-file expects a JSON object at the document root.")
    })?;
    let has_wrapper = object.contains_key("dashboard");
    if has_wrapper {
        let dashboard = object
            .get_mut("dashboard")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| {
                message("Dashboard patch-file expects dashboard to be a JSON object.")
            })?;
        if let Some(name) = args.name.as_ref() {
            dashboard.insert("title".to_string(), Value::String(name.clone()));
        }
        if let Some(uid) = args.uid.as_ref() {
            dashboard.insert("uid".to_string(), Value::String(uid.clone()));
        }
        if !args.tags.is_empty() {
            dashboard.insert(
                "tags".to_string(),
                Value::Array(args.tags.iter().cloned().map(Value::String).collect()),
            );
        }
    } else {
        if let Some(name) = args.name.as_ref() {
            object.insert("title".to_string(), Value::String(name.clone()));
        }
        if let Some(uid) = args.uid.as_ref() {
            object.insert("uid".to_string(), Value::String(uid.clone()));
        }
        if !args.tags.is_empty() {
            object.insert(
                "tags".to_string(),
                Value::Array(args.tags.iter().cloned().map(Value::String).collect()),
            );
        }
    }

    if args.folder_uid.is_some() || args.message.is_some() {
        let meta = object
            .entry("meta".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let meta = meta
            .as_object_mut()
            .ok_or_else(|| message("Dashboard patch-file expects meta to be a JSON object."))?;
        if let Some(folder_uid) = args.folder_uid.as_ref() {
            meta.insert("folderUid".to_string(), Value::String(folder_uid.clone()));
        }
        if let Some(message_text) = args.message.as_ref() {
            meta.insert("message".to_string(), Value::String(message_text.clone()));
        }
    }
    Ok(())
}

fn join_tags(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn review_next_action(has_blocking_issues: bool, dashboard_id_is_null: bool) -> String {
    let base_action = if dashboard_id_is_null {
        "publish --dry-run"
    } else {
        "patch-file"
    };
    if has_blocking_issues {
        format!("fix blocking issues, then {base_action}")
    } else {
        base_action.to_string()
    }
}

pub(crate) fn review_dashboard_file(input: &Path) -> Result<DashboardAuthoringReviewResult> {
    let dashboard_input = load_dashboard_input_document(input)?;
    let document = dashboard_input.document;
    let document_object = value_as_object(&document, "Dashboard review expects a JSON object.")?;
    let dashboard = extract_dashboard_object(document_object)?;
    let document_kind = if document_object.contains_key("dashboard") {
        "wrapped".to_string()
    } else {
        "bare".to_string()
    };
    let title = string_field(dashboard, "title", "");
    let uid = string_field(dashboard, "uid", "");
    let folder_uid = document_object
        .get("meta")
        .and_then(Value::as_object)
        .map(|meta| string_field(meta, "folderUid", ""))
        .filter(|value| !value.is_empty());
    let tags = join_tags(dashboard.get("tags"));
    let dashboard_id_is_null = matches!(dashboard.get("id"), Some(Value::Null));
    let meta_message_present = document_object
        .get("meta")
        .and_then(Value::as_object)
        .map(|meta| meta.contains_key("message"))
        .unwrap_or(false);

    let blocking_issues = match validate_dashboard_import_document(
        &document,
        &dashboard_input.validation_path,
        false,
        None,
    ) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    };
    let has_blocking_issues = !blocking_issues.is_empty();
    let suggested_next_action = review_next_action(has_blocking_issues, dashboard_id_is_null);

    Ok(DashboardAuthoringReviewResult {
        input_file: dashboard_input.display_label,
        document_kind,
        title,
        uid,
        folder_uid,
        tags,
        dashboard_id_is_null,
        meta_message_present,
        blocking_issues,
        suggested_next_action,
    })
}

fn review_result_document(result: &DashboardAuthoringReviewResult) -> Value {
    serde_json::json!({
        "kind": "grafana-utils-dashboard-authoring-review",
        "schemaVersion": 1,
        "summary": {
            "inputFile": result.input_file,
            "documentKind": result.document_kind,
            "title": result.title,
            "uid": result.uid,
            "folderUid": result.folder_uid,
            "tags": result.tags,
            "dashboardIdState": if result.dashboard_id_is_null { "null" } else { "non-null" },
            "dashboardIdIsNull": result.dashboard_id_is_null,
            "metaMessagePresent": result.meta_message_present,
            "suggestedNextAction": result.suggested_next_action,
        },
        "blockingIssues": result.blocking_issues,
    })
}

pub(crate) fn render_dashboard_review_text(result: &DashboardAuthoringReviewResult) -> Vec<String> {
    let mut lines = vec![
        "Dashboard authoring review".to_string(),
        format!("File: {}", result.input_file),
        format!("Kind: {}", result.document_kind),
        format!("Title: {}", display_text(&result.title)),
        format!("UID: {}", display_text(&result.uid)),
    ];
    if let Some(folder_uid) = &result.folder_uid {
        lines.push(format!("Folder UID: {folder_uid}"));
    }
    lines.push(format!(
        "Tags: {}",
        if result.tags.is_empty() {
            "-".to_string()
        } else {
            result.tags.join(", ")
        }
    ));
    lines.push(format!(
        "dashboard.id: {}",
        if result.dashboard_id_is_null {
            "null"
        } else {
            "non-null"
        }
    ));
    lines.push(format!(
        "meta.message: {}",
        if result.meta_message_present {
            "present"
        } else {
            "absent"
        }
    ));
    if result.blocking_issues.is_empty() {
        lines.push("Blocking issues: none".to_string());
    } else {
        lines.push("Blocking issues:".to_string());
        for issue in &result.blocking_issues {
            lines.push(format!("- {issue}"));
        }
    }
    lines.push(format!("Next action: {}", result.suggested_next_action));
    lines
}

fn review_result_summary_rows(
    result: &DashboardAuthoringReviewResult,
) -> Vec<(&'static str, String)> {
    let mut rows = vec![
        ("file", result.input_file.clone()),
        ("kind", result.document_kind.clone()),
        ("title", display_text(&result.title)),
        ("uid", display_text(&result.uid)),
        (
            "folder_uid",
            result.folder_uid.clone().unwrap_or_else(|| "-".to_string()),
        ),
        (
            "tags",
            if result.tags.is_empty() {
                "-".to_string()
            } else {
                result.tags.join(", ")
            },
        ),
        (
            "dashboard_id",
            if result.dashboard_id_is_null {
                "null".to_string()
            } else {
                "non-null".to_string()
            },
        ),
        (
            "meta_message",
            if result.meta_message_present {
                "present".to_string()
            } else {
                "absent".to_string()
            },
        ),
    ];
    if result.blocking_issues.is_empty() {
        rows.push(("blocking_issues", "none".to_string()));
    } else {
        rows.push(("blocking_issues", result.blocking_issues.join(" | ")));
    }
    rows.push(("next_action", result.suggested_next_action.clone()));
    rows
}

pub(crate) fn render_dashboard_review_table(
    result: &DashboardAuthoringReviewResult,
) -> Vec<String> {
    render_summary_table(&review_result_summary_rows(result))
}

pub(crate) fn render_dashboard_review_csv(result: &DashboardAuthoringReviewResult) -> Vec<String> {
    render_summary_csv(&review_result_summary_rows(result))
}

pub(crate) fn render_dashboard_review_json(
    result: &DashboardAuthoringReviewResult,
) -> Result<String> {
    render_json_value(&review_result_document(result))
}

pub(crate) fn render_dashboard_review_yaml(
    result: &DashboardAuthoringReviewResult,
) -> Result<String> {
    Ok(format!(
        "{}\n",
        render_yaml(&review_result_document(result))?
    ))
}

fn display_text(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

pub(crate) fn patch_dashboard_file(args: &PatchFileArgs) -> Result<()> {
    validate_patch_file_args(args)?;
    let dashboard_input = load_dashboard_input_document(&args.input)?;
    let mut document = dashboard_input.document;
    validate_dashboard_import_document(&document, &dashboard_input.validation_path, false, None)?;
    patch_dashboard_document(&mut document, args)?;
    let output_path = args.output.as_ref().unwrap_or(&args.input);
    write_json_document(&document, output_path)
}

fn build_publish_import_args(temp_input: PathBuf, args: &PublishArgs) -> ImportArgs {
    ImportArgs {
        common: args.common.clone(),
        org_id: None,
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        input_dir: temp_input,
        input_format: super::DashboardImportInputFormat::Raw,
        import_folder_uid: args.folder_uid.clone(),
        ensure_folders: false,
        replace_existing: args.replace_existing,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: false,
        strict_schema: false,
        target_schema_version: None,
        import_message: args.message.clone(),
        interactive: false,
        dry_run: args.dry_run,
        table: args.table,
        json: args.json,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    }
}

fn publish_dashboard_once_with_request<F>(request_json: &mut F, args: &PublishArgs) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_publish_args(args)?;
    let dashboard_input = load_dashboard_input_document(&args.input)?;
    validate_dashboard_import_document(
        &dashboard_input.document,
        &dashboard_input.validation_path,
        false,
        None,
    )?;
    let temp_dir = std::env::temp_dir().join(format!(
        "grafana-dashboard-publish-{}-{}",
        process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| message(format!("Failed to build publish temp path: {error}")))?
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir)?;
    let result = (|| -> Result<()> {
        let staged_path = temp_dir.join("dashboard.json");
        write_dashboard(&dashboard_input.document, &staged_path, false)?;
        let import_args = build_publish_import_args(temp_dir.clone(), args);
        let _ = super::import::import_dashboards_with_request(request_json, &import_args)?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&temp_dir);
    result
}

pub(crate) fn publish_dashboard_with_request<F>(
    mut request_json: F,
    args: &PublishArgs,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    publish_dashboard_once_with_request(&mut request_json, args)
}

fn watch_publish_dashboard_with_client(client: &JsonHttpClient, args: &PublishArgs) -> Result<()> {
    validate_publish_args(args)?;
    let input_path = &args.input;
    eprintln!("{}", watch_start_message(input_path));

    match publish_dashboard_once_with_request(
        &mut |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    ) {
        Ok(()) => {}
        Err(error) => eprintln!("Initial publish failed: {error}"),
    }

    let mut last_seen = current_file_watch_fingerprint(input_path)?;
    loop {
        thread::sleep(Duration::from_millis(500));
        let current = current_file_watch_fingerprint(input_path)?;
        if current == last_seen {
            continue;
        }

        eprintln!("{}", watch_change_detected_message(input_path));

        thread::sleep(Duration::from_millis(300));
        let stabilized = current_file_watch_fingerprint(input_path)?;
        if stabilized != current {
            last_seen = stabilized;
            eprintln!("{}", watch_change_unstable_message(input_path));
            continue;
        }

        match stabilized {
            Some(_) => match publish_dashboard_once_with_request(
                &mut |method, path, params, payload| {
                    client.request_json(method, path, params, payload)
                },
                args,
            ) {
                Ok(()) => eprintln!("Re-ran dashboard publish for {}.", input_path.display()),
                Err(error) => eprintln!(
                    "Dashboard publish failed for {}: {error}",
                    input_path.display()
                ),
            },
            None => eprintln!(
                "Dashboard input file is missing; still watching {}.",
                input_path.display()
            ),
        }
        last_seen = stabilized;
    }
}

pub(crate) fn publish_dashboard_with_client(
    client: &JsonHttpClient,
    args: &PublishArgs,
) -> Result<()> {
    if args.watch {
        watch_publish_dashboard_with_client(client, args)
    } else {
        publish_dashboard_with_request(
            |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
        )?;
        Ok(())
    }
}

pub(crate) fn build_live_dashboard_authoring_document(
    payload: &Value,
    title_override: Option<&str>,
    uid_override: Option<&str>,
    folder_uid_override: Option<&str>,
) -> Result<Value> {
    build_authoring_document(payload, title_override, uid_override, folder_uid_override)
}

#[cfg(test)]
pub(crate) fn build_live_dashboard_authoring_document_with_request<F>(
    mut request_json: F,
    uid: &str,
    title_override: Option<&str>,
    uid_override: Option<&str>,
    folder_uid_override: Option<&str>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    build_live_dashboard_authoring_document(
        &payload,
        title_override,
        uid_override,
        folder_uid_override,
    )
}

pub(crate) fn get_live_dashboard_to_file_with_client(
    client: &JsonHttpClient,
    args: &GetArgs,
) -> Result<()> {
    let payload = fetch_dashboard(client, &args.dashboard_uid)?;
    let document = build_live_dashboard_authoring_document(&payload, None, None, None)?;
    write_dashboard(&document, &args.output, false)
}

pub(crate) fn clone_live_dashboard_to_file_with_client(
    client: &JsonHttpClient,
    args: &CloneLiveArgs,
) -> Result<()> {
    let payload = fetch_dashboard(client, &args.source_uid)?;
    let document = build_live_dashboard_authoring_document(
        &payload,
        args.name.as_deref(),
        args.uid.as_deref(),
        args.folder_uid.as_deref(),
    )?;
    write_dashboard(&document, &args.output, false)
}

#[cfg(test)]
pub(crate) fn get_live_dashboard_to_file_with_request<F>(
    request_json: F,
    uid: &str,
    output: &Path,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let document =
        build_live_dashboard_authoring_document_with_request(request_json, uid, None, None, None)?;
    write_dashboard(&document, output, false)?;
    Ok(document)
}

#[cfg(test)]
pub(crate) fn clone_live_dashboard_to_file_with_request<F>(
    request_json: F,
    source_uid: &str,
    output: &Path,
    title_override: Option<&str>,
    uid_override: Option<&str>,
    folder_uid_override: Option<&str>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let document = build_live_dashboard_authoring_document_with_request(
        request_json,
        source_uid,
        title_override,
        uid_override,
        folder_uid_override,
    )?;
    write_dashboard(&document, output, false)?;
    Ok(document)
}
