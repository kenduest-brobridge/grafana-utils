//! User access plan resource planner.

use reqwest::Method;
use serde_json::{Map, Value};
use std::path::Path;

use crate::access::cli_defs::{AccessPlanArgs, AccessPlanResource};
use crate::access::render::{map_get_text, normalize_user_row, user_scope_text, value_bool};
use crate::access::user::{
    build_record_diff_fields, build_user_diff_map, build_user_export_records_for_diff,
    list_user_teams_with_request, load_access_import_records, validate_user_scope_auth,
};
use crate::access::{
    build_auth_context, Scope, ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_USER_EXPORT_FILENAME,
};
use crate::common::{load_json_object_file, message, string_field, Result};

use super::{
    build_access_plan_document_from_parts, AccessPlanAction, AccessPlanChange, AccessPlanDocument,
    AccessPlanResourceReport,
};

#[derive(Debug, Clone)]
struct BundleInput {
    records: Vec<Map<String, Value>>,
    metadata: Option<Map<String, Value>>,
}

fn build_action_id(identity: &str) -> String {
    format!("access:user:{identity}")
}

fn build_target_evidence(user: &Map<String, Value>) -> Map<String, Value> {
    let mut target = Map::new();
    for key in [
        "id",
        "login",
        "email",
        "name",
        "orgRole",
        "grafanaAdmin",
        "isExternal",
        "isProvisioned",
        "isExternallySynced",
        "isGrafanaAdminExternallySynced",
        "scope",
        "origin",
        "lastActive",
        "teams",
    ] {
        if let Some(value) = user.get(key) {
            target.insert(key.to_string(), value.clone());
        }
    }
    target
}

fn user_plan_blockers(live: &Map<String, Value>, changed_fields: &[String]) -> Vec<String> {
    let is_external = value_bool(live.get("isExternal")).unwrap_or(false);
    let is_provisioned = value_bool(live.get("isProvisioned")).unwrap_or(false);
    let is_externally_synced = value_bool(live.get("isExternallySynced")).unwrap_or(false);
    let is_admin_externally_synced =
        value_bool(live.get("isGrafanaAdminExternallySynced")).unwrap_or(false);
    let mut blockers = Vec::new();

    let profile_changed = changed_fields
        .iter()
        .any(|field| matches!(field.as_str(), "login" | "email" | "name"));
    if profile_changed && (is_external || is_provisioned) {
        blockers.push(
            "external or provisioned user profile cannot be updated through Grafana user API"
                .to_string(),
        );
    }
    if changed_fields.iter().any(|field| field == "orgRole") && is_externally_synced {
        blockers.push(
            "externally synced user orgRole cannot be updated through Grafana org user API"
                .to_string(),
        );
    }
    if changed_fields.iter().any(|field| field == "grafanaAdmin") && is_admin_externally_synced {
        blockers.push(
            "externally synced grafanaAdmin cannot be updated through Grafana permissions API"
                .to_string(),
        );
    }
    blockers
}

fn normalize_plan_metadata(input_dir: &Path) -> Option<Map<String, Value>> {
    let metadata_path = input_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return None;
    }
    load_json_object_file(&metadata_path, "Access plan metadata")
        .ok()
        .and_then(|value| value.as_object().cloned())
}

fn load_user_bundle(input_dir: &Path) -> Result<BundleInput> {
    let records = load_access_import_records(input_dir, ACCESS_EXPORT_KIND_USERS)?;
    Ok(BundleInput {
        records,
        metadata: normalize_plan_metadata(input_dir),
    })
}

