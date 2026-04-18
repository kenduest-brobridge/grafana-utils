//! Service-account access plan review helpers.
//!
//! Minimal vertical slice:
//! - local export bundle vs live Grafana comparison
//! - stable review document and action rows
//! - TUI-friendly action metadata

use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::access::cli_defs::AccessPlanArgs;
use crate::access::render::{map_get_text, normalize_service_account_row, value_bool};
use crate::access::service_account::{
    build_service_account_diff_map, list_all_service_accounts_with_request,
    load_service_account_import_records,
};
use crate::access::user::build_record_diff_fields;
use crate::access::{ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME};
use crate::common::{message, tool_version, Result};

use super::{
    sort_actions, AccessPlanAction, AccessPlanChange, AccessPlanDocument, AccessPlanResourceReport,
    AccessPlanSummary,
};

type ServiceAccountPlanIndex = BTreeMap<String, (String, Map<String, Value>)>;

fn build_action_id(identity: &str) -> String {
    format!("access:service-account:{identity}")
}

fn build_target_evidence(record: &Map<String, Value>) -> Map<String, Value> {
    let mut target = Map::new();
    for key in ["id", "name", "login", "role", "disabled", "tokens", "orgId"] {
        if let Some(value) = record.get(key) {
            target.insert(key.to_string(), value.clone());
        }
    }
    if let Some(is_disabled) =
        value_bool(record.get("disabled")).or_else(|| value_bool(record.get("isDisabled")))
    {
        target.insert("disabled".to_string(), Value::Bool(is_disabled));
    }
    target
}

fn review_hints(record: &Map<String, Value>) -> Vec<String> {
    let mut hints = Vec::new();
    let role = map_get_text(record, "role");
    if role.eq_ignore_ascii_case("admin") {
        hints.push("service-account admin role deserves manual review".to_string());
    }
    if value_bool(record.get("disabled")).unwrap_or(false) {
        hints.push("service-account is disabled".to_string());
    }
    hints
}

fn service_account_scope(record: &Map<String, Value>) -> Option<String> {
    let org_id = map_get_text(record, "orgId");
    if org_id.is_empty() {
        None
    } else {
        Some(format!("orgId={org_id}"))
    }
}

#[allow(clippy::too_many_arguments)]
fn build_service_account_action(
    identity: String,
    source_path: String,
    action: &'static str,
    status: &'static str,
    changed_fields: Vec<String>,
    changes: Vec<AccessPlanChange>,
    target: Option<Map<String, Value>>,
    blocked_reason: Option<String>,
    review_hints: Vec<String>,
) -> AccessPlanAction {
    AccessPlanAction {
        action_id: build_action_id(&identity),
        domain: "access".to_string(),
        resource_kind: "service-account".to_string(),
        identity,
        scope: target.as_ref().and_then(service_account_scope),
        action: action.to_string(),
        status: status.to_string(),
        changed_fields,
        changes,
        target,
        blocked_reason,
        review_hints,
        source_path,
    }
}

