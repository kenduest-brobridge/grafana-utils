//! Offline migration path for converting raw dashboard JSON into prompt-lane artifacts.

use serde_json::Value;
use std::path::Path;

use crate::common::{message, Result};
use crate::grafana_api::{DashboardResourceClient, DatasourceResourceClient};

use super::raw_to_prompt_datasource_resolution::{
    build_synthetic_catalog, collect_reference_families, rewrite_datasource_refs,
};
use super::raw_to_prompt_prompt_paths::{
    collect_library_panel_portability_warnings, collect_panel_placeholder_datasource_paths,
    is_dashboard_v2_payload, load_live_library_panel_exports, raw_to_prompt_live_lookup_requested,
    rewrite_prompt_panel_placeholder_paths,
};
use super::raw_to_prompt_types::{
    DashboardScanContext, DatasourceMapDocument, RawToPromptOutcome, RawToPromptResolutionKind,
    RawToPromptStats,
};
use super::{
    build_datasource_catalog, build_datasource_inventory_record, build_external_export_document,
    build_http_client, build_http_client_for_org, load_json_file, CommonCliArgs,
    DatasourceInventoryItem, RawToPromptArgs, RawToPromptResolution, DEFAULT_TIMEOUT, DEFAULT_URL,
};
use crate::dashboard::prompt::build_external_export_document_with_library_panels;

pub(crate) use super::raw_to_prompt_datasource_resolution::load_datasource_mapping;

pub(crate) fn load_live_datasource_inventory(
    args: &RawToPromptArgs,
) -> Result<Vec<DatasourceInventoryItem>> {
    if !raw_to_prompt_live_lookup_requested(args) {
        return Ok(Vec::new());
    }
    let common = CommonCliArgs {
        color: args.color,
        profile: args.profile.clone(),
        url: args.url.clone().unwrap_or_else(|| DEFAULT_URL.to_string()),
        api_token: args.api_token.clone(),
        username: args.username.clone(),
        password: args.password.clone(),
        prompt_password: args.prompt_password,
        prompt_token: args.prompt_token,
        timeout: args.timeout.unwrap_or(DEFAULT_TIMEOUT),
        verify_ssl: args.verify_ssl,
    };
    let client = match args.org_id {
        Some(org_id) => build_http_client_for_org(&common, org_id)?,
        None => build_http_client(&common)?,
    };
    let dashboard = DashboardResourceClient::new(&client);
    let datasource = DatasourceResourceClient::new(&client);
    let current_org = dashboard.fetch_current_org()?;
    let datasources = datasource.list_datasources()?;
    Ok(datasources
        .iter()
        .map(|datasource| build_datasource_inventory_record(datasource, &current_org))
        .collect())
}

pub(crate) fn convert_raw_dashboard_file(
    input_path: &Path,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    resolution: RawToPromptResolution,
    live_args: Option<&RawToPromptArgs>,
) -> Result<RawToPromptOutcome> {
    let payload = load_json_file(input_path)?;
    if is_dashboard_v2_payload(&payload) {
        return Err(message(
            "dashboard raw-to-prompt does not support Grafana dashboard v2 resources yet; export classic dashboard JSON for the prompt lane.",
        ));
    }
    let mut dashboard = super::build_preserved_web_import_document(&payload)?;
    let mut warnings = collect_library_panel_portability_warnings(&dashboard);
    let placeholder_paths = collect_panel_placeholder_datasource_paths(&dashboard);
    let mut scan = DashboardScanContext::default();
    collect_reference_families(&mut dashboard, &mut scan);
    let mut stats = RawToPromptStats::default();
    rewrite_datasource_refs(
        &mut dashboard,
        datasource_inventory,
        mapping,
        &scan,
        resolution,
        &mut warnings,
        &mut stats,
    )?;
    let datasource_catalog = build_datasource_catalog(&build_synthetic_catalog(&dashboard));
    let live_library_panels = if let Some(args) = live_args {
        if raw_to_prompt_live_lookup_requested(args) {
            Some(load_live_library_panel_exports(args, &dashboard)?)
        } else {
            None
        }
    } else {
        None
    };
    let mut prompt_document = if let Some(library_panels) = live_library_panels.as_ref() {
        build_external_export_document_with_library_panels(
            &dashboard,
            &datasource_catalog,
            Some(library_panels),
        )?
    } else {
        build_external_export_document(&dashboard, &datasource_catalog)?
    };
    rewrite_prompt_panel_placeholder_paths(&mut prompt_document, &placeholder_paths);
    let datasource_slots = prompt_document
        .get("__inputs")
        .and_then(Value::as_array)
        .map(|items: &Vec<Value>| items.len())
        .unwrap_or(0);
    let resolution_kind = if stats.inferred > 0 {
        RawToPromptResolutionKind::Inferred
    } else {
        RawToPromptResolutionKind::Exact
    };
    Ok(RawToPromptOutcome {
        prompt_document,
        datasource_slots,
        resolution: resolution_kind,
        warnings,
    })
}
