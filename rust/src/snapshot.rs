//! Snapshot wrappers for dashboard and datasource exports plus local review.
//!
//! This module stays thin: it derives the per-domain paths/args for a snapshot
//! export root, then builds a snapshot-native inventory review document from
//! the exported dashboard and datasource metadata.

#[path = "snapshot_review.rs"]
mod snapshot_review;
#[path = "snapshot_support.rs"]
mod snapshot_support;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};
use serde_json::{json, Value};

use crate::common::CliColorChoice;
use crate::common::Result;
use crate::dashboard::{
    CommonCliArgs, EXPORT_METADATA_FILENAME, ROOT_INDEX_KIND, TOOL_SCHEMA_VERSION,
};
use crate::overview::OverviewOutputFormat;
use crate::staged_export_scopes::{
    resolve_dashboard_export_scope_dirs, resolve_datasource_export_scope_dirs,
};

pub use self::snapshot_review::render_snapshot_review_text;
#[cfg(test)]
pub(crate) use self::snapshot_review::{
    build_snapshot_review_browser_items, build_snapshot_review_summary_lines,
};
#[allow(unused_imports)]
#[cfg(any(feature = "tui", test))]
pub use self::snapshot_support::root_command;
pub(crate) use self::snapshot_support::{
    build_snapshot_access_lane_summaries, export_scope_kind_from_metadata_value, run_snapshot_cli,
};
#[cfg(test)]
pub(crate) use self::snapshot_support::{
    build_snapshot_overview_args, build_snapshot_paths, build_snapshot_root_metadata,
    materialize_snapshot_common_auth_with_prompt, run_snapshot_export_with_handlers,
    run_snapshot_review_document_with_handler,
};

pub const SNAPSHOT_DASHBOARD_DIR: &str = "dashboards";
pub const SNAPSHOT_DATASOURCE_DIR: &str = "datasources";
pub const SNAPSHOT_ACCESS_DIR: &str = "access";
pub const SNAPSHOT_ACCESS_USERS_DIR: &str = "users";
pub const SNAPSHOT_ACCESS_TEAMS_DIR: &str = "teams";
pub const SNAPSHOT_ACCESS_ORGS_DIR: &str = "orgs";
pub const SNAPSHOT_ACCESS_SERVICE_ACCOUNTS_DIR: &str = "service-accounts";
pub const SNAPSHOT_DATASOURCE_EXPORT_FILENAME: &str = "datasources.json";
pub const SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
pub const SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND: &str = "grafana-utils-datasource-export-index";
pub const SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION: i64 = 1;
pub const SNAPSHOT_METADATA_FILENAME: &str = "snapshot-metadata.json";
const SNAPSHOT_REVIEW_KIND: &str = "grafana-utils-snapshot-review";
const SNAPSHOT_REVIEW_SCHEMA_VERSION: i64 = 1;
const SNAPSHOT_ROOT_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./snapshot\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./snapshot --overwrite\n\n  grafana-util snapshot review --input-dir ./snapshot --output-format table\n\n  grafana-util snapshot review --input-dir ./snapshot --interactive";
const SNAPSHOT_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./snapshot\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./snapshot --overwrite";
const SNAPSHOT_REVIEW_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot review --input-dir ./snapshot --output-format table\n  grafana-util snapshot review --input-dir ./snapshot --output-format csv\n  grafana-util snapshot review --input-dir ./snapshot --output-format text\n  grafana-util snapshot review --input-dir ./snapshot --output-format json\n  grafana-util snapshot review --input-dir ./snapshot --output-format yaml\n  grafana-util snapshot review --input-dir ./snapshot --interactive";

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
        long = "output-dir",
        default_value = "snapshot",
        help = "Directory to write the snapshot export root into. The live export writes dashboard and datasource bundles under this root."
    )]
    pub output_dir: PathBuf,
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
        conflicts_with = "output_format",
        help = "Shortcut for --output-format interactive."
    )]
    pub interactive: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = OverviewOutputFormat::Text,
        help = SNAPSHOT_REVIEW_OUTPUT_HELP
    )]
    pub output_format: OverviewOutputFormat,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util snapshot",
    about = "Export and review Grafana snapshot inventory bundles.",
    after_help = SNAPSHOT_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
pub struct SnapshotCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
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
    pub access: PathBuf,
    pub access_users: PathBuf,
    pub access_teams: PathBuf,
    pub access_orgs: PathBuf,
    pub access_service_accounts: PathBuf,
    pub metadata: PathBuf,
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
    let metadata = fs::read_to_string(&metadata_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let has_non_root_scopes = scope_dirs.iter().any(|scope| scope != datasource_lane_dir);
    let scope_kind = export_scope_kind_from_metadata_value(&metadata);
    let inventory_scope_dirs: Vec<&PathBuf> =
        if matches!(scope_kind, "all-orgs-root" | "workspace-root") && has_non_root_scopes {
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
    let resource = metadata
        .get("resource")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if kind != SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND
        || schema_version != SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION
        || resource != "datasource"
        || !matches!(
            export_scope_kind_from_metadata_value(&metadata),
            "org-root" | "all-orgs-root" | "workspace-root"
        )
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

#[allow(clippy::too_many_arguments)]
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
    let (access_lane_summary, access_counts, mut access_warnings) =
        build_snapshot_access_lane_summaries(dashboard_dir.parent().unwrap_or(dashboard_dir))?;
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
    let mut warnings = warnings;
    warnings.append(&mut access_warnings);

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
            "accessUserCount": access_counts.user_count,
            "accessTeamCount": access_counts.team_count,
            "accessOrgCount": access_counts.org_count,
            "accessServiceAccountCount": access_counts.service_account_count,
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
            "access": access_lane_summary,
        },
        "folders": folder_rows,
        "datasourceTypes": datasource_type_documents,
        "datasources": datasource_documents,
        "warnings": warnings,
    }))
}

#[cfg(test)]
mod tests {
    use super::materialize_snapshot_common_auth_with_prompt;
    use crate::dashboard::CommonCliArgs;

    fn sample_common_args() -> CommonCliArgs {
        CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
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
