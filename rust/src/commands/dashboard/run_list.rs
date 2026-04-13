//! Reusable dashboard list execution surface.
use crate::common::{render_json_value, Result};
use crate::tabular_output::render_yaml;
use serde_json::{json, Map, Value};

use super::{build_api_client, build_http_client, build_http_client_for_org};
use super::{build_http_client_for_org_from_api, list, ListArgs};

fn rendered_output_to_lines(output: String) -> Vec<String> {
    output
        .trim_end_matches('\n')
        .split('\n')
        .map(str::to_string)
        .collect()
}

pub(crate) fn collect_dashboard_list_summaries(args: &ListArgs) -> Result<Vec<Map<String, Value>>> {
    let mut summaries = Vec::new();
    if args.all_orgs {
        let admin_api = build_api_client(&args.common)?;
        let admin_client = admin_api.http_client();
        let orgs = list::list_orgs_with_request(|method, path, params, payload| {
            admin_client.request_json(method, path, params, payload)
        })?;
        for org in orgs {
            let org_id = list::org_id_value(&org)?;
            let org_client = build_http_client_for_org_from_api(&admin_api, org_id)?;
            let mut scoped = list::collect_list_dashboards_with_request(
                &mut |method, path, params, payload| {
                    org_client.request_json(method, path, params, payload)
                },
                args,
                Some(&org),
                None,
            )?;
            summaries.append(&mut scoped);
        }
        return Ok(summaries);
    }
    if let Some(org_id) = args.org_id {
        let org_client = build_http_client_for_org(&args.common, org_id)?;
        return list::collect_list_dashboards_with_request(
            &mut |method, path, params, payload| {
                org_client.request_json(method, path, params, payload)
            },
            args,
            None,
            None,
        );
    }
    let client = build_http_client(&args.common)?;
    list::collect_list_dashboards_with_request(
        &mut |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
        None,
        None,
    )
}

// Build a single dashboard list output document used by reusable execution callers.
pub fn execute_dashboard_list(args: &ListArgs) -> Result<super::DashboardWebRunOutput> {
    let summaries = collect_dashboard_list_summaries(args)?;
    let rows = list::render_dashboard_summary_json(&summaries, &args.output_columns);
    let text_lines = if args.json {
        rendered_output_to_lines(render_json_value(&rows)?)
    } else if args.yaml {
        rendered_output_to_lines(render_yaml(&rows)?)
    } else if args.csv {
        list::render_dashboard_summary_csv(&summaries, &args.output_columns)
    } else if args.text {
        let mut lines = summaries
            .iter()
            .map(list::format_dashboard_summary_line)
            .collect::<Vec<String>>();
        lines.push(String::new());
        lines.push(format!("Listed {} dashboard(s).", summaries.len()));
        lines
    } else {
        let mut lines =
            list::render_dashboard_summary_table(&summaries, &args.output_columns, !args.no_header);
        lines.push(String::new());
        lines.push(format!("Listed {} dashboard(s).", summaries.len()));
        lines
    };
    Ok(super::DashboardWebRunOutput {
        document: json!({
            "kind": "grafana-utils-dashboard-list",
            "dashboardCount": summaries.len(),
            "rows": rows,
        }),
        text_lines,
    })
}
