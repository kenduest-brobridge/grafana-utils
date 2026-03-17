//! Access team command handlers.
//! Covers team CRUD, membership discovery, and table/list rendering helpers for CLI output.
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::common::{
    load_json_object_file, message, string_field, value_as_object, write_json_file, Result,
};

use super::access_render::{
    format_table, map_get_text, normalize_team_row, render_csv, render_objects_json, scalar_text,
    team_summary_line, team_table_rows, value_bool,
};
use super::access_user::lookup_org_user_by_identity;
use super::{
    request_array, request_object, TeamAddArgs, TeamDiffArgs, TeamExportArgs, TeamImportArgs,
    TeamListArgs, TeamModifyArgs, ACCESS_EXPORT_KIND_TEAMS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_EXPORT_VERSION, ACCESS_TEAM_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};

fn normalize_access_identity(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

type DiffPayload = (String, Map<String, Value>);
type DiffPayloadMap = BTreeMap<String, DiffPayload>;

fn user_id_from_record(record: &Map<String, Value>) -> String {
    let user_id = scalar_text(record.get("userId"));
    if user_id.is_empty() {
        scalar_text(record.get("id"))
    } else {
        user_id
    }
}

fn sorted_membership_union(members: &[String], admins: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut merged = Vec::new();
    for identity in members.iter().chain(admins.iter()) {
        let key = normalize_access_identity(identity);
        if seen.insert(key) {
            merged.push(identity.clone());
        }
    }
    merged
}

fn normalize_team_for_diff(
    record: &Map<String, Value>,
    include_members: bool,
) -> Map<String, Value> {
    let mut members = parse_access_identity_list(record.get("members").unwrap_or(&Value::Null));
    let mut admins = parse_access_identity_list(record.get("admins").unwrap_or(&Value::Null));
    members.sort();
    admins.sort();
    let mut payload = Map::from_iter(vec![
        (
            "name".to_string(),
            Value::String(string_field(record, "name", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(record, "email", "")),
        ),
    ]);
    if include_members {
        payload.insert(
            "members".to_string(),
            Value::Array(members.into_iter().map(Value::String).collect()),
        );
        payload.insert(
            "admins".to_string(),
            Value::Array(admins.into_iter().map(Value::String).collect()),
        );
    } else {
        payload.insert("members".to_string(), Value::Array(Vec::new()));
        payload.insert("admins".to_string(), Value::Array(Vec::new()));
    }
    payload
}

fn build_team_diff_map(
    records: &[Map<String, Value>],
    source: &str,
    include_members: bool,
) -> Result<DiffPayloadMap> {
    let mut indexed = BTreeMap::new();
    for record in records {
        let team_name = string_field(record, "name", "");
        if team_name.trim().is_empty() {
            return Err(message(format!(
                "Team diff record in {} does not include name.",
                source
            )));
        }
        let key = normalize_access_identity(&team_name);
        if indexed.contains_key(&key) {
            return Err(message(format!(
                "Duplicate team name in {}: {}",
                source, team_name
            )));
        }
        let payload = normalize_team_for_diff(record, include_members);
        indexed.insert(key, (team_name, payload));
    }
    Ok(indexed)
}

fn build_team_live_records_for_diff<F>(
    mut request_json: F,
    include_members: bool,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let teams = iter_teams_with_request(&mut request_json, None)?;
    let mut records = Vec::new();
    for team in teams {
        let mut row = Map::from_iter(vec![
            (
                "name".to_string(),
                Value::String(string_field(&team, "name", "")),
            ),
            (
                "email".to_string(),
                Value::String(string_field(&team, "email", "")),
            ),
        ]);
        if include_members {
            let team_id = string_field(&team, "id", "");
            let mut members: Vec<String> = Vec::new();
            let mut admins: Vec<String> = Vec::new();
            if !team_id.is_empty() {
                for member in list_team_members_with_request(&mut request_json, &team_id)? {
                    let identity = team_member_identity(&member);
                    if identity.is_empty() {
                        continue;
                    }
                    if !members.iter().any(|value| {
                        normalize_access_identity(value) == normalize_access_identity(&identity)
                    }) {
                        members.push(identity.clone());
                    }
                    if team_member_is_admin(&member)
                        && !admins.iter().any(|value| {
                            normalize_access_identity(value) == normalize_access_identity(&identity)
                        })
                    {
                        admins.push(identity);
                    }
                }
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
        records.push(row);
    }
    Ok(records)
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

/// Purpose: implementation note.
pub(crate) fn diff_teams_with_request<F>(mut request_json: F, args: &TeamDiffArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let local_records = load_team_import_records(&args.diff_dir, ACCESS_EXPORT_KIND_TEAMS)?;
    let include_members = local_records.iter().any(|record| {
        (match record.get("members") {
            Some(Value::Array(values)) => !values.is_empty(),
            Some(Value::String(text)) => !text.trim().is_empty(),
            _ => false,
        }) || (match record.get("admins") {
            Some(Value::Array(values)) => !values.is_empty(),
            Some(Value::String(text)) => !text.trim().is_empty(),
            _ => false,
        })
    });
    let local_map = build_team_diff_map(
        &local_records,
        &args.diff_dir.to_string_lossy(),
        include_members,
    )?;
    let live_records = build_team_live_records_for_diff(&mut request_json, include_members)?;
    let live_map = build_team_diff_map(&live_records, "Grafana live teams", include_members)?;

    let mut differences = 0usize;
    let mut checked = 0usize;
    for key in local_map.keys() {
        checked += 1;
        let (local_identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                println!("Diff missing-live team {}", local_identity);
                differences += 1;
            }
            Some((_live_identity, live_payload)) => {
                let changed = build_record_diff_fields(local_payload, live_payload);
                if changed.is_empty() {
                    println!("Diff same team {}", local_identity);
                } else {
                    differences += 1;
                    println!(
                        "Diff different team {} fields={}",
                        local_identity,
                        changed.join(",")
                    );
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
        let (_, live_payload) = &live_map[key];
        println!(
            "Diff extra-live team {}",
            map_get_text(live_payload, "name")
        );
    }
    if differences > 0 {
        println!(
            "Diff checked {} team(s); {} difference(s) found.",
            checked, differences
        );
    } else {
        println!("No team differences across {} team(s).", checked);
    }
    Ok(differences)
}

fn build_membership_payloads(members: &[String], admins: &[String]) -> (Vec<String>, Vec<String>) {
    let admin_keys = admins
        .iter()
        .map(|identity| normalize_access_identity(identity))
        .collect::<BTreeSet<_>>();
    let mut regular_members = Vec::new();
    for identity in members {
        if !admin_keys.contains(&normalize_access_identity(identity))
            && !regular_members.contains(identity)
        {
            regular_members.push(identity.clone());
        }
    }
    (regular_members, admins.to_vec())
}

fn parse_access_identity_list(value: &Value) -> Vec<String> {
// Call graph (hierarchy): this function is used in related modules.
// Upstream callers: access_team.rs:normalize_team_for_diff
// Downstream callees: access_team.rs:normalize_access_identity

    value
        .as_array()
        .map(|values| {
            let mut seen = BTreeSet::new();
            values
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|identity| !identity.is_empty())
                .filter_map(|identity| {
                    let lowered = normalize_access_identity(identity);
                    if seen.insert(lowered.clone()) {
                        Some(identity.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_team_import_dry_run_row(
    index: usize,
    identity: &str,
    action: &str,
    detail: &str,
) -> Map<String, Value> {
    Map::from_iter(vec![
        ("index".to_string(), Value::String(index.to_string())),
        ("identity".to_string(), Value::String(identity.to_string())),
        ("action".to_string(), Value::String(action.to_string())),
        ("detail".to_string(), Value::String(detail.to_string())),
    ])
}

fn build_team_import_dry_run_rows(rows: &[Map<String, Value>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                map_get_text(row, "index"),
                map_get_text(row, "identity"),
                map_get_text(row, "action"),
                map_get_text(row, "detail"),
            ]
        })
        .collect()
}

fn validate_team_import_dry_run_output(args: &TeamImportArgs) -> Result<()> {
    if (args.table || args.json) && !args.dry_run {
        return Err(message(
            "--table/--json for team import are only supported with --dry-run.",
        ));
    }
    if args.table && args.json {
        return Err(message(
            "--table and --json cannot be used together for team import.",
        ));
    }
    Ok(())
}

fn assert_not_overwrite(path: &Path, dry_run: bool, overwrite: bool) -> Result<()> {
    if dry_run || !path.exists() || overwrite {
        return Ok(());
    }
    Err(message(format!(
        "Refusing to overwrite existing file: {}. Use --overwrite.",
        path.display()
    )))
}

fn build_team_access_export_metadata(
    source_url: &str,
    source_dir: &Path,
    record_count: usize,
) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ACCESS_EXPORT_KIND_TEAMS.to_string()),
        ),
        (
            "version".to_string(),
            Value::Number((ACCESS_EXPORT_VERSION).into()),
        ),
        (
            "sourceUrl".to_string(),
            Value::String(source_url.to_string()),
        ),
        (
            "recordCount".to_string(),
            Value::Number((record_count as i64).into()),
        ),
        (
            "sourceDir".to_string(),
            Value::String(source_dir.to_string_lossy().to_string()),
        ),
    ])
}

fn load_team_import_records(
    import_dir: &Path,
    expected_kind: &str,
) -> Result<Vec<Map<String, Value>>> {
    let path = import_dir.join(ACCESS_TEAM_EXPORT_FILENAME);
    if !path.is_file() {
        return Err(message(format!(
            "Access import file not found: {}",
            path.display()
        )));
    }

    let raw = fs::read_to_string(&path)?;
    let payload: Value = serde_json::from_str(&raw)?;
    let records = match payload {
        Value::Array(values) => values,
        Value::Object(object) => {
            if let Some(kind) = object.get("kind").and_then(Value::as_str) {
                if kind != expected_kind {
                    return Err(message(format!(
                        "Access import kind mismatch in {}: expected {}, got {}",
                        path.display(),
                        expected_kind,
                        kind
                    )));
                }
            }
            if let Some(version) = object.get("version").and_then(Value::as_i64) {
                if version > ACCESS_EXPORT_VERSION {
                    return Err(message(format!(
                        "Unsupported access import version {} in {}. Supported <= {}.",
                        version,
                        path.display(),
                        ACCESS_EXPORT_VERSION
                    )));
                }
            }
            object
                .get("records")
                .cloned()
                .ok_or_else(|| {
                    message(format!(
                        "Access import bundle is missing records list: {}",
                        path.display()
                    ))
                })?
                .as_array()
                .ok_or_else(|| {
                    message(format!(
                        "Access import records must be a list in {}",
                        path.display()
                    ))
                })?
                .to_vec()
        }
        _ => {
            return Err(message(format!(
                "Unsupported access import payload in {}",
                path.display()
            )))
        }
    };

    let metadata_path = import_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    if metadata_path.is_file() {
        let _metadata = load_json_object_file(&metadata_path, "Access import metadata")?;
    }

    let mut normalized = Vec::new();
    for value in records {
        normalized.push(
            value_as_object(
                &value,
                &format!("Access import entry in {}", path.display()),
            )?
            .clone(),
        );
    }
    Ok(normalized)
}

fn list_teams_with_request<F>(
    mut request_json: F,
    query: Option<&str>,
    page: usize,
    per_page: usize,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let params = vec![
        ("query".to_string(), query.unwrap_or("").to_string()),
        ("page".to_string(), page.to_string()),
        ("perpage".to_string(), per_page.to_string()),
    ];
    let object = request_object(
        &mut request_json,
        Method::GET,
        "/api/teams/search",
        &params,
        None,
        "Unexpected team list response from Grafana.",
    )?;
    match object.get("teams") {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                Ok(value_as_object(value, "Unexpected team list response from Grafana.")?.clone())
            })
            .collect(),
        _ => Err(message("Unexpected team list response from Grafana.")),
    }
}

fn list_team_members_with_request<F>(
    mut request_json: F,
    team_id: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        &format!("/api/teams/{team_id}/members"),
        &[],
        None,
        &format!("Unexpected member list response for Grafana team {team_id}."),
    )
}

fn get_team_with_request<F>(mut request_json: F, team_id: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::GET,
        &format!("/api/teams/{team_id}"),
        &[],
        None,
        &format!("Unexpected team lookup response for Grafana team {team_id}."),
    )
}

fn create_team_with_request<F>(mut request_json: F, payload: &Value) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        "/api/teams",
        &[],
        Some(payload),
        "Unexpected team create response from Grafana.",
    )
}

