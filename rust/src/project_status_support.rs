//! Shared project-status helpers used by live status workflows.
//!
//! Responsibilities:
//! - Build authenticated HTTP clients and auth header sets for status checks.
//! - Resolve per-org connection settings and default behavior for live runs.

use crate::common::Result as CommonResult;
pub(crate) use crate::grafana_api::project_status_live;
use crate::grafana_api::{AuthInputs, GrafanaApiClient, GrafanaConnection};
use crate::http::JsonHttpClient;
use crate::profile_config::ConnectionMergeInput;
use crate::project_status_command::ProjectStatusLiveArgs;

fn resolve_live_project_status_connection(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
    include_org_header: bool,
) -> CommonResult<GrafanaConnection> {
    GrafanaConnection::resolve(
        args.profile.as_deref(),
        ConnectionMergeInput {
            url: &args.url,
            url_default: "",
            api_token: args.api_token.as_deref(),
            username: args.username.as_deref(),
            password: args.password.as_deref(),
            org_id,
            timeout: args.timeout,
            timeout_default: 30,
            verify_ssl: args.verify_ssl,
            insecure: args.insecure,
            ca_cert: args.ca_cert.as_deref(),
        },
        AuthInputs {
            api_token: args.api_token.as_deref(),
            username: args.username.as_deref(),
            password: args.password.as_deref(),
            prompt_password: args.prompt_password,
            prompt_token: args.prompt_token,
        },
        include_org_header,
    )
}

#[cfg(test)]
pub(crate) fn resolve_live_project_status_headers(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
) -> CommonResult<Vec<(String, String)>> {
    let connection = resolve_live_project_status_connection(args, org_id, true)?;
    Ok(connection.headers)
}

pub(crate) fn build_live_project_status_api_client(
    args: &ProjectStatusLiveArgs,
) -> CommonResult<GrafanaApiClient> {
    let connection = resolve_live_project_status_connection(
        args,
        if args.all_orgs { None } else { args.org_id },
        true,
    )?;
    GrafanaApiClient::from_connection(connection)
}

pub(crate) fn build_live_project_status_client_from_api(
    api: &GrafanaApiClient,
    org_id: Option<i64>,
) -> CommonResult<JsonHttpClient> {
    Ok(match org_id {
        Some(org_id) => api.scoped_to_org(org_id)?.into_http_client(),
        None => api.http_client().clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_live_project_status_api_client, build_live_project_status_client_from_api,
        resolve_live_project_status_headers,
    };
    use crate::project_status_command::{ProjectStatusLiveArgs, ProjectStatusOutputFormat};
    use reqwest::Method;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

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
        let address = listener.local_addr().unwrap();
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
                requests_thread
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&request).to_string());
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{address}"), requests, handle)
    }

    #[test]
    fn resolve_live_project_status_headers_adds_org_scope_when_requested() {
        let args = ProjectStatusLiveArgs {
            profile: None,
            url: "http://localhost:3000".to_string(),
            api_token: Some("token-123".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
            insecure: false,
            ca_cert: None,
            all_orgs: false,
            org_id: Some(7),
            sync_summary_file: None,
            bundle_preflight_file: None,
            promotion_summary_file: None,
            mapping_file: None,
            availability_file: None,
            output_format: ProjectStatusOutputFormat::Text,
        };

        let headers = resolve_live_project_status_headers(&args, args.org_id).unwrap();

        assert!(headers
            .iter()
            .any(|(name, value)| { name == "X-Grafana-Org-Id" && value == "7" }));
    }

    #[test]
    fn build_live_project_status_client_from_api_reuses_root_headers_and_adds_org_scope() {
        let responses = vec![http_response("200 OK", "{}")];
        let (base_url, requests, handle) = spawn_sequence_server(responses);
        let args = ProjectStatusLiveArgs {
            profile: None,
            url: base_url,
            api_token: Some("token-123".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
            insecure: false,
            ca_cert: None,
            all_orgs: true,
            org_id: None,
            sync_summary_file: None,
            bundle_preflight_file: None,
            promotion_summary_file: None,
            mapping_file: None,
            availability_file: None,
            output_format: ProjectStatusOutputFormat::Text,
        };
        let api = build_live_project_status_api_client(&args).unwrap();
        let scoped = build_live_project_status_client_from_api(&api, Some(9)).unwrap();

        let root_authorization = api
            .connection()
            .headers
            .iter()
            .find(|(name, _)| name == "Authorization")
            .map(|(_, value)| value.clone())
            .unwrap();
        assert_eq!(root_authorization, "Bearer token-123");

        scoped
            .request_json(Method::GET, "/api/org", &[], None)
            .unwrap();
        handle.join().unwrap();

        let request_text = requests.lock().unwrap()[0].to_ascii_lowercase();
        assert!(request_text.contains("authorization: bearer token-123"));
        assert!(request_text.contains("x-grafana-org-id: 9"));
    }
}
