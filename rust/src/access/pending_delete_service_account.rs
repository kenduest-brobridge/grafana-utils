use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};

use super::super::render::{map_get_text, normalize_service_account_row, scalar_text};
use super::super::{request_object, request_object_list_field, DEFAULT_PAGE_SIZE};
use super::pending_delete_support::{
    render_single_object_json, validate_confirmation, validate_exactly_one_identity,
    validate_token_identity, ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs,
};

/// List one page of service accounts for delete resolution.
fn list_service_accounts_with_request<F>(
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
    request_object_list_field(
        &mut request_json,
        Method::GET,
        "/api/serviceaccounts/search",
        &params,
        None,
        "serviceAccounts",
        (
            "Unexpected service-account list response from Grafana.",
            "Unexpected service-account list response from Grafana.",
        ),
    )
}

/// Find a service account by exact name.
fn lookup_service_account_by_name<F>(mut request_json: F, name: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let accounts =
        list_service_accounts_with_request(&mut request_json, Some(name), 1, DEFAULT_PAGE_SIZE)?;
    accounts
        .into_iter()
        .find(|item| string_field(item, "name", "") == name)
        .ok_or_else(|| {
            message(format!(
                "Grafana service-account lookup did not find {name}."
            ))
        })
}

/// Fetch one service account record for id-backed delete workflows.
fn get_service_account_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::GET,
        &format!("/api/serviceaccounts/{service_account_id}"),
        &[],
        None,
        &format!(
            "Unexpected service-account lookup response for Grafana service account {service_account_id}."
        ),
    )
}

/// Delete one service account and return Grafana's response payload.
fn delete_service_account_api_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/serviceaccounts/{service_account_id}"),
        &[],
        None,
        &format!(
            "Unexpected service-account delete response for Grafana service account {service_account_id}."
        ),
    )
}

/// Merge delete target info with response message for stable reporting.
fn service_account_delete_result(
    service_account: &Map<String, Value>,
    response: &Map<String, Value>,
) -> Map<String, Value> {
    let mut row = normalize_service_account_row(service_account);
    row.insert(
        "serviceAccountId".to_string(),
        Value::String({
            let id = map_get_text(&row, "id");
            if id.is_empty() {
                scalar_text(response.get("serviceAccountId"))
            } else {
                id
            }
        }),
    );
    row.insert(
        "message".to_string(),
        Value::String(string_field(
            response,
            "message",
            "Service account deleted.",
        )),
    );
    row
}

/// Build a stable summary line for a deleted service account.
fn service_account_delete_summary_line(result: &Map<String, Value>) -> String {
    let mut parts = vec![
        format!(
            "serviceAccountId={}",
            map_get_text(result, "serviceAccountId")
        ),
        format!("name={}", map_get_text(result, "name")),
    ];
    let login = map_get_text(result, "login");
    if !login.is_empty() {
        parts.push(format!("login={login}"));
    }
    let role = map_get_text(result, "role");
    if !role.is_empty() {
        parts.push(format!("role={role}"));
    }
    let message = map_get_text(result, "message");
    if !message.is_empty() {
        parts.push(format!("message={message}"));
    }
    parts.join(" ")
}

/// Delete a service account after identity checks and optional JSON output.
pub(crate) fn delete_service_account_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_exactly_one_identity(
        args.service_account_id.is_some(),
        args.name.is_some(),
        "Service-account",
        "--service-account-id",
    )?;
    validate_confirmation(args.yes, "Service-account")?;
    let service_account = if let Some(service_account_id) = &args.service_account_id {
        get_service_account_with_request(&mut request_json, service_account_id)?
    } else {
        lookup_service_account_by_name(&mut request_json, args.name.as_deref().unwrap_or(""))?
    };
    let service_account_id = {
        let id = scalar_text(service_account.get("id"));
        if id.is_empty() {
            scalar_text(service_account.get("serviceAccountId"))
        } else {
            id
        }
    };
    let response = delete_service_account_api_with_request(&mut request_json, &service_account_id)?;
    let result = service_account_delete_result(&service_account, &response);
    if args.json {
        println!("{}", render_single_object_json(&result)?);
    } else {
        println!("{}", service_account_delete_summary_line(&result));
    }
    Ok(0)
}

