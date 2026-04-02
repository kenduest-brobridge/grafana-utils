//! Interactive browse workflows and terminal-driven state flow for Dashboard entities.

use super::*;

#[test]
fn dashboard_delete_validate_args_requires_yes_without_dry_run() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: None,
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: Some("cpu-main".to_string()),
        path: None,
        delete_folders: false,
        yes: false,
        interactive: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };

    let error = validate_delete_args(&args).unwrap_err();
    assert!(error.to_string().contains("requires --yes"));
}

#[test]
fn dashboard_delete_build_plan_matches_path_subtree() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: None,
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: None,
        path: Some("Platform / Infra".to_string()),
        delete_folders: true,
        yes: true,
        interactive: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };

    let plan = build_delete_plan_with_request(
        |method, path, params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => {
                let page = params
                    .iter()
                    .find(|(key, _)| key == "page")
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("1");
                if page == "1" {
                    Ok(Some(json!([
                        {"uid":"cpu-main","title":"CPU","folderUid":"infra","folderTitle":"Infra"},
                        {"uid":"mem-main","title":"Memory","folderUid":"child","folderTitle":"Child"},
                        {"uid":"ops-main","title":"Ops","folderUid":"ops","folderTitle":"Ops"}
                    ])))
                } else {
                    Ok(Some(json!([])))
                }
            }
            (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                "uid":"infra",
                "title":"Infra",
                "parents":[{"uid":"platform","title":"Platform"}]
            }))),
            (Method::GET, "/api/folders/child") => Ok(Some(json!({
                "uid":"child",
                "title":"Child",
                "parents":[{"uid":"platform","title":"Platform"},{"uid":"infra","title":"Infra"}]
            }))),
            (Method::GET, "/api/folders/ops") => Ok(Some(json!({
                "uid":"ops",
                "title":"Ops"
            }))),
            (Method::GET, "/api/folders/platform") => Ok(Some(json!({
                "uid":"platform",
                "title":"Platform"
            }))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(plan.dashboards.len(), 2);
    assert_eq!(plan.folders.len(), 2);
    assert_eq!(plan.dashboards[0].uid, "cpu-main");
    assert_eq!(plan.dashboards[1].uid, "mem-main");
    assert_eq!(plan.folders[0].uid, "child");
    assert_eq!(plan.folders[1].uid, "infra");
}