fn build_diff_rows(
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

#[allow(clippy::too_many_arguments)]
fn build_service_account_report(
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
) -> AccessPlanResourceReport {
    AccessPlanResourceReport {
        resource_kind: "service-account".to_string(),
        source_path,
        bundle_present: true,
        source_count,
        live_count,
        checked,
        same,
        create,
        update,
        extra_remote,
        delete,
        blocked,
        warning,
        scope: None,
        notes: vec![
            "vertical slice: service-account resource only".to_string(),
            "review role and disabled state before applying".to_string(),
        ],
    }
}

fn index_service_account_records(
    records: &[Map<String, Value>],
    source: &str,
) -> Result<ServiceAccountPlanIndex> {
    let mut indexed = BTreeMap::new();
    for record in records {
        let identity = map_get_text(record, "name");
        if identity.trim().is_empty() {
            return Err(message(format!(
                "Service-account plan record in {} does not include name.",
                source
            )));
        }
        let key = identity.trim().to_ascii_lowercase();
        if indexed.contains_key(&key) {
            return Err(message(format!(
                "Duplicate service-account name in {}: {}",
                source, identity
            )));
        }
        indexed.insert(key, (identity.clone(), record.clone()));
    }
    Ok(indexed)
}

pub(crate) fn build_service_account_plan_document<F>(
    mut request_json: F,
    args: &AccessPlanArgs,
) -> Result<AccessPlanDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let local_records =
        load_service_account_import_records(&args.input_dir, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS)?;
    let local_map =
        build_service_account_diff_map(&local_records, &args.input_dir.to_string_lossy())?;
    let local_evidence =
        index_service_account_records(&local_records, &args.input_dir.to_string_lossy())?;

    let live_records = list_all_service_accounts_with_request(&mut request_json)?
        .into_iter()
        .map(|item| normalize_service_account_row(&item))
        .collect::<Vec<Map<String, Value>>>();
    let live_map = build_service_account_diff_map(&live_records, "Grafana live service accounts")?;
    let live_evidence =
        index_service_account_records(&live_records, "Grafana live service accounts")?;

    let mut resources = Vec::new();
    let mut actions = Vec::new();
    let mut checked = 0usize;
    let mut same = 0usize;
    let mut create = 0usize;
    let mut update = 0usize;
    let mut extra_remote = 0usize;
    let mut delete = 0usize;
    let blocked = 0usize;
    let mut warning = 0usize;

    let source_path = args
        .input_dir
        .join(ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME)
        .to_string_lossy()
        .to_string();

    for key in local_map.keys() {
        checked += 1;
        let (identity, local_payload) = &local_map[key];
        let local_record = local_evidence
            .get(key)
            .map(|(_, record)| record)
            .unwrap_or(local_payload);
        match live_map.get(key) {
            None => {
                create += 1;
                let hints = review_hints(local_record);
                if !hints.is_empty() {
                    warning += 1;
                }
                actions.push(build_service_account_action(
                    identity.clone(),
                    source_path.clone(),
                    "would-create",
                    if hints.is_empty() { "ready" } else { "warning" },
                    Vec::new(),
                    Vec::new(),
                    Some(build_target_evidence(local_record)),
                    None,
                    hints,
                ));
            }
            Some((_, live_payload)) => {
                let live_record = live_evidence
                    .get(key)
                    .map(|(_, record)| record)
                    .unwrap_or(live_payload);
                let (changed_fields, changes) = build_diff_rows(local_payload, live_payload);
                if changed_fields.is_empty() {
                    same += 1;
                    actions.push(build_service_account_action(
                        identity.clone(),
                        source_path.clone(),
                        "same",
                        "same",
                        Vec::new(),
                        Vec::new(),
                        Some(build_target_evidence(live_record)),
                        None,
                        Vec::new(),
                    ));
                } else {
                    update += 1;
                    let hints = review_hints(live_record);
                    warning += 1;
                    actions.push(build_service_account_action(
                        identity.clone(),
                        source_path.clone(),
                        "would-update",
                        "warning",
                        changed_fields,
                        changes,
                        Some(build_target_evidence(live_record)),
                        None,
                        hints,
                    ));
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
        let live_record = live_evidence
            .get(key)
            .map(|(_, record)| record)
            .unwrap_or(live_payload);
        let hints = review_hints(live_record);
        let (action, status) = if args.prune {
            delete += 1;
            ("would-delete", "ready")
        } else {
            warning += 1;
            ("extra-remote", "warning")
        };
        actions.push(build_service_account_action(
            identity.clone(),
            source_path.clone(),
            action,
            status,
            Vec::new(),
            Vec::new(),
            Some(build_target_evidence(live_record)),
            if args.prune {
                None
            } else {
                Some("use --prune to include delete candidates".to_string())
            },
            hints,
        ));
    }

    let resource = build_service_account_report(
        source_path.clone(),
        local_map.len(),
        live_map.len(),
        checked,
        same,
        create,
        update,
        extra_remote,
        delete,
        blocked,
        warning,
    );
    resources.push(resource);
    sort_actions(&mut actions);

    Ok(AccessPlanDocument {
        kind: super::ACCESS_PLAN_KIND.to_string(),
        schema_version: super::ACCESS_PLAN_SCHEMA_VERSION,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::cli_defs::{
        AccessPlanArgs, AccessPlanResource, CommonCliArgs, PlanOutputFormat,
    };
    use reqwest::Method;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
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

    fn make_args(input_dir: &Path, prune: bool) -> AccessPlanArgs {
        AccessPlanArgs {
            common: make_common(),
            input_dir: input_dir.to_path_buf(),
            resource: AccessPlanResource::ServiceAccount,
            prune,
            output_columns: Vec::new(),
            list_columns: false,
            no_header: false,
            show_same: false,
            output_format: PlanOutputFormat::Text,
        }
    }

    fn write_bundle(dir: &Path) {
        fs::write(
            dir.join("service-accounts.json"),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-access-service-account-export-index",
                "version": 1,
                "records": [
                    {"name": "svc-same", "login": "sa-same", "role": "Viewer", "disabled": false, "orgId": 1},
                    {"name": "svc-create", "login": "sa-create", "role": "Editor", "disabled": true, "orgId": 1},
                    {"name": "svc-update", "login": "sa-update", "role": "Viewer", "disabled": false, "orgId": 1}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn mock_live_response(method: Method, path: &str) -> Option<Value> {
        match (method, path) {
            (Method::GET, "/api/serviceaccounts/search") => Some(json!({
                "serviceAccounts": [
                    {"id": 1, "name": "svc-same", "login": "sa-same", "role": "Viewer", "disabled": false, "tokens": 1, "orgId": 1},
                    {"id": 2, "name": "svc-update", "login": "sa-update", "role": "Editor", "disabled": false, "tokens": 2, "orgId": 1},
                    {"id": 3, "name": "svc-extra", "login": "sa-extra", "role": "Viewer", "disabled": false, "tokens": 0, "orgId": 1}
                ]
            })),
            _ => None,
        }
    }

    #[test]
    fn service_account_plan_reports_create_same_update_and_extra_remote() {
        let temp_dir = tempdir().unwrap();
        write_bundle(temp_dir.path());
        let args = make_args(temp_dir.path(), false);
        let document = build_service_account_plan_document(
            |method, path, _params, _payload| {
                mock_live_response(method, path)
                    .map(Some)
                    .ok_or_else(|| crate::common::message(format!("unexpected path {path}")))
            },
            &args,
        )
        .unwrap();

        assert_eq!(document.kind, "grafana-util-access-plan");
        assert_eq!(document.resources[0].resource_kind, "service-account");
        assert_eq!(document.summary.checked, 4);
        assert_eq!(document.summary.same, 1);
        assert_eq!(document.summary.create, 1);
        assert_eq!(document.summary.update, 1);
        assert_eq!(document.summary.extra_remote, 1);
        assert_eq!(document.summary.delete, 0);

        let actions: BTreeMap<_, _> = document
            .actions
            .iter()
            .map(|action| (action.identity.clone(), action))
            .collect();
        assert_eq!(actions["svc-same"].action, "same");
        assert_eq!(actions["svc-create"].action, "would-create");
        assert_eq!(actions["svc-update"].action, "would-update");
        assert_eq!(actions["svc-extra"].action, "extra-remote");
        assert!(actions["svc-create"]
            .review_hints
            .iter()
            .any(|hint| hint.contains("disabled")));
        assert!(actions["svc-update"]
            .changes
            .iter()
            .any(|change| change.field == "role"));
    }

    #[test]
    fn service_account_plan_turns_remote_only_rows_into_delete_candidates_with_prune() {
        let temp_dir = tempdir().unwrap();
        write_bundle(temp_dir.path());
        let args = make_args(temp_dir.path(), true);
        let document = build_service_account_plan_document(
            |method, path, _params, _payload| {
                mock_live_response(method, path)
                    .map(Some)
                    .ok_or_else(|| crate::common::message(format!("unexpected path {path}")))
            },
            &args,
        )
        .unwrap();

        assert_eq!(document.summary.extra_remote, 1);
        assert_eq!(document.summary.delete, 1);
        let action = document
            .actions
            .iter()
            .find(|action| action.identity == "svc-extra")
            .unwrap();
        assert_eq!(action.action, "would-delete");
        assert_eq!(action.status, "ready");
    }
}
