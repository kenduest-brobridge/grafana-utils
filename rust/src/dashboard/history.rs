#![cfg_attr(not(any(feature = "tui", test)), allow(dead_code))]

use crate::common::{
    build_shared_diff_document, message, render_json_value, string_field, tool_version,
    value_as_object, DiffOutputFormat, Result, SharedDiffSummary,
};
use crate::tabular_output::{render_table, render_yaml};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use super::{
    fetch_dashboard_with_request,
    import_compare::{
        build_compare_diff_text_with_labels, build_compare_document, serialize_compare_document,
    },
    import_dashboard_request_with_request, write_json_document, HistoryDiffArgs, HistoryExportArgs,
    HistoryListArgs, HistoryOutputFormat, HistoryRestoreArgs, DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_UID, TOOL_SCHEMA_VERSION,
};

#[allow(dead_code)]
pub(crate) const BROWSE_HISTORY_RESTORE_MESSAGE: &str =
    "Restored by grafana-utils dashboard browse";
pub(crate) const DASHBOARD_HISTORY_RESTORE_MESSAGE: &str =
    "Restored by grafana-util dashboard history";
pub(crate) const DASHBOARD_HISTORY_LIST_KIND: &str = "grafana-util-dashboard-history-list";
pub(crate) const DASHBOARD_HISTORY_RESTORE_KIND: &str = "grafana-util-dashboard-history-restore";
pub(crate) const DASHBOARD_HISTORY_EXPORT_KIND: &str = "grafana-util-dashboard-history-export";
pub(crate) const DASHBOARD_HISTORY_INVENTORY_KIND: &str =
    "grafana-util-dashboard-history-inventory";
