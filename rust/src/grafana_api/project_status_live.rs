//! Shared live reads for the project-status workflow.
//!
//! Keep this module workflow-level: it should gather the live documents needed
//! by project-status without turning into a generic endpoint SDK.

use crate::common::{message, value_as_object, Result};
use crate::dashboard::LiveDashboardProjectStatusInputs;
use crate::grafana_api::{
    AccessResourceClient, AlertingResourceClient, DashboardResourceClient,
    DatasourceResourceClient,
};
use crate::http::JsonHttpClient;
use crate::project_status_freshness::{ProjectStatusFreshnessSample};
use reqwest::Method;
use serde_json::{Map, Value};

#[cfg(test)]
use crate::dashboard::build_live_dashboard_domain_status_from_inputs;
#[cfg(test)]
use crate::project_status::{ProjectDomainStatus, PROJECT_STATUS_PARTIAL};
#[cfg(test)]
use crate::project_status_freshness::{
    build_live_project_status_freshness_from_samples,
    build_live_project_status_freshness_from_source_count,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProjectStatusAlertSurfaceDocuments {
    pub(crate) rules: Option<Value>,
    pub(crate) contact_points: Option<Value>,
    pub(crate) mute_timings: Option<Value>,
    pub(crate) policies: Option<Value>,
    pub(crate) templates: Option<Value>,
}

fn request_json_best_effort_with_request<F>(
    request_json: &mut F,
    path: &str,
    params: &[(String, String)],
) -> Option<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, path, params, None).ok().flatten() {
        Some(Value::Null) => None,
        other => other,
    }
}

fn request_object_list_with_request<F>(
    request_json: &mut F,
    path: &str,
    params: &[(String, String)],
    error_message: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, path, params, None)? {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| Ok(value_as_object(item, error_message)?.clone()))
            .collect(),
        Some(_) => Err(message(error_message)),
        None => Ok(Vec::new()),
    }
}

pub(crate) fn project_status_timestamp_from_object(object: &Map<String, Value>) -> Option<&str> {
    for key in ["updated", "updatedAt", "modified", "createdAt", "created"] {
        if let Some(observed_at) = object.get(key).and_then(Value::as_str) {
            let observed_at = observed_at.trim();
            if !observed_at.is_empty() {
                return Some(observed_at);
            }
        }
    }
    None
}

pub(crate) fn project_status_freshness_samples_from_value<'a>(
    source: &'static str,
    value: &'a Value,
) -> Vec<ProjectStatusFreshnessSample<'a>> {
    match value {
        Value::Array(items) => items
            .iter()
            .flat_map(|item| project_status_freshness_samples_from_value(source, item))
            .collect(),
        Value::Object(object) => project_status_timestamp_from_object(object)
            .map(|observed_at| {
                vec![ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                    source,
                    observed_at,
                }]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub(crate) fn project_status_freshness_samples_from_records<'a>(
    source: &'static str,
    records: &'a [Map<String, Value>],
) -> Vec<ProjectStatusFreshnessSample<'a>> {
    records
        .iter()
        .filter_map(|record| {
            project_status_timestamp_from_object(record).map(|observed_at| {
                ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                    source,
                    observed_at,
                }
            })
        })
        .collect()
}

