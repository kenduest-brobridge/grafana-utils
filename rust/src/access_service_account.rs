use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};

use super::access_render::{
    format_table, map_get_text, normalize_service_account_row, render_csv, render_objects_json,
    scalar_text, service_account_role_to_api, service_account_summary_line, service_account_table_rows,
};
use super::{request_object, ServiceAccountAddArgs, ServiceAccountListArgs, ServiceAccountTokenAddArgs, DEFAULT_PAGE_SIZE};

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
    let object = request_object(
        &mut request_json,
        Method::GET,
        "/api/serviceaccounts/search",
        &params,
        None,
        "Unexpected service-account list response from Grafana.",
    )?;
    match object.get("serviceAccounts") {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| Ok(value_as_object(value, "Unexpected service-account list response from Grafana.")?.clone()))
            .collect(),
        _ => Err(message("Unexpected service-account list response from Grafana.")),
    }
}

fn create_service_account_with_request<F>(mut request_json: F, payload: &Value) -> Result<Map<String, Value>>
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
    let accounts = list_service_accounts_with_request(&mut request_json, Some(name), 1, DEFAULT_PAGE_SIZE)?;
    let account = accounts
        .into_iter()
        .find(|item| string_field(item, "name", "") == name)
        .ok_or_else(|| message(format!("Grafana service-account lookup did not find {name}.")))?;
    Ok(scalar_text(account.get("id")))
}

pub(crate) fn list_service_accounts_command_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountListArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = list_service_accounts_with_request(&mut request_json, args.query.as_deref(), args.page, args.per_page)?
        .into_iter()
        .map(|item| normalize_service_account_row(&item))
        .collect::<Vec<Map<String, Value>>>();
    if let Some(query) = &args.query {
        let query = query.to_ascii_lowercase();
        rows.retain(|row| {
            map_get_text(row, "name").to_ascii_lowercase().contains(&query)
                || map_get_text(row, "login").to_ascii_lowercase().contains(&query)
        });
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.csv {
        for line in render_csv(
            &["id", "name", "login", "role", "disabled", "tokens", "orgId"],
            &service_account_table_rows(&rows),
        ) {
            println!("{line}");
        }
    } else if args.table {
        for line in format_table(
            &["ID", "NAME", "LOGIN", "ROLE", "DISABLED", "TOKENS", "ORG_ID"],
            &service_account_table_rows(&rows),
        ) {
            println!("{line}");
        }
        println!();
        println!("Listed {} service account(s) at {}", rows.len(), args.common.url);
    } else {
        for row in &rows {
            println!("{}", service_account_summary_line(row));
        }
        println!();
        println!("Listed {} service account(s) at {}", rows.len(), args.common.url);
    }
    Ok(rows.len())
}

pub(crate) fn add_service_account_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountAddArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload = Value::Object(Map::from_iter(vec![
        ("name".to_string(), Value::String(args.name.clone())),
        (
            "role".to_string(),
            Value::String(service_account_role_to_api(&args.role)),
        ),
        ("isDisabled".to_string(), Value::Bool(args.disabled)),
    ]));
    let created =
        normalize_service_account_row(&create_service_account_with_request(&mut request_json, &payload)?);
    if args.json {
        println!("{}", render_objects_json(&[created])?);
    } else {
        println!(
            "Created service-account {} -> id={} role={} disabled={}",
            args.name,
            map_get_text(&created, "id"),
            map_get_text(&created, "role"),
            map_get_text(&created, "disabled")
        );
    }
    Ok(0)
}

pub(crate) fn add_service_account_token_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountTokenAddArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let service_account_id = match &args.service_account_id {
        Some(value) => value.clone(),
        None => lookup_service_account_id_by_name(&mut request_json, args.name.as_deref().unwrap_or(""))?,
    };
    let mut payload =
        Map::from_iter(vec![("name".to_string(), Value::String(args.token_name.clone()))]);
    if let Some(seconds) = args.seconds_to_live {
        payload.insert("secondsToLive".to_string(), Value::Number((seconds as i64).into()));
    }
    let mut token = create_service_account_token_with_request(
        &mut request_json,
        &service_account_id,
        &Value::Object(payload),
    )?;
    token.insert(
        "serviceAccountId".to_string(),
        Value::String(service_account_id.clone()),
    );
    if args.json {
        println!("{}", render_objects_json(&[token])?);
    } else {
        println!(
            "Created service-account token {} -> serviceAccountId={}",
            args.token_name, service_account_id
        );
    }
    Ok(0)
}
