use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, Result};

use super::access_render::{
    bool_label, map_get_text, normalize_org_role, normalize_user_row, paginate_rows, render_csv,
    render_objects_json, user_matches, user_scope_text, user_summary_line, user_table_rows,
    value_bool, scalar_text, format_table,
};
use super::{
    build_auth_context, request_array, request_object, Scope, UserAddArgs,
    UserDeleteArgs, UserListArgs, UserModifyArgs, DEFAULT_PAGE_SIZE,
};

pub(crate) fn list_org_users_with_request<F>(mut request_json: F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        "/api/org/users",
        &[],
        None,
        "Unexpected org user list response from Grafana.",
    )
}

fn iter_global_users_with_request<F>(mut request_json: F, page_size: usize) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut users = Vec::new();
    let mut page = 1usize;
    loop {
        let params = vec![
            ("page".to_string(), page.to_string()),
            ("perpage".to_string(), page_size.to_string()),
        ];
        let batch = request_array(
            &mut request_json,
            Method::GET,
            "/api/users",
            &params,
            None,
            "Unexpected global user list response from Grafana.",
        )?;
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        users.extend(batch);
        if batch_len < page_size {
            break;
        }
        page += 1;
    }
    Ok(users)
}

fn list_user_teams_with_request<F>(mut request_json: F, user_id: &str) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        &format!("/api/users/{user_id}/teams"),
        &[],
        None,
        &format!("Unexpected team list response for Grafana user {user_id}."),
    )
}

fn get_user_with_request<F>(mut request_json: F, user_id: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::GET,
        &format!("/api/users/{user_id}"),
        &[],
        None,
        &format!("Unexpected user lookup response for Grafana user {user_id}."),
    )
}

fn create_user_with_request<F>(mut request_json: F, payload: &Value) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        "/api/admin/users",
        &[],
        Some(payload),
        "Unexpected user create response from Grafana.",
    )
}

fn update_user_with_request<F>(mut request_json: F, user_id: &str, payload: &Value) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/users/{user_id}"),
        &[],
        Some(payload),
        &format!("Unexpected user update response for Grafana user {user_id}."),
    )
}

fn update_user_password_with_request<F>(
    mut request_json: F,
    user_id: &str,
    password: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/admin/users/{user_id}/password"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "password".to_string(),
            Value::String(password.to_string()),
        )]))),
        &format!("Unexpected password update response for Grafana user {user_id}."),
    )
}

fn update_user_org_role_with_request<F>(
    mut request_json: F,
    user_id: &str,
    role: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PATCH,
        &format!("/api/org/users/{user_id}"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "role".to_string(),
            Value::String(role.to_string()),
        )]))),
        &format!("Unexpected org-role update response for Grafana user {user_id}."),
    )
}

fn update_user_permissions_with_request<F>(
    mut request_json: F,
    user_id: &str,
    is_admin: bool,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/admin/users/{user_id}/permissions"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "isGrafanaAdmin".to_string(),
            Value::Bool(is_admin),
        )]))),
        &format!("Unexpected permission update response for Grafana user {user_id}."),
    )
}

fn delete_global_user_with_request<F>(mut request_json: F, user_id: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/admin/users/{user_id}"),
        &[],
        None,
        &format!("Unexpected global delete response for Grafana user {user_id}."),
    )
}

fn delete_org_user_with_request<F>(mut request_json: F, user_id: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/org/users/{user_id}"),
        &[],
        None,
        &format!("Unexpected org delete response for Grafana user {user_id}."),
    )
}

fn lookup_global_user_by_identity<F>(
    mut request_json: F,
    login: Option<&str>,
    email: Option<&str>,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let users = iter_global_users_with_request(&mut request_json, DEFAULT_PAGE_SIZE)?;
    users
        .into_iter()
        .find(|user| {
            login.is_some_and(|value| string_field(user, "login", "") == value)
                || email.is_some_and(|value| string_field(user, "email", "") == value)
        })
        .ok_or_else(|| message("Grafana user lookup did not find a matching global user."))
}

pub(crate) fn lookup_org_user_by_identity<F>(mut request_json: F, identity: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let users = list_org_users_with_request(&mut request_json)?;
    users
        .into_iter()
        .find(|user| {
            string_field(user, "login", "") == identity
                || string_field(user, "email", "") == identity
                || scalar_text(user.get("userId")) == identity
                || scalar_text(user.get("id")) == identity
        })
        .ok_or_else(|| message(format!("Grafana org user lookup did not find {identity}.")))
}

fn validate_basic_auth_only(auth_mode: &str, operation: &str) -> Result<()> {
    if auth_mode != "basic" {
        Err(message(format!(
            "{operation} requires Basic auth (--basic-user / --basic-password)."
        )))
    } else {
        Ok(())
    }
}