fn add_team_member_with_request<F>(
    mut request_json: F,
    team_id: &str,
    user_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        &format!("/api/teams/{team_id}/members"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "userId".to_string(),
            Value::String(user_id.to_string()),
        )]))),
        &format!("Unexpected add-member response for Grafana team {team_id}."),
    )
}

fn remove_team_member_with_request<F>(
    mut request_json: F,
    team_id: &str,
    user_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/teams/{team_id}/members/{user_id}"),
        &[],
        None,
        &format!("Unexpected remove-member response for Grafana team {team_id}."),
    )
}

fn update_team_members_with_request<F>(
    mut request_json: F,
    team_id: &str,
    members: Vec<String>,
    admins: Vec<String>,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/teams/{team_id}/members"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![
            (
                "members".to_string(),
                Value::Array(members.into_iter().map(Value::String).collect()),
            ),
            (
                "admins".to_string(),
                Value::Array(admins.into_iter().map(Value::String).collect()),
            ),
        ]))),
        &format!("Unexpected team member update response for Grafana team {team_id}."),
    )
}

/// lookup team by name.
pub(crate) fn lookup_team_by_name<F>(mut request_json: F, name: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let teams = list_teams_with_request(&mut request_json, Some(name), 1, DEFAULT_PAGE_SIZE)?;
    teams
        .into_iter()
        .find(|team| string_field(team, "name", "") == name)
        .ok_or_else(|| message(format!("Grafana team lookup did not find {name}.")))
}

