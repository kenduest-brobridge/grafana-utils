//! Access plan review helpers for organization bundles.

use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

use super::access_plan::{AccessPlanAction, AccessPlanChange, AccessPlanResourceReport};
use super::cli_defs::AccessPlanArgs;
use super::org::load_org_import_records;
use super::org::{build_org_diff_map, build_org_live_records_for_diff, build_record_diff_fields};
use crate::access::render::scalar_text;
use crate::common::{load_json_object_file, string_field, Result};
use crate::review_contract::{
    REVIEW_ACTION_EXTRA_REMOTE, REVIEW_ACTION_SAME, REVIEW_ACTION_WOULD_CREATE,
    REVIEW_ACTION_WOULD_DELETE, REVIEW_ACTION_WOULD_UPDATE, REVIEW_HINT_REMOTE_ONLY,
    REVIEW_STATUS_READY, REVIEW_STATUS_SAME, REVIEW_STATUS_WARNING,
};

fn build_action_id(identity: &str) -> String {
    format!("access:org:{identity}")
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

fn normalize_plan_metadata(input_dir: &Path) -> Option<Map<String, Value>> {
    let metadata_path = input_dir.join(super::ACCESS_EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return None;
    }
    load_json_object_file(&metadata_path, "Organization plan metadata")
        .ok()
        .and_then(|value| value.as_object().cloned())
}

fn plan_scope(metadata: Option<&Map<String, Value>>) -> Option<String> {
    metadata
        .and_then(|value| value.get("source"))
        .and_then(Value::as_object)
        .and_then(|source| source.get("orgScope"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn build_target_evidence(org: &Map<String, Value>) -> Map<String, Value> {
    let mut target = Map::new();
    for key in ["id", "name", "userCount", "users"] {
        if let Some(value) = org.get(key) {
            target.insert(key.to_string(), value.clone());
        }
    }
    if !target.contains_key("userCount") {
        if let Some(Value::Array(users)) = org.get("users") {
            target.insert(
                "userCount".to_string(),
                Value::Number((users.len() as i64).into()),
            );
        }
    }
    target
}

fn org_user_identity(user: &Map<String, Value>) -> String {
    let login = string_field(user, "login", "");
    if !login.is_empty() {
        return login;
    }
    let email = string_field(user, "email", "");
    if !email.is_empty() {
        return email;
    }
    scalar_text(user.get("id"))
}

fn normalize_identity(identity: &str) -> String {
    identity.trim().to_ascii_lowercase()
}

fn org_user_role_hint(local: &Map<String, Value>, live: &Map<String, Value>) -> Vec<String> {
    let local_users = local
        .get("users")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let live_users = live
        .get("users")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if local_users.is_empty() && live_users.is_empty() {
        return Vec::new();
    }

    let mut local_index = BTreeMap::new();
    for user in local_users {
        let Some(user) = user.as_object() else {
            continue;
        };
        let identity = normalize_identity(&org_user_identity(user));
        if !identity.is_empty() {
            local_index.insert(identity, user.clone());
        }
    }

    let mut live_index = BTreeMap::new();
    for user in live_users {
        let Some(user) = user.as_object() else {
            continue;
        };
        let identity = normalize_identity(&org_user_identity(user));
        if !identity.is_empty() {
            live_index.insert(identity, user.clone());
        }
    }

    let mut role_changed = false;
    let mut membership_changed = false;
    for key in local_index.keys().chain(live_index.keys()) {
        match (local_index.get(key), live_index.get(key)) {
            (Some(local_user), Some(live_user)) => {
                if local_user.get("orgRole") != live_user.get("orgRole") {
                    role_changed = true;
                }
            }
            _ => {
                membership_changed = true;
            }
        }
    }

    let mut hints = Vec::new();
    if role_changed {
        hints.push("review org user role changes before applying".to_string());
    }
    if membership_changed {
        hints.push(
            "org import reconciles listed users but does not remove extra live users".to_string(),
        );
    }
    hints
}

#[allow(clippy::too_many_arguments)]
fn build_org_action(
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
        domain: "access".to_string(),
        resource_kind: "org".to_string(),
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
fn build_org_report(
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
    include_users: bool,
) -> AccessPlanResourceReport {
    AccessPlanResourceReport {
        resource_kind: "org".to_string(),
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
            "vertical slice: org resource only".to_string(),
            if include_users {
                "listed org memberships are compared when present".to_string()
            } else {
                "membership comparison disabled because bundle has no users arrays".to_string()
            },
        ],
    }
}

pub(crate) fn build_org_access_plan_actions<F>(
    mut request_json: F,
    args: &AccessPlanArgs,
    input_dir: &Path,
) -> Result<(AccessPlanResourceReport, Vec<AccessPlanAction>)>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let metadata = normalize_plan_metadata(input_dir);
    let local_records = load_org_import_records(input_dir)?;
    let include_users = local_records
        .iter()
        .any(|record| record.contains_key("users"));
    let local_map =
        build_org_diff_map(&local_records, &input_dir.to_string_lossy(), include_users)?;
    let live_records = build_org_live_records_for_diff(&mut request_json, include_users)?;
    let live_map = build_org_diff_map(&live_records, "Grafana live orgs", include_users)?;

    let mut actions = Vec::new();
    let mut checked = 0usize;
    let mut same = 0usize;
    let mut create = 0usize;
    let mut update = 0usize;
    let mut extra_remote = 0usize;
    let mut delete = 0usize;
    let blocked = 0usize;
    let mut warning = 0usize;

    let source_path = input_dir.join(super::ACCESS_ORG_EXPORT_FILENAME);
    let source_path = source_path.to_string_lossy().to_string();
    let scope = plan_scope(metadata.as_ref()).or_else(|| Some("global".to_string()));

    for key in local_map.keys() {
        checked += 1;
        let (identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                create += 1;
                let mut review_hints = Vec::new();
                if include_users {
                    if let Some(Value::Array(users)) = local_payload.get("users") {
                        if !users.is_empty() {
                            review_hints
                                .push("listed org memberships will be added on create".to_string());
                        }
                    }
                }
                actions.push(build_org_action(
                    identity.clone(),
                    scope.clone(),
                    source_path.clone(),
                    REVIEW_ACTION_WOULD_CREATE,
                    REVIEW_STATUS_READY,
                    Vec::new(),
                    Vec::new(),
                    Some(build_target_evidence(local_payload)),
                    None,
                    review_hints,
                ));
            }
            Some((_, live_payload)) => {
                let (changed_fields, changes) = build_change_rows(local_payload, live_payload);
                if changed_fields.is_empty() {
                    same += 1;
                    actions.push(build_org_action(
                        identity.clone(),
                        scope.clone(),
                        source_path.clone(),
                        REVIEW_ACTION_SAME,
                        REVIEW_STATUS_SAME,
                        Vec::new(),
                        Vec::new(),
                        Some(build_target_evidence(live_payload)),
                        None,
                        Vec::new(),
                    ));
                } else {
                    update += 1;
                    warning += 1;
                    let mut review_hints = Vec::new();
                    if changed_fields.iter().any(|field| field == "users") {
                        review_hints.extend(org_user_role_hint(local_payload, live_payload));
                        if !review_hints
                            .iter()
                            .any(|hint| hint.contains("org user role changes"))
                        {
                            review_hints
                                .push("review org membership changes before applying".to_string());
                        }
                    }
                    if review_hints.is_empty() {
                        review_hints.push("review the live org target before applying".to_string());
                    }
                    actions.push(build_org_action(
                        identity.clone(),
                        scope.clone(),
                        source_path.clone(),
                        REVIEW_ACTION_WOULD_UPDATE,
                        REVIEW_STATUS_WARNING,
                        changed_fields,
                        changes,
                        Some(build_target_evidence(live_payload)),
                        None,
                        review_hints,
                    ));
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
        if args.prune {
            delete += 1;
            actions.push(build_org_action(
                identity.clone(),
                scope.clone(),
                source_path.clone(),
                REVIEW_ACTION_WOULD_DELETE,
                REVIEW_STATUS_READY,
                Vec::new(),
                Vec::new(),
                Some(build_target_evidence(live_payload)),
                None,
                vec![format!("{REVIEW_HINT_REMOTE_ONLY} org record")],
            ));
        } else {
            warning += 1;
            actions.push(build_org_action(
                identity.clone(),
                scope.clone(),
                source_path.clone(),
                REVIEW_ACTION_EXTRA_REMOTE,
                REVIEW_STATUS_WARNING,
                Vec::new(),
                Vec::new(),
                Some(build_target_evidence(live_payload)),
                Some("use --prune to include delete candidates".to_string()),
                vec![format!("{REVIEW_HINT_REMOTE_ONLY} org record")],
            ));
        }
    }

    let report = build_org_report(
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
        include_users,
    );

    Ok((report, actions))
}
