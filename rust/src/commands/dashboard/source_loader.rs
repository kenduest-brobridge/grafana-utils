// Thin dashboard source-loader facade for local/file-backed source resolution.
//
// This keeps source resolution centralized without introducing a heavy adapter
// hierarchy. The goal is to give analysis/inspect callers one pragmatic entry
// point for raw, provisioning, and Git Sync workspace roots, while leaving
// room for later live/history/prompt-backed sources.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::common::{message, Result};

use super::cli_defs::{DashboardImportInputFormat, InspectExportInputType};
use super::files::{
    resolve_dashboard_export_root, resolve_dashboard_import_source, DashboardSourceKind,
    ResolvedDashboardImportSource,
};
use super::inspect_live::{prepare_inspect_export_import_dir_for_variant, TempInspectDir};
use super::{PROMPT_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR};

const PROVISIONING_EXPORT_SUBDIR: &str = "provisioning";

/// Request shape for resolving a local dashboard source.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DashboardSourceRequest<'a> {
    pub(crate) input_dir: &'a Path,
    pub(crate) input_format: DashboardImportInputFormat,
    pub(crate) input_type: Option<InspectExportInputType>,
    pub(crate) strict_workspace: bool,
}

impl<'a> DashboardSourceRequest<'a> {
    pub(crate) fn new(
        input_dir: &'a Path,
        input_format: DashboardImportInputFormat,
        input_type: Option<InspectExportInputType>,
        strict_workspace: bool,
    ) -> Self {
        Self {
            input_dir,
            input_format,
            input_type,
            strict_workspace,
        }
    }
}

/// Resolved local dashboard source plus the inferred workspace root.
///
/// `input_dir` is the normalized local dashboard directory that downstream
/// analysis/import callers should operate on. `workspace_root` is the broader
/// repo/workspace root inferred from that directory when possible.
pub(crate) struct LoadedDashboardSource {
    pub(crate) workspace_root: PathBuf,
    pub(crate) input_dir: PathBuf,
    pub(crate) expected_variant: &'static str,
    pub(crate) resolved: ResolvedDashboardImportSource,
    pub(crate) temp_dir: Option<TempInspectDir>,
}

impl fmt::Debug for LoadedDashboardSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoadedDashboardSource")
            .field("workspace_root", &self.workspace_root)
            .field("input_dir", &self.input_dir)
            .field("expected_variant", &self.expected_variant)
            .field("resolved", &self.resolved)
            .finish()
    }
}

/// Resolve a dashboard workspace root from a local path.
pub(crate) fn infer_dashboard_workspace_root(input_dir: &Path) -> PathBuf {
    if let Some(workspace_root) = infer_workspace_root_from_layout_ancestors(input_dir) {
        return workspace_root;
    }
    infer_dashboard_workspace_root_fallback(input_dir)
}

fn infer_workspace_root_from_layout_ancestors(input_dir: &Path) -> Option<PathBuf> {
    if input_dir.is_file()
        && input_dir.file_name().and_then(|name| name.to_str()) == Some("datasources.yaml")
    {
        let parent = input_dir.parent();
        let grandparent = parent.and_then(Path::parent);
        let great_grandparent = grandparent.and_then(Path::parent);
        if parent.and_then(Path::file_name).and_then(|v| v.to_str()) == Some("provisioning")
            && grandparent
                .and_then(Path::file_name)
                .and_then(|v| v.to_str())
                == Some("datasources")
        {
            return Some(great_grandparent.unwrap_or(input_dir).to_path_buf());
        }
    }

    for ancestor in input_dir.ancestors() {
        let Some(name) = ancestor.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        match name {
            "dashboards" | "alerts" | "datasources" => {
                if ancestor
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|v| v.to_str())
                    == Some("provisioning")
                {
                    continue;
                }
                return Some(ancestor.parent().unwrap_or(ancestor).to_path_buf());
            }
            "git-sync" => {
                let parent = ancestor.parent();
                let grandparent = parent.and_then(Path::parent);
                if parent.and_then(Path::file_name).and_then(|v| v.to_str()) == Some("dashboards") {
                    return Some(grandparent.unwrap_or(ancestor).to_path_buf());
                }
            }
            _ => {}
        }
    }

    None
}

