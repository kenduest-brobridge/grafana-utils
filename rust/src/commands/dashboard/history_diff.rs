use crate::common::{
    build_shared_diff_document, message, render_json_value, value_as_object, DiffOutputFormat,
    Result, SharedDiffSummary,
};
use reqwest::Method;
use serde_json::{json, Value};
use std::path::PathBuf;

use super::history_artifacts::{
    load_dashboard_history_export_document, load_history_artifact_for_uid,
};
use super::history_live::fetch_dashboard_history_version_data_with_request;
use super::history_render::render_dashboard_history_diff_text;
use super::history_types::{
    DashboardHistoryDiffDocument, DashboardHistoryExportDocument, HistoryDiffSource,
    ResolvedHistoryDiffSide, DASHBOARD_HISTORY_DIFF_KIND,
};
use super::import_compare::{
    build_compare_diff_text_with_labels, build_compare_document, serialize_compare_document,
};
use super::{
    string_field, tool_version, HistoryDiffArgs, DEFAULT_DASHBOARD_TITLE, TOOL_SCHEMA_VERSION,
};

fn resolve_history_diff_source(
    dashboard_uid: &Option<String>,
    input: &Option<PathBuf>,
    input_dir: &Option<PathBuf>,
    side: &str,
) -> Result<HistoryDiffSource> {
    match (dashboard_uid, input, input_dir) {
        (Some(uid), None, None) => Ok(HistoryDiffSource::Live {
            dashboard_uid: uid.clone(),
        }),
        (None, Some(path), None) => Ok(HistoryDiffSource::Artifact { path: path.clone() }),
        (Some(uid), None, Some(dir)) => Ok(HistoryDiffSource::ImportDir {
            input_dir: dir.clone(),
            dashboard_uid: uid.clone(),
        }),
        (None, None, Some(_)) => Err(message(format!(
            "dashboard history diff {side} side requires --{side}-dashboard-uid when --{side}-input-dir is set."
        ))),
        (None, None, None) => Err(message(format!(
            "dashboard history diff {side} side requires exactly one source: --{side}-dashboard-uid, --{side}-input, or --{side}-input-dir with --{side}-dashboard-uid."
        ))),
        _ => Err(message(format!(
            "dashboard history diff {side} side must choose exactly one source."
        ))),
    }
}

fn dashboard_history_export_version<'a>(
    document: &'a DashboardHistoryExportDocument,
    version: i64,
    label: &str,
) -> Result<&'a super::history_types::DashboardHistoryExportVersion> {
    document
        .versions
        .iter()
        .find(|item| item.version == version)
        .ok_or_else(|| {
            message(format!(
                "History source {label} does not contain dashboard version {version}."
            ))
        })
}

fn build_history_compare_document(dashboard: &Value) -> Result<Value> {
    let dashboard_object = value_as_object(
        dashboard,
        "Dashboard history artifact version did not include dashboard JSON.",
    )?;
    let folder_uid = dashboard_object.get("folderUid").and_then(Value::as_str);
    Ok(build_compare_document(dashboard_object, folder_uid))
}

fn resolve_history_diff_side_from_document(
    document: &DashboardHistoryExportDocument,
    label: String,
    version: i64,
) -> Result<ResolvedHistoryDiffSide> {
    let version_entry = dashboard_history_export_version(document, version, &label)?;
    Ok(ResolvedHistoryDiffSide {
        source_label: format!("{label}@{version}"),
        dashboard_uid: document.dashboard_uid.clone(),
        version,
        title: string_field(
            value_as_object(
                &version_entry.dashboard,
                "Dashboard history artifact version did not include dashboard JSON.",
            )?,
            "title",
            DEFAULT_DASHBOARD_TITLE,
        ),
        dashboard: version_entry.dashboard.clone(),
        compare_document: build_history_compare_document(&version_entry.dashboard)?,
    })
}

fn resolve_history_diff_side_with_request<F>(
    mut request_json: F,
    source: &HistoryDiffSource,
    version: i64,
) -> Result<ResolvedHistoryDiffSide>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match source {
        HistoryDiffSource::Live { dashboard_uid } => {
            let dashboard = Value::Object(fetch_dashboard_history_version_data_with_request(
                &mut request_json,
                dashboard_uid,
                version,
            )?);
            let dashboard_object = value_as_object(
                &dashboard,
                "Dashboard history version payload did not include dashboard data.",
            )?;
            Ok(ResolvedHistoryDiffSide {
                source_label: format!("grafana:{dashboard_uid}@{version}"),
                dashboard_uid: dashboard_uid.clone(),
                version,
                title: string_field(dashboard_object, "title", DEFAULT_DASHBOARD_TITLE),
                dashboard: dashboard.clone(),
                compare_document: build_history_compare_document(&dashboard)?,
            })
        }
        HistoryDiffSource::Artifact { path } => {
            let document = load_dashboard_history_export_document(path)?;
            resolve_history_diff_side_from_document(&document, path.display().to_string(), version)
        }
        HistoryDiffSource::ImportDir {
            input_dir,
            dashboard_uid,
        } => {
            let artifact = load_history_artifact_for_uid(input_dir, dashboard_uid)?;
            let label = artifact.path.display().to_string();
            resolve_history_diff_side_from_document(&artifact.document, label, version)
        }
    }
}