fn plan_user_scope(records: &[Map<String, Value>], metadata: Option<&Map<String, Value>>) -> Scope {
    if let Some(scope) = metadata
        .and_then(|value| value.get("scope"))
        .and_then(Value::as_str)
    {
        if scope.eq_ignore_ascii_case("global") {
            return Scope::Global;
        }
        if scope.eq_ignore_ascii_case("org") {
            return Scope::Org;
        }
    }
    if records.iter().any(|record| {
        matches!(
            record.get("scope").and_then(Value::as_str),
            Some(scope) if scope.eq_ignore_ascii_case("global")
        )
    }) {
        Scope::Global
    } else {
        Scope::Org
    }
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

struct UserActionInput {
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
}

fn build_user_action(input: UserActionInput) -> AccessPlanAction {
    AccessPlanAction {
        action_id: build_action_id(&input.identity),
        domain: "access".to_string(),
        resource_kind: "user".to_string(),
        identity: input.identity,
        scope: input.scope,
        action: input.action.to_string(),
        status: input.status.to_string(),
        changed_fields: input.changed_fields,
        changes: input.changes,
        target: input.target,
        blocked_reason: input.blocked_reason,
        review_hints: input.review_hints,
        source_path: input.source_path,
    }
}

struct UserReportInput {
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
}

fn build_user_report(input: UserReportInput) -> AccessPlanResourceReport {
    AccessPlanResourceReport {
        resource_kind: "user".to_string(),
        source_path: input.source_path,
        bundle_present: true,
        source_count: input.source_count,
        live_count: input.live_count,
        checked: input.checked,
        same: input.same,
        create: input.create,
        update: input.update,
        extra_remote: input.extra_remote,
        delete: input.delete,
        blocked: input.blocked,
        warning: input.warning,
        scope: input.scope,
        notes: vec!["vertical slice: user resource only".to_string()],
    }
}

fn build_access_plan_actions<F>(
    mut request_json: F,
    args: &AccessPlanArgs,
    input_dir: &Path,
    bundle: &BundleInput,
) -> Result<(AccessPlanResourceReport, Vec<AccessPlanAction>)>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if !matches!(args.resource, AccessPlanResource::User) {
        return Err(message(
            "access plan user planner only accepts --resource user.",
        ));
    }

    let scope = plan_user_scope(&bundle.records, bundle.metadata.as_ref());
    let include_teams = bundle.records.iter().any(
        |record| matches!(record.get("teams"), Some(Value::Array(values)) if !values.is_empty()),
    );
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_scope_auth(&scope, include_teams, &auth_mode)?;

    let live_records =
        build_user_export_records_for_diff(&mut request_json, &scope, include_teams)?;
    let mut local_rows = bundle.records.clone();
    let mut live_rows = live_records
        .into_iter()
        .map(|row| normalize_user_row(&row, &scope))
        .collect::<Vec<Map<String, Value>>>();

    if include_teams {
        for row in &mut local_rows {
            let mut teams = row
                .get("teams")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|item| item.as_str().map(str::trim).map(str::to_string))
                .filter(|item| !item.is_empty())
                .collect::<Vec<String>>();
            teams.sort();
            teams.dedup();
            row.insert(
                "teams".to_string(),
                Value::Array(teams.into_iter().map(Value::String).collect()),
            );
        }

        for row in &mut live_rows {
            let user_id = map_get_text(row, "id");
            let mut teams = list_user_teams_with_request(&mut request_json, &user_id)?
                .into_iter()
                .map(|team| string_field(&team, "name", ""))
                .filter(|name: &String| !name.is_empty())
                .collect::<Vec<String>>();
            teams.sort();
            teams.dedup();
            row.insert(
                "teams".to_string(),
                Value::Array(teams.into_iter().map(Value::String).collect()),
            );
        }
    }

    let local_map = build_user_diff_map(&local_rows, &input_dir.to_string_lossy(), include_teams)?;
    let live_map = build_user_diff_map(&live_rows, "Grafana live users", include_teams)?;

    let mut actions = Vec::new();
    let mut checked = 0usize;
    let mut same = 0usize;
    let mut create = 0usize;
    let mut update = 0usize;
    let mut extra_remote = 0usize;
    let mut delete = 0usize;
    let mut blocked = 0usize;
    let mut warning = 0usize;

    let source_path = input_dir.join(ACCESS_USER_EXPORT_FILENAME);
    let source_path = source_path.to_string_lossy().to_string();

    for key in local_map.keys() {
        checked += 1;
        let (identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                create += 1;
                actions.push(build_user_action(UserActionInput {
                    identity: identity.clone(),
                    scope: Some(user_scope_text(&scope).to_string()),
                    source_path: source_path.clone(),
                    action: "would-create",
                    status: "ready",
                    changed_fields: Vec::new(),
                    changes: Vec::new(),
                    target: Some(build_target_evidence(local_payload)),
                    blocked_reason: None,
                    review_hints: Vec::new(),
                }));
            }
            Some((_, live_payload)) => {
                let (changed_fields, changes) = build_change_rows(local_payload, live_payload);
                if changed_fields.is_empty() {
                    same += 1;
                    actions.push(build_user_action(UserActionInput {
                        identity: identity.clone(),
                        scope: Some(user_scope_text(&scope).to_string()),
                        source_path: source_path.clone(),
                        action: "same",
                        status: "same",
                        changed_fields: Vec::new(),
                        changes: Vec::new(),
                        target: Some(build_target_evidence(live_payload)),
                        blocked_reason: None,
                        review_hints: Vec::new(),
                    }));
                } else {
                    let blockers = user_plan_blockers(live_payload, &changed_fields);
                    if blockers.is_empty() {
                        update += 1;
                        warning += 1;
                        actions.push(build_user_action(UserActionInput {
                            identity: identity.clone(),
                            scope: Some(user_scope_text(&scope).to_string()),
                            source_path: source_path.clone(),
                            action: "would-update",
                            status: "warning",
                            changed_fields,
                            changes,
                            target: Some(build_target_evidence(live_payload)),
                            blocked_reason: None,
                            review_hints: vec![
                                "review the live user target before applying".to_string()
                            ],
                        }));
                    } else {
                        blocked += 1;
                        actions.push(build_user_action(UserActionInput {
                            identity: identity.clone(),
                            scope: Some(user_scope_text(&scope).to_string()),
                            source_path: source_path.clone(),
                            action: "blocked",
                            status: "blocked",
                            changed_fields,
                            changes,
                            target: Some(build_target_evidence(live_payload)),
                            blocked_reason: Some(blockers.join("; ")),
                            review_hints: vec![
                                "review the target origin before attempting an update".to_string(),
                            ],
                        }));
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
        let action = if args.prune {
            delete += 1;
            "would-delete"
        } else {
            warning += 1;
            "extra-remote"
        };
        actions.push(build_user_action(UserActionInput {
            identity: identity.clone(),
            scope: Some(user_scope_text(&scope).to_string()),
            source_path: source_path.clone(),
            action,
            status: if args.prune { "ready" } else { "warning" },
            changed_fields: Vec::new(),
            changes: Vec::new(),
            target: Some(build_target_evidence(live_payload)),
            blocked_reason: if args.prune {
                None
            } else {
                Some("use --prune to include delete candidates".to_string())
            },
            review_hints: vec!["remote-only user record".to_string()],
        }));
    }

    let report = build_user_report(UserReportInput {
        source_path,
        source_count: local_map.len(),
        live_count: live_map.len(),
        checked,
        same,
        create,
        update,
        extra_remote,
        delete,
        blocked,
        warning,
        scope: Some(user_scope_text(&scope).to_string()),
    });

    Ok((report, actions))
}

pub(super) fn build_user_access_plan_document<F>(
    request_json: F,
    args: &AccessPlanArgs,
) -> Result<AccessPlanDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let bundle = load_user_bundle(&args.input_dir)?;
    let (resource, actions) =
        build_access_plan_actions(request_json, args, &args.input_dir, &bundle)?;
    let resources = vec![resource];
    Ok(build_access_plan_document_from_parts(
        resources, actions, args.prune,
    ))
}