fn validate_user_list_auth(args: &UserListArgs, auth_mode: &str) -> Result<()> {
    if args.scope == Scope::Global && auth_mode != "basic" {
        return Err(message(
            "User list with --scope global requires Basic auth (--basic-user / --basic-password).",
        ));
    }
    if args.with_teams && auth_mode != "basic" {
        return Err(message("--with-teams requires Basic auth."));
    }
    Ok(())
}

fn validate_user_modify_args(args: &UserModifyArgs) -> Result<()> {
    let has_identity = args.user_id.is_some() || args.login.is_some() || args.email.is_some();
    if !has_identity {
        return Err(message("User modify requires one of --user-id, --login, or --email."));
    }
    if args.set_login.is_none()
        && args.set_email.is_none()
        && args.set_name.is_none()
        && args.set_password.is_none()
        && args.set_org_role.is_none()
        && args.set_grafana_admin.is_none()
    {
        return Err(message(
            "User modify requires at least one of --set-login, --set-email, --set-name, --set-password, --set-org-role, or --set-grafana-admin.",
        ));
    }
    Ok(())
}

fn validate_user_delete_args(args: &UserDeleteArgs) -> Result<()> {
    if !args.yes {
        return Err(message("User delete requires --yes."));
    }
    if args.user_id.is_none() && args.login.is_none() && args.email.is_none() {
        return Err(message("User delete requires one of --user-id, --login, or --email."));
    }
    Ok(())
}

pub(crate) fn list_users_with_request<F>(mut request_json: F, args: &UserListArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_list_auth(args, &auth_mode)?;
    let mut rows = match args.scope {
        Scope::Org => list_org_users_with_request(&mut request_json)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Org))
            .collect::<Vec<Map<String, Value>>>(),
        Scope::Global => iter_global_users_with_request(&mut request_json, DEFAULT_PAGE_SIZE)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Global))
            .collect::<Vec<Map<String, Value>>>(),
    };
    if args.with_teams {
        for row in &mut rows {
            let user_id = map_get_text(row, "id");
            let teams = list_user_teams_with_request(&mut request_json, &user_id)?
                .into_iter()
                .map(|team| string_field(&team, "name", ""))
                .filter(|name| !name.is_empty())
                .map(Value::String)
                .collect::<Vec<Value>>();
            row.insert("teams".to_string(), Value::Array(teams));
        }
    }
    rows.retain(|row| user_matches(row, args));
    let rows = paginate_rows(&rows, args.page, args.per_page);
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.csv {
        for line in render_csv(
            &["id", "login", "email", "name", "orgRole", "grafanaAdmin", "scope", "teams"],
            &user_table_rows(&rows),
        ) {
            println!("{line}");
        }
    } else if args.table {
        for line in format_table(
            &["ID", "LOGIN", "EMAIL", "NAME", "ORG_ROLE", "GRAFANA_ADMIN", "SCOPE", "TEAMS"],
            &user_table_rows(&rows),
        ) {
            println!("{line}");
        }
        println!();
        println!(
            "Listed {} user(s) from {} scope at {}",
            rows.len(),
            user_scope_text(&args.scope),
            args.common.url
        );
    } else {
        for row in &rows {
            println!("{}", user_summary_line(row));
        }
        println!();
        println!(
            "Listed {} user(s) from {} scope at {}",
            rows.len(),
            user_scope_text(&args.scope),
            args.common.url
        );
    }
    Ok(rows.len())
}

