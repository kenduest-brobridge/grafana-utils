//! Live datasource domain-status producer.
//!
//! Maintainer note:
//! - This module derives one datasource-owned domain-status row from live
//!   datasource inventory surfaces.
//! - Keep it conservative and source-attributable: prefer the live list
//!   response, fall back to a single live read response when needed, and only
//!   derive counts that are directly visible in the payloads.

use serde_json::{Map, Value};
use std::collections::BTreeSet;

use crate::common::{string_field, Result};
use crate::grafana_api::datasource_live_project_status as datasource_live_project_status_support;
use crate::project_status::{
    status_finding, ProjectDomainStatus, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY,
};

const DATASOURCE_DOMAIN_ID: &str = "datasource";
const DATASOURCE_SCOPE: &str = "live";
const DATASOURCE_MODE: &str = "live-inventory";
const DATASOURCE_REASON_READY: &str = PROJECT_STATUS_READY;
const DATASOURCE_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";

const DATASOURCE_SOURCE_KIND_LIST: &str = "live-datasource-list";
const DATASOURCE_SOURCE_KIND_READ: &str = "live-datasource-read";
const DATASOURCE_SOURCE_KIND_ORG_LIST: &str = "live-org-list";
const DATASOURCE_SOURCE_KIND_ORG_READ: &str = "live-org-read";

const DATASOURCE_SIGNAL_KEYS: &[&str] = &[
    "live.datasourceCount",
    "live.defaultCount",
    "live.orgCount",
    "live.orgIdCount",
    "live.uidCount",
    "live.nameCount",
    "live.accessCount",
    "live.typeCount",
    "live.jsonDataCount",
    "live.basicAuthCount",
    "live.basicAuthPasswordCount",
    "live.passwordCount",
    "live.httpHeaderValueCount",
    "live.withCredentialsCount",
    "live.secureJsonFieldsCount",
    "live.tlsAuthCount",
    "live.tlsSkipVerifyCount",
    "live.serverNameCount",
    "live.readOnlyCount",
];

const DATASOURCE_WARNING_MISSING_DEFAULT: &str = "missing-default";
const DATASOURCE_WARNING_MULTIPLE_DEFAULTS: &str = "multiple-defaults";
const DATASOURCE_WARNING_MISSING_UID: &str = "missing-uid";
const DATASOURCE_WARNING_DUPLICATE_UID: &str = "duplicate-uid";
const DATASOURCE_WARNING_MISSING_NAME: &str = "missing-name";
const DATASOURCE_WARNING_MISSING_ACCESS: &str = "missing-access";
const DATASOURCE_WARNING_MISSING_TYPE: &str = "missing-type";
const DATASOURCE_WARNING_MISSING_ORG_ID: &str = "missing-org-id";
const DATASOURCE_WARNING_MIXED_ORG_IDS: &str = "mixed-org-ids";
const DATASOURCE_WARNING_ORG_SCOPE_MISMATCH: &str = "org-scope-mismatch";
const DATASOURCE_WARNING_ORG_LIST_MISMATCH: &str = "org-list-mismatch";
const DATASOURCE_WARNING_PROVIDER_JSON_DATA: &str = "provider-json-data-present";
const DATASOURCE_WARNING_BASIC_AUTH: &str = "basic-auth-configured";
const DATASOURCE_WARNING_BASIC_AUTH_PASSWORD: &str = "basic-auth-password-present";
const DATASOURCE_WARNING_PASSWORD: &str = "datasource-password-present";
const DATASOURCE_WARNING_HTTP_HEADER_VALUES: &str = "http-header-secret-values-present";
const DATASOURCE_WARNING_WITH_CREDENTIALS: &str = "with-credentials-configured";
const DATASOURCE_WARNING_SECURE_JSON_FIELDS: &str = "secure-json-fields-present";
const DATASOURCE_WARNING_TLS_AUTH: &str = "tls-auth-configured";
const DATASOURCE_WARNING_TLS_SKIP_VERIFY: &str = "tls-skip-verify-configured";
const DATASOURCE_WARNING_SERVER_NAME: &str = "server-name-configured";
const DATASOURCE_WARNING_READ_ONLY: &str = "read-only";

const DATASOURCE_CREATE_OR_SYNC_ACTIONS: &[&str] =
    &["create or sync at least one datasource in Grafana"];
const DATASOURCE_MARK_DEFAULT_ACTIONS: &[&str] = &["mark a default datasource in Grafana"];
const DATASOURCE_KEEP_SINGLE_DEFAULT_ACTIONS: &[&str] =
    &["keep exactly one datasource marked as the default"];
