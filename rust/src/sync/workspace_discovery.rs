//! Workspace discovery and discovery/provenance helpers for task-first sync workflows.

use std::env;
use std::path::{Path, PathBuf};

use crate::common::{message, Result};
use crate::dashboard::{infer_dashboard_workspace_root, DashboardSourceKind};
use crate::overview::OverviewDocument;
use crate::project_status::ProjectStatus;
use serde_json::Value;

use super::discovery_model::{
    render_discovery_provenance_line, render_discovery_summary_line, ChangeDiscoveryDocument,
    DiscoveryInput, DiscoveryInputKind,
};
use super::{SyncCommandOutput, SyncOutputFormat};

const ACCESS_USER_EXPORT_DIR_NAME: &str = "access-users";
const ACCESS_TEAM_EXPORT_DIR_NAME: &str = "access-teams";
const ACCESS_ORG_EXPORT_DIR_NAME: &str = "access-orgs";
const ACCESS_SERVICE_ACCOUNT_EXPORT_DIR_NAME: &str = "access-service-accounts";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DiscoveredChangeInputs {
    pub workspace_root: Option<PathBuf>,
    pub dashboard_export_dir: Option<PathBuf>,
    pub dashboard_provisioning_dir: Option<PathBuf>,
    pub datasource_provisioning_file: Option<PathBuf>,
    pub access_user_export_dir: Option<PathBuf>,
    pub access_team_export_dir: Option<PathBuf>,
    pub access_org_export_dir: Option<PathBuf>,
    pub access_service_account_export_dir: Option<PathBuf>,
    pub alert_export_dir: Option<PathBuf>,
    pub desired_file: Option<PathBuf>,
    pub source_bundle: Option<PathBuf>,
    pub target_inventory: Option<PathBuf>,
    pub availability_file: Option<PathBuf>,
    pub mapping_file: Option<PathBuf>,
    pub reviewed_plan_file: Option<PathBuf>,
}

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
            Self::RawExport => "raw",
            Self::ProvisioningExport => "provisioning",
            Self::GitSyncRawExport => "raw",
            Self::GitSyncProvisioningExport => "provisioning",
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