fn iter_teams_with_request<F>(
    mut request_json: F,
    query: Option<&str>,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut teams = Vec::new();
    let mut page = 1usize;
    loop {
        let batch = list_teams_with_request(&mut request_json, query, page, DEFAULT_PAGE_SIZE)?;
        let batch_len = batch.len();
        teams.extend(batch);
        if batch_len < DEFAULT_PAGE_SIZE {
            break;
        }
        page += 1;
    }
    Ok(teams)
}

fn validate_team_modify_args(args: &TeamModifyArgs) -> Result<()> {
    if args.team_id.is_none() && args.name.is_none() {
        return Err(message("Team modify requires one of --team-id or --name."));
    }
    if args.add_member.is_empty()
        && args.remove_member.is_empty()
        && args.add_admin.is_empty()
        && args.remove_admin.is_empty()
    {
        return Err(message(
            "Team modify requires at least one of --add-member, --remove-member, --add-admin, or --remove-admin.",
        ));
    }
    Ok(())
}

fn team_member_identity(member: &Map<String, Value>) -> String {
    let email = string_field(member, "email", "");
    if !email.is_empty() {
        email
    } else {
        string_field(member, "login", "")
    }
}

fn team_member_is_admin(member: &Map<String, Value>) -> bool {
    value_bool(member.get("isAdmin"))
        .unwrap_or_else(|| value_bool(member.get("admin")).unwrap_or(false))
}

