//! Snapshot wrappers for dashboard and datasource exports plus local review.
//!
//! This module stays thin: it derives the per-domain paths/args for a snapshot
//! export root, then builds a snapshot-native inventory review document from
//! the exported dashboard and datasource metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Args, CommandFactory, Parser, Subcommand};
use rpassword::prompt_password;
use serde_json::{json, Value};

use crate::common::{GrafanaCliError, Result};
use crate::dashboard::{
    self, CommonCliArgs, DashboardCliArgs, DashboardCommand, ExportArgs as DashboardExportArgs,
    TempInspectDir, EXPORT_METADATA_FILENAME, ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION,
};
use crate::datasource::{DatasourceExportArgs, DatasourceGroupCommand};
#[cfg(any(feature = "tui", test))]
use crate::interactive_browser::{run_interactive_browser, BrowserItem};
use crate::overview::{OverviewArgs, OverviewOutputFormat};
use crate::staged_export_scopes::{
    resolve_dashboard_export_scope_dirs, resolve_datasource_export_scope_dirs,
};
use crate::tabular_output::{print_lines, render_csv, render_table, render_yaml};

pub const SNAPSHOT_DASHBOARD_DIR: &str = "dashboards";
pub const SNAPSHOT_DATASOURCE_DIR: &str = "datasources";
pub const SNAPSHOT_DATASOURCE_EXPORT_FILENAME: &str = "datasources.json";
pub const SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
pub const SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND: &str = "grafana-utils-datasource-export-index";
pub const SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION: i64 = 1;
const SNAPSHOT_REVIEW_KIND: &str = "grafana-utils-snapshot-review";
const SNAPSHOT_REVIEW_SCHEMA_VERSION: i64 = 1;
const SNAPSHOT_ROOT_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./snapshot\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./snapshot --overwrite\n\n  grafana-util snapshot review --input-dir ./snapshot --output table\n\n  grafana-util snapshot review --input-dir ./snapshot --interactive";
const SNAPSHOT_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./snapshot\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./snapshot --overwrite";
const SNAPSHOT_REVIEW_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot review --input-dir ./snapshot --output table\n  grafana-util snapshot review --input-dir ./snapshot --output csv\n  grafana-util snapshot review --input-dir ./snapshot --output text\n  grafana-util snapshot review --input-dir ./snapshot --output json\n  grafana-util snapshot review --input-dir ./snapshot --output yaml\n  grafana-util snapshot review --input-dir ./snapshot --interactive";

#[cfg(feature = "tui")]
const SNAPSHOT_REVIEW_OUTPUT_HELP: &str =
    "Render the snapshot inventory review as table, csv, text, json, yaml, or interactive browser output.";

#[cfg(not(feature = "tui"))]
const SNAPSHOT_REVIEW_OUTPUT_HELP: &str =
    "Render the snapshot inventory review as table, csv, text, json, or yaml output.";

#[derive(Debug, Clone, Args)]
pub struct SnapshotExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = "snapshot",
        help = "Directory to write the snapshot export root into. The live export writes dashboard and datasource bundles under this root."
    )]
    pub export_dir: PathBuf,
    #[arg(
        long,
        help = "Replace an existing snapshot export root instead of failing when the dashboard or datasource export directories already exist."
    )]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Args)]
pub struct SnapshotReviewArgs {
    #[arg(
        long,
        default_value = "snapshot",
        help = "Directory containing a previously exported snapshot root. The review reads the dashboard and datasource inventory under this root."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "output",
        help = "Shortcut for --output interactive."
    )]
    pub interactive: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = OverviewOutputFormat::Text,
        help = SNAPSHOT_REVIEW_OUTPUT_HELP
    )]
    pub output: OverviewOutputFormat,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util snapshot",
    about = "Export and review Grafana snapshot inventory bundles.",
    after_help = SNAPSHOT_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