#[cfg(test)]
fn stamp_live_domain_freshness(
    mut domain: ProjectDomainStatus,
    samples: &[ProjectStatusFreshnessSample<'_>],
) -> ProjectDomainStatus {
    domain.freshness = if samples.is_empty() {
        build_live_project_status_freshness_from_source_count(domain.source_kinds.len())
    } else {
        build_live_project_status_freshness_from_samples(samples)
    };
    domain
}

#[cfg(test)]
pub(crate) fn build_live_dashboard_status_with_request<F>(mut request_json: F) -> ProjectDomainStatus
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match collect_live_dashboard_project_status_inputs_with_request(&mut request_json, 500) {
        Ok(inputs) => {
            let status = build_live_dashboard_domain_status_from_inputs(&inputs);
            let mut freshness_samples = project_status_freshness_samples_from_records(
                "dashboard-search",
                &inputs.dashboard_summaries,
            );
            freshness_samples.extend(project_status_freshness_samples_from_records(
                "datasource-list",
                &inputs.datasources,
            ));
            let dashboard_version_timestamp = if freshness_samples.is_empty() {
                latest_dashboard_version_timestamp_with_request(
                    &mut request_json,
                    &inputs.dashboard_summaries,
                )
            } else {
                None
            };
            if let Some(observed_at) = dashboard_version_timestamp.as_deref() {
                freshness_samples.push(ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                    source: "dashboard-version-history",
                    observed_at,
                });
            }
            stamp_live_domain_freshness(status, &freshness_samples)
        }
        Err(_) => ProjectDomainStatus {
            id: "dashboard".to_string(),
            scope: "live".to_string(),
            mode: "live-dashboard-read".to_string(),
            status: PROJECT_STATUS_PARTIAL.to_string(),
            reason_code: "live-read-failed".to_string(),
            primary_count: 0,
            blocker_count: 1,
            warning_count: 0,
            source_kinds: vec!["live-dashboard-search".to_string()],
            signal_keys: vec!["live.dashboardCount".to_string()],
            blockers: vec![crate::project_status::status_finding(
                "live-read-failed",
                1,
                "live.dashboardCount",
            )],
            warnings: Vec::new(),
            next_actions: vec![
                "restore dashboard search access, then re-run live status".to_string(),
            ],
            freshness: Default::default(),
        },
    }
}

#[cfg(test)]
fn first_dashboard_uid(dashboard_summaries: &[Map<String, Value>]) -> Option<&str> {
    dashboard_summaries.iter().find_map(|summary| {
        summary
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

pub(crate) fn list_visible_orgs(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    AccessResourceClient::new(client).list_orgs()
}

pub(crate) fn collect_live_dashboard_project_status_inputs(
    client: &JsonHttpClient,
    page_size: usize,
) -> Result<LiveDashboardProjectStatusInputs> {
    let dashboard_client = DashboardResourceClient::new(client);
    let datasource_client = DatasourceResourceClient::new(client);
    Ok(LiveDashboardProjectStatusInputs {
        dashboard_summaries: dashboard_client.list_dashboard_summaries(page_size)?,
        datasources: datasource_client.list_datasources()?,
    })
}

pub(crate) fn collect_live_dashboard_project_status_inputs_with_request<F>(
    request_json: &mut F,
    page_size: usize,
) -> Result<LiveDashboardProjectStatusInputs>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut dashboard_summaries = Vec::new();
    let mut seen_uids = std::collections::BTreeSet::new();
    let mut page = 1usize;
    loop {
        let params = vec![
            ("type".to_string(), "dash-db".to_string()),
            ("limit".to_string(), page_size.to_string()),
            ("page".to_string(), page.to_string()),
        ];
        let response = request_json(Method::GET, "/api/search", &params, None)?;
        let batch = match response {
            Some(Value::Array(batch)) => batch,
            Some(_) => return Err(message("Unexpected search response from Grafana.")),
            None => Vec::new(),
        };
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        for item in batch {
            let object = value_as_object(&item, "Unexpected dashboard summary payload from Grafana.")?;
            let uid = object.get("uid").and_then(Value::as_str).map(str::trim).unwrap_or("");
            if uid.is_empty() || seen_uids.contains(uid) {
                continue;
            }
            seen_uids.insert(uid.to_string());
            dashboard_summaries.push(object.clone());
        }
        if batch_len < page_size {
            break;
        }
        page += 1;
    }

    let datasources = match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                Ok(value_as_object(item, "Unexpected datasource payload from Grafana.")?.clone())
            })
            .collect::<Result<Vec<Map<String, Value>>>>()?,
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => Vec::new(),
    };

    Ok(LiveDashboardProjectStatusInputs {
        dashboard_summaries,
        datasources,
    })
}

pub(crate) fn list_visible_orgs_with_request<F>(request_json: &mut F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object_list_with_request(
        request_json,
        "/api/orgs",
        &[],
        "Unexpected /api/orgs payload from Grafana.",
    )
}

#[cfg(test)]
pub(crate) fn fetch_current_org(client: &JsonHttpClient) -> Result<Map<String, Value>> {
    AccessResourceClient::new(client).fetch_current_org()
}