pub(crate) fn add_user_with_request<F>(mut request_json: F, args: &UserAddArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_basic_auth_only(&auth_mode, "User add")?;
    let mut payload = Map::from_iter(vec![
        ("login".to_string(), Value::String(args.login.clone())),
        ("email".to_string(), Value::String(args.email.clone())),
        ("name".to_string(), Value::String(args.name.clone())),
        ("password".to_string(), Value::String(args.new_user_password.clone())),
    ]);
    if let Some(org_id) = args.common.org_id {
        payload.insert("OrgId".to_string(), Value::Number(org_id.into()));
    }
    let created = create_user_with_request(&mut request_json, &Value::Object(payload))?;
    let user_id = scalar_text(created.get("id"));
    if user_id.is_empty() {
        return Err(message("Grafana user create response did not include an id."));
    }
    if let Some(role) = &args.org_role {
        let _ = update_user_org_role_with_request(&mut request_json, &user_id, role)?;
    }
    if let Some(is_admin) = args.grafana_admin {
        let _ = update_user_permissions_with_request(&mut request_json, &user_id, is_admin)?;
    }
    let row = Map::from_iter(vec![
        ("id".to_string(), Value::String(user_id.clone())),
        ("login".to_string(), Value::String(args.login.clone())),
        ("email".to_string(), Value::String(args.email.clone())),
        ("name".to_string(), Value::String(args.name.clone())),
        (
            "orgRole".to_string(),
            Value::String(args.org_role.clone().unwrap_or_default()),
        ),
        (
            "grafanaAdmin".to_string(),
            Value::String(bool_label(args.grafana_admin)),
        ),
        ("scope".to_string(), Value::String("global".to_string())),
        ("teams".to_string(), Value::Array(Vec::new())),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!(
            "Created user {} -> id={} orgRole={} grafanaAdmin={}",
            args.login,
            user_id,
            args.org_role.clone().unwrap_or_default(),
            bool_label(args.grafana_admin)
        );
    }
    Ok(0)
}

pub(crate) fn modify_user_with_request<F>(
    mut request_json: F,
    args: &UserModifyArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_basic_auth_only(&auth_mode, "User modify")?;
    validate_user_modify_args(args)?;
    let base_user = if let Some(user_id) = &args.user_id {
        get_user_with_request(&mut request_json, user_id)?
    } else {
        lookup_global_user_by_identity(&mut request_json, args.login.as_deref(), args.email.as_deref())?
    };
    let user_id = string_field(&base_user, "id", "");
    let user_id = if user_id.is_empty() {
        scalar_text(base_user.get("id"))
    } else {
        user_id
    };
    let mut payload = Map::new();
    if let Some(value) = &args.set_login {
        payload.insert("login".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &args.set_email {
        payload.insert("email".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &args.set_name {
        payload.insert("name".to_string(), Value::String(value.clone()));
    }
    if !payload.is_empty() {
        let _ = update_user_with_request(&mut request_json, &user_id, &Value::Object(payload))?;
    }
    if let Some(password) = &args.set_password {
        let _ = update_user_password_with_request(&mut request_json, &user_id, password)?;
    }
    if let Some(role) = &args.set_org_role {
        let _ = update_user_org_role_with_request(&mut request_json, &user_id, role)?;
    }
    if let Some(is_admin) = args.set_grafana_admin {
        let _ = update_user_permissions_with_request(&mut request_json, &user_id, is_admin)?;
    }
    let login = args
        .set_login
        .clone()
        .unwrap_or_else(|| string_field(&base_user, "login", ""));
    let row = Map::from_iter(vec![
        ("id".to_string(), Value::String(user_id.clone())),
        ("login".to_string(), Value::String(login.clone())),
        (
            "email".to_string(),
            Value::String(
                args.set_email
                    .clone()
                    .unwrap_or_else(|| string_field(&base_user, "email", "")),
            ),
        ),
        (
            "name".to_string(),
            Value::String(
                args.set_name
                    .clone()
                    .unwrap_or_else(|| string_field(&base_user, "name", "")),
            ),
        ),
        (
            "orgRole".to_string(),
            Value::String(
                args.set_org_role
                    .clone()
                    .unwrap_or_else(|| normalize_org_role(base_user.get("role"))),
            ),
        ),
        (
            "grafanaAdmin".to_string(),
            Value::String(bool_label(
                args.set_grafana_admin
                    .or_else(|| value_bool(base_user.get("isGrafanaAdmin"))),
            )),
        ),
        ("scope".to_string(), Value::String("global".to_string())),
        ("teams".to_string(), Value::Array(Vec::new())),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!("Modified user {} -> id={}", login, user_id);
    }
    Ok(0)
}

pub(crate) fn delete_user_with_request<F>(
    mut request_json: F,
    args: &UserDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_delete_args(args)?;
    if args.scope == Scope::Global {
        validate_basic_auth_only(&auth_mode, "User delete with --scope global")?;
    }
    let base_user = match args.scope {
        Scope::Org => {
            if let Some(user_id) = &args.user_id {
                lookup_org_user_by_identity(&mut request_json, user_id)?
            } else {
                lookup_org_user_by_identity(
                    &mut request_json,
                    args.login.as_deref().or(args.email.as_deref()).unwrap_or(""),
                )?
            }
        }
        Scope::Global => {
            if let Some(user_id) = &args.user_id {
                get_user_with_request(&mut request_json, user_id)?
            } else {
                lookup_global_user_by_identity(
                    &mut request_json,
                    args.login.as_deref(),
                    args.email.as_deref(),
                )?
            }
        }
    };
    let user_id = {
        let user_id = scalar_text(base_user.get("userId"));
        if user_id.is_empty() {
            scalar_text(base_user.get("id"))
        } else {
            user_id
        }
    };
    match args.scope {
        Scope::Org => {
            let _ = delete_org_user_with_request(&mut request_json, &user_id)?;
        }
        Scope::Global => {
            let _ = delete_global_user_with_request(&mut request_json, &user_id)?;
        }
    }
    let row = Map::from_iter(vec![
        ("id".to_string(), Value::String(user_id.clone())),
        (
            "login".to_string(),
            Value::String(string_field(&base_user, "login", "")),
        ),
        (
            "scope".to_string(),
            Value::String(user_scope_text(&args.scope).to_string()),
        ),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!(
            "Deleted user {} -> id={} scope={}",
            map_get_text(&row, "login"),
            user_id,
            user_scope_text(&args.scope)
        );
    }
    Ok(0)
}
