use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::grafana_api::connection::auth_mode_from_headers;
use crate::grafana_api::{AccessResourceClient, AuthInputs, GrafanaApiClient, GrafanaConnection};
use crate::profile_config::ConnectionMergeInput;
use serde_json::json;

fn http_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn spawn_sequence_server(
    responses: Vec<String>,
) -> (String, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(false).unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let requests_thread = Arc::clone(&requests);
    let handle = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();

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
            let mut request_text = request_line;
            request_text.push('\n');
            request_text.push_str(&String::from_utf8_lossy(&request));
            requests_thread.lock().unwrap().push(request_text);

            stream.write_all(response.as_bytes()).unwrap();
            let _ = stream.flush();
        }
    });
    (format!("http://{address}"), requests, handle)
}

fn build_test_api(base_url: String) -> GrafanaApiClient {
    GrafanaApiClient::from_connection(GrafanaConnection::new(
        base_url,
        vec![("Authorization".to_string(), "Bearer token".to_string())],
        5,
        false,
        None,
        "token".to_string(),
    ))
    .unwrap()
}

#[test]
fn auth_mode_reports_basic_and_token_headers() {
    assert_eq!(
        auth_mode_from_headers(&[("Authorization".to_string(), "Basic abc".to_string())]),
        "basic"
    );
    assert_eq!(
        auth_mode_from_headers(&[("Authorization".to_string(), "Bearer abc".to_string())]),
        "token"
    );
}

#[test]
fn grafana_connection_resolve_adds_org_scope_when_requested() {
    let connection = GrafanaConnection::resolve(
        None,
        ConnectionMergeInput {
            url: "http://localhost:3000",
            url_default: "http://localhost:3000",
            api_token: Some("token-123"),
            username: None,
            password: None,
            org_id: Some(7),
            timeout: 30,
            timeout_default: 30,
            verify_ssl: false,
            insecure: false,
            ca_cert: None::<&Path>,
        },
        AuthInputs {
            api_token: Some("token-123"),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
        },
        true,
    )
    .unwrap();

    assert_eq!(connection.auth_mode, "token");
    assert!(connection
        .headers
        .iter()
        .any(|(name, value)| { name == "X-Grafana-Org-Id" && value == "7" }));
}

#[test]
fn grafana_connection_with_org_id_replaces_existing_header() {
    let connection = GrafanaConnection::new(
        "http://localhost:3000".to_string(),
        vec![
            ("Authorization".to_string(), "Bearer token".to_string()),
            ("X-Grafana-Org-Id".to_string(), "7".to_string()),
        ],
        30,
        false,
        None,
        "token".to_string(),
    );

    let scoped = connection.with_org_id(9);

    let org_headers = scoped
        .headers
        .iter()
        .filter(|(name, _)| name == "X-Grafana-Org-Id")
        .collect::<Vec<_>>();
    assert_eq!(org_headers.len(), 1);
    assert_eq!(org_headers[0].1, "9");
}

#[test]
fn grafana_api_client_scoped_to_org_reuses_existing_auth_headers() {
    let api = GrafanaApiClient::from_connection(GrafanaConnection::new(
        "http://localhost:3000".to_string(),
        vec![("Authorization".to_string(), "Basic abc".to_string())],
        30,
        false,
        None,
        "basic".to_string(),
    ))
    .unwrap();

    let scoped = api.scoped_to_org(12).unwrap();
    let headers = &scoped.connection().headers;

    assert!(headers
        .iter()
        .any(|(name, value)| name == "Authorization" && value == "Basic abc"));
    let org_headers = headers
        .iter()
        .filter(|(name, _)| name == "X-Grafana-Org-Id")
        .collect::<Vec<_>>();
    assert_eq!(org_headers.len(), 1);
    assert_eq!(org_headers[0].1, "12");
}

#[test]
fn dashboard_resource_client_lists_orgs_and_current_org() {
    let responses = vec![
        http_response("200 OK", r#"{"id":7,"name":"Main Org."}"#),
        http_response(
            "200 OK",
            r#"[{"id":1,"name":"Alpha"},{"id":2,"name":"Beta"}]"#,
        ),
    ];
    let (base_url, requests, handle) = spawn_sequence_server(responses);
    let api = build_test_api(base_url);
    let dashboard = api.dashboard();

    let current_org = dashboard.fetch_current_org().unwrap();
    let orgs = dashboard.list_orgs().unwrap();

    handle.join().unwrap();

    assert_eq!(current_org["id"], 7);
    assert_eq!(current_org["name"], "Main Org.");
    assert_eq!(orgs.len(), 2);
    assert_eq!(orgs[0]["name"], "Alpha");
    assert_eq!(orgs[1]["name"], "Beta");

    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("GET /api/org "));
    assert!(requests[1].starts_with("GET /api/orgs "));
}

#[test]
fn access_resource_client_lists_orgs_and_current_org() {
    let responses = vec![
        http_response("200 OK", r#"{"id":7,"name":"Main Org."}"#),
        http_response(
            "200 OK",
            r#"[{"id":1,"name":"Alpha"},{"id":2,"name":"Beta"}]"#,
        ),
    ];
    let (base_url, requests, handle) = spawn_sequence_server(responses);
    let api = build_test_api(base_url);
    let access = AccessResourceClient::new(api.http_client());

    let current_org = access.fetch_current_org().unwrap();
    let orgs = access.list_orgs().unwrap();

    handle.join().unwrap();

    assert_eq!(current_org["id"], 7);
    assert_eq!(current_org["name"], "Main Org.");
    assert_eq!(orgs.len(), 2);
    assert_eq!(orgs[0]["name"], "Alpha");
    assert_eq!(orgs[1]["name"], "Beta");

    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("GET /api/org "));
    assert!(requests[1].starts_with("GET /api/orgs "));
}

