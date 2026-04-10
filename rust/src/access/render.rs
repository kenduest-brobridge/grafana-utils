//! Shared render/format helpers for access CLI output.
//! Centralizes table/csv/json field normalization and human-facing value formatting.
use serde_json::{Map, Value};
use std::collections::BTreeSet;

use crate::common::{render_json_value, requested_columns_include_all, string_field};
pub(crate) use crate::tabular_output::render_yaml;

use super::Scope;

/// bool label.
pub(crate) fn bool_label(value: Option<bool>) -> String {
    match value {
        Some(true) => "true".to_string(),
        Some(false) => "false".to_string(),
        None => String::new(),
    }
}

/// scalar text.
pub(crate) fn scalar_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

/// value bool.
pub(crate) fn value_bool(value: Option<&Value>) -> Option<bool> {
    match value {
        Some(Value::Bool(v)) => Some(*v),
        Some(Value::String(text)) => match text.to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        Some(Value::Number(number)) => match number.as_i64() {
            Some(1) => Some(true),
            Some(0) => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn value_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        _ => None,
    }
}

fn value_string_array(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Value::String(value.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

fn value_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    match value {
        Some(Value::Object(map)) => Some(map),
        _ => None,
    }
}

fn normalized_user_origin(user: &Map<String, Value>) -> Value {
    let origin = value_object(user.get("origin"));
    let labels = if let Some(existing) = origin.and_then(|value| value.get("labels")) {
        value_string_array(Some(existing))
    } else {
        value_string_array(user.get("authLabels"))
    };
    let external = origin
        .and_then(|value| value_bool(value.get("external")))
        .or_else(|| value_bool(user.get("isExternal")))
        .unwrap_or(false);
    let provisioned = origin
        .and_then(|value| value_bool(value.get("provisioned")))
        .or_else(|| value_bool(user.get("isProvisioned")))
        .or_else(|| value_bool(user.get("provisioned")))
        .unwrap_or(false);
    let kind = if let Some(kind) = origin.and_then(|value| value_string(value.get("kind"))) {
        kind
    } else if provisioned {
        "provisioned".to_string()
    } else if external {
        "external".to_string()
    } else {
        "local".to_string()
    };

    Value::Object(Map::from_iter(vec![
        ("kind".to_string(), Value::String(kind)),
        ("external".to_string(), Value::Bool(external)),
        ("provisioned".to_string(), Value::Bool(provisioned)),
        ("labels".to_string(), Value::Array(labels)),
    ]))
}

fn normalized_user_last_active(user: &Map<String, Value>) -> Value {
    let last_active = value_object(user.get("lastActive"));
    let at = last_active
        .and_then(|value| value_string(value.get("at")))
        .or_else(|| value_string(user.get("lastSeenAt")))
        .unwrap_or_default();
    let age = last_active
        .and_then(|value| value_string(value.get("age")))
        .or_else(|| value_string(user.get("lastSeenAtAge")))
        .unwrap_or_default();

    Value::Object(Map::from_iter(vec![
        ("at".to_string(), Value::String(at)),
        ("age".to_string(), Value::String(age)),
    ]))
}

// Normalize user/team role payloads into a canonical display/case convention used by
// list output and diffing.
/// Purpose: implementation note.
pub(crate) fn normalize_org_role(value: Option<&Value>) -> String {
    let text = match value {
        Some(Value::String(text)) => text.trim(),
        _ => "",
    };
    match text.to_ascii_lowercase().as_str() {
        "" => String::new(),
        "nobasicrole" | "none" => "None".to_string(),
        lowered => {
            let mut chars = lowered.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        }
    }
}

/// service account role to api.
pub(crate) fn service_account_role_to_api(role: &str) -> String {
    match role.trim().to_ascii_lowercase().as_str() {
        "none" => "NoBasicRole".to_string(),
        "viewer" => "Viewer".to_string(),
        "editor" => "Editor".to_string(),
        "admin" => "Admin".to_string(),
        other => other.to_string(),
    }
}

/// user scope text.
pub(crate) fn user_scope_text(scope: &Scope) -> &'static str {
    match scope {
        Scope::Org => "org",
        Scope::Global => "global",
    }
}

/// User account identity scope text.
pub(crate) fn user_account_scope_text() -> &'static str {
    "global-shared"
}

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

/// Format a consistent diff summary line for access workflows.
pub(crate) fn access_diff_summary_line(
    kind: &str,
    checked: usize,
    differences: usize,
    local_source: &str,
    live_source: &str,
) -> String {
    if differences > 0 {
        format!(
            "Diff checked {} {}(s) from {} against {}; {} difference(s) found.",
            checked, kind, local_source, live_source, differences
        )
    } else {
        format!(
            "No {} differences across {} {}(s) from {} against {}.",
            kind, checked, kind, local_source, live_source
        )
    }
}

/// Format a consistent import summary line for access workflows.
pub(crate) fn access_import_summary_line(
    kind: &str,
    processed: usize,
    created: usize,
    updated: usize,
    skipped: usize,
    source: &str,
) -> String {
    format!(
        "Import summary for {kind}: processed={} created={} updated={} skipped={} source={}",
        processed, created, updated, skipped, source
    )
}

