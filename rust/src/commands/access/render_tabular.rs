use serde_json::{Map, Value};

use crate::common::{render_json_value, requested_columns_include_all, Result};

use super::normalization::{bool_label, map_get_text};
use crate::access::UserListArgs;

const USER_LIST_COLUMNS: [(&str, &str, &str); 11] = [
    ("id", "ID", "id"),
    ("login", "LOGIN", "login"),
    ("email", "EMAIL", "email"),
    ("name", "NAME", "name"),
    ("org_role", "ORG_ROLE", "orgRole"),
    ("grafana_admin", "GRAFANA_ADMIN", "grafanaAdmin"),
    ("scope", "SCOPE", "scope"),
    ("account_scope", "ACCOUNT_SCOPE", "accountScope"),
    ("origin", "ORIGIN", "origin.kind"),
    ("last_active", "LAST_ACTIVE", "lastActive.at"),
    ("teams", "TEAMS", "teams"),
];

const TEAM_LIST_COLUMNS: [(&str, &str, &str); 5] = [
    ("id", "ID", "id"),
    ("name", "NAME", "name"),
    ("email", "EMAIL", "email"),
    ("member_count", "MEMBER_COUNT", "memberCount"),
    ("members", "MEMBERS", "members"),
];

const SERVICE_ACCOUNT_LIST_COLUMNS: [(&str, &str, &str); 7] = [
    ("id", "ID", "id"),
    ("name", "NAME", "name"),
    ("login", "LOGIN", "login"),
    ("role", "ROLE", "role"),
    ("disabled", "DISABLED", "disabled"),
    ("tokens", "TOKENS", "tokens"),
    ("org_id", "ORG_ID", "orgId"),
];

/// format table.
pub(crate) fn format_table(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let header_row = headers
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<String>>();
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>();
    let mut lines = vec![format_row(&header_row), format_row(&separator)];
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

fn csv_escape(value: String) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

/// Purpose: implementation note.
pub(crate) fn render_csv(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    let mut lines = vec![headers.join(",")];
    lines.extend(rows.iter().map(|row| {
        row.iter()
            .cloned()
            .map(csv_escape)
            .collect::<Vec<String>>()
            .join(",")
    }));
    lines
}

fn map_get_path_text(map: &Map<String, Value>, path: &str) -> String {
    let mut current = None;
    for (index, key) in path.split('.').enumerate() {
        current = if index == 0 {
            map.get(key)
        } else {
            match current {
                Some(Value::Object(inner)) => inner.get(key),
                _ => None,
            }
        };
    }
    match current {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<&str>>()
            .join(","),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn resolve_list_columns<'a>(
    supported: &'a [(&'a str, &'a str, &'a str)],
    requested: &[String],
) -> Vec<(&'a str, &'a str, &'a str)> {
    if requested.is_empty() {
        return supported.to_vec();
    }
    if requested_columns_include_all(requested) {
        return supported.to_vec();
    }
    requested
        .iter()
        .filter_map(|value| supported.iter().copied().find(|(id, _, _)| id == value))
        .collect()
}

fn build_table_rows_for_columns(
    rows: &[Map<String, Value>],
    columns: &[(&str, &str, &str)],
) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            columns
                .iter()
                .map(|(_, _, path)| map_get_path_text(row, path))
                .collect::<Vec<String>>()
        })
        .collect()
}