pub(crate) fn fetch_current_org_with_request<F>(request_json: &mut F) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(Method::GET, "/api/org", &[], None)? {
        Some(value) => {
            let object =
                value_as_object(&value, "Unexpected current-org payload from Grafana.")?;
            Ok(object.clone())
        }
        None => Err(message("Grafana did not return current-org metadata.")),
    }
}

pub(crate) fn latest_dashboard_version_timestamp(
    client: &JsonHttpClient,
    dashboard_summaries: &[Map<String, Value>],
) -> Option<String> {
    DashboardResourceClient::new(client).latest_dashboard_version_timestamp(dashboard_summaries)
}

#[cfg(test)]
pub(crate) fn latest_dashboard_version_timestamp_with_request<F>(
    mut request_json: F,
    dashboard_summaries: &[Map<String, Value>],
) -> Option<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let uid = first_dashboard_uid(dashboard_summaries)?;
    let path = format!("/api/dashboards/uid/{uid}/versions");
    let params = vec![("limit".to_string(), "1".to_string())];
    let response = request_json(Method::GET, &path, &params, None)
        .ok()
        .flatten()?;
    let versions = match response {
        Value::Array(items) => items,
        Value::Object(object) => object
            .get("versions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    versions
        .first()
        .and_then(Value::as_object)
        .and_then(project_status_timestamp_from_object)
        .map(str::to_string)
}

pub(crate) fn load_alert_surface_documents(
    client: &JsonHttpClient,
) -> ProjectStatusAlertSurfaceDocuments {
    let alerting = AlertingResourceClient::new(client);
    ProjectStatusAlertSurfaceDocuments {
        rules: request_json_best_effort_with_request(
            &mut |method, path, params, payload| alerting.request_json(method, path, params, payload),
            "/api/v1/provisioning/alert-rules",
            &[],
        ),
        contact_points: request_json_best_effort_with_request(
            &mut |method, path, params, payload| alerting.request_json(method, path, params, payload),
            "/api/v1/provisioning/contact-points",
            &[],
        ),
        mute_timings: request_json_best_effort_with_request(
            &mut |method, path, params, payload| alerting.request_json(method, path, params, payload),
            "/api/v1/provisioning/mute-timings",
            &[],
        ),
        policies: request_json_best_effort_with_request(
            &mut |method, path, params, payload| alerting.request_json(method, path, params, payload),
            "/api/v1/provisioning/policies",
            &[],
        ),
        templates: request_json_best_effort_with_request(
            &mut |method, path, params, payload| alerting.request_json(method, path, params, payload),
            "/api/v1/provisioning/templates",
            &[],
        ),
    }
}

#[cfg(test)]
pub(crate) fn load_alert_surface_documents_with_request<F>(
    request_json: &mut F,
) -> ProjectStatusAlertSurfaceDocuments
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    ProjectStatusAlertSurfaceDocuments {
        rules: request_json_best_effort_with_request(
            request_json,
            "/api/v1/provisioning/alert-rules",
            &[],
        ),
        contact_points: request_json_best_effort_with_request(
            request_json,
            "/api/v1/provisioning/contact-points",
            &[],
        ),
        mute_timings: request_json_best_effort_with_request(
            request_json,
            "/api/v1/provisioning/mute-timings",
            &[],
        ),
        policies: request_json_best_effort_with_request(
            request_json,
            "/api/v1/provisioning/policies",
            &[],
        ),
        templates: request_json_best_effort_with_request(
            request_json,
            "/api/v1/provisioning/templates",
            &[],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        collect_live_dashboard_project_status_inputs,
        collect_live_dashboard_project_status_inputs_with_request, fetch_current_org,
        fetch_current_org_with_request, latest_dashboard_version_timestamp,
        latest_dashboard_version_timestamp_with_request, list_visible_orgs,
        list_visible_orgs_with_request, load_alert_surface_documents,
        load_alert_surface_documents_with_request,
    };
    use crate::dashboard::DEFAULT_PAGE_SIZE;
    use crate::http::{JsonHttpClient, JsonHttpClientConfig};
    use reqwest::Method;
    use serde_json::{json, Value};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn build_test_client(
        responses: Vec<String>,
    ) -> (
        JsonHttpClient,
        Arc<Mutex<Vec<String>>>,
        thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_thread = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                loop {
                    let bytes_read = stream.read(&mut buffer).unwrap();
                    if bytes_read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..bytes_read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let request_line = String::from_utf8_lossy(&request)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                requests_thread.lock().unwrap().push(request_line);
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        let client = JsonHttpClient::new(JsonHttpClientConfig {
            base_url: format!("http://{addr}"),
            headers: vec![("Authorization".to_string(), "Bearer token".to_string())],
            timeout_secs: 5,
            verify_ssl: false,
        })
        .unwrap();
        (client, requests, handle)
    }

    fn http_response(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn list_visible_orgs_parses_orgs() {
        let responses = vec![http_response(
            "200 OK",
            r#"[{"id":1,"name":"Main"},{"id":2,"name":"Edge"}]"#,
        )];
        let (client, requests, handle) = build_test_client(responses);
        let orgs = list_visible_orgs(&client).unwrap();
        handle.join().unwrap();

        assert_eq!(orgs.len(), 2);
        assert_eq!(requests.lock().unwrap()[0], "GET /api/orgs HTTP/1.1");
    }

    #[test]
    fn list_visible_orgs_with_request_parses_orgs() {
        let orgs = list_visible_orgs_with_request(&mut |method, path, params, _payload| {
            assert_eq!(method, Method::GET);
            assert_eq!(path, "/api/orgs");
            assert!(params.is_empty());
            Ok(Some(json!([{"id":1,"name":"Main"},{"id":2,"name":"Edge"}])))
        })
        .unwrap();

        assert_eq!(orgs.len(), 2);
    }

    #[test]
    fn fetch_current_org_with_request_parses_org() {
        let org = fetch_current_org_with_request(&mut |method, path, params, _payload| {
            assert_eq!(method, Method::GET);
            assert_eq!(path, "/api/org");
            assert!(params.is_empty());
            Ok(Some(json!({"id":1,"name":"Main"})))
        })
        .unwrap();

        assert_eq!(org.get("name").and_then(Value::as_str), Some("Main"));
    }

    #[test]
    fn fetch_current_org_parses_org() {
        let responses = vec![http_response("200 OK", r#"{"id":1,"name":"Main"}"#)];
        let (client, requests, handle) = build_test_client(responses);
        let org = fetch_current_org(&client).unwrap();
        handle.join().unwrap();

        assert_eq!(org.get("id").and_then(Value::as_i64), Some(1));
        assert_eq!(requests.lock().unwrap()[0], "GET /api/org HTTP/1.1");
    }

    #[test]
    fn latest_dashboard_version_timestamp_uses_first_summary_uid() {
        let responses = vec![http_response(
            "200 OK",
            r#"[{"version":7,"created":"2026-01-01T00:00:00Z"}]"#,
        )];
        let (client, requests, handle) = build_test_client(responses);
        let timestamp = latest_dashboard_version_timestamp(
            &client,
            &[json!({"uid":"cpu-main","title":"CPU"})
                .as_object()
                .unwrap()
                .clone()],
        );
        handle.join().unwrap();

        assert!(timestamp.is_some());
        assert_eq!(
            requests.lock().unwrap()[0],
            "GET /api/dashboards/uid/cpu-main/versions?limit=1 HTTP/1.1"
        );
    }

    #[test]
    fn collect_live_dashboard_project_status_inputs_reads_dashboard_and_datasource_surfaces() {
        let responses = vec![
            http_response(
                "200 OK",
                r#"[{"uid":"cpu-main","title":"CPU Main","folderUid":"infra","folderTitle":"Infra"}]"#,
            ),
            http_response(
                "200 OK",
                r#"[{"uid":"prom-main","name":"Prometheus Main","type":"prometheus"}]"#,
            ),
        ];
        let (client, requests, handle) = build_test_client(responses);
        let inputs =
            collect_live_dashboard_project_status_inputs(&client, DEFAULT_PAGE_SIZE).unwrap();
        handle.join().unwrap();

        assert_eq!(inputs.dashboard_summaries.len(), 1);
        assert_eq!(inputs.datasources.len(), 1);
        assert_eq!(requests.lock().unwrap()[0], "GET /api/search?type=dash-db&limit=500&page=1 HTTP/1.1");
        assert_eq!(requests.lock().unwrap()[1], "GET /api/datasources HTTP/1.1");
    }

    #[test]
    fn latest_dashboard_version_timestamp_with_request_uses_first_summary_uid() {
        let timestamp = latest_dashboard_version_timestamp_with_request(
            |method, path, params, _payload| {
                assert_eq!(method, Method::GET);
                assert_eq!(path, "/api/dashboards/uid/cpu-main/versions");
                assert_eq!(params, &vec![("limit".to_string(), "1".to_string())]);
                Ok(Some(
                    json!([{"version": 7, "created": "2026-01-01T00:00:00Z"}]),
                ))
            },
            &[json!({"uid":"cpu-main","title":"CPU"})
                .as_object()
                .unwrap()
                .clone()],
        );

        assert_eq!(timestamp.as_deref(), Some("2026-01-01T00:00:00Z"));
    }

    #[test]
    fn load_alert_surface_documents_tolerates_null_templates() {
        let responses = vec![
            http_response("200 OK", "[]"),
            http_response("200 OK", "[]"),
            http_response("200 OK", "[]"),
            http_response("200 OK", "{}"),
            http_response("200 OK", "null"),
        ];
        let (client, requests, handle) = build_test_client(responses);
        let docs = load_alert_surface_documents(&client);
        handle.join().unwrap();

        assert!(docs.templates.is_none());
        assert_eq!(requests.lock().unwrap().len(), 5);
    }

    #[test]
    fn load_alert_surface_documents_with_request_tolerates_null_templates() {
        let docs = load_alert_surface_documents_with_request(&mut |method, path, params, _payload| {
            assert_eq!(method, Method::GET);
            assert!(params.is_empty());
            match path {
                "/api/v1/provisioning/alert-rules"
                | "/api/v1/provisioning/contact-points"
                | "/api/v1/provisioning/mute-timings" => Ok(Some(json!([]))),
                "/api/v1/provisioning/policies" => Ok(Some(json!({}))),
                "/api/v1/provisioning/templates" => Ok(Some(Value::Null)),
                _ => Err(crate::common::message(format!("unexpected request {path}"))),
            }
        });

        assert!(docs.templates.is_none());
    }

    #[test]
    fn collect_live_dashboard_project_status_inputs_with_request_reads_dashboard_and_datasource_surfaces(
    ) {
        let inputs = collect_live_dashboard_project_status_inputs_with_request(
            &mut |method, path, params, _payload| {
                assert_eq!(method, Method::GET);
                match path {
                    "/api/search" => {
                        let page = params
                            .iter()
                            .find(|(key, _)| key == "page")
                            .map(|(_, value)| value.as_str())
                            .unwrap_or("1");
                        if page == "1" {
                            Ok(Some(json!([
                                {"uid":"cpu-main","title":"CPU","folderUid":"infra","folderTitle":"Infra"},
                                {"uid":"cpu-main","title":"CPU","folderUid":"infra","folderTitle":"Infra"},
                                {"uid":"logs-main","title":"Logs","folderUid":"platform","folderTitle":"Platform"}
                            ])))
                        } else {
                            Ok(Some(json!([])))
                        }
                    }
                    "/api/datasources" => Ok(Some(json!([
                        {"uid":"prom-main","name":"Prometheus"},
                        {"uid":"loki-main","name":"Loki"}
                    ]))),
                    _ => Err(crate::common::message(format!("unexpected request {path}"))),
                }
            },
            2,
        )
        .unwrap();

        assert_eq!(inputs.dashboard_summaries.len(), 2);
        assert_eq!(inputs.datasources.len(), 2);
        assert_eq!(
            inputs
                .dashboard_summaries
                .first()
                .and_then(|summary| summary.get("uid"))
                .and_then(Value::as_str),
            Some("cpu-main")
        );
        assert_eq!(
            inputs
                .datasources
                .first()
                .and_then(|datasource| datasource.get("uid"))
                .and_then(Value::as_str),
            Some("prom-main")
        );
    }
}