const DATASOURCE_FIX_METADATA_ACTIONS: &[&str] =
    &["re-run live datasource read after correcting datasource identity or org scope"];
const DATASOURCE_REVIEW_SECRET_PROVIDER_ACTIONS: &[&str] =
    &["review live datasource secret and provider fields before export or import"];

#[derive(Debug, Clone, Default)]
pub(crate) struct DatasourceLiveProjectStatusInputs<'a> {
    pub datasource_list: Option<&'a [Map<String, Value>]>,
    pub datasource_read: Option<&'a Map<String, Value>>,
    pub org_list: Option<&'a [Map<String, Value>]>,
    pub current_org: Option<&'a Map<String, Value>>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LiveDatasourceProjectStatusInputs {
    pub datasource_list: Vec<Map<String, Value>>,
    pub org_list: Vec<Map<String, Value>>,
    pub current_org: Option<Map<String, Value>>,
}

fn record_bool(record: &Map<String, Value>, key: &str) -> bool {
    record.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn record_string(record: &Map<String, Value>, key: &str) -> String {
    string_field(record, key, "")
}

fn record_scalar(record: &Map<String, Value>, key: &str) -> String {
    match record.get(key) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn nested_object<'a>(record: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    record.get(key).and_then(Value::as_object)
}

fn nested_record_bool(record: &Map<String, Value>, parent_key: &str, child_key: &str) -> bool {
    nested_object(record, parent_key)
        .and_then(|object| object.get(child_key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn nested_record_string(record: &Map<String, Value>, parent_key: &str, child_key: &str) -> String {
    nested_object(record, parent_key)
        .and_then(|object| object.get(child_key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_default()
}

fn distinct_non_empty_values(records: &[Map<String, Value>], key: &str) -> usize {
    let mut values = BTreeSet::new();
    for record in records {
        let value = record_string(record, key);
        if !value.is_empty() {
            values.insert(value);
        }
    }
    values.len()
}

fn distinct_non_empty_scalar_values(records: &[Map<String, Value>], key: &str) -> usize {
    let mut values = BTreeSet::new();
    for record in records {
        let value = record_scalar(record, key);
        if !value.is_empty() {
            values.insert(value);
        }
    }
    values.len()
}

fn missing_string_values(records: &[Map<String, Value>], key: &str) -> usize {
    records
        .iter()
        .filter(|record| record_string(record, key).is_empty())
        .count()
}

fn missing_scalar_values(records: &[Map<String, Value>], key: &str) -> usize {
    records
        .iter()
        .filter(|record| record_scalar(record, key).is_empty())
        .count()
}

fn non_empty_object_values(records: &[Map<String, Value>], key: &str) -> usize {
    records
        .iter()
        .filter(|record| {
            record
                .get(key)
                .and_then(Value::as_object)
                .map(|object| !object.is_empty())
                .unwrap_or(false)
        })
        .count()
}

fn nested_bool_values(records: &[Map<String, Value>], parent_key: &str, child_key: &str) -> usize {
    records
        .iter()
        .filter(|record| nested_record_bool(record, parent_key, child_key))
        .count()
}

fn nested_string_values(
    records: &[Map<String, Value>],
    parent_key: &str,
    child_key: &str,
) -> usize {
    records
        .iter()
        .filter(|record| !nested_record_string(record, parent_key, child_key).is_empty())
        .count()
}

fn nested_bool_key_prefix_values(
    records: &[Map<String, Value>],
    parent_key: &str,
    child_key_prefix: &str,
) -> usize {
    records
        .iter()
        .map(|record| {
            nested_object(record, parent_key)
                .map(|object| {
                    object
                        .iter()
                        .filter(|(key, value)| {
                            key.starts_with(child_key_prefix) && value.as_bool().unwrap_or(false)
                        })
                        .count()
                })
                .unwrap_or(0)
        })
        .sum()
}

fn datasource_records<'a>(
    inputs: &DatasourceLiveProjectStatusInputs<'a>,
) -> (&'a [Map<String, Value>], &'static str) {
    if let Some(records) = inputs.datasource_list {
        return (records, DATASOURCE_SOURCE_KIND_LIST);
    }
    if let Some(record) = inputs.datasource_read {
        return (std::slice::from_ref(record), DATASOURCE_SOURCE_KIND_READ);
    }
    (&[], DATASOURCE_SOURCE_KIND_LIST)
}

pub(crate) fn datasource_live_project_status_org_count(
    inputs: &DatasourceLiveProjectStatusInputs<'_>,
    records: &[Map<String, Value>],
) -> usize {
    if let Some(orgs) = inputs.org_list {
        return orgs.len();
    }
    if inputs.current_org.is_some() {
        return 1;
    }

    let mut org_ids = BTreeSet::new();
    for record in records {
        let org_id = record_scalar(record, "orgId");
        if !org_id.is_empty() {
            org_ids.insert(org_id);
        }
    }
    org_ids.len()
}

pub(crate) fn datasource_live_project_status_source_kinds(
    inputs: &DatasourceLiveProjectStatusInputs<'_>,
    datasource_source_kind: &'static str,
) -> Vec<String> {
    let mut source_kinds = vec![datasource_source_kind.to_string()];
    if inputs.org_list.is_some() {
        source_kinds.push(DATASOURCE_SOURCE_KIND_ORG_LIST.to_string());
    } else if inputs.current_org.is_some() {
        source_kinds.push(DATASOURCE_SOURCE_KIND_ORG_READ.to_string());
    }
    source_kinds
}

pub(crate) fn collect_live_datasource_project_status_inputs_with_request<F>(
    request_json: &mut F,
) -> Result<LiveDatasourceProjectStatusInputs>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    datasource_live_project_status_support::collect_live_datasource_project_status_inputs_with_request(
        request_json,
    )
}

pub(crate) fn build_datasource_live_project_status_from_inputs(
    inputs: &LiveDatasourceProjectStatusInputs,
) -> Option<ProjectDomainStatus> {
    build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
        datasource_list: Some(&inputs.datasource_list),
        datasource_read: None,
        org_list: if inputs.org_list.is_empty() {
            None
        } else {
            Some(&inputs.org_list)
        },
        current_org: inputs.current_org.as_ref(),
    })
}

pub(crate) fn build_datasource_live_project_status(
    inputs: DatasourceLiveProjectStatusInputs<'_>,
) -> Option<ProjectDomainStatus> {
    if inputs.datasource_list.is_none() && inputs.datasource_read.is_none() {
        return None;
    }

    let (records, datasource_source_kind) = datasource_records(&inputs);
    let datasource_count = records.len();
    let default_count = records
        .iter()
        .filter(|record| record_bool(record, "isDefault"))
        .count();
    let uid_count = distinct_non_empty_values(records, "uid");
    let _name_count = distinct_non_empty_values(records, "name");
    let _access_count = distinct_non_empty_values(records, "access");
    let _type_count = distinct_non_empty_values(records, "type");
    let org_id_count = distinct_non_empty_scalar_values(records, "orgId");
    let missing_uid_count = missing_string_values(records, "uid");
    let missing_name_count = missing_string_values(records, "name");
    let missing_access_count = missing_string_values(records, "access");
    let missing_type_count = missing_string_values(records, "type");
    let missing_org_id_count = missing_scalar_values(records, "orgId");
    let basic_auth_count = records
        .iter()
        .filter(|record| record_bool(record, "basicAuth"))
        .count();
    let basic_auth_password_count =
        nested_bool_values(records, "secureJsonFields", "basicAuthPassword");
    let password_count = nested_bool_values(records, "secureJsonFields", "password");
    let http_header_value_count =
        nested_bool_key_prefix_values(records, "secureJsonFields", "httpHeaderValue");
    let with_credentials_count = records
        .iter()
        .filter(|record| record_bool(record, "withCredentials"))
        .count();
    // Non-empty jsonData is the broadest non-secret provider/config surface in live inventory.
    let json_data_count = non_empty_object_values(records, "jsonData");
    let secure_json_fields_count = non_empty_object_values(records, "secureJsonFields");
    let tls_auth_count = nested_bool_values(records, "jsonData", "tlsAuth");
    let tls_skip_verify_count = nested_bool_values(records, "jsonData", "tlsSkipVerify");
    let server_name_count = nested_string_values(records, "jsonData", "serverName");
    let read_only_count = records
        .iter()
        .filter(|record| record_bool(record, "readOnly"))
        .count();
    let _org_count = datasource_live_project_status_org_count(&inputs, records);
    let current_org_id = inputs
        .current_org
        .map(|record| record_scalar(record, "id"))
        .filter(|value| !value.is_empty());
    let org_list_ids = inputs.org_list.map(|orgs| {
        orgs.iter()
            .map(|record| record_scalar(record, "id"))
            .filter(|value| !value.is_empty())
            .collect::<BTreeSet<_>>()
    });

    let source_kinds = datasource_live_project_status_source_kinds(&inputs, datasource_source_kind);

    let mut warnings = Vec::new();
    let mut metadata_issue_found = false;
    let mut readiness_signal_found = false;
    if missing_uid_count > 0 {
        metadata_issue_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_MISSING_UID,
            missing_uid_count,
            "live.uidCount",
        ));
    }
    if missing_name_count > 0 {
        metadata_issue_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_MISSING_NAME,
            missing_name_count,
            "live.nameCount",
        ));
    }
    if missing_access_count > 0 {
        metadata_issue_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_MISSING_ACCESS,
            missing_access_count,
            "live.accessCount",
        ));
    }
    if datasource_count > 0 && uid_count + missing_uid_count < datasource_count {
        metadata_issue_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_DUPLICATE_UID,
            datasource_count - uid_count - missing_uid_count,
            "live.uidCount",
        ));
    }
    if missing_type_count > 0 {
        metadata_issue_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_MISSING_TYPE,
            missing_type_count,
            "live.typeCount",
        ));
    }
    if missing_org_id_count > 0 {
        metadata_issue_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_MISSING_ORG_ID,
            missing_org_id_count,
            "live.orgIdCount",
        ));
    }
    if inputs.current_org.is_some() && org_id_count > 1 {
        metadata_issue_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_MIXED_ORG_IDS,
            org_id_count - 1,
            "live.orgIdCount",
        ));
    }
    if let Some(current_org_id) = current_org_id.as_ref() {
        if org_id_count == 1 {
            let datasource_org_id = records
                .iter()
                .map(|record| record_scalar(record, "orgId"))
                .find(|value| !value.is_empty())
                .unwrap_or_default();
            if !datasource_org_id.is_empty() && datasource_org_id != *current_org_id {
                metadata_issue_found = true;
                warnings.push(status_finding(
                    DATASOURCE_WARNING_ORG_SCOPE_MISMATCH,
                    1,
                    "live.orgIdCount",
                ));
            }
        }
    }
    if let Some(org_list_ids) = org_list_ids.as_ref() {
        let datasource_org_ids = records
            .iter()
            .map(|record| record_scalar(record, "orgId"))
            .filter(|value| !value.is_empty())
            .collect::<BTreeSet<_>>();
        let missing_org_ids = datasource_org_ids.difference(org_list_ids).count();
        if missing_org_ids > 0 {
            metadata_issue_found = true;
            warnings.push(status_finding(
                DATASOURCE_WARNING_ORG_LIST_MISMATCH,
                missing_org_ids,
                "live.orgCount",
            ));
        }
    }
    if json_data_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_PROVIDER_JSON_DATA,
            json_data_count,
            "live.jsonDataCount",
        ));
    }
    if basic_auth_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_BASIC_AUTH,
            basic_auth_count,
            "live.basicAuthCount",
        ));
    }
    if basic_auth_password_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_BASIC_AUTH_PASSWORD,
            basic_auth_password_count,
            "live.basicAuthPasswordCount",
        ));
    }
    if password_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_PASSWORD,
            password_count,
            "live.passwordCount",
        ));
    }
    if http_header_value_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_HTTP_HEADER_VALUES,
            http_header_value_count,
            "live.httpHeaderValueCount",
        ));
    }
    if with_credentials_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_WITH_CREDENTIALS,
            with_credentials_count,
            "live.withCredentialsCount",
        ));
    }
    if secure_json_fields_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_SECURE_JSON_FIELDS,
            secure_json_fields_count,
            "live.secureJsonFieldsCount",
        ));
    }
    if tls_auth_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_TLS_AUTH,
            tls_auth_count,
            "live.tlsAuthCount",
        ));
    }
    if tls_skip_verify_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_TLS_SKIP_VERIFY,
            tls_skip_verify_count,
            "live.tlsSkipVerifyCount",
        ));
    }
    if server_name_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_SERVER_NAME,
            server_name_count,
            "live.serverNameCount",
        ));
    }
    if read_only_count > 0 {
        readiness_signal_found = true;
        warnings.push(status_finding(
            DATASOURCE_WARNING_READ_ONLY,
            read_only_count,
            "live.readOnlyCount",
        ));
    }

    let mut next_actions = if datasource_count == 0 {
        DATASOURCE_CREATE_OR_SYNC_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    } else if default_count == 0 {
        warnings.push(status_finding(
            DATASOURCE_WARNING_MISSING_DEFAULT,
            1,
            "live.defaultCount",
        ));
        DATASOURCE_MARK_DEFAULT_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    } else if default_count > 1 {
        warnings.push(status_finding(
            DATASOURCE_WARNING_MULTIPLE_DEFAULTS,
            default_count - 1,
            "live.defaultCount",
        ));
        DATASOURCE_KEEP_SINGLE_DEFAULT_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    } else {
        Vec::new()
    };
    if metadata_issue_found && datasource_count > 0 {
        next_actions.extend(
            DATASOURCE_FIX_METADATA_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }
    if readiness_signal_found && datasource_count > 0 {
        next_actions.extend(
            DATASOURCE_REVIEW_SECRET_PROVIDER_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }

    let (status, reason_code) = if datasource_count == 0 {
        (PROJECT_STATUS_PARTIAL, DATASOURCE_REASON_PARTIAL_NO_DATA)
    } else {
        (PROJECT_STATUS_READY, DATASOURCE_REASON_READY)
    };

    Some(ProjectDomainStatus {
        id: DATASOURCE_DOMAIN_ID.to_string(),
        scope: DATASOURCE_SCOPE.to_string(),
        mode: DATASOURCE_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: datasource_count,
        blocker_count: 0,
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds,
        signal_keys: DATASOURCE_SIGNAL_KEYS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        blockers: Vec::new(),
        warnings,
        next_actions,
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_datasource_live_project_status, build_datasource_live_project_status_from_inputs,
        collect_live_datasource_project_status_inputs_with_request,
        datasource_live_project_status_org_count, datasource_live_project_status_source_kinds,
        DatasourceLiveProjectStatusInputs, LiveDatasourceProjectStatusInputs,
    };
    use crate::common::message;
    use serde_json::json;
    use serde_json::{Map, Value};

    fn live_datasource(
        id: i64,
        uid: &str,
        name: &str,
        datasource_type: &str,
        access: &str,
        is_default: bool,
        org_id: &str,
    ) -> Map<String, Value> {
        json!({
            "id": id,
            "uid": uid,
            "name": name,
            "type": datasource_type,
            "access": access,
            "isDefault": is_default,
            "orgId": org_id,
        })
        .as_object()
        .unwrap()
        .clone()
    }

    #[test]
    fn datasource_live_project_status_source_kinds_prefers_org_list_over_current_org() {
        let current_org = json!({"id": 1});
        let inputs = DatasourceLiveProjectStatusInputs {
            datasource_list: None,
            datasource_read: None,
            org_list: Some(&[]),
            current_org: Some(current_org.as_object().unwrap()),
        };

        assert_eq!(
            datasource_live_project_status_source_kinds(&inputs, "live-datasource-list"),
            vec![
                "live-datasource-list".to_string(),
                "live-org-list".to_string()
            ]
        );
    }

    #[test]
    fn datasource_live_project_status_org_count_prefers_explicit_org_surfaces() {
        let records = vec![
            live_datasource(
                1,
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "proxy",
                true,
                "1",
            ),
            live_datasource(2, "loki-main", "Loki Main", "loki", "proxy", false, "2"),
        ];
        let orgs = vec![json!({"id": 1}), json!({"id": 2}), json!({"id": 3})]
            .into_iter()
            .map(|value| value.as_object().unwrap().clone())
            .collect::<Vec<_>>();
        let current_org = json!({"id": 99});

        assert_eq!(
            datasource_live_project_status_org_count(
                &DatasourceLiveProjectStatusInputs {
                    datasource_list: Some(&records),
                    datasource_read: None,
                    org_list: Some(&orgs),
                    current_org: Some(current_org.as_object().unwrap()),
                },
                &records,
            ),
            3
        );

        assert_eq!(
            datasource_live_project_status_org_count(
                &DatasourceLiveProjectStatusInputs {
                    datasource_list: Some(&records),
                    datasource_read: None,
                    org_list: None,
                    current_org: Some(current_org.as_object().unwrap()),
                },
                &records,
            ),
            1
        );

        assert_eq!(
            datasource_live_project_status_org_count(
                &DatasourceLiveProjectStatusInputs {
                    datasource_list: Some(&records),
                    datasource_read: None,
                    org_list: None,
                    current_org: None,
                },
                &records,
            ),
            2
        );
    }

    #[test]
    fn collect_live_datasource_project_status_inputs_with_request_reads_inventory_and_org_surfaces()
    {
        let mut request = |method: reqwest::Method,
                           path: &str,
                           _params: &[(String, String)],
                           _payload: Option<&Value>| {
            match (method, path) {
                (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([
                    {"uid":"prom-main","name":"Prometheus Main","type":"prometheus","access":"proxy","orgId":"1","isDefault":true},
                    {"uid":"loki-main","name":"Loki Main","type":"loki","access":"proxy","orgId":"2","isDefault":false}
                ]))),
                (reqwest::Method::GET, "/api/orgs") => {
                    Ok(Some(json!([{"id":1},{"id":2},{"id":3}])))
                }
                (reqwest::Method::GET, "/api/org") => Ok(Some(json!({"id": 1, "name": "Main"}))),
                _ => Err(message(format!("unexpected request {path}"))),
            }
        };

        let inputs =
            collect_live_datasource_project_status_inputs_with_request(&mut request).unwrap();

        assert_eq!(inputs.datasource_list.len(), 2);
        assert_eq!(inputs.org_list.len(), 3);
        assert_eq!(
            inputs
                .current_org
                .as_ref()
                .and_then(|record| record.get("id"))
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn build_datasource_live_project_status_from_inputs_preserves_domain_surface() {
        let inputs = LiveDatasourceProjectStatusInputs {
            datasource_list: vec![
                live_datasource(
                    1,
                    "prom-main",
                    "Prometheus Main",
                    "prometheus",
                    "proxy",
                    true,
                    "1",
                ),
                live_datasource(2, "loki-main", "Loki Main", "loki", "proxy", false, "2"),
            ],
            org_list: vec![json!({"id": 1}).as_object().unwrap().clone()],
            current_org: Some(
                json!({"id": 1, "name": "Main"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        };

        let domain = build_datasource_live_project_status_from_inputs(&inputs).unwrap();

        assert_eq!(domain.id, "datasource");
        assert_eq!(domain.scope, "live");
        assert_eq!(domain.mode, "live-inventory");
    }

    #[test]
    fn build_datasource_live_project_status_tracks_live_list_and_org_fields() {
        let datasources = vec![
            live_datasource(
                1,
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "proxy",
                true,
                "1",
            ),
            live_datasource(2, "loki-main", "Loki Main", "loki", "proxy", false, "2"),
            live_datasource(3, "tempo-main", "Tempo Main", "tempo", "proxy", false, "2"),
        ];
        let orgs = vec![json!({"id": 1}), json!({"id": 2})]
            .into_iter()
            .map(|value| value.as_object().unwrap().clone())
            .collect::<Vec<_>>();

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&datasources),
            datasource_read: None,
            org_list: Some(&orgs),
            current_org: None,
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["id"], json!("datasource"));
        assert_eq!(domain["scope"], json!("live"));
        assert_eq!(domain["mode"], json!("live-inventory"));
        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["reasonCode"], json!("ready"));
        assert_eq!(domain["primaryCount"], json!(3));
        assert_eq!(domain["blockerCount"], json!(0));
        assert_eq!(domain["warningCount"], json!(0));
        assert_eq!(
            domain["sourceKinds"],
            json!(["live-datasource-list", "live-org-list"])
        );
        assert_eq!(
            domain["signalKeys"],
            json!([
                "live.datasourceCount",
                "live.defaultCount",
                "live.orgCount",
                "live.orgIdCount",
                "live.uidCount",
                "live.nameCount",
                "live.accessCount",
                "live.typeCount",
                "live.jsonDataCount",
                "live.basicAuthCount",
                "live.basicAuthPasswordCount",
                "live.passwordCount",
                "live.httpHeaderValueCount",
                "live.withCredentialsCount",
                "live.secureJsonFieldsCount",
                "live.tlsAuthCount",
                "live.tlsSkipVerifyCount",
                "live.serverNameCount",
                "live.readOnlyCount",
            ])
        );
        assert_eq!(domain["warnings"], json!([]));
        assert_eq!(domain["nextActions"], json!([]));
    }

    #[test]
    fn build_datasource_live_project_status_flags_missing_default_from_live_list() {
        let datasources = vec![
            live_datasource(
                1,
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "proxy",
                false,
                "1",
            ),
            live_datasource(2, "loki-main", "Loki Main", "loki", "proxy", false, "1"),
        ];

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&datasources),
            datasource_read: None,
            org_list: None,
            current_org: Some(json!({"id": 1, "name": "Main Org."}).as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["warningCount"], json!(1));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "missing-default",
                    "count": 1,
                    "source": "live.defaultCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!(["mark a default datasource in Grafana"])
        );
        assert_eq!(
            domain["sourceKinds"],
            json!(["live-datasource-list", "live-org-read"])
        );
    }

    #[test]
    fn build_datasource_live_project_status_surfaces_metadata_drift_from_live_payload_fields() {
        let mut datasources = vec![
            live_datasource(
                1,
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "proxy",
                true,
                "1",
            ),
            live_datasource(2, "loki-main", "Loki Main", "loki", "proxy", false, "2"),
        ];
        if let Some(second) = datasources.get_mut(1) {
            second.insert("uid".to_string(), Value::String(String::new()));
            second.insert("type".to_string(), Value::String(String::new()));
        }

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&datasources),
            datasource_read: None,
            org_list: None,
            current_org: Some(json!({"id": 1, "name": "Main Org."}).as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["warningCount"], json!(3));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "missing-uid",
                    "count": 1,
                    "source": "live.uidCount",
                },
                {
                    "kind": "missing-type",
                    "count": 1,
                    "source": "live.typeCount",
                },
                {
                    "kind": "mixed-org-ids",
                    "count": 1,
                    "source": "live.orgIdCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!([
                "re-run live datasource read after correcting datasource identity or org scope"
            ])
        );
    }

    #[test]
    fn build_datasource_live_project_status_surfaces_provider_config_readiness_from_json_data_fields(
    ) {
        let mut datasources = vec![
            live_datasource(
                1,
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "proxy",
                true,
                "1",
            ),
            live_datasource(2, "loki-main", "Loki Main", "loki", "proxy", false, "1"),
        ];
        if let Some(first) = datasources.get_mut(0) {
            first.insert(
                "jsonData".to_string(),
                Value::Object(
                    json!({
                        "httpMethod": "POST",
                        "tlsSkipVerify": false,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            );
        }

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&datasources),
            datasource_read: None,
            org_list: None,
            current_org: Some(json!({"id": 1, "name": "Main Org."}).as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["warningCount"], json!(1));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "provider-json-data-present",
                    "count": 1,
                    "source": "live.jsonDataCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!(["review live datasource secret and provider fields before export or import"])
        );
    }

    #[test]
    fn build_datasource_live_project_status_surfaces_secret_and_provider_readiness_from_live_payload_fields(
    ) {
        let mut datasources = vec![
            live_datasource(
                1,
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "proxy",
                true,
                "1",
            ),
            live_datasource(2, "loki-main", "Loki Main", "loki", "proxy", false, "1"),
        ];
        if let Some(first) = datasources.get_mut(0) {
            first.insert("basicAuth".to_string(), Value::Bool(true));
            first.insert("withCredentials".to_string(), Value::Bool(true));
            first.insert("readOnly".to_string(), Value::Bool(true));
            first.insert(
                "jsonData".to_string(),
                Value::Object(
                    json!({
                        "tlsAuth": true,
                        "serverName": "prom.example.internal",
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            );
            first.insert(
                "secureJsonFields".to_string(),
                Value::Object(
                    json!({
                        "basicAuthPassword": true,
                        "httpHeaderValue1": true,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            );
        }
        if let Some(second) = datasources.get_mut(1) {
            second.insert("readOnly".to_string(), Value::Bool(true));
            second.insert(
                "jsonData".to_string(),
                Value::Object(
                    json!({
                        "tlsSkipVerify": true,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            );
            second.insert(
                "secureJsonFields".to_string(),
                Value::Object(
                    json!({
                        "password": true,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            );
        }

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&datasources),
            datasource_read: None,
            org_list: None,
            current_org: Some(json!({"id": 1, "name": "Main Org."}).as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["warningCount"], json!(14));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "provider-json-data-present",
                    "count": 2,
                    "source": "live.jsonDataCount",
                },
                {
                    "kind": "basic-auth-configured",
                    "count": 1,
                    "source": "live.basicAuthCount",
                },
                {
                    "kind": "basic-auth-password-present",
                    "count": 1,
                    "source": "live.basicAuthPasswordCount",
                },
                {
                    "kind": "datasource-password-present",
                    "count": 1,
                    "source": "live.passwordCount",
                },
                {
                    "kind": "http-header-secret-values-present",
                    "count": 1,
                    "source": "live.httpHeaderValueCount",
                },
                {
                    "kind": "with-credentials-configured",
                    "count": 1,
                    "source": "live.withCredentialsCount",
                },
                {
                    "kind": "secure-json-fields-present",
                    "count": 2,
                    "source": "live.secureJsonFieldsCount",
                },
                {
                    "kind": "tls-auth-configured",
                    "count": 1,
                    "source": "live.tlsAuthCount",
                },
                {
                    "kind": "tls-skip-verify-configured",
                    "count": 1,
                    "source": "live.tlsSkipVerifyCount",
                },
                {
                    "kind": "server-name-configured",
                    "count": 1,
                    "source": "live.serverNameCount",
                },
                {
                    "kind": "read-only",
                    "count": 2,
                    "source": "live.readOnlyCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!(["review live datasource secret and provider fields before export or import"])
        );
        assert_eq!(
            domain["signalKeys"],
            json!([
                "live.datasourceCount",
                "live.defaultCount",
                "live.orgCount",
                "live.orgIdCount",
                "live.uidCount",
                "live.nameCount",
                "live.accessCount",
                "live.typeCount",
                "live.jsonDataCount",
                "live.basicAuthCount",
                "live.basicAuthPasswordCount",
                "live.passwordCount",
                "live.httpHeaderValueCount",
                "live.withCredentialsCount",
                "live.secureJsonFieldsCount",
                "live.tlsAuthCount",
                "live.tlsSkipVerifyCount",
                "live.serverNameCount",
                "live.readOnlyCount",
            ])
        );
    }

    #[test]
    fn build_datasource_live_project_status_flags_missing_name_and_access() {
        let mut datasources = vec![
            live_datasource(
                1,
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "proxy",
                true,
                "1",
            ),
            live_datasource(2, "loki-main", "Loki Main", "loki", "proxy", false, "1"),
        ];
        if let Some(second) = datasources.get_mut(1) {
            second.insert("name".to_string(), Value::String(String::new()));
            second.insert("access".to_string(), Value::String(String::new()));
        }

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&datasources),
            datasource_read: None,
            org_list: None,
            current_org: Some(json!({"id": 1, "name": "Main Org."}).as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["warningCount"], json!(2));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "missing-name",
                    "count": 1,
                    "source": "live.nameCount",
                },
                {
                    "kind": "missing-access",
                    "count": 1,
                    "source": "live.accessCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!([
                "re-run live datasource read after correcting datasource identity or org scope"
            ])
        );
    }

    #[test]
    fn build_datasource_live_project_status_flags_org_scope_and_org_list_mismatch() {
        let datasources = vec![live_datasource(
            1,
            "prom-main",
            "Prometheus Main",
            "prometheus",
            "proxy",
            true,
            "2",
        )];
        let orgs = vec![json!({"id": 1})]
            .into_iter()
            .map(|value| value.as_object().unwrap().clone())
            .collect::<Vec<_>>();

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&datasources),
            datasource_read: None,
            org_list: Some(&orgs),
            current_org: Some(json!({"id": 1, "name": "Main Org."}).as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["warningCount"], json!(2));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "org-scope-mismatch",
                    "count": 1,
                    "source": "live.orgIdCount",
                },
                {
                    "kind": "org-list-mismatch",
                    "count": 1,
                    "source": "live.orgCount",
                }
            ])
        );
        assert_eq!(
            domain["sourceKinds"],
            json!(["live-datasource-list", "live-org-list"])
        );
        assert_eq!(
            domain["nextActions"],
            json!([
                "re-run live datasource read after correcting datasource identity or org scope"
            ])
        );
    }

    #[test]
    fn build_datasource_live_project_status_flags_duplicate_live_uids() {
        let datasources = vec![
            live_datasource(
                1,
                "shared",
                "Prometheus Main",
                "prometheus",
                "proxy",
                true,
                "1",
            ),
            live_datasource(2, "shared", "Loki Main", "loki", "proxy", false, "1"),
        ];

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&datasources),
            datasource_read: None,
            org_list: None,
            current_org: Some(json!({"id": 1, "name": "Main Org."}).as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["warningCount"], json!(1));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "duplicate-uid",
                    "count": 1,
                    "source": "live.uidCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!([
                "re-run live datasource read after correcting datasource identity or org scope"
            ])
        );
    }

    #[test]
    fn build_datasource_live_project_status_falls_back_to_read_surface() {
        let datasource = live_datasource(
            7,
            "prom-main",
            "Prometheus Main",
            "prometheus",
            "proxy",
            true,
            "1",
        );
        let org = json!({"id": 1, "name": "Main Org."});

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: None,
            datasource_read: Some(&datasource),
            org_list: None,
            current_org: Some(org.as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["primaryCount"], json!(1));
        assert_eq!(
            domain["sourceKinds"],
            json!(["live-datasource-read", "live-org-read"])
        );
        assert_eq!(domain["warningCount"], json!(0));
        assert_eq!(domain["nextActions"], json!([]));
    }

    #[test]
    fn build_datasource_live_project_status_is_partial_without_datasources() {
        let org = json!({"id": 1, "name": "Main Org."});

        let domain = build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
            datasource_list: Some(&[]),
            datasource_read: None,
            org_list: None,
            current_org: Some(org.as_object().unwrap()),
        })
        .unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("partial"));
        assert_eq!(domain["reasonCode"], json!("partial-no-data"));
        assert_eq!(
            domain["nextActions"],
            json!(["create or sync at least one datasource in Grafana"])
        );
    }
}