fn infer_dashboard_workspace_root_fallback(input_dir: &Path) -> PathBuf {
    let Some(name) = input_dir.file_name().and_then(|name| name.to_str()) else {
        return input_dir.to_path_buf();
    };
    if input_dir.is_file() {
        if name == "datasources.yaml" {
            let parent = input_dir.parent();
            let grandparent = parent.and_then(Path::parent);
            let great_grandparent = grandparent.and_then(Path::parent);
            if parent.and_then(Path::file_name).and_then(|v| v.to_str()) == Some("provisioning")
                && grandparent
                    .and_then(Path::file_name)
                    .and_then(|v| v.to_str())
                    == Some("datasources")
            {
                return great_grandparent.unwrap_or(input_dir).to_path_buf();
            }
        }
        return input_dir.parent().unwrap_or(input_dir).to_path_buf();
    }
    let parent = input_dir.parent();
    let grandparent = parent.and_then(Path::parent);
    let great_grandparent = grandparent.and_then(Path::parent);
    match name {
        "dashboards" | "alerts" | "datasources" => parent.unwrap_or(input_dir).to_path_buf(),
        "git-sync" => {
            if parent.and_then(Path::file_name).and_then(|v| v.to_str()) == Some("dashboards") {
                grandparent.unwrap_or(input_dir).to_path_buf()
            } else {
                input_dir.to_path_buf()
            }
        }
        "raw" | "provisioning" => {
            if parent.and_then(Path::file_name).and_then(|v| v.to_str()) == Some("git-sync")
                && grandparent
                    .and_then(Path::file_name)
                    .and_then(|v| v.to_str())
                    == Some("dashboards")
            {
                great_grandparent.unwrap_or(input_dir).to_path_buf()
            } else {
                grandparent.unwrap_or(input_dir).to_path_buf()
            }
        }
        _ if matches!(
            parent.and_then(Path::file_name).and_then(|v| v.to_str()),
            Some("git-sync")
        ) && matches!(
            grandparent
                .and_then(Path::file_name)
                .and_then(|v| v.to_str()),
            Some("dashboards")
        ) =>
        {
            great_grandparent.unwrap_or(input_dir).to_path_buf()
        }
        _ => input_dir.to_path_buf(),
    }
}

