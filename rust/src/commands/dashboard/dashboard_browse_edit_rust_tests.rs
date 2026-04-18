use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{json, Map, Value};

#[test]
fn dashboard_edit_resolves_destination_folder_uid_from_browser_tree() {
    let document = build_dashboard_browse_document(
        &[serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "infra",
            "folderTitle": "Infra",
            "folderPath": "Platform / Infra"
        }))
        .unwrap()],
        &[crate::dashboard::FolderInventoryItem {
            uid: "infra".to_string(),
            title: "Infra".to_string(),
            path: "Platform / Infra".to_string(),
            parent_uid: Some("platform".to_string()),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        }],
        None,
    )
    .unwrap();

    let uid = resolve_folder_uid_for_path(&document, "Platform / Infra").unwrap();
    assert_eq!(uid, "infra");
}

#[test]
fn dashboard_edit_fetch_draft_reads_current_live_title_and_tags() {
    let node = crate::dashboard::browse_support::DashboardBrowseNode {
        kind: crate::dashboard::browse_support::DashboardBrowseNodeKind::Dashboard,
        title: "CPU Main".to_string(),
        path: "Platform / Infra".to_string(),
        uid: Some("cpu-main".to_string()),
        depth: 1,
        meta: "uid=cpu-main".to_string(),
        details: Vec::new(),
        url: None,
        org_name: "Main Org.".to_string(),
        org_id: "1".to_string(),
        child_count: 0,
    };

    let draft = fetch_dashboard_edit_draft_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "tags": ["prod", "infra"]
                },
                "meta": {
                    "folderUid": "infra"
                }
            }))),
            _ => Err(message("unexpected request")),
        },
        &node,
    )
    .unwrap();

    assert_eq!(draft.uid, "cpu-main");
    assert_eq!(draft.title, "CPU Main");
    assert_eq!(draft.folder_path, "Platform / Infra");
    assert_eq!(draft.tags, vec!["prod".to_string(), "infra".to_string()]);
}

#[test]
fn dashboard_edit_apply_posts_updated_title_tags_and_folder_uid() {
    let payloads = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let recorded = payloads.clone();
    let draft = DashboardEditDraft {
        uid: "cpu-main".to_string(),
        title: "CPU Main".to_string(),
        folder_path: "Platform / Infra".to_string(),
        tags: vec!["prod".to_string()],
    };
    let update = DashboardEditUpdate {
        title: Some("CPU Overview".to_string()),
        folder_path: Some("Platform / Ops".to_string()),
        tags: Some(vec!["ops".to_string(), "gold".to_string()]),
    };

    apply_dashboard_edit_with_request(
        move |method, path, _params, payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "tags": ["prod"]
                },
                "meta": {
                    "folderUid": "infra"
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
        &draft,
        &update,
        Some("ops"),
    )
    .unwrap();

    let payloads = payloads.lock().unwrap();
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0]["dashboard"]["title"], "CPU Overview");
    assert_eq!(payloads[0]["dashboard"]["tags"], json!(["ops", "gold"]));
    assert_eq!(payloads[0]["folderUid"], "ops");
    assert_eq!(payloads[0]["overwrite"], true);
}

#[test]
fn dashboard_edit_dialog_folder_picker_selects_existing_folder_path() {
    let document = build_dashboard_browse_document(
        &[serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "infra",
            "folderTitle": "Infra",
            "folderPath": "Platform / Infra"
        }))
        .unwrap()],
        &[
            crate::dashboard::FolderInventoryItem {
                uid: "platform".to_string(),
                title: "Platform".to_string(),
                path: "Platform".to_string(),
                parent_uid: None,
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
            crate::dashboard::FolderInventoryItem {
                uid: "infra".to_string(),
                title: "Infra".to_string(),
                path: "Platform / Infra".to_string(),
                parent_uid: Some("platform".to_string()),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
            crate::dashboard::FolderInventoryItem {
                uid: "ops".to_string(),
                title: "Ops".to_string(),
                path: "Platform / Ops".to_string(),
                parent_uid: Some("platform".to_string()),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
        ],
        None,
    )
    .unwrap();
    let draft = DashboardEditDraft {
        uid: "cpu-main".to_string(),
        title: "CPU Main".to_string(),
        folder_path: "Platform / Infra".to_string(),
        tags: vec!["prod".to_string()],
    };
    let mut dialog =
        crate::dashboard::browse_edit_dialog::EditDialogState::from_draft(draft, &document);

    let _ = dialog.handle_key(&KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let _ = dialog.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let _ = dialog.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let action = dialog.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        crate::dashboard::browse_edit_dialog::EditDialogAction::Continue
    );

    let save = dialog.handle_key(&KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    match save {
        crate::dashboard::browse_edit_dialog::EditDialogAction::Save { update, .. } => {
            assert_eq!(update.folder_path.as_deref(), Some("Platform / Ops"));
        }
        _ => panic!("expected save action"),
    }
}

#[test]
fn dashboard_edit_dialog_ctrl_x_closes_dialog() {
    let document = build_dashboard_browse_document(
        &[],
        &[crate::dashboard::FolderInventoryItem {
            uid: "infra".to_string(),
            title: "Infra".to_string(),
            path: "Platform / Infra".to_string(),
            parent_uid: Some("platform".to_string()),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        }],
        None,
    )
    .unwrap();
    let draft = DashboardEditDraft {
        uid: "cpu-main".to_string(),
        title: "CPU Main".to_string(),
        folder_path: "Platform / Infra".to_string(),
        tags: vec!["prod".to_string()],
    };
    let mut dialog =
        crate::dashboard::browse_edit_dialog::EditDialogState::from_draft(draft, &document);

    let action = dialog.handle_key(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    assert_eq!(
        action,
        crate::dashboard::browse_edit_dialog::EditDialogAction::Cancelled
    );
}