fn build_summary_line_for_columns(
    row: &Map<String, Value>,
    columns: &[(&str, &str, &str)],
) -> String {
    columns
        .iter()
        .filter_map(|(id, _, path)| {
            let value = map_get_path_text(row, path);
            if value.is_empty() {
                None
            } else {
                Some(format!("{id}={value}"))
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

pub(crate) fn user_list_column_ids() -> &'static [&'static str] {
    &[
        "id",
        "login",
        "email",
        "name",
        "org_role",
        "grafana_admin",
        "scope",
        "account_scope",
        "origin",
        "last_active",
        "teams",
    ]
}

pub(crate) fn team_list_column_ids() -> &'static [&'static str] {
    &["id", "name", "email", "member_count", "members"]
}

pub(crate) fn service_account_list_column_ids() -> &'static [&'static str] {
    &[
        "id", "name", "login", "role", "disabled", "tokens", "org_id",
    ]
}

/// Purpose: implementation note.
pub(crate) fn render_objects_json(rows: &[Map<String, Value>]) -> Result<String> {
    render_json_value(&Value::Array(
        rows.iter().cloned().map(Value::Object).collect(),
    ))
}

/// user table rows.
pub(crate) fn user_table_rows(
    rows: &[Map<String, Value>],
    requested_columns: &[String],
) -> Vec<Vec<String>> {
    let columns = resolve_list_columns(&USER_LIST_COLUMNS, requested_columns);
    build_table_rows_for_columns(rows, &columns)
}

pub(crate) fn user_table_headers(requested_columns: &[String]) -> Vec<&'static str> {
    resolve_list_columns(&USER_LIST_COLUMNS, requested_columns)
        .into_iter()
        .map(|(_, header, _)| header)
        .collect()
}

/// team table rows.
pub(crate) fn team_table_rows(
    rows: &[Map<String, Value>],
    requested_columns: &[String],
) -> Vec<Vec<String>> {
    let columns = resolve_list_columns(&TEAM_LIST_COLUMNS, requested_columns);
    build_table_rows_for_columns(rows, &columns)
}

pub(crate) fn team_table_headers(requested_columns: &[String]) -> Vec<&'static str> {
    resolve_list_columns(&TEAM_LIST_COLUMNS, requested_columns)
        .into_iter()
        .map(|(_, header, _)| header)
        .collect()
}

/// service account table rows.
pub(crate) fn service_account_table_rows(
    rows: &[Map<String, Value>],
    requested_columns: &[String],
) -> Vec<Vec<String>> {
    let columns = resolve_list_columns(&SERVICE_ACCOUNT_LIST_COLUMNS, requested_columns);
    build_table_rows_for_columns(rows, &columns)
}

pub(crate) fn service_account_table_headers(requested_columns: &[String]) -> Vec<&'static str> {
    resolve_list_columns(&SERVICE_ACCOUNT_LIST_COLUMNS, requested_columns)
        .into_iter()
        .map(|(_, header, _)| header)
        .collect()
}

/// user summary line.
pub(crate) fn user_summary_line(row: &Map<String, Value>, requested_columns: &[String]) -> String {
    if requested_columns.is_empty() {
        let mut parts = vec![
            format!("id={}", map_get_text(row, "id")),
            format!("login={}", map_get_text(row, "login")),
        ];
        let email = map_get_text(row, "email");
        if !email.is_empty() {
            parts.push(format!("email={email}"));
        }
        let name = map_get_text(row, "name");
        if !name.is_empty() {
            parts.push(format!("name={name}"));
        }
        let role = map_get_text(row, "orgRole");
        if !role.is_empty() {
            parts.push(format!("orgRole={role}"));
        }
        let admin = map_get_text(row, "grafanaAdmin");
        if !admin.is_empty() {
            parts.push(format!("grafanaAdmin={admin}"));
        }
        let account_scope = map_get_text(row, "accountScope");
        if !account_scope.is_empty() {
            parts.push(format!("accountScope={account_scope}"));
        }
        let teams = map_get_text(row, "teams");
        if !teams.is_empty() {
            parts.push(format!("teams={teams}"));
        }
        parts.push(format!("scope={}", map_get_text(row, "scope")));
        return parts.join(" ");
    }
    let columns = resolve_list_columns(&USER_LIST_COLUMNS, requested_columns);
    build_summary_line_for_columns(row, &columns)
}

