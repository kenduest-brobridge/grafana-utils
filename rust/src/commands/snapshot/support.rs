use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use clap::CommandFactory;
use rpassword::prompt_password;
use serde_json::{json, Map, Value};

use crate::access::{
    self, AccessCliArgs, AccessCommand, CommonCliArgs as AccessCommonCliArgs,
    CommonCliArgsNoOrgId as AccessCommonCliArgsNoOrgId, OrgCommand, OrgExportArgs, Scope,
    ServiceAccountCommand, ServiceAccountExportArgs, TeamCommand, TeamExportArgs, UserCommand,
    UserExportArgs,
};
use crate::common::{GrafanaCliError, Result};
use crate::dashboard::{
    self, CommonCliArgs, DashboardCliArgs, DashboardCommand, ExportArgs as DashboardExportArgs,
    TempInspectDir, EXPORT_METADATA_FILENAME,
};
use crate::datasource::{DatasourceExportArgs, DatasourceGroupCommand};
use crate::overview::{OverviewArgs, OverviewOutputFormat};
use crate::staged_export_scopes::resolve_datasource_export_scope_dirs;

#[derive(Debug, Clone, Default)]
pub(crate) struct SnapshotAccessReviewCounts {
    pub(crate) user_count: usize,
    pub(crate) team_count: usize,
    pub(crate) org_count: usize,
    pub(crate) service_account_count: usize,
}

pub(crate) fn export_scope_kind_from_metadata_value(metadata: &Value) -> &str {
    metadata
        .get("scopeKind")
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            match metadata
                .get("variant")
                .and_then(Value::as_str)
                .unwrap_or_default()
            {
                "all-orgs-root" => "all-orgs-root",
                "root" => "org-root",
                _ => "",
            }
        })
}

fn rewrite_export_scope_kind(metadata_path: &Path, scope_kind: &str) -> Result<()> {
    if !metadata_path.is_file() {
        return Ok(());
    }
    let mut metadata =
        crate::common::load_json_object_file(metadata_path, "Snapshot export metadata")?;
    let object = metadata
        .as_object_mut()
        .ok_or_else(|| crate::common::message("Snapshot export metadata must be a JSON object."))?;
    object.insert(
        "scopeKind".to_string(),
        Value::String(scope_kind.to_string()),
    );
    fs::write(metadata_path, serde_json::to_string_pretty(&metadata)?)?;
    Ok(())
}

fn annotate_snapshot_root_scope_kinds(output_dir: &Path) -> Result<()> {
    let paths = build_snapshot_paths(output_dir);
    rewrite_export_scope_kind(
        &paths.dashboards.join(EXPORT_METADATA_FILENAME),
        "workspace-root",
    )?;
    rewrite_export_scope_kind(
        &paths
            .datasources
            .join(super::SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME),
        "workspace-root",
    )?;
    Ok(())
}

fn snapshot_common_source(common: &CommonCliArgs) -> Map<String, Value> {
    let mut source = Map::from_iter(vec![("url".to_string(), Value::String(common.url.clone()))]);
    if let Some(profile) = &common.profile {
        source.insert("profile".to_string(), Value::String(profile.clone()));
    }
    source
}

fn access_common_from_snapshot(common: &CommonCliArgs) -> AccessCommonCliArgs {
    AccessCommonCliArgs {
        profile: common.profile.clone(),
        url: common.url.clone(),
        api_token: common.api_token.clone(),
        username: common.username.clone(),
        password: common.password.clone(),
        prompt_password: common.prompt_password,
        prompt_token: common.prompt_token,
        org_id: None,
        timeout: common.timeout,
        verify_ssl: common.verify_ssl,
        insecure: false,
        ca_cert: None,
    }
}

fn access_common_no_org_id_from_snapshot(common: &CommonCliArgs) -> AccessCommonCliArgsNoOrgId {
    AccessCommonCliArgsNoOrgId {
        profile: common.profile.clone(),
        url: common.url.clone(),
        api_token: common.api_token.clone(),
        username: common.username.clone(),
        password: common.password.clone(),
        prompt_password: common.prompt_password,
        prompt_token: common.prompt_token,
        timeout: common.timeout,
        verify_ssl: common.verify_ssl,
        insecure: false,
        ca_cert: None,
    }
}

