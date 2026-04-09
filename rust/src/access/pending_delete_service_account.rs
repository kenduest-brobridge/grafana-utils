//! Resolve and validate pending service-account deletes before destructive API calls.
//! This module resolves service-account and token identities, confirms the caller's
//! delete intent, and prepares the target data consumed by the final delete flow.
//! It stops short of sending the delete request, so the dangerous part stays in the
//! dedicated live delete handlers.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};

use super::super::render::{map_get_text, normalize_service_account_row, scalar_text};
use super::super::{request_object, request_object_list_field, DEFAULT_PAGE_SIZE};
use super::pending_delete_support::{
    format_prompt_row, print_delete_confirmation_summary, prompt_confirm_delete,
    prompt_select_index, prompt_select_indexes, validate_confirmation, validate_delete_prompt,
    validate_exactly_one_identity, validate_token_identity, ServiceAccountDeleteArgs,
    ServiceAccountTokenDeleteArgs,
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

fn service_account_prompt_label(service_account: &Map<String, Value>) -> String {
    let name = string_field(service_account, "name", "-");
    let login = string_field(service_account, "login", "-");
    let id = scalar_text(service_account.get("id"));
    format_prompt_row(&[(&name, 22), (&login, 22)], &format!("id={id}"))
}

fn service_account_context_label(service_account: &Map<String, Value>) -> String {
    let name = string_field(service_account, "name", "-");
    let login = string_field(service_account, "login", "-");
    let id = {
        let id = scalar_text(service_account.get("id"));
        if id.is_empty() {
            scalar_text(service_account.get("serviceAccountId"))
        } else {
            id
        }
    };
    format!("service-account={} login={} id={}", name, login, id)
}

fn service_account_token_prompt_label(
    service_account: &Map<String, Value>,
    token: &Map<String, Value>,
) -> String {
    let name = string_field(token, "name", "-");
    let id = scalar_text(token.get("id"));
    format_prompt_row(
        &[(&name, 30)],
        &format!("id={id} {}", service_account_context_label(service_account)),
    )
}

/// Delete a service account after identity checks and optional JSON output.
pub(crate) fn delete_service_account_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_delete_prompt(args.prompt, args.json, "Service-account")?;
    if !args.prompt {
        validate_exactly_one_identity(
            args.service_account_id.is_some(),
            args.name.is_some(),
            "Service-account",
            "--service-account-id",
        )?;
        validate_confirmation(args.yes, "Service-account")?;
    }
    let service_accounts =
        if args.prompt && args.service_account_id.is_none() && args.name.is_none() {
            let accounts =
                list_service_accounts_with_request(&mut request_json, None, 1, DEFAULT_PAGE_SIZE)?;
            if accounts.is_empty() {
                return Err(message(
                    "Service-account delete --prompt did not find any matching service accounts.",
                ));
            }
            let labels = accounts
                .iter()
                .map(service_account_prompt_label)
                .collect::<Vec<_>>();
            let Some(indexes) = prompt_select_indexes("Service Accounts To Delete", &labels)?
            else {
                println!("Cancelled service-account delete.");
                return Ok(0);
            };
            indexes
                .into_iter()
                .filter_map(|index| accounts.get(index).cloned())
                .collect::<Vec<_>>()
        } else if let Some(service_account_id) = &args.service_account_id {
            vec![get_service_account_with_request(
                &mut request_json,
                service_account_id,
            )?]
        } else {
            vec![lookup_service_account_by_name(
                &mut request_json,
                args.name.as_deref().unwrap_or(""),
            )?]
        };
    if args.prompt {
        let labels = service_accounts
            .iter()
            .map(service_account_prompt_label)
            .collect::<Vec<_>>();
        print_delete_confirmation_summary(
            "The following service accounts will be deleted:",
            &labels,
        );
    }
    if args.prompt
        && !prompt_confirm_delete(&format!(
            "Delete {} service account(s)?",
            service_accounts.len()
        ))?
    {
        println!("Cancelled service-account delete.");
        return Ok(0);
    }
    let mut results = Vec::new();
    for service_account in &service_accounts {
        let service_account_id = {
            let id = scalar_text(service_account.get("id"));
            if id.is_empty() {
                scalar_text(service_account.get("serviceAccountId"))
            } else {
                id
            }
        };
        let response =
            delete_service_account_api_with_request(&mut request_json, &service_account_id)?;
        results.push(service_account_delete_result(service_account, &response));
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&Value::Array(
                results.iter().cloned().map(Value::Object).collect()
            ))?
        );
    } else {
        for result in &results {
            println!("{}", service_account_delete_summary_line(result));
        }
        if results.len() > 1 {
            println!("Deleted {} service account(s).", results.len());
        }
    }
    Ok(results.len())
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
    validate_delete_prompt(args.prompt, args.json, "Service-account token")?;
    if !args.prompt {
        validate_token_identity(args)?;
        validate_confirmation(args.yes, "Service-account token")?;
    }
    let service_account = if args.prompt && args.service_account_id.is_none() && args.name.is_none()
    {
        let accounts =
            list_service_accounts_with_request(&mut request_json, None, 1, DEFAULT_PAGE_SIZE)?;
        if accounts.is_empty() {
            return Err(message(
                "Service-account token delete --prompt did not find any matching service accounts.",
            ));
        }
        let labels = accounts
            .iter()
            .map(service_account_prompt_label)
            .collect::<Vec<_>>();
        let Some(index) = prompt_select_index("Service Account", &labels)? else {
            println!("Cancelled service-account token delete.");
            return Ok(0);
        };
        accounts[index].clone()
    } else if let Some(service_account_id) = &args.service_account_id {
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
    let tokens = if args.prompt && args.token_id.is_none() && args.token_name.is_none() {
        let tokens =
            list_service_account_tokens_with_request(&mut request_json, &service_account_id)?;
        if tokens.is_empty() {
            return Err(message(format!(
                "Service-account token delete --prompt did not find any tokens for service account {service_account_id}."
            )));
        }
        let labels = tokens
            .iter()
            .map(|token| service_account_token_prompt_label(&service_account, token))
            .collect::<Vec<_>>();
        let Some(indexes) = prompt_select_indexes("Tokens To Delete", &labels)? else {
            println!("Cancelled service-account token delete.");
            return Ok(0);
        };
        indexes
            .into_iter()
            .filter_map(|index| tokens.get(index).cloned())
            .collect::<Vec<_>>()
    } else if let Some(token_id) = &args.token_id {
        vec![Map::from_iter(vec![
            ("id".to_string(), Value::String(token_id.clone())),
            ("name".to_string(), Value::String(String::new())),
        ])]
    } else {
        vec![lookup_service_account_token_by_name(
            &mut request_json,
            &service_account_id,
            args.token_name.as_deref().unwrap_or(""),
        )?]
    };
    if args.prompt {
        let labels = tokens
            .iter()
            .map(|token| service_account_token_prompt_label(&service_account, token))
            .collect::<Vec<_>>();
        print_delete_confirmation_summary(
            &format!(
                "The following service-account tokens will be deleted from {}:",
                service_account_context_label(&service_account)
            ),
            &labels,
        );
    }
    if args.prompt
        && !prompt_confirm_delete(&format!(
            "Delete {} token(s) from {}?",
            tokens.len(),
            service_account_context_label(&service_account)
        ))?
    {
        println!("Cancelled service-account token delete.");
        return Ok(0);
    }
    let mut results = Vec::new();
    for token in &tokens {
        let token_id = scalar_text(token.get("id"));
        let response = delete_service_account_token_api_with_request(
            &mut request_json,
            &service_account_id,
            &token_id,
        )?;
        results.push(service_account_token_delete_result(
            &service_account,
            token,
            &response,
        ));
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&Value::Array(
                results.iter().cloned().map(Value::Object).collect()
            ))?
        );
    } else {
        for result in &results {
            println!("{}", service_account_token_delete_summary_line(result));
        }
        if results.len() > 1 {
            println!(
                "Deleted {} service-account token(s) from {}.",
                results.len(),
                string_field(&service_account, "name", "")
            );
        }
    }
    Ok(results.len())
}