pub struct SnapshotCliArgs {
    #[command(subcommand)]
    pub command: SnapshotCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SnapshotCommand {
    #[command(
        name = "export",
        about = "Export dashboard and datasource inventory into a local snapshot bundle.",
        after_help = SNAPSHOT_EXPORT_HELP_TEXT
    )]
    Export(SnapshotExportArgs),
    #[command(
        name = "review",
        about = "Review a local snapshot inventory without touching Grafana.",
        after_help = SNAPSHOT_REVIEW_HELP_TEXT
    )]
    Review(SnapshotReviewArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPaths {
    pub dashboards: PathBuf,
    pub datasources: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct SnapshotReviewOrgCounts {
    org: String,
    org_id: String,
    dashboard_count: usize,
    folder_count: usize,
    datasource_count: usize,
    default_datasource_count: usize,
    datasource_types: BTreeMap<String, usize>,
}

pub fn build_snapshot_paths(export_dir: &Path) -> SnapshotPaths {
    SnapshotPaths {
        dashboards: export_dir.join(SNAPSHOT_DASHBOARD_DIR),
        datasources: export_dir.join(SNAPSHOT_DATASOURCE_DIR),
    }
}

pub fn root_command() -> clap::Command {
    SnapshotCliArgs::command()
}

pub fn build_snapshot_overview_args(args: &SnapshotReviewArgs) -> OverviewArgs {
    let paths = build_snapshot_paths(&args.input_dir);
    OverviewArgs {
        dashboard_export_dir: Some(paths.dashboards),
        dashboard_provisioning_dir: None,
        datasource_export_dir: Some(paths.datasources),
        datasource_provisioning_file: None,
        access_user_export_dir: None,
        access_team_export_dir: None,
        access_org_export_dir: None,
        access_service_account_export_dir: None,
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output: args.output,
    }
}

pub fn build_snapshot_dashboard_export_args(args: &SnapshotExportArgs) -> DashboardExportArgs {
    let paths = build_snapshot_paths(&args.export_dir);
    DashboardExportArgs {
        common: args.common.clone(),
        export_dir: paths.dashboards,
        page_size: dashboard::DEFAULT_PAGE_SIZE,
        org_id: None,
        all_orgs: true,
        flat: false,
        overwrite: args.overwrite,
        without_dashboard_raw: false,
        without_dashboard_prompt: false,
        without_dashboard_provisioning: false,
        provisioning_provider_name: "grafana-utils-dashboards".to_string(),
        provisioning_provider_org_id: None,
        provisioning_provider_path: None,
        provisioning_provider_disable_deletion: false,
        provisioning_provider_allow_ui_updates: false,
        provisioning_provider_update_interval_seconds: 30,
        dry_run: false,
        progress: false,
        verbose: false,
    }
}

pub fn build_snapshot_datasource_export_args(args: &SnapshotExportArgs) -> DatasourceExportArgs {
    let paths = build_snapshot_paths(&args.export_dir);
    DatasourceExportArgs {
        common: args.common.clone(),
        export_dir: paths.datasources,
        org_id: None,
        all_orgs: true,
        overwrite: args.overwrite,
        without_datasource_provisioning: false,
        dry_run: false,
    }
}

fn materialize_snapshot_common_auth_with_prompt<F, G>(
    mut common: CommonCliArgs,
    mut prompt_password_reader: F,
    mut prompt_token_reader: G,
) -> Result<CommonCliArgs>
where
    F: FnMut() -> Result<String>,
    G: FnMut() -> Result<String>,
{
    if common.prompt_password && common.password.is_none() {
        common.password = Some(prompt_password_reader()?);
    }
    if common.prompt_token && common.api_token.is_none() {
        common.api_token = Some(prompt_token_reader()?);
    }
    common.prompt_password = false;
    common.prompt_token = false;
    Ok(common)
}

fn materialize_snapshot_common_auth(common: CommonCliArgs) -> Result<CommonCliArgs> {
    materialize_snapshot_common_auth_with_prompt(
        common,
        || prompt_password("Grafana Basic auth password: ").map_err(GrafanaCliError::from),
        || prompt_password("Grafana API token: ").map_err(GrafanaCliError::from),
    )
}

fn snapshot_review_org_key(org_id: &str, org: &str) -> String {
    if !org_id.trim().is_empty() {
        format!("org-id:{org_id}")
    } else if !org.trim().is_empty() {
        format!("org:{org}")
    } else {
        "org:unknown".to_string()
    }
}

fn load_json_value_file(path: &Path, label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|error| {
        crate::common::message(format!(
            "{label} must contain valid JSON in {}: {}",
            path.display(),
            error
        ))
    })
}

fn load_snapshot_dashboard_metadata(dashboard_dir: &Path) -> Result<Value> {
    let metadata_path = dashboard_dir.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Err(crate::common::message(format!(
            "Snapshot dashboard export is missing metadata: {}",
            metadata_path.display()
        )));
    }
    let metadata = load_json_value_file(&metadata_path, "Snapshot dashboard export metadata")?;
    let kind = metadata
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let schema_version = metadata
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let variant = metadata
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if kind != ROOT_INDEX_KIND || schema_version != TOOL_SCHEMA_VERSION || variant != "root" {
        return Err(crate::common::message(format!(
            "Snapshot dashboard export metadata is not a supported root export: {}",
            metadata_path.display()
        )));
    }
    Ok(metadata)
}

fn load_snapshot_dashboard_index(dashboard_dir: &Path) -> Result<Value> {
    let index_path = dashboard_dir.join("index.json");
    if index_path.is_file() {
        return load_json_value_file(&index_path, "Snapshot dashboard export index");
    }
    Ok(json!({
        "kind": ROOT_INDEX_KIND,
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "items": [],
        "variants": {
            "raw": null,
            "prompt": null,
            "provisioning": null
        },
        "folders": []
    }))
}

fn build_dashboard_lane_summary(scope_dirs: &[PathBuf]) -> Value {
    let scope_count = scope_dirs.len() as u64;
    let raw_count = scope_dirs
        .iter()
        .filter(|scope| scope.join("raw").join("index.json").is_file())
        .count() as u64;
    let prompt_count = scope_dirs
        .iter()
        .filter(|scope| scope.join("prompt").join("index.json").is_file())
        .count() as u64;
    let provisioning_count = scope_dirs
        .iter()
        .filter(|scope| {
            scope.join("provisioning").join("index.json").is_file()
                && scope
                    .join("provisioning")
                    .join("provisioning")
                    .join("dashboards.yaml")
                    .is_file()
        })
        .count() as u64;
    json!({
        "scopeCount": scope_count,
        "rawScopeCount": raw_count,
        "promptScopeCount": prompt_count,
        "provisioningScopeCount": provisioning_count,
    })
}

