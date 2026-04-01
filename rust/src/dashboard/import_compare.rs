//! Import orchestration for Dashboard resources, including input normalization and apply contract handling.

use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::path::Path;

use crate::common::{message, object_field, string_field, value_as_object, Result};

use super::{build_import_payload, build_preserved_web_import_document};

fn build_compare_document(dashboard: &Map<String, Value>, folder_uid: Option<&str>) -> Value {
    let mut compare = Map::new();
    compare.insert("dashboard".to_string(), Value::Object(dashboard.clone()));
    if let Some(folder_uid) = folder_uid.filter(|value| !value.is_empty()) {
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

fn serialize_compare_document(document: &Value) -> Result<String> {
    Ok(serde_json::to_string(document)?)
}

fn build_compare_diff_text(
    remote_compare: &Value,
    local_compare: &Value,
    uid: &str,
    dashboard_file: &Path,
    _context_lines: usize,
) -> Result<String> {
    let remote_pretty = serde_json::to_string_pretty(remote_compare)?;
    let local_pretty = serde_json::to_string_pretty(local_compare)?;
    let mut text = String::new();
    let _ = writeln!(&mut text, "--- grafana:{uid}");
    let _ = writeln!(&mut text, "+++ {}", dashboard_file.display());
    for line in remote_pretty.lines() {
        let _ = writeln!(&mut text, "-{line}");
    }
    for line in local_pretty.lines() {
        let _ = writeln!(&mut text, "+{line}");
    }
    Ok(text)
}

pub(crate) fn diff_dashboards_with_request<F>(
    mut request_json: F,
    args: &super::DiffArgs,
) -> Result<usize>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let resolved = super::import::resolve_diff_source(args)?;
    let expected_variant = match args.input_format {
        super::DashboardImportInputFormat::Raw => super::RAW_EXPORT_SUBDIR,
        super::DashboardImportInputFormat::Provisioning => super::PROVISIONING_EXPORT_SUBDIR,
    };
    let _ = super::load_export_metadata(&resolved.metadata_dir, Some(expected_variant))?;
    let dashboard_files = super::import::dashboard_files_for_import(&resolved.dashboard_dir)?;
    let mut differences = 0;
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
            println!(
                "Diff missing in Grafana for uid={} from {}",
                uid,
                dashboard_file.display()
            );
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
            println!("{diff_text}");
            differences += 1;
        } else {
            println!("Diff matched uid={} for {}", uid, dashboard_file.display());
        }
    }
    println!(
        "Diff checked {} dashboard(s); {} difference(s) found.",
        dashboard_files.len(),
        differences
    );
    Ok(differences)
}