fn path_from_components(components: &[Component<'_>]) -> PathBuf {
    let mut path = PathBuf::new();
    for component in components {
        path.push(component.as_os_str());
    }
    path
}

fn canonical_dashboard_variant_root(input_dir: &Path, variant_dir_name: &str) -> Option<PathBuf> {
    let components: Vec<Component<'_>> = input_dir.components().collect();
    let dashboards_index = components
        .iter()
        .position(|component| component.as_os_str() == "dashboards")?;
    if dashboards_index + 2 < components.len()
        && components[dashboards_index + 1].as_os_str() == "git-sync"
        && components[dashboards_index + 2].as_os_str() == variant_dir_name
    {
        return Some(path_from_components(&components[..=dashboards_index + 2]));
    }
    if dashboards_index + 1 < components.len()
        && components[dashboards_index + 1].as_os_str() == variant_dir_name
    {
        return Some(path_from_components(&components[..=dashboards_index + 1]));
    }
    None
}

/// Resolve a dashboard variant root from a workspace, dashboards root, or repo root.
pub(crate) fn resolve_dashboard_workspace_variant_dir(
    input_dir: &Path,
    variant_dir_name: &str,
) -> Option<PathBuf> {
    if let Some(canonical_root) = canonical_dashboard_variant_root(input_dir, variant_dir_name) {
        return Some(canonical_root);
    }
    if input_dir.file_name().and_then(|name| name.to_str()) == Some(variant_dir_name)
        && input_dir.is_dir()
    {
        return Some(input_dir.to_path_buf());
    }

    let direct_candidate = input_dir.join(variant_dir_name);
    if direct_candidate.is_dir() {
        return Some(direct_candidate);
    }

    let dashboards_dir =
        if input_dir.file_name().and_then(|name| name.to_str()) == Some("dashboards") {
            input_dir.to_path_buf()
        } else {
            input_dir.join("dashboards")
        };
    let direct_dashboards_candidate = dashboards_dir.join(variant_dir_name);
    if direct_dashboards_candidate.is_dir() {
        return Some(direct_dashboards_candidate);
    }

    let git_sync_dir = if input_dir.file_name().and_then(|name| name.to_str()) == Some("git-sync") {
        input_dir.to_path_buf()
    } else {
        dashboards_dir.join("git-sync")
    };
    let wrapped_candidate = git_sync_dir.join(variant_dir_name);
    wrapped_candidate.is_dir().then_some(wrapped_candidate)
}

fn select_expected_variant(
    input_format: DashboardImportInputFormat,
    input_type: Option<InspectExportInputType>,
) -> &'static str {
    match input_type {
        Some(InspectExportInputType::Raw) => RAW_EXPORT_SUBDIR,
        Some(InspectExportInputType::Source) => PROMPT_EXPORT_SUBDIR,
        None => match input_format {
            DashboardImportInputFormat::Raw => RAW_EXPORT_SUBDIR,
            DashboardImportInputFormat::Provisioning => PROVISIONING_EXPORT_SUBDIR,
        },
    }
}

fn resolve_root_export_source(
    input_dir: &Path,
    expected_variant: &'static str,
    source_kind: DashboardSourceKind,
) -> Result<LoadedDashboardSource> {
    let temp_dir = TempInspectDir::new("dashboard-source-loader")?;
    let dashboard_dir =
        prepare_inspect_export_import_dir_for_variant(&temp_dir.path, input_dir, expected_variant)?;
    let resolved = ResolvedDashboardImportSource {
        source_kind,
        dashboard_dir: dashboard_dir.clone(),
        metadata_dir: dashboard_dir.clone(),
    };
    Ok(LoadedDashboardSource {
        workspace_root: infer_dashboard_workspace_root(input_dir),
        input_dir: dashboard_dir.clone(),
        expected_variant,
        resolved,
        temp_dir: Some(temp_dir),
    })
}

fn resolve_worktree_source(
    input_dir: &Path,
    input_format: DashboardImportInputFormat,
    input_type: Option<InspectExportInputType>,
    strict_workspace: bool,
) -> Result<LoadedDashboardSource> {
    let workspace_root = infer_dashboard_workspace_root(input_dir);
    let expected_variant = select_expected_variant(input_format, input_type);
    let source_kind = DashboardSourceKind::from_expected_variant(expected_variant)
        .unwrap_or_else(|| DashboardSourceKind::from_import_input_format(input_format));

    if resolve_dashboard_export_root(input_dir)?
        .map(|resolved| resolved.manifest.scope_kind.is_root())
        .unwrap_or(false)
    {
        return resolve_root_export_source(input_dir, expected_variant, source_kind);
    }

    if let Some(workspace_dir) =
        resolve_dashboard_workspace_variant_dir(input_dir, expected_variant)
    {
        let resolved = resolve_dashboard_import_source(&workspace_dir, input_format)?;
        let input_dir = resolved.dashboard_dir.clone();
        return Ok(LoadedDashboardSource {
            workspace_root,
            input_dir,
            expected_variant,
            resolved,
            temp_dir: None,
        });
    }

    if strict_workspace {
        return Err(message(format!(
            "Workspace path {} does not contain a browsable {expected_variant} dashboard tree. Point --workspace at a repo root, workspace root, dashboards/ root, or export directory.",
            input_dir.display()
        )));
    }

    let resolved = resolve_dashboard_import_source(input_dir, input_format)?;
    let input_dir = resolved.dashboard_dir.clone();
    Ok(LoadedDashboardSource {
        workspace_root,
        input_dir,
        expected_variant,
        resolved,
        temp_dir: None,
    })
}

