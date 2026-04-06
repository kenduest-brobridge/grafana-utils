//! Listing and summary surface for Access resources.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::Result;

use crate::access::render::{
    format_table, map_get_text, normalize_team_row, paginate_rows, render_csv, render_objects_json,
    render_yaml, team_summary_line, team_table_rows,
};
use crate::access::team_import_export_diff::{
    build_record_diff_fields, build_team_diff_map, load_team_import_records,
};
use crate::access::team_runtime::{
    iter_teams_with_request, list_team_members_with_request, list_teams_with_request,
    team_member_identity,
};
use crate::access::{TeamDiffArgs, TeamListArgs, ACCESS_EXPORT_KIND_TEAMS};

pub(crate) fn diff_teams_with_request<F>(mut request_json: F, args: &TeamDiffArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let local_records = load_team_import_records(&args.diff_dir, ACCESS_EXPORT_KIND_TEAMS)?;
    let include_members = local_records.iter().any(|record| {
        (match record.get("members") {
            Some(Value::Array(values)) => !values.is_empty(),
            Some(Value::String(text)) => !text.trim().is_empty(),
            _ => false,
        }) || (match record.get("admins") {
            Some(Value::Array(values)) => !values.is_empty(),
            Some(Value::String(text)) => !text.trim().is_empty(),
            _ => false,
        })
    });
    let local_map = build_team_diff_map(
        &local_records,
        &args.diff_dir.to_string_lossy(),
        include_members,
    )?;
    let live_records = build_team_live_records_for_diff(&mut request_json, include_members)?;
    let live_map = build_team_diff_map(&live_records, "Grafana live teams", include_members)?;

    let mut differences = 0usize;
    let mut checked = 0usize;
    for key in local_map.keys() {
        checked += 1;
        let (local_identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                println!("Diff missing-live team {}", local_identity);
                differences += 1;
            }
            Some((_live_identity, live_payload)) => {
                let changed = build_record_diff_fields(local_payload, live_payload);
                if changed.is_empty() {
                    println!("Diff same team {}", local_identity);
                } else {
                    differences += 1;
                    println!(
                        "Diff different team {} fields={}",
                        local_identity,
                        changed.join(",")
                    );
                }
            }
        }
    }

    for key in live_map.keys() {
        if local_map.contains_key(key) {
            continue;
        }
        differences += 1;
        checked += 1;
        let (_, live_payload) = &live_map[key];
        println!(
            "Diff extra-live team {}",
            map_get_text(live_payload, "name")
        );
    }
    if differences > 0 {
        println!(
            "Diff checked {} team(s); {} difference(s) found.",
            checked, differences
        );
    } else {
        println!("No team differences across {} team(s).", checked);
    }
    Ok(differences)
}

pub(crate) fn list_teams_command_with_request<F>(
    mut request_json: F,
    args: &TeamListArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = list_teams_with_request(
        &mut request_json,
        args.query.as_deref(),
        args.page,
        args.per_page,
    )?
    .into_iter()
    .map(|team| normalize_team_row(&team))
    .collect::<Vec<Map<String, Value>>>();
    if let Some(name) = &args.name {
        rows.retain(|row| map_get_text(row, "name") == *name);
    }
    if args.with_members {
        for row in &mut rows {
            let team_id = map_get_text(row, "id");
            let members = list_team_members_with_request(&mut request_json, &team_id)?
                .into_iter()
                .map(|member| team_member_identity(&member))
                .filter(|identity| !identity.is_empty())
                .map(Value::String)
                .collect::<Vec<Value>>();
            row.insert("members".to_string(), Value::Array(members));
        }
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.yaml {
        println!("{}", render_yaml(&rows)?);
    } else if args.csv {
        for line in render_csv(
            &["id", "name", "email", "memberCount", "members"],
            &team_table_rows(&rows),
        ) {
            println!("{line}");
        }
    } else if args.table {
        for line in format_table(
            &["ID", "NAME", "EMAIL", "MEMBER_COUNT", "MEMBERS"],
            &team_table_rows(&rows),
        ) {
            println!("{line}");
        }
        println!();
        println!("Listed {} team(s) at {}", rows.len(), args.common.url);
    } else {
        for row in &rows {
            println!("{}", team_summary_line(row));
        }
        println!();
        println!("Listed {} team(s) at {}", rows.len(), args.common.url);
    }
    Ok(rows.len())
}

pub(crate) fn list_teams_from_input_dir(args: &TeamListArgs) -> Result<usize> {
    let input_dir = args
        .input_dir
        .as_ref()
        .expect("team local list requires input_dir");
    let mut rows = load_team_import_records(input_dir, ACCESS_EXPORT_KIND_TEAMS)?
        .into_iter()
        .map(|team| normalize_team_row(&team))
        .collect::<Vec<Map<String, Value>>>();
    if let Some(query) = &args.query {
        let query = query.to_ascii_lowercase();
        rows.retain(|row| {
            map_get_text(row, "name")
                .to_ascii_lowercase()
                .contains(&query)
                || map_get_text(row, "email")
                    .to_ascii_lowercase()
                    .contains(&query)
        });
    }
    if let Some(name) = &args.name {
        rows.retain(|row| map_get_text(row, "name") == *name);
    }
    if !args.with_members {
        for row in &mut rows {
            row.insert("members".to_string(), Value::Array(Vec::new()));
        }
    }
    let rows = paginate_rows(&rows, args.page, args.per_page);
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.yaml {
        println!("{}", render_yaml(&rows)?);
    } else if args.csv {
        for line in render_csv(
            &["id", "name", "email", "memberCount", "members"],
            &team_table_rows(&rows),
        ) {
            println!("{line}");
        }
    } else if args.table {
        for line in format_table(
            &["ID", "NAME", "EMAIL", "MEMBER_COUNT", "MEMBERS"],
            &team_table_rows(&rows),
        ) {
            println!("{line}");
        }
        println!();
        println!(
            "Listed {} team(s) from local bundle at {}",
            rows.len(),
            input_dir.display()
        );
    } else {
        for row in &rows {
            println!("{}", team_summary_line(row));
        }
        println!();
        println!(
            "Listed {} team(s) from local bundle at {}",
            rows.len(),
            input_dir.display()
        );
    }
    Ok(rows.len())
}

fn build_team_live_records_for_diff<F>(
    request_json: &mut F,
    include_members: bool,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = iter_teams_with_request(&mut *request_json, None)?
        .into_iter()
        .map(|team| normalize_team_row(&team))
        .collect::<Vec<Map<String, Value>>>();
    if include_members {
        for row in &mut rows {
            let team_id = map_get_text(row, "id");
            let members = list_team_members_with_request(&mut *request_json, &team_id)?
                .into_iter()
                .map(|member| team_member_identity(&member))
                .filter(|identity| !identity.is_empty())
                .map(Value::String)
                .collect::<Vec<Value>>();
            row.insert("members".to_string(), Value::Array(members));
        }
    }
    Ok(rows)
}
