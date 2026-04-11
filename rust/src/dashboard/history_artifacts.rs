use crate::common::{message, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::history_render::{
    render_dashboard_history_inventory_output, render_dashboard_history_list_output,
};
use super::history_types::{
    DashboardHistoryExportDocument, DashboardHistoryInventoryDocument,
    DashboardHistoryInventoryItem, DashboardHistoryListDocument, LocalHistoryArtifact,
};
use super::{tool_version, HistoryListArgs, TOOL_SCHEMA_VERSION};

pub(crate) fn load_dashboard_history_export_document(
    path: &Path,
) -> Result<DashboardHistoryExportDocument> {
    let raw = fs::read_to_string(path)?;
    let document: DashboardHistoryExportDocument = serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Failed to parse dashboard history artifact {}: {error}",
            path.display()
        ))
    })?;
    if document.kind != super::history_types::DASHBOARD_HISTORY_EXPORT_KIND {
        return Err(message(format!(
            "Expected {} at {}, found {}.",
            super::history_types::DASHBOARD_HISTORY_EXPORT_KIND,
            path.display(),
            document.kind
        )));
    }
    Ok(document)
}

pub(crate) fn ensure_history_artifact_uid_matches(
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

pub(crate) fn build_dashboard_history_list_document_from_export_document(
    document: &DashboardHistoryExportDocument,
) -> DashboardHistoryListDocument {
    DashboardHistoryListDocument {
        kind: super::history_types::DASHBOARD_HISTORY_LIST_KIND.to_string(),
        schema_version: TOOL_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        dashboard_uid: document.dashboard_uid.clone(),
        version_count: document.version_count,
        versions: document
            .versions
            .iter()
            .map(|item| super::history_types::DashboardHistoryVersion {
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

pub(crate) fn load_history_artifacts_from_import_dir(
    input_dir: &Path,
) -> Result<Vec<LocalHistoryArtifact>> {
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

pub(crate) fn build_dashboard_history_inventory_document(
    input_dir: &Path,
    artifacts: &[LocalHistoryArtifact],
) -> DashboardHistoryInventoryDocument {
    DashboardHistoryInventoryDocument {
        kind: super::history_types::DASHBOARD_HISTORY_INVENTORY_KIND.to_string(),
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

pub(crate) fn load_history_artifact_for_uid(
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

pub(crate) fn run_dashboard_history_list_from_import_dir(
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
