//! Alert list commands and rendering.
//! Converts alerting resources (rules, contact points, templates, timings) into compact table/JSON output.
use reqwest::header::AUTHORIZATION;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::common::{message, string_field, Result};

use super::{
    build_auth_context, AlertAuthContext, AlertCliArgs, AlertListKind, GrafanaAlertClient,
};

const ALERT_RULE_LIST_FIELDS: [&str; 4] = ["uid", "title", "folderUID", "ruleGroup"];
const CONTACT_POINT_LIST_FIELDS: [&str; 3] = ["uid", "name", "type"];
const MUTE_TIMING_LIST_FIELDS: [&str; 2] = ["name", "intervals"];
const TEMPLATE_LIST_FIELDS: [&str; 1] = ["name"];
const ORG_SCOPE_FIELDS: [&str; 2] = ["org", "orgId"];

fn auth_header_is_basic(headers: &[(String, String)]) -> bool {
    headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case(AUTHORIZATION.as_str()) && value.starts_with("Basic ")
    })
}

fn build_alert_client_for_org(
    context: &AlertAuthContext,
    org_id: i64,
) -> Result<GrafanaAlertClient> {
    let mut scoped = context.clone();
    scoped
        .headers
        .push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    GrafanaAlertClient::new(&scoped)
}

fn org_name_value(org: &Map<String, Value>) -> String {
    string_field(org, "name", "")
}

fn org_id_value(org: &Map<String, Value>) -> Result<i64> {
    org.get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Grafana org list entry is missing numeric id."))
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn render_alert_table(
    rows: &[BTreeMap<&str, String>],
    fields: &[&str],
    headers: &[(&str, &str)],
    include_header: bool,
) -> String {
    let mut widths = BTreeMap::new();
    for (field, header) in headers {
        let mut width = header.len();
        for row in rows {
            width = width.max(row.get(field).map(|item| item.len()).unwrap_or(0));
        }
        widths.insert(*field, width);
    }

    let build_row = |values: &BTreeMap<&str, String>| -> String {
        fields
            .iter()
            .map(|field| {
                let width = *widths.get(field).unwrap_or(&0);
                format!(
                    "{:width$}",
                    values.get(field).map(String::as_str).unwrap_or(""),
                    width = width
                )
            })
            .collect::<Vec<_>>()
            .join("  ")
    };

    let mut lines = Vec::new();
    if include_header {
        let header_map = headers
            .iter()
            .map(|(field, header)| (*field, header.to_string()))
            .collect::<BTreeMap<_, _>>();
        lines.push(build_row(&header_map));
        lines.push(
            fields
                .iter()
                .map(|field| "-".repeat(*widths.get(field).unwrap_or(&0)))
                .collect::<Vec<_>>()
                .join("  "),
        );
    }
    for row in rows {
        lines.push(build_row(row));
    }
    lines.join("\n")
}

fn render_alert_csv(rows: &[BTreeMap<&str, String>], fields: &[&str]) -> String {
    let mut output = String::new();
    let _ = writeln!(&mut output, "{}", fields.join(","));
    for row in rows {
        let line = fields
            .iter()
            .map(|field| csv_escape(row.get(field).map(String::as_str).unwrap_or("")))
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(&mut output, "{line}");
    }
    output
}

/// serialize rule list rows.
pub(crate) fn serialize_rule_list_rows(
    items: &[Map<String, Value>],
) -> Vec<BTreeMap<&'static str, String>> {
    items
        .iter()
        .map(|item| {
            let mut row = BTreeMap::from([
                ("uid", string_field(item, "uid", "")),
                ("title", string_field(item, "title", "")),
                ("folderUID", string_field(item, "folderUID", "")),
                ("ruleGroup", string_field(item, "ruleGroup", "")),
            ]);
            if let Some(org) = item.get("org").and_then(Value::as_object) {
                row.insert("org", org_name_value(org));
                row.insert("orgId", org_id_value(org).unwrap_or_default().to_string());
            }
            row
        })
        .collect()
}

