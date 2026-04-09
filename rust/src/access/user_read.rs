//! Read and render Grafana users for org and global admin scopes.
//! This module fetches users from the org or admin endpoints, paginates global user
//! listings, and turns the results into table, CSV, YAML, JSON, or text output.
//! It owns user filtering and summary rendering, but not mutation commands.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, Result};

use super::render::{
    format_table, map_get_text, normalize_user_row, paginate_rows, render_csv, render_objects_json,
    render_yaml, scalar_text, user_account_scope_text, user_list_column_ids, user_matches,
    user_scope_text, user_summary_line, user_table_headers, user_table_rows,
};
use super::user_workflows::load_access_import_records;
use super::{build_auth_context, request_array, Scope, UserListArgs, DEFAULT_PAGE_SIZE};
use crate::access::ACCESS_EXPORT_KIND_USERS;

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

pub(crate) fn iter_global_users_with_request<F>(
    mut request_json: F,
    page_size: usize,
) -> Result<Vec<Map<String, Value>>>
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

pub(crate) fn list_user_teams_with_request<F>(
    mut request_json: F,
    user_id: &str,
) -> Result<Vec<Map<String, Value>>>
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

pub(crate) fn lookup_global_user_by_identity<F>(
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

/// lookup org user by identity.
pub(crate) fn lookup_org_user_by_identity<F>(
    mut request_json: F,
    identity: &str,
) -> Result<Map<String, Value>>
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

pub(crate) fn validate_user_scope_auth(
    scope: &Scope,
    with_teams: bool,
    auth_mode: &str,
) -> Result<()> {
    if *scope == Scope::Global && auth_mode != "basic" {
        return Err(message(
            "User list with --scope global requires Basic auth (--basic-user / --basic-password).",
        ));
    }
    if with_teams && auth_mode != "basic" {
        return Err(message("--with-teams requires Basic auth."));
    }
    Ok(())
}

pub(crate) fn annotate_user_account_scope(rows: &mut [Map<String, Value>]) {
    for row in rows {
        row.insert(
            "accountScope".to_string(),
            Value::String(user_account_scope_text().to_string()),
        );
    }
}

pub(crate) fn list_users_with_request<F>(mut request_json: F, args: &UserListArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.list_columns {
        println!("{}", user_list_column_ids().join("\n"));
        return Ok(0);
    }
    if !args.output_columns.is_empty() && (args.json || args.yaml) {
        return Err(message(
            "--output-columns is only supported with text, table, or csv output for access user list.",
        ));
    }
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_scope_auth(&args.scope, args.with_teams, &auth_mode)?;
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
    annotate_user_account_scope(&mut rows);
    rows.retain(|row| user_matches(row, args));
    let rows = paginate_rows(&rows, args.page, args.per_page);
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.yaml {
        println!("{}", render_yaml(&rows)?);
    } else if args.csv {
        let headers = user_table_headers(&args.output_columns);
        for line in render_csv(&headers, &user_table_rows(&rows, &args.output_columns)) {
            println!("{line}");
        }
    } else if args.table {
        let headers = user_table_headers(&args.output_columns);
        for line in format_table(&headers, &user_table_rows(&rows, &args.output_columns)) {
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
            println!("{}", user_summary_line(row, &args.output_columns));
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

fn local_user_scope(row: &Map<String, Value>, args: &UserListArgs) -> Scope {
    match string_field(row, "scope", "").to_ascii_lowercase().as_str() {
        "global" => Scope::Global,
        "org" => Scope::Org,
        _ => args.scope.clone(),
    }
}

pub(crate) fn list_users_from_input_dir(args: &UserListArgs) -> Result<usize> {
    if args.list_columns {
        println!("{}", user_list_column_ids().join("\n"));
        return Ok(0);
    }
    if !args.output_columns.is_empty() && (args.json || args.yaml) {
        return Err(message(
            "--output-columns is only supported with text, table, or csv output for access user list.",
        ));
    }
    let input_dir = args
        .input_dir
        .as_ref()
        .ok_or_else(|| message("User list local mode requires --input-dir."))?;
    let mut rows = load_access_import_records(input_dir, ACCESS_EXPORT_KIND_USERS)?
        .into_iter()
        .map(|item| {
            let scope = local_user_scope(&item, args);
            normalize_user_row(&item, &scope)
        })
        .collect::<Vec<Map<String, Value>>>();
    if !args.with_teams {
        for row in &mut rows {
            row.insert("teams".to_string(), Value::Array(Vec::<Value>::new()));
        }
    }
    annotate_user_account_scope(&mut rows);
    rows.retain(|row| user_matches(row, args));
    let rows = paginate_rows(&rows, args.page, args.per_page);
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.yaml {
        println!("{}", render_yaml(&rows)?);
    } else if args.csv {
        let headers = user_table_headers(&args.output_columns);
        for line in render_csv(&headers, &user_table_rows(&rows, &args.output_columns)) {
            println!("{line}");
        }
    } else if args.table {
        let headers = user_table_headers(&args.output_columns);
        for line in format_table(&headers, &user_table_rows(&rows, &args.output_columns)) {
            println!("{line}");
        }
        println!();
        println!(
            "Listed {} user(s) from local bundle at {}",
            rows.len(),
            input_dir.display()
        );
    } else {
        for row in &rows {
            println!("{}", user_summary_line(row, &args.output_columns));
        }
        println!();
        println!(
            "Listed {} user(s) from local bundle at {}",
            rows.len(),
            input_dir.display()
        );
    }
    Ok(rows.len())
}
