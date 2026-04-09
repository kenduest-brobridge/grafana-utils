//! Mutation builders and payload plumbing for Access updates.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};

use super::super::super::render::{
    format_table, map_get_text, normalize_service_account_row, paginate_rows, render_csv,
    render_objects_json, render_yaml, service_account_list_column_ids, service_account_role_to_api,
    service_account_summary_line, service_account_table_headers, service_account_table_rows,
};
use super::super::super::{
    ServiceAccountAddArgs, ServiceAccountListArgs, ServiceAccountTokenAddArgs,
};
use super::super::{
    create_service_account_token_with_request, create_service_account_with_request,
    list_service_accounts_with_request, lookup_service_account_id_by_name,
};
use super::service_account_workflows_support::{
    load_service_account_import_records, render_single_object_json,
};
use crate::access::ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS;

/// Purpose: implementation note.
pub(crate) fn list_service_accounts_command_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountListArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.list_columns {
        println!("{}", service_account_list_column_ids().join("\n"));
        return Ok(0);
    }
    if !args.output_columns.is_empty() && (args.json || args.yaml) {
        return Err(message(
            "--output-columns is only supported with text, table, or csv output for access service-account list.",
        ));
    }
    let mut rows = list_service_accounts_with_request(
        &mut request_json,
        args.query.as_deref(),
        args.page,
        args.per_page,
    )?
    .into_iter()
    .map(|item| normalize_service_account_row(&item))
    .collect::<Vec<Map<String, Value>>>();
    if let Some(query) = &args.query {
        let query = query.to_ascii_lowercase();
        rows.retain(|row| {
            map_get_text(row, "name")
                .to_ascii_lowercase()
                .contains(&query)
                || map_get_text(row, "login")
                    .to_ascii_lowercase()
                    .contains(&query)
        });
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.yaml {
        println!("{}", render_yaml(&rows)?);
    } else if args.csv {
        let headers = service_account_table_headers(&args.output_columns);
        for line in render_csv(
            &headers,
            &service_account_table_rows(&rows, &args.output_columns),
        ) {
            println!("{line}");
        }
    } else if args.table {
        let headers = service_account_table_headers(&args.output_columns);
        for line in format_table(
            &headers,
            &service_account_table_rows(&rows, &args.output_columns),
        ) {
            println!("{line}");
        }
        println!();
        println!(
            "Listed {} service account(s) at {}",
            rows.len(),
            args.common.url
        );
    } else {
        for row in &rows {
            println!(
                "{}",
                service_account_summary_line(row, &args.output_columns)
            );
        }
        println!();
        println!(
            "Listed {} service account(s) at {}",
            rows.len(),
            args.common.url
        );
    }
    Ok(rows.len())
}

pub(crate) fn list_service_accounts_from_input_dir(args: &ServiceAccountListArgs) -> Result<usize> {
    if args.list_columns {
        println!("{}", service_account_list_column_ids().join("\n"));
        return Ok(0);
    }
    if !args.output_columns.is_empty() && (args.json || args.yaml) {
        return Err(message(
            "--output-columns is only supported with text, table, or csv output for access service-account list.",
        ));
    }
    let input_dir = args
        .input_dir
        .as_ref()
        .expect("service-account local list requires input_dir");
    let mut rows =
        load_service_account_import_records(input_dir, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS)?
            .into_iter()
            .map(|item| normalize_service_account_row(&item))
            .collect::<Vec<Map<String, Value>>>();
    if let Some(query) = &args.query {
        let query = query.to_ascii_lowercase();
        rows.retain(|row| {
            map_get_text(row, "name")
                .to_ascii_lowercase()
                .contains(&query)
                || map_get_text(row, "login")
                    .to_ascii_lowercase()
                    .contains(&query)
        });
    }
    let rows = paginate_rows(&rows, args.page, args.per_page);
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.yaml {
        println!("{}", render_yaml(&rows)?);
    } else if args.csv {
        let headers = service_account_table_headers(&args.output_columns);
        for line in render_csv(
            &headers,
            &service_account_table_rows(&rows, &args.output_columns),
        ) {
            println!("{line}");
        }
    } else if args.table {
        let headers = service_account_table_headers(&args.output_columns);
        for line in format_table(
            &headers,
            &service_account_table_rows(&rows, &args.output_columns),
        ) {
            println!("{line}");
        }
        println!();
        println!(
            "Listed {} service account(s) from local bundle at {}",
            rows.len(),
            input_dir.display()
        );
    } else {
        for row in &rows {
            println!(
                "{}",
                service_account_summary_line(row, &args.output_columns)
            );
        }
        println!();
        println!(
            "Listed {} service account(s) from local bundle at {}",
            rows.len(),
            input_dir.display()
        );
    }
    Ok(rows.len())
}

/// Purpose: implementation note.
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
    let created = normalize_service_account_row(&create_service_account_with_request(
        &mut request_json,
        &payload,
    )?);
    if args.json {
        println!("{}", render_single_object_json(&created)?);
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

/// Purpose: implementation note.
pub(crate) fn add_service_account_token_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountTokenAddArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let service_account_id = match &args.service_account_id {
        Some(value) => value.clone(),
        None => lookup_service_account_id_by_name(
            &mut request_json,
            args.name.as_deref().unwrap_or(""),
        )?,
    };
    let mut payload = Map::from_iter(vec![(
        "name".to_string(),
        Value::String(args.token_name.clone()),
    )]);
    if let Some(seconds) = args.seconds_to_live {
        payload.insert(
            "secondsToLive".to_string(),
            Value::Number((seconds as i64).into()),
        );
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
        println!("{}", render_single_object_json(&token)?);
    } else {
        println!(
            "Created service-account token {} -> serviceAccountId={}",
            args.token_name, service_account_id
        );
    }
    Ok(0)
}
