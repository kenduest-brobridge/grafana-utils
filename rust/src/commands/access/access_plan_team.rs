//! Team access plan review helpers.

use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::common::{load_json_object_file, string_field, tool_version, Result};
use crate::review_contract::{
    REVIEW_ACTION_BLOCKED, REVIEW_ACTION_EXTRA_REMOTE, REVIEW_ACTION_SAME,
    REVIEW_ACTION_WOULD_CREATE, REVIEW_ACTION_WOULD_DELETE, REVIEW_ACTION_WOULD_UPDATE,
    REVIEW_HINT_REMOTE_ONLY, REVIEW_STATUS_BLOCKED, REVIEW_STATUS_READY, REVIEW_STATUS_SAME,
    REVIEW_STATUS_WARNING,
};

use super::access_plan::{
    AccessPlanAction, AccessPlanChange, AccessPlanDocument, AccessPlanResourceReport,
    AccessPlanSummary,
};
use super::cli_defs::AccessPlanArgs;
use super::render::{map_get_text, normalize_team_row, value_bool};
use super::team_import_export_diff::{
    build_record_diff_fields, build_team_diff_map, load_team_import_records,
};
use super::team_runtime::{
    iter_teams_with_request, list_team_members_with_request, normalize_access_identity,
    team_member_identity, team_member_is_admin,
};
use super::{
    ACCESS_EXPORT_KIND_TEAMS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_TEAM_EXPORT_FILENAME,
};

const TEAM_PLAN_DOMAIN: &str = "access";
const TEAM_PLAN_RESOURCE_KIND: &str = "team";

fn load_team_plan_metadata(input_dir: &Path) -> Option<Map<String, Value>> {
    let metadata_path = input_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return None;
    }
    load_json_object_file(&metadata_path, "Access plan metadata")
        .ok()
        .and_then(|value| value.as_object().cloned())
}

fn plan_team_scope(metadata: Option<&Map<String, Value>>) -> Option<String> {
    metadata
        .and_then(|value| value.get("scope"))
        .and_then(Value::as_str)
        .map(|scope| scope.to_string())
        .or_else(|| Some("org".to_string()))
}

fn team_has_membership_payload(record: &Map<String, Value>) -> bool {
    ["members", "admins"]
        .iter()
        .any(|key| match record.get(*key) {
            Some(Value::Array(values)) => !values.is_empty(),
            Some(Value::String(text)) => !text.trim().is_empty(),
            _ => false,
        })
}

fn build_action_id(identity: &str) -> String {
    format!("access:team:{identity}")
}

fn build_change_rows(
    local: &Map<String, Value>,
    live: &Map<String, Value>,
) -> (Vec<String>, Vec<AccessPlanChange>) {
    let changed_fields = build_record_diff_fields(local, live);
    let mut changes = Vec::new();
    for field in &changed_fields {
        changes.push(AccessPlanChange {
            field: field.to_string(),
            before: local.get(field).cloned().unwrap_or(Value::Null),
            after: live.get(field).cloned().unwrap_or(Value::Null),
        });
    }
    (changed_fields, changes)
}

fn normalize_string_array(value: Option<&Value>) -> Vec<Value> {
    let mut values = value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.as_str().map(str::trim).map(str::to_string))
        .filter(|item| !item.is_empty())
        .collect::<Vec<String>>();
    values.sort();
    values.dedup();
    values.into_iter().map(Value::String).collect()
}

fn build_team_target_evidence(team: &Map<String, Value>) -> Map<String, Value> {
    let mut target = Map::new();
    for key in [
        "id",
        "uid",
        "name",
        "email",
        "memberCount",
        "isProvisioned",
        "scope",
    ] {
        if let Some(value) = team.get(key) {
            target.insert(key.to_string(), value.clone());
        }
    }
    let members = normalize_string_array(team.get("members"));
    if !members.is_empty() {
        target.insert("members".to_string(), Value::Array(members));
    }
    let admins = normalize_string_array(team.get("admins"));
    if !admins.is_empty() {
        target.insert("admins".to_string(), Value::Array(admins));
    }
    target
}

fn team_plan_blockers(live: &Map<String, Value>, changed_fields: &[String]) -> Vec<String> {
    let mut blockers = Vec::new();
    let is_provisioned = value_bool(live.get("isProvisioned")).unwrap_or(false);
    let membership_changed = changed_fields
        .iter()
        .any(|field| matches!(field.as_str(), "members" | "admins"));
    if is_provisioned && membership_changed {
        blockers.push("provisioned team memberships cannot be changed".to_string());
    }
    blockers
}

