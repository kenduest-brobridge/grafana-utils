use serde_json::{Map, Value};

use crate::common::{build_shared_diff_document, SharedDiffSummary, TOOL_VERSION};

use super::normalization::{
    bool_label, normalize_org_role, scalar_text, user_scope_text, value_bool,
};
use crate::access::Scope;

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

fn value_object(value: Option<&Value>) -> Option<&Map<String, Value>> {
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

/// Format a consistent review contract line for access diff workflows.
pub(crate) fn access_diff_review_line(
    kind: &str,
    checked: usize,
    differences: usize,
    local_source: &str,
    live_source: &str,
) -> String {
    let same = checked.saturating_sub(differences);
    format!(
        "Review: required=true reviewed=false kind={kind} checked={checked} same={same} different={differences} source={local_source} live={live_source}"
    )
}

/// Build a shared access diff review document.
pub(crate) fn build_access_diff_review_document(
    resource_kind: &str,
    summary: SharedDiffSummary,
    local_source: &str,
    live_source: &str,
    rows: &[Value],
) -> Value {
    let mut document =
        build_shared_diff_document("grafana-utils-access-diff-review", 1, summary, rows);
    if let Some(object) = document.as_object_mut() {
        object.insert(
            "toolVersion".to_string(),
            Value::String(TOOL_VERSION.to_string()),
        );
        object.insert("reviewRequired".to_string(), Value::Bool(true));
        object.insert("reviewed".to_string(), Value::Bool(false));
        object.insert(
            "resourceKind".to_string(),
            Value::String(resource_kind.to_string()),
        );
        object.insert(
            "localSource".to_string(),
            Value::String(local_source.to_string()),
        );
        object.insert(
            "liveSource".to_string(),
            Value::String(live_source.to_string()),
        );
    }
    document
}

/// Build a shared access delete review document.
pub(crate) fn build_access_delete_review_document(
    resource_kind: &str,
    live_source: &str,
    rows: &[Value],
) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String("grafana-utils-access-delete-review".to_string()),
        ),
        ("schemaVersion".to_string(), Value::Number(1.into())),
        (
            "toolVersion".to_string(),
            Value::String(TOOL_VERSION.to_string()),
        ),
        ("reviewRequired".to_string(), Value::Bool(true)),
        ("reviewed".to_string(), Value::Bool(false)),
        (
            "resourceKind".to_string(),
            Value::String(resource_kind.to_string()),
        ),
        (
            "liveSource".to_string(),
            Value::String(live_source.to_string()),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "deleted".to_string(),
                    Value::Number((rows.len() as i64).into()),
                ),
                (
                    "liveSource".to_string(),
                    Value::String(live_source.to_string()),
                ),
            ])),
        ),
        ("rows".to_string(), Value::Array(rows.to_vec())),
    ]))
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

/// Format a consistent export summary line for access workflows.
pub(crate) fn access_export_summary_line(
    kind: &str,
    exported: usize,
    source: &str,
    payload_path: &str,
    metadata_path: &str,
    dry_run: bool,
) -> String {
    let action = if dry_run { "Would export" } else { "Exported" };
    format!("{action} {exported} {kind}(s) from {source} -> {payload_path} and {metadata_path}")
}

