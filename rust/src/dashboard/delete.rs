use reqwest::Method;
use serde_json::Value;

use crate::common::Result;
use crate::http::JsonHttpClient;

use super::delete_interactive::{confirm_live_delete, prepare_interactive_delete_args};
use super::delete_render::{
    format_live_dashboard_delete_line, format_live_folder_delete_line, render_delete_dry_run_json,
    render_delete_dry_run_table, render_delete_dry_run_text,
};
use super::delete_support::{build_delete_plan_with_request, validate_delete_args};
use super::live::{delete_dashboard_request_with_request, delete_folder_request_with_request};
use super::{build_http_client, build_http_client_for_org, DeleteArgs};

pub fn delete_dashboards_with_client(client: &JsonHttpClient, args: &DeleteArgs) -> Result<usize> {
    delete_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

pub(crate) fn delete_dashboards_with_org_clients(args: &DeleteArgs) -> Result<usize> {
    let client = match args.org_id {
        Some(org_id) => build_http_client_for_org(&args.common, org_id)?,
        None => build_http_client(&args.common)?,
    };
    delete_dashboards_with_client(&client, args)
}

pub(crate) fn delete_dashboards_with_request<F>(
    mut request_json: F,
    args: &DeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let effective_args = if args.interactive {
        prepare_interactive_delete_args(args)?
    } else {
        args.clone()
    };
    validate_delete_args(&effective_args)?;
    let plan = build_delete_plan_with_request(&mut request_json, &effective_args)?;

    if effective_args.dry_run {
        if effective_args.json {
            println!("{}", render_delete_dry_run_json(&plan)?);
        } else if effective_args.table {
            for line in render_delete_dry_run_table(&plan, !effective_args.no_header) {
                println!("{line}");
            }
        } else {
            for line in render_delete_dry_run_text(&plan) {
                println!("{line}");
            }
        }
        return Ok(plan.dashboards.len() + plan.folders.len());
    }

    if effective_args.interactive {
        for line in render_delete_dry_run_text(&plan) {
            println!("{line}");
        }
        if !confirm_live_delete()? {
            println!("Cancelled dashboard delete.");
            return Ok(0);
        }
    }

    for item in &plan.dashboards {
        let _ = delete_dashboard_request_with_request(&mut request_json, &item.uid)?;
        println!("{}", format_live_dashboard_delete_line(item));
    }
    for item in &plan.folders {
        let _ = delete_folder_request_with_request(&mut request_json, &item.uid)?;
        println!("{}", format_live_folder_delete_line(item));
    }
    println!(
        "Deleted {} dashboard(s){}",
        plan.dashboards.len(),
        if plan.folders.is_empty() {
            String::new()
        } else {
            format!(" and {} folder(s)", plan.folders.len())
        }
    );
    Ok(plan.dashboards.len() + plan.folders.len())
}