/// List tokens for one service account to support exact-token selection.
fn list_service_account_tokens_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(
        Method::GET,
        &format!("/api/serviceaccounts/{service_account_id}/tokens"),
        &[],
        None,
    )? {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                Ok(value_as_object(
                    item,
                    "Unexpected service-account token list response from Grafana.",
                )?
                .clone())
            })
            .collect(),
        Some(_) => Err(message(
            "Unexpected service-account token list response from Grafana.",
        )),
        None => Ok(Vec::new()),
    }
}

/// Find one token by exact name for token deletion workflows.
fn lookup_service_account_token_by_name<F>(
    mut request_json: F,
    service_account_id: &str,
    token_name: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let tokens = list_service_account_tokens_with_request(&mut request_json, service_account_id)?;
    tokens
        .into_iter()
        .find(|token| string_field(token, "name", "") == token_name)
        .ok_or_else(|| {
            message(format!(
                "Grafana service-account token lookup did not find {token_name}."
            ))
        })
}

/// Delete one token from a service account and return API response.
fn delete_service_account_token_api_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
    token_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/serviceaccounts/{service_account_id}/tokens/{token_id}"),
        &[],
        None,
        &format!(
            "Unexpected service-account token delete response for Grafana service account {service_account_id} token {token_id}."
        ),
    )
}

/// Build a stable result row for a deleted service-account token.
fn service_account_token_delete_result(
    service_account: &Map<String, Value>,
    token: &Map<String, Value>,
    response: &Map<String, Value>,
) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "serviceAccountId".to_string(),
            Value::String({
                let id = scalar_text(service_account.get("id"));
                if id.is_empty() {
                    scalar_text(service_account.get("serviceAccountId"))
                } else {
                    id
                }
            }),
        ),
        (
            "serviceAccountName".to_string(),
            Value::String(string_field(service_account, "name", "")),
        ),
        (
            "tokenId".to_string(),
            Value::String({
                let id = scalar_text(token.get("id"));
                if id.is_empty() {
                    scalar_text(response.get("tokenId"))
                } else {
                    id
                }
            }),
        ),
        (
            "tokenName".to_string(),
            Value::String(string_field(token, "name", "")),
        ),
        (
            "message".to_string(),
            Value::String(string_field(
                response,
                "message",
                "Service-account token deleted.",
            )),
        ),
    ])
}

/// Build a stable summary line for a deleted service-account token.
fn service_account_token_delete_summary_line(result: &Map<String, Value>) -> String {
    [
        format!(
            "serviceAccountId={}",
            map_get_text(result, "serviceAccountId")
        ),
        format!(
            "serviceAccountName={}",
            map_get_text(result, "serviceAccountName")
        ),
        format!("tokenId={}", map_get_text(result, "tokenId")),
        format!("tokenName={}", map_get_text(result, "tokenName")),
        format!("message={}", map_get_text(result, "message")),
    ]
    .join(" ")
}

/// Delete one service-account token with mutually-exclusive identity checks.
pub(crate) fn delete_service_account_token_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountTokenDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_token_identity(args)?;
    validate_confirmation(args.yes, "Service-account token")?;
    let service_account = if let Some(service_account_id) = &args.service_account_id {
        get_service_account_with_request(&mut request_json, service_account_id)?
    } else {
        lookup_service_account_by_name(&mut request_json, args.name.as_deref().unwrap_or(""))?
    };
    let service_account_id = {
        let id = scalar_text(service_account.get("id"));
        if id.is_empty() {
            scalar_text(service_account.get("serviceAccountId"))
        } else {
            id
        }
    };
    let token = if let Some(token_id) = &args.token_id {
        Map::from_iter(vec![
            ("id".to_string(), Value::String(token_id.clone())),
            ("name".to_string(), Value::String(String::new())),
        ])
    } else {
        lookup_service_account_token_by_name(
            &mut request_json,
            &service_account_id,
            args.token_name.as_deref().unwrap_or(""),
        )?
    };
    let token_id = scalar_text(token.get("id"));
    let response = delete_service_account_token_api_with_request(
        &mut request_json,
        &service_account_id,
        &token_id,
    )?;
    let result = service_account_token_delete_result(&service_account, &token, &response);
    if args.json {
        println!("{}", render_single_object_json(&result)?);
    } else {
        println!("{}", service_account_token_delete_summary_line(&result));
    }
    Ok(0)
}