/// Format a consistent delete summary line for access workflows.
pub(crate) fn access_delete_summary_line(
    kind: &str,
    identity: &str,
    details: &[(&str, String)],
) -> String {
    let mut parts = vec![format!("Deleted {kind} {identity}")];
    for (label, value) in details {
        if !value.trim().is_empty() {
            parts.push(format!("{label}={value}"));
        }
    }
    parts.join(" ")
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
            Value::String(super::normalization::map_get_text(user, "login")),
        ),
        (
            "email".to_string(),
            Value::String(super::normalization::map_get_text(user, "email")),
        ),
        (
            "name".to_string(),
            Value::String(super::normalization::map_get_text(user, "name")),
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
    let mut seen = std::collections::BTreeSet::new();
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
            Value::String(super::normalization::map_get_text(team, "name")),
        ),
        (
            "email".to_string(),
            Value::String(super::normalization::map_get_text(team, "email")),
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
            Value::String(super::normalization::map_get_text(team, "name")),
        ),
        (
            "login".to_string(),
            Value::String(super::normalization::map_get_text(team, "login")),
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
    use super::{
        access_delete_summary_line, access_diff_review_line, access_diff_summary_line,
        access_export_summary_line, access_import_summary_line,
        build_access_delete_review_document, build_access_diff_review_document,
    };
    use crate::common::SharedDiffSummary;
    use crate::common::TOOL_VERSION;
    use serde_json::{json, Value};

    #[test]
    fn access_diff_summary_line_includes_source_context_for_differences() {
        let line = access_diff_summary_line("user", 2, 1, "./access-users", "Grafana live users");
        assert_eq!(
            line,
            "Diff checked 2 user(s) from ./access-users against Grafana live users; 1 difference(s) found."
        );
    }

    #[test]
    fn access_diff_summary_line_includes_source_context_for_matches() {
        let line = access_diff_summary_line("team", 3, 0, "./access-teams", "Grafana live teams");
        assert_eq!(
            line,
            "No team differences across 3 team(s) from ./access-teams against Grafana live teams."
        );
    }

    #[test]
    fn access_diff_review_line_surfaces_review_contract_for_diff() {
        let line = access_diff_review_line("team", 3, 1, "./access-teams", "Grafana live teams");
        assert_eq!(
            line,
            "Review: required=true reviewed=false kind=team checked=3 same=2 different=1 source=./access-teams live=Grafana live teams"
        );
    }

    #[test]
    fn access_diff_review_document_surfaces_shared_review_contract() {
        let rows = vec![json!({
            "status": "different",
            "identity": "alice",
            "changedFields": ["email"]
        })];
        let document = build_access_diff_review_document(
            "user",
            SharedDiffSummary {
                checked: 2,
                same: 1,
                different: 1,
                missing_remote: 0,
                extra_remote: 0,
                ambiguous: 0,
            },
            "./access-users",
            "Grafana live users",
            &rows,
        );

        assert_eq!(
            document.get("kind"),
            Some(&json!("grafana-utils-access-diff-review"))
        );
        assert_eq!(document.get("schemaVersion"), Some(&json!(1)));
        assert_eq!(document.get("toolVersion"), Some(&json!(TOOL_VERSION)));
        assert_eq!(document.get("reviewRequired"), Some(&json!(true)));
        assert_eq!(document.get("reviewed"), Some(&json!(false)));
        assert_eq!(document.get("resourceKind"), Some(&json!("user")));
        assert_eq!(document.get("localSource"), Some(&json!("./access-users")));
        assert_eq!(
            document.get("liveSource"),
            Some(&json!("Grafana live users"))
        );
        assert_eq!(
            document
                .get("summary")
                .and_then(|summary| summary.get("checked")),
            Some(&json!(2))
        );
        assert_eq!(
            document.get("rows").and_then(Value::as_array).map(Vec::len),
            Some(1)
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

    #[test]
    fn access_export_summary_line_is_consistent_across_resources() {
        let dry_run = access_export_summary_line(
            "team",
            2,
            "/tmp/access-teams",
            "/tmp/access-teams/teams.json",
            "/tmp/access-teams/metadata.json",
            true,
        );
        assert_eq!(
            dry_run,
            "Would export 2 team(s) from /tmp/access-teams -> /tmp/access-teams/teams.json and /tmp/access-teams/metadata.json"
        );

        let live = access_export_summary_line(
            "service-account",
            1,
            "/tmp/access-service-accounts",
            "/tmp/access-service-accounts/service-accounts.json",
            "/tmp/access-service-accounts/metadata.json",
            false,
        );
        assert_eq!(
            live,
            "Exported 1 service-account(s) from /tmp/access-service-accounts -> /tmp/access-service-accounts/service-accounts.json and /tmp/access-service-accounts/metadata.json"
        );
    }

    #[test]
    fn access_delete_summary_line_surfaces_identity_and_context() {
        let line = access_delete_summary_line(
            "service-account",
            "svc",
            &[
                ("serviceAccountId", "4".to_string()),
                ("login", "sa-svc".to_string()),
                ("role", "Viewer".to_string()),
                ("disabled", "false".to_string()),
                ("tokens", "2".to_string()),
                ("message", "Service account deleted.".to_string()),
            ],
        );

        assert_eq!(
            line,
            "Deleted service-account svc serviceAccountId=4 login=sa-svc role=Viewer disabled=false tokens=2 message=Service account deleted."
        );
    }

    #[test]
    fn access_delete_review_document_surfaces_shared_review_contract() {
        let rows = vec![json!({
            "id": "9",
            "login": "alice",
            "scope": "global",
            "message": "Deleted."
        })];
        let document = build_access_delete_review_document("user", "Grafana live users", &rows);

        assert_eq!(
            document.get("kind"),
            Some(&json!("grafana-utils-access-delete-review"))
        );
        assert_eq!(document.get("schemaVersion"), Some(&json!(1)));
        assert_eq!(document.get("toolVersion"), Some(&json!(TOOL_VERSION)));
        assert_eq!(document.get("reviewRequired"), Some(&json!(true)));
        assert_eq!(document.get("reviewed"), Some(&json!(false)));
        assert_eq!(document.get("resourceKind"), Some(&json!("user")));
        assert_eq!(
            document.get("liveSource"),
            Some(&json!("Grafana live users"))
        );
        assert_eq!(
            document
                .get("summary")
                .and_then(|summary| summary.get("deleted")),
            Some(&json!(1))
        );
        assert_eq!(
            document.get("rows").and_then(Value::as_array).map(Vec::len),
            Some(1)
        );
    }
}
