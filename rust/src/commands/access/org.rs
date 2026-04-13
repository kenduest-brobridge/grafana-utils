//! Access organization command handlers.
//! Handles org CRUD plus snapshot export/import behind shared access-request wrappers.
mod org_import_export_diff;
#[path = "org_workflows.rs"]
mod org_workflows;
use reqwest::Method;
use serde_json::{Map, Value};

use crate::access::cli_defs::{build_auth_context_no_org_id, CommonCliArgsNoOrgId};
use crate::common::{message, string_field, value_as_object, Result};

pub(crate) use self::org_workflows::{
    add_org_with_request, delete_org_with_request, diff_orgs_with_request,
    export_orgs_with_request, import_orgs_with_request, list_orgs_from_input_dir,
    list_orgs_with_request, modify_org_with_request,
};
use super::render::{normalize_org_role, scalar_text};
use super::{request_array, request_object, OrgListArgs};

fn validate_basic_auth_only(common: &CommonCliArgsNoOrgId) -> Result<()> {
    let auth_mode = build_auth_context_no_org_id(common)?.auth_mode;
    if auth_mode != "basic" {
        Err(message(
            "Organization commands require Basic auth (--basic-user / --basic-password).",
        ))
    } else {
        Ok(())
    }
}

fn list_organizations_with_request<F>(mut request_json: F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        "/api/orgs",
        &[],
        None,
        "Unexpected organization list response from Grafana.",
    )
}

fn create_organization_with_request<F>(
    mut request_json: F,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        "/api/orgs",
        &[],
        Some(payload),
        "Unexpected organization create response from Grafana.",
    )
}

fn update_organization_with_request<F>(
    mut request_json: F,
    org_id: &str,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/orgs/{org_id}"),
        &[],
        Some(payload),
        &format!("Unexpected organization update response for Grafana org {org_id}."),
    )
}

fn delete_organization_with_request<F>(
    mut request_json: F,
    org_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/orgs/{org_id}"),
        &[],
        None,
        &format!("Unexpected organization delete response for Grafana org {org_id}."),
    )
}

fn list_org_users_with_request<F>(
    mut request_json: F,
    org_id: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        &format!("/api/orgs/{org_id}/users"),
        &[],
        None,
        &format!("Unexpected organization user list response for Grafana org {org_id}."),
    )
}

fn add_user_to_org_with_request<F>(
    mut request_json: F,
    org_id: &str,
    login_or_email: &str,
    role: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        &format!("/api/orgs/{org_id}/users"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![
            (
                "loginOrEmail".to_string(),
                Value::String(login_or_email.to_string()),
            ),
            ("role".to_string(), Value::String(role.to_string())),
        ]))),
        &format!("Unexpected organization add-user response for Grafana org {org_id}."),
    )
}

fn update_org_user_role_with_request<F>(
    mut request_json: F,
    org_id: &str,
    user_id: &str,
    role: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PATCH,
        &format!("/api/orgs/{org_id}/users/{user_id}"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "role".to_string(),
            Value::String(role.to_string()),
        )]))),
        &format!(
            "Unexpected organization user role update response for Grafana org {org_id} user {user_id}."
        ),
    )
}

fn normalize_org_user_row(user: &Map<String, Value>) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "userId".to_string(),
            Value::String({
                let user_id = scalar_text(user.get("userId"));
                if user_id.is_empty() {
                    scalar_text(user.get("id"))
                } else {
                    user_id
                }
            }),
        ),
        (
            "login".to_string(),
            Value::String(string_field(user, "login", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(user, "email", "")),
        ),
        (
            "name".to_string(),
            Value::String(string_field(user, "name", "")),
        ),
        (
            "orgRole".to_string(),
            Value::String(normalize_org_role(
                user.get("role").or_else(|| user.get("orgRole")),
            )),
        ),
    ])
}

