use super::*;
use serde_json::{json, Map, Value};

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