fn build_datasource_lane_summary(datasource_lane_dir: &Path, scope_dirs: &[PathBuf]) -> Value {
    let scope_count = scope_dirs.len() as u64;
    let metadata_path = datasource_lane_dir.join(SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME);
    let variant = fs::read_to_string(&metadata_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| {
            value
                .get("variant")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    let has_non_root_scopes = scope_dirs.iter().any(|scope| scope != datasource_lane_dir);
    let inventory_scope_dirs: Vec<&PathBuf> = if variant == "all-orgs-root" && has_non_root_scopes {
        scope_dirs
            .iter()
            .filter(|scope| scope.as_path() != datasource_lane_dir)
            .collect()
    } else {
        scope_dirs.iter().collect()
    };
    let inventory_count = inventory_scope_dirs
        .iter()
        .filter(|scope| scope.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME).is_file())
        .count() as u64;
    let provisioning_count = scope_dirs
        .iter()
        .filter(|scope| {
            scope
                .join("provisioning")
                .join("datasources.yaml")
                .is_file()
        })
        .count() as u64;
    json!({
        "scopeCount": scope_count,
        "inventoryExpectedScopeCount": inventory_scope_dirs.len() as u64,
        "inventoryScopeCount": inventory_count,
        "provisioningExpectedScopeCount": scope_count,
        "provisioningScopeCount": provisioning_count,
    })
}

fn load_snapshot_datasource_rows(datasource_dir: &Path) -> Result<Vec<Value>> {
    let metadata_path = datasource_dir.join(SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME);
    let metadata = load_json_value_file(&metadata_path, "Snapshot datasource export metadata")?;
    let kind = metadata
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let schema_version = metadata
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let variant = metadata
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let resource = metadata
        .get("resource")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if kind != SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND
        || schema_version != SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION
        || variant != "root"
        || resource != "datasource"
    {
        return Err(crate::common::message(format!(
            "Snapshot datasource export metadata is not a supported root export: {}",
            metadata_path.display()
        )));
    }

    let datasources_path = datasource_dir.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME);
    if !datasources_path.is_file() {
        return Err(crate::common::message(format!(
            "Snapshot datasource export is missing inventory: {}",
            datasources_path.display()
        )));
    }
    let raw = fs::read_to_string(&datasources_path)?;
    serde_json::from_str(&raw).map_err(|error| {
        crate::common::message(format!(
            "Snapshot datasource inventory must contain valid JSON in {}: {}",
            datasources_path.display(),
            error
        ))
    })
}

fn collect_dashboard_org_counts(
    dashboard_metadata: &Value,
    dashboard_index: &Value,
) -> Result<(Vec<SnapshotReviewOrgCounts>, usize, bool)> {
    let mut rows = Vec::new();
    let mut missing_org_scope = false;
    if let Some(orgs) = dashboard_metadata.get("orgs").and_then(Value::as_array) {
        for org in orgs {
            let org = org.as_object().ok_or_else(|| {
                crate::common::message("Snapshot dashboard export org entry must be a JSON object.")
            })?;
            let org_name = org
                .get("org")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            let org_id = org
                .get("orgId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if org_name.is_empty() && org_id.is_empty() {
                missing_org_scope = true;
            }
            rows.push(SnapshotReviewOrgCounts {
                org: org_name,
                org_id,
                dashboard_count: org
                    .get("dashboardCount")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as usize,
                folder_count: 0,
                datasource_count: 0,
                default_datasource_count: 0,
                datasource_types: BTreeMap::new(),
            });
        }
    } else {
        let org = dashboard_metadata
            .get("org")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let org_id = dashboard_metadata
            .get("orgId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if org.is_empty() && org_id.is_empty() {
            missing_org_scope = true;
        }
        rows.push(SnapshotReviewOrgCounts {
            org,
            org_id,
            dashboard_count: dashboard_metadata
                .get("dashboardCount")
                .and_then(Value::as_u64)
                .unwrap_or_default() as usize,
            folder_count: 0,
            datasource_count: 0,
            default_datasource_count: 0,
            datasource_types: BTreeMap::new(),
        });
    }

    if let Some(folders) = dashboard_index.get("folders").and_then(Value::as_array) {
        for folder in folders {
            let folder = folder.as_object().ok_or_else(|| {
                crate::common::message("Snapshot dashboard folder entry must be a JSON object.")
            })?;
            let org = folder
                .get("org")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            let org_id = folder
                .get("orgId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            let key = snapshot_review_org_key(&org_id, &org);
            if let Some(entry) = rows
                .iter_mut()
                .find(|entry| snapshot_review_org_key(&entry.org_id, &entry.org) == key)
            {
                entry.folder_count += 1;
            }
        }
    }

    let dashboard_count = dashboard_metadata
        .get("dashboardCount")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            rows.iter()
                .map(|row| row.dashboard_count as u64)
                .sum::<u64>()
        }) as usize;
    Ok((rows, dashboard_count, missing_org_scope))
}

fn collect_datasource_org_counts(
    datasource_rows: &[Value],
) -> Result<(Vec<SnapshotReviewOrgCounts>, usize, bool)> {
    let mut rows = BTreeMap::<String, SnapshotReviewOrgCounts>::new();
    let mut missing_org_scope = false;
    for datasource in datasource_rows {
        let datasource = datasource.as_object().ok_or_else(|| {
            crate::common::message("Snapshot datasource inventory entry must be a JSON object.")
        })?;
        let org = datasource
            .get("org")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let org_id = datasource
            .get("orgId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if org.is_empty() && org_id.is_empty() {
            missing_org_scope = true;
        }
        let key = snapshot_review_org_key(&org_id, &org);
        let entry = rows.entry(key).or_insert_with(|| SnapshotReviewOrgCounts {
            org: org.clone(),
            org_id: org_id.clone(),
            dashboard_count: 0,
            folder_count: 0,
            datasource_count: 0,
            default_datasource_count: 0,
            datasource_types: BTreeMap::new(),
        });
        if entry.org.is_empty() && !org.is_empty() {
            entry.org = org.clone();
        }
        if entry.org_id.is_empty() && !org_id.is_empty() {
            entry.org_id = org_id.clone();
        }
        entry.datasource_count += 1;
        if datasource
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                datasource
                    .get("isDefault")
                    .and_then(Value::as_str)
                    .map(|value| value == "true")
                    .unwrap_or(false)
            })
        {
            entry.default_datasource_count += 1;
        }
        let datasource_type = datasource
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        if !datasource_type.is_empty() {
            *entry
                .datasource_types
                .entry(datasource_type.to_string())
                .or_insert(0) += 1;
        }
    }
    Ok((
        rows.into_values().collect(),
        datasource_rows.len(),
        missing_org_scope,
    ))
}