fn build_snapshot_access_user_export_args(args: &super::SnapshotExportArgs) -> UserExportArgs {
    UserExportArgs {
        common: access_common_from_snapshot(&args.common),
        output_dir: build_snapshot_paths(&args.output_dir)
            .access
            .join(super::SNAPSHOT_ACCESS_USERS_DIR),
        overwrite: args.overwrite,
        dry_run: false,
        scope: Scope::Org,
        with_teams: true,
    }
}

fn build_snapshot_access_team_export_args(args: &super::SnapshotExportArgs) -> TeamExportArgs {
    TeamExportArgs {
        common: access_common_from_snapshot(&args.common),
        output_dir: build_snapshot_paths(&args.output_dir)
            .access
            .join(super::SNAPSHOT_ACCESS_TEAMS_DIR),
        overwrite: args.overwrite,
        dry_run: false,
        with_members: true,
    }
}

fn build_snapshot_access_org_export_args(args: &super::SnapshotExportArgs) -> OrgExportArgs {
    OrgExportArgs {
        common: access_common_no_org_id_from_snapshot(&args.common),
        org_id: None,
        output_dir: build_snapshot_paths(&args.output_dir)
            .access
            .join(super::SNAPSHOT_ACCESS_ORGS_DIR),
        overwrite: args.overwrite,
        dry_run: false,
        name: None,
        with_users: true,
    }
}

fn build_snapshot_access_service_account_export_args(
    args: &super::SnapshotExportArgs,
) -> ServiceAccountExportArgs {
    ServiceAccountExportArgs {
        common: access_common_from_snapshot(&args.common),
        output_dir: build_snapshot_paths(&args.output_dir)
            .access
            .join(super::SNAPSHOT_ACCESS_SERVICE_ACCOUNTS_DIR),
        overwrite: args.overwrite,
        dry_run: false,
    }
}

fn load_metadata_count(metadata_path: &Path, count_keys: &[&str]) -> Result<Option<usize>> {
    if !metadata_path.is_file() {
        return Ok(None);
    }
    let metadata = crate::common::load_json_object_file(metadata_path, "Snapshot lane metadata")?;
    let object = metadata
        .as_object()
        .ok_or_else(|| crate::common::message("Snapshot lane metadata must be a JSON object."))?;
    for key in count_keys {
        if let Some(count) = object.get(*key).and_then(Value::as_u64) {
            return Ok(Some(count as usize));
        }
    }
    Ok(None)
}

fn load_snapshot_lane_metadata_summary(
    lane_dir: &Path,
    payload_filename: &str,
    count_keys: &[&str],
    lane_kind: &str,
    lane_resource: &str,
) -> Result<Value> {
    let metadata_path = lane_dir.join(EXPORT_METADATA_FILENAME);
    let payload_path = lane_dir.join(payload_filename);
    let metadata_present = metadata_path.is_file();
    let payload_present = payload_path.is_file();
    let record_count = load_metadata_count(&metadata_path, count_keys)?.unwrap_or_else(|| {
        if payload_present {
            match fs::read_to_string(&payload_path)
                .ok()
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            {
                Some(Value::Array(values)) => values.len(),
                Some(Value::Object(object)) => object
                    .get("records")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or_default(),
                _ => 0,
            }
        } else {
            0
        }
    });
    Ok(json!({
        "present": metadata_present || payload_present,
        "metadataPresent": metadata_present,
        "payloadPresent": payload_present,
        "path": lane_dir.to_string_lossy(),
        "metadataPath": metadata_path.to_string_lossy(),
        "payloadPath": payload_path.to_string_lossy(),
        "kind": lane_kind,
        "resource": lane_resource,
        "recordCount": record_count,
    }))
}