#[test]
fn dashboard_delete_with_request_deletes_dashboards_then_folders() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: None,
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: None,
        path: Some("Platform / Infra".to_string()),
        delete_folders: true,
        yes: true,
        interactive: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = calls.clone();

    let count = delete_dashboards_with_request(
        move |method, path, params, _payload| {
            recorded
                .lock()
                .unwrap()
                .push((method.clone(), path.to_string(), params.to_vec()));
            match (method.clone(), path) {
                (Method::GET, "/api/search") => {
                    let page = params
                        .iter()
                        .find(|(key, _)| key == "page")
                        .map(|(_, value)| value.as_str())
                        .unwrap_or("1");
                    if page == "1" {
                        Ok(Some(json!([
                            {"uid":"cpu-main","title":"CPU","folderUid":"infra","folderTitle":"Infra"},
                            {"uid":"mem-main","title":"Memory","folderUid":"child","folderTitle":"Child"}
                        ])))
                    } else {
                        Ok(Some(json!([])))
                    }
                }
                (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                    "uid":"infra",
                    "title":"Infra",
                    "parents":[{"uid":"platform","title":"Platform"}]
                }))),
                (Method::GET, "/api/folders/child") => Ok(Some(json!({
                    "uid":"child",
                    "title":"Child",
                    "parents":[{"uid":"platform","title":"Platform"},{"uid":"infra","title":"Infra"}]
                }))),
                (Method::GET, "/api/folders/platform") => Ok(Some(json!({
                    "uid":"platform",
                    "title":"Platform"
                }))),
                (Method::DELETE, "/api/dashboards/uid/cpu-main") => {
                    Ok(Some(json!({"status":"success"})))
                }
                (Method::DELETE, "/api/dashboards/uid/mem-main") => {
                    Ok(Some(json!({"status":"success"})))
                }
                (Method::DELETE, "/api/folders/child") => Ok(Some(json!({"status":"success"}))),
                (Method::DELETE, "/api/folders/infra") => Ok(Some(json!({"status":"success"}))),
                _ => Err(message(format!("unexpected request {method} {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 4);
    let calls = calls.lock().unwrap();
    let delete_paths: Vec<String> = calls
        .iter()
        .filter(|(method, _, _)| *method == Method::DELETE)
        .map(|(_, path, _)| path.clone())
        .collect();
    assert_eq!(
        delete_paths,
        vec![
            "/api/dashboards/uid/cpu-main".to_string(),
            "/api/dashboards/uid/mem-main".to_string(),
            "/api/folders/child".to_string(),
            "/api/folders/infra".to_string(),
        ]
    );
}

#[test]
fn dashboard_browse_document_builds_tree_with_general_and_nested_folders() {
    let summaries = vec![
        serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "infra",
            "folderTitle": "Infra",
            "folderPath": "Platform / Infra",
            "url": "/d/cpu-main/cpu-main"
        }))
        .unwrap(),
        serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "mem-main",
            "title": "Memory Main",
            "folderUid": "",
            "folderTitle": "General",
            "folderPath": "General",
            "url": "/d/mem-main/memory-main"
        }))
        .unwrap(),
    ];
    let folders = vec![crate::dashboard::FolderInventoryItem {
        uid: "infra".to_string(),
        title: "Infra".to_string(),
        path: "Platform / Infra".to_string(),
        parent_uid: Some("platform".to_string()),
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    }];

    let document = build_dashboard_browse_document(&summaries, &folders, None).unwrap();

    assert_eq!(document.summary.folder_count, 3);
    assert_eq!(document.summary.dashboard_count, 2);
    assert_eq!(document.nodes[0].title, "General");
    assert_eq!(document.nodes[1].title, "Memory Main");
    assert_eq!(document.nodes[1].depth, 1);
    assert_eq!(document.nodes[2].title, "Platform");
    assert_eq!(document.nodes[3].title, "Infra");
    assert_eq!(document.nodes[4].title, "CPU Main");
    assert_eq!(document.nodes[4].depth, 2);
}

#[test]
fn dashboard_browse_document_filters_to_requested_root_path() {
    let summaries = vec![
        serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "infra",
            "folderTitle": "Infra",
            "folderPath": "Platform / Infra"
        }))
        .unwrap(),
        serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "ops-main",
            "title": "Ops Main",
            "folderUid": "ops",
            "folderTitle": "Ops",
            "folderPath": "Ops"
        }))
        .unwrap(),
    ];
    let folders = vec![
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
            path: "Ops".to_string(),
            parent_uid: None,
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
    ];

    let document =
        build_dashboard_browse_document(&summaries, &folders, Some("Platform / Infra")).unwrap();

    assert_eq!(
        document.summary.root_path.as_deref(),
        Some("Platform / Infra")
    );
    assert_eq!(document.summary.folder_count, 1);
    assert_eq!(document.summary.dashboard_count, 1);
    assert_eq!(document.nodes.len(), 2);
    assert_eq!(document.nodes[0].title, "Infra");
    assert_eq!(document.nodes[0].depth, 0);
    assert_eq!(document.nodes[1].title, "CPU Main");
    assert_eq!(document.nodes[1].depth, 1);
}

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
fn browser_state_replace_document_preserves_selected_dashboard_uid() {
    let old_document = build_dashboard_browse_document(
        &[
            serde_json::from_value::<Map<String, Value>>(json!({
                "uid": "cpu-main",
                "title": "CPU Main",
                "folderUid": "infra",
                "folderTitle": "Infra",
                "folderPath": "Platform / Infra"
            }))
            .unwrap(),
            serde_json::from_value::<Map<String, Value>>(json!({
                "uid": "mem-main",
                "title": "Memory Main",
                "folderUid": "infra",
                "folderTitle": "Infra",
                "folderPath": "Platform / Infra"
            }))
            .unwrap(),
        ],
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
    let new_document = build_dashboard_browse_document(
        &[
            serde_json::from_value::<Map<String, Value>>(json!({
                "uid": "cpu-main",
                "title": "CPU Main",
                "folderUid": "ops",
                "folderTitle": "Ops",
                "folderPath": "Platform / Ops"
            }))
            .unwrap(),
            serde_json::from_value::<Map<String, Value>>(json!({
                "uid": "mem-main",
                "title": "Memory Main",
                "folderUid": "infra",
                "folderTitle": "Infra",
                "folderPath": "Platform / Infra"
            }))
            .unwrap(),
        ],
        &[
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
    let mut state = crate::dashboard::browse_state::BrowserState::new(old_document);
    let selected_index = state
        .document
        .nodes
        .iter()
        .position(|node| node.uid.as_deref() == Some("cpu-main"))
        .expect("cpu-main index");
    state.list_state.select(Some(selected_index));

    state.replace_document(new_document);

    let selected = state.selected_node().expect("selected node");
    assert_eq!(selected.uid.as_deref(), Some("cpu-main"));
    assert_eq!(selected.path, "Platform / Ops");
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
                    "uid": "cpu-main",
                    "title": "CPU Main"
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
    assert_eq!(payload["dashboard"]["id"], Value::Null);
    assert_eq!(payload["dashboard"]["title"], json!("CPU Old"));
    assert!(payload["dashboard"].get("version").is_none());
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
fn interactive_import_loads_dashboard_titles_and_folder_paths() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );

    let args = make_import_args(raw_dir);
    let items = load_interactive_import_items(&args).unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].uid, "cpu-main");
    assert_eq!(items[0].title, "CPU Main");
    assert_eq!(items[0].folder_path, "Infra");
}