fn normalize_org_row(org: &Map<String, Value>) -> Map<String, Value> {
    let users = match org.get("users") {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(|item| value_as_object(item, "Unexpected org user record.").ok())
            .map(normalize_org_user_row)
            .map(Value::Object)
            .collect::<Vec<Value>>(),
        _ => Vec::new(),
    };
    let user_count = match org.get("userCount") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        _ => users.len().to_string(),
    };
    Map::from_iter(vec![
        (
            "id".to_string(),
            Value::String({
                let id = scalar_text(org.get("id"));
                if id.is_empty() {
                    scalar_text(org.get("orgId"))
                } else {
                    id
                }
            }),
        ),
        (
            "name".to_string(),
            Value::String(string_field(org, "name", "")),
        ),
        ("userCount".to_string(), Value::String(user_count)),
        ("users".to_string(), Value::Array(users)),
    ])
}

fn org_matches(org: &Map<String, Value>, args: &OrgListArgs) -> bool {
    if let Some(org_id) = args.org_id {
        if scalar_text(org.get("id")) != org_id.to_string() {
            return false;
        }
    }
    if let Some(name) = &args.name {
        if string_field(org, "name", "") != *name {
            return false;
        }
    }
    if let Some(query) = &args.query {
        if !string_field(org, "name", "")
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
        {
            return false;
        }
    }
    true
}

pub(crate) fn org_user_summary(row: &Map<String, Value>) -> String {
    let Some(Value::Array(users)) = row.get("users") else {
        return "-".to_string();
    };
    if users.is_empty() {
        return "-".to_string();
    }
    let labels = users
        .iter()
        .filter_map(Value::as_object)
        .map(|user| {
            let identity = [
                string_field(user, "login", ""),
                string_field(user, "email", ""),
                string_field(user, "name", ""),
                scalar_text(user.get("userId")),
            ]
            .into_iter()
            .find(|value| !value.is_empty())
            .unwrap_or_else(|| "-".to_string());
            let role = string_field(user, "orgRole", "");
            if role.is_empty() {
                identity
            } else {
                format!("{identity}({role})")
            }
        })
        .collect::<Vec<String>>();
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join("; ")
    }
}

pub(crate) fn org_table_headers(with_users: bool) -> Vec<&'static str> {
    if with_users {
        vec!["ID", "NAME", "USER_COUNT", "USERS"]
    } else {
        vec!["ID", "NAME", "USER_COUNT"]
    }
}

pub(crate) fn org_csv_headers(with_users: bool) -> Vec<&'static str> {
    if with_users {
        vec!["id", "name", "userCount", "users"]
    } else {
        vec!["id", "name", "userCount"]
    }
}

pub(crate) fn org_table_rows(rows: &[Map<String, Value>], with_users: bool) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            let mut cells = vec![
                scalar_text(row.get("id")),
                string_field(row, "name", ""),
                scalar_text(row.get("userCount")),
            ];
            if with_users {
                cells.push(org_user_summary(row));
            }
            cells
        })
        .collect()
}

pub(crate) fn org_summary_line(row: &Map<String, Value>, with_users: bool) -> String {
    let mut line = format!(
        "id={} name={} userCount={}",
        scalar_text(row.get("id")),
        string_field(row, "name", ""),
        scalar_text(row.get("userCount"))
    );
    if with_users {
        line.push_str(&format!(" users={}", org_user_summary(row)));
    }
    line
}

fn lookup_org_by_identity<F>(
    mut request_json: F,
    org_id: Option<i64>,
    name: Option<&str>,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let orgs = list_organizations_with_request(&mut request_json)?;
    orgs.into_iter()
        .find(|org| {
            org_id.is_some_and(|value| scalar_text(org.get("id")) == value.to_string())
                || name.is_some_and(|value| string_field(org, "name", "") == value)
        })
        .map(|org| normalize_org_row(&org))
        .ok_or_else(|| message("Grafana org lookup did not find a matching organization."))
}