fn current_repo_dir() -> Result<PathBuf> {
    env::current_dir()
        .map_err(|error| message(format!("Could not resolve current directory: {error}")))
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

fn discover_from_workspace_root(base_dir: &Path) -> DiscoveredChangeInputs {
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

fn infer_workspace_root(base_dir: &Path) -> PathBuf {
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

fn overlay_direct_workspace_input(discovered: &mut DiscoveredChangeInputs, base_dir: &Path) {
    let Some(name) = base_dir.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if base_dir.is_file() {
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
        return;
    }
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

pub(crate) fn render_discovery_provenance(discovered: &DiscoveredChangeInputs) -> Option<String> {
    build_change_discovery(discovered)
        .and_then(|document| render_discovery_provenance_line(&document))
}

pub(crate) fn render_discovery_summary(discovered: &DiscoveredChangeInputs) -> Option<String> {
    build_change_discovery(discovered).and_then(|document| render_discovery_summary_line(&document))
}

pub(crate) fn build_discovery_document(discovered: &DiscoveredChangeInputs) -> Option<Value> {
    build_change_discovery(discovered).map(|document| document.to_value())
}

pub(crate) fn attach_discovery_to_overview(
    mut document: OverviewDocument,
    discovered: &DiscoveredChangeInputs,
) -> OverviewDocument {
    document.discovery = build_discovery_document(discovered);
    document
}

pub(crate) fn attach_discovery_to_status(
    mut status: ProjectStatus,
    discovered: &DiscoveredChangeInputs,
) -> ProjectStatus {
    status.discovery = build_discovery_document(discovered);
    status
}

pub(crate) fn attach_discovery_to_sync_output(
    mut output: SyncCommandOutput,
    discovered: &DiscoveredChangeInputs,
) -> SyncCommandOutput {
    if let Some(discovery) = build_discovery_document(discovered) {
        if let Some(object) = output.document.as_object_mut() {
            object.insert("discovery".to_string(), discovery.clone());
        }
        if let Some(summary) = render_discovery_summary(discovered) {
            output.text_lines.insert(0, summary);
        }
    }
    output
}

pub(crate) fn emit_discovery_provenance(
    discovered: &DiscoveredChangeInputs,
    output_format: SyncOutputFormat,
) {
    if let Some(note) = render_discovery_provenance(discovered) {
        if !matches!(output_format, SyncOutputFormat::Text) {
            eprintln!("{note}");
        }
    }
}

pub(crate) fn discover_change_staged_inputs(
    base_dir: Option<&Path>,
) -> Result<DiscoveredChangeInputs> {
    let base_dir = match base_dir {
        Some(path) => path.to_path_buf(),
        None => current_repo_dir()?,
    };
    let workspace_root = infer_workspace_root(&base_dir);
    let mut discovered = discover_from_workspace_root(&workspace_root);
    overlay_direct_workspace_input(&mut discovered, &base_dir);
    Ok(discovered)
}

fn build_change_discovery(discovered: &DiscoveredChangeInputs) -> Option<ChangeDiscoveryDocument> {
    let workspace_root = discovered.workspace_root.clone()?;
    let mut inputs = Vec::new();
    if let Some(path) = discovered.dashboard_export_dir.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::DashboardExportDir,
            path,
        });
    }
    if let Some(path) = discovered.dashboard_provisioning_dir.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::DashboardProvisioningDir,
            path,
        });
    }
    if let Some(path) = discovered.datasource_provisioning_file.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::DatasourceProvisioningFile,
            path,
        });
    }
    if let Some(path) = discovered.access_user_export_dir.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::AccessUserExportDir,
            path,
        });
    }
    if let Some(path) = discovered.access_team_export_dir.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::AccessTeamExportDir,
            path,
        });
    }
    if let Some(path) = discovered.access_org_export_dir.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::AccessOrgExportDir,
            path,
        });
    }
    if let Some(path) = discovered.access_service_account_export_dir.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::AccessServiceAccountExportDir,
            path,
        });
    }
    if let Some(path) = discovered.alert_export_dir.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::AlertExportDir,
            path,
        });
    }
    if let Some(path) = discovered.desired_file.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::DesiredFile,
            path,
        });
    }
    if let Some(path) = discovered.source_bundle.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::SourceBundle,
            path,
        });
    }
    if let Some(path) = discovered.target_inventory.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::TargetInventory,
            path,
        });
    }
    if let Some(path) = discovered.availability_file.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::AvailabilityFile,
            path,
        });
    }
    if let Some(path) = discovered.mapping_file.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::MappingFile,
            path,
        });
    }
    if let Some(path) = discovered.reviewed_plan_file.clone() {
        inputs.push(DiscoveryInput {
            kind: DiscoveryInputKind::ReviewedPlanFile,
            path,
        });
    }
    Some(ChangeDiscoveryDocument::from_inputs(
        Some(workspace_root),
        inputs,
    ))
}

