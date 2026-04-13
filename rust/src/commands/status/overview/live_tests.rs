use crate::overview::run_overview_live;
use crate::project_status_command::{execute_project_status_live, ProjectStatusLiveArgs};
use serde_json::json;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;

struct LiveRequestRecord {
    path: String,
    org_id: Option<String>,
}

fn live_response_body(target: &str, org_id: Option<&str>) -> String {
    let path = target.split('?').next().unwrap_or(target);
    let scoped_org_id = org_id.unwrap_or("1");
    let scoped_org_name = if scoped_org_id == "2" {
        "Ops Org"
    } else {
        "Main Org"
    };

    match path {
        "/api/search" => serde_json::to_string(&json!([
            {
                "uid": format!("dash-{scoped_org_id}"),
                "title": format!("Dashboard {scoped_org_id}"),
                "type": "dash-db",
                "folderUid": "general",
                "folderTitle": "General",
                "url": format!("/d/dash-{scoped_org_id}/dashboard-{scoped_org_id}")
            }
        ]))
        .unwrap(),
        "/api/datasources" => serde_json::to_string(&json!([
            {
                "id": scoped_org_id.parse::<i64>().unwrap_or(1),
                "uid": format!("ds-{scoped_org_id}"),
                "name": format!("Datasource {scoped_org_id}"),
                "type": "prometheus",
                "access": "proxy",
                "isDefault": true,
                "orgId": scoped_org_id
            }
        ]))
        .unwrap(),
        "/api/orgs" => serde_json::to_string(&json!([
            {"id": 1, "name": "Main Org"},
            {"id": 2, "name": "Ops Org"}
        ]))
        .unwrap(),
        "/api/org" => serde_json::to_string(&json!({
            "id": scoped_org_id.parse::<i64>().unwrap_or(1),
            "name": scoped_org_name
        }))
        .unwrap(),
        "/api/dashboards/uid/dash-1/versions" | "/api/dashboards/uid/dash-2/versions" => {
            serde_json::to_string(&json!({
                "versions": [{"created": "2026-03-30T00:00:00Z"}]
            }))
            .unwrap()
        }
        "/api/v1/provisioning/alert-rules" => "[]".to_string(),
        "/api/v1/provisioning/contact-points" => "[]".to_string(),
        "/api/v1/provisioning/mute-timings" => "[]".to_string(),
        "/api/v1/provisioning/policies" => "{}".to_string(),
        "/api/v1/provisioning/templates" => "[]".to_string(),
        "/api/org/users" => "[]".to_string(),
        "/api/teams/search" => r#"{"teams":[]}"#.to_string(),
        "/api/serviceaccounts/search" => r#"{"serviceAccounts":[]}"#.to_string(),
        _ => "{}".to_string(),
    }
}

#[allow(clippy::type_complexity)]
fn spawn_live_project_status_test_server() -> (
    String,
    Arc<Mutex<Vec<LiveRequestRecord>>>,
    mpsc::Sender<()>,
    thread::JoinHandle<()>,
) {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            let (stop_tx, _stop_rx) = mpsc::channel();
            return (
                String::new(),
                Arc::new(Mutex::new(Vec::new())),
                stop_tx,
                thread::spawn(|| {}),
            );
        }
        Err(error) => panic!("failed to bind live project-status test listener: {error}"),
    };
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let requests_for_thread = Arc::clone(&requests);
    let (stop_tx, stop_rx) = mpsc::channel();

    let handle = thread::spawn(move || loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();

                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                loop {
                    let bytes_read = match stream.read(&mut buffer) {
                        Ok(bytes_read) => bytes_read,
                        Err(error)
                            if matches!(
                                error.kind(),
                                ErrorKind::WouldBlock | ErrorKind::TimedOut
                            ) =>
                        {
                            0
                        }
                        Err(error) => panic!("failed to read live test request: {error}"),
                    };
                    if bytes_read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..bytes_read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }

                let request_text = String::from_utf8(request).unwrap();
                let mut lines = request_text.lines();
                let request_line = lines.next().unwrap_or_default();
                let target = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();
                let path = target.split('?').next().unwrap_or("/").to_string();
                let org_id = lines.find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("X-Grafana-Org-Id") {
                        Some(value.trim().to_string())
                    } else {
                        None
                    }
                });

                requests_for_thread.lock().unwrap().push(LiveRequestRecord {
                    path: path.clone(),
                    org_id: org_id.clone(),
                });

                let body = live_response_body(&target, org_id.as_deref());
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).unwrap();
                let _ = stream.flush();
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => match stop_rx.try_recv() {
                Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(10));
                }
            },
            Err(error) => panic!("failed to accept live test request: {error}"),
        }
    });

    (format!("http://{address}"), requests, stop_tx, handle)
}

