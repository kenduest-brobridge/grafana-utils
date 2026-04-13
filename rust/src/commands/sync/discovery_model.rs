//! Shared discovery/provenance model for task-first sync workflows.
//!
//! This module keeps the workspace discovery payload in a small canonical
//! shape so inspect/check/preview/bundle can share the same serialization and
//! text-summary logic instead of hand-assembling ad hoc JSON maps.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DiscoveryInputKind {
    DashboardExportDir,
    DashboardProvisioningDir,
    DatasourceProvisioningFile,
    DatasourceExportFile,
    AccessUserExportDir,
    AccessTeamExportDir,
    AccessOrgExportDir,
    AccessServiceAccountExportDir,
    AlertExportDir,
    DesiredFile,
    SourceBundle,
    TargetInventory,
    AvailabilityFile,
    MappingFile,
    ReviewedPlanFile,
    MetadataFile,
}

impl DiscoveryInputKind {
    pub(crate) fn json_key(self) -> &'static str {
        match self {
            Self::DashboardExportDir => "dashboardExportDir",
            Self::DashboardProvisioningDir => "dashboardProvisioningDir",
            Self::DatasourceProvisioningFile => "datasourceProvisioningFile",
            Self::DatasourceExportFile => "datasourceExportFile",
            Self::AccessUserExportDir => "accessUserExportDir",
            Self::AccessTeamExportDir => "accessTeamExportDir",
            Self::AccessOrgExportDir => "accessOrgExportDir",
            Self::AccessServiceAccountExportDir => "accessServiceAccountExportDir",
            Self::AlertExportDir => "alertExportDir",
            Self::DesiredFile => "desiredFile",
            Self::SourceBundle => "sourceBundle",
            Self::TargetInventory => "targetInventory",
            Self::AvailabilityFile => "availabilityFile",
            Self::MappingFile => "mappingFile",
            Self::ReviewedPlanFile => "reviewedPlanFile",
            Self::MetadataFile => "metadataFile",
        }
    }

    pub(crate) fn summary_label(self) -> &'static str {
        match self {
            Self::DashboardExportDir => "dashboard-export",
            Self::DashboardProvisioningDir => "dashboard-provisioning",
            Self::DatasourceProvisioningFile => "datasource-provisioning",
            Self::DatasourceExportFile => "datasource-export",
            Self::AccessUserExportDir => "access-users",
            Self::AccessTeamExportDir => "access-teams",
            Self::AccessOrgExportDir => "access-orgs",
            Self::AccessServiceAccountExportDir => "access-service-accounts",
            Self::AlertExportDir => "alert-export",
            Self::DesiredFile => "desired-file",
            Self::SourceBundle => "source-bundle",
            Self::TargetInventory => "target-inventory",
            Self::AvailabilityFile => "availability-file",
            Self::MappingFile => "mapping-file",
            Self::ReviewedPlanFile => "reviewed-plan-file",
            Self::MetadataFile => "metadata-file",
        }
    }

    pub(crate) fn from_json_key(key: &str) -> Option<Self> {
        match key {
            "dashboardExportDir" => Some(Self::DashboardExportDir),
            "dashboardProvisioningDir" => Some(Self::DashboardProvisioningDir),
            "datasourceProvisioningFile" => Some(Self::DatasourceProvisioningFile),
            "datasourceExportFile" => Some(Self::DatasourceExportFile),
            "accessUserExportDir" => Some(Self::AccessUserExportDir),
            "accessTeamExportDir" => Some(Self::AccessTeamExportDir),
            "accessOrgExportDir" => Some(Self::AccessOrgExportDir),
            "accessServiceAccountExportDir" => Some(Self::AccessServiceAccountExportDir),
            "alertExportDir" => Some(Self::AlertExportDir),
            "desiredFile" => Some(Self::DesiredFile),
            "sourceBundle" => Some(Self::SourceBundle),
            "targetInventory" => Some(Self::TargetInventory),
            "availabilityFile" => Some(Self::AvailabilityFile),
            "mappingFile" => Some(Self::MappingFile),
            "reviewedPlanFile" => Some(Self::ReviewedPlanFile),
            "metadataFile" => Some(Self::MetadataFile),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoveryInput {
    pub(crate) kind: DiscoveryInputKind,
    pub(crate) path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ChangeDiscoveryDocument {
    pub(crate) workspace_root: Option<PathBuf>,
    pub(crate) inputs: BTreeMap<DiscoveryInputKind, PathBuf>,
}

impl ChangeDiscoveryDocument {
    pub(crate) fn new(workspace_root: Option<PathBuf>) -> Self {
        Self {
            workspace_root,
            inputs: BTreeMap::new(),
        }
    }

    pub(crate) fn from_inputs(
        workspace_root: Option<PathBuf>,
        inputs: impl IntoIterator<Item = DiscoveryInput>,
    ) -> Self {
        let mut document = Self::new(workspace_root);
        document.extend(inputs);
        document
    }

    pub(crate) fn insert(
        &mut self,
        kind: DiscoveryInputKind,
        path: impl Into<PathBuf>,
    ) -> Option<PathBuf> {
        self.inputs.insert(kind, path.into())
    }

    pub(crate) fn extend(&mut self, inputs: impl IntoIterator<Item = DiscoveryInput>) {
        for input in inputs {
            self.insert(input.kind, input.path);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    pub(crate) fn summary_line(&self) -> Option<String> {
        if self.inputs.is_empty() {
            return None;
        }
        let workspace_root = self.workspace_root.as_ref()?;
        let sources = self
            .inputs
            .keys()
            .map(|kind| kind.summary_label())
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            "Discovery: workspace-root={} sources={}",
            workspace_root.display(),
            sources
        ))
    }

    pub(crate) fn provenance_line(&self) -> Option<String> {
        if self.inputs.is_empty() {
            return None;
        }
        let workspace_root = self.workspace_root.as_ref()?;
        let sources = self
            .inputs
            .iter()
            .map(|(kind, path)| format!("{}={}", kind.summary_label(), path.display()))
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            "Discovered change workspace root {} from {}.",
            workspace_root.display(),
            sources
        ))
    }

    pub(crate) fn to_value(&self) -> Value {
        let mut object = Map::new();
        if let Some(workspace_root) = self.workspace_root.as_ref() {
            object.insert(
                "workspaceRoot".to_string(),
                Value::String(workspace_root.display().to_string()),
            );
        }
        object.insert(
            "inputCount".to_string(),
            Value::from(self.inputs.len() as i64),
        );
        object.insert(
            "inputs".to_string(),
            Value::Object(
                self.inputs
                    .iter()
                    .map(|(kind, path)| {
                        (
                            kind.json_key().to_string(),
                            Value::String(path.display().to_string()),
                        )
                    })
                    .collect(),
            ),
        );
        Value::Object(object)
    }

    pub(crate) fn from_value_object(discovery: &Map<String, Value>) -> Option<Self> {
        let workspace_root = discovery
            .get("workspaceRoot")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        let inputs = discovery.get("inputs").and_then(Value::as_object)?;
        let mut document = Self::new(workspace_root);
        for (key, value) in inputs {
            let kind = DiscoveryInputKind::from_json_key(key)?;
            let path = value.as_str()?;
            document.insert(kind, PathBuf::from(path));
        }
        Some(document)
    }
}

impl From<&ChangeDiscoveryDocument> for Value {
    fn from(document: &ChangeDiscoveryDocument) -> Self {
        document.to_value()
    }
}

pub(crate) fn render_discovery_summary_line(document: &ChangeDiscoveryDocument) -> Option<String> {
    document.summary_line()
}

pub(crate) fn render_discovery_provenance_line(
    document: &ChangeDiscoveryDocument,
) -> Option<String> {
    document.provenance_line()
}

pub(crate) fn render_discovery_summary_from_value(
    discovery: &Map<String, Value>,
) -> Option<String> {
    ChangeDiscoveryDocument::from_value_object(discovery)?.summary_line()
}

#[cfg(test)]
mod discovery_model_tests {
    use super::*;

    #[test]
    fn serializes_discovery_document_to_expected_value_shape() {
        let mut document =
            ChangeDiscoveryDocument::new(Some(PathBuf::from("/tmp/grafana-oac-repo")));
        document.insert(
            DiscoveryInputKind::DashboardExportDir,
            "/tmp/grafana-oac-repo/dashboards/raw",
        );
        document.insert(
            DiscoveryInputKind::AlertExportDir,
            "/tmp/grafana-oac-repo/alerts/raw",
        );
        document.insert(
            DiscoveryInputKind::MetadataFile,
            "/tmp/grafana-oac-repo/metadata.json",
        );

        let value = document.to_value();
        assert_eq!(
            value["workspaceRoot"],
            Value::String("/tmp/grafana-oac-repo".to_string())
        );
        assert_eq!(value["inputCount"], Value::from(3));
        assert_eq!(
            value["inputs"]["dashboardExportDir"],
            Value::String("/tmp/grafana-oac-repo/dashboards/raw".to_string())
        );
        assert_eq!(
            value["inputs"]["alertExportDir"],
            Value::String("/tmp/grafana-oac-repo/alerts/raw".to_string())
        );
        assert_eq!(
            value["inputs"]["metadataFile"],
            Value::String("/tmp/grafana-oac-repo/metadata.json".to_string())
        );
    }

    #[test]
    fn renders_summary_and_provenance_lines_from_document() {
        let document = ChangeDiscoveryDocument::from_inputs(
            Some(PathBuf::from("/tmp/grafana-oac-repo")),
            vec![
                DiscoveryInput {
                    kind: DiscoveryInputKind::DashboardExportDir,
                    path: PathBuf::from("/tmp/grafana-oac-repo/dashboards/raw"),
                },
                DiscoveryInput {
                    kind: DiscoveryInputKind::DatasourceProvisioningFile,
                    path: PathBuf::from(
                        "/tmp/grafana-oac-repo/datasources/provisioning/datasources.yaml",
                    ),
                },
            ],
        );

        assert_eq!(
            render_discovery_summary_line(&document),
            Some(
                "Discovery: workspace-root=/tmp/grafana-oac-repo sources=dashboard-export, datasource-provisioning"
                    .to_string()
            )
        );
        assert_eq!(
            render_discovery_provenance_line(&document),
            Some(
                "Discovered change workspace root /tmp/grafana-oac-repo from dashboard-export=/tmp/grafana-oac-repo/dashboards/raw, datasource-provisioning=/tmp/grafana-oac-repo/datasources/provisioning/datasources.yaml."
                    .to_string()
            )
        );
    }
}