#[test]
fn interactive_import_state_toggles_and_confirms_selected_files() {
    let items = vec![
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("a.json"),
            uid: "a".to_string(),
            title: "CPU".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "a.json".to_string(),
            review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
        },
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("b.json"),
            uid: "b".to_string(),
            title: "Memory".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "b.json".to_string(),
            review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
        },
    ];
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);

    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(state.selected_files(), vec![PathBuf::from("a.json")]);
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.selected_files(),
        vec![PathBuf::from("a.json"), PathBuf::from("b.json")]
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        InteractiveImportAction::Confirm(vec![PathBuf::from("a.json"), PathBuf::from("b.json")])
    );
}

#[test]
fn interactive_import_grouping_cycles_folder_action_flat() {
    let items = vec![
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("a.json"),
            uid: "a".to_string(),
            title: "CPU".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "a.json".to_string(),
            review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
        },
    ];
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);

    assert_eq!(
        state.grouping,
        crate::dashboard::import_interactive::InteractiveImportGrouping::Folder
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.grouping,
        crate::dashboard::import_interactive::InteractiveImportGrouping::Action
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.grouping,
        crate::dashboard::import_interactive::InteractiveImportGrouping::Flat
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.grouping,
        crate::dashboard::import_interactive::InteractiveImportGrouping::Folder
    );
}

#[test]
fn interactive_import_context_view_scope_and_diff_depth_cycle() {
    let items = vec![
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("a.json"),
            uid: "a".to_string(),
            title: "CPU".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "a.json".to_string(),
            review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
        },
    ];
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);

    assert_eq!(
        state.context_view,
        crate::dashboard::import_interactive::InteractiveImportContextView::Summary
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.context_view,
        crate::dashboard::import_interactive::InteractiveImportContextView::Destination
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.summary_scope,
        crate::dashboard::import_interactive::InteractiveImportSummaryScope::Selected
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.diff_depth,
        crate::dashboard::import_interactive::InteractiveImportDiffDepth::Structural
    );
}