pub(crate) fn build_snapshot_access_lane_summaries(
    output_dir: &Path,
) -> Result<(Value, SnapshotAccessReviewCounts, Vec<Value>)> {
    let access_root = output_dir.join(super::SNAPSHOT_ACCESS_DIR);
    if !access_root.exists() {
        return Ok((
            json!({
                "present": false
            }),
            SnapshotAccessReviewCounts::default(),
            Vec::new(),
        ));
    }

    let users = load_snapshot_lane_metadata_summary(
        &access_root.join(super::SNAPSHOT_ACCESS_USERS_DIR),
        "users.json",
        &["recordCount"],
        access::ACCESS_EXPORT_KIND_USERS,
        "users",
    )?;
    let teams = load_snapshot_lane_metadata_summary(
        &access_root.join(super::SNAPSHOT_ACCESS_TEAMS_DIR),
        "teams.json",
        &["recordCount"],
        access::ACCESS_EXPORT_KIND_TEAMS,
        "teams",
    )?;
    let orgs = load_snapshot_lane_metadata_summary(
        &access_root.join(super::SNAPSHOT_ACCESS_ORGS_DIR),
        "orgs.json",
        &["recordCount"],
        access::ACCESS_EXPORT_KIND_ORGS,
        "orgs",
    )?;
    let service_accounts = load_snapshot_lane_metadata_summary(
        &access_root.join(super::SNAPSHOT_ACCESS_SERVICE_ACCOUNTS_DIR),
        "service-accounts.json",
        &["recordCount"],
        access::ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
        "service-accounts",
    )?;

    let mut warnings = Vec::new();
    for (code, lane, label, payload_name) in [
        ("access-users-lane-missing", &users, "users", "users.json"),
        ("access-teams-lane-missing", &teams, "teams", "teams.json"),
        ("access-orgs-lane-missing", &orgs, "orgs", "orgs.json"),
        (
            "access-service-accounts-lane-missing",
            &service_accounts,
            "service accounts",
            "service-accounts.json",
        ),
    ] {
        let present = lane
            .get("present")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !present {
            warnings.push(json!({
                "code": code,
                "message": format!("At least one access export scope is missing {}.", payload_name)
            }));
            continue;
        }
        let metadata_present = lane
            .get("metadataPresent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let payload_present = lane
            .get("payloadPresent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !metadata_present || !payload_present {
            warnings.push(json!({
                "code": format!("{}-partial", code),
                "message": format!(
                    "Access lane {} is incomplete (metadata={}, payload={}).",
                    label,
                    metadata_present,
                    payload_present
                )
            }));
        }
    }

    let counts = SnapshotAccessReviewCounts {
        user_count: users
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        team_count: teams
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        org_count: orgs.get("recordCount").and_then(Value::as_u64).unwrap_or(0) as usize,
        service_account_count: service_accounts
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
    };

    Ok((
        json!({
            "present": true,
            "users": users,
            "teams": teams,
            "orgs": orgs,
            "serviceAccounts": service_accounts,
        }),
        counts,
        warnings,
    ))
}

pub(crate) fn build_snapshot_root_metadata(
    output_dir: &Path,
    common: &CommonCliArgs,
) -> Result<Value> {
    let paths = build_snapshot_paths(output_dir);
    let dashboard_metadata = load_snapshot_lane_metadata_summary(
        &paths.dashboards,
        "index.json",
        &["dashboardCount"],
        super::ROOT_INDEX_KIND,
        "dashboards",
    )?;
    let datasource_metadata = load_snapshot_lane_metadata_summary(
        &paths.datasources,
        super::SNAPSHOT_DATASOURCE_EXPORT_FILENAME,
        &["datasourceCount"],
        super::SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND,
        "datasources",
    )?;
    let access_root = paths.access.clone();
    let access_users = load_snapshot_lane_metadata_summary(
        &access_root.join(super::SNAPSHOT_ACCESS_USERS_DIR),
        "users.json",
        &["recordCount"],
        access::ACCESS_EXPORT_KIND_USERS,
        "users",
    )?;
    let access_teams = load_snapshot_lane_metadata_summary(
        &access_root.join(super::SNAPSHOT_ACCESS_TEAMS_DIR),
        "teams.json",
        &["recordCount"],
        access::ACCESS_EXPORT_KIND_TEAMS,
        "teams",
    )?;
    let access_orgs = load_snapshot_lane_metadata_summary(
        &access_root.join(super::SNAPSHOT_ACCESS_ORGS_DIR),
        "orgs.json",
        &["recordCount"],
        access::ACCESS_EXPORT_KIND_ORGS,
        "orgs",
    )?;
    let access_service_accounts = load_snapshot_lane_metadata_summary(
        &access_root.join(super::SNAPSHOT_ACCESS_SERVICE_ACCOUNTS_DIR),
        "service-accounts.json",
        &["recordCount"],
        access::ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
        "service-accounts",
    )?;
    let summary = json!({
        "dashboardCount": dashboard_metadata
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "datasourceCount": datasource_metadata
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "accessUserCount": access_users
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "accessTeamCount": access_teams
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "accessOrgCount": access_orgs
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "accessServiceAccountCount": access_service_accounts
            .get("recordCount")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    });
    Ok(json!({
        "kind": "grafana-utils-snapshot-root",
        "schemaVersion": 2,
        "toolVersion": crate::common::tool_version(),
        "capturedAt": DateTime::<Utc>::from(std::time::SystemTime::now()).to_rfc3339(),
        "source": snapshot_common_source(common),
        "paths": {
            "root": output_dir.to_string_lossy(),
            "dashboards": paths.dashboards.to_string_lossy(),
            "datasources": paths.datasources.to_string_lossy(),
            "access": paths.access.to_string_lossy(),
        },
        "summary": summary,
        "lanes": {
            "dashboards": dashboard_metadata,
            "datasources": datasource_metadata,
            "access": {
                "users": access_users,
                "teams": access_teams,
                "orgs": access_orgs,
                "serviceAccounts": access_service_accounts,
            },
        }
    }))
}

pub fn build_snapshot_paths(output_dir: &Path) -> super::SnapshotPaths {
    let access = output_dir.join(super::SNAPSHOT_ACCESS_DIR);
    super::SnapshotPaths {
        dashboards: output_dir.join(super::SNAPSHOT_DASHBOARD_DIR),
        datasources: output_dir.join(super::SNAPSHOT_DATASOURCE_DIR),
        access_users: access.join(super::SNAPSHOT_ACCESS_USERS_DIR),
        access_teams: access.join(super::SNAPSHOT_ACCESS_TEAMS_DIR),
        access_orgs: access.join(super::SNAPSHOT_ACCESS_ORGS_DIR),
        access_service_accounts: access.join(super::SNAPSHOT_ACCESS_SERVICE_ACCOUNTS_DIR),
        access,
        metadata: output_dir.join(super::SNAPSHOT_METADATA_FILENAME),
    }
}

pub fn root_command() -> clap::Command {
    super::SnapshotCliArgs::command()
}

#[allow(dead_code)]
pub fn build_snapshot_overview_args(args: &super::SnapshotReviewArgs) -> OverviewArgs {
    let paths = build_snapshot_paths(&args.input_dir);
    OverviewArgs {
        dashboard_export_dir: Some(paths.dashboards),
        dashboard_provisioning_dir: None,
        datasource_export_dir: Some(paths.datasources),
        datasource_provisioning_file: None,
        access_user_export_dir: Some(paths.access_users),
        access_team_export_dir: Some(paths.access_teams),
        access_org_export_dir: Some(paths.access_orgs),
        access_service_account_export_dir: Some(paths.access_service_accounts),
        desired_file: None,
        source_bundle: None,
        target_inventory: None,
        alert_export_dir: None,
        availability_file: None,
        mapping_file: None,
        output_format: args.output_format,
    }
}

pub fn build_snapshot_dashboard_export_args(
    args: &super::SnapshotExportArgs,
) -> DashboardExportArgs {
    let paths = build_snapshot_paths(&args.output_dir);
    DashboardExportArgs {
        common: args.common.clone(),
        output_dir: paths.dashboards,
        page_size: dashboard::DEFAULT_PAGE_SIZE,
        org_id: None,
        all_orgs: true,
        flat: false,
        overwrite: args.overwrite,
        without_dashboard_raw: false,
        without_dashboard_prompt: false,
        without_dashboard_provisioning: false,
        include_history: false,
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

pub fn build_snapshot_datasource_export_args(
    args: &super::SnapshotExportArgs,
) -> DatasourceExportArgs {
    let paths = build_snapshot_paths(&args.output_dir);
    DatasourceExportArgs {
        common: args.common.clone(),
        output_dir: paths.datasources,
        org_id: None,
        all_orgs: true,
        overwrite: args.overwrite,
        without_datasource_provisioning: false,
        dry_run: false,
    }
}

pub(crate) fn materialize_snapshot_common_auth_with_prompt<F, G>(
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

// Build and execute Access CLI commands for each selected Access snapshot lane.
fn run_snapshot_access_exports_with_handler<FA>(
    args: &super::SnapshotExportArgs,
    selection: &super::SnapshotExportSelection,
    mut run_access: FA,
) -> Result<()>
where
    FA: FnMut(AccessCliArgs) -> Result<()>,
{
    if selection.contains(super::SnapshotExportLane::AccessUsers) {
        run_access(AccessCliArgs {
            command: AccessCommand::User {
                command: UserCommand::Export(build_snapshot_access_user_export_args(args)),
            },
        })?;
    }
    if selection.contains(super::SnapshotExportLane::AccessTeams) {
        run_access(AccessCliArgs {
            command: AccessCommand::Team {
                command: TeamCommand::Export(build_snapshot_access_team_export_args(args)),
            },
        })?;
    }
    if selection.contains(super::SnapshotExportLane::AccessOrgs) {
        run_access(AccessCliArgs {
            command: AccessCommand::Org {
                command: OrgCommand::Export(build_snapshot_access_org_export_args(args)),
            },
        })?;
    }
    if selection.contains(super::SnapshotExportLane::AccessServiceAccounts) {
        run_access(AccessCliArgs {
            command: AccessCommand::ServiceAccount {
                command: ServiceAccountCommand::Export(
                    build_snapshot_access_service_account_export_args(args),
                ),
            },
        })?;
    }
    Ok(())
}

fn write_snapshot_root_metadata_file(args: &super::SnapshotExportArgs) -> Result<()> {
    let metadata_path = build_snapshot_paths(&args.output_dir).metadata;
    let metadata = build_snapshot_root_metadata(&args.output_dir, &args.common)?;
    fs::write(metadata_path, serde_json::to_string_pretty(&metadata)?)?;
    Ok(())
}

// Coordinate snapshot export by delegating each selected lane to its domain handler.
pub(crate) fn run_snapshot_export_selected_with_handlers<FD, FS, FA>(
    args: super::SnapshotExportArgs,
    selection: &super::SnapshotExportSelection,
    mut run_dashboard: FD,
    mut run_datasource: FS,
    mut run_access: FA,
) -> Result<()>
where
    FD: FnMut(DashboardCliArgs) -> Result<()>,
    FS: FnMut(DatasourceGroupCommand) -> Result<()>,
    FA: FnMut(AccessCliArgs) -> Result<()>,
{
    fs::create_dir_all(&args.output_dir)?;
    if selection.contains(super::SnapshotExportLane::Dashboards) {
        run_dashboard(DashboardCliArgs {
            color: args.common.color,
            command: DashboardCommand::Export(build_snapshot_dashboard_export_args(&args)),
        })?;
    }
    if selection.contains(super::SnapshotExportLane::Datasources) {
        run_datasource(DatasourceGroupCommand::Export(
            build_snapshot_datasource_export_args(&args),
        ))?;
    }
    run_snapshot_access_exports_with_handler(&args, selection, &mut run_access)?;
    annotate_snapshot_root_scope_kinds(&args.output_dir)?;
    write_snapshot_root_metadata_file(&args)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn run_snapshot_export_with_handlers<FD, FS, FA>(
    args: super::SnapshotExportArgs,
    mut run_dashboard: FD,
    mut run_datasource: FS,
    mut run_access: FA,
) -> Result<()>
where
    FD: FnMut(DashboardCliArgs) -> Result<()>,
    FS: FnMut(DatasourceGroupCommand) -> Result<()>,
    FA: FnMut(AccessCliArgs) -> Result<()>,
{
    run_snapshot_export_selected_with_handlers(
        args,
        &super::SnapshotExportSelection::all(),
        &mut run_dashboard,
        &mut run_datasource,
        &mut run_access,
    )
}

pub fn run_snapshot_export(args: super::SnapshotExportArgs) -> Result<()> {
    // Snapshot export is cross-domain orchestration:
    // each lane (dashboard/datasource/access) shares auth materialization and a single selection contract.
    let mut args = args;
    args.common = materialize_snapshot_common_auth(args.common)?;
    let selection = if args.prompt {
        match super::prompt_snapshot_export_selection()? {
            Some(selection) => selection,
            None => return Ok(()),
        }
    } else {
        super::SnapshotExportSelection::all()
    };
    run_snapshot_export_selected_with_handlers(
        args,
        &selection,
        dashboard::run_dashboard_cli,
        crate::datasource::run_datasource_cli,
        access::run_access_cli,
    )
}

fn normalize_snapshot_datasource_dir(temp_root: &Path, datasource_dir: &Path) -> Result<PathBuf> {
    let metadata_path = datasource_dir.join(super::SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Ok(datasource_dir.to_path_buf());
    }

    let metadata: Value = serde_json::from_str(&fs::read_to_string(&metadata_path)?)?;
    let kind = metadata
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let resource = metadata
        .get("resource")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if kind != super::SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND
        || resource != "datasource"
        || !matches!(
            export_scope_kind_from_metadata_value(&metadata),
            "all-orgs-root" | "workspace-root"
        )
    {
        return Ok(datasource_dir.to_path_buf());
    }

    let mut merged = Vec::new();
    let mut seen_rows = BTreeMap::<String, ()>::new();
    let mut append_rows = |rows: Vec<Value>| -> Result<()> {
        for row in rows {
            let key = serde_json::to_string(&row)?;
            if seen_rows.insert(key, ()).is_none() {
                merged.push(row);
            }
        }
        Ok(())
    };

    let root_datasources_path = datasource_dir.join(super::SNAPSHOT_DATASOURCE_EXPORT_FILENAME);
    if root_datasources_path.is_file() {
        let rows: Vec<Value> = serde_json::from_str(&fs::read_to_string(&root_datasources_path)?)?;
        append_rows(rows)?;
    }

    let scope_dirs = resolve_datasource_export_scope_dirs(datasource_dir);
    for path in scope_dirs {
        if path == datasource_dir {
            continue;
        }
        let datasources_path = path.join(super::SNAPSHOT_DATASOURCE_EXPORT_FILENAME);
        if !datasources_path.is_file() {
            continue;
        }
        let rows: Vec<Value> = serde_json::from_str(&fs::read_to_string(&datasources_path)?)?;
        append_rows(rows)?;
    }

    let normalized_dir = temp_root.join("snapshot-review-datasources");
    fs::create_dir_all(&normalized_dir)?;
    fs::write(
        normalized_dir.join(super::SNAPSHOT_DATASOURCE_EXPORT_FILENAME),
        serde_json::to_string_pretty(&merged)?,
    )?;
    fs::write(
        normalized_dir.join(super::SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": super::SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION,
            "kind": super::SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND,
            "variant": "root",
            "resource": "datasource",
            "datasourceCount": merged.len(),
            "datasourcesFile": super::SNAPSHOT_DATASOURCE_EXPORT_FILENAME,
            "indexFile": "index.json",
            "format": "grafana-datasource-inventory-v1",
        }))?,
    )?;
    Ok(normalized_dir)
}

// Build normalized snapshot review documents once, then route to a caller-provided renderer.
pub(crate) fn run_snapshot_review_document_with_handler<FO>(
    args: super::SnapshotReviewArgs,
    mut run_review: FO,
) -> Result<()>
where
    FO: FnMut(Value) -> Result<()>,
{
    let paths = build_snapshot_paths(&args.input_dir);
    let temp_dir = TempInspectDir::new("snapshot-review")?;
    let datasource_dir = normalize_snapshot_datasource_dir(&temp_dir.path, &paths.datasources)?;
    let document = super::build_snapshot_review_document(
        &paths.dashboards,
        &datasource_dir,
        &paths.datasources,
    )?;
    run_review(document)
}

#[allow(dead_code)]
pub fn run_snapshot_review(args: super::SnapshotReviewArgs) -> Result<()> {
    // Review path is output-format routing over a prebuilt snapshot document;
    // it intentionally does not mutate source artifacts.
    let output = if args.interactive {
        #[cfg(feature = "tui")]
        {
            OverviewOutputFormat::Interactive
        }
        #[cfg(not(feature = "tui"))]
        {
            OverviewOutputFormat::Text
        }
    } else {
        args.output_format
    };
    run_snapshot_review_document_with_handler(args, move |document| {
        super::snapshot_review::emit_snapshot_review_output(&document, output)
    })
}

pub fn run_snapshot_cli(command: super::SnapshotCommand) -> Result<()> {
    // Snapshot namespace boundary keeps only two concrete commands and delegates each to
    // its dedicated orchestration path.
    match command {
        super::SnapshotCommand::Export(args) => run_snapshot_export(args),
        super::SnapshotCommand::Review(args) => run_snapshot_review(args),
    }
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
        let mut password_prompts = 0usize;
        let mut token_prompts = 0usize;
        let mut prompt_password = || {
            password_prompts += 1;
            Ok("prompted-password".to_string())
        };
        let mut prompt_token = || {
            token_prompts += 1;
            Ok("prompted-token".to_string())
        };

        let common = materialize_snapshot_common_auth_with_prompt(
            sample_common_args(),
            &mut prompt_password,
            &mut prompt_token,
        )
        .unwrap();

        assert_eq!(common.password.as_deref(), Some("prompted-password"));
        assert_eq!(common.api_token, None);
        assert!(!common.prompt_password);
        assert!(!common.prompt_token);
        assert_eq!(password_prompts, 1);
        assert_eq!(token_prompts, 0);
    }
}