pub(crate) const DASHBOARD_HISTORY_DIFF_KIND: &str = "grafana-util-dashboard-history-diff";
const HISTORY_RESTORE_PROMPT_LIMIT: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DashboardHistoryVersion {
    pub version: i64,
    pub created: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DashboardHistoryListDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "dashboardUid")]
    pub dashboard_uid: String,
    #[serde(rename = "versionCount")]
    pub version_count: usize,
    pub versions: Vec<DashboardHistoryVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DashboardHistoryRestoreDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    pub mode: String,
    #[serde(rename = "dashboardUid")]
    pub dashboard_uid: String,
    #[serde(rename = "currentVersion")]
    pub current_version: i64,
    #[serde(rename = "restoreVersion")]
    pub restore_version: i64,
    #[serde(rename = "currentTitle")]
    pub current_title: String,
    #[serde(rename = "restoredTitle")]
    pub restored_title: String,
    #[serde(rename = "targetFolderUid", skip_serializing_if = "Option::is_none")]
    pub target_folder_uid: Option<String>,
    #[serde(rename = "createsNewRevision")]
    pub creates_new_revision: bool,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct DashboardHistoryExportVersion {
    pub version: i64,
    pub created: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    pub message: String,
    pub dashboard: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct DashboardHistoryExportDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "dashboardUid")]
    pub dashboard_uid: String,
    #[serde(rename = "currentVersion")]
    pub current_version: i64,
    #[serde(rename = "currentTitle")]
    pub current_title: String,
    #[serde(rename = "versionCount")]
    pub version_count: usize,
    pub versions: Vec<DashboardHistoryExportVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DashboardHistoryInventoryItem {
    #[serde(rename = "dashboardUid")]
    pub dashboard_uid: String,
    #[serde(rename = "currentTitle")]
    pub current_title: String,
    #[serde(rename = "currentVersion")]
    pub current_version: i64,
    #[serde(rename = "versionCount")]
    pub version_count: usize,
    pub path: String,
    #[serde(rename = "scope", skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DashboardHistoryInventoryDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "artifactCount")]
    pub artifact_count: usize,
    pub artifacts: Vec<DashboardHistoryInventoryItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardHistoryDiffDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    pub summary: SharedDiffSummary,
    pub rows: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DashboardRestorePreview {
    current_version: i64,
    current_title: String,
    restored_title: String,
    target_folder_uid: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct LocalHistoryArtifact {
    path: PathBuf,
    scope: Option<String>,
    document: DashboardHistoryExportDocument,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HistoryDiffSource {
    Live {
        dashboard_uid: String,
    },
    Artifact {
        path: PathBuf,
    },
    ImportDir {
        input_dir: PathBuf,
        dashboard_uid: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct ResolvedHistoryDiffSide {
    source_label: String,
    dashboard_uid: String,
    version: i64,
    title: String,
    dashboard: Value,
    compare_document: Value,
}

pub(crate) fn list_dashboard_history_versions_with_request<F>(
    mut request_json: F,
    uid: &str,
    limit: usize,
) -> Result<Vec<DashboardHistoryVersion>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let path = format!("/api/dashboards/uid/{uid}/versions");
    let params = vec![("limit".to_string(), limit.to_string())];
    let response = request_json(Method::GET, &path, &params, None)?;
    let Some(value) = response else {
        return Ok(Vec::new());
    };
    let versions = match value {
        Value::Array(items) => items,
        Value::Object(object) => object
            .get("versions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => {
            return Err(message(
                "Unexpected dashboard versions payload from Grafana.",
            ))
        }
    };
    Ok(versions
        .into_iter()
        .filter_map(|item| item.as_object().cloned())
        .map(|item| DashboardHistoryVersion {
            version: item
                .get("version")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            created: item
                .get("created")
                .map(display_value)
                .unwrap_or_else(|| "-".to_string()),
            created_by: string_field(&item, "createdBy", "-"),
            message: string_field(&item, "message", ""),
        })
        .collect())
}

pub(crate) fn build_dashboard_history_list_document_with_request<F>(
    mut request_json: F,
    uid: &str,
    limit: usize,
) -> Result<DashboardHistoryListDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let versions = list_dashboard_history_versions_with_request(&mut request_json, uid, limit)?;
    Ok(DashboardHistoryListDocument {
        kind: DASHBOARD_HISTORY_LIST_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        dashboard_uid: uid.to_string(),
        version_count: versions.len(),
        versions,
    })
}

fn fetch_dashboard_history_version_data_with_request<F>(
    mut request_json: F,
    uid: &str,
    version: i64,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let version_path = format!("/api/dashboards/uid/{uid}/versions/{version}");
    let version_payload =
        request_json(Method::GET, &version_path, &[], None)?.ok_or_else(|| {
            message(format!(
                "Dashboard history version {version} was not returned."
            ))
        })?;
    let version_object = value_as_object(
        &version_payload,
        "Unexpected dashboard history version payload from Grafana.",
    )?;
    version_object
        .get("data")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Dashboard history version payload did not include dashboard data."))
}

fn build_dashboard_restore_preview_with_request<F>(
    mut request_json: F,
    uid: &str,
    version: i64,
) -> Result<DashboardRestorePreview>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let current_payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let current_object = value_as_object(
        &current_payload,
        "Unexpected current dashboard payload for history restore.",
    )?;
    let current_dashboard = current_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard data."))?;
    let current_version = current_dashboard
        .get("version")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let current_title = string_field(current_dashboard, "title", DEFAULT_DASHBOARD_TITLE);
    let target_folder_uid = current_object
        .get("meta")
        .and_then(Value::as_object)
        .map(|meta| string_field(meta, "folderUid", DEFAULT_FOLDER_UID))
        .filter(|value| !value.is_empty() && value != DEFAULT_FOLDER_UID);
    let restored_dashboard =
        fetch_dashboard_history_version_data_with_request(&mut request_json, uid, version)?;
    let restored_title = string_field(&restored_dashboard, "title", DEFAULT_DASHBOARD_TITLE);
    Ok(DashboardRestorePreview {
        current_version,
        current_title,
        restored_title,
        target_folder_uid,
    })
}

fn prompt_dashboard_history_restore_version(
    uid: &str,
    versions: &[DashboardHistoryVersion],
) -> Result<Option<i64>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(message("Dashboard history restore --prompt requires a TTY."));
    }
    if versions.is_empty() {
        return Err(message(format!(
            "Dashboard history restore --prompt did not find any versions for {uid}."
        )));
    }
    let labels = versions
        .iter()
        .map(|item| {
            let mut line = format!(
                "v{}  {}  {}",
                item.version, item.created, item.created_by
            );
            if !item.message.is_empty() {
                line.push_str("  ");
                line.push_str(&item.message);
            }
            line
        })
        .collect::<Vec<_>>();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Select a dashboard history version to restore for {uid}"))
        .items(&labels)
        .default(0)
        .interact_opt()
        .map_err(|error| message(format!("Dashboard history restore prompt failed: {error}")))?;
    Ok(selection.and_then(|index| versions.get(index).map(|item| item.version)))
}

