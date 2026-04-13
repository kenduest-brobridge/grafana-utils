//! Live datasource domain-status producer.
//!
//! Maintainer note:
//! - This module now acts as the thin orchestration wrapper around the
//!   datasource live-status analysis helpers.
//! - Keep it conservative and source-attributable: prefer the live list
//!   response, fall back to a single live read response when needed, and only
//!   derive counts that are directly visible in the payloads.

use serde_json::Value;

use crate::common::Result;
use crate::grafana_api::datasource_live_project_status as datasource_live_project_status_support;

#[path = "live_analysis.rs"]
mod analysis;

#[allow(unused_imports)]
pub(crate) use analysis::{
    build_datasource_live_project_status, build_datasource_live_project_status_from_inputs,
    datasource_live_project_status_org_count, datasource_live_project_status_source_kinds,
    DatasourceLiveProjectStatusInputs, LiveDatasourceProjectStatusInputs,
};

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
                "re-run live datasource read after aligning datasource org scope with the current org and visible org list"
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
                "re-run live datasource read after aligning datasource org scope with the current org and visible org list"
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
