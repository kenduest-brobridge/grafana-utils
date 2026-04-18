use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::common::{message, string_field, tool_version, Result};

use super::super::DatasourceImportRecord;
use super::model::{
    DatasourcePlanAction, DatasourcePlanChange, DatasourcePlanInput, DatasourcePlanOrgInput,
    DatasourcePlanOrgSummary, DatasourcePlanReport, DatasourcePlanSummary,
    PLAN_ACTION_BLOCKED_AMBIGUOUS, PLAN_ACTION_BLOCKED_MISSING_ORG, PLAN_ACTION_BLOCKED_READ_ONLY,
    PLAN_ACTION_BLOCKED_UID_MISMATCH, PLAN_ACTION_EXTRA_REMOTE, PLAN_ACTION_SAME,
    PLAN_ACTION_WOULD_CREATE, PLAN_ACTION_WOULD_DELETE, PLAN_ACTION_WOULD_UPDATE,
    PLAN_HINT_MISSING_REMOTE, PLAN_HINT_REMOTE_ONLY, PLAN_HINT_REQUIRES_SECRET_VALUES,
    PLAN_REASON_AMBIGUOUS_LIVE_NAME_MATCH, PLAN_REASON_TARGET_ORG_MISSING,
    PLAN_REASON_TARGET_READ_ONLY, PLAN_REASON_UID_NAME_MISMATCH, PLAN_STATUS_BLOCKED,
    PLAN_STATUS_READY, PLAN_STATUS_SAME, PLAN_STATUS_WARNING,
};

const PLAN_KIND: &str = "grafana-util-datasource-plan";
const PLAN_SCHEMA_VERSION: i64 = 1;

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

fn build_org_actions(org_input: &DatasourcePlanOrgInput, prune: bool) -> Vec<DatasourcePlanAction> {
    if org_input.target_org_id.is_none() && org_input.org_action != PLAN_ACTION_WOULD_CREATE {
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
                    action: PLAN_ACTION_BLOCKED_MISSING_ORG,
                    status: PLAN_STATUS_BLOCKED,
                    blocked_reason: Some(PLAN_REASON_TARGET_ORG_MISSING.to_string()),
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
                action: PLAN_ACTION_WOULD_CREATE,
                status: PLAN_STATUS_READY,
                blocked_reason: None,
                review_hints: vec![PLAN_HINT_MISSING_REMOTE.to_string()],
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
                action: PLAN_ACTION_BLOCKED_AMBIGUOUS,
                status: PLAN_STATUS_BLOCKED,
                blocked_reason: Some(PLAN_REASON_AMBIGUOUS_LIVE_NAME_MATCH.to_string()),
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
                action: PLAN_ACTION_BLOCKED_UID_MISMATCH,
                status: PLAN_STATUS_BLOCKED,
                blocked_reason: Some(PLAN_REASON_UID_NAME_MISMATCH.to_string()),
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
            PLAN_ACTION_BLOCKED_READ_ONLY,
            PLAN_STATUS_BLOCKED,
            Some(PLAN_REASON_TARGET_READ_ONLY.to_string()),
        )
    } else if changes.is_empty() {
        (PLAN_ACTION_SAME, PLAN_STATUS_SAME, None)
    } else {
        (PLAN_ACTION_WOULD_UPDATE, PLAN_STATUS_READY, None)
    };
    let mut hints = Vec::new();
    if record.secure_json_data_placeholders.is_some() {
        hints.push(PLAN_HINT_REQUIRES_SECRET_VALUES.to_string());
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
                .or_else(|| {
                    (!org_input.source_org_id.is_empty())
                        .then_some(org_input.source_org_id.as_str())
                })
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
            PLAN_ACTION_BLOCKED_READ_ONLY
        } else {
            PLAN_ACTION_WOULD_DELETE
        }
    } else {
        PLAN_ACTION_EXTRA_REMOTE
    };
    let status = if prune && read_only {
        PLAN_STATUS_BLOCKED
    } else if prune {
        PLAN_STATUS_READY
    } else {
        PLAN_STATUS_WARNING
    };
    let blocked_reason = (prune && read_only).then_some(PLAN_REASON_TARGET_READ_ONLY.to_string());
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
        review_hints: vec![PLAN_HINT_REMOTE_ONLY.to_string()],
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
        same: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_SAME)
            .count(),
        create: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_WOULD_CREATE)
            .count(),
        update: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_WOULD_UPDATE)
            .count(),
        extra: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_EXTRA_REMOTE)
            .count(),
        delete: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_WOULD_DELETE)
            .count(),
        blocked: actions
            .iter()
            .filter(|item| item.status == PLAN_STATUS_BLOCKED)
            .count(),
    }
}

fn build_summary(
    orgs: &[DatasourcePlanOrgSummary],
    actions: &[DatasourcePlanAction],
) -> DatasourcePlanSummary {
    DatasourcePlanSummary {
        checked: actions.len(),
        same: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_SAME)
            .count(),
        create: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_WOULD_CREATE)
            .count(),
        update: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_WOULD_UPDATE)
            .count(),
        extra: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_EXTRA_REMOTE)
            .count(),
        delete: actions
            .iter()
            .filter(|item| item.action == PLAN_ACTION_WOULD_DELETE)
            .count(),
        blocked: actions
            .iter()
            .filter(|item| item.status == PLAN_STATUS_BLOCKED)
            .count(),
        warning: actions
            .iter()
            .filter(|item| item.status == PLAN_STATUS_WARNING)
            .count(),
        org_count: orgs.len(),
        would_create_org_count: orgs
            .iter()
            .filter(|item| item.org_action == PLAN_ACTION_WOULD_CREATE)
            .count(),
    }
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
