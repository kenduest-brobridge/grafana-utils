use serde_json::{Map, Value};
use std::collections::BTreeSet;

use crate::common::string_field;
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

pub(crate) const DATASOURCE_SIGNAL_KEYS: &[&str] = &[
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
