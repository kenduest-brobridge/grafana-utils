//! User diff workflow helpers.
#![allow(unused_imports)]

use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    iter_global_users_with_request, list_org_users_with_request, list_user_teams_with_request,
    Scope, UserDiffArgs, ACCESS_EXPORT_KIND_USERS, DEFAULT_PAGE_SIZE,
};
use super::user_workflows_import_export::load_access_import_records;
use super::{normalize_access_identity, parse_access_identity_list, DiffPayload, DiffPayloadMap};
use crate::access::render::{
    access_diff_review_line, access_diff_summary_line, build_access_diff_review_document,
    map_get_text, normalize_org_role, normalize_user_row, scalar_text, value_bool,
};
use crate::common::{message, string_field, Result, SharedDiffSummary};
use serde_json::json;

fn normalize_bool_for_diff(value: Option<&Value>) -> Value {
    match value_bool(value) {
        Some(value) => Value::Bool(value),
        None => Value::Null,
    }
}

fn normalize_user_for_diff(record: &Map<String, Value>, include_teams: bool) -> Map<String, Value> {
    let mut payload = Map::from_iter(vec![
        (
            "login".to_string(),
            Value::String(string_field(record, "login", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(record, "email", "")),
        ),
        (
            "name".to_string(),
            Value::String(string_field(record, "name", "")),
        ),
        (
            "orgRole".to_string(),
            Value::String(normalize_org_role(
                record.get("orgRole").or_else(|| record.get("role")),
            )),
        ),
        (
            "grafanaAdmin".to_string(),
            normalize_bool_for_diff(
                record
                    .get("grafanaAdmin")
                    .or(record.get("isGrafanaAdmin"))
                    .or(record.get("isAdmin")),
            ),
        ),
    ]);
    if include_teams {
        let mut teams = parse_access_identity_list(record.get("teams").unwrap_or(&Value::Null));
        teams.sort();
        payload.insert(
            "teams".to_string(),
            Value::Array(teams.iter().cloned().map(Value::String).collect()),
        );
    } else {
        payload.insert("teams".to_string(), Value::Array(Vec::new()));
    }
    payload
}

fn build_user_diff_map(
    records: &[Map<String, Value>],
    source: &str,
    include_teams: bool,
) -> Result<DiffPayloadMap> {
    let mut indexed = BTreeMap::new();
    for record in records {
        let login = string_field(record, "login", "");
        let email = string_field(record, "email", "");
        let identity = if login.is_empty() { email } else { login };
        if identity.trim().is_empty() {
            return Err(message(format!(
                "User diff record in {} does not include login or email.",
                source
            )));
        }
        let key = normalize_access_identity(&identity);
        if indexed.contains_key(&key) {
            return Err(message(format!(
                "Duplicate user identity in {}: {}",
                source, identity
            )));
        }
        let payload = normalize_user_for_diff(record, include_teams);
        indexed.insert(key, (identity, payload));
    }
    Ok(indexed)
}

fn build_user_export_records_for_diff<F>(
    mut request_json: F,
    scope: &Scope,
    include_teams: bool,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = match scope {
        Scope::Org => list_org_users_with_request(&mut request_json)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Org))
            .collect::<Vec<Map<String, Value>>>(),
        Scope::Global => iter_global_users_with_request(&mut request_json, DEFAULT_PAGE_SIZE)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Global))
            .collect::<Vec<Map<String, Value>>>(),
    };

    if include_teams {
        for row in &mut rows {
            let user_id = map_get_text(row, "id");
            let mut team_names = Vec::new();
            if !user_id.is_empty() {
                team_names = list_user_teams_with_request(&mut request_json, &user_id)?
                    .into_iter()
                    .map(|team| string_field(&team, "name", ""))
                    .filter(|name| !name.is_empty())
                    .collect();
                team_names.sort();
                team_names.dedup();
            }
            row.insert(
                "teams".to_string(),
                Value::Array(team_names.into_iter().map(Value::String).collect()),
            );
        }
    }
    Ok(rows)
}

