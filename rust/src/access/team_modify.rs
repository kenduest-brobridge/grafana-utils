use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{string_field, Result};

use crate::access::render::{render_objects_json, scalar_text};
use crate::access::team_runtime::{
    add_or_remove_member, create_team_with_request, get_team_with_request,
    list_team_members_with_request, lookup_team_by_name, team_member_identity,
    team_member_is_admin, team_modify_result, team_modify_summary_line,
    update_team_members_with_request, validate_team_modify_args,
};
use crate::access::user::lookup_org_user_by_identity;
use crate::access::{TeamAddArgs, TeamModifyArgs};

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
