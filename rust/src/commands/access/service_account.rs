//! Access service-account command handlers.
//! Handles service account CRUD and token lifecycle operations behind shared access-request wrappers.
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, Result};

use super::render::scalar_text;
use super::{request_object, request_object_list_field, DEFAULT_PAGE_SIZE};

#[path = "service_account_workflows.rs"]
mod service_account_workflows;

pub(crate) use service_account_workflows::{
    add_service_account_token_with_request, add_service_account_with_request,
    diff_service_accounts_with_request, export_service_accounts_with_request,
    import_service_accounts_with_request, list_service_accounts_command_with_request,
    list_service_accounts_from_input_dir,
};
#[allow(unused_imports)]
pub(crate) use service_account_workflows::{
    list_all_service_accounts_with_request, load_service_account_import_records,
};

/// Fetch one page of service-account search results from Grafana.
///
/// Keep page parameters explicit because Grafana truncates responses by `perpage`.
/// Consumers should treat a returned batch smaller than the requested size as
/// the terminal page and stop pagination immediately.
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

fn create_service_account_with_request<F>(
    mut request_json: F,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        "/api/serviceaccounts",
        &[],
        Some(payload),
        "Unexpected service-account create response from Grafana.",
    )
}

fn update_service_account_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PATCH,
        &format!("/api/serviceaccounts/{service_account_id}"),
        &[],
        Some(payload),
        "Unexpected service-account update response from Grafana.",
    )
}

fn create_service_account_token_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        &format!("/api/serviceaccounts/{service_account_id}/tokens"),
        &[],
        Some(payload),
        "Unexpected service-account token create response from Grafana.",
    )
}

fn lookup_service_account_id_by_name<F>(mut request_json: F, name: &str) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let accounts =
        list_service_accounts_with_request(&mut request_json, Some(name), 1, DEFAULT_PAGE_SIZE)?;
    let account = accounts
        .into_iter()
        .find(|item| string_field(item, "name", "") == name)
        .ok_or_else(|| {
            message(format!(
                "Grafana service-account lookup did not find {name}."
            ))
        })?;
    Ok(scalar_text(account.get("id")))
}