fn merge_snapshot_review_org_counts(
    dashboard_rows: Vec<SnapshotReviewOrgCounts>,
    datasource_rows: Vec<SnapshotReviewOrgCounts>,
) -> Vec<SnapshotReviewOrgCounts> {
    let mut orgs = BTreeMap::<String, SnapshotReviewOrgCounts>::new();
    for row in dashboard_rows {
        let key = snapshot_review_org_key(&row.org_id, &row.org);
        let entry = orgs.entry(key).or_default();
        if entry.org.is_empty() {
            entry.org = row.org.clone();
        }
        if entry.org_id.is_empty() {
            entry.org_id = row.org_id.clone();
        }
        entry.dashboard_count += row.dashboard_count;
        entry.folder_count += row.folder_count;
    }
    for row in datasource_rows {
        let key = snapshot_review_org_key(&row.org_id, &row.org);
        let entry = orgs.entry(key).or_default();
        if entry.org.is_empty() {
            entry.org = row.org.clone();
        }
        if entry.org_id.is_empty() {
            entry.org_id = row.org_id.clone();
        }
        entry.datasource_count += row.datasource_count;
        entry.default_datasource_count += row.default_datasource_count;
        for (datasource_type, count) in row.datasource_types {
            *entry.datasource_types.entry(datasource_type).or_insert(0) += count;
        }
    }
    orgs.into_values().collect()
}

