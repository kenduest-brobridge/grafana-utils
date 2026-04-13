//! Shared source-bundle input loading pipeline.

use std::path::PathBuf;

use serde_json::{Map, Value};

use super::{
    build_alert_sync_specs, load_alerting_bundle_section, load_dashboard_bundle_sections,
    load_dashboard_provisioning_bundle_sections, load_datasource_provisioning_records,
    load_json_array_file, load_optional_json_object_file, normalize_datasource_bundle_item, Result,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct SyncBundleInputSelection {
    pub(crate) workspace_root: Option<PathBuf>,
    pub(crate) dashboard_export_dir: Option<PathBuf>,
    pub(crate) dashboard_provisioning_dir: Option<PathBuf>,
    pub(crate) alert_export_dir: Option<PathBuf>,
    pub(crate) datasource_export_file: Option<PathBuf>,
    pub(crate) datasource_provisioning_file: Option<PathBuf>,
    pub(crate) metadata_file: Option<PathBuf>,
}

impl SyncBundleInputSelection {
    pub(crate) fn has_inputs(&self) -> bool {
        self.dashboard_export_dir.is_some()
            || self.dashboard_provisioning_dir.is_some()
            || self.alert_export_dir.is_some()
            || self.datasource_export_file.is_some()
            || self.datasource_provisioning_file.is_some()
            || self.metadata_file.is_some()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SyncBundleInputArtifacts {
    pub(crate) dashboards: Vec<Value>,
    pub(crate) datasources: Vec<Value>,
    pub(crate) folders: Vec<Value>,
    pub(crate) alerting: Value,
    pub(crate) alerts: Vec<Value>,
    pub(crate) metadata: Map<String, Value>,
}

pub(crate) fn load_sync_bundle_input_artifacts(
    selection: &SyncBundleInputSelection,
) -> Result<SyncBundleInputArtifacts> {
    let mut artifacts = SyncBundleInputArtifacts::default();
    if let Some(workspace_root) = selection.workspace_root.as_ref() {
        artifacts.metadata.insert(
            "workspaceRoot".to_string(),
            Value::String(workspace_root.display().to_string()),
        );
    }
    load_dashboard_inputs(selection, &mut artifacts)?;
    load_datasource_inputs(selection, &mut artifacts)?;
    load_alert_inputs(selection, &mut artifacts)?;
    load_extra_metadata(selection.metadata_file.as_ref(), &mut artifacts)?;
    Ok(artifacts)
}

fn load_dashboard_inputs(
    selection: &SyncBundleInputSelection,
    artifacts: &mut SyncBundleInputArtifacts,
) -> Result<()> {
    if let Some(output_dir) = selection.dashboard_export_dir.as_ref() {
        let (dashboard_items, dashboard_datasources, folder_items, dashboard_metadata) =
            load_dashboard_bundle_sections(
                output_dir,
                output_dir,
                selection.datasource_provisioning_file.as_deref(),
            )?;
        artifacts.dashboards = dashboard_items;
        artifacts.datasources.extend(dashboard_datasources);
        artifacts.folders = folder_items;
        artifacts.metadata.extend(dashboard_metadata);
        artifacts.metadata.insert(
            "dashboardExportDir".to_string(),
            Value::String(output_dir.display().to_string()),
        );
    } else if let Some(provisioning_dir) = selection.dashboard_provisioning_dir.as_ref() {
        let (dashboard_items, dashboard_datasources, folder_items, dashboard_metadata) =
            load_dashboard_provisioning_bundle_sections(
                provisioning_dir,
                selection.datasource_provisioning_file.as_deref(),
            )?;
        artifacts.dashboards = dashboard_items;
        artifacts.datasources.extend(dashboard_datasources);
        artifacts.folders = folder_items;
        artifacts.metadata.extend(dashboard_metadata);
        artifacts.metadata.insert(
            "dashboardProvisioningDir".to_string(),
            Value::String(provisioning_dir.display().to_string()),
        );
    }
    Ok(())
}

fn load_datasource_inputs(
    selection: &SyncBundleInputSelection,
    artifacts: &mut SyncBundleInputArtifacts,
) -> Result<()> {
    if let Some(datasource_provisioning_file) = selection.datasource_provisioning_file.as_ref() {
        artifacts.datasources = load_datasource_provisioning_records(datasource_provisioning_file)?
            .into_iter()
            .map(|item| normalize_datasource_bundle_item(&item))
            .collect::<Result<Vec<_>>>()?;
        artifacts.metadata.insert(
            "datasourceProvisioningFile".to_string(),
            Value::String(datasource_provisioning_file.display().to_string()),
        );
    } else if let Some(datasource_export_file) = selection.datasource_export_file.as_ref() {
        artifacts.datasources =
            load_json_array_file(datasource_export_file, "Datasource export inventory")?
                .into_iter()
                .map(|item| normalize_datasource_bundle_item(&item))
                .collect::<Result<Vec<_>>>()?;
        artifacts.metadata.insert(
            "datasourceExportFile".to_string(),
            Value::String(datasource_export_file.display().to_string()),
        );
    }
    Ok(())
}

fn load_alert_inputs(
    selection: &SyncBundleInputSelection,
    artifacts: &mut SyncBundleInputArtifacts,
) -> Result<()> {
    artifacts.alerting = match selection.alert_export_dir.as_ref() {
        Some(output_dir) => {
            artifacts.metadata.insert(
                "alertExportDir".to_string(),
                Value::String(output_dir.display().to_string()),
            );
            load_alerting_bundle_section(output_dir)?
        }
        None => Value::Object(Map::new()),
    };
    artifacts.alerts = build_alert_sync_specs(&artifacts.alerting)?;
    Ok(())
}

fn load_extra_metadata(
    metadata_file: Option<&PathBuf>,
    artifacts: &mut SyncBundleInputArtifacts,
) -> Result<()> {
    if let Some(extra_metadata) =
        load_optional_json_object_file(metadata_file, "Sync bundle metadata input")?
    {
        if let Some(object) = extra_metadata.as_object() {
            artifacts.metadata.extend(object.clone());
        }
    }
    Ok(())
}
