use reqwest::Method;
use rpassword::prompt_password;
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

use crate::common::{message, string_field, Result};

use super::render::{
    bool_label, map_get_text, normalize_org_role, render_objects_json, scalar_text,
    user_scope_text, value_bool,
};
use super::{
    build_auth_context, lookup_global_user_by_identity, lookup_org_user_by_identity,
    request_object, Scope, UserAddArgs, UserDeleteArgs, UserModifyArgs,
};

pub(crate) fn get_user_with_request<F>(
    mut request_json: F,
    user_id: &str,
) -> Result<Map<String, Value>>
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

pub(crate) fn create_user_with_request<F>(
    mut request_json: F,
    payload: &Value,
) -> Result<Map<String, Value>>
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

pub(crate) fn update_user_with_request<F>(
    mut request_json: F,
    user_id: &str,
    payload: &Value,
) -> Result<Map<String, Value>>
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

pub(crate) fn update_user_password_with_request<F>(
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

pub(crate) fn update_user_org_role_with_request<F>(
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

pub(crate) fn update_user_permissions_with_request<F>(
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

pub(crate) fn delete_global_user_with_request<F>(
    mut request_json: F,
    user_id: &str,
) -> Result<Map<String, Value>>
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

pub(crate) fn delete_org_user_with_request<F>(
    mut request_json: F,
    user_id: &str,
) -> Result<Map<String, Value>>
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

fn validate_basic_auth_only(auth_mode: &str, operation: &str) -> Result<()> {
    if auth_mode != "basic" {
        Err(message(format!(
            "{operation} requires Basic auth (--basic-user / --basic-password)."
        )))
    } else {
        Ok(())
    }
}

fn validate_user_modify_args(args: &UserModifyArgs) -> Result<()> {
    let has_identity = args.user_id.is_some() || args.login.is_some() || args.email.is_some();
    if !has_identity {
        return Err(message(
            "User modify requires one of --user-id, --login, or --email.",
        ));
    }
    if args.set_login.is_none()
        && args.set_email.is_none()
        && args.set_name.is_none()
        && args.set_password.is_none()
        && args.set_password_file.is_none()
        && !args.prompt_set_password
        && args.set_org_role.is_none()
        && args.set_grafana_admin.is_none()
    {
        return Err(message(
            "User modify requires at least one of --set-login, --set-email, --set-name, --set-password, --set-password-file, --prompt-set-password, --set-org-role, or --set-grafana-admin.",
        ));
    }
    Ok(())
}

fn read_secret_file(path: &Path, label: &str) -> Result<String> {
    let raw = fs::read_to_string(path)?;
    let value = raw.trim_end_matches(&['\r', '\n'][..]).to_string();
    if value.is_empty() {
        return Err(message(format!(
            "{label} file did not contain a usable value: {}",
            path.display()
        )));
    }
    Ok(value)
}

fn resolve_user_add_password(args: &UserAddArgs) -> Result<String> {
    if let Some(password) = &args.new_user_password {
        return Ok(password.clone());
    }
    if let Some(path) = &args.new_user_password_file {
        return read_secret_file(path, "User password");
    }
    if args.prompt_user_password {
        let password = prompt_password("New Grafana user password: ")?;
        if password.is_empty() {
            return Err(message("Prompted user password cannot be empty."));
        }
        return Ok(password);
    }
    Err(message(
        "User add requires one of --password, --password-file, or --prompt-user-password.",
    ))
}

fn resolve_user_modify_password(args: &UserModifyArgs) -> Result<Option<String>> {
    if let Some(password) = &args.set_password {
        return Ok(Some(password.clone()));
    }
    if let Some(path) = &args.set_password_file {
        return Ok(Some(read_secret_file(path, "Replacement user password")?));
    }
    if args.prompt_set_password {
        let password = prompt_password("Replacement Grafana user password: ")?;
        if password.is_empty() {
            return Err(message(
                "Prompted replacement user password cannot be empty.",
            ));
        }
        return Ok(Some(password));
    }
    Ok(None)
}

fn validate_user_delete_args(args: &UserDeleteArgs) -> Result<()> {
    if !args.yes {
        return Err(message("User delete requires --yes."));
    }
    if args.user_id.is_none() && args.login.is_none() && args.email.is_none() {
        return Err(message(
            "User delete requires one of --user-id, --login, or --email.",
        ));
    }
    Ok(())
}

pub(crate) fn add_user_with_request<F>(mut request_json: F, args: &UserAddArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_basic_auth_only(&auth_mode, "User add")?;
    let user_password = resolve_user_add_password(args)?;
    let mut payload = Map::from_iter(vec![
        ("login".to_string(), Value::String(args.login.clone())),
        ("email".to_string(), Value::String(args.email.clone())),
        ("name".to_string(), Value::String(args.name.clone())),
        ("password".to_string(), Value::String(user_password)),
    ]);
    if let Some(org_id) = args.common.org_id {
        payload.insert("OrgId".to_string(), Value::Number(org_id.into()));
    }
    let created = create_user_with_request(&mut request_json, &Value::Object(payload))?;
    let user_id = scalar_text(created.get("id"));
    if user_id.is_empty() {
        return Err(message(
            "Grafana user create response did not include an id.",
        ));
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
        lookup_global_user_by_identity(
            &mut request_json,
            args.login.as_deref(),
            args.email.as_deref(),
        )?
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
    if let Some(password) = resolve_user_modify_password(args)? {
        let _ = update_user_password_with_request(&mut request_json, &user_id, &password)?;
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
                    args.login
                        .as_deref()
                        .or(args.email.as_deref())
                        .unwrap_or(""),
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