#[test]
fn interactive_import_summary_counts_track_pending_selected_and_reviewed_actions() {
    let mut state = InteractiveImportState::new(
        vec![
            crate::dashboard::import_interactive::InteractiveImportItem {
                path: PathBuf::from("a.json"),
                uid: "a".to_string(),
                title: "CPU".to_string(),
                folder_path: "Infra".to_string(),
                file_label: "a.json".to_string(),
                review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
            },
            crate::dashboard::import_interactive::InteractiveImportItem {
                path: PathBuf::from("b.json"),
                uid: "b".to_string(),
                title: "Memory".to_string(),
                folder_path: "Infra".to_string(),
                file_label: "b.json".to_string(),
                review:
                    crate::dashboard::import_interactive::InteractiveImportReviewState::Resolved(
                        Box::new(
                            crate::dashboard::import_interactive::InteractiveImportReview {
                                action: "would-update".to_string(),
                                destination: "exists".to_string(),
                                action_label: "update".to_string(),
                                folder_path: "Infra".to_string(),
                                source_folder_path: "Infra".to_string(),
                                destination_folder_path: "Infra".to_string(),
                                reason: "".to_string(),
                                diff_status: "changed".to_string(),
                                diff_summary_lines: vec!["Title: old -> new".to_string()],
                                diff_structural_lines: vec!["Panels: 1 -> 2".to_string()],
                                diff_raw_lines: vec!["REMOTE".to_string(), "LOCAL".to_string()],
                            },
                        ),
                    ),
            },
        ],
        "create-only".to_string(),
        false,
    );
    state.selected_paths.insert(PathBuf::from("b.json"));

    let counts = state.review_summary_counts();

    assert_eq!(counts.total, 2);
    assert_eq!(counts.selected, 1);
    assert_eq!(counts.pending, 1);
    assert_eq!(counts.reviewed, 1);
    assert_eq!(counts.update, 1);
}

#[test]
fn interactive_import_dry_run_state_uses_dry_run_status_and_enter_copy() {
    let state = InteractiveImportState::new(
        vec![
            crate::dashboard::import_interactive::InteractiveImportItem {
                path: PathBuf::from("a.json"),
                uid: "a".to_string(),
                title: "CPU".to_string(),
                folder_path: "Infra".to_string(),
                file_label: "a.json".to_string(),
                review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
            },
        ],
        "create-only".to_string(),
        true,
    );

    assert!(state.dry_run);
    assert!(state.status.contains("dry-run"));
}

#[test]
fn interactive_import_resolves_focused_review_to_update_existing() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );
    let args = make_import_args(raw_dir.clone());
    let items = load_interactive_import_items(&args).unwrap();
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);
    let mut cache = crate::dashboard::import_lookup::ImportLookupCache::default();

    state.resolve_focused_review_with_request(
        &mut |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => Ok(Some(json!([
                {"uid":"cpu-main","title":"CPU Main","folderUid":"infra"}
            ]))),
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid":"cpu-main",
                    "title":"CPU Main",
                    "tags":[],
                    "panels":[{"id":1}]
                },
                "meta": {"folderUid":"infra"}
            }))),
            (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                "uid":"infra",
                "title":"Infra"
            }))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &mut cache,
        &args,
    );

    let item = state.selected_item().unwrap();
    match &item.review {
        crate::dashboard::import_interactive::InteractiveImportReviewState::Resolved(review) => {
            assert_eq!(review.action, "would-fail-existing");
            assert_eq!(review.destination, "exists");
            assert_eq!(review.action_label, "blocked-existing");
            assert_eq!(review.folder_path, "Infra");
            assert_eq!(review.diff_status, "matches live");
            assert!(review.diff_summary_lines[0].contains("already matches"));
        }
        other => panic!("expected resolved review, got {other:?}"),
    }
}

#[test]
fn interactive_import_resolves_skip_missing_for_update_existing_only() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );
    let mut args = make_import_args(raw_dir.clone());
    args.update_existing_only = true;
    let items = load_interactive_import_items(&args).unwrap();
    let mut state = InteractiveImportState::new(items, "update-or-skip-missing".to_string(), false);
    let mut cache = crate::dashboard::import_lookup::ImportLookupCache::default();

    state.resolve_focused_review_with_request(
        &mut |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => Ok(Some(json!([]))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &mut cache,
        &args,
    );

    let item = state.selected_item().unwrap();
    match &item.review {
        crate::dashboard::import_interactive::InteractiveImportReviewState::Resolved(review) => {
            assert_eq!(review.action, "would-skip-missing");
            assert_eq!(review.action_label, "skip-missing");
            assert_eq!(review.destination, "missing");
            assert_eq!(review.diff_status, "new dashboard");
        }
        other => panic!("expected resolved review, got {other:?}"),
    }
}

