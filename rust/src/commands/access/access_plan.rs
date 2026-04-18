//! Access plan review helpers.
//!
//! Minimal vertical slice:
//! - parser already lives in `cli_defs`
//! - pure model lives here
//! - renderers return strings so later TUI code can reuse the same document
//! - current implementation supports `--resource user` only

use reqwest::Method;
use serde::Serialize;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::path::Path;

use crate::access::cli_defs::{AccessPlanArgs, AccessPlanResource, PlanOutputFormat};
use crate::access::render::{
    format_table, map_get_text, normalize_user_row, user_scope_text, value_bool,
};
use crate::access::user::{
    build_record_diff_fields, build_user_diff_map, build_user_export_records_for_diff,
    list_user_teams_with_request, load_access_import_records, validate_user_scope_auth,
};
use crate::access::{
    build_auth_context, Scope, ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_USER_EXPORT_FILENAME,
};
use crate::common::{
    load_json_object_file, message, render_json_value, string_field, tool_version, Result,
};

const ACCESS_PLAN_KIND: &str = "grafana-util-access-plan";
const ACCESS_PLAN_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanChange {
    pub field: String,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanAction {
    pub action_id: String,
    pub domain: String,
    pub resource_kind: String,
    pub identity: String,
    pub scope: Option<String>,
    pub action: String,
    pub status: String,
    pub changed_fields: Vec<String>,
    pub changes: Vec<AccessPlanChange>,
    pub target: Option<Map<String, Value>>,
    pub blocked_reason: Option<String>,
    pub review_hints: Vec<String>,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanResourceReport {
    pub resource_kind: String,
    pub source_path: String,
    pub bundle_present: bool,
    pub source_count: usize,
    pub live_count: usize,
    pub checked: usize,
    pub same: usize,
    pub create: usize,
    pub update: usize,
    pub extra_remote: usize,
    pub delete: usize,
    pub blocked: usize,
    pub warning: usize,
    pub scope: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanSummary {
    pub resource_count: usize,
    pub checked: usize,
    pub same: usize,
    pub create: usize,
    pub update: usize,
    pub extra_remote: usize,
    pub delete: usize,
    pub blocked: usize,
    pub warning: usize,
    pub prune: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanDocument {
    pub kind: String,
    pub schema_version: i64,
    pub tool_version: String,
    pub summary: AccessPlanSummary,
    pub resources: Vec<AccessPlanResourceReport>,
    pub actions: Vec<AccessPlanAction>,
}

#[derive(Debug, Clone)]
struct BundleInput {
    records: Vec<Map<String, Value>>,
    metadata: Option<Map<String, Value>>,
}

fn plan_supported_columns() -> &'static [&'static str] {
    &[
        "action_id",
        "resource_kind",
        "identity",
        "action",
        "status",
        "changed_fields",
        "changes",
        "target",
        "blocked_reason",
        "review_hints",
        "source_path",
    ]
}

fn default_plan_columns() -> Vec<&'static str> {
    vec![
        "action_id",
        "identity",
        "action",
        "status",
        "blocked_reason",
    ]
}

fn plan_columns_label(columns: &[String]) -> Vec<String> {
    if columns.len() == 1 && columns[0] == "all" {
        return plan_supported_columns()
            .iter()
            .map(|column| (*column).to_string())
            .collect();
    }
    if columns.is_empty() {
        return default_plan_columns()
            .iter()
            .map(|column| (*column).to_string())
            .collect();
    }
    columns.to_vec()
}

fn normalize_plan_columns(columns: &[String]) -> Vec<String> {
    let mut resolved = Vec::new();
    for column in plan_columns_label(columns) {
        if !plan_supported_columns().contains(&column.as_str()) {
            continue;
        }
        if !resolved.contains(&column) {
            resolved.push(column);
        }
    }
    resolved
}

fn plan_header_text(document: &AccessPlanDocument) -> String {
    format!(
        "access plan: resources={} checked={} same={} create={} update={} extra_remote={} delete={} blocked={} warning={} prune={}",
        document.summary.resource_count,
        document.summary.checked,
        document.summary.same,
        document.summary.create,
        document.summary.update,
        document.summary.extra_remote,
        document.summary.delete,
        document.summary.blocked,
        document.summary.warning,
        document.summary.prune,
    )
}

fn build_action_id(identity: &str) -> String {
    format!("access:user:{identity}")
}

fn build_target_evidence(user: &Map<String, Value>) -> Map<String, Value> {
    let mut target = Map::new();
    for key in [
        "id",
        "login",
        "email",
        "name",
        "orgRole",
        "grafanaAdmin",
        "isExternal",
        "isProvisioned",
        "isExternallySynced",
        "isGrafanaAdminExternallySynced",
        "scope",
        "origin",
        "lastActive",
        "teams",
    ] {
        if let Some(value) = user.get(key) {
            target.insert(key.to_string(), value.clone());
        }
    }
    target
}

fn user_plan_blockers(live: &Map<String, Value>, changed_fields: &[String]) -> Vec<String> {
    let is_external = value_bool(live.get("isExternal")).unwrap_or(false);
    let is_provisioned = value_bool(live.get("isProvisioned")).unwrap_or(false);
    let is_externally_synced = value_bool(live.get("isExternallySynced")).unwrap_or(false);
    let is_admin_externally_synced =
        value_bool(live.get("isGrafanaAdminExternallySynced")).unwrap_or(false);
    let mut blockers = Vec::new();

    let profile_changed = changed_fields
        .iter()
        .any(|field| matches!(field.as_str(), "login" | "email" | "name"));
    if profile_changed && (is_external || is_provisioned) {
        blockers.push(
            "external or provisioned user profile cannot be updated through Grafana user API"
                .to_string(),
        );
    }
    if changed_fields.iter().any(|field| field == "orgRole") && is_externally_synced {
        blockers.push(
            "externally synced user orgRole cannot be updated through Grafana org user API"
                .to_string(),
        );
    }
    if changed_fields.iter().any(|field| field == "grafanaAdmin") && is_admin_externally_synced {
        blockers.push(
            "externally synced grafanaAdmin cannot be updated through Grafana permissions API"
                .to_string(),
        );
    }
    blockers
}

fn normalize_plan_metadata(input_dir: &Path) -> Option<Map<String, Value>> {
    let metadata_path = input_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return None;
    }
    load_json_object_file(&metadata_path, "Access plan metadata")
        .ok()
        .and_then(|value| value.as_object().cloned())
}

fn load_user_bundle(input_dir: &Path) -> Result<BundleInput> {
    let records = load_access_import_records(input_dir, ACCESS_EXPORT_KIND_USERS)?;
    Ok(BundleInput {
        records,
        metadata: normalize_plan_metadata(input_dir),
    })
}

fn plan_user_scope(records: &[Map<String, Value>], metadata: Option<&Map<String, Value>>) -> Scope {
    if let Some(scope) = metadata
        .and_then(|value| value.get("scope"))
        .and_then(Value::as_str)
    {
        if scope.eq_ignore_ascii_case("global") {
            return Scope::Global;
        }
        if scope.eq_ignore_ascii_case("org") {
            return Scope::Org;
        }
    }
    if records.iter().any(|record| {
        matches!(
            record.get("scope").and_then(Value::as_str),
            Some(scope) if scope.eq_ignore_ascii_case("global")
        )
    }) {
        Scope::Global
    } else {
        Scope::Org
    }
}

fn sort_actions(actions: &mut [AccessPlanAction]) {
    actions.sort_by(|left, right| {
        left.resource_kind
            .cmp(&right.resource_kind)
            .then_with(|| left.identity.cmp(&right.identity))
            .then_with(|| left.action.cmp(&right.action))
    });
}

fn build_change_rows(
    local: &Map<String, Value>,
    live: &Map<String, Value>,
) -> (Vec<String>, Vec<AccessPlanChange>) {
    let changed_fields = build_record_diff_fields(local, live);
    let mut changes = Vec::new();
    for field in &changed_fields {
        changes.push(AccessPlanChange {
            field: field.to_string(),
            before: local.get(field).cloned().unwrap_or(Value::Null),
            after: live.get(field).cloned().unwrap_or(Value::Null),
        });
    }
    (changed_fields, changes)
}

struct UserActionInput {
    identity: String,
    scope: Option<String>,
    source_path: String,
    action: &'static str,
    status: &'static str,
    changed_fields: Vec<String>,
    changes: Vec<AccessPlanChange>,
    target: Option<Map<String, Value>>,
    blocked_reason: Option<String>,
    review_hints: Vec<String>,
}

fn build_user_action(input: UserActionInput) -> AccessPlanAction {
    AccessPlanAction {
        action_id: build_action_id(&input.identity),
        domain: "access".to_string(),
        resource_kind: "user".to_string(),
        identity: input.identity,
        scope: input.scope,
        action: input.action.to_string(),
        status: input.status.to_string(),
        changed_fields: input.changed_fields,
        changes: input.changes,
        target: input.target,
        blocked_reason: input.blocked_reason,
        review_hints: input.review_hints,
        source_path: input.source_path,
    }
}

struct UserReportInput {
    source_path: String,
    source_count: usize,
    live_count: usize,
    checked: usize,
    same: usize,
    create: usize,
    update: usize,
    extra_remote: usize,
    delete: usize,
    blocked: usize,
    warning: usize,
    scope: Option<String>,
}

fn build_user_report(input: UserReportInput) -> AccessPlanResourceReport {
    AccessPlanResourceReport {
        resource_kind: "user".to_string(),
        source_path: input.source_path,
        bundle_present: true,
        source_count: input.source_count,
        live_count: input.live_count,
        checked: input.checked,
        same: input.same,
        create: input.create,
        update: input.update,
        extra_remote: input.extra_remote,
        delete: input.delete,
        blocked: input.blocked,
        warning: input.warning,
        scope: input.scope,
        notes: vec!["vertical slice: user resource only".to_string()],
    }
}

fn render_action_row(action: &AccessPlanAction, columns: &[String]) -> Vec<String> {
    columns
        .iter()
        .map(|column| match column.as_str() {
            "action_id" => action.action_id.clone(),
            "resource_kind" => action.resource_kind.clone(),
            "identity" => action.identity.clone(),
            "action" => action.action.clone(),
            "status" => action.status.clone(),
            "changed_fields" => serde_json::to_string(&action.changed_fields).unwrap_or_default(),
            "changes" => serde_json::to_string(&action.changes).unwrap_or_default(),
            "target" => serde_json::to_string(&action.target).unwrap_or_default(),
            "blocked_reason" => action.blocked_reason.clone().unwrap_or_default(),
            "review_hints" => serde_json::to_string(&action.review_hints).unwrap_or_default(),
            "source_path" => action.source_path.clone(),
            _ => String::new(),
        })
        .collect()
}

fn build_access_plan_actions<F>(
    mut request_json: F,
    args: &AccessPlanArgs,
    input_dir: &Path,
    bundle: &BundleInput,
) -> Result<(AccessPlanResourceReport, Vec<AccessPlanAction>)>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if !matches!(args.resource, AccessPlanResource::User) {
        return Err(message(
            "access plan currently supports --resource user only in this slice.",
        ));
    }

    let scope = plan_user_scope(&bundle.records, bundle.metadata.as_ref());
    let include_teams = bundle.records.iter().any(
        |record| matches!(record.get("teams"), Some(Value::Array(values)) if !values.is_empty()),
    );
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_scope_auth(&scope, include_teams, &auth_mode)?;

    let live_records =
        build_user_export_records_for_diff(&mut request_json, &scope, include_teams)?;
    let mut local_rows = bundle.records.clone();
    let mut live_rows = live_records
        .into_iter()
        .map(|row| normalize_user_row(&row, &scope))
        .collect::<Vec<Map<String, Value>>>();

    if include_teams {
        for row in &mut local_rows {
            let mut teams = row
                .get("teams")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|item| item.as_str().map(str::trim).map(str::to_string))
                .filter(|item| !item.is_empty())
                .collect::<Vec<String>>();
            teams.sort();
            teams.dedup();
            row.insert(
                "teams".to_string(),
                Value::Array(teams.into_iter().map(Value::String).collect()),
            );
        }

        for row in &mut live_rows {
            let user_id = map_get_text(row, "id");
            let mut teams = list_user_teams_with_request(&mut request_json, &user_id)?
                .into_iter()
                .map(|team| string_field(&team, "name", ""))
                .filter(|name: &String| !name.is_empty())
                .collect::<Vec<String>>();
            teams.sort();
            teams.dedup();
            row.insert(
                "teams".to_string(),
                Value::Array(teams.into_iter().map(Value::String).collect()),
            );
        }
    }

    let local_map = build_user_diff_map(&local_rows, &input_dir.to_string_lossy(), include_teams)?;
    let live_map = build_user_diff_map(&live_rows, "Grafana live users", include_teams)?;

    let mut actions = Vec::new();
    let mut checked = 0usize;
    let mut same = 0usize;
    let mut create = 0usize;
    let mut update = 0usize;
    let mut extra_remote = 0usize;
    let mut delete = 0usize;
    let mut blocked = 0usize;
    let mut warning = 0usize;

    let source_path = input_dir.join(ACCESS_USER_EXPORT_FILENAME);
    let source_path = source_path.to_string_lossy().to_string();

    for key in local_map.keys() {
        checked += 1;
        let (identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                create += 1;
                actions.push(build_user_action(UserActionInput {
                    identity: identity.clone(),
                    scope: Some(user_scope_text(&scope).to_string()),
                    source_path: source_path.clone(),
                    action: "would-create",
                    status: "ready",
                    changed_fields: Vec::new(),
                    changes: Vec::new(),
                    target: Some(build_target_evidence(local_payload)),
                    blocked_reason: None,
                    review_hints: Vec::new(),
                }));
            }
            Some((_, live_payload)) => {
                let (changed_fields, changes) = build_change_rows(local_payload, live_payload);
                if changed_fields.is_empty() {
                    same += 1;
                    actions.push(build_user_action(UserActionInput {
                        identity: identity.clone(),
                        scope: Some(user_scope_text(&scope).to_string()),
                        source_path: source_path.clone(),
                        action: "same",
                        status: "same",
                        changed_fields: Vec::new(),
                        changes: Vec::new(),
                        target: Some(build_target_evidence(live_payload)),
                        blocked_reason: None,
                        review_hints: Vec::new(),
                    }));
                } else {
                    let blockers = user_plan_blockers(live_payload, &changed_fields);
                    if blockers.is_empty() {
                        update += 1;
                        warning += 1;
                        actions.push(build_user_action(UserActionInput {
                            identity: identity.clone(),
                            scope: Some(user_scope_text(&scope).to_string()),
                            source_path: source_path.clone(),
                            action: "would-update",
                            status: "warning",
                            changed_fields,
                            changes,
                            target: Some(build_target_evidence(live_payload)),
                            blocked_reason: None,
                            review_hints: vec![
                                "review the live user target before applying".to_string()
                            ],
                        }));
                    } else {
                        blocked += 1;
                        actions.push(build_user_action(UserActionInput {
                            identity: identity.clone(),
                            scope: Some(user_scope_text(&scope).to_string()),
                            source_path: source_path.clone(),
                            action: "blocked",
                            status: "blocked",
                            changed_fields,
                            changes,
                            target: Some(build_target_evidence(live_payload)),
                            blocked_reason: Some(blockers.join("; ")),
                            review_hints: vec![
                                "review the target origin before attempting an update".to_string(),
                            ],
                        }));
                    }
                }
            }
        }
    }

    for key in live_map.keys() {
        if local_map.contains_key(key) {
            continue;
        }
        checked += 1;
        extra_remote += 1;
        let (identity, live_payload) = &live_map[key];
        let action = if args.prune {
            delete += 1;
            "would-delete"
        } else {
            warning += 1;
            "extra-remote"
        };
        actions.push(build_user_action(UserActionInput {
            identity: identity.clone(),
            scope: Some(user_scope_text(&scope).to_string()),
            source_path: source_path.clone(),
            action,
            status: if args.prune { "ready" } else { "warning" },
            changed_fields: Vec::new(),
            changes: Vec::new(),
            target: Some(build_target_evidence(live_payload)),
            blocked_reason: if args.prune {
                None
            } else {
                Some("use --prune to include delete candidates".to_string())
            },
            review_hints: vec!["remote-only user record".to_string()],
        }));
    }

    let report = build_user_report(UserReportInput {
        source_path,
        source_count: local_map.len(),
        live_count: live_map.len(),
        checked,
        same,
        create,
        update,
        extra_remote,
        delete,
        blocked,
        warning,
        scope: Some(user_scope_text(&scope).to_string()),
    });

    Ok((report, actions))
}