static LIVE_PROJECT_STATUS_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn collect_scoped_paths<'a>(
    requests: &'a [LiveRequestRecord],
    path: &str,
    org_id: &str,
) -> Vec<&'a LiveRequestRecord> {
    requests
        .iter()
        .filter(|request| request.path == path && request.org_id.as_deref() == Some(org_id))
        .collect()
}

fn sample_project_status_live_args(base_url: String) -> ProjectStatusLiveArgs {
    ProjectStatusLiveArgs {
        profile: None,
        url: base_url,
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 5,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
        all_orgs: false,
        org_id: None,
        sync_summary_file: None,
        bundle_preflight_file: None,
        promotion_summary_file: None,
        mapping_file: None,
        availability_file: None,
        output_format: crate::project_status_command::ProjectStatusOutputFormat::Text,
    }
}

#[test]
fn project_status_live_org_id_scopes_live_reads() {
    let _guard = LIVE_PROJECT_STATUS_TEST_LOCK.lock().unwrap();
    let (base_url, requests, stop_tx, handle) = spawn_live_project_status_test_server();
    if base_url.is_empty() {
        return;
    }
    let mut args = sample_project_status_live_args(base_url);
    args.org_id = Some(7);

    let status = execute_project_status_live(&args).unwrap();

    stop_tx.send(()).unwrap();
    handle.join().unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(status.scope, "live");
    assert!(!collect_scoped_paths(&requests, "/api/datasources", "7").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/org", "7").is_empty());
}

#[test]
fn project_status_live_all_orgs_fans_out_across_visible_orgs() {
    let _guard = LIVE_PROJECT_STATUS_TEST_LOCK.lock().unwrap();
    let (base_url, requests, stop_tx, handle) = spawn_live_project_status_test_server();
    if base_url.is_empty() {
        return;
    }
    let mut args = sample_project_status_live_args(base_url);
    args.api_token = None;
    args.username = Some("admin".to_string());
    args.password = Some("admin".to_string());
    args.all_orgs = true;

    let status = execute_project_status_live(&args).unwrap();

    stop_tx.send(()).unwrap();
    handle.join().unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(status.scope, "live");
    assert!(requests.iter().any(|request| request.path == "/api/orgs"));
    assert!(!collect_scoped_paths(&requests, "/api/datasources", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/datasources", "2").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/org/users", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/org/users", "2").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/teams/search", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/teams/search", "2").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/serviceaccounts/search", "1").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/serviceaccounts/search", "2").is_empty());
}

#[test]
fn overview_live_delegates_org_scoped_reads_to_shared_live_path() {
    let _guard = LIVE_PROJECT_STATUS_TEST_LOCK.lock().unwrap();
    let (base_url, requests, stop_tx, handle) = spawn_live_project_status_test_server();
    if base_url.is_empty() {
        return;
    }
    let mut args = sample_project_status_live_args(base_url);
    args.org_id = Some(9);

    run_overview_live(args).unwrap();

    stop_tx.send(()).unwrap();
    handle.join().unwrap();

    let requests = requests.lock().unwrap();
    assert!(!collect_scoped_paths(&requests, "/api/datasources", "9").is_empty());
    assert!(!collect_scoped_paths(&requests, "/api/org", "9").is_empty());
}