#[test]
fn interactive_import_review_surfaces_changed_live_summary() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );
    let args = make_import_args(raw_dir.clone());
    let items = load_interactive_import_items(&args).unwrap();
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);
    let mut cache = crate::dashboard::import_lookup::ImportLookupCache::default();

    state.resolve_focused_review_with_request(
        &mut |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => Ok(Some(json!([
                {"uid":"cpu-main","title":"CPU Overview","folderUid":"ops"}
            ]))),
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid":"cpu-main",
                    "title":"CPU Overview",
                    "tags":["gold","ops"],
                    "panels":[{"id":1},{"id":2}]
                },
                "meta": {"folderUid":"ops"}
            }))),
            (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                "uid":"infra",
                "title":"Infra"
            }))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &mut cache,
        &args,
    );

    let item = state.selected_item().unwrap();
    match &item.review {
        crate::dashboard::import_interactive::InteractiveImportReviewState::Resolved(review) => {
            assert_eq!(review.diff_status, "changed");
            assert!(review
                .diff_summary_lines
                .iter()
                .any(|line| line.contains("Title:")));
            assert!(review
                .diff_summary_lines
                .iter()
                .any(|line| line.contains("Folder UID:")));
            assert!(review
                .diff_summary_lines
                .iter()
                .any(|line| line.contains("Tags:")));
            assert!(review
                .diff_summary_lines
                .iter()
                .any(|line| line.contains("Panels:")));
        }
        other => panic!("expected resolved review, got {other:?}"),
    }
}

#[test]
fn interactive_import_with_use_export_org_falls_through_to_tty_validation() {
    let temp = tempdir().unwrap();
    let mut args = make_import_args(temp.path().join("exports"));
    args.use_export_org = true;
    args.interactive = true;
    let mut cache = crate::dashboard::import_lookup::ImportLookupCache::default();

    let error = crate::dashboard::import_interactive::select_import_dashboard_files(
        &mut |_method, _path, _params, _payload| Ok(None),
        &mut cache,
        &args,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("Dashboard import interactive mode requires a TTY."));
}

#[test]
fn dashboard_raw_edit_validation_rejects_overwrite_in_user_payload() {
    let error = validate_external_dashboard_edit_value(&json!({
        "dashboard": {
            "uid": "cpu-main",
            "title": "CPU Main"
        },
        "overwrite": true
    }))
    .unwrap_err();

    assert!(error.to_string().contains("must not include overwrite"));
}

#[test]
fn dashboard_raw_edit_review_summarizes_title_tags_and_folder_uid_changes() {
    let draft = ExternalDashboardEditDraft {
        uid: "cpu-main".to_string(),
        title: "CPU Main".to_string(),
        payload: json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "tags": ["prod"]
            },
            "folderUid": "infra"
        }),
    };

    let review = review_external_dashboard_edit(
        &draft,
        &json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Overview",
                "tags": ["gold", "ops"]
            },
            "folderUid": "ops"
        }),
    )
    .unwrap()
    .unwrap();

    assert!(review.summary_lines[0].contains("uid=cpu-main"));
    assert!(review.summary_lines[1].contains("CPU Main -> CPU Overview"));
    assert!(review.summary_lines[3].contains("infra -> ops"));
    assert!(review.summary_lines[4].contains("prod -> gold, ops"));
}

#[test]
fn dashboard_raw_edit_apply_posts_payload_with_overwrite_and_message() {
    let payloads = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let recorded = payloads.clone();

    apply_external_dashboard_edit_with_request(
        move |method, path, _params, payload| match (method, path) {
            (Method::POST, "/api/dashboards/db") => {
                recorded
                    .lock()
                    .unwrap()
                    .push(payload.cloned().unwrap_or(Value::Null));
                Ok(Some(json!({"status":"success"})))
            }
            _ => Err(message("unexpected request")),
        },
        &json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Overview",
                "tags": ["gold", "ops"]
            },
            "folderUid": "ops"
        }),
    )
    .unwrap();

    let payloads = payloads.lock().unwrap();
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0]["dashboard"]["title"], "CPU Overview");
    assert_eq!(payloads[0]["folderUid"], "ops");
    assert_eq!(payloads[0]["overwrite"], true);
    assert_eq!(
        payloads[0]["message"],
        "Edited by grafana-utils dashboard browse"
    );
}
