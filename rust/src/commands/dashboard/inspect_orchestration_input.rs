use std::path::{Path, PathBuf};

use crate::common::{message, Result};

use super::super::super::cli_defs::{DashboardImportInputFormat, InspectExportInputType};
use super::super::super::files::{
    load_export_metadata, resolve_dashboard_export_root, DashboardSourceKind,
};
use super::super::super::inspect_live::TempInspectDir;
use super::super::super::source_loader::{
    load_dashboard_source, resolve_dashboard_workspace_variant_dir,
};
use super::super::super::{PROMPT_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR};

pub(crate) struct ResolvedInspectExportInput {
    pub(crate) input_dir: PathBuf,
    pub(crate) expected_variant: &'static str,
    pub(crate) source_kind: Option<DashboardSourceKind>,
    _temp_dir: Option<TempInspectDir>,
}

pub(super) fn resolve_inspect_export_import_dir_with_prompt<F>(
    temp_root: &Path,
    input_dir: &Path,
    input_format: DashboardImportInputFormat,
    input_type: Option<InspectExportInputType>,
    interactive: bool,
    choose_input_type: F,
) -> Result<ResolvedInspectExportInput>
where
    F: FnMut(&Path) -> Result<InspectExportInputType>,
{
    match input_format {
        DashboardImportInputFormat::Raw => resolve_raw_inspect_input(
            temp_root,
            input_dir,
            input_type,
            interactive,
            choose_input_type,
        ),
        DashboardImportInputFormat::Provisioning => {
            let resolved = load_dashboard_source(
                input_dir,
                DashboardImportInputFormat::Provisioning,
                None,
                false,
            )?;
            Ok(ResolvedInspectExportInput {
                input_dir: resolved.input_dir,
                expected_variant: RAW_EXPORT_SUBDIR,
                source_kind: Some(DashboardSourceKind::ProvisioningExport),
                _temp_dir: resolved.temp_dir,
            })
        }
    }
}

fn discover_org_variant_export_dirs(
    input_dir: &Path,
    variant_dir_name: &'static str,
) -> Result<Vec<PathBuf>> {
    let mut org_variant_dirs = Vec::new();
    if !input_dir.is_dir() {
        return Ok(org_variant_dirs);
    }
    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        let org_root = entry.path();
        if !org_root.is_dir() {
            continue;
        }
        let org_name = entry.file_name().to_string_lossy().to_string();
        if !org_name.starts_with("org_") {
            continue;
        }
        let variant_dir = org_root.join(variant_dir_name);
        if variant_dir.is_dir() {
            org_variant_dirs.push(variant_dir);
        }
    }
    org_variant_dirs.sort();
    Ok(org_variant_dirs)
}

fn resolve_raw_inspect_input<F>(
    _temp_root: &Path,
    input_dir: &Path,
    input_type: Option<InspectExportInputType>,
    _interactive: bool,
    mut choose_input_type: F,
) -> Result<ResolvedInspectExportInput>
where
    F: FnMut(&Path) -> Result<InspectExportInputType>,
{
    let input_dir = resolve_dashboard_workspace_import_dir(input_dir)?;
    let metadata = load_export_metadata(&input_dir, None)?;
    let raw_dirs = discover_org_variant_export_dirs(&input_dir, RAW_EXPORT_SUBDIR)?;
    let source_dirs = discover_org_variant_export_dirs(&input_dir, PROMPT_EXPORT_SUBDIR)?;
    let raw_workspace_variant =
        resolve_dashboard_workspace_variant_dir(&input_dir, RAW_EXPORT_SUBDIR);
    let source_workspace_variant =
        resolve_dashboard_workspace_variant_dir(&input_dir, PROMPT_EXPORT_SUBDIR);
    let selected_input_type = match input_type {
        Some(input_type) => input_type,
        None if (!raw_dirs.is_empty() && !source_dirs.is_empty())
            || (raw_workspace_variant.is_some() && source_workspace_variant.is_some()) =>
        {
            choose_input_type(&input_dir)?
        }
        None if matches!(
            metadata.as_ref().map(|item| item.variant.as_str()),
            Some(PROMPT_EXPORT_SUBDIR)
        ) =>
        {
            InspectExportInputType::Source
        }
        None => InspectExportInputType::Raw,
    };

    let resolved = load_dashboard_source(
        &input_dir,
        DashboardImportInputFormat::Raw,
        Some(selected_input_type),
        false,
    )?;

    Ok(ResolvedInspectExportInput {
        input_dir: resolved.input_dir,
        expected_variant: resolved.expected_variant,
        source_kind: DashboardSourceKind::from_expected_variant(resolved.expected_variant),
        _temp_dir: resolved.temp_dir,
    })
}

