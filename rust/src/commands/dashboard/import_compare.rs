//! Import orchestration for Dashboard resources, including input normalization and apply contract handling.

use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::path::Path;

use crate::common::{
    build_shared_diff_document, message, object_field, render_json_value, string_field,
    value_as_object, Result, SharedDiffSummary,
};

use super::{build_import_payload, build_preserved_web_import_document, DEFAULT_FOLDER_UID};

pub(crate) fn build_compare_document(
    dashboard: &Map<String, Value>,
    folder_uid: Option<&str>,
) -> Value {
    let mut compare = Map::new();
    compare.insert("dashboard".to_string(), Value::Object(dashboard.clone()));
    if let Some(folder_uid) = folder_uid
        .filter(|value| !value.is_empty())
        .filter(|value| *value != DEFAULT_FOLDER_UID)
    {
        compare.insert(
            "folderUid".to_string(),
            Value::String(folder_uid.to_string()),
        );
    }
    Value::Object(compare)
}

fn build_local_compare_document(
    document: &Value,
    folder_uid_override: Option<&str>,
) -> Result<Value> {
    let payload = build_import_payload(document, folder_uid_override, false, "")?;
    let payload_object =
        value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
    let dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let folder_uid = payload_object.get("folderUid").and_then(Value::as_str);
    Ok(build_compare_document(dashboard, folder_uid))
}

fn build_remote_compare_document(
    payload: &Value,
    folder_uid_override: Option<&str>,
) -> Result<Value> {
    let dashboard = build_preserved_web_import_document(payload)?;
    let dashboard_object =
        value_as_object(&dashboard, "Unexpected dashboard payload from Grafana.")?;
    let payload_object = value_as_object(payload, "Unexpected dashboard payload from Grafana.")?;
    let folder_uid = folder_uid_override.or_else(|| {
        object_field(payload_object, "meta")
            .and_then(|meta| meta.get("folderUid"))
            .and_then(Value::as_str)
    });
    Ok(build_compare_document(dashboard_object, folder_uid))
}

pub(crate) fn serialize_compare_document(document: &Value) -> Result<String> {
    Ok(serde_json::to_string(document)?)
}

pub(crate) fn build_compare_diff_text_with_labels(
    base_compare: &Value,
    new_compare: &Value,
    base_label: &str,
    new_label: &str,
    _context_lines: usize,
) -> Result<String> {
    let remote_pretty = serde_json::to_string_pretty(base_compare)?;
    let local_pretty = serde_json::to_string_pretty(new_compare)?;
    let mut text = String::new();
    let _ = writeln!(&mut text, "--- {base_label}");
    let _ = writeln!(&mut text, "+++ {new_label}");
    for line in remote_pretty.lines() {
        let _ = writeln!(&mut text, "-{line}");
    }
    for line in local_pretty.lines() {
        let _ = writeln!(&mut text, "+{line}");
    }
    Ok(text)
}

fn build_compare_diff_text(
    remote_compare: &Value,
    local_compare: &Value,
    uid: &str,
    dashboard_file: &Path,
    context_lines: usize,
) -> Result<String> {
    build_compare_diff_text_with_labels(
        remote_compare,
        local_compare,
        &format!("grafana:{uid}"),
        &dashboard_file.display().to_string(),
        context_lines,
    )
}

pub(crate) fn diff_dashboards_with_request<F>(
    mut request_json: F,
    args: &super::DiffArgs,
) -> Result<usize>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let resolved = super::import::resolve_diff_source(args)?;
    let expected_variant = super::DashboardSourceKind::from_import_input_format(args.input_format)
        .expected_variant()
        .ok_or_else(|| message("Dashboard diff local mode requires an export-backed source."))?;
    let _ = super::load_export_metadata(resolved.metadata_dir(), Some(expected_variant))?;
    let dashboard_files = super::import::dashboard_files_for_import(resolved.dashboard_dir())?;
    let mut differences = 0;
    let mut rows = Vec::new();
    for dashboard_file in &dashboard_files {
        let document = super::load_json_file(dashboard_file)?;
        let payload = build_import_payload(&document, None, false, "")?;
        let payload_object =
            value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
        let dashboard = payload_object
            .get("dashboard")
            .and_then(Value::as_object)
            .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
        let uid = string_field(dashboard, "uid", "");
        let local_compare =
            build_local_compare_document(&document, args.import_folder_uid.as_deref())?;
        let Some(remote_payload) =
            super::fetch_dashboard_if_exists_with_request(&mut request_json, &uid)?
        else {
            if matches!(args.output_format, crate::common::DiffOutputFormat::Text) {
                println!(
                    "Diff missing in Grafana for uid={} from {}",
                    uid,
                    dashboard_file.display()
                );
            }
            rows.push(serde_json::json!({
                "domain": "dashboard",
                "resourceKind": "dashboard",
                "identity": uid,
                "status": "missing-remote",
                "path": dashboard_file.display().to_string(),
                "changedFields": Vec::<String>::new(),
                "diffText": Value::Null,
                "contextLines": args.context_lines,
            }));
            differences += 1;
            continue;
        };
        let remote_compare =
            build_remote_compare_document(&remote_payload, args.import_folder_uid.as_deref())?;
        if serialize_compare_document(&local_compare)?
            != serialize_compare_document(&remote_compare)?
        {
            let diff_text = build_compare_diff_text(
                &remote_compare,
                &local_compare,
                &uid,
                dashboard_file,
                args.context_lines,
            )?;
            if matches!(args.output_format, crate::common::DiffOutputFormat::Text) {
                println!("{diff_text}");
            }
            rows.push(serde_json::json!({
                "domain": "dashboard",
                "resourceKind": "dashboard",
                "identity": uid,
                "status": "different",
                "path": dashboard_file.display().to_string(),
                "changedFields": ["dashboard"],
                "diffText": diff_text,
                "contextLines": args.context_lines,
            }));
            differences += 1;
        } else {
            if matches!(args.output_format, crate::common::DiffOutputFormat::Text) {
                println!("Diff matched uid={} for {}", uid, dashboard_file.display());
            }
            rows.push(serde_json::json!({
                "domain": "dashboard",
                "resourceKind": "dashboard",
                "identity": uid,
                "status": "same",
                "path": dashboard_file.display().to_string(),
                "changedFields": Vec::<String>::new(),
                "diffText": Value::Null,
                "contextLines": args.context_lines,
            }));
        }
    }
    match args.output_format {
        crate::common::DiffOutputFormat::Text => {
            println!(
                "Diff checked {} dashboard(s); {} difference(s) found.",
                dashboard_files.len(),
                differences
            );
        }
        crate::common::DiffOutputFormat::Json => {
            let same = rows
                .iter()
                .filter(|row| row.get("status").and_then(Value::as_str) == Some("same"))
                .count();
            let different = rows
                .iter()
                .filter(|row| row.get("status").and_then(Value::as_str) == Some("different"))
                .count();
            let missing_remote = rows
                .iter()
                .filter(|row| row.get("status").and_then(Value::as_str) == Some("missing-remote"))
                .count();
            print!(
                "{}",
                render_json_value(&build_shared_diff_document(
                    "grafana-util-dashboard-diff",
                    1,
                    SharedDiffSummary {
                        checked: dashboard_files.len(),
                        same,
                        different,
                        missing_remote,
                        extra_remote: 0,
                        ambiguous: 0,
                    },
                    &rows,
                ))?
            );
        }
    }
    Ok(differences)
}
