//! Datasource reconcile plan model, builder, and renderers.
//!
//! The builder returns a pure plan document so CLI renderers and future TUI
//! views can consume the same stable action model.

use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::common::{
    message, render_json_value, requested_columns_include_all, string_field, tool_version, Result,
};

use super::{DatasourceImportRecord, DatasourcePlanOutputFormat};

const PLAN_KIND: &str = "grafana-util-datasource-plan";
const PLAN_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone)]
pub(crate) struct DatasourcePlanOrgInput {
    pub(crate) source_org_id: String,
    pub(crate) source_org_name: String,
    pub(crate) target_org_id: Option<String>,
    pub(crate) target_org_name: String,
    pub(crate) org_action: String,
    pub(crate) input_dir: PathBuf,
    pub(crate) records: Vec<DatasourceImportRecord>,
    pub(crate) live: Vec<Map<String, Value>>,
}

#[derive(Debug, Clone)]
pub(crate) struct DatasourcePlanInput {
    pub(crate) scope: String,
    pub(crate) input_format: String,
    pub(crate) prune: bool,
    pub(crate) orgs: Vec<DatasourcePlanOrgInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DatasourcePlanChange {
    pub(crate) field: String,
    pub(crate) before: Value,
    pub(crate) after: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DatasourcePlanAction {
    pub(crate) action_id: String,
    pub(crate) domain: String,
    pub(crate) resource_kind: String,
    pub(crate) uid: String,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) datasource_type: String,
    pub(crate) source_org_id: Option<String>,
    pub(crate) target_org_id: Option<String>,
    pub(crate) match_basis: String,
    pub(crate) action: String,
    pub(crate) status: String,
    pub(crate) changed_fields: Vec<String>,
    pub(crate) changes: Vec<DatasourcePlanChange>,
    pub(crate) source_file: Option<String>,
    pub(crate) target_uid: Option<String>,
    pub(crate) target_version: Option<i64>,
    pub(crate) target_read_only: Option<bool>,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) review_hints: Vec<String>,
    pub(crate) requires_secret_values: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DatasourcePlanOrgSummary {
    pub(crate) source_org_id: Option<String>,
    pub(crate) source_org_name: String,
    pub(crate) target_org_id: Option<String>,
    pub(crate) target_org_name: String,
    pub(crate) org_action: String,
    pub(crate) input_dir: String,
    pub(crate) checked: usize,
    pub(crate) same: usize,
    pub(crate) create: usize,
    pub(crate) update: usize,
    pub(crate) extra: usize,
    pub(crate) delete: usize,
    pub(crate) blocked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DatasourcePlanSummary {
    pub(crate) checked: usize,
    pub(crate) same: usize,
    pub(crate) create: usize,
    pub(crate) update: usize,
    pub(crate) extra: usize,
    pub(crate) delete: usize,
    pub(crate) blocked: usize,
    pub(crate) warning: usize,
    pub(crate) org_count: usize,
    pub(crate) would_create_org_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DatasourcePlanReport {
    pub(crate) kind: String,
    pub(crate) schema_version: i64,
    pub(crate) tool_version: String,
    pub(crate) mode: String,
    pub(crate) scope: String,
    pub(crate) input_format: String,
    pub(crate) prune: bool,
    pub(crate) summary: DatasourcePlanSummary,
    pub(crate) orgs: Vec<DatasourcePlanOrgSummary>,
    pub(crate) actions: Vec<DatasourcePlanAction>,
}

struct RecordActionDraft<'a> {
    org_input: &'a DatasourcePlanOrgInput,
    record: &'a DatasourceImportRecord,
    index: usize,
    live: Option<&'a Map<String, Value>>,
    match_basis: &'a str,
    action: &'a str,
    status: &'a str,
    blocked_reason: Option<String>,
    review_hints: Vec<String>,
}

pub(crate) fn datasource_plan_column_ids() -> &'static [&'static str] {
    &[
        "action_id",
        "action",
        "status",
        "uid",
        "name",
        "type",
        "match_basis",
        "source_org_id",
        "target_org_id",
        "target_uid",
        "target_version",
        "target_read_only",
        "changed_fields",
        "blocked_reason",
        "source_file",
    ]
}

pub(crate) fn build_datasource_plan(input: DatasourcePlanInput) -> DatasourcePlanReport {
    let mut orgs = Vec::new();
    let mut actions = Vec::new();
    for org_input in input.orgs {
        let start = actions.len();
        actions.extend(build_org_actions(&org_input, input.prune));
        let org_actions = &actions[start..];
        orgs.push(build_org_summary(&org_input, org_actions));
    }
    let summary = build_summary(&orgs, &actions);
    DatasourcePlanReport {
        kind: PLAN_KIND.to_string(),
        schema_version: PLAN_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        mode: "review".to_string(),
        scope: input.scope,
        input_format: input.input_format,
        prune: input.prune,
        summary,
        orgs,
        actions,
    }
}

pub(crate) fn build_datasource_plan_json(report: &DatasourcePlanReport) -> Result<Value> {
    serde_json::to_value(report).map_err(|error| message(error.to_string()))
}

pub(crate) fn print_datasource_plan_report(
    report: &DatasourcePlanReport,
    output_format: DatasourcePlanOutputFormat,
    show_same: bool,
    no_header: bool,
    selected_columns: &[String],
) -> Result<()> {
    match output_format {
        DatasourcePlanOutputFormat::Json => {
            print!(
                "{}",
                render_json_value(&build_datasource_plan_json(report)?)?
            );
        }
        DatasourcePlanOutputFormat::Table => {
            for line in render_plan_table(report, show_same, no_header, selected_columns) {
                println!("{line}");
            }
            println!("{}", plan_summary_line(report));
        }
        DatasourcePlanOutputFormat::Text => {
            println!("{}", plan_summary_line(report));
            for line in render_plan_text_details(report, show_same) {
                println!("{line}");
            }
        }
    }
    Ok(())
}

fn build_org_actions(org_input: &DatasourcePlanOrgInput, prune: bool) -> Vec<DatasourcePlanAction> {
    if org_input.target_org_id.is_none() && org_input.org_action != "would-create" {
        return org_input
            .records
            .iter()
            .enumerate()
            .map(|(index, record)| {
                build_record_action(RecordActionDraft {
                    org_input,
                    record,
                    index,
                    live: None,
                    match_basis: "unknown",
                    action: "blocked-missing-org",
                    status: "blocked",
                    blocked_reason: Some("target-org-missing".to_string()),
                    review_hints: Vec::new(),
                })
            })
            .collect();
    }

    let mut live_by_uid = BTreeMap::new();
    let mut live_by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut live_matched = vec![false; org_input.live.len()];
    for (index, item) in org_input.live.iter().enumerate() {
        let uid = string_field(item, "uid", "");
        if !uid.is_empty() {
            live_by_uid.insert(uid, index);
        }
        let name = string_field(item, "name", "");
        if !name.is_empty() {
            live_by_name.entry(name).or_default().push(index);
        }
    }

    let mut actions = Vec::new();
    for (index, record) in org_input.records.iter().enumerate() {
        if !record.uid.is_empty() {
            if let Some(live_index) = live_by_uid.get(&record.uid) {
                live_matched[*live_index] = true;
                actions.push(build_pair_action(
                    org_input,
                    record,
                    index,
                    &org_input.live[*live_index],
                    "uid",
                ));
                continue;
            }
        }
        let name_matches = live_by_name.get(&record.name).cloned().unwrap_or_default();
        if name_matches.is_empty() {
            actions.push(build_record_action(RecordActionDraft {
                org_input,
                record,
                index,
                live: None,
                match_basis: if record.uid.is_empty() { "name" } else { "uid" },
                action: "would-create",
                status: "ready",
                blocked_reason: None,
                review_hints: vec!["missing-remote".to_string()],
            }));
            continue;
        }
        if name_matches.len() > 1 {
            actions.push(build_record_action(RecordActionDraft {
                org_input,
                record,
                index,
                live: None,
                match_basis: "name",
                action: "blocked-ambiguous",
                status: "blocked",
                blocked_reason: Some("ambiguous-live-name-match".to_string()),
                review_hints: Vec::new(),
            }));
            continue;
        }
        let live_index = name_matches[0];
        let live_uid = string_field(&org_input.live[live_index], "uid", "");
        if !record.uid.is_empty() && !live_uid.is_empty() && record.uid != live_uid {
            actions.push(build_record_action(RecordActionDraft {
                org_input,
                record,
                index,
                live: Some(&org_input.live[live_index]),
                match_basis: "name",
                action: "blocked-uid-mismatch",
                status: "blocked",
                blocked_reason: Some("uid-name-mismatch".to_string()),
                review_hints: Vec::new(),
            }));
            continue;
        }
        live_matched[live_index] = true;
        actions.push(build_pair_action(
            org_input,
            record,
            index,
            &org_input.live[live_index],
            "name",
        ));
    }

    for (index, live) in org_input.live.iter().enumerate() {
        if live_matched[index] {
            continue;
        }
        actions.push(build_remote_only_action(org_input, live, prune));
    }
    actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    actions
}

fn build_pair_action(
    org_input: &DatasourcePlanOrgInput,
    record: &DatasourceImportRecord,
    index: usize,
    live: &Map<String, Value>,
    match_basis: &str,
) -> DatasourcePlanAction {
    let changes = compare_records(record, live);
    let read_only = live
        .get("readOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (action, status, blocked_reason) = if read_only && !changes.is_empty() {
        (
            "blocked-read-only",
            "blocked",
            Some("target-read-only".to_string()),
        )
    } else if changes.is_empty() {
        ("same", "same", None)
    } else {
        ("would-update", "ready", None)
    };
    let mut hints = Vec::new();
    if record.secure_json_data_placeholders.is_some() {
        hints.push("requires-secret-values".to_string());
    }
    build_record_action(RecordActionDraft {
        org_input,
        record,
        index,
        live: Some(live),
        match_basis,
        action,
        status,
        blocked_reason,
        review_hints: hints,
    })
    .with_changes(changes)
}

fn build_record_action(draft: RecordActionDraft<'_>) -> DatasourcePlanAction {
    let org_input = draft.org_input;
    let record = draft.record;
    let live = draft.live;
    let target_uid = live
        .map(|item| string_field(item, "uid", ""))
        .filter(|item| !item.is_empty());
    let target_version = live.and_then(|item| item.get("version").and_then(Value::as_i64));
    let target_read_only = live.and_then(|item| item.get("readOnly").and_then(Value::as_bool));
    let uid = if record.uid.is_empty() {
        target_uid.clone().unwrap_or_default()
    } else {
        record.uid.clone()
    };
    let name = if record.name.is_empty() {
        live.map(|item| string_field(item, "name", ""))
            .unwrap_or_default()
    } else {
        record.name.clone()
    };
    let datasource_type = if record.datasource_type.is_empty() {
        live.map(|item| string_field(item, "type", ""))
            .unwrap_or_default()
    } else {
        record.datasource_type.clone()
    };
    let identity_kind = if !uid.is_empty() { "uid" } else { "name" };
    let identity_value = if !uid.is_empty() { &uid } else { &name };
    DatasourcePlanAction {
        action_id: format!(
            "org:{}/datasource:{}:{}",
            org_input
                .target_org_id
                .as_deref()
                .or_else(|| (!org_input.source_org_id.is_empty())
                    .then_some(org_input.source_org_id.as_str()))
                .unwrap_or("current"),
            identity_kind,
            identity_value
        ),
        domain: "datasource".to_string(),
        resource_kind: "datasource".to_string(),
        uid,
        name,
        datasource_type,
        source_org_id: empty_to_none(&org_input.source_org_id),
        target_org_id: org_input.target_org_id.clone(),
        match_basis: draft.match_basis.to_string(),
        action: draft.action.to_string(),
        status: draft.status.to_string(),
        changed_fields: Vec::new(),
        changes: Vec::new(),
        source_file: Some(format!("{}#{}", org_input.input_dir.display(), draft.index)),
        target_uid,
        target_version,
        target_read_only,
        blocked_reason: draft.blocked_reason,
        review_hints: draft.review_hints,
        requires_secret_values: record.secure_json_data_placeholders.is_some(),
    }
}

fn build_remote_only_action(
    org_input: &DatasourcePlanOrgInput,
    live: &Map<String, Value>,
    prune: bool,
) -> DatasourcePlanAction {
    let read_only = live
        .get("readOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let uid = string_field(live, "uid", "");
    let name = string_field(live, "name", "");
    let datasource_type = string_field(live, "type", "");
    let action = if prune {
        if read_only {
            "blocked-read-only"
        } else {
            "would-delete"
        }
    } else {
        "extra-remote"
    };
    let status = if prune && read_only {
        "blocked"
    } else if prune {
        "ready"
    } else {
        "warning"
    };
    let blocked_reason = (prune && read_only).then_some("target-read-only".to_string());
    let identity_kind = if !uid.is_empty() { "uid" } else { "name" };
    let identity_value = if !uid.is_empty() { &uid } else { &name };
    DatasourcePlanAction {
        action_id: format!(
            "org:{}/datasource:{}:{}",
            org_input.target_org_id.as_deref().unwrap_or("current"),
            identity_kind,
            identity_value
        ),
        domain: "datasource".to_string(),
        resource_kind: "datasource".to_string(),
        uid: uid.clone(),
        name,
        datasource_type,
        source_org_id: empty_to_none(&org_input.source_org_id),
        target_org_id: org_input.target_org_id.clone(),
        match_basis: if uid.is_empty() { "name" } else { "uid" }.to_string(),
        action: action.to_string(),
        status: status.to_string(),
        changed_fields: Vec::new(),
        changes: Vec::new(),
        source_file: None,
        target_uid: empty_to_none(&uid),
        target_version: live.get("version").and_then(Value::as_i64),
        target_read_only: Some(read_only),
        blocked_reason,
        review_hints: vec!["remote-only".to_string()],
        requires_secret_values: false,
    }
}

trait WithChanges {
    fn with_changes(self, changes: Vec<DatasourcePlanChange>) -> Self;
}

impl WithChanges for DatasourcePlanAction {
    fn with_changes(mut self, changes: Vec<DatasourcePlanChange>) -> Self {
        self.changed_fields = changes.iter().map(|item| item.field.clone()).collect();
        self.changes = changes;
        self
    }
}

fn compare_records(
    expected: &DatasourceImportRecord,
    actual: &Map<String, Value>,
) -> Vec<DatasourcePlanChange> {
    let mut changes = Vec::new();
    push_change(
        &mut changes,
        "uid",
        string_value(&expected.uid),
        live_string(actual, "uid"),
    );
    push_change(
        &mut changes,
        "name",
        string_value(&expected.name),
        live_string(actual, "name"),
    );
    push_change(
        &mut changes,
        "type",
        string_value(&expected.datasource_type),
        live_string(actual, "type"),
    );
    push_change(
        &mut changes,
        "access",
        string_value(&expected.access),
        live_string(actual, "access"),
    );
    push_change(
        &mut changes,
        "url",
        string_value(&expected.url),
        live_string(actual, "url"),
    );
    push_change(
        &mut changes,
        "isDefault",
        Value::Bool(expected.is_default),
        normalize_bool_value(actual.get("isDefault")),
    );
    push_change(
        &mut changes,
        "orgId",
        string_value(&expected.org_id),
        live_string_or_number(actual, "orgId"),
    );
    if let Some(value) = expected.basic_auth {
        push_change(
            &mut changes,
            "basicAuth",
            Value::Bool(value),
            normalize_bool_value(actual.get("basicAuth")),
        );
    }
    push_optional_string_change(
        &mut changes,
        "basicAuthUser",
        &expected.basic_auth_user,
        actual,
    );
    push_optional_string_change(&mut changes, "database", &expected.database, actual);
    push_optional_string_change(&mut changes, "user", &expected.user, actual);
    if let Some(value) = expected.with_credentials {
        push_change(
            &mut changes,
            "withCredentials",
            Value::Bool(value),
            normalize_bool_value(actual.get("withCredentials")),
        );
    }
    if let Some(json_data) = &expected.json_data {
        push_change(
            &mut changes,
            "jsonData",
            Value::Object(json_data.clone()),
            actual.get("jsonData").cloned().unwrap_or(Value::Null),
        );
    }
    if let Some(placeholders) = &expected.secure_json_data_placeholders {
        push_change(
            &mut changes,
            "secureJsonDataPlaceholders",
            Value::Object(placeholders.clone()),
            actual
                .get("secureJsonDataPlaceholders")
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    changes
}

fn push_optional_string_change(
    changes: &mut Vec<DatasourcePlanChange>,
    field: &str,
    expected: &str,
    actual: &Map<String, Value>,
) {
    if expected.is_empty() {
        return;
    }
    push_change(
        changes,
        field,
        string_value(expected),
        live_string(actual, field),
    );
}

fn push_change(changes: &mut Vec<DatasourcePlanChange>, field: &str, before: Value, after: Value) {
    if before == after {
        return;
    }
    changes.push(DatasourcePlanChange {
        field: field.to_string(),
        before,
        after,
    });
}

fn string_value(value: &str) -> Value {
    if value.is_empty() {
        Value::Null
    } else {
        Value::String(value.to_string())
    }
}

fn live_string(actual: &Map<String, Value>, field: &str) -> Value {
    string_value(&string_field(actual, field, ""))
}

fn live_string_or_number(actual: &Map<String, Value>, field: &str) -> Value {
    match actual.get(field) {
        Some(Value::Number(value)) => Value::String(value.to_string()),
        Some(Value::String(value)) if !value.is_empty() => Value::String(value.clone()),
        _ => Value::Null,
    }
}

fn normalize_bool_value(value: Option<&Value>) -> Value {
    match value {
        Some(Value::Bool(value)) => Value::Bool(*value),
        Some(Value::String(value)) if value == "true" => Value::Bool(true),
        Some(Value::String(value)) if value == "false" => Value::Bool(false),
        _ => Value::Null,
    }
}

fn empty_to_none(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn build_org_summary(
    org_input: &DatasourcePlanOrgInput,
    actions: &[DatasourcePlanAction],
) -> DatasourcePlanOrgSummary {
    DatasourcePlanOrgSummary {
        source_org_id: empty_to_none(&org_input.source_org_id),
        source_org_name: org_input.source_org_name.clone(),
        target_org_id: org_input.target_org_id.clone(),
        target_org_name: org_input.target_org_name.clone(),
        org_action: org_input.org_action.clone(),
        input_dir: org_input.input_dir.display().to_string(),
        checked: actions.len(),
        same: actions.iter().filter(|item| item.action == "same").count(),
        create: actions
            .iter()
            .filter(|item| item.action == "would-create")
            .count(),
        update: actions
            .iter()
            .filter(|item| item.action == "would-update")
            .count(),
        extra: actions
            .iter()
            .filter(|item| item.action == "extra-remote")
            .count(),
        delete: actions
            .iter()
            .filter(|item| item.action == "would-delete")
            .count(),
        blocked: actions
            .iter()
            .filter(|item| item.status == "blocked")
            .count(),
    }
}

fn build_summary(
    orgs: &[DatasourcePlanOrgSummary],
    actions: &[DatasourcePlanAction],
) -> DatasourcePlanSummary {
    DatasourcePlanSummary {
        checked: actions.len(),
        same: actions.iter().filter(|item| item.action == "same").count(),
        create: actions
            .iter()
            .filter(|item| item.action == "would-create")
            .count(),
        update: actions
            .iter()
            .filter(|item| item.action == "would-update")
            .count(),
        extra: actions
            .iter()
            .filter(|item| item.action == "extra-remote")
            .count(),
        delete: actions
            .iter()
            .filter(|item| item.action == "would-delete")
            .count(),
        blocked: actions
            .iter()
            .filter(|item| item.status == "blocked")
            .count(),
        warning: actions
            .iter()
            .filter(|item| item.status == "warning")
            .count(),
        org_count: orgs.len(),
        would_create_org_count: orgs
            .iter()
            .filter(|item| item.org_action == "would-create")
            .count(),
    }
}

fn plan_summary_line(report: &DatasourcePlanReport) -> String {
    format!(
        "Datasource plan: checked={} same={} create={} update={} extra={} delete={} blocked={} warning={} orgs={} would-create-orgs={} prune={}",
        report.summary.checked,
        report.summary.same,
        report.summary.create,
        report.summary.update,
        report.summary.extra,
        report.summary.delete,
        report.summary.blocked,
        report.summary.warning,
        report.summary.org_count,
        report.summary.would_create_org_count,
        report.prune
    )
}

fn render_plan_text_details(report: &DatasourcePlanReport, show_same: bool) -> Vec<String> {
    report
        .actions
        .iter()
        .filter(|action| show_same || action.action != "same")
        .map(|action| {
            let changed = if action.changed_fields.is_empty() {
                "-".to_string()
            } else {
                action.changed_fields.join(",")
            };
            format!(
                "{} status={} uid={} name={} type={} fields={} reason={}",
                action.action,
                action.status,
                action.uid,
                action.name,
                action.datasource_type,
                changed,
                action.blocked_reason.as_deref().unwrap_or("-")
            )
        })
        .collect()
}

fn render_plan_table(
    report: &DatasourcePlanReport,
    show_same: bool,
    include_header: bool,
    selected_columns: &[String],
) -> Vec<String> {
    let columns = resolve_plan_columns(selected_columns);
    let rows = report
        .actions
        .iter()
        .filter(|action| show_same || action.action != "same")
        .map(plan_action_row)
        .collect::<Vec<Vec<String>>>();
    render_table_rows(&rows, &columns, include_header)
}

fn resolve_plan_columns(selected_columns: &[String]) -> Vec<(usize, &'static str)> {
    let all = vec![
        (0usize, "ACTION_ID"),
        (1usize, "ACTION"),
        (2usize, "STATUS"),
        (3usize, "UID"),
        (4usize, "NAME"),
        (5usize, "TYPE"),
        (6usize, "MATCH_BASIS"),
        (7usize, "SOURCE_ORG_ID"),
        (8usize, "TARGET_ORG_ID"),
        (9usize, "TARGET_UID"),
        (10usize, "TARGET_VERSION"),
        (11usize, "TARGET_READ_ONLY"),
        (12usize, "CHANGED_FIELDS"),
        (13usize, "BLOCKED_REASON"),
        (14usize, "SOURCE_FILE"),
    ];
    if selected_columns.is_empty() {
        return vec![
            (1usize, "ACTION"),
            (2usize, "STATUS"),
            (3usize, "UID"),
            (4usize, "NAME"),
            (5usize, "TYPE"),
            (12usize, "CHANGED_FIELDS"),
            (13usize, "BLOCKED_REASON"),
        ];
    }
    if requested_columns_include_all(selected_columns) {
        return all;
    }
    selected_columns
        .iter()
        .filter_map(|column| {
            datasource_plan_column_ids()
                .iter()
                .position(|item| item == column)
                .map(|index| all[index])
        })
        .collect()
}

fn plan_action_row(action: &DatasourcePlanAction) -> Vec<String> {
    vec![
        action.action_id.clone(),
        action.action.clone(),
        action.status.clone(),
        action.uid.clone(),
        action.name.clone(),
        action.datasource_type.clone(),
        action.match_basis.clone(),
        action.source_org_id.clone().unwrap_or_default(),
        action.target_org_id.clone().unwrap_or_default(),
        action.target_uid.clone().unwrap_or_default(),
        action
            .target_version
            .map(|value| value.to_string())
            .unwrap_or_default(),
        action
            .target_read_only
            .map(|value| value.to_string())
            .unwrap_or_default(),
        action.changed_fields.join(","),
        action.blocked_reason.clone().unwrap_or_default(),
        action.source_file.clone().unwrap_or_default(),
    ]
}

fn render_table_rows(
    rows: &[Vec<String>],
    columns: &[(usize, &'static str)],
    include_header: bool,
) -> Vec<String> {
    let headers = columns
        .iter()
        .map(|(_, header)| header.to_string())
        .collect::<Vec<String>>();
    let mut widths = headers
        .iter()
        .map(|item| item.len())
        .collect::<Vec<usize>>();
    for row in rows {
        for (index, (source_index, _)) in columns.iter().enumerate() {
            let value = row.get(*source_index).map(String::as_str).unwrap_or("");
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
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>();
    let mut lines = Vec::new();
    if include_header {
        lines.push(format_row(&headers));
        lines.push(format_row(&separator));
    }
    lines.extend(rows.iter().map(|row| {
        let values = columns
            .iter()
            .map(|(source_index, _)| row.get(*source_index).cloned().unwrap_or_default())
            .collect::<Vec<String>>();
        format_row(&values)
    }));
    lines
}

#[allow(dead_code)]
pub(crate) fn collect_action_ids(report: &DatasourcePlanReport) -> BTreeSet<String> {
    report
        .actions
        .iter()
        .map(|action| action.action_id.clone())
        .collect()
}

#[allow(dead_code)]
pub(crate) fn report_input_label(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(uid: &str, name: &str, datasource_type: &str) -> DatasourceImportRecord {
        DatasourceImportRecord {
            uid: uid.to_string(),
            name: name.to_string(),
            datasource_type: datasource_type.to_string(),
            access: "proxy".to_string(),
            url: "http://example:9090".to_string(),
            is_default: false,
            org_name: String::new(),
            org_id: "1".to_string(),
            basic_auth: None,
            basic_auth_user: String::new(),
            database: String::new(),
            json_data: None,
            read_only: None,
            version: None,
            api_version: None,
            secure_json_data_placeholders: None,
            user: String::new(),
            with_credentials: None,
        }
    }

    fn live(uid: &str, name: &str, datasource_type: &str) -> Map<String, Value> {
        json!({
            "uid": uid,
            "name": name,
            "type": datasource_type,
            "access": "proxy",
            "url": "http://example:9090",
            "isDefault": false,
            "orgId": 1,
            "version": 7
        })
        .as_object()
        .unwrap()
        .clone()
    }

    fn report(
        records: Vec<DatasourceImportRecord>,
        live: Vec<Map<String, Value>>,
        prune: bool,
    ) -> DatasourcePlanReport {
        build_datasource_plan(DatasourcePlanInput {
            scope: "current-org".to_string(),
            input_format: "inventory".to_string(),
            prune,
            orgs: vec![DatasourcePlanOrgInput {
                source_org_id: "1".to_string(),
                source_org_name: "Main".to_string(),
                target_org_id: Some("1".to_string()),
                target_org_name: "Main".to_string(),
                org_action: "exists".to_string(),
                input_dir: PathBuf::from("/tmp/datasources"),
                records,
                live,
            }],
        })
    }

    #[test]
    fn datasource_plan_marks_create_update_same_and_extra_remote() {
        let mut changed = record("loki", "Loki", "loki");
        changed.url = "http://loki:3100".to_string();
        let report = report(
            vec![
                record("prom", "Prometheus", "prometheus"),
                changed,
                record("tempo", "Tempo", "tempo"),
            ],
            vec![
                live("prom", "Prometheus", "prometheus"),
                live("loki", "Loki", "loki"),
                live("remote", "Remote Only", "prometheus"),
            ],
            false,
        );

        assert_eq!(report.summary.same, 1);
        assert_eq!(report.summary.create, 1);
        assert_eq!(report.summary.update, 1);
        assert_eq!(report.summary.extra, 1);
        assert_eq!(report.summary.delete, 0);
        assert!(report
            .actions
            .iter()
            .any(|item| item.action == "would-update" && item.changed_fields == vec!["url"]));
    }

    #[test]
    fn datasource_plan_prune_turns_extra_remote_into_delete_candidate() {
        let report = report(
            Vec::new(),
            vec![live("remote", "Remote Only", "loki")],
            true,
        );

        assert_eq!(report.summary.extra, 0);
        assert_eq!(report.summary.delete, 1);
        assert_eq!(report.actions[0].action, "would-delete");
        assert_eq!(report.actions[0].status, "ready");
    }

    #[test]
    fn datasource_plan_blocks_read_only_update_and_delete() {
        let mut changed = record("prom", "Prometheus", "prometheus");
        changed.url = "http://new-prometheus:9090".to_string();
        let mut live_record = live("prom", "Prometheus", "prometheus");
        live_record.insert("readOnly".to_string(), Value::Bool(true));
        let report = report(vec![changed], vec![live_record], true);

        assert_eq!(report.summary.blocked, 1);
        assert_eq!(report.actions[0].action, "blocked-read-only");
        assert_eq!(
            report.actions[0].blocked_reason.as_deref(),
            Some("target-read-only")
        );
    }

    #[test]
    fn datasource_plan_json_keeps_tui_stable_action_id() {
        let report = report(
            vec![record("prom", "Prometheus", "prometheus")],
            vec![live("prom", "Prometheus", "prometheus")],
            false,
        );
        let value = build_datasource_plan_json(&report).unwrap();

        assert_eq!(value["kind"], json!("grafana-util-datasource-plan"));
        assert_eq!(value["schemaVersion"], json!(1));
        assert_eq!(
            value["actions"][0]["actionId"],
            json!("org:1/datasource:uid:prom")
        );
    }
}