#[cfg(test)]
mod workspace_discovery_rust_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn discover_change_staged_inputs_accepts_dashboards_git_sync_raw_dir_as_workspace() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join("dashboards/git-sync/raw")).unwrap();
        std::fs::create_dir_all(workspace.join("dashboards/git-sync/provisioning")).unwrap();

        let discovered =
            discover_change_staged_inputs(Some(&workspace.join("dashboards/git-sync/raw")))
                .unwrap();

        assert_eq!(
            discovered.dashboard_export_dir,
            Some(workspace.join("dashboards/git-sync/raw"))
        );
        assert_eq!(
            discovered.dashboard_provisioning_dir,
            Some(workspace.join("dashboards/git-sync/provisioning"))
        );
    }

    #[test]
    fn discover_change_staged_inputs_accepts_mixed_git_sync_repo_workspace() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join("dashboards/git-sync/raw")).unwrap();
        std::fs::create_dir_all(workspace.join("dashboards/git-sync/provisioning")).unwrap();
        std::fs::create_dir_all(workspace.join("alerts/raw")).unwrap();
        std::fs::create_dir_all(workspace.join("datasources/provisioning")).unwrap();
        std::fs::write(
            workspace.join("datasources/provisioning/datasources.yaml"),
            "apiVersion: 1\n",
        )
        .unwrap();

        let discovered = discover_change_staged_inputs(Some(workspace)).unwrap();

        assert_eq!(
            discovered.dashboard_export_dir,
            Some(workspace.join("dashboards/git-sync/raw"))
        );
        assert_eq!(
            discovered.dashboard_provisioning_dir,
            Some(workspace.join("dashboards/git-sync/provisioning"))
        );
        assert_eq!(
            discovered.alert_export_dir,
            Some(workspace.join("alerts/raw"))
        );
        assert_eq!(
            discovered.datasource_provisioning_file,
            Some(workspace.join("datasources/provisioning/datasources.yaml"))
        );
        assert_eq!(discovered.workspace_root, Some(workspace.to_path_buf()));
        let provenance = render_discovery_provenance(&discovered).unwrap();
        assert!(provenance.contains("dashboard-export="));
        assert!(provenance.contains("dashboard-provisioning="));
        assert!(provenance.contains("alert-export="));
        assert!(provenance.contains("datasource-provisioning="));
    }

    #[test]
    fn discover_change_staged_inputs_accepts_mixed_workspace_with_access_exports() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join("dashboards/raw")).unwrap();
        std::fs::create_dir_all(workspace.join("alerts/raw")).unwrap();
        std::fs::create_dir_all(workspace.join("datasources/provisioning")).unwrap();
        std::fs::write(
            workspace.join("datasources/provisioning/datasources.yaml"),
            "apiVersion: 1\n",
        )
        .unwrap();
        std::fs::create_dir_all(workspace.join("access-users")).unwrap();
        std::fs::create_dir_all(workspace.join("access-teams")).unwrap();
        std::fs::create_dir_all(workspace.join("access-orgs")).unwrap();
        std::fs::create_dir_all(workspace.join("access-service-accounts")).unwrap();

        let discovered = discover_change_staged_inputs(Some(workspace)).unwrap();

        assert_eq!(
            discovered.access_user_export_dir,
            Some(workspace.join("access-users"))
        );
        assert_eq!(
            discovered.access_team_export_dir,
            Some(workspace.join("access-teams"))
        );
        assert_eq!(
            discovered.access_org_export_dir,
            Some(workspace.join("access-orgs"))
        );
        assert_eq!(
            discovered.access_service_account_export_dir,
            Some(workspace.join("access-service-accounts"))
        );
        let provenance = render_discovery_provenance(&discovered).unwrap();
        assert!(provenance.contains("access-users="));
        assert!(provenance.contains("access-teams="));
        assert!(provenance.contains("access-orgs="));
        assert!(provenance.contains("access-service-accounts="));
    }

    #[test]
    fn render_discovery_provenance_reports_workspace_root_and_sources() {
        let discovered = DiscoveredChangeInputs {
            workspace_root: Some(PathBuf::from("/tmp/grafana-oac-repo")),
            dashboard_export_dir: Some(PathBuf::from(
                "/tmp/grafana-oac-repo/dashboards/git-sync/raw",
            )),
            dashboard_provisioning_dir: Some(PathBuf::from(
                "/tmp/grafana-oac-repo/dashboards/git-sync/provisioning",
            )),
            datasource_provisioning_file: Some(PathBuf::from(
                "/tmp/grafana-oac-repo/datasources/provisioning/datasources.yaml",
            )),
            alert_export_dir: Some(PathBuf::from("/tmp/grafana-oac-repo/alerts/raw")),
            ..DiscoveredChangeInputs::default()
        };

        let provenance = render_discovery_provenance(&discovered).unwrap();
        assert!(provenance.contains("Discovered change workspace root /tmp/grafana-oac-repo"));
        assert!(
            provenance.contains("dashboard-export=/tmp/grafana-oac-repo/dashboards/git-sync/raw")
        );
        assert!(provenance.contains(
            "dashboard-provisioning=/tmp/grafana-oac-repo/dashboards/git-sync/provisioning"
        ));
        assert!(provenance.contains("datasource-provisioning=/tmp/grafana-oac-repo/datasources/provisioning/datasources.yaml"));
        assert!(provenance.contains("alert-export=/tmp/grafana-oac-repo/alerts/raw"));
    }

    #[test]
    fn build_discovery_document_reports_workspace_root_and_inputs() {
        let discovered = DiscoveredChangeInputs {
            workspace_root: Some(PathBuf::from("/tmp/grafana-oac-repo")),
            dashboard_export_dir: Some(PathBuf::from(
                "/tmp/grafana-oac-repo/dashboards/git-sync/raw",
            )),
            alert_export_dir: Some(PathBuf::from("/tmp/grafana-oac-repo/alerts/raw")),
            datasource_provisioning_file: Some(PathBuf::from(
                "/tmp/grafana-oac-repo/datasources/provisioning/datasources.yaml",
            )),
            ..DiscoveredChangeInputs::default()
        };

        let document = build_discovery_document(&discovered).unwrap();
        assert_eq!(
            document["workspaceRoot"],
            Value::String("/tmp/grafana-oac-repo".to_string())
        );
        assert_eq!(document["inputCount"], Value::from(3));
        assert_eq!(
            document["inputs"]["dashboardExportDir"],
            Value::String("/tmp/grafana-oac-repo/dashboards/git-sync/raw".to_string())
        );
        assert_eq!(
            document["inputs"]["alertExportDir"],
            Value::String("/tmp/grafana-oac-repo/alerts/raw".to_string())
        );
    }

    #[test]
    fn attach_discovery_to_sync_output_adds_top_level_document_and_text_summary() {
        let discovered = DiscoveredChangeInputs {
            workspace_root: Some(PathBuf::from("/tmp/grafana-oac-repo")),
            dashboard_export_dir: Some(PathBuf::from(
                "/tmp/grafana-oac-repo/dashboards/git-sync/raw",
            )),
            ..DiscoveredChangeInputs::default()
        };
        let output = SyncCommandOutput {
            document: serde_json::json!({
                "kind": "grafana-utils-sync-plan",
                "schemaVersion": 1
            }),
            text_lines: vec!["SYNC PLAN".to_string()],
        };

        let attached = attach_discovery_to_sync_output(output, &discovered);
        assert_eq!(
            attached.document["discovery"]["workspaceRoot"],
            Value::String("/tmp/grafana-oac-repo".to_string())
        );
        assert_eq!(
            attached.text_lines.first(),
            Some(
                &"Discovery: workspace-root=/tmp/grafana-oac-repo sources=dashboard-export"
                    .to_string()
            )
        );
    }

    #[test]
    fn discover_change_staged_inputs_accepts_org_scoped_git_sync_raw_subtree() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join("dashboards/git-sync/raw/org_1/raw")).unwrap();
        std::fs::create_dir_all(workspace.join("dashboards/git-sync/provisioning")).unwrap();

        let discovered = discover_change_staged_inputs(Some(
            &workspace.join("dashboards/git-sync/raw/org_1/raw"),
        ))
        .unwrap();

        assert_eq!(discovered.workspace_root, Some(workspace.to_path_buf()));
        assert_eq!(
            discovered.dashboard_export_dir,
            Some(workspace.join("dashboards/git-sync/raw"))
        );
        assert_eq!(
            discovered.dashboard_provisioning_dir,
            Some(workspace.join("dashboards/git-sync/provisioning"))
        );
    }

    #[test]
    fn discover_change_staged_inputs_accepts_org_scoped_git_sync_provisioning_subtree() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join("dashboards/git-sync/raw")).unwrap();
        std::fs::create_dir_all(
            workspace.join("dashboards/git-sync/provisioning/org_1/provisioning/dashboards"),
        )
        .unwrap();

        let discovered = discover_change_staged_inputs(Some(
            &workspace.join("dashboards/git-sync/provisioning/org_1/provisioning/dashboards"),
        ))
        .unwrap();

        assert_eq!(discovered.workspace_root, Some(workspace.to_path_buf()));
        assert_eq!(
            discovered.dashboard_export_dir,
            Some(workspace.join("dashboards/git-sync/raw"))
        );
        assert_eq!(
            discovered.dashboard_provisioning_dir,
            Some(workspace.join("dashboards/git-sync/provisioning"))
        );
    }
}