fn add_or_remove_member<F>(
    request_json: &mut F,
    team_id: &str,
    identity: &str,
    add: bool,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let user = lookup_org_user_by_identity(&mut *request_json, identity)?;
    let user_id = {
        let found = user_id_from_record(&user);
        if found.is_empty() {
            return Err(message(format!(
                "Team member lookup did not return an id: {identity}"
            )));
        }
        found
    };
    if add {
        let _ = add_team_member_with_request(&mut *request_json, team_id, &user_id)?;
    } else {
        let _ = remove_team_member_with_request(&mut *request_json, team_id, &user_id)?;
    }
    Ok(string_field(
        &user,
        "email",
        &string_field(&user, "login", identity),
    ))
}

/// Purpose: implementation note.
pub(crate) fn list_teams_command_with_request<F>(
    mut request_json: F,
    args: &TeamListArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = list_teams_with_request(
        &mut request_json,
        args.query.as_deref(),
        args.page,
        args.per_page,
    )?
    .into_iter()
    .map(|team| normalize_team_row(&team))
    .collect::<Vec<Map<String, Value>>>();
    if let Some(name) = &args.name {
        rows.retain(|row| map_get_text(row, "name") == *name);
    }
    if args.with_members {
        for row in &mut rows {
            let team_id = map_get_text(row, "id");
            let members = list_team_members_with_request(&mut request_json, &team_id)?
                .into_iter()
                .map(|member| team_member_identity(&member))
                .filter(|identity| !identity.is_empty())
                .map(Value::String)
                .collect::<Vec<Value>>();
            row.insert("members".to_string(), Value::Array(members));
        }
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.csv {
        for line in render_csv(
            &["id", "name", "email", "memberCount", "members"],
            &team_table_rows(&rows),
        ) {
            println!("{line}");
        }
    } else if args.table {
        for line in format_table(
            &["ID", "NAME", "EMAIL", "MEMBER_COUNT", "MEMBERS"],
            &team_table_rows(&rows),
        ) {
            println!("{line}");
        }
        println!();
        println!("Listed {} team(s) at {}", rows.len(), args.common.url);
    } else {
        for row in &rows {
            println!("{}", team_summary_line(row));
        }
        println!();
        println!("Listed {} team(s) at {}", rows.len(), args.common.url);
    }
    Ok(rows.len())
}

fn team_modify_result(
    team_id: &str,
    team_name: &str,
    added_members: Vec<String>,
    removed_members: Vec<String>,
    added_admins: Vec<String>,
    removed_admins: Vec<String>,
    email: String,
) -> Map<String, Value> {
    Map::from_iter(vec![
        ("teamId".to_string(), Value::String(team_id.to_string())),
        ("name".to_string(), Value::String(team_name.to_string())),
        ("email".to_string(), Value::String(email)),
        (
            "addedMembers".to_string(),
            Value::Array(added_members.into_iter().map(Value::String).collect()),
        ),
        (
            "removedMembers".to_string(),
            Value::Array(removed_members.into_iter().map(Value::String).collect()),
        ),
        (
            "addedAdmins".to_string(),
            Value::Array(added_admins.into_iter().map(Value::String).collect()),
        ),
        (
            "removedAdmins".to_string(),
            Value::Array(removed_admins.into_iter().map(Value::String).collect()),
        ),
    ])
}

fn team_modify_summary_line(result: &Map<String, Value>) -> String {
    let mut text = format!(
        "teamId={} name={}",
        map_get_text(result, "teamId"),
        map_get_text(result, "name")
    );
    for key in [
        "addedMembers",
        "removedMembers",
        "addedAdmins",
        "removedAdmins",
    ] {
        let value = map_get_text(result, key);
        if !value.is_empty() {
            let _ = write!(&mut text, " {}={}", key, value);
        }
    }
    text
}

/// modify team with request.
pub(crate) fn modify_team_with_request<F>(
    mut request_json: F,
    args: &TeamModifyArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_team_modify_args(args)?;
    let team = if let Some(team_id) = &args.team_id {
        get_team_with_request(&mut request_json, team_id)?
    } else {
        lookup_team_by_name(&mut request_json, args.name.as_deref().unwrap_or(""))?
    };
    let team_id = scalar_text(team.get("id"));
    let team_name = string_field(&team, "name", "");
    let mut added_members = Vec::new();
    let mut removed_members = Vec::new();
    for identity in &args.add_member {
        added_members.push(add_or_remove_member(
            &mut request_json,
            &team_id,
            identity,
            true,
        )?);
    }
    for identity in &args.remove_member {
        removed_members.push(add_or_remove_member(
            &mut request_json,
            &team_id,
            identity,
            false,
        )?);
    }
    let existing_members = list_team_members_with_request(&mut request_json, &team_id)?;
    let mut member_identities = existing_members
        .iter()
        .map(team_member_identity)
        .collect::<Vec<String>>();
    let mut admin_identities = existing_members
        .iter()
        .filter(|member| team_member_is_admin(member))
        .map(team_member_identity)
        .collect::<Vec<String>>();
    let mut added_admins = Vec::new();
    let mut removed_admins = Vec::new();
    if !args.add_admin.is_empty() || !args.remove_admin.is_empty() {
        for identity in &args.add_admin {
            let user = lookup_org_user_by_identity(&mut request_json, identity)?;
            let resolved = string_field(&user, "email", &string_field(&user, "login", identity));
            if !member_identities.contains(&resolved) {
                member_identities.push(resolved.clone());
            }
            if !admin_identities.contains(&resolved) {
                admin_identities.push(resolved.clone());
                added_admins.push(resolved);
            }
        }
        for identity in &args.remove_admin {
            let user = lookup_org_user_by_identity(&mut request_json, identity)?;
            let resolved = string_field(&user, "email", &string_field(&user, "login", identity));
            if let Some(index) = admin_identities.iter().position(|value| value == &resolved) {
                admin_identities.remove(index);
                removed_admins.push(resolved);
            }
        }
        member_identities.sort();
        member_identities.dedup();
        admin_identities.sort();
        admin_identities.dedup();
        let _ = update_team_members_with_request(
            &mut request_json,
            &team_id,
            member_identities.clone(),
            admin_identities.clone(),
        )?;
    }
    let result = team_modify_result(
        &team_id,
        &team_name,
        added_members,
        removed_members,
        added_admins,
        removed_admins,
        string_field(&team, "email", ""),
    );
    if args.json {
        println!("{}", render_objects_json(&[result])?);
    } else {
        println!("{}", team_modify_summary_line(&result));
    }
    Ok(0)
}

/// Purpose: implementation note.
pub(crate) fn add_team_with_request<F>(mut request_json: F, args: &TeamAddArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut payload = Map::from_iter(vec![("name".to_string(), Value::String(args.name.clone()))]);
    if let Some(email) = &args.email {
        payload.insert("email".to_string(), Value::String(email.clone()));
    }
    let created = create_team_with_request(&mut request_json, &Value::Object(payload))?;
    let team_id = {
        let team_id = scalar_text(created.get("teamId"));
        if team_id.is_empty() {
            scalar_text(created.get("id"))
        } else {
            team_id
        }
    };
    let team = get_team_with_request(&mut request_json, &team_id)?;
    if args.members.is_empty() && args.admins.is_empty() {
        let result = team_modify_result(
            &team_id,
            &string_field(&team, "name", &args.name),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            string_field(&team, "email", args.email.as_deref().unwrap_or("")),
        );
        if args.json {
            println!("{}", render_objects_json(&[result])?);
        } else {
            println!("{}", team_modify_summary_line(&result));
        }
        return Ok(0);
    }

    let modify = TeamModifyArgs {
        common: args.common.clone(),
        team_id: Some(team_id.clone()),
        name: None,
        add_member: args.members.clone(),
        remove_member: Vec::new(),
        add_admin: args.admins.clone(),
        remove_admin: Vec::new(),
        json: true,
    };
    let _ = modify_team_with_request(&mut request_json, &modify)?;
    let result = team_modify_result(
        &team_id,
        &string_field(&team, "name", &args.name),
        args.members.clone(),
        Vec::new(),
        args.admins.clone(),
        Vec::new(),
        string_field(&team, "email", args.email.as_deref().unwrap_or("")),
    );
    if args.json {
        println!("{}", render_objects_json(&[result])?);
    } else {
        println!("{}", team_modify_summary_line(&result));
    }
    Ok(0)
}

/// Purpose: implementation note.
pub(crate) fn export_teams_with_request<F>(
    mut request_json: F,
    args: &TeamExportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let teams = iter_teams_with_request(&mut request_json, None)?;
    let mut records = teams
        .into_iter()
        .map(|team| normalize_team_row(&team))
        .collect::<Vec<Map<String, Value>>>();
    records.sort_by(|left, right| {
        let lhs = map_get_text(left, "name");
        let rhs = map_get_text(right, "name");
        lhs.cmp(&rhs)
            .then_with(|| map_get_text(left, "id").cmp(&map_get_text(right, "id")))
    });

    if args.with_members {
        for row in &mut records {
            let team_id = map_get_text(row, "id");
            let mut members: Vec<String> = Vec::new();
            let mut admins: Vec<String> = Vec::new();
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
                Value::Array(members.iter().cloned().map(Value::String).collect()),
            );
            row.insert(
                "admins".to_string(),
                Value::Array(admins.iter().cloned().map(Value::String).collect()),
            );
        }
    }

    let teams_path = args.export_dir.join(ACCESS_TEAM_EXPORT_FILENAME);
    let metadata_path = args.export_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    assert_not_overwrite(&teams_path, args.dry_run, args.overwrite)?;
    assert_not_overwrite(&metadata_path, args.dry_run, args.overwrite)?;

    if !args.dry_run {
        let payload = Value::Object(Map::from_iter(vec![
            (
                "kind".to_string(),
                Value::String(ACCESS_EXPORT_KIND_TEAMS.to_string()),
            ),
            (
                "version".to_string(),
                Value::Number((ACCESS_EXPORT_VERSION).into()),
            ),
            (
                "records".to_string(),
                Value::Array(records.iter().cloned().map(Value::Object).collect()),
            ),
        ]));
        write_json_file(&teams_path, &payload, args.overwrite)?;
        write_json_file(
            &metadata_path,
            &Value::Object(build_team_access_export_metadata(
                &args.common.url,
                &args.export_dir,
                records.len(),
            )),
            args.overwrite,
        )?;
    }

    let action = if args.dry_run {
        "Would export"
    } else {
        "Exported"
    };
    println!(
        "{} {} team(s) from {} -> {} and {}",
        action,
        records.len(),
        args.common.url,
        teams_path.display(),
        metadata_path.display()
    );

    Ok(records.len())
}

