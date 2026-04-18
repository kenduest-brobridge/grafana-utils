use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{json, Value};

#[test]
fn dashboard_view_lines_include_recent_versions_when_history_exists() {
    let node = crate::dashboard::browse_support::DashboardBrowseNode {
        kind: crate::dashboard::browse_support::DashboardBrowseNodeKind::Dashboard,
        title: "CPU Main".to_string(),
        path: "Platform / Infra".to_string(),
        uid: Some("cpu-main".to_string()),
        depth: 1,
        meta: "uid=cpu-main".to_string(),
        details: vec!["Type: Dashboard".to_string()],
        url: None,
        org_name: "Main Org.".to_string(),
        org_id: "1".to_string(),
        child_count: 0,
    };

    let lines = fetch_dashboard_view_lines_with_request(
        |method, path, params, _payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "id": 42,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "version": 7,
                    "schemaVersion": 39,
                    "tags": ["prod"],
                    "panels": [],
                    "links": []
                },
                "meta": {
                    "slug": "cpu-main",
                    "canEdit": true
                }
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions") => {
                assert_eq!(params, &vec![("limit".to_string(), "5".to_string())]);
                Ok(Some(json!([
                    {
                        "version": 7,
                        "created": "2026-03-26T10:00:00Z",
                        "createdBy": "admin",
                        "message": "rename"
                    },
                    {
                        "version": 6,
                        "created": "2026-03-20T08:00:00Z",
                        "createdBy": "ops",
                        "message": ""
                    }
                ])))
            }
            _ => Err(message("unexpected request")),
        },
        &node,
    )
    .unwrap();

    assert!(lines.iter().any(|line| line == "Recent versions:"));
    assert!(lines
        .iter()
        .any(|line| line.contains("v7 | 2026-03-26T10:00:00Z | admin | rename")));
    assert!(lines
        .iter()
        .any(|line| line.contains("v6 | 2026-03-20T08:00:00Z | ops")));
}

#[test]
fn dashboard_view_lines_ignore_missing_versions_endpoint() {
    let node = crate::dashboard::browse_support::DashboardBrowseNode {
        kind: crate::dashboard::browse_support::DashboardBrowseNodeKind::Dashboard,
        title: "CPU Main".to_string(),
        path: "Platform / Infra".to_string(),
        uid: Some("cpu-main".to_string()),
        depth: 1,
        meta: "uid=cpu-main".to_string(),
        details: vec!["Type: Dashboard".to_string()],
        url: None,
        org_name: "Main Org.".to_string(),
        org_id: "1".to_string(),
        child_count: 0,
    };

    let lines = fetch_dashboard_view_lines_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "id": 42,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "version": 7,
                    "schemaVersion": 39,
                    "tags": ["prod"],
                    "panels": [],
                    "links": []
                },
                "meta": {
                    "slug": "cpu-main",
                    "canEdit": true
                }
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions") => Err(api_response(
                404,
                "http://localhost:3000/api/dashboards/uid/cpu-main/versions?limit=5",
                "{\"message\":\"Not found\"}",
            )),
            _ => Err(message("unexpected request")),
        },
        &node,
    )
    .unwrap();

    assert!(!lines.iter().any(|line| line == "Recent versions:"));
    assert!(lines.iter().any(|line| line == "Version: 7"));
}

#[test]
fn dashboard_history_versions_lists_recent_versions_by_uid() {
    let versions = list_dashboard_history_versions_with_request(
        |method, path, params, _payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main/versions") => {
                assert_eq!(params, &vec![("limit".to_string(), "20".to_string())]);
                Ok(Some(json!({
                    "versions": [
                        {
                            "version": 7,
                            "created": "2026-03-26T10:00:00Z",
                            "createdBy": "admin",
                            "message": "rename"
                        },
                        {
                            "version": 6,
                            "created": "2026-03-20T08:00:00Z",
                            "createdBy": "ops",
                            "message": ""
                        }
                    ]
                })))
            }
            _ => Err(message("unexpected request")),
        },
        "cpu-main",
        20,
    )
    .unwrap();

    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 7);
    assert_eq!(versions[0].created_by, "admin");
    assert_eq!(versions[1].version, 6);
}

#[test]
fn dashboard_history_restore_reimports_selected_version_payload() {
    let payloads = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let recorded = payloads.clone();

    restore_dashboard_history_version_with_request(
        move |method, path, _params, payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "id": 42,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "version": 7
                },
                "meta": {
                    "folderUid": "infra"
                }
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions/5") => Ok(Some(json!({
                "version": 5,
                "data": {
                    "id": 42,
                    "version": 5,
                    "uid": "cpu-main",
                    "title": "CPU Old",
                    "tags": ["legacy"]
                }
            }))),
            (Method::POST, "/api/dashboards/db") => {
                recorded
                    .lock()
                    .unwrap()
                    .push(payload.cloned().unwrap_or(Value::Null));
                Ok(Some(json!({"status": "success"})))
            }
            _ => Err(message("unexpected request")),
        },
        "cpu-main",
        5,
    )
    .unwrap();

    let payloads = payloads.lock().unwrap();
    assert_eq!(payloads.len(), 1);
    let payload = payloads[0].as_object().unwrap();
    assert_eq!(payload["overwrite"], json!(true));
    assert_eq!(payload["folderUid"], json!("infra"));
    assert_eq!(payload["dashboard"]["uid"], json!("cpu-main"));
    assert_eq!(payload["dashboard"]["id"], json!(42));
    assert_eq!(payload["dashboard"]["title"], json!("CPU Old"));
    assert_eq!(payload["dashboard"]["version"], json!(7));
}

#[test]
fn dashboard_history_dialog_escape_and_q_close_dialog() {
    let versions = vec![crate::dashboard::history::DashboardHistoryVersion {
        version: 7,
        created: "2026-03-26T10:00:00Z".to_string(),
        created_by: "admin".to_string(),
        message: "rename".to_string(),
    }];
    let mut dialog = crate::dashboard::browse_history_dialog::HistoryDialogState::new(
        "cpu-main".to_string(),
        "CPU Main".to_string(),
        versions.clone(),
    );
    let esc = dialog.handle_key(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(
        esc,
        crate::dashboard::browse_history_dialog::HistoryDialogAction::Close
    );

    let mut dialog = crate::dashboard::browse_history_dialog::HistoryDialogState::new(
        "cpu-main".to_string(),
        "CPU Main".to_string(),
        versions,
    );
    let q = dialog.handle_key(&KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(
        q,
        crate::dashboard::browse_history_dialog::HistoryDialogAction::Close
    );
}

#[test]
fn dashboard_history_dialog_restore_review_uses_human_message() {
    let versions = vec![crate::dashboard::history::DashboardHistoryVersion {
        version: 7,
        created: "2026-03-26T10:00:00Z".to_string(),
        created_by: "admin".to_string(),
        message: "before query regression".to_string(),
    }];
    let mut dialog = crate::dashboard::browse_history_dialog::HistoryDialogState::new(
        "cpu-main".to_string(),
        "CPU Main".to_string(),
        versions,
    );
    assert_eq!(
        dialog.handle_key(&KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
        crate::dashboard::browse_history_dialog::HistoryDialogAction::Continue
    );
    let confirm = dialog.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        confirm,
        crate::dashboard::browse_history_dialog::HistoryDialogAction::Restore {
            uid: "cpu-main".to_string(),
            version: 7,
            message: "Restore CPU Main to version 7 (before query regression)".to_string(),
        }
    );
}
