//! Workspace path rules for task-first sync discovery.

use std::path::{Path, PathBuf};

use crate::dashboard::{infer_dashboard_workspace_root, DashboardSourceKind};

use super::DiscoveredChangeInputs;

const ACCESS_USER_EXPORT_DIR_NAME: &str = "access-users";
const ACCESS_TEAM_EXPORT_DIR_NAME: &str = "access-teams";
const ACCESS_ORG_EXPORT_DIR_NAME: &str = "access-orgs";
const ACCESS_SERVICE_ACCOUNT_EXPORT_DIR_NAME: &str = "access-service-accounts";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
enum DashboardWorkspaceLayout {
    RawExport,
    ProvisioningExport,
    GitSyncRawExport,
    GitSyncProvisioningExport,
}

impl DashboardWorkspaceLayout {
    fn source_kind(self) -> DashboardSourceKind {
        match self {
            Self::RawExport => DashboardSourceKind::RawExport,
            Self::ProvisioningExport => DashboardSourceKind::ProvisioningExport,
            Self::GitSyncRawExport => DashboardSourceKind::RawExport,
            Self::GitSyncProvisioningExport => DashboardSourceKind::ProvisioningExport,
        }
    }

    fn workspace_subdir(self) -> &'static str {
        match self {
            Self::RawExport | Self::GitSyncRawExport => "raw",
            Self::ProvisioningExport | Self::GitSyncProvisioningExport => "provisioning",
        }
    }

    fn wrapper_subdir(self) -> Option<&'static str> {
        match self {
            Self::GitSyncRawExport | Self::GitSyncProvisioningExport => Some("git-sync"),
            _ => None,
        }
    }
}

fn dashboard_workspace_layout_from_path(base_dir: &Path) -> Option<DashboardWorkspaceLayout> {
    let name = base_dir.file_name().and_then(|name| name.to_str())?;
    let parent_name = base_dir
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    let grandparent_name = base_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    match (grandparent_name, parent_name, name) {
        (Some("dashboards"), Some("git-sync"), "raw") => {
            Some(DashboardWorkspaceLayout::GitSyncRawExport)
        }
        (Some("dashboards"), Some("git-sync"), "provisioning") => {
            Some(DashboardWorkspaceLayout::GitSyncProvisioningExport)
        }
        (Some("dashboards"), _, "raw") => Some(DashboardWorkspaceLayout::RawExport),
        (Some("dashboards"), _, "provisioning") => {
            Some(DashboardWorkspaceLayout::ProvisioningExport)
        }
        _ => None,
    }
}

fn resolve_dashboard_workspace_dir(
    dashboards_dir: &Path,
    layout: DashboardWorkspaceLayout,
) -> Option<PathBuf> {
    let direct_candidate = dashboards_dir.join(layout.workspace_subdir());
    if direct_candidate.is_dir() {
        return Some(direct_candidate);
    }
    let wrapper_subdir = layout.wrapper_subdir()?;
    let wrapped_candidate = dashboards_dir
        .join(wrapper_subdir)
        .join(layout.workspace_subdir());
    wrapped_candidate.is_dir().then_some(wrapped_candidate)
}

fn dashboard_workspace_roots(base_dir: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    let dashboards_dir =
        if base_dir.file_name().and_then(|name| name.to_str()) == Some("dashboards") {
            base_dir.to_path_buf()
        } else {
            base_dir.join("dashboards")
        };
    let raw_dir =
        resolve_dashboard_workspace_dir(&dashboards_dir, DashboardWorkspaceLayout::RawExport)
            .or_else(|| {
                resolve_dashboard_workspace_dir(
                    &dashboards_dir,
                    DashboardWorkspaceLayout::GitSyncRawExport,
                )
            });
    let provisioning_dir = resolve_dashboard_workspace_dir(
        &dashboards_dir,
        DashboardWorkspaceLayout::ProvisioningExport,
    )
    .or_else(|| {
        resolve_dashboard_workspace_dir(
            &dashboards_dir,
            DashboardWorkspaceLayout::GitSyncProvisioningExport,
        )
    });
    (raw_dir, provisioning_dir)
}

fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

fn access_workspace_dirs(
    base_dir: &Path,
) -> (
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
) {
    (
        first_existing(&[base_dir.join(ACCESS_USER_EXPORT_DIR_NAME)]),
        first_existing(&[base_dir.join(ACCESS_TEAM_EXPORT_DIR_NAME)]),
        first_existing(&[base_dir.join(ACCESS_ORG_EXPORT_DIR_NAME)]),
        first_existing(&[base_dir.join(ACCESS_SERVICE_ACCOUNT_EXPORT_DIR_NAME)]),
    )
}