fn history_diff_identity(base_uid: &str, new_uid: &str) -> String {
    if base_uid == new_uid {
        base_uid.to_string()
    } else {
        format!("{base_uid} -> {new_uid}")
    }
}

fn build_dashboard_history_diff_document(
    base: &ResolvedHistoryDiffSide,
    new: &ResolvedHistoryDiffSide,
    context_lines: usize,
) -> Result<(DashboardHistoryDiffDocument, bool)> {
    let same = serialize_compare_document(&base.compare_document)?
        == serialize_compare_document(&new.compare_document)?;
    let diff_text = if same {
        Value::Null
    } else {
        Value::String(build_compare_diff_text_with_labels(
            &base.compare_document,
            &new.compare_document,
            &base.source_label,
            &new.source_label,
            context_lines,
        )?)
    };
    let status = if same { "same" } else { "different" };
    let rows = vec![json!({
        "domain": "dashboard",
        "resourceKind": "dashboard-history",
        "identity": history_diff_identity(&base.dashboard_uid, &new.dashboard_uid),
        "status": status,
        "path": format!("{} -> {}", base.source_label, new.source_label),
        "baseSource": base.source_label,
        "newSource": new.source_label,
        "baseVersion": base.version,
        "newVersion": new.version,
        "changedFields": if same { Vec::<String>::new() } else { vec!["dashboard".to_string()] },
        "diffText": diff_text,
        "contextLines": context_lines,
    })];
    Ok((
        DashboardHistoryDiffDocument {
            kind: DASHBOARD_HISTORY_DIFF_KIND.to_string(),
            schema_version: TOOL_SCHEMA_VERSION,
            tool_version: tool_version().to_string(),
            summary: SharedDiffSummary {
                checked: 1,
                same: usize::from(same),
                different: usize::from(!same),
                missing_remote: 0,
                extra_remote: 0,
                ambiguous: 0,
            },
            rows,
        },
        same,
    ))
}

#[allow(dead_code)]
pub(crate) fn build_dashboard_history_diff_document_with_request<F>(
    mut request_json: F,
    args: &HistoryDiffArgs,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let base_source = resolve_history_diff_source(
        &args.base_dashboard_uid,
        &args.base_input,
        &args.base_input_dir,
        "base",
    )?;
    let new_source = resolve_history_diff_source(
        &args.new_dashboard_uid,
        &args.new_input,
        &args.new_input_dir,
        "new",
    )?;
    let base =
        resolve_history_diff_side_with_request(&mut request_json, &base_source, args.base_version)?;
    let new =
        resolve_history_diff_side_with_request(&mut request_json, &new_source, args.new_version)?;
    let (document, _) = build_dashboard_history_diff_document(&base, &new, args.context_lines)?;
    Ok(build_shared_diff_document(
        &document.kind,
        document.schema_version,
        document.summary,
        &document.rows,
    ))
}

pub(crate) fn run_dashboard_history_diff<F>(
    mut request_json: F,
    args: &HistoryDiffArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let base_source = resolve_history_diff_source(
        &args.base_dashboard_uid,
        &args.base_input,
        &args.base_input_dir,
        "base",
    )?;
    let new_source = resolve_history_diff_source(
        &args.new_dashboard_uid,
        &args.new_input,
        &args.new_input_dir,
        "new",
    )?;
    let base =
        resolve_history_diff_side_with_request(&mut request_json, &base_source, args.base_version)?;
    let new =
        resolve_history_diff_side_with_request(&mut request_json, &new_source, args.new_version)?;
    let (document, same) = build_dashboard_history_diff_document(&base, &new, args.context_lines)?;
    match args.output_format {
        DiffOutputFormat::Text => {
            println!(
                "{}",
                render_dashboard_history_diff_text(&base, &new, &document)
            )
        }
        DiffOutputFormat::Json => {
            print!(
                "{}",
                render_json_value(&build_shared_diff_document(
                    DASHBOARD_HISTORY_DIFF_KIND,
                    1,
                    document.summary,
                    &document.rows,
                ))?
            )
        }
    }
    Ok(usize::from(!same))
}