/// Purpose: implementation note.
pub(crate) fn import_teams_with_request<F>(
    mut request_json: F,
    args: &TeamImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_team_import_dry_run_output(args)?;
    let records = load_team_import_records(&args.import_dir, ACCESS_EXPORT_KIND_TEAMS)?;
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut processed = 0usize;
    let mut dry_run_rows: Vec<Map<String, Value>> = Vec::new();
    let is_dry_run_table_or_json = args.dry_run && (args.table || args.json);

    for (index, record) in records.iter().enumerate() {
        processed += 1;
        let team_name = string_field(record, "name", "");
        if team_name.is_empty() {
            return Err(message(format!(
                "Access team import record {} in {} is missing name.",
                index + 1,
                args.import_dir.display()
            )));
        }

        let record_members =
            parse_access_identity_list(record.get("members").unwrap_or(&Value::Null));
        let record_admins =
            parse_access_identity_list(record.get("admins").unwrap_or(&Value::Null));
        let merged_members = sorted_membership_union(&record_members, &record_admins);
        let (regular_members_payload, admin_payload) =
            build_membership_payloads(&record_members, &record_admins);
        let target_keys = merged_members
            .iter()
            .map(|identity| normalize_access_identity(identity))
            .collect::<BTreeSet<String>>();

        let existing = lookup_team_by_name(&mut request_json, &team_name).ok();

        let existing_team_id = existing.as_ref().and_then(|team| {
            let team_id = scalar_text(team.get("teamId"));
            if team_id.is_empty() {
                let id = scalar_text(team.get("id"));
                if id.is_empty() {
                    None
                } else {
                    Some(id)
                }
            } else {
                Some(team_id)
            }
        });

        if existing_team_id.is_none() {
            if args.dry_run {
                if is_dry_run_table_or_json {
                    dry_run_rows.push(build_team_import_dry_run_row(
                        index + 1,
                        &team_name,
                        "create",
                        "would create team",
                    ));
                } else {
                    println!("Would create team {}", team_name);
                }
                created += 1;
                continue;
            }

            let created_team = create_team_with_request(
                &mut request_json,
                &Value::Object(Map::from_iter([
                    ("name".to_string(), Value::String(team_name.clone())),
                    (
                        "email".to_string(),
                        Value::String(string_field(record, "email", "")),
                    ),
                ])),
            )?;
            let team_id = {
                let team_id = scalar_text(created_team.get("teamId"));
                if team_id.is_empty() {
                    scalar_text(created_team.get("id"))
                } else {
                    team_id
                }
            };
            if team_id.is_empty() {
                return Err(message(format!(
                    "Team import did not return team id for {}",
                    team_name
                )));
            }

            if !(record_members.is_empty() && record_admins.is_empty()) {
                for identity in merged_members.iter() {
                    let user = lookup_org_user_by_identity(&mut request_json, identity)?;
                    let user_id = user_id_from_record(&user);
                    if user_id.is_empty() {
                        return Err(message(format!(
                            "Team member lookup did not return an id: {}",
                            identity
                        )));
                    }
                    add_team_member_with_request(&mut request_json, &team_id, &user_id)?;
                }

                if !regular_members_payload.is_empty() || !admin_payload.is_empty() {
                    let _ = update_team_members_with_request(
                        &mut request_json,
                        &team_id,
                        regular_members_payload,
                        admin_payload,
                    )?;
                }
            }

            println!("Created team {}", team_name);
            created += 1;
            continue;
        }

        let team_id = existing_team_id.unwrap();
        if !args.replace_existing {
            skipped += 1;
            if is_dry_run_table_or_json {
                dry_run_rows.push(build_team_import_dry_run_row(
                    index + 1,
                    &team_name,
                    "skip",
                    "existing and --replace-existing was not set.",
                ));
            } else {
                println!("Skipped team {} ({})", team_name, index + 1);
            }
            continue;
        }

        let mut existing_members = BTreeMap::<String, (String, bool, String)>::new();
        for member in list_team_members_with_request(&mut request_json, &team_id)? {
            let identity = team_member_identity(&member);
            if identity.is_empty() {
                continue;
            }
            existing_members.insert(
                normalize_access_identity(&identity),
                (
                    identity,
                    team_member_is_admin(&member),
                    scalar_text(member.get("userId")),
                ),
            );
        }

        let remove_keys: Vec<String> = existing_members
            .keys()
            .filter(|identity| !target_keys.contains(*identity))
            .cloned()
            .collect();
        if !remove_keys.is_empty() && !args.yes {
            return Err(message(format!(
                "Team import would remove team memberships for {}. Add --yes to confirm.",
                team_name
            )));
        }

        if !args.dry_run {
            for identity in record_members
                .iter()
                .chain(record_admins.iter())
                .collect::<Vec<&String>>()
                .iter()
            {
                let key = normalize_access_identity(identity);
                if existing_members.contains_key(&key) {
                    continue;
                }
                let user = lookup_org_user_by_identity(&mut request_json, identity)?;
                let user_id = user_id_from_record(&user);
                if user_id.is_empty() {
                    return Err(message(format!(
                        "Team member lookup did not return an id: {}",
                        identity
                    )));
                }
                add_team_member_with_request(&mut request_json, &team_id, &user_id)?;
                existing_members.insert(key, (identity.to_string(), false, user_id));
            }

            if !remove_keys.is_empty() {
                for key in remove_keys.iter() {
                    if let Some((_, _, user_id)) = existing_members.remove(key.as_str()) {
                        if !user_id.is_empty() {
                            remove_team_member_with_request(&mut request_json, &team_id, &user_id)?;
                        }
                    }
                }
            }

            let _ = update_team_members_with_request(
                &mut request_json,
                &team_id,
                regular_members_payload,
                admin_payload,
            )?;
        }

        if args.dry_run {
            for identity in record_members.iter().chain(record_admins.iter()) {
                let key = normalize_access_identity(identity);
                if !existing_members.contains_key(&key) {
                    if is_dry_run_table_or_json {
                        dry_run_rows.push(build_team_import_dry_run_row(
                            index + 1,
                            &team_name,
                            "add-member",
                            &format!("would add team member {identity}"),
                        ));
                    } else {
                        println!("Would add team {} member {}", team_name, identity);
                    }
                }
            }
            for key in remove_keys.iter() {
                if let Some((identity, _, _)) = existing_members.get(key.as_str()) {
                    if is_dry_run_table_or_json {
                        dry_run_rows.push(build_team_import_dry_run_row(
                            index + 1,
                            &team_name,
                            "remove-member",
                            &format!("would remove team member {identity}"),
                        ));
                    } else {
                        println!("Would remove team {} member {}", team_name, identity);
                    }
                }
            }
        }

        updated += 1;
        if is_dry_run_table_or_json {
            dry_run_rows.push(build_team_import_dry_run_row(
                index + 1,
                &team_name,
                "updated",
                "would update team",
            ));
        } else {
            println!("Updated team {}", team_name);
        }
    }

    if args.dry_run && is_dry_run_table_or_json {
        if args.table {
            for line in format_table(
                &["INDEX", "IDENTITY", "ACTION", "DETAIL"],
                &build_team_import_dry_run_rows(&dry_run_rows),
            ) {
                println!("{line}");
            }
        } else if args.json {
            println!("{}", render_objects_json(&dry_run_rows)?);
        }
    }

    println!(
        "Import summary: processed={} created={} updated={} skipped={} source={}",
        processed,
        created,
        updated,
        skipped,
        args.import_dir.display()
    );
    Ok(processed)
}
