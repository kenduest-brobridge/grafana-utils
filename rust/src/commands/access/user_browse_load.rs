//! User browse row loading, normalization, and membership summary helpers.

use std::collections::{BTreeMap, BTreeSet};

use reqwest::Method;
use serde_json::{Map, Value};

use crate::access::render::{
    map_get_text, normalize_org_role, normalize_user_row, paginate_rows, scalar_text,
};
use crate::access::user::{
    annotate_user_account_scope, iter_global_users_with_request, list_org_users_with_request,
    list_user_teams_with_request, load_access_import_records, validate_user_scope_auth,
};
use crate::access::{build_auth_context, request_array, Scope, ACCESS_EXPORT_KIND_USERS};
use crate::common::{message, Result};

use super::super::user_browse_state::{row_kind, row_matches_args, DisplayMode};
use super::super::UserBrowseArgs;

type RawOrgUsers = (String, String, Vec<Map<String, Value>>);

pub(crate) fn load_rows<F>(
    mut request_json: F,
    args: &UserBrowseArgs,
    display_mode: DisplayMode,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.input_dir.is_some() {
        return load_rows_from_input_dir(args);
    }
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_scope_auth(&args.scope, true, &auth_mode)?;
    let mut rows = match (args.scope.clone(), display_mode) {
        (Scope::Global, DisplayMode::OrgMemberships) => {
            load_grouped_org_membership_rows(&mut request_json, args)?
        }
        (Scope::Org, _) => list_org_users_with_request(&mut request_json)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Org))
            .collect::<Vec<_>>(),
        (Scope::Global, _) => {
            iter_global_users_with_request(&mut request_json, args.per_page.max(1))?
                .into_iter()
                .map(|item| normalize_user_row(&item, &Scope::Global))
                .collect::<Vec<_>>()
        }
    };
    if display_mode != DisplayMode::OrgMemberships {
        for row in &mut rows {
            let user_id = map_get_text(row, "id");
            let team_records = list_user_teams_with_request(&mut request_json, &user_id)?;
            let teams = team_records
                .iter()
                .map(|team| crate::common::string_field(team, "name", ""))
                .filter(|name| !name.is_empty())
                .map(Value::String)
                .collect::<Vec<_>>();
            let team_rows = team_records
                .into_iter()
                .map(|team| {
                    let team_id = {
                        let value = scalar_text(team.get("teamId"));
                        if value.is_empty() {
                            scalar_text(team.get("id"))
                        } else {
                            value
                        }
                    };
                    Value::Object(Map::from_iter(vec![
                        ("teamId".to_string(), Value::String(team_id)),
                        (
                            "teamName".to_string(),
                            Value::String(crate::common::string_field(&team, "name", "")),
                        ),
                    ]))
                })
                .collect::<Vec<_>>();
            row.insert("teams".to_string(), Value::Array(teams));
            row.insert("teamRows".to_string(), Value::Array(team_rows));
            row.insert("rowKind".to_string(), Value::String("user".to_string()));
        }
        if args.scope == Scope::Global {
            annotate_global_membership_summaries(&mut request_json, &mut rows)?;
        }
    }
    for row in &mut rows {
        if row_kind(row) != "org" {
            annotate_user_account_scope(std::slice::from_mut(row));
        }
    }
    rows.retain(|row| row_matches_args(row, args));
    Ok(paginate_rows(&rows, args.page, args.per_page))
}

fn local_user_scope(row: &Map<String, Value>, args: &UserBrowseArgs) -> Scope {
    match scalar_text(row.get("scope")).to_ascii_lowercase().as_str() {
        "global" => Scope::Global,
        "org" => Scope::Org,
        _ => args.scope.clone(),
    }
}

fn load_rows_from_input_dir(args: &UserBrowseArgs) -> Result<Vec<Map<String, Value>>> {
    let input_dir = args
        .input_dir
        .as_ref()
        .ok_or_else(|| message("User browse local mode requires --input-dir."))?;
    let mut rows = load_access_import_records(input_dir, ACCESS_EXPORT_KIND_USERS)?
        .into_iter()
        .map(|item| {
            let scope = local_user_scope(&item, args);
            normalize_user_row(&item, &scope)
        })
        .collect::<Vec<Map<String, Value>>>();
    annotate_user_account_scope(&mut rows);
    rows.retain(|row| row_matches_args(row, args));
    Ok(paginate_rows(&rows, args.page, args.per_page))
}

