//! Shared render/format helpers for access CLI output.
//! Centralizes table/csv/json field normalization and human-facing value formatting.
use serde_json::{Map, Value};
use std::collections::BTreeSet;

use crate::common::{render_json_value, string_field};
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

/// Purpose: implementation note.
pub(crate) fn render_objects_json(rows: &[Map<String, Value>]) -> super::Result<String> {
    render_json_value(&Value::Array(
        rows.iter().cloned().map(Value::Object).collect(),
    ))
}

/// user table rows.
pub(crate) fn user_table_rows(rows: &[Map<String, Value>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                map_get_text(row, "id"),
                map_get_text(row, "login"),
                map_get_text(row, "email"),
                map_get_text(row, "name"),
                map_get_text(row, "orgRole"),
                map_get_text(row, "grafanaAdmin"),
                map_get_text(row, "scope"),
                map_get_text(row, "accountScope"),
                map_get_text(row, "teams"),
            ]
        })
        .collect()
}

/// team table rows.
pub(crate) fn team_table_rows(rows: &[Map<String, Value>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                map_get_text(row, "id"),
                map_get_text(row, "name"),
                map_get_text(row, "email"),
                map_get_text(row, "memberCount"),
                map_get_text(row, "members"),
            ]
        })
        .collect()
}

/// service account table rows.
pub(crate) fn service_account_table_rows(rows: &[Map<String, Value>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                map_get_text(row, "id"),
                map_get_text(row, "name"),
                map_get_text(row, "login"),
                map_get_text(row, "role"),
                map_get_text(row, "disabled"),
                map_get_text(row, "tokens"),
                map_get_text(row, "orgId"),
            ]
        })
        .collect()
}

/// user summary line.
pub(crate) fn user_summary_line(row: &Map<String, Value>) -> String {
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
    parts.join(" ")
}

/// team summary line.
pub(crate) fn team_summary_line(row: &Map<String, Value>) -> String {
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
    parts.join(" ")
}

/// service account summary line.
pub(crate) fn service_account_summary_line(row: &Map<String, Value>) -> String {
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
    parts.join(" ")
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