fn confirm_dashboard_history_restore(uid: &str, version: i64) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Restore dashboard {uid} to version {version} and create a new latest revision?"
        ))
        .default(false)
        .interact_opt()
        .map(|choice| choice.unwrap_or(false))
        .map_err(|error| message(format!("Dashboard history restore confirmation failed: {error}")))
}

fn build_dashboard_history_restore_document(
    uid: &str,
    version: i64,
    preview: &DashboardRestorePreview,
    message_text: &str,
    dry_run: bool,
) -> DashboardHistoryRestoreDocument {
    DashboardHistoryRestoreDocument {
        kind: DASHBOARD_HISTORY_RESTORE_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        mode: if dry_run { "dry-run" } else { "live" }.to_string(),
        dashboard_uid: uid.to_string(),
        current_version: preview.current_version,
        restore_version: version,
        current_title: preview.current_title.clone(),
        restored_title: preview.restored_title.clone(),
        target_folder_uid: preview.target_folder_uid.clone(),
        creates_new_revision: true,
        message: message_text.to_string(),
    }
}

pub(crate) fn restore_dashboard_history_version_with_request_and_message<F>(
    mut request_json: F,
    uid: &str,
    version: i64,
    message_text: &str,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let current_payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let current_object = value_as_object(
        &current_payload,
        "Unexpected current dashboard payload for history restore.",
    )?;
    let current_folder_uid = current_object
        .get("meta")
        .and_then(Value::as_object)
        .map(|meta| string_field(meta, "folderUid", ""))
        .filter(|value| !value.is_empty());
    let current_dashboard = current_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard data."))?;
    let current_id = current_dashboard
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard id."))?;
    let current_version = current_dashboard
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard version."))?;

    let mut dashboard =
        fetch_dashboard_history_version_data_with_request(&mut request_json, uid, version)?;
    dashboard.insert("id".to_string(), Value::from(current_id));
    dashboard.insert("uid".to_string(), Value::String(uid.to_string()));
    dashboard.insert("version".to_string(), Value::from(current_version));
    if !dashboard.contains_key("title") {
        dashboard.insert(
            "title".to_string(),
            Value::String(DEFAULT_DASHBOARD_TITLE.to_string()),
        );
    }

    let mut import_payload = Map::new();
    import_payload.insert("dashboard".to_string(), Value::Object(dashboard));
    import_payload.insert("overwrite".to_string(), Value::Bool(true));
    import_payload.insert(
        "message".to_string(),
        Value::String(message_text.to_string()),
    );
    if let Some(folder_uid) = current_folder_uid {
        import_payload.insert("folderUid".to_string(), Value::String(folder_uid));
    }
    let _ =
        import_dashboard_request_with_request(&mut request_json, &Value::Object(import_payload))?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn restore_dashboard_history_version_with_request<F>(
    request_json: F,
    uid: &str,
    version: i64,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    restore_dashboard_history_version_with_request_and_message(
        request_json,
        uid,
        version,
        &format!("{BROWSE_HISTORY_RESTORE_MESSAGE} to version {version}"),
    )
}

fn render_dashboard_history_list_text(document: &DashboardHistoryListDocument) -> String {
    let mut lines = vec![format!(
        "Dashboard history: {} versions={}",
        document.dashboard_uid, document.version_count
    )];
    for item in &document.versions {
        let summary = if item.message.is_empty() {
            format!("  v{} {} {}", item.version, item.created, item.created_by)
        } else {
            format!(
                "  v{} {} {} {}",
                item.version, item.created, item.created_by, item.message
            )
        };
        lines.push(summary);
    }
    lines.join("\n")
}

fn render_dashboard_history_list_table(document: &DashboardHistoryListDocument) -> String {
    render_table(
        &["version", "created", "createdBy", "message"],
        &document
            .versions
            .iter()
            .map(|item| {
                vec![
                    item.version.to_string(),
                    item.created.clone(),
                    item.created_by.clone(),
                    item.message.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    )
    .join("\n")
}

fn render_dashboard_history_inventory_text(document: &DashboardHistoryInventoryDocument) -> String {
    let mut lines = vec![format!(
        "Dashboard history artifacts: count={}",
        document.artifact_count
    )];
    for item in &document.artifacts {
        let scope = item.scope.as_deref().unwrap_or("current");
        lines.push(format!(
            "  {} title={} current-version={} versions={} scope={} path={}",
            item.dashboard_uid,
            item.current_title,
            item.current_version,
            item.version_count,
            scope,
            item.path
        ));
    }
    lines.join("\n")
}

fn render_dashboard_history_inventory_table(
    document: &DashboardHistoryInventoryDocument,
) -> String {
    render_table(
        &[
            "dashboardUid",
            "currentTitle",
            "currentVersion",
            "versionCount",
            "scope",
            "path",
        ],
        &document
            .artifacts
            .iter()
            .map(|item| {
                vec![
                    item.dashboard_uid.clone(),
                    item.current_title.clone(),
                    item.current_version.to_string(),
                    item.version_count.to_string(),
                    item.scope.clone().unwrap_or_else(|| "current".to_string()),
                    item.path.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    )
    .join("\n")
}

fn render_dashboard_history_restore_text(document: &DashboardHistoryRestoreDocument) -> String {
    let mut lines = vec![format!(
        "Dashboard history restore: {} current-version={} restore-version={} mode={} creates-new-revision={}",
        document.dashboard_uid,
        document.current_version,
        document.restore_version,
        document.mode,
        document.creates_new_revision
    )];
    lines.push(format!("Current title: {}", document.current_title));
    lines.push(format!("Restored title: {}", document.restored_title));
    if let Some(folder_uid) = &document.target_folder_uid {
        lines.push(format!("Target folder UID: {folder_uid}"));
    }
    lines.push(format!("Message: {}", document.message));
    lines.join("\n")
}

fn render_dashboard_history_restore_table(document: &DashboardHistoryRestoreDocument) -> String {
    let mut rows = vec![
        ("dashboardUid", document.dashboard_uid.clone()),
        ("mode", document.mode.clone()),
        ("currentVersion", document.current_version.to_string()),
        ("restoreVersion", document.restore_version.to_string()),
        ("currentTitle", document.current_title.clone()),
        ("restoredTitle", document.restored_title.clone()),
        (
            "createsNewRevision",
            document.creates_new_revision.to_string(),
        ),
        ("message", document.message.clone()),
    ];
    if let Some(folder_uid) = &document.target_folder_uid {
        rows.push(("targetFolderUid", folder_uid.clone()));
    }
    render_table(
        &["field", "value"],
        &rows
            .into_iter()
            .map(|(field, value)| vec![field.to_string(), value])
            .collect::<Vec<_>>(),
    )
    .join("\n")
}

pub(crate) fn run_dashboard_history_list<F>(
    mut request_json: F,
    args: &HistoryListArgs,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(input_path) = &args.input {
        let document = load_dashboard_history_export_document(input_path)?;
        if let Some(uid) = &args.dashboard_uid {
            ensure_history_artifact_uid_matches(uid, &document, input_path)?;
        }
        let list_document = build_dashboard_history_list_document_from_export_document(&document);
        return render_dashboard_history_list_output(&list_document, args.output_format);
    }

    if let Some(input_dir) = &args.input_dir {
        return run_dashboard_history_list_from_import_dir(input_dir, args);
    }

    let dashboard_uid = args.dashboard_uid.as_deref().ok_or_else(|| {
        message(
            "Dashboard history list requires --dashboard-uid unless --input or --input-dir is set.",
        )
    })?;
    let document = build_dashboard_history_list_document_with_request(
        &mut request_json,
        dashboard_uid,
        args.limit,
    )?;
    render_dashboard_history_list_output(&document, args.output_format)
}

pub(crate) fn run_dashboard_history_restore<F>(
    mut request_json: F,
    args: &HistoryRestoreArgs,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let version = if let Some(version) = args.version {
        version
    } else if args.prompt {
        let versions = list_dashboard_history_versions_with_request(
            &mut request_json,
            &args.dashboard_uid,
            HISTORY_RESTORE_PROMPT_LIMIT,
        )?;
        let Some(version) = prompt_dashboard_history_restore_version(&args.dashboard_uid, &versions)?
        else {
            println!("Cancelled dashboard history restore.");
            return Ok(());
        };
        version
    } else {
        return Err(message(
            "Dashboard history restore requires --version unless --prompt is used.",
        ));
    };
    let preview = build_dashboard_restore_preview_with_request(
        &mut request_json,
        &args.dashboard_uid,
        version,
    )?;
    let message_text = args.message.clone().unwrap_or_else(|| {
        format!("{DASHBOARD_HISTORY_RESTORE_MESSAGE} to version {version}")
    });
    let document = build_dashboard_history_restore_document(
        &args.dashboard_uid,
        version,
        &preview,
        &message_text,
        args.dry_run,
    );
    let rendered = match args.output_format {
        HistoryOutputFormat::Text => render_dashboard_history_restore_text(&document),
        HistoryOutputFormat::Table => render_dashboard_history_restore_table(&document),
        HistoryOutputFormat::Json => render_json_value(&document)?.trim_end().to_string(),
        HistoryOutputFormat::Yaml => render_yaml(&document)?.trim_end().to_string(),
    };
    if args.dry_run {
        println!("{rendered}");
        return Ok(());
    }
    if args.prompt {
        println!("{rendered}");
        if !confirm_dashboard_history_restore(&args.dashboard_uid, version)? {
            println!("Cancelled dashboard history restore.");
            return Ok(());
        }
    } else if !args.yes {
        return Err(message(
            "Dashboard history restore requires --yes unless --dry-run or --prompt is set.",
        ));
    }
    restore_dashboard_history_version_with_request_and_message(
        &mut request_json,
        &args.dashboard_uid,
        version,
        &message_text,
    )?;
    println!("{rendered}");
    Ok(())
}

pub(crate) fn build_dashboard_history_export_document_with_request<F>(
    mut request_json: F,
    uid: &str,
    limit: usize,
) -> Result<DashboardHistoryExportDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let current_payload = fetch_dashboard_with_request(&mut request_json, uid)?;
    let current_object = value_as_object(
        &current_payload,
        "Unexpected current dashboard payload for history export.",
    )?;
    let current_dashboard = current_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Current dashboard payload did not include dashboard data."))?;
    let current_version = current_dashboard
        .get("version")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let current_title = string_field(current_dashboard, "title", DEFAULT_DASHBOARD_TITLE);
    let versions = list_dashboard_history_versions_with_request(&mut request_json, uid, limit)?;
    let versions = versions
        .into_iter()
        .map(|version| {
            let dashboard = Value::Object(fetch_dashboard_history_version_data_with_request(
                &mut request_json,
                uid,
                version.version,
            )?);
            Ok(DashboardHistoryExportVersion {
                version: version.version,
                created: version.created,
                created_by: version.created_by,
                message: version.message,
                dashboard,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(DashboardHistoryExportDocument {
        kind: DASHBOARD_HISTORY_EXPORT_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        dashboard_uid: uid.to_string(),
        current_version,
        current_title,
        version_count: versions.len(),
        versions,
    })
}

pub(crate) fn export_dashboard_history_with_request<F>(
    mut request_json: F,
    args: &HistoryExportArgs,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.output.exists() && !args.overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            args.output.display()
        )));
    }
    let document = build_dashboard_history_export_document_with_request(
        &mut request_json,
        &args.dashboard_uid,
        args.limit,
    )?;
    write_json_document(&document, &args.output)?;
    Ok(())
}

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
) -> Result<&'a DashboardHistoryExportVersion> {
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

fn load_history_artifact_for_uid(
    input_dir: &Path,
    dashboard_uid: &str,
) -> Result<LocalHistoryArtifact> {
    let artifacts = load_history_artifacts_from_import_dir(input_dir)?;
    if artifacts.is_empty() {
        return Err(message(format!(
            "No dashboard history artifacts found under {}. Export with `dashboard export --include-history` first.",
            input_dir.display()
        )));
    }
    let matching = artifacts
        .into_iter()
        .filter(|artifact| artifact.document.dashboard_uid == dashboard_uid)
        .collect::<Vec<_>>();
    match matching.len() {
        0 => Err(message(format!(
            "No dashboard history artifact for UID {} found under {}.",
            dashboard_uid,
            input_dir.display()
        ))),
        1 => Ok(matching.into_iter().next().expect("single artifact")),
        _ => {
            let scopes = matching
                .iter()
                .map(|artifact| {
                    artifact
                        .scope
                        .clone()
                        .unwrap_or_else(|| artifact.path.display().to_string())
                })
                .collect::<Vec<_>>()
                .join(", ");
            Err(message(format!(
                "Multiple dashboard history artifacts for UID {} found under {}: {}. Narrow the export root or inspect one artifact with --input.",
                dashboard_uid,
                input_dir.display(),
                scopes
            )))
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
    let rows = vec![serde_json::json!({
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
            schema_version: 1,
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

fn render_dashboard_history_diff_text(
    base: &ResolvedHistoryDiffSide,
    new: &ResolvedHistoryDiffSide,
    document: &DashboardHistoryDiffDocument,
) -> String {
    let row = &document.rows[0];
    let status = row
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("different");
    let mut lines = vec![format!(
        "Dashboard history diff: {} status={} base-version={} new-version={}",
        history_diff_identity(&base.dashboard_uid, &new.dashboard_uid),
        status,
        base.version,
        new.version
    )];
    lines.push(format!("Base source: {}", base.source_label));
    lines.push(format!("New source: {}", new.source_label));
    if let Some(path) = row.get("path").and_then(Value::as_str) {
        lines.push(format!("Path: {path}"));
    }
    lines.push(format!("Base title: {}", base.title));
    lines.push(format!("New title: {}", new.title));
    if let Some(diff_text) = row.get("diffText").and_then(Value::as_str) {
        lines.push(String::new());
        lines.push(diff_text.trim_end().to_string());
    }
    lines.join("\n")
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

fn display_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

fn render_dashboard_history_list_output(
    document: &DashboardHistoryListDocument,
    output_format: HistoryOutputFormat,
) -> Result<()> {
    let rendered = match output_format {
        HistoryOutputFormat::Text => render_dashboard_history_list_text(document),
        HistoryOutputFormat::Table => render_dashboard_history_list_table(document),
        HistoryOutputFormat::Json => render_json_value(document)?.trim_end().to_string(),
        HistoryOutputFormat::Yaml => render_yaml(document)?.trim_end().to_string(),
    };
    println!("{rendered}");
    Ok(())
}

fn render_dashboard_history_inventory_output(
    document: &DashboardHistoryInventoryDocument,
    output_format: HistoryOutputFormat,
) -> Result<()> {
    let rendered = match output_format {
        HistoryOutputFormat::Text => render_dashboard_history_inventory_text(document),
        HistoryOutputFormat::Table => render_dashboard_history_inventory_table(document),
        HistoryOutputFormat::Json => render_json_value(document)?.trim_end().to_string(),
        HistoryOutputFormat::Yaml => render_yaml(document)?.trim_end().to_string(),
    };
    println!("{rendered}");
    Ok(())
}

fn load_dashboard_history_export_document(path: &Path) -> Result<DashboardHistoryExportDocument> {
    let raw = fs::read_to_string(path)?;
    let document: DashboardHistoryExportDocument = serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Failed to parse dashboard history artifact {}: {error}",
            path.display()
        ))
    })?;
    if document.kind != DASHBOARD_HISTORY_EXPORT_KIND {
        return Err(message(format!(
            "Expected {} at {}, found {}.",
            DASHBOARD_HISTORY_EXPORT_KIND,
            path.display(),
            document.kind
        )));
    }
    Ok(document)
}

fn ensure_history_artifact_uid_matches(
    expected_uid: &str,
    document: &DashboardHistoryExportDocument,
    path: &Path,
) -> Result<()> {
    if document.dashboard_uid != expected_uid {
        return Err(message(format!(
            "History artifact {} contains dashboard UID {} instead of {}.",
            path.display(),
            document.dashboard_uid,
            expected_uid
        )));
    }
    Ok(())
}

fn build_dashboard_history_list_document_from_export_document(
    document: &DashboardHistoryExportDocument,
) -> DashboardHistoryListDocument {
    DashboardHistoryListDocument {
        kind: DASHBOARD_HISTORY_LIST_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        dashboard_uid: document.dashboard_uid.clone(),
        version_count: document.version_count,
        versions: document
            .versions
            .iter()
            .map(|item| DashboardHistoryVersion {
                version: item.version,
                created: item.created.clone(),
                created_by: item.created_by.clone(),
                message: item.message.clone(),
            })
            .collect(),
    }
}

fn collect_history_artifact_paths(root: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_history_artifact_paths(&path, output)?;
            continue;
        }
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".history.json"))
        {
            output.push(path);
        }
    }
    Ok(())
}

fn derive_history_artifact_scope(input_dir: &Path, artifact_path: &Path) -> Option<String> {
    let relative = artifact_path.strip_prefix(input_dir).ok()?;
    let mut scope_parts = Vec::new();
    for component in relative.components() {
        let piece = component.as_os_str().to_string_lossy().to_string();
        if piece == "history" {
            break;
        }
        scope_parts.push(piece);
    }
    if scope_parts.is_empty() {
        None
    } else {
        Some(scope_parts.join("/"))
    }
}

fn load_history_artifacts_from_import_dir(input_dir: &Path) -> Result<Vec<LocalHistoryArtifact>> {
    let mut paths = Vec::new();
    collect_history_artifact_paths(input_dir, &mut paths)?;
    let mut artifacts = Vec::new();
    for path in paths {
        let document = load_dashboard_history_export_document(&path)?;
        artifacts.push(LocalHistoryArtifact {
            scope: derive_history_artifact_scope(input_dir, &path),
            path,
            document,
        });
    }
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(artifacts)
}

fn build_dashboard_history_inventory_document(
    input_dir: &Path,
    artifacts: &[LocalHistoryArtifact],
) -> DashboardHistoryInventoryDocument {
    DashboardHistoryInventoryDocument {
        kind: DASHBOARD_HISTORY_INVENTORY_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        artifact_count: artifacts.len(),
        artifacts: artifacts
            .iter()
            .map(|artifact| DashboardHistoryInventoryItem {
                dashboard_uid: artifact.document.dashboard_uid.clone(),
                current_title: artifact.document.current_title.clone(),
                current_version: artifact.document.current_version,
                version_count: artifact.document.version_count,
                path: artifact
                    .path
                    .strip_prefix(input_dir)
                    .unwrap_or(&artifact.path)
                    .display()
                    .to_string(),
                scope: artifact.scope.clone(),
            })
            .collect(),
    }
}

fn run_dashboard_history_list_from_import_dir(
    input_dir: &Path,
    args: &HistoryListArgs,
) -> Result<()> {
    let artifacts = load_history_artifacts_from_import_dir(input_dir)?;
    if artifacts.is_empty() {
        return Err(message(format!(
            "No dashboard history artifacts found under {}. Export with `dashboard export --include-history` first.",
            input_dir.display()
        )));
    }
    if let Some(uid) = &args.dashboard_uid {
        let matching = artifacts
            .iter()
            .filter(|artifact| artifact.document.dashboard_uid == *uid)
            .collect::<Vec<_>>();
        match matching.len() {
            0 => {
                return Err(message(format!(
                    "No dashboard history artifact for UID {} found under {}.",
                    uid,
                    input_dir.display()
                )))
            }
            1 => {
                let document = build_dashboard_history_list_document_from_export_document(
                    &matching[0].document,
                );
                return render_dashboard_history_list_output(&document, args.output_format);
            }
            _ => {
                let scopes = matching
                    .iter()
                    .map(|artifact| {
                        artifact
                            .scope
                            .clone()
                            .unwrap_or_else(|| artifact.path.display().to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(message(format!(
                    "Multiple dashboard history artifacts for UID {} found under {}: {}. Narrow the export root or inspect one artifact with --input.",
                    uid,
                    input_dir.display(),
                    scopes
                )));
            }
        }
    }
    let document = build_dashboard_history_inventory_document(input_dir, &artifacts);
    render_dashboard_history_inventory_output(&document, args.output_format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_dashboard_history_version_keeps_live_dashboard_id_and_version() {
        let payloads = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
        let recorded = payloads.clone();

        restore_dashboard_history_version_with_request(
            move |method, path, _params, payload| match (method, path) {
                (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(serde_json::json!({
                    "dashboard": {
                        "id": 42,
                        "uid": "cpu-main",
                        "title": "CPU Main",
                        "version": 7
                    },
                    "meta": {
                        "folderUid": "infra"
                    }
                }))),
                (Method::GET, "/api/dashboards/uid/cpu-main/versions/5") => {
                    Ok(Some(serde_json::json!({
                        "version": 5,
                        "data": {
                            "id": 42,
                            "version": 5,
                            "uid": "cpu-main",
                            "title": "CPU Old"
                        }
                    })))
                }
                (Method::POST, "/api/dashboards/db") => {
                    recorded
                        .lock()
                        .unwrap()
                        .push(payload.cloned().unwrap_or(Value::Null));
                    Ok(Some(serde_json::json!({"status": "success"})))
                }
                _ => Err(message("unexpected request")),
            },
            "cpu-main",
            5,
        )
        .unwrap();

        let payloads = payloads.lock().unwrap();
        assert_eq!(payloads.len(), 1);
        let payload = payloads[0].as_object().unwrap();
        assert_eq!(payload["overwrite"], serde_json::json!(true));
        assert_eq!(payload["folderUid"], serde_json::json!("infra"));
        assert_eq!(payload["dashboard"]["uid"], serde_json::json!("cpu-main"));
        assert_eq!(payload["dashboard"]["id"], serde_json::json!(42));
        assert_eq!(payload["dashboard"]["version"], serde_json::json!(7));
        assert_eq!(payload["dashboard"]["title"], serde_json::json!("CPU Old"));
    }
}