fn build_access_plan_document<F>(
    request_json: F,
    args: &AccessPlanArgs,
) -> Result<AccessPlanDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let bundle = load_user_bundle(&args.input_dir)?;
    let (resource, actions) =
        build_access_plan_actions(request_json, args, &args.input_dir, &bundle)?;
    let resources = vec![resource];
    let mut actions = actions;
    sort_actions(&mut actions);
    Ok(AccessPlanDocument {
        kind: ACCESS_PLAN_KIND.to_string(),
        schema_version: ACCESS_PLAN_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        summary: AccessPlanSummary {
            resource_count: resources.len(),
            checked: resources.iter().map(|item| item.checked).sum(),
            same: resources.iter().map(|item| item.same).sum(),
            create: resources.iter().map(|item| item.create).sum(),
            update: resources.iter().map(|item| item.update).sum(),
            extra_remote: resources.iter().map(|item| item.extra_remote).sum(),
            delete: resources.iter().map(|item| item.delete).sum(),
            blocked: resources.iter().map(|item| item.blocked).sum(),
            warning: resources.iter().map(|item| item.warning).sum(),
            prune: args.prune,
        },
        resources,
        actions,
    })
}

fn validate_plan_columns(args: &AccessPlanArgs) -> Result<()> {
    if !args.output_columns.is_empty() && matches!(args.output_format, PlanOutputFormat::Json) {
        return Err(message(
            "--output-columns is only supported with text or table output for access plan.",
        ));
    }
    Ok(())
}