fn serialize_contact_point_list_rows(
    items: &[Map<String, Value>],
) -> Vec<BTreeMap<&'static str, String>> {
    items
        .iter()
        .map(|item| {
            let mut row = BTreeMap::from([
                ("uid", string_field(item, "uid", "")),
                ("name", string_field(item, "name", "")),
                ("type", string_field(item, "type", "")),
            ]);
            if let Some(org) = item.get("org").and_then(Value::as_object) {
                row.insert("org", org_name_value(org));
                row.insert("orgId", org_id_value(org).unwrap_or_default().to_string());
            }
            row
        })
        .collect()
}

fn serialize_mute_timing_list_rows(
    items: &[Map<String, Value>],
) -> Vec<BTreeMap<&'static str, String>> {
    items
        .iter()
        .map(|item| {
            let intervals = item
                .get("time_intervals")
                .and_then(Value::as_array)
                .map(|value| value.len())
                .unwrap_or(0);
            let mut row = BTreeMap::from([
                ("name", string_field(item, "name", "")),
                ("intervals", intervals.to_string()),
            ]);
            if let Some(org) = item.get("org").and_then(Value::as_object) {
                row.insert("org", org_name_value(org));
                row.insert("orgId", org_id_value(org).unwrap_or_default().to_string());
            }
            row
        })
        .collect()
}

fn serialize_template_list_rows(
    items: &[Map<String, Value>],
) -> Vec<BTreeMap<&'static str, String>> {
    items
        .iter()
        .map(|item| {
            let mut row = BTreeMap::from([("name", string_field(item, "name", ""))]);
            if let Some(org) = item.get("org").and_then(Value::as_object) {
                row.insert("org", org_name_value(org));
                row.insert("orgId", org_id_value(org).unwrap_or_default().to_string());
            }
            row
        })
        .collect()
}

fn append_org_scope(
    items: Vec<Map<String, Value>>,
    org: &Map<String, Value>,
) -> Vec<Map<String, Value>> {
    items
        .into_iter()
        .map(|mut item| {
            item.insert("org".to_string(), Value::Object(org.clone()));
            item
        })
        .collect()
}

