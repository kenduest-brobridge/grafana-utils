//! Shared live reads for the project-status workflow.
//!
//! Keep this module workflow-level: it should gather the live documents needed
//! by project-status without turning into a generic endpoint SDK.

use crate::common::{message, value_as_object, Result};
use crate::grafana_api::{AccessResourceClient, AlertingResourceClient, DashboardResourceClient};
use crate::http::JsonHttpClient;
use reqwest::Method;
use serde_json::{Map, Value};

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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub(crate) fn latest_dashboard_version_timestamp(
    client: &JsonHttpClient,
    dashboard_summaries: &[Map<String, Value>],
) -> Option<String> {
    DashboardResourceClient::new(client).latest_dashboard_version_timestamp(dashboard_summaries)
}

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

#[allow(dead_code)]
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
        fetch_current_org, fetch_current_org_with_request, latest_dashboard_version_timestamp,
        latest_dashboard_version_timestamp_with_request, list_visible_orgs,
        list_visible_orgs_with_request, load_alert_surface_documents,
        load_alert_surface_documents_with_request,
    };
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
}