fn build_snapshot_review_warnings(
    dashboard_lane_summary: &Value,
    datasource_lane_summary: &Value,
    dashboard_org_count: usize,
    datasource_org_count: usize,
    dashboard_count: usize,
    datasource_count: usize,
    orgs: &[SnapshotReviewOrgCounts],
    missing_dashboard_org_scope: bool,
    missing_datasource_org_scope: bool,
) -> Vec<Value> {
    let mut warnings = Vec::new();
    if dashboard_org_count != datasource_org_count {
        warnings.push(json!({
            "code": "org-count-mismatch",
            "message": format!(
                "Dashboard export covers {} org(s) while datasource inventory covers {} org(s).",
                dashboard_org_count,
                datasource_org_count
            )
        }));
    }
    if dashboard_count == 0 {
        warnings.push(json!({
            "code": "empty-dashboard-inventory",
            "message": "Dashboard export did not record any dashboards."
        }));
    }
    if datasource_count == 0 {
        warnings.push(json!({
            "code": "empty-datasource-inventory",
            "message": "Datasource inventory did not record any datasources."
        }));
    }
    if missing_dashboard_org_scope {
        warnings.push(json!({
            "code": "dashboard-org-missing-scope",
            "message": "At least one dashboard export org entry is missing org or orgId metadata."
        }));
    }
    if missing_datasource_org_scope {
        warnings.push(json!({
            "code": "datasource-org-missing-scope",
            "message": "At least one datasource inventory row is missing org or orgId metadata."
        }));
    }
    let dashboard_scope_count = dashboard_lane_summary
        .get("scopeCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if dashboard_lane_summary
        .get("rawScopeCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < dashboard_scope_count
    {
        warnings.push(json!({
            "code": "dashboard-raw-lane-missing",
            "message": "At least one dashboard export scope is missing raw/ artifacts."
        }));
    }
    if dashboard_lane_summary
        .get("promptScopeCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < dashboard_scope_count
    {
        warnings.push(json!({
            "code": "dashboard-prompt-lane-missing",
            "message": "At least one dashboard export scope is missing prompt/ artifacts."
        }));
    }
    if dashboard_lane_summary
        .get("provisioningScopeCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < dashboard_scope_count
    {
        warnings.push(json!({
            "code": "dashboard-provisioning-lane-missing",
            "message": "At least one dashboard export scope is missing provisioning/ artifacts."
        }));
    }
    let datasource_inventory_scope_count = datasource_lane_summary
        .get("inventoryExpectedScopeCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if datasource_lane_summary
        .get("inventoryScopeCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < datasource_inventory_scope_count
    {
        warnings.push(json!({
            "code": "datasource-inventory-lane-missing",
            "message": "At least one datasource export scope is missing datasources.json."
        }));
    }
    let datasource_provisioning_scope_count = datasource_lane_summary
        .get("provisioningExpectedScopeCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if datasource_lane_summary
        .get("provisioningScopeCount")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < datasource_provisioning_scope_count
    {
        warnings.push(json!({
            "code": "datasource-provisioning-lane-missing",
            "message": "At least one datasource export scope is missing provisioning/datasources.yaml."
        }));
    }
    for org in orgs {
        if org.dashboard_count == 0 || org.datasource_count == 0 {
            warnings.push(json!({
                "code": "org-partial-coverage",
                "message": format!(
                    "Org {} (orgId={}) has {} dashboard(s) and {} datasource(s).",
                    if org.org.is_empty() { "unknown" } else { &org.org },
                    if org.org_id.is_empty() { "unknown" } else { &org.org_id },
                    org.dashboard_count,
                    org.datasource_count
                )
            }));
        }
    }
    warnings
}

pub fn build_snapshot_review_document(
    dashboard_dir: &Path,
    datasource_inventory_dir: &Path,
    datasource_lane_dir: &Path,
) -> Result<Value> {
    let dashboard_metadata = load_snapshot_dashboard_metadata(dashboard_dir)?;
    let dashboard_index = load_snapshot_dashboard_index(dashboard_dir)?;
    let datasource_rows = load_snapshot_datasource_rows(datasource_inventory_dir)?;
    let dashboard_scope_dirs =
        resolve_dashboard_export_scope_dirs(dashboard_dir, &dashboard_metadata);
    let datasource_scope_dirs = resolve_datasource_export_scope_dirs(datasource_lane_dir);
    let dashboard_lane_summary = build_dashboard_lane_summary(&dashboard_scope_dirs);
    let datasource_lane_summary =
        build_datasource_lane_summary(datasource_lane_dir, &datasource_scope_dirs);
    let (dashboard_org_rows, dashboard_count, missing_dashboard_org_scope) =
        collect_dashboard_org_counts(&dashboard_metadata, &dashboard_index)?;
    let dashboard_org_count = dashboard_org_rows.len();
    let (datasource_org_rows, datasource_count, missing_datasource_org_scope) =
        collect_datasource_org_counts(&datasource_rows)?;
    let datasource_org_count = datasource_org_rows.len();
    let orgs = merge_snapshot_review_org_counts(dashboard_org_rows, datasource_org_rows);
    let folder_rows = dashboard_index
        .get("folders")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let folder_count = folder_rows.len();
    let mut datasource_type_totals = BTreeMap::<String, usize>::new();
    let mut datasource_documents = Vec::new();
    let mut default_datasource_count = 0usize;
    for row in &datasource_rows {
        let object = row.as_object().ok_or_else(|| {
            crate::common::message("Snapshot datasource inventory entry must be a JSON object.")
        })?;
        let datasource_type = object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if !datasource_type.is_empty() {
            *datasource_type_totals
                .entry(datasource_type.clone())
                .or_insert(0) += 1;
        }
        let is_default = object
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                object
                    .get("isDefault")
                    .and_then(Value::as_str)
                    .map(|value| value == "true")
                    .unwrap_or(false)
            });
        if is_default {
            default_datasource_count += 1;
        }
        datasource_documents.push(json!({
            "uid": object.get("uid").and_then(Value::as_str).unwrap_or_default(),
            "name": object.get("name").and_then(Value::as_str).unwrap_or_default(),
            "type": datasource_type,
            "org": object.get("org").and_then(Value::as_str).unwrap_or_default(),
            "orgId": object.get("orgId").and_then(Value::as_str).unwrap_or_default(),
            "url": object.get("url").and_then(Value::as_str).unwrap_or_default(),
            "access": object.get("access").and_then(Value::as_str).unwrap_or_default(),
            "isDefault": is_default,
        }));
    }
    let datasource_type_documents = datasource_type_totals
        .iter()
        .map(|(datasource_type, count)| {
            json!({
                "type": datasource_type,
                "count": count,
            })
        })
        .collect::<Vec<Value>>();
    let warnings = build_snapshot_review_warnings(
        &dashboard_lane_summary,
        &datasource_lane_summary,
        dashboard_org_count,
        datasource_org_count,
        dashboard_count,
        datasource_count,
        &orgs,
        missing_dashboard_org_scope,
        missing_datasource_org_scope,
    );

    Ok(json!({
        "kind": SNAPSHOT_REVIEW_KIND,
        "schemaVersion": SNAPSHOT_REVIEW_SCHEMA_VERSION,
        "summary": {
            "orgCount": orgs.len(),
            "dashboardOrgCount": dashboard_org_count,
            "datasourceOrgCount": datasource_org_count,
            "dashboardCount": dashboard_count,
            "folderCount": folder_count,
            "datasourceCount": datasource_count,
            "datasourceTypeCount": datasource_type_totals.len(),
            "defaultDatasourceCount": default_datasource_count,
        },
        "orgs": orgs.into_iter().map(|org| json!({
            "org": org.org,
            "orgId": org.org_id,
            "dashboardCount": org.dashboard_count,
            "folderCount": org.folder_count,
            "datasourceCount": org.datasource_count,
            "defaultDatasourceCount": org.default_datasource_count,
            "datasourceTypes": org.datasource_types,
        })).collect::<Vec<Value>>(),
        "lanes": {
            "dashboard": dashboard_lane_summary,
            "datasource": datasource_lane_summary,
        },
        "folders": folder_rows,
        "datasourceTypes": datasource_type_documents,
        "datasources": datasource_documents,
        "warnings": warnings,
    }))
}

pub fn render_snapshot_review_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(SNAPSHOT_REVIEW_KIND) {
        return Err(crate::common::message(
            "Snapshot review document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| crate::common::message("Snapshot review document is missing summary."))?;
    let mut lines = vec![
        "Snapshot review".to_string(),
        format!(
            "Org coverage: {} combined org(s), {} dashboard org(s), {} datasource org(s)",
            summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0),
            summary
                .get("dashboardOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("datasourceOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        format!(
            "Totals: {} dashboard(s), {} folder(s), {} datasource(s)",
            summary
                .get("dashboardCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("folderCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("datasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        format!(
            "Datasource profile: {} type(s), {} default datasource(s)",
            summary
                .get("datasourceTypeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("defaultDatasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
    ];
    if let Some(lanes) = document.get("lanes").and_then(Value::as_object) {
        let dashboard = lanes
            .get("dashboard")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let datasource = lanes
            .get("datasource")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        lines.push(format!(
            "Dashboard lanes: raw {}/{}, prompt {}/{}, provisioning {}/{}",
            dashboard
                .get("rawScopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            dashboard
                .get("scopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            dashboard
                .get("promptScopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            dashboard
                .get("scopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            dashboard
                .get("provisioningScopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            dashboard
                .get("scopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ));
        lines.push(format!(
            "Datasource lanes: inventory {}/{}, provisioning {}/{}",
            datasource
                .get("inventoryScopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            datasource
                .get("inventoryExpectedScopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            datasource
                .get("provisioningScopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            datasource
                .get("provisioningExpectedScopeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ));
    }
    let datasource_types = document
        .get("datasourceTypes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !datasource_types.is_empty() {
        let summary_text = datasource_types
            .iter()
            .filter_map(|item| {
                item.as_object().map(|item| {
                    format!(
                        "{}:{}",
                        item.get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown"),
                        item.get("count").and_then(Value::as_u64).unwrap_or(0)
                    )
                })
            })
            .collect::<Vec<String>>()
            .join(", ");
        lines.push(format!("Datasource types: {summary_text}"));
    }
    let warnings = document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if warnings.is_empty() {
        lines.push("Warnings: none".to_string());
    } else {
        lines.push(format!("Warnings: {}", warnings.len()));
        for warning in warnings {
            let warning = warning.as_object().ok_or_else(|| {
                crate::common::message("Snapshot review warning entry must be an object.")
            })?;
            lines.push(format!(
                "- {}: {}",
                warning
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                warning.get("message").and_then(Value::as_str).unwrap_or("")
            ));
        }
    }
    let orgs = document
        .get("orgs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !orgs.is_empty() {
        lines.push(String::new());
        lines.push("# Orgs".to_string());
        for org in orgs {
            let org = org.as_object().ok_or_else(|| {
                crate::common::message("Snapshot review org entry must be an object.")
            })?;
            lines.push(format!(
                "- org={} orgId={} dashboards={} folders={} datasources={} defaults={} types={}",
                org.get("org").and_then(Value::as_str).unwrap_or("unknown"),
                org.get("orgId")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                org.get("dashboardCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                org.get("folderCount").and_then(Value::as_u64).unwrap_or(0),
                org.get("datasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                org.get("defaultDatasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                org.get("datasourceTypes")
                    .and_then(Value::as_object)
                    .map(|types| {
                        types
                            .iter()
                            .map(|(name, count)| {
                                format!("{}:{}", name, count.as_u64().unwrap_or(0))
                            })
                            .collect::<Vec<String>>()
                            .join(",")
                    })
                    .unwrap_or_default(),
            ));
        }
    }
    Ok(lines)
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_snapshot_review_summary_lines(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(SNAPSHOT_REVIEW_KIND) {
        return Err(crate::common::message(
            "Snapshot review document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| crate::common::message("Snapshot review document is missing summary."))?;
    let warnings = document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(vec![
        format!(
            "Org coverage: {} combined org(s), {} dashboard org(s), {} datasource org(s)",
            summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0),
            summary
                .get("dashboardOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("datasourceOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        format!(
            "Totals: {} dashboard(s), {} folder(s), {} datasource(s)",
            summary
                .get("dashboardCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("folderCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("datasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        format!(
            "Datasource profile: {} type(s), {} default datasource(s)",
            summary
                .get("datasourceTypeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("defaultDatasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        if warnings.is_empty() {
            "Warnings: none".to_string()
        } else {
            format!("Warnings: {}", warnings.len())
        },
    ])
}

fn snapshot_review_folder_depth(path: &str) -> usize {
    let separator = if path.contains(" / ") { " / " } else { "/" };
    path.split(separator)
        .filter(|segment| !segment.trim().is_empty())
        .count()
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_snapshot_review_browser_items(document: &Value) -> Result<Vec<BrowserItem>> {
    if document.get("kind").and_then(Value::as_str) != Some(SNAPSHOT_REVIEW_KIND) {
        return Err(crate::common::message(
            "Snapshot review document kind is not supported.",
        ));
    }

    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| crate::common::message("Snapshot review document is missing summary."))?;
    let mut items = vec![BrowserItem {
        kind: "snapshot".to_string(),
        title: "Snapshot summary".to_string(),
        meta: format!(
            "{} org(s)  {} dashboard(s)  {} folder(s)  {} datasource(s)",
            summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0),
            summary
                .get("dashboardCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("folderCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("datasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        details: vec![
            format!(
                "Combined orgs: {}",
                summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0)
            ),
            format!(
                "Dashboard orgs: {}",
                summary
                    .get("dashboardOrgCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Datasource orgs: {}",
                summary
                    .get("datasourceOrgCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Dashboards: {}",
                summary
                    .get("dashboardCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Folders: {}",
                summary
                    .get("folderCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Datasources: {}",
                summary
                    .get("datasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Datasource types: {}",
                summary
                    .get("datasourceTypeCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Default datasources: {}",
                summary
                    .get("defaultDatasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
        ],
    }];

    let warnings = document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for warning in &warnings {
        let warning = warning.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review warning entry must be an object.")
        })?;
        let code = warning
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let message = warning
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        items.push(BrowserItem {
            kind: "warning".to_string(),
            title: code.to_string(),
            meta: message.to_string(),
            details: vec![format!("Code: {}", code), format!("Message: {}", message)],
        });
    }

    if let Some(lanes) = document.get("lanes").and_then(Value::as_object) {
        if let Some(dashboard) = lanes.get("dashboard").and_then(Value::as_object) {
            items.push(BrowserItem {
                kind: "lane".to_string(),
                title: "Dashboard lanes".to_string(),
                meta: format!(
                    "raw {}/{}  prompt {}/{}  provisioning {}/{}",
                    dashboard
                        .get("rawScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("scopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("promptScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("scopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("provisioningScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("scopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                ),
                details: vec![
                    format!(
                        "Raw scopes: {}/{}",
                        dashboard
                            .get("rawScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        dashboard
                            .get("scopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                    format!(
                        "Prompt scopes: {}/{}",
                        dashboard
                            .get("promptScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        dashboard
                            .get("scopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                    format!(
                        "Provisioning scopes: {}/{}",
                        dashboard
                            .get("provisioningScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        dashboard
                            .get("scopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                ],
            });
        }
        if let Some(datasource) = lanes.get("datasource").and_then(Value::as_object) {
            items.push(BrowserItem {
                kind: "lane".to_string(),
                title: "Datasource lanes".to_string(),
                meta: format!(
                    "inventory {}/{}  provisioning {}/{}",
                    datasource
                        .get("inventoryScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    datasource
                        .get("inventoryExpectedScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    datasource
                        .get("provisioningScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    datasource
                        .get("provisioningExpectedScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                ),
                details: vec![
                    format!(
                        "Inventory scopes: {}/{}",
                        datasource
                            .get("inventoryScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        datasource
                            .get("inventoryExpectedScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                    format!(
                        "Provisioning scopes: {}/{}",
                        datasource
                            .get("provisioningScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        datasource
                            .get("provisioningExpectedScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                ],
            });
        }
    }

    let orgs = document
        .get("orgs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for org in orgs {
        let org = org.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review org entry must be an object.")
        })?;
        let org_name = org
            .get("org")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unknown");
        let org_id = org
            .get("orgId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unknown");
        let dashboard_count = org
            .get("dashboardCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let datasource_count = org
            .get("datasourceCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        items.push(BrowserItem {
            kind: "org".to_string(),
            title: org_name.to_string(),
            meta: format!(
                "orgId={}  dashboards={}  folders={}  datasources={}  defaults={}",
                org_id,
                dashboard_count,
                org.get("folderCount").and_then(Value::as_u64).unwrap_or(0),
                datasource_count,
                org.get("defaultDatasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            details: vec![
                format!("Org: {}", org_name),
                format!("Org ID: {}", org_id),
                format!("Dashboards: {}", dashboard_count),
                format!(
                    "Folders: {}",
                    org.get("folderCount").and_then(Value::as_u64).unwrap_or(0)
                ),
                format!("Datasources: {}", datasource_count),
                format!(
                    "Default datasources: {}",
                    org.get("defaultDatasourceCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                ),
                format!(
                    "Datasource types: {}",
                    org.get("datasourceTypes")
                        .and_then(Value::as_object)
                        .map(|types| {
                            types
                                .iter()
                                .map(|(name, count)| {
                                    format!("{}:{}", name, count.as_u64().unwrap_or(0))
                                })
                                .collect::<Vec<String>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| "none".to_string())
                ),
            ],
        });
    }

    let datasource_types = document
        .get("datasourceTypes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for datasource_type in datasource_types {
        let datasource_type = datasource_type.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review datasource type entry must be an object.")
        })?;
        items.push(BrowserItem {
            kind: "datasource-type".to_string(),
            title: datasource_type
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            meta: format!(
                "count={}",
                datasource_type
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            details: vec![
                format!(
                    "Type: {}",
                    datasource_type
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                ),
                format!(
                    "Count: {}",
                    datasource_type
                        .get("count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                ),
            ],
        });
    }

    let datasources = document
        .get("datasources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for datasource in datasources {
        let datasource = datasource.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review datasource entry must be an object.")
        })?;
        items.push(BrowserItem {
            kind: "datasource".to_string(),
            title: datasource
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            meta: format!(
                "{}  org={}  default={}",
                datasource
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                datasource
                    .get("org")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                if datasource
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "true"
                } else {
                    "false"
                }
            ),
            details: vec![
                format!(
                    "Name: {}",
                    datasource
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "UID: {}",
                    datasource
                        .get("uid")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "Type: {}",
                    datasource
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "Org: {} ({})",
                    datasource
                        .get("org")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    datasource
                        .get("orgId")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "URL: {}",
                    datasource
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "Access: {}",
                    datasource
                        .get("access")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "Default: {}",
                    if datasource
                        .get("isDefault")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        "true"
                    } else {
                        "false"
                    }
                ),
            ],
        });
    }

    for folder in document
        .get("folders")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let folder = folder.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review folder entry must be an object.")
        })?;
        let title = folder
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let path = folder
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let org = folder
            .get("org")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let org_id = folder
            .get("orgId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let uid = folder
            .get("uid")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let depth = snapshot_review_folder_depth(path);
        items.push(BrowserItem {
            kind: "folder".to_string(),
            title: title.to_string(),
            meta: format!("depth={} path={} org={} uid={}", depth, path, org, uid),
            details: vec![
                format!("Title: {}", title),
                format!("Depth: {}", depth),
                format!("Path: {}", path),
                format!("Org: {}", org),
                format!("Org ID: {}", org_id),
                format!("UID: {}", uid),
            ],
        });
    }

    Ok(items)
}

#[cfg(feature = "tui")]
fn run_snapshot_review_interactive(document: &Value) -> Result<()> {
    let summary_lines = build_snapshot_review_summary_lines(document)?;
    let items = build_snapshot_review_browser_items(document)?;
    run_interactive_browser("Snapshot review", &summary_lines, &items)
}

fn emit_snapshot_review_output(document: &Value, output: OverviewOutputFormat) -> Result<()> {
    match output {
        OverviewOutputFormat::Table => {
            print_lines(&render_table(
                &[
                    "ROW_KIND", "NAME", "STATUS", "PRIMARY", "BLOCKERS", "WARNINGS", "DETAIL",
                ],
                &build_snapshot_review_tabular_rows(document)?,
            ));
        }
        OverviewOutputFormat::Csv => {
            print_lines(&render_csv(
                &[
                    "row_kind", "name", "status", "primary", "blockers", "warnings", "detail",
                ],
                &build_snapshot_review_tabular_rows(document)?,
            ));
        }
        OverviewOutputFormat::Json => println!("{}", serde_json::to_string_pretty(document)?),
        OverviewOutputFormat::Text => {
            for line in render_snapshot_review_text(document)? {
                println!("{line}");
            }
        }
        OverviewOutputFormat::Yaml => println!("{}", render_yaml(document)?),
        #[cfg(feature = "tui")]
        OverviewOutputFormat::Interactive => {
            run_snapshot_review_interactive(document)?;
        }
    }
    Ok(())
}

fn build_snapshot_review_tabular_rows(document: &Value) -> Result<Vec<Vec<String>>> {
    if document.get("kind").and_then(Value::as_str) != Some(SNAPSHOT_REVIEW_KIND) {
        return Err(crate::common::message(
            "Snapshot review document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| crate::common::message("Snapshot review document is missing summary."))?;
    let mut rows = vec![vec![
        "overall".to_string(),
        "snapshot".to_string(),
        "ready".to_string(),
        summary
            .get("dashboardCount")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        document
            .get("warnings")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0)
            .to_string(),
        summary
            .get("defaultDatasourceCount")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        format!(
            "orgs={} datasources={}",
            summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0),
            summary
                .get("datasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
    ]];
    for org in document
        .get("orgs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let org = org.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review org entry must be an object.")
        })?;
        rows.push(vec![
            "org".to_string(),
            org.get("org")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            org.get("orgId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            org.get("dashboardCount")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .to_string(),
            org.get("folderCount")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .to_string(),
            org.get("datasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .to_string(),
            format!(
                "defaults={}",
                org.get("defaultDatasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
        ]);
    }
    for warning in document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let warning = warning.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review warning entry must be an object.")
        })?;
        rows.push(vec![
            "warning".to_string(),
            warning
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            "warning".to_string(),
            String::new(),
            String::new(),
            String::new(),
            warning
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ]);
    }
    Ok(rows)
}

fn normalize_snapshot_datasource_dir(temp_root: &Path, datasource_dir: &Path) -> Result<PathBuf> {
    let metadata_path = datasource_dir.join(SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Ok(datasource_dir.to_path_buf());
    }

    let metadata: Value = serde_json::from_str(&fs::read_to_string(&metadata_path)?)?;
    let variant = metadata
        .get("variant")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let kind = metadata
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let resource = metadata
        .get("resource")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if kind != SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND
        || resource != "datasource"
        || variant != "all-orgs-root"
    {
        return Ok(datasource_dir.to_path_buf());
    }

    let mut merged = Vec::new();
    let mut seen_rows = BTreeSet::<String>::new();
    let mut append_rows = |rows: Vec<Value>| -> Result<()> {
        for row in rows {
            let key = serde_json::to_string(&row)?;
            if seen_rows.insert(key) {
                merged.push(row);
            }
        }
        Ok(())
    };

    let root_datasources_path = datasource_dir.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME);
    if root_datasources_path.is_file() {
        let rows: Vec<Value> = serde_json::from_str(&fs::read_to_string(&root_datasources_path)?)?;
        append_rows(rows)?;
    }

    let scope_dirs = resolve_datasource_export_scope_dirs(datasource_dir);
    for path in scope_dirs {
        if path == datasource_dir {
            continue;
        }
        let datasources_path = path.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME);
        if !datasources_path.is_file() {
            continue;
        }
        let rows: Vec<Value> = serde_json::from_str(&fs::read_to_string(&datasources_path)?)?;
        append_rows(rows)?;
    }

    let normalized_dir = temp_root.join("snapshot-review-datasources");
    fs::create_dir_all(&normalized_dir)?;
    fs::write(
        normalized_dir.join(SNAPSHOT_DATASOURCE_EXPORT_FILENAME),
        serde_json::to_string_pretty(&merged)?,
    )?;
    fs::write(
        normalized_dir.join(SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION,
            "kind": SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND,
            "variant": "root",
            "resource": "datasource",
            "datasourceCount": merged.len(),
            "datasourcesFile": SNAPSHOT_DATASOURCE_EXPORT_FILENAME,
            "indexFile": "index.json",
            "format": "grafana-datasource-inventory-v1",
        }))?,
    )?;
    Ok(normalized_dir)
}

pub(crate) fn run_snapshot_export_with_handlers<FD, FS>(
    mut args: SnapshotExportArgs,
    mut run_dashboard: FD,
    mut run_datasource: FS,
) -> Result<()>
where
    FD: FnMut(DashboardCliArgs) -> Result<()>,
    FS: FnMut(DatasourceGroupCommand) -> Result<()>,
{
    args.common = materialize_snapshot_common_auth(args.common)?;
    run_dashboard(DashboardCliArgs {
        command: DashboardCommand::Export(build_snapshot_dashboard_export_args(&args)),
    })?;
    run_datasource(DatasourceGroupCommand::Export(
        build_snapshot_datasource_export_args(&args),
    ))?;
    Ok(())
}

pub fn run_snapshot_export(args: SnapshotExportArgs) -> Result<()> {
    run_snapshot_export_with_handlers(
        args,
        dashboard::run_dashboard_cli,
        crate::datasource::run_datasource_cli,
    )
}

pub(crate) fn run_snapshot_review_document_with_handler<FO>(
    args: SnapshotReviewArgs,
    mut run_review: FO,
) -> Result<()>
where
    FO: FnMut(Value) -> Result<()>,
{
    let paths = build_snapshot_paths(&args.input_dir);
    let temp_dir = TempInspectDir::new("snapshot-review")?;
    let datasource_dir = normalize_snapshot_datasource_dir(&temp_dir.path, &paths.datasources)?;
    let document =
        build_snapshot_review_document(&paths.dashboards, &datasource_dir, &paths.datasources)?;
    run_review(document)
}

#[allow(dead_code)]
pub fn run_snapshot_review(args: SnapshotReviewArgs) -> Result<()> {
    let output = if args.interactive {
        OverviewOutputFormat::Interactive
    } else {
        args.output
    };
    run_snapshot_review_document_with_handler(args, move |document| {
        emit_snapshot_review_output(&document, output)
    })
}

pub fn run_snapshot_cli(command: SnapshotCommand) -> Result<()> {
    match command {
        SnapshotCommand::Export(args) => run_snapshot_export(args),
        SnapshotCommand::Review(args) => run_snapshot_review(args),
    }
}

#[cfg(test)]
mod tests {
    use super::materialize_snapshot_common_auth_with_prompt;
    use crate::dashboard::CommonCliArgs;

    fn sample_common_args() -> CommonCliArgs {
        CommonCliArgs {
            profile: Some("prod".to_string()),
            url: "http://grafana.example.com".to_string(),
            api_token: None,
            username: Some("admin".to_string()),
            password: None,
            prompt_password: true,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        }
    }

    #[test]
    fn materialize_snapshot_common_auth_prompts_password_once_and_clears_prompt_flags() {
        let common = sample_common_args();
        let mut password_prompts = 0;
        let mut token_prompts = 0;

        let resolved = materialize_snapshot_common_auth_with_prompt(
            common,
            || {
                password_prompts += 1;
                Ok("secret".to_string())
            },
            || {
                token_prompts += 1;
                Ok("token".to_string())
            },
        )
        .expect("resolved auth");

        assert_eq!(resolved.password.as_deref(), Some("secret"));
        assert!(!resolved.prompt_password);
        assert!(!resolved.prompt_token);
        assert_eq!(password_prompts, 1);
        assert_eq!(token_prompts, 0);
    }
}