fn build_record_diff_fields(left: &Map<String, Value>, right: &Map<String, Value>) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for key in left.keys().chain(right.keys()) {
        keys.insert(key.clone());
    }
    let mut changed = Vec::new();
    for key in keys {
        if left.get(&key) != right.get(&key) {
            changed.push(key);
        }
    }
    changed
}

pub(crate) fn build_user_diff_review_document(
    summary: SharedDiffSummary,
    local_source: &str,
    live_source: &str,
    rows: &[Value],
) -> Value {
    build_access_diff_review_document("user", summary, local_source, live_source, rows)
}

/// Purpose: implementation note.
pub(crate) fn diff_users_with_request<F>(mut request_json: F, args: &UserDiffArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let local_records = load_access_import_records(&args.diff_dir, ACCESS_EXPORT_KIND_USERS)?;
    let include_teams = local_records
        .iter()
        .any(|record| match record.get("teams") {
            Some(Value::Array(values)) => !values.is_empty(),
            Some(Value::String(text)) => !text.trim().is_empty(),
            _ => false,
        });
    let local_map = build_user_diff_map(
        &local_records,
        &args.diff_dir.to_string_lossy(),
        include_teams,
    )?;
    let live_records =
        build_user_export_records_for_diff(&mut request_json, &args.scope, include_teams)?;
    let live_map = build_user_diff_map(&live_records, "Grafana live users", include_teams)?;

    let mut differences = 0usize;
    let mut checked = 0usize;
    let mut same = 0usize;
    let mut different = 0usize;
    let mut missing_remote = 0usize;
    let mut extra_remote = 0usize;
    let mut review_rows = Vec::new();
    for key in local_map.keys() {
        checked += 1;
        let (local_identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                println!("Diff missing-live user {}", local_identity);
                differences += 1;
                missing_remote += 1;
                review_rows.push(json!({
                    "status": "missing-live",
                    "identity": local_identity,
                    "localSource": args.diff_dir.to_string_lossy(),
                    "liveSource": "Grafana live users",
                }));
            }
            Some((_live_identity, live_payload)) => {
                let changed = build_record_diff_fields(local_payload, live_payload);
                if changed.is_empty() {
                    println!("Diff same user {}", local_identity);
                    same += 1;
                    review_rows.push(json!({
                        "status": "same",
                        "identity": local_identity,
                        "localSource": args.diff_dir.to_string_lossy(),
                        "liveSource": "Grafana live users",
                    }));
                } else {
                    differences += 1;
                    different += 1;
                    println!(
                        "Diff different user {} fields={}",
                        local_identity,
                        changed.join(",")
                    );
                    review_rows.push(json!({
                        "status": "different",
                        "identity": local_identity,
                        "changedFields": changed,
                        "localSource": args.diff_dir.to_string_lossy(),
                        "liveSource": "Grafana live users",
                    }));
                }
            }
        }
    }

    for key in live_map.keys() {
        if local_map.contains_key(key) {
            continue;
        }
        differences += 1;
        checked += 1;
        extra_remote += 1;
        let (live_identity, _) = &live_map[key];
        println!("Diff extra-live user {}", live_identity);
        review_rows.push(json!({
            "status": "extra-live",
            "identity": live_identity,
            "localSource": args.diff_dir.to_string_lossy(),
            "liveSource": "Grafana live users",
        }));
    }

    let _review_document = build_user_diff_review_document(
        SharedDiffSummary {
            checked,
            same,
            different,
            missing_remote,
            extra_remote,
            ambiguous: 0,
        },
        &args.diff_dir.to_string_lossy(),
        "Grafana live users",
        &review_rows,
    );

    println!(
        "{}",
        access_diff_review_line(
            "user",
            checked,
            differences,
            &args.diff_dir.to_string_lossy(),
            "Grafana live users",
        )
    );

    println!(
        "{}",
        access_diff_summary_line(
            "user",
            checked,
            differences,
            &args.diff_dir.to_string_lossy(),
            "Grafana live users",
        )
    );
    Ok(differences)
}
