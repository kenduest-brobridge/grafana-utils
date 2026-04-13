//! Workspace discovery and discovery/provenance helpers for task-first sync workflows.

use std::env;
use std::path::{Path, PathBuf};

use crate::common::{message, Result};
use crate::overview::OverviewDocument;
use crate::project_status::ProjectStatus;
use serde_json::Value;

#[path = "workspace_discovery_rules.rs"]
mod workspace_discovery_rules;

use super::discovery_model::{
    render_discovery_provenance_line, render_discovery_summary_line, ChangeDiscoveryDocument,
    DiscoveryInput, DiscoveryInputKind,
};
use super::{SyncCommandOutput, SyncOutputFormat};

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

fn current_repo_dir() -> Result<PathBuf> {
    env::current_dir()
        .map_err(|error| message(format!("Could not resolve current directory: {error}")))
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
    let workspace_root = workspace_discovery_rules::infer_workspace_root(&base_dir);
    let mut discovered = workspace_discovery_rules::discover_from_workspace_root(&workspace_root);
    workspace_discovery_rules::overlay_direct_workspace_input(&mut discovered, &base_dir);
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
