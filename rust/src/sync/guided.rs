//! Task-first `grafana-util change` workflow helpers.

use std::env;
use std::path::{Path, PathBuf};

use super::{
    attach_lineage, attach_trace_id, build_alert_sync_specs, build_sync_plan_document,
    cli::{
        execute_sync_bundle_preflight, execute_sync_plan, execute_sync_promotion_preflight,
        load_sync_live_array, render_and_emit_sync_command_output,
    },
    load_alerting_bundle_section, load_dashboard_bundle_sections,
    load_dashboard_provisioning_bundle_sections, load_datasource_provisioning_records,
    load_json_array_file, load_json_value, normalize_datasource_bundle_item, render_sync_plan_text,
    ChangeCheckArgs, ChangeInspectArgs, ChangePreviewArgs, ChangeStagedInputsArgs, Result,
    SyncBundlePreflightArgs, SyncCommandOutput, SyncPlanArgs, SyncPromotionPreflightArgs,
};
use crate::common::{emit_plain_output, message};
use crate::overview::{run_overview, OverviewArgs, OverviewOutputFormat};
use crate::project_status_command::{
    run_project_status_staged, ProjectStatusOutputFormat, ProjectStatusStagedArgs,
};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoveredChangeInputs {
    pub dashboard_export_dir: Option<PathBuf>,
    pub dashboard_provisioning_dir: Option<PathBuf>,
    pub datasource_provisioning_file: Option<PathBuf>,
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

fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

fn discover_from_workspace_root(base_dir: &Path) -> DiscoveredChangeInputs {
    let dashboards_dir = base_dir.join("dashboards");
    let datasources_dir = base_dir.join("datasources");
    let alerts_dir = base_dir.join("alerts");
    DiscoveredChangeInputs {
        dashboard_export_dir: first_existing(&[dashboards_dir.join("raw")]),
        dashboard_provisioning_dir: first_existing(&[dashboards_dir.join("provisioning")]),
        datasource_provisioning_file: first_existing(&[datasources_dir
            .join("provisioning")
            .join("datasources.yaml")]),
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
    let Some(name) = base_dir.file_name().and_then(|name| name.to_str()) else {
        return base_dir.to_path_buf();
    };
    if base_dir.is_file() {
        if name == "datasources.yaml" {
            let parent = base_dir.parent();
            let grandparent = parent.and_then(Path::parent);
            let great_grandparent = grandparent.and_then(Path::parent);
            if parent.and_then(Path::file_name).and_then(|v| v.to_str()) == Some("provisioning")
                && grandparent
                    .and_then(Path::file_name)
                    .and_then(|v| v.to_str())
                    == Some("datasources")
            {
                return great_grandparent.unwrap_or(base_dir).to_path_buf();
            }
        }
        return base_dir.parent().unwrap_or(base_dir).to_path_buf();
    }
    let parent = base_dir.parent();
    let grandparent = parent.and_then(Path::parent);
    match name {
        "dashboards" | "alerts" | "datasources" => parent.unwrap_or(base_dir).to_path_buf(),
        "raw" | "provisioning" => grandparent.unwrap_or(base_dir).to_path_buf(),
        _ => base_dir.to_path_buf(),
    }
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
    let parent_name = base_dir
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    match (parent_name, name) {
        (Some("dashboards"), "raw") => {
            discovered.dashboard_export_dir = Some(base_dir.to_path_buf());
        }
        (Some("dashboards"), "provisioning") => {
            discovered.dashboard_provisioning_dir = Some(base_dir.to_path_buf());
        }
        (Some("alerts"), "raw") => {
            discovered.alert_export_dir = Some(base_dir.to_path_buf());
        }
        (Some("datasources"), "provisioning") => {
            discovered.datasource_provisioning_file =
                first_existing(&[base_dir.join("datasources.yaml")]);
        }
        (_, "dashboards") => {
            discovered.dashboard_export_dir = first_existing(&[base_dir.join("raw")]);
            discovered.dashboard_provisioning_dir =
                first_existing(&[base_dir.join("provisioning")]);
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

fn ensure_any_discovered(discovered: &DiscoveredChangeInputs) -> Result<()> {
    if discovered == &DiscoveredChangeInputs::default() {
        return Err(message(
            "Change input discovery did not find staged inputs in the current directory. Provide explicit flags such as --desired-file, --dashboard-export-dir, or --source-bundle.",
        ));
    }
    Ok(())
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

fn build_overview_args(
    args: &ChangeInspectArgs,
    discovered: &DiscoveredChangeInputs,
) -> OverviewArgs {
    OverviewArgs {
        dashboard_export_dir: args
            .inputs
            .dashboard_export_dir
            .clone()
            .or(discovered.dashboard_export_dir.clone()),
        dashboard_provisioning_dir: args
            .inputs
            .dashboard_provisioning_dir
            .clone()
            .or(discovered.dashboard_provisioning_dir.clone()),
        datasource_export_dir: None,
        datasource_provisioning_file: args
            .inputs
            .datasource_provisioning_file
            .clone()
            .or(discovered.datasource_provisioning_file.clone()),
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: args
            .inputs
            .desired_file
            .clone()
            .or(discovered.desired_file.clone()),
        source_bundle: args
            .inputs
            .source_bundle
            .clone()
            .or(discovered.source_bundle.clone()),
        target_inventory: discovered.target_inventory.clone(),
        alert_export_dir: args
            .inputs
            .alert_export_dir
            .clone()
            .or(discovered.alert_export_dir.clone()),
        availability_file: discovered.availability_file.clone(),
        mapping_file: discovered.mapping_file.clone(),
        output_format: match args.output.output_format {
            super::SyncOutputFormat::Text => OverviewOutputFormat::Text,
            super::SyncOutputFormat::Json => OverviewOutputFormat::Json,
        },
    }
}

fn build_status_args(
    args: &ChangeCheckArgs,
    discovered: &DiscoveredChangeInputs,
) -> ProjectStatusStagedArgs {
    ProjectStatusStagedArgs {
        dashboard_export_dir: args
            .inputs
            .dashboard_export_dir
            .clone()
            .or(discovered.dashboard_export_dir.clone()),
        dashboard_provisioning_dir: args
            .inputs
            .dashboard_provisioning_dir
            .clone()
            .or(discovered.dashboard_provisioning_dir.clone()),
        datasource_export_dir: None,
        datasource_provisioning_file: args
            .inputs
            .datasource_provisioning_file
            .clone()
            .or(discovered.datasource_provisioning_file.clone()),
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: args
            .inputs
            .desired_file
            .clone()
            .or(discovered.desired_file.clone()),
        source_bundle: args
            .inputs
            .source_bundle
            .clone()
            .or(discovered.source_bundle.clone()),
        target_inventory: args
            .target_inventory
            .clone()
            .or(discovered.target_inventory.clone()),
        alert_export_dir: args
            .inputs
            .alert_export_dir
            .clone()
            .or(discovered.alert_export_dir.clone()),
        availability_file: args
            .availability_file
            .clone()
            .or(discovered.availability_file.clone()),
        mapping_file: args
            .mapping_file
            .clone()
            .or(discovered.mapping_file.clone()),
        output_format: match args.output.output_format {
            super::SyncOutputFormat::Text => ProjectStatusOutputFormat::Text,
            super::SyncOutputFormat::Json => ProjectStatusOutputFormat::Json,
        },
    }
}

fn emit_preview_output(
    output: SyncCommandOutput,
    output_file: Option<&PathBuf>,
    also_stdout: bool,
    format: super::SyncOutputFormat,
) -> Result<()> {
    if let Some(path) = output_file {
        let persisted = match format {
            super::SyncOutputFormat::Json => serde_json::to_string_pretty(&output.document)?,
            super::SyncOutputFormat::Text => output.text_lines.join("\n"),
        };
        emit_plain_output(&persisted, Some(path.as_path()), false)?;
    }
    if output_file.is_none() || also_stdout {
        render_and_emit_sync_command_output(output, format)?;
    }
    Ok(())
}

fn select_preview_dashboard_sources<'a>(
    inputs: &'a ChangeStagedInputsArgs,
    discovered: &'a DiscoveredChangeInputs,
) -> Result<(Option<&'a PathBuf>, Option<&'a PathBuf>)> {
    if inputs.dashboard_export_dir.is_some() && inputs.dashboard_provisioning_dir.is_some() {
        return Err(message(
            "Change preview accepts only one dashboard source: --dashboard-export-dir or --dashboard-provisioning-dir.",
        ));
    }
    if let Some(path) = inputs.dashboard_export_dir.as_ref() {
        return Ok((Some(path), None));
    }
    if let Some(path) = inputs.dashboard_provisioning_dir.as_ref() {
        return Ok((None, Some(path)));
    }
    if let Some(path) = discovered.dashboard_export_dir.as_ref() {
        return Ok((Some(path), None));
    }
    Ok((None, discovered.dashboard_provisioning_dir.as_ref()))
}

fn build_change_bundle_specs(
    inputs: &ChangeStagedInputsArgs,
    discovered: &DiscoveredChangeInputs,
) -> Result<Option<Vec<Value>>> {
    let (dashboard_export_dir, dashboard_provisioning_dir) =
        select_preview_dashboard_sources(inputs, discovered)?;
    let alert_export_dir = inputs
        .alert_export_dir
        .as_ref()
        .or(discovered.alert_export_dir.as_ref());
    let datasource_provisioning_file = inputs
        .datasource_provisioning_file
        .as_ref()
        .or(discovered.datasource_provisioning_file.as_ref());

    if dashboard_export_dir.is_none()
        && dashboard_provisioning_dir.is_none()
        && alert_export_dir.is_none()
        && datasource_provisioning_file.is_none()
    {
        return Ok(None);
    }

    let mut dashboards = Vec::new();
    let mut datasources = Vec::new();
    let mut folders = Vec::new();
    if let Some(output_dir) = dashboard_export_dir {
        let (dashboard_items, dashboard_datasources, folder_items, _dashboard_metadata) =
            load_dashboard_bundle_sections(
                output_dir,
                output_dir,
                datasource_provisioning_file.map(PathBuf::as_path),
            )?;
        dashboards = dashboard_items;
        datasources.extend(dashboard_datasources);
        folders = folder_items;
    } else if let Some(provisioning_dir) = dashboard_provisioning_dir {
        let (dashboard_items, dashboard_datasources, folder_items, _dashboard_metadata) =
            load_dashboard_provisioning_bundle_sections(
                provisioning_dir,
                datasource_provisioning_file.map(PathBuf::as_path),
            )?;
        dashboards = dashboard_items;
        datasources.extend(dashboard_datasources);
        folders = folder_items;
    }
    if let Some(path) = datasource_provisioning_file {
        datasources = load_datasource_provisioning_records(path)?
            .into_iter()
            .map(|item| normalize_datasource_bundle_item(&item))
            .collect::<Result<Vec<_>>>()?;
    }
    let alerting = match alert_export_dir {
        Some(path) => load_alerting_bundle_section(path)?,
        None => Value::Object(Map::new()),
    };
    let alerts = build_alert_sync_specs(&alerting)?;

    let mut desired_specs = Vec::new();
    desired_specs.extend(dashboards);
    desired_specs.extend(datasources);
    desired_specs.extend(folders);
    desired_specs.extend(alerts);
    Ok(Some(desired_specs))
}

fn load_preview_desired_specs(
    inputs: &ChangeStagedInputsArgs,
    discovered: &DiscoveredChangeInputs,
) -> Result<Vec<Value>> {
    if let Some(path) = inputs
        .desired_file
        .as_ref()
        .or(discovered.desired_file.as_ref())
    {
        return load_json_array_file(path, "Change desired input");
    }
    if let Some(path) = inputs
        .source_bundle
        .as_ref()
        .or(discovered.source_bundle.as_ref())
    {
        let source_bundle = load_json_value(path, "Change source bundle input")?;
        let bundle = source_bundle
            .as_object()
            .ok_or_else(|| message("Change source bundle input must be a JSON object."))?;
        let mut desired_specs = Vec::new();
        for key in ["dashboards", "datasources", "folders", "alerts"] {
            if let Some(items) = bundle.get(key).and_then(Value::as_array) {
                desired_specs.extend(items.iter().cloned());
            }
        }
        return Ok(desired_specs);
    }
    if let Some(specs) = build_change_bundle_specs(inputs, discovered)? {
        return Ok(specs);
    }
    Err(message(
        "Change preview could not find a staged desired change file, source bundle, or staged export/provisioning inputs.",
    ))
}

pub(crate) fn run_sync_inspect(args: ChangeInspectArgs) -> Result<()> {
    let discovered = discover_change_staged_inputs(Some(args.inputs.workspace.as_path()))?;
    let merged = build_overview_args(&args, &discovered);
    if merged.dashboard_export_dir.is_none()
        && merged.dashboard_provisioning_dir.is_none()
        && merged.datasource_provisioning_file.is_none()
        && merged.alert_export_dir.is_none()
        && merged.desired_file.is_none()
        && merged.source_bundle.is_none()
    {
        ensure_any_discovered(&discovered)?;
    }
    run_overview(merged)
}

pub(crate) fn run_sync_check(args: ChangeCheckArgs) -> Result<()> {
    let discovered = discover_change_staged_inputs(Some(args.inputs.workspace.as_path()))?;
    let merged = build_status_args(&args, &discovered);
    if merged.dashboard_export_dir.is_none()
        && merged.dashboard_provisioning_dir.is_none()
        && merged.datasource_provisioning_file.is_none()
        && merged.alert_export_dir.is_none()
        && merged.desired_file.is_none()
        && merged.source_bundle.is_none()
    {
        ensure_any_discovered(&discovered)?;
    }
    run_project_status_staged(merged)
}

pub(crate) fn run_sync_preview(args: ChangePreviewArgs) -> Result<()> {
    let discovered = discover_change_staged_inputs(Some(args.inputs.workspace.as_path()))?;
    let source_bundle = args
        .inputs
        .source_bundle
        .clone()
        .or(discovered.source_bundle.clone());
    let target_inventory = args
        .target_inventory
        .clone()
        .or(discovered.target_inventory.clone());
    let mapping_file = args
        .mapping_file
        .clone()
        .or(discovered.mapping_file.clone());
    let availability_file = args
        .availability_file
        .clone()
        .or(discovered.availability_file.clone());
    if let (Some(source_bundle), Some(target_inventory), Some(mapping_file)) = (
        source_bundle.clone(),
        target_inventory.clone(),
        mapping_file.clone(),
    ) {
        let output = execute_sync_promotion_preflight(&SyncPromotionPreflightArgs {
            source_bundle,
            target_inventory,
            mapping_file: Some(mapping_file),
            availability_file,
            fetch_live: args.fetch_live,
            common: args.common.clone(),
            org_id: args.org_id,
            output_format: args.output.output_format,
        })?;
        return emit_preview_output(
            output,
            args.output.output_file.as_ref(),
            args.output.also_stdout,
            args.output.output_format,
        );
    }
    if let (Some(source_bundle), Some(target_inventory)) = (source_bundle, target_inventory) {
        let output = execute_sync_bundle_preflight(&SyncBundlePreflightArgs {
            source_bundle,
            target_inventory,
            availability_file,
            fetch_live: args.fetch_live,
            common: args.common.clone(),
            org_id: args.org_id,
            output_format: args.output.output_format,
        })?;
        return emit_preview_output(
            output,
            args.output.output_file.as_ref(),
            args.output.also_stdout,
            args.output.output_format,
        );
    }
    let output = if let Some(desired_file) = args
        .inputs
        .desired_file
        .clone()
        .or(discovered.desired_file.clone())
    {
        execute_sync_plan(&SyncPlanArgs {
            desired_file,
            live_file: args.live_file.clone(),
            fetch_live: args.fetch_live,
            common: args.common.clone(),
            org_id: args.org_id,
            page_size: args.page_size,
            allow_prune: args.allow_prune,
            output_format: args.output.output_format,
            trace_id: args.trace_id.clone(),
        })?
    } else {
        let desired = load_preview_desired_specs(&args.inputs, &discovered)?;
        let live = load_sync_live_array(
            args.fetch_live,
            &args.common,
            args.org_id,
            args.page_size,
            args.live_file.as_ref(),
            "Change preview requires --live-file unless --fetch-live is used.",
        )?;
        let document = attach_lineage(
            &attach_trace_id(
                &build_sync_plan_document(&desired, &live, args.allow_prune)?,
                args.trace_id.as_deref(),
            )?,
            "plan",
            1,
            None,
        )?;
        SyncCommandOutput {
            text_lines: render_sync_plan_text(&document)?,
            document,
        }
    };
    emit_preview_output(
        output,
        args.output.output_file.as_ref(),
        args.output.also_stdout,
        args.output.output_format,
    )
}

#[cfg(test)]
mod guided_rust_tests {
    use super::*;
    use tempfile::tempdir;

    fn staged_inputs(
        dashboard_export_dir: Option<&str>,
        dashboard_provisioning_dir: Option<&str>,
    ) -> ChangeStagedInputsArgs {
        ChangeStagedInputsArgs {
            workspace: PathBuf::from("."),
            desired_file: None,
            source_bundle: None,
            dashboard_export_dir: dashboard_export_dir.map(PathBuf::from),
            dashboard_provisioning_dir: dashboard_provisioning_dir.map(PathBuf::from),
            alert_export_dir: None,
            datasource_export_file: None,
            datasource_provisioning_file: None,
        }
    }

    #[test]
    fn select_preview_dashboard_sources_prefers_explicit_export_input() {
        let inputs = staged_inputs(Some("./dashboards/raw"), None);
        let discovered = DiscoveredChangeInputs {
            dashboard_provisioning_dir: Some(PathBuf::from("./dashboards/provisioning")),
            ..DiscoveredChangeInputs::default()
        };

        let (output_dir, provisioning_dir) =
            select_preview_dashboard_sources(&inputs, &discovered).unwrap();

        assert_eq!(output_dir, Some(&PathBuf::from("./dashboards/raw")));
        assert!(provisioning_dir.is_none());
    }

    #[test]
    fn select_preview_dashboard_sources_prefers_discovered_export_over_provisioning() {
        let inputs = staged_inputs(None, None);
        let discovered = DiscoveredChangeInputs {
            dashboard_export_dir: Some(PathBuf::from("./dashboards/raw")),
            dashboard_provisioning_dir: Some(PathBuf::from("./dashboards/provisioning")),
            ..DiscoveredChangeInputs::default()
        };

        let (output_dir, provisioning_dir) =
            select_preview_dashboard_sources(&inputs, &discovered).unwrap();

        assert_eq!(output_dir, Some(&PathBuf::from("./dashboards/raw")));
        assert!(provisioning_dir.is_none());
    }

    #[test]
    fn select_preview_dashboard_sources_rejects_two_explicit_dashboard_inputs() {
        let inputs = staged_inputs(Some("./dashboards/raw"), Some("./dashboards/provisioning"));
        let error = select_preview_dashboard_sources(&inputs, &DiscoveredChangeInputs::default())
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Change preview accepts only one dashboard source: --dashboard-export-dir or --dashboard-provisioning-dir."
        );
    }

    #[test]
    fn discover_change_staged_inputs_accepts_dashboards_tree_as_workspace() {
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

        let discovered =
            discover_change_staged_inputs(Some(&workspace.join("dashboards"))).unwrap();

        assert_eq!(
            discovered.dashboard_export_dir,
            Some(workspace.join("dashboards/raw"))
        );
        assert_eq!(
            discovered.alert_export_dir,
            Some(workspace.join("alerts/raw"))
        );
        assert_eq!(
            discovered.datasource_provisioning_file,
            Some(workspace.join("datasources/provisioning/datasources.yaml"))
        );
    }

    #[test]
    fn discover_change_staged_inputs_accepts_dashboard_raw_tree_as_workspace() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join("dashboards/raw")).unwrap();
        std::fs::create_dir_all(workspace.join("dashboards/provisioning")).unwrap();
        std::fs::create_dir_all(workspace.join("alerts/raw")).unwrap();

        let discovered =
            discover_change_staged_inputs(Some(&workspace.join("dashboards/raw"))).unwrap();

        assert_eq!(
            discovered.dashboard_export_dir,
            Some(workspace.join("dashboards/raw"))
        );
        assert_eq!(
            discovered.dashboard_provisioning_dir,
            Some(workspace.join("dashboards/provisioning"))
        );
        assert_eq!(
            discovered.alert_export_dir,
            Some(workspace.join("alerts/raw"))
        );
    }

    #[test]
    fn discover_change_staged_inputs_accepts_datasource_provisioning_tree_as_workspace() {
        let temp = tempdir().unwrap();
        let workspace = temp.path();
        std::fs::create_dir_all(workspace.join("dashboards/raw")).unwrap();
        std::fs::create_dir_all(workspace.join("datasources/provisioning")).unwrap();
        std::fs::write(
            workspace.join("datasources/provisioning/datasources.yaml"),
            "apiVersion: 1\n",
        )
        .unwrap();

        let discovered =
            discover_change_staged_inputs(Some(&workspace.join("datasources/provisioning")))
                .unwrap();

        assert_eq!(
            discovered.datasource_provisioning_file,
            Some(workspace.join("datasources/provisioning/datasources.yaml"))
        );
        assert_eq!(
            discovered.dashboard_export_dir,
            Some(workspace.join("dashboards/raw"))
        );
    }
}