// Build a normalized user row shape expected by access list renderers.
/// Purpose: implementation note.
pub(crate) fn normalize_user_row(user: &Map<String, Value>, scope: &Scope) -> Map<String, Value> {
    let teams = match user.get("teams") {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|team| !team.is_empty())
            .map(|team| Value::String(team.to_string()))
            .collect::<Vec<Value>>(),
        _ => Vec::new(),
    };
    Map::from_iter(vec![
        (
            "id".to_string(),
            Value::String({
                let user_id = scalar_text(user.get("userId"));
                if user_id.is_empty() {
                    scalar_text(user.get("id"))
                } else {
                    user_id
                }
            }),
        ),
        (
            "login".to_string(),
            Value::String(string_field(user, "login", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(user, "email", "")),
        ),
        (
            "name".to_string(),
            Value::String(string_field(user, "name", "")),
        ),
        (
            "orgRole".to_string(),
            Value::String(normalize_org_role(
                user.get("role").or_else(|| user.get("orgRole")),
            )),
        ),
        (
            "grafanaAdmin".to_string(),
            Value::String(bool_label(
                value_bool(user.get("grafanaAdmin"))
                    .or_else(|| value_bool(user.get("isGrafanaAdmin")))
                    .or_else(|| value_bool(user.get("isAdmin"))),
            )),
        ),
        (
            "scope".to_string(),
            Value::String(user_scope_text(scope).to_string()),
        ),
        ("origin".to_string(), normalized_user_origin(user)),
        ("lastActive".to_string(), normalized_user_last_active(user)),
        ("teams".to_string(), Value::Array(teams)),
    ])
}

// Build a normalized team row shape expected by team list renderers.
/// Purpose: implementation note.
pub(crate) fn normalize_team_row(team: &Map<String, Value>) -> Map<String, Value> {
    let mut members = Vec::new();
    let mut seen = BTreeSet::new();
    for key in ["members", "admins"] {
        if let Some(Value::Array(values)) = team.get(key) {
            for item in values {
                if let Some(identity) = item.as_str() {
                    let identity = identity.trim();
                    if identity.is_empty() {
                        continue;
                    }
                    if seen.insert(identity.to_ascii_lowercase()) {
                        members.push(Value::String(identity.to_string()));
                    }
                }
            }
        }
    }
    Map::from_iter(vec![
        ("id".to_string(), Value::String(scalar_text(team.get("id")))),
        (
            "name".to_string(),
            Value::String(string_field(team, "name", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(team, "email", "")),
        ),
        (
            "memberCount".to_string(),
            Value::String({
                let value = scalar_text(team.get("memberCount"));
                if value.is_empty() {
                    members.len().to_string()
                } else {
                    value
                }
            }),
        ),
        ("members".to_string(), Value::Array(members)),
    ])
}

// Build a normalized service-account row shape expected by service-account list renderers.
/// Purpose: implementation note.
pub(crate) fn normalize_service_account_row(team: &Map<String, Value>) -> Map<String, Value> {
    Map::from_iter(vec![
        ("id".to_string(), Value::String(scalar_text(team.get("id")))),
        (
            "name".to_string(),
            Value::String(string_field(team, "name", "")),
        ),
        (
            "login".to_string(),
            Value::String(string_field(team, "login", "")),
        ),
        (
            "role".to_string(),
            Value::String(normalize_org_role(team.get("role"))),
        ),
        (
            "disabled".to_string(),
            Value::String(bool_label(
                value_bool(team.get("disabled")).or_else(|| value_bool(team.get("isDisabled"))),
            )),
        ),
        (
            "tokens".to_string(),
            Value::String({
                let value = scalar_text(team.get("tokens"));
                if value.is_empty() {
                    "0".to_string()
                } else {
                    value
                }
            }),
        ),
        (
            "orgId".to_string(),
            Value::String(scalar_text(team.get("orgId"))),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::{access_diff_summary_line, access_import_summary_line};

    #[test]
    fn access_diff_summary_line_includes_source_context_for_differences() {
        let line = access_diff_summary_line(
            "user",
            2,
            1,
            "./access-users",
            "Grafana live users",
        );
        assert_eq!(
            line,
            "Diff checked 2 user(s) from ./access-users against Grafana live users; 1 difference(s) found."
        );
    }

    #[test]
    fn access_diff_summary_line_includes_source_context_for_matches() {
        let line = access_diff_summary_line(
            "team",
            3,
            0,
            "./access-teams",
            "Grafana live teams",
        );
        assert_eq!(
            line,
            "No team differences across 3 team(s) from ./access-teams against Grafana live teams."
        );
    }

    #[test]
    fn access_import_summary_line_includes_resource_and_source_context() {
        let line = access_import_summary_line("user", 3, 1, 1, 1, "/tmp/access-users");
        assert_eq!(
            line,
            "Import summary for user: processed=3 created=1 updated=1 skipped=1 source=/tmp/access-users"
        );
    }
}

/// map get text.
pub(crate) fn map_get_text(map: &Map<String, Value>, key: &str) -> String {
    match map.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<&str>>()
            .join(","),
        _ => String::new(),
    }
}

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
pub(crate) fn render_objects_json(rows: &[Map<String, Value>]) -> super::Result<String> {
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
pub(crate) fn user_matches(row: &Map<String, Value>, args: &super::UserListArgs) -> bool {
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