pub(crate) fn discover_from_workspace_root(base_dir: &Path) -> DiscoveredChangeInputs {
    let datasources_dir = base_dir.join("datasources");
    let alerts_dir = base_dir.join("alerts");
    let (dashboard_export_dir, dashboard_provisioning_dir) = dashboard_workspace_roots(base_dir);
    let (
        access_user_export_dir,
        access_team_export_dir,
        access_org_export_dir,
        access_service_account_export_dir,
    ) = access_workspace_dirs(base_dir);
    DiscoveredChangeInputs {
        workspace_root: Some(base_dir.to_path_buf()),
        dashboard_export_dir,
        dashboard_provisioning_dir,
        datasource_provisioning_file: first_existing(&[datasources_dir
            .join("provisioning")
            .join("datasources.yaml")]),
        access_user_export_dir,
        access_team_export_dir,
        access_org_export_dir,
        access_service_account_export_dir,
        alert_export_dir: first_existing(&[alerts_dir.join("raw"), alerts_dir]),
        desired_file: first_existing(&[base_dir.join("desired.json")]),
        source_bundle: first_existing(&[
            base_dir.join("sync-source-bundle.json"),
            base_dir.join("bundle.json"),
        ]),
        target_inventory: first_existing(&[
            base_dir.join("target-inventory.json"),
            base_dir.join("target.json"),
        ]),
        availability_file: first_existing(&[base_dir.join("availability.json")]),
        mapping_file: first_existing(&[
            base_dir.join("promotion-map.json"),
            base_dir.join("promotion-mapping.json"),
            base_dir.join("mapping.json"),
        ]),
        reviewed_plan_file: first_existing(&[
            base_dir.join("sync-plan-reviewed.json"),
            base_dir.join("reviewed-plan.json"),
            base_dir.join("sync-plan.json"),
        ]),
    }
}

pub(crate) fn infer_workspace_root(base_dir: &Path) -> PathBuf {
    for ancestor in base_dir.ancestors() {
        let Some(name) = ancestor.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if matches!(
            name,
            ACCESS_USER_EXPORT_DIR_NAME
                | ACCESS_TEAM_EXPORT_DIR_NAME
                | ACCESS_ORG_EXPORT_DIR_NAME
                | ACCESS_SERVICE_ACCOUNT_EXPORT_DIR_NAME
        ) {
            return ancestor.parent().unwrap_or(ancestor).to_path_buf();
        }
    }
    infer_dashboard_workspace_root(base_dir)
}

pub(crate) fn overlay_direct_workspace_input(
    discovered: &mut DiscoveredChangeInputs,
    base_dir: &Path,
) {
    let Some(name) = base_dir.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if base_dir.is_file() {
        overlay_direct_workspace_file(discovered, base_dir, name);
        return;
    }
    overlay_direct_workspace_dir(discovered, base_dir, name);
}

fn overlay_direct_workspace_file(
    discovered: &mut DiscoveredChangeInputs,
    base_dir: &Path,
    name: &str,
) {
    if name == "desired.json" {
        discovered.desired_file = Some(base_dir.to_path_buf());
    } else if matches!(name, "sync-source-bundle.json" | "bundle.json") {
        discovered.source_bundle = Some(base_dir.to_path_buf());
    } else if matches!(name, "target-inventory.json" | "target.json") {
        discovered.target_inventory = Some(base_dir.to_path_buf());
    } else if name == "availability.json" {
        discovered.availability_file = Some(base_dir.to_path_buf());
    } else if matches!(
        name,
        "promotion-map.json" | "promotion-mapping.json" | "mapping.json"
    ) {
        discovered.mapping_file = Some(base_dir.to_path_buf());
    } else if matches!(
        name,
        "sync-plan-reviewed.json" | "reviewed-plan.json" | "sync-plan.json"
    ) {
        discovered.reviewed_plan_file = Some(base_dir.to_path_buf());
    } else if name == "datasources.yaml" {
        let parent = base_dir.parent();
        let grandparent = parent.and_then(Path::parent);
        if parent.and_then(Path::file_name).and_then(|v| v.to_str()) == Some("provisioning")
            && grandparent
                .and_then(Path::file_name)
                .and_then(|v| v.to_str())
                == Some("datasources")
        {
            discovered.datasource_provisioning_file = Some(base_dir.to_path_buf());
        }
    }
}

fn overlay_direct_workspace_dir(
    discovered: &mut DiscoveredChangeInputs,
    base_dir: &Path,
    name: &str,
) {
    if let Some(layout) = dashboard_workspace_layout_from_path(base_dir) {
        match layout.source_kind() {
            DashboardSourceKind::RawExport => {
                discovered.dashboard_export_dir = Some(base_dir.to_path_buf())
            }
            DashboardSourceKind::ProvisioningExport => {
                discovered.dashboard_provisioning_dir = Some(base_dir.to_path_buf())
            }
            DashboardSourceKind::LiveGrafana | DashboardSourceKind::HistoryArtifact => {}
        }
        return;
    }
    let parent_name = base_dir
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    match (parent_name, name) {
        (_, ACCESS_USER_EXPORT_DIR_NAME) => {
            discovered.access_user_export_dir = Some(base_dir.to_path_buf());
        }
        (_, ACCESS_TEAM_EXPORT_DIR_NAME) => {
            discovered.access_team_export_dir = Some(base_dir.to_path_buf());
        }
        (_, ACCESS_ORG_EXPORT_DIR_NAME) => {
            discovered.access_org_export_dir = Some(base_dir.to_path_buf());
        }
        (_, ACCESS_SERVICE_ACCOUNT_EXPORT_DIR_NAME) => {
            discovered.access_service_account_export_dir = Some(base_dir.to_path_buf());
        }
        (Some("alerts"), "raw") => {
            discovered.alert_export_dir = Some(base_dir.to_path_buf());
        }
        (Some("datasources"), "provisioning") => {
            discovered.datasource_provisioning_file =
                first_existing(&[base_dir.join("datasources.yaml")]);
        }
        (_, "alerts") => {
            discovered.alert_export_dir = first_existing(&[base_dir.join("raw"), base_dir.into()]);
        }
        (_, "datasources") => {
            discovered.datasource_provisioning_file =
                first_existing(&[base_dir.join("provisioning").join("datasources.yaml")]);
        }
        _ => {}
    }
}