#[test]
fn dashboard_resource_client_builds_expected_dashboard_requests() {
    let responses = vec![
        http_response(
            "200 OK",
            r#"[{"uid":"cpu-main","title":"CPU"},{"uid":"mem-main","title":"Memory"}]"#,
        ),
        http_response("200 OK", r#"[{"uid":"disk-main","title":"Disk"}]"#),
        http_response(
            "200 OK",
            r#"[{"uid":"search-a","title":"Search A"},{"uid":"search-b","title":"Search B"}]"#,
        ),
        http_response("200 OK", r#"{"uid":"infra","title":"Infra"}"#),
        http_response("200 OK", r#"{"dashboard":{"uid":"cpu-main"}}"#),
        http_response("200 OK", r#"{"id":1,"name":"Main Org."}"#),
        http_response(
            "200 OK",
            r#"[{"id":1,"name":"Main Org."},{"id":2,"name":"Platform"}]"#,
        ),
        http_response("200 OK", r#"[{"role":"Admin"}]"#),
        http_response("200 OK", r#"[{"role":"Viewer"}]"#),
        http_response("200 OK", r#"{"status":"success"}"#),
        http_response("200 OK", r#"{"status":"success"}"#),
        http_response("200 OK", r#"{"status":"success"}"#),
        http_response("200 OK", r#"{"uid":"platform","title":"Platform"}"#),
        http_response("200 OK", r#"[{"uid":"ds-main"}]"#),
    ];
    let (base_url, requests, handle) = spawn_sequence_server(responses);
    let api = build_test_api(base_url);
    let dashboard = api.dashboard();

    let summaries = dashboard.list_dashboard_summaries(2).unwrap();
    let search = dashboard.search_dashboards("cpu").unwrap();
    let folder = dashboard.fetch_folder_if_exists("infra").unwrap();
    let payload = dashboard.fetch_dashboard("cpu-main").unwrap();
    let current_org = dashboard.fetch_current_org().unwrap();
    let orgs = dashboard.list_orgs().unwrap();
    let dashboard_permissions = dashboard.fetch_dashboard_permissions("cpu-main").unwrap();
    let folder_permissions = dashboard.fetch_folder_permissions("infra").unwrap();
    let import = dashboard
        .import_dashboard_request(&json!({"dashboard":{"uid":"cpu-main"}}))
        .unwrap();
    let deleted_dashboard = dashboard.delete_dashboard_request("cpu-main").unwrap();
    let deleted_folder = dashboard.delete_folder_request("infra").unwrap();
    let created_folder = dashboard
        .create_folder_entry("Platform", "platform", Some("ops"))
        .unwrap();
    let datasources = dashboard.list_datasources().unwrap();

    handle.join().unwrap();

    assert_eq!(summaries.len(), 3);
    assert_eq!(summaries[0]["uid"], "cpu-main");
    assert_eq!(summaries[1]["uid"], "mem-main");
    assert_eq!(summaries[2]["uid"], "disk-main");
    assert_eq!(search.len(), 2);
    assert_eq!(folder.unwrap()["uid"], "infra");
    assert_eq!(payload["dashboard"]["uid"], "cpu-main");
    assert_eq!(current_org["name"], "Main Org.");
    assert_eq!(orgs.len(), 2);
    assert_eq!(dashboard_permissions.len(), 1);
    assert_eq!(folder_permissions.len(), 1);
    assert_eq!(import["status"], "success");
    assert_eq!(deleted_dashboard["status"], "success");
    assert_eq!(deleted_folder["status"], "success");
    assert_eq!(created_folder["uid"], "platform");
    assert_eq!(created_folder["title"], "Platform");
    assert_eq!(datasources.len(), 1);
    assert_eq!(datasources[0]["uid"], "ds-main");

    let requests = requests.lock().unwrap().clone();
    assert!(requests[0].starts_with("GET /api/search?type=dash-db&limit=2&page=1 "));
    assert!(requests[1].starts_with("GET /api/search?type=dash-db&limit=2&page=2 "));
    assert!(requests[2].starts_with("GET /api/search?type=dash-db&query=cpu&limit=500 "));
    assert!(requests[3].starts_with("GET /api/folders/infra "));
    assert!(requests[4].starts_with("GET /api/dashboards/uid/cpu-main "));
    assert!(requests[5].starts_with("GET /api/org "));
    assert!(requests[6].starts_with("GET /api/orgs "));
    assert!(requests[7].starts_with("GET /api/dashboards/uid/cpu-main/permissions "));
    assert!(requests[8].starts_with("GET /api/folders/infra/permissions "));
    assert!(requests[9].starts_with("POST /api/dashboards/db "));
    assert!(requests[10].starts_with("DELETE /api/dashboards/uid/cpu-main "));
    assert!(requests[11].starts_with("DELETE /api/folders/infra "));
    assert!(requests[12].starts_with("POST /api/folders "));
    assert!(requests[12].contains("\"uid\":\"platform\""));
    assert!(requests[12].contains("\"title\":\"Platform\""));
    assert!(requests[12].contains("\"parentUid\":\"ops\""));
    assert!(requests[13].starts_with("GET /api/datasources "));
}

#[test]
fn datasource_resource_client_lists_datasources() {
    let responses = vec![http_response(
        "200 OK",
        r#"[{"uid":"prom-main","name":"Prometheus"}]"#,
    )];
    let (base_url, requests, handle) = spawn_sequence_server(responses);
    let api = build_test_api(base_url);
    let datasource = api.datasource();

    let items = datasource.list_datasources().unwrap();

    handle.join().unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["uid"], "prom-main");

    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /api/datasources "));
}