fn render_plan_text(document: &AccessPlanDocument, args: &AccessPlanArgs) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "{}", plan_header_text(document));
    for resource in &document.resources {
        let _ = writeln!(
            output,
            "- {} source={} bundle={} checked={} same={} create={} update={} extra={} delete={} blocked={} warning={}",
            resource.resource_kind,
            resource.source_path,
            if resource.bundle_present { "present" } else { "missing" },
            resource.checked,
            resource.same,
            resource.create,
            resource.update,
            resource.extra_remote,
            resource.delete,
            resource.blocked,
            resource.warning
        );
    }

    for action in document
        .actions
        .iter()
        .filter(|action| args.show_same || action.action != "same")
    {
        let _ = write!(
            output,
            "{} {} {}",
            action.status.to_uppercase(),
            action.identity,
            action.action
        );
        if !action.changed_fields.is_empty() {
            let _ = write!(output, " fields={}", action.changed_fields.join(","));
        }
        if let Some(reason) = &action.blocked_reason {
            let _ = write!(output, " blocked={reason}");
        }
        if !action.review_hints.is_empty() {
            let _ = write!(output, " hints={}", action.review_hints.join(" | "));
        }
        let _ = writeln!(output);
    }

    output
}

fn render_plan_table(document: &AccessPlanDocument, args: &AccessPlanArgs) -> String {
    let columns = normalize_plan_columns(&args.output_columns);
    let headers = columns
        .iter()
        .map(|column| column.replace('_', " ").to_ascii_uppercase())
        .collect::<Vec<String>>();
    let header_refs = headers
        .iter()
        .map(|value| value.as_str())
        .collect::<Vec<&str>>();
    let rows = document
        .actions
        .iter()
        .filter(|action| args.show_same || action.action != "same")
        .map(|action| render_action_row(action, &columns))
        .collect::<Vec<Vec<String>>>();
    let mut rendered = String::new();
    let table = format_table(&header_refs, &rows);
    if args.no_header {
        for line in table.into_iter().skip(2) {
            let _ = writeln!(rendered, "{line}");
        }
    } else {
        for line in table {
            let _ = writeln!(rendered, "{line}");
        }
    }
    rendered
}

