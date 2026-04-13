use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::common::Result;
use crate::staged_export_scopes::{
    resolve_dashboard_export_scope_dirs, resolve_datasource_export_scope_dirs,
};

use super::snapshot_review_counts::{
    build_snapshot_review_warnings, collect_dashboard_org_counts, collect_datasource_org_counts,
    merge_snapshot_review_org_counts,
};
use super::snapshot_review_lanes::{
    build_dashboard_lane_summary, build_datasource_lane_summary, load_snapshot_dashboard_index,
    load_snapshot_dashboard_metadata, load_snapshot_datasource_rows,
};
use super::snapshot_support::build_snapshot_access_lane_summaries;
use super::{SNAPSHOT_REVIEW_KIND, SNAPSHOT_REVIEW_SCHEMA_VERSION};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotReviewDocument {
    kind: &'static str,
    schema_version: i64,
    summary: SnapshotReviewSummary,
    orgs: Vec<SnapshotReviewOrgDocument>,
    lanes: SnapshotReviewLanes,
    folders: Vec<Value>,
    datasource_types: Vec<SnapshotReviewDatasourceTypeDocument>,
    datasources: Vec<SnapshotReviewDatasourceDocument>,
    warnings: Vec<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotReviewSummary {
    org_count: usize,
    dashboard_org_count: usize,
    datasource_org_count: usize,
    dashboard_count: usize,
    folder_count: usize,
    datasource_count: usize,
    datasource_type_count: usize,
    default_datasource_count: usize,
    access_user_count: usize,
    access_team_count: usize,
    access_org_count: usize,
    access_service_account_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotReviewOrgDocument {
    org: String,
    org_id: String,
    dashboard_count: usize,
    folder_count: usize,
    datasource_count: usize,
    default_datasource_count: usize,
    datasource_types: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
struct SnapshotReviewLanes {
    dashboard: Value,
    datasource: Value,
    access: Value,
}

#[derive(Debug, Serialize)]
struct SnapshotReviewDatasourceTypeDocument {
    #[serde(rename = "type")]
    datasource_type: String,
    count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotReviewDatasourceDocument {
    uid: String,
    name: String,
    #[serde(rename = "type")]
    datasource_type: String,
    org: String,
    org_id: String,
    url: String,
    access: String,
    is_default: bool,
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
        datasource_documents.push(SnapshotReviewDatasourceDocument {
            uid: object
                .get("uid")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            name: object
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            datasource_type,
            org: object
                .get("org")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            org_id: object
                .get("orgId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            url: object
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            access: object
                .get("access")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            is_default,
        });
    }
    let datasource_type_documents = datasource_type_totals
        .iter()
        .map(
            |(datasource_type, count)| SnapshotReviewDatasourceTypeDocument {
                datasource_type: datasource_type.clone(),
                count: *count,
            },
        )
        .collect::<Vec<_>>();
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

    let document = SnapshotReviewDocument {
        kind: SNAPSHOT_REVIEW_KIND,
        schema_version: SNAPSHOT_REVIEW_SCHEMA_VERSION,
        summary: SnapshotReviewSummary {
            org_count: orgs.len(),
            dashboard_org_count,
            datasource_org_count,
            dashboard_count,
            folder_count,
            datasource_count,
            datasource_type_count: datasource_type_totals.len(),
            default_datasource_count,
            access_user_count: access_counts.user_count,
            access_team_count: access_counts.team_count,
            access_org_count: access_counts.org_count,
            access_service_account_count: access_counts.service_account_count,
        },
        orgs: orgs
            .into_iter()
            .map(|org| SnapshotReviewOrgDocument {
                org: org.org,
                org_id: org.org_id,
                dashboard_count: org.dashboard_count,
                folder_count: org.folder_count,
                datasource_count: org.datasource_count,
                default_datasource_count: org.default_datasource_count,
                datasource_types: org.datasource_types,
            })
            .collect(),
        lanes: SnapshotReviewLanes {
            dashboard: dashboard_lane_summary,
            datasource: datasource_lane_summary,
            access: access_lane_summary,
        },
        folders: folder_rows,
        datasource_types: datasource_type_documents,
        datasources: datasource_documents,
        warnings,
    };
    serde_json::to_value(document).map_err(|error| {
        crate::common::message(format!(
            "Snapshot review document serialization failed: {error}"
        ))
    })
}
