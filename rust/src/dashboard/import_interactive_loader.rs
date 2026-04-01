#![cfg(feature = "tui")]

use std::collections::BTreeMap;

use crate::common::{string_field, value_as_object, Result};

use super::import_interactive::{InteractiveImportItem, InteractiveImportReviewState};
use super::import_lookup::resolve_source_dashboard_folder_path;

#[cfg(test)]
pub(crate) fn load_interactive_import_items(
    args: &super::ImportArgs,
) -> Result<Vec<InteractiveImportItem>> {
    Ok(load_interactive_import_context(args)?.0)
}

pub(crate) fn load_interactive_import_context(
    args: &super::ImportArgs,
) -> Result<(
    Vec<InteractiveImportItem>,
    BTreeMap<String, super::FolderInventoryItem>,
)> {
    let resolved_import = super::import::resolve_import_source(args)?;
    let metadata = super::load_export_metadata(
        &resolved_import.metadata_dir,
        Some(super::import::import_metadata_variant(args)),
    )?;
    let folder_inventory =
        super::load_folder_inventory(&resolved_import.metadata_dir, metadata.as_ref())?;
    let folders_by_uid: BTreeMap<String, super::FolderInventoryItem> = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect();
    let mut items = Vec::new();
    for path in super::import::dashboard_files_for_import(&resolved_import.dashboard_dir)? {
        let document = super::load_json_file(&path)?;
        let document_object =
            value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = super::extract_dashboard_object(document_object)?;
        let uid = string_field(dashboard, "uid", super::DEFAULT_UNKNOWN_UID).to_string();
        let title = string_field(dashboard, "title", super::DEFAULT_DASHBOARD_TITLE).to_string();
        let folder_path = resolve_source_dashboard_folder_path(
            &document,
            &path,
            &resolved_import.metadata_dir,
            &folders_by_uid,
        )
        .unwrap_or_default();
        let file_label = path
            .strip_prefix(&resolved_import.dashboard_dir)
            .unwrap_or(&path)
            .display()
            .to_string();
        items.push(InteractiveImportItem {
            path,
            uid,
            title,
            folder_path,
            file_label,
            review: InteractiveImportReviewState::Pending,
        });
    }
    items.sort_by(|left, right| {
        (
            left.folder_path.as_str(),
            left.title.as_str(),
            left.uid.as_str(),
        )
            .cmp(&(
                right.folder_path.as_str(),
                right.title.as_str(),
                right.uid.as_str(),
            ))
    });
    Ok((items, folders_by_uid))
}