fn render_plan_json(document: &AccessPlanDocument) -> Result<String> {
    render_json_value(&serde_json::to_value(document)?)
}

pub(crate) fn print_access_plan_columns() {
    println!(
        "Supported --output-columns values: all, {}",
        plan_supported_columns().join(", ")
    );
}

pub(crate) fn access_plan_with_request<F>(
    mut request_json: F,
    args: &AccessPlanArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_plan_columns(args)?;
    if !matches!(args.resource, AccessPlanResource::User) {
        return Err(message(
            "access plan currently supports --resource user only in this slice.",
        ));
    }

    let document = build_access_plan_document(&mut request_json, args)?;
    match args.output_format {
        PlanOutputFormat::Text => {
            print!("{}", render_plan_text(&document, args));
        }
        PlanOutputFormat::Table => {
            print!("{}", render_plan_table(&document, args));
        }
        PlanOutputFormat::Json => {
            print!("{}", render_plan_json(&document)?);
        }
    }
    Ok(document.actions.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::cli_defs::PlanOutputFormat;
    use crate::access::{parse_cli_from, CommonCliArgs};
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    fn make_common() -> CommonCliArgs {
        CommonCliArgs {
            profile: None,
            url: "http://127.0.0.1:3000".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            org_id: None,
            timeout: 30,
            verify_ssl: false,
            insecure: false,
            ca_cert: None,
        }
    }

    fn write_user_bundle(dir: &Path) {
        fs::write(
            dir.join("users.json"),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-access-user-export-index",
                "version": 1,
                "records": [
                    {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor"},
                    {"login": "bob", "email": "bob@example.com", "name": "Bob", "orgRole": "Viewer"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn parse_access_plan_defaults_to_user_resource() {
        let args = parse_cli_from(["grafana-util", "plan", "--input-dir", "./access-users"]);
        match args.command {
            crate::access::AccessCommand::Plan(plan) => {
                assert!(matches!(plan.resource, AccessPlanResource::User));
                assert!(matches!(plan.output_format, PlanOutputFormat::Text));
            }
            _ => panic!("expected access plan"),
        }
    }

    #[test]
    fn user_plan_builds_summary_and_renderers() {
        let temp_dir = tempdir().unwrap();
        write_user_bundle(temp_dir.path());
        let args = AccessPlanArgs {
            common: make_common(),
            input_dir: temp_dir.path().to_path_buf(),
            resource: AccessPlanResource::User,
            prune: false,
            output_columns: vec![
                "identity".to_string(),
                "action".to_string(),
                "status".to_string(),
            ],
            list_columns: false,
            no_header: false,
            show_same: false,
            output_format: PlanOutputFormat::Text,
        };
        let document = build_access_plan_document(
            |method, path, _params, _payload| match (method, path) {
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": "1", "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Editor"}
                ]))),
                _ => panic!("unexpected path {path}"),
            },
            &args,
        )
        .unwrap();

        assert_eq!(document.kind, ACCESS_PLAN_KIND);
        assert_eq!(document.summary.checked, 2);
        assert_eq!(document.summary.same, 1);
        assert_eq!(document.summary.create, 1);
        assert_eq!(document.actions.len(), 2);
        assert!(document
            .actions
            .iter()
            .any(|action| action.identity == "bob"));

        let text = render_plan_text(&document, &args);
        assert!(text.contains("access plan:"));
        assert!(text.contains("would-create"));
        assert!(!text.contains("\nSAME "));

        let table = render_plan_table(&document, &args);
        assert!(table.contains("IDENTITY"));
        assert!(table.contains("bob"));

        let json = render_plan_json(&document).unwrap();
        assert!(json.contains("\"kind\": \"grafana-util-access-plan\""));
    }

    #[test]
    fn plan_rejects_non_user_resource_in_this_slice() {
        let temp_dir = tempdir().unwrap();
        write_user_bundle(temp_dir.path());
        let args = AccessPlanArgs {
            common: make_common(),
            input_dir: temp_dir.path().to_path_buf(),
            resource: AccessPlanResource::Team,
            prune: false,
            output_columns: Vec::new(),
            list_columns: false,
            no_header: false,
            show_same: false,
            output_format: PlanOutputFormat::Text,
        };
        let err =
            build_access_plan_document(|_method, _path, _params, _payload| unreachable!(), &args)
                .unwrap_err();
        assert!(err.to_string().contains("supports --resource user only"));
    }
}