#[cfg(test)]
mod pending_delete_service_account_tests {
    use super::*;

    #[test]
    fn service_account_token_prompt_label_includes_service_account_context() {
        let service_account = Map::from_iter(vec![
            ("id".to_string(), Value::String("4".to_string())),
            ("name".to_string(), Value::String("svc".to_string())),
            ("login".to_string(), Value::String("sa-svc".to_string())),
        ]);
        let token = Map::from_iter(vec![
            ("id".to_string(), Value::String("7".to_string())),
            ("name".to_string(), Value::String("automation".to_string())),
        ]);

        let label = service_account_token_prompt_label(&service_account, &token);

        assert!(label.contains("automation"));
        assert!(label.contains("id=7"));
        assert!(label.contains("service-account=svc"));
        assert!(label.contains("login=sa-svc"));
    }

    #[test]
    fn service_account_context_label_prefers_numeric_id_when_present() {
        let service_account = Map::from_iter(vec![
            ("serviceAccountId".to_string(), Value::String("9".to_string())),
            ("name".to_string(), Value::String("svc".to_string())),
            ("login".to_string(), Value::String("sa-svc".to_string())),
        ]);

        let label = service_account_context_label(&service_account);

        assert_eq!(label, "service-account=svc login=sa-svc id=9");
    }
}