fn annotate_global_membership_summaries<F>(
    request_json: &mut F,
    rows: &mut [Map<String, Value>],
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let orgs = request_array(
        &mut *request_json,
        Method::GET,
        "/api/orgs",
        &[],
        None,
        "Unexpected organization list response from Grafana.",
    )?;
    let mut summaries = BTreeMap::<String, Vec<String>>::new();
    let mut roles = BTreeMap::<String, BTreeSet<String>>::new();
    let mut org_counts = BTreeMap::<String, usize>::new();
    for org in orgs {
        let org_id = scalar_text(org.get("id"));
        let org_name = crate::common::string_field(&org, "name", "");
        let users = request_array(
            &mut *request_json,
            Method::GET,
            &format!("/api/orgs/{org_id}/users"),
            &[],
            None,
            &format!("Unexpected organization user list response for Grafana org {org_id}."),
        )?;
        for user in users {
            let user_id = {
                let value = scalar_text(user.get("userId"));
                if value.is_empty() {
                    scalar_text(user.get("id"))
                } else {
                    value
                }
            };
            let org_role = normalize_org_role(user.get("role").or_else(|| user.get("orgRole")));
            let summary_role = if org_role.is_empty() {
                "Unknown".to_string()
            } else {
                org_role.clone()
            };
            summaries
                .entry(user_id.clone())
                .or_default()
                .push(format!("{org_name}: {summary_role}"));
            if !org_role.is_empty() {
                roles.entry(user_id.clone()).or_default().insert(org_role);
            }
            *org_counts.entry(user_id).or_default() += 1;
        }
    }
    for row in rows {
        let user_id = map_get_text(row, "id");
        row.insert(
            "crossOrgMemberships".to_string(),
            Value::String(
                summaries
                    .get(&user_id)
                    .cloned()
                    .unwrap_or_default()
                    .join(" | "),
            ),
        );
        row.insert(
            "roleSummary".to_string(),
            Value::String(
                roles
                    .get(&user_id)
                    .map(|set| set.iter().cloned().collect::<Vec<_>>().join("/"))
                    .unwrap_or_default(),
            ),
        );
        row.insert(
            "orgMembershipCount".to_string(),
            Value::String(org_counts.get(&user_id).copied().unwrap_or(0).to_string()),
        );
    }
    Ok(())
}

fn load_grouped_org_membership_rows<F>(
    request_json: &mut F,
    args: &UserBrowseArgs,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let global_users = iter_global_users_with_request(&mut *request_json, args.per_page.max(1))?;
    let global_by_id = global_users
        .into_iter()
        .map(|user| {
            let normalized = normalize_user_row(&user, &Scope::Global);
            (map_get_text(&normalized, "id"), normalized)
        })
        .collect::<BTreeMap<_, _>>();

    let orgs = request_array(
        &mut *request_json,
        Method::GET,
        "/api/orgs",
        &[],
        None,
        "Unexpected organization list response from Grafana.",
    )?;

    let mut raw_orgs: Vec<RawOrgUsers> = Vec::new();
    let mut summaries: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for org in orgs {
        let org_id = scalar_text(org.get("id"));
        let org_name = crate::common::string_field(&org, "name", "");
        let users = request_array(
            &mut *request_json,
            Method::GET,
            &format!("/api/orgs/{org_id}/users"),
            &[],
            None,
            &format!("Unexpected organization user list response for Grafana org {org_id}."),
        )?;
        let mut membership_rows = Vec::new();
        for user in users {
            let user_id = {
                let value = scalar_text(user.get("userId"));
                if value.is_empty() {
                    scalar_text(user.get("id"))
                } else {
                    value
                }
            };
            let global = global_by_id.get(&user_id);
            let login = global
                .map(|row| map_get_text(row, "login"))
                .unwrap_or_else(|| crate::common::string_field(&user, "login", ""));
            let email = global
                .map(|row| map_get_text(row, "email"))
                .unwrap_or_else(|| crate::common::string_field(&user, "email", ""));
            let name = global
                .map(|row| map_get_text(row, "name"))
                .unwrap_or_else(|| crate::common::string_field(&user, "name", ""));
            let grafana_admin = global
                .map(|row| map_get_text(row, "grafanaAdmin"))
                .unwrap_or_default();
            let org_role = normalize_org_role(user.get("role").or_else(|| user.get("orgRole")));
            summaries
                .entry(user_id.clone())
                .or_default()
                .push(format!("{org_name}: {org_role}"));
            membership_rows.push(Map::from_iter(vec![
                ("rowKind".to_string(), Value::String("member".to_string())),
                (
                    "id".to_string(),
                    Value::String(format!("{org_id}:{user_id}")),
                ),
                ("userId".to_string(), Value::String(user_id)),
                ("orgId".to_string(), Value::String(org_id.clone())),
                ("orgName".to_string(), Value::String(org_name.clone())),
                (
                    "scope".to_string(),
                    Value::String("org-membership".to_string()),
                ),
                ("login".to_string(), Value::String(login)),
                ("email".to_string(), Value::String(email)),
                ("name".to_string(), Value::String(name)),
                ("orgRole".to_string(), Value::String(org_role)),
                ("grafanaAdmin".to_string(), Value::String(grafana_admin)),
                ("teams".to_string(), Value::String(String::new())),
            ]));
        }
        raw_orgs.push((org_id, org_name, membership_rows));
    }

    let mut rows = Vec::new();
    for (org_id, org_name, mut members) in raw_orgs {
        let member_count = members.len().to_string();
        rows.push(Map::from_iter(vec![
            ("rowKind".to_string(), Value::String("org".to_string())),
            ("id".to_string(), Value::String(org_id.clone())),
            ("orgId".to_string(), Value::String(org_id)),
            ("orgName".to_string(), Value::String(org_name.clone())),
            ("name".to_string(), Value::String(org_name)),
            ("memberCount".to_string(), Value::String(member_count)),
        ]));
        for member in &mut members {
            let user_id = map_get_text(member, "userId");
            let cross = summaries
                .get(&user_id)
                .cloned()
                .unwrap_or_default()
                .join(" | ");
            member.insert("crossOrgMemberships".to_string(), Value::String(cross));
        }
        rows.extend(members);
    }
    Ok(rows)
}