fn resolve_dashboard_workspace_import_dir(input_dir: &Path) -> Result<PathBuf> {
    if let Some(resolved_root) = resolve_dashboard_export_root(input_dir)? {
        return Ok(resolved_root.metadata_dir);
    }

    let dashboard_dir = input_dir.join("dashboards");
    if dashboard_dir.is_dir() && input_dir.join("datasources").is_dir() {
        return Err(message(format!(
            "Import path {} looks like a workspace export root containing dashboards/ and datasources/, but dashboards/export-metadata.json is missing. Point --input-dir at {} or at a dashboard variant directory such as {}/.../{}.",
            input_dir.display(),
            dashboard_dir.display(),
            dashboard_dir.display(),
            RAW_EXPORT_SUBDIR
        )));
    }
    Ok(input_dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::{resolve_inspect_export_import_dir_with_prompt, InspectExportInputType};
    use crate::dashboard::{DashboardImportInputFormat, DashboardSourceKind};
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolve_inspect_export_import_dir_marks_provisioning_source_kind() {
        let temp = tempdir().unwrap();
        let input_dir = temp.path().join("provisioning");
        let dashboards_dir = input_dir.join("dashboards");
        fs::create_dir_all(&dashboards_dir).unwrap();
        fs::write(
            input_dir.join("export-metadata.json"),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": 1,
                "variant": "provisioning",
                "dashboardCount": 0,
                "indexFile": "index.json",
                "org": "Main Org.",
                "orgId": "1"
            }))
            .unwrap(),
        )
        .unwrap();

        let resolved = resolve_inspect_export_import_dir_with_prompt(
            temp.path(),
            &input_dir,
            DashboardImportInputFormat::Provisioning,
            None,
            false,
            |_| panic!("provisioning input should not prompt"),
        )
        .unwrap();

        assert_eq!(resolved.expected_variant, super::RAW_EXPORT_SUBDIR);
        assert_eq!(
            resolved.source_kind,
            Some(DashboardSourceKind::ProvisioningExport)
        );
    }

    #[test]
    fn resolve_inspect_export_import_dir_accepts_git_sync_wrapped_raw_tree() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::write(
            workspace.join("export-metadata.json"),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": 1,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "org": "Main Org.",
                "orgId": "1"
            }))
            .unwrap(),
        )
        .unwrap();
        let raw_root = workspace.join("dashboards/git-sync/raw");
        std::fs::create_dir_all(raw_root.join("org_1/raw")).unwrap();
        std::fs::write(
            raw_root.join("export-metadata.json"),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": 1,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "org": "Main Org.",
                "orgId": "1"
            }))
            .unwrap(),
        )
        .unwrap();

        let resolved = resolve_inspect_export_import_dir_with_prompt(
            workspace,
            workspace,
            DashboardImportInputFormat::Raw,
            None,
            false,
            |_| panic!("raw-only input should not prompt"),
        )
        .unwrap();

        assert_eq!(resolved.expected_variant, super::RAW_EXPORT_SUBDIR);
        assert_eq!(resolved.source_kind, Some(DashboardSourceKind::RawExport));
    }

    #[test]
    fn resolve_inspect_export_import_dir_accepts_git_sync_repo_root_without_export_metadata() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join(".git")).unwrap();
        let raw_root = workspace.join("dashboards/git-sync/raw");
        std::fs::create_dir_all(raw_root.join("org_1/raw")).unwrap();
        std::fs::write(
            raw_root.join("export-metadata.json"),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": 1,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "org": "Main Org.",
                "orgId": "1"
            }))
            .unwrap(),
        )
        .unwrap();

        let resolved = resolve_inspect_export_import_dir_with_prompt(
            workspace,
            workspace,
            DashboardImportInputFormat::Raw,
            None,
            false,
            |_| panic!("raw-only input should not prompt"),
        )
        .unwrap();

        assert_eq!(resolved.expected_variant, super::RAW_EXPORT_SUBDIR);
        assert_eq!(resolved.source_kind, Some(DashboardSourceKind::RawExport));
    }

    #[test]
    fn resolve_inspect_export_import_dir_respects_explicit_source_input_type() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join(".git")).unwrap();
        std::fs::create_dir_all(workspace.join("dashboards/raw")).unwrap();
        let prompt_root = workspace.join("dashboards/prompt");
        std::fs::create_dir_all(&prompt_root).unwrap();

        let resolved = resolve_inspect_export_import_dir_with_prompt(
            workspace,
            workspace,
            DashboardImportInputFormat::Raw,
            Some(InspectExportInputType::Source),
            false,
            |_| panic!("explicit input type should not prompt"),
        )
        .unwrap();

        assert_eq!(resolved.expected_variant, super::PROMPT_EXPORT_SUBDIR);
        assert_eq!(resolved.source_kind, None);
        assert_eq!(resolved.input_dir, prompt_root);
    }
}