fn team_review_hints(team: &Map<String, Value>, changed_fields: &[String]) -> Vec<String> {
    let mut hints = Vec::new();
    let is_provisioned = value_bool(team.get("isProvisioned")).unwrap_or(false);
    let membership_changed = changed_fields
        .iter()
        .any(|field| matches!(field.as_str(), "members" | "admins"));

    if is_provisioned && membership_changed {
        hints.push("review the provisioned team target before changing membership".to_string());
    } else if is_provisioned && !changed_fields.is_empty() {
        hints.push("review the provisioned team target before applying".to_string());
    } else if membership_changed {
        hints.push("review team membership before applying".to_string());
    } else if !changed_fields.is_empty() {
        hints.push("review the live team target before applying".to_string());
    }

    hints
}

fn team_rows_by_key(records: &[Map<String, Value>]) -> BTreeMap<String, Map<String, Value>> {
    let mut indexed = BTreeMap::new();
    for record in records {
        let name = string_field(record, "name", "");
        if name.trim().is_empty() {
            continue;
        }
        indexed.insert(normalize_access_identity(&name), record.clone());
    }
    indexed
}

fn build_live_team_rows<F>(
    mut request_json: F,
    include_members: bool,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = iter_teams_with_request(&mut request_json, None)?
        .into_iter()
        .map(|team| {
            let mut row = normalize_team_row(&team);
            if let Some(value) = team.get("uid") {
                row.insert("uid".to_string(), value.clone());
            }
            if let Some(value) = team.get("isProvisioned") {
                row.insert("isProvisioned".to_string(), value.clone());
            }
            row
        })
        .collect::<Vec<Map<String, Value>>>();
    if include_members {
        for row in &mut rows {
            let team_id = map_get_text(row, "id");
            let mut members = Vec::new();
            let mut admins = Vec::new();
            for member in list_team_members_with_request(&mut request_json, &team_id)? {
                let identity = team_member_identity(&member);
                if identity.is_empty() {
                    continue;
                }
                if team_member_is_admin(&member) {
                    admins.push(identity.clone());
                }
                members.push(identity);
            }
            members.sort();
            members.dedup();
            admins.sort();
            admins.dedup();
            row.insert(
                "members".to_string(),
                Value::Array(members.into_iter().map(Value::String).collect()),
            );
            row.insert(
                "admins".to_string(),
                Value::Array(admins.into_iter().map(Value::String).collect()),
            );
        }
    }
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
fn build_team_action(
    identity: String,
    scope: Option<String>,
    source_path: String,
    action: &'static str,
    status: &'static str,
    changed_fields: Vec<String>,
    changes: Vec<AccessPlanChange>,
    target: Option<Map<String, Value>>,
    blocked_reason: Option<String>,
    review_hints: Vec<String>,
) -> AccessPlanAction {
    AccessPlanAction {
        action_id: build_action_id(&identity),
        domain: TEAM_PLAN_DOMAIN.to_string(),
        resource_kind: TEAM_PLAN_RESOURCE_KIND.to_string(),
        identity,
        scope,
        action: action.to_string(),
        status: status.to_string(),
        changed_fields,
        changes,
        target,
        blocked_reason,
        review_hints,
        source_path,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_team_report(
    source_path: String,
    source_count: usize,
    live_count: usize,
    checked: usize,
    same: usize,
    create: usize,
    update: usize,
    extra_remote: usize,
    delete: usize,
    blocked: usize,
    warning: usize,
    scope: Option<String>,
) -> AccessPlanResourceReport {
    AccessPlanResourceReport {
        resource_kind: TEAM_PLAN_RESOURCE_KIND.to_string(),
        source_path,
        bundle_present: true,
        source_count,
        live_count,
        checked,
        same,
        create,
        update,
        extra_remote,
        delete,
        blocked,
        warning,
        scope,
        notes: vec![
            "vertical slice: team resource only".to_string(),
            "team membership is compared when the bundle contains members or admins".to_string(),
        ],
    }
}

fn sort_actions(actions: &mut [AccessPlanAction]) {
    actions.sort_by(|left, right| {
        left.resource_kind
            .cmp(&right.resource_kind)
            .then_with(|| left.identity.cmp(&right.identity))
            .then_with(|| left.action.cmp(&right.action))
    });
}

pub(crate) fn build_team_access_plan_document<F>(
    mut request_json: F,
    args: &AccessPlanArgs,
    kind: &str,
    schema_version: i64,
) -> Result<AccessPlanDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let metadata = load_team_plan_metadata(&args.input_dir);
    let records = load_team_import_records(&args.input_dir, ACCESS_EXPORT_KIND_TEAMS)?;
    let include_members = records.iter().any(team_has_membership_payload);
    let scope = plan_team_scope(metadata.as_ref());

    let local_map =
        build_team_diff_map(&records, &args.input_dir.to_string_lossy(), include_members)?;
    let local_rows_by_key = team_rows_by_key(&records);
    let live_rows = build_live_team_rows(&mut request_json, include_members)?;
    let live_map = build_team_diff_map(&live_rows, "Grafana live teams", include_members)?;
    let live_rows_by_key = team_rows_by_key(&live_rows);

    let mut actions = Vec::new();
    let mut checked = 0usize;
    let mut same = 0usize;
    let mut create = 0usize;
    let mut update = 0usize;
    let mut extra_remote = 0usize;
    let mut delete = 0usize;
    let mut blocked = 0usize;
    let mut warning = 0usize;

    let source_path = args
        .input_dir
        .join(ACCESS_TEAM_EXPORT_FILENAME)
        .to_string_lossy()
        .to_string();

    for key in local_map.keys() {
        checked += 1;
        let (identity, local_payload) = &local_map[key];
        let local_row = local_rows_by_key.get(key).unwrap_or(local_payload);
        match live_map.get(key) {
            None => {
                create += 1;
                actions.push(build_team_action(
                    identity.clone(),
                    scope.clone(),
                    source_path.clone(),
                    REVIEW_ACTION_WOULD_CREATE,
                    REVIEW_STATUS_READY,
                    Vec::new(),
                    Vec::new(),
                    Some(build_team_target_evidence(local_row)),
                    None,
                    team_review_hints(local_row, &[]),
                ));
            }
            Some((_, live_payload)) => {
                let live_row = live_rows_by_key.get(key).unwrap_or(live_payload);
                let (changed_fields, changes) = build_change_rows(local_payload, live_payload);
                if changed_fields.is_empty() {
                    same += 1;
                    actions.push(build_team_action(
                        identity.clone(),
                        scope.clone(),
                        source_path.clone(),
                        REVIEW_ACTION_SAME,
                        REVIEW_STATUS_SAME,
                        Vec::new(),
                        Vec::new(),
                        Some(build_team_target_evidence(live_row)),
                        None,
                        Vec::new(),
                    ));
                } else {
                    let blockers = team_plan_blockers(live_row, &changed_fields);
                    if blockers.is_empty() {
                        update += 1;
                        warning += 1;
                        actions.push(build_team_action(
                            identity.clone(),
                            scope.clone(),
                            source_path.clone(),
                            REVIEW_ACTION_WOULD_UPDATE,
                            REVIEW_STATUS_WARNING,
                            changed_fields.clone(),
                            changes,
                            Some(build_team_target_evidence(live_row)),
                            None,
                            team_review_hints(live_row, &changed_fields),
                        ));
                    } else {
                        blocked += 1;
                        actions.push(build_team_action(
                            identity.clone(),
                            scope.clone(),
                            source_path.clone(),
                            REVIEW_ACTION_BLOCKED,
                            REVIEW_STATUS_BLOCKED,
                            changed_fields.clone(),
                            changes,
                            Some(build_team_target_evidence(live_row)),
                            Some(blockers.join("; ")),
                            team_review_hints(live_row, &changed_fields),
                        ));
                    }
                }
            }
        }
    }

    for key in live_map.keys() {
        if local_map.contains_key(key) {
            continue;
        }
        checked += 1;
        extra_remote += 1;
        let (identity, live_payload) = &live_map[key];
        let live_row = live_rows_by_key.get(key).unwrap_or(live_payload);
        let action = if args.prune {
            delete += 1;
            REVIEW_ACTION_WOULD_DELETE
        } else {
            warning += 1;
            REVIEW_ACTION_EXTRA_REMOTE
        };
        actions.push(build_team_action(
            identity.clone(),
            scope.clone(),
            source_path.clone(),
            action,
            if args.prune {
                REVIEW_STATUS_READY
            } else {
                REVIEW_STATUS_WARNING
            },
            Vec::new(),
            Vec::new(),
            Some(build_team_target_evidence(live_row)),
            if args.prune {
                None
            } else {
                Some("use --prune to include delete candidates".to_string())
            },
            {
                let mut hints = vec![format!("{REVIEW_HINT_REMOTE_ONLY} team record")];
                if value_bool(live_row.get("isProvisioned")).unwrap_or(false) {
                    hints.push(
                        "team is provisioned; verify delete support before pruning".to_string(),
                    );
                }
                hints
            },
        ));
    }

    sort_actions(&mut actions);
    Ok(AccessPlanDocument {
        kind: kind.to_string(),
        schema_version,
        tool_version: tool_version().to_string(),
        summary: AccessPlanSummary {
            resource_count: 1,
            checked,
            same,
            create,
            update,
            extra_remote,
            delete,
            blocked,
            warning,
            prune: args.prune,
        },
        resources: vec![build_team_report(
            source_path,
            local_map.len(),
            live_map.len(),
            checked,
            same,
            create,
            update,
            extra_remote,
            delete,
            blocked,
            warning,
            scope,
        )],
        actions,
    })
}

#[cfg(test)]
#[path = "access_plan_team_tests.rs"]
mod tests;