fn rows_include_org_scope(rows: &[BTreeMap<&'static str, String>]) -> bool {
    rows.iter()
        .any(|row| row.contains_key("org") || row.contains_key("orgId"))
}

fn fields_with_org_scope(base: &[&'static str], include_org_scope: bool) -> Vec<&'static str> {
    if include_org_scope {
        ORG_SCOPE_FIELDS
            .iter()
            .copied()
            .chain(base.iter().copied())
            .collect()
    } else {
        base.to_vec()
    }
}

fn headers_with_org_scope(
    base: &[(&'static str, &'static str)],
    include_org_scope: bool,
) -> Vec<(&'static str, &'static str)> {
    if include_org_scope {
        [("org", "ORG"), ("orgId", "ORG_ID")]
            .into_iter()
            .chain(base.iter().copied())
            .collect()
    } else {
        base.to_vec()
    }
}

fn list_items_for_kind(
    client: &GrafanaAlertClient,
    kind: AlertListKind,
) -> Result<Vec<Map<String, Value>>> {
    match kind {
        AlertListKind::Rules => client.list_alert_rules(),
        AlertListKind::ContactPoints => client.list_contact_points(),
        AlertListKind::MuteTimings => client.list_mute_timings(),
        AlertListKind::Templates => client.list_templates(),
    }
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn list_alert_resources(args: &AlertCliArgs) -> Result<()> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: alert.rs:run_alert_cli
    // Downstream callees: alert_list.rs:append_org_scope, alert_list.rs:auth_header_is_basic, alert_list.rs:build_alert_client_for_org, alert_list.rs:fields_with_org_scope, alert_list.rs:headers_with_org_scope, alert_list.rs:list_items_for_kind, alert_list.rs:org_id_value, alert_list.rs:render_alert_csv, alert_list.rs:render_alert_table, alert_list.rs:rows_include_org_scope, alert_list.rs:serialize_contact_point_list_rows, alert_list.rs:serialize_mute_timing_list_rows ...

    let kind = args
        .list_kind
        .ok_or_else(|| message("Alert list command is required."))?;
    let context = build_auth_context(args)?;
    let items = if args.all_orgs {
        if !auth_header_is_basic(&context.headers) {
            return Err(message(
                "Alert list with --all-orgs requires Basic auth (--basic-user / --basic-password).",
            ));
        }
        let admin_client = GrafanaAlertClient::new(&context)?;
        let mut rows = Vec::new();
        for org in admin_client.list_orgs()? {
            let org_id = org_id_value(&org)?;
            let org_client = build_alert_client_for_org(&context, org_id)?;
            rows.extend(append_org_scope(
                list_items_for_kind(&org_client, kind)?,
                &org,
            ));
        }
        rows
    } else if let Some(org_id) = args.org_id {
        if !auth_header_is_basic(&context.headers) {
            return Err(message(
                "Alert list with --org-id requires Basic auth (--basic-user / --basic-password).",
            ));
        }
        let org_client = build_alert_client_for_org(&context, org_id)?;
        list_items_for_kind(&org_client, kind)?
    } else {
        let client = GrafanaAlertClient::new(&context)?;
        list_items_for_kind(&client, kind)?
    };
    match kind {
        AlertListKind::Rules => {
            let rows = serialize_rule_list_rows(&items);
            let include_org_scope = rows_include_org_scope(&rows);
            let fields = fields_with_org_scope(&ALERT_RULE_LIST_FIELDS, include_org_scope);
            let headers = headers_with_org_scope(
                &[
                    ("uid", "UID"),
                    ("title", "TITLE"),
                    ("folderUID", "FOLDER_UID"),
                    ("ruleGroup", "RULE_GROUP"),
                ],
                include_org_scope,
            );
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else if args.csv {
                print!("{}", render_alert_csv(&rows, &fields));
            } else {
                println!(
                    "{}",
                    render_alert_table(&rows, &fields, &headers, !args.no_header,)
                );
            }
        }
        AlertListKind::ContactPoints => {
            let rows = serialize_contact_point_list_rows(&items);
            let include_org_scope = rows_include_org_scope(&rows);
            let fields = fields_with_org_scope(&CONTACT_POINT_LIST_FIELDS, include_org_scope);
            let headers = headers_with_org_scope(
                &[("uid", "UID"), ("name", "NAME"), ("type", "TYPE")],
                include_org_scope,
            );
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else if args.csv {
                print!("{}", render_alert_csv(&rows, &fields));
            } else {
                println!(
                    "{}",
                    render_alert_table(&rows, &fields, &headers, !args.no_header,)
                );
            }
        }
        AlertListKind::MuteTimings => {
            let rows = serialize_mute_timing_list_rows(&items);
            let include_org_scope = rows_include_org_scope(&rows);
            let fields = fields_with_org_scope(&MUTE_TIMING_LIST_FIELDS, include_org_scope);
            let headers = headers_with_org_scope(
                &[("name", "NAME"), ("intervals", "INTERVALS")],
                include_org_scope,
            );
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else if args.csv {
                print!("{}", render_alert_csv(&rows, &fields));
            } else {
                println!(
                    "{}",
                    render_alert_table(&rows, &fields, &headers, !args.no_header,)
                );
            }
        }
        AlertListKind::Templates => {
            let rows = serialize_template_list_rows(&items);
            let include_org_scope = rows_include_org_scope(&rows);
            let fields = fields_with_org_scope(&TEMPLATE_LIST_FIELDS, include_org_scope);
            let headers = headers_with_org_scope(&[("name", "NAME")], include_org_scope);
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else if args.csv {
                print!("{}", render_alert_csv(&rows, &fields));
            } else {
                println!(
                    "{}",
                    render_alert_table(&rows, &fields, &headers, !args.no_header,)
                );
            }
        }
    }
    Ok(())
}