/// Resolve a local dashboard source without forcing callers to know the source layout.
///
/// This is the main facade intended for analysis/inspect callers. It accepts a
/// repo root, dashboards root, raw/provisioning export dir, or Git Sync wrapped
/// dashboard tree and returns the normalized local dashboard directory together
/// with the inferred workspace root.
pub(crate) fn load_dashboard_source(
    input_dir: &Path,
    input_format: DashboardImportInputFormat,
    input_type: Option<InspectExportInputType>,
    strict_workspace: bool,
) -> Result<LoadedDashboardSource> {
    load_dashboard_source_for_request(DashboardSourceRequest::new(
        input_dir,
        input_format,
        input_type,
        strict_workspace,
    ))
}

/// Resolve a dashboard source using a request struct.
pub(crate) fn load_dashboard_source_for_request(
    request: DashboardSourceRequest<'_>,
) -> Result<LoadedDashboardSource> {
    resolve_worktree_source(
        request.input_dir,
        request.input_format,
        request.input_type,
        request.strict_workspace,
    )
}

#[cfg(test)]
mod source_loader_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn infers_git_sync_workspace_root_from_wrapped_raw_tree() {
        let temp = tempdir().unwrap();
        let repo_root = temp.path();
        std::fs::create_dir_all(repo_root.join("dashboards/git-sync/raw")).unwrap();
        assert_eq!(
            infer_dashboard_workspace_root(&repo_root.join("dashboards/git-sync/raw")),
            repo_root.to_path_buf()
        );
    }

    #[test]
    fn infers_workspace_root_from_org_scoped_git_sync_raw_tree() {
        let temp = tempdir().unwrap();
        let repo_root = temp.path();
        std::fs::create_dir_all(repo_root.join("dashboards/git-sync/raw/org_1/raw")).unwrap();
        assert_eq!(
            infer_dashboard_workspace_root(&repo_root.join("dashboards/git-sync/raw/org_1/raw")),
            repo_root.to_path_buf()
        );
    }

    #[test]
    fn infers_workspace_root_from_org_scoped_raw_tree() {
        let temp = tempdir().unwrap();
        let repo_root = temp.path();
        std::fs::create_dir_all(repo_root.join("dashboards/raw/org_1/raw")).unwrap();
        assert_eq!(
            infer_dashboard_workspace_root(&repo_root.join("dashboards/raw/org_1/raw")),
            repo_root.to_path_buf()
        );
    }

    #[test]
    fn infers_workspace_root_from_org_scoped_git_sync_provisioning_tree() {
        let temp = tempdir().unwrap();
        let repo_root = temp.path();
        std::fs::create_dir_all(
            repo_root.join("dashboards/git-sync/provisioning/org_1/provisioning/dashboards"),
        )
        .unwrap();
        assert_eq!(
            infer_dashboard_workspace_root(
                &repo_root.join("dashboards/git-sync/provisioning/org_1/provisioning/dashboards")
            ),
            repo_root.to_path_buf()
        );
    }

    #[test]
    fn resolves_git_sync_wrapped_dashboard_variant_root() {
        let temp = tempdir().unwrap();
        let repo_root = temp.path();
        std::fs::create_dir_all(repo_root.join("dashboards/git-sync/provisioning")).unwrap();
        assert_eq!(
            resolve_dashboard_workspace_variant_dir(repo_root, "provisioning"),
            Some(repo_root.join("dashboards/git-sync/provisioning"))
        );
    }
}

#[cfg(test)]
#[path = "source_loader_contract_rust_tests.rs"]
mod source_loader_contract_rust_tests;