/// team summary line.
pub(crate) fn team_summary_line(row: &Map<String, Value>, requested_columns: &[String]) -> String {
    if requested_columns.is_empty() {
        let mut parts = vec![
            format!("id={}", map_get_text(row, "id")),
            format!("name={}", map_get_text(row, "name")),
        ];
        let email = map_get_text(row, "email");
        if !email.is_empty() {
            parts.push(format!("email={email}"));
        }
        parts.push(format!("memberCount={}", map_get_text(row, "memberCount")));
        let members = map_get_text(row, "members");
        if !members.is_empty() {
            parts.push(format!("members={members}"));
        }
        return parts.join(" ");
    }
    let columns = resolve_list_columns(&TEAM_LIST_COLUMNS, requested_columns);
    build_summary_line_for_columns(row, &columns)
}

/// service account summary line.
pub(crate) fn service_account_summary_line(
    row: &Map<String, Value>,
    requested_columns: &[String],
) -> String {
    if requested_columns.is_empty() {
        let mut parts = vec![
            format!("id={}", map_get_text(row, "id")),
            format!("name={}", map_get_text(row, "name")),
        ];
        let login = map_get_text(row, "login");
        if !login.is_empty() {
            parts.push(format!("login={login}"));
        }
        parts.push(format!("role={}", map_get_text(row, "role")));
        parts.push(format!("disabled={}", map_get_text(row, "disabled")));
        parts.push(format!("tokens={}", map_get_text(row, "tokens")));
        let org_id = map_get_text(row, "orgId");
        if !org_id.is_empty() {
            parts.push(format!("orgId={org_id}"));
        }
        return parts.join(" ");
    }
    let columns = resolve_list_columns(&SERVICE_ACCOUNT_LIST_COLUMNS, requested_columns);
    build_summary_line_for_columns(row, &columns)
}

fn exact_text_matches(text: &str, filter: &Option<String>) -> bool {
    match filter {
        Some(value) => text == value,
        None => true,
    }
}

/// user matches.
pub(crate) fn user_matches(row: &Map<String, Value>, args: &UserListArgs) -> bool {
    let login = map_get_text(row, "login");
    let email = map_get_text(row, "email");
    let name = map_get_text(row, "name");
    if let Some(query) = &args.query {
        let query = query.to_ascii_lowercase();
        if !login.to_ascii_lowercase().contains(&query)
            && !email.to_ascii_lowercase().contains(&query)
            && !name.to_ascii_lowercase().contains(&query)
        {
            return false;
        }
    }
    if !exact_text_matches(&login, &args.login) {
        return false;
    }
    if !exact_text_matches(&email, &args.email) {
        return false;
    }
    if let Some(role) = &args.org_role {
        if map_get_text(row, "orgRole") != *role {
            return false;
        }
    }
    if let Some(admin) = args.grafana_admin {
        if map_get_text(row, "grafanaAdmin") != bool_label(Some(admin)) {
            return false;
        }
    }
    true
}

/// paginate rows.
pub(crate) fn paginate_rows(
    rows: &[Map<String, Value>],
    page: usize,
    per_page: usize,
) -> Vec<Map<String, Value>> {
    let start = per_page.saturating_mul(page.saturating_sub(1));
    rows.iter().skip(start).take(per_page).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::{format_table, paginate_rows, render_csv};
    use serde_json::json;
    #[test]
    fn format_table_pads_columns_consistently() {
        let lines = format_table(
            &["ID", "NAME"],
            &[vec!["1".to_string(), "alice".to_string()]],
        );

        assert_eq!(lines[0], "ID  NAME ");
        assert_eq!(lines[1], "--  -----");
        assert_eq!(lines[2], "1   alice");
    }

    #[test]
    fn render_csv_quotes_commas_and_quotes() {
        let lines = render_csv(
            &["id", "name"],
            &[vec!["1".to_string(), "a,b\"c".to_string()]],
        );

        assert_eq!(lines, vec!["id,name", "1,\"a,b\"\"c\""]);
    }

    #[test]
    fn paginate_rows_returns_expected_slice() {
        let rows = vec![
            serde_json::from_value(json!({"id": "1"})).unwrap(),
            serde_json::from_value(json!({"id": "2"})).unwrap(),
            serde_json::from_value(json!({"id": "3"})).unwrap(),
        ];

        let page = paginate_rows(&rows, 2, 1);
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].get("id"), Some(&json!("2")));
    }
}
