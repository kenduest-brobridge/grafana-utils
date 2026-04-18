use super::super::team_browse_state::SearchDirection;
use super::*;
use crate::access::CommonCliArgs;
use crossterm::event::KeyEvent;
use reqwest::Method;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

fn common_args(api_token: Option<&str>) -> CommonCliArgs {
    CommonCliArgs {
        profile: None,
        url: "http://127.0.0.1:3000".to_string(),
        api_token: api_token.map(ToOwned::to_owned),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        org_id: None,
        timeout: 30,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
    }
}

fn live_browse_args() -> TeamBrowseArgs {
    TeamBrowseArgs {
        common: common_args(Some("token")),
        input_dir: None,
        query: None,
        name: None,
        with_members: true,
        page: 1,
        per_page: 100,
    }
}

fn member_row(identity: &str, role: &str) -> Value {
    let email = format!("{identity}@example.com");
    Value::Object(Map::from_iter(vec![
        (
            "memberIdentity".to_string(),
            Value::String(identity.to_string()),
        ),
        ("memberRole".to_string(), Value::String(role.to_string())),
        (
            "memberLogin".to_string(),
            Value::String(identity.to_string()),
        ),
        ("memberEmail".to_string(), Value::String(email)),
        ("parentTeamId".to_string(), Value::String("7".to_string())),
        (
            "parentTeamName".to_string(),
            Value::String("platform-ops".to_string()),
        ),
    ]))
}

fn selected_member_state(members: Vec<Value>) -> BrowserState {
    let mut state = BrowserState::new(vec![Map::from_iter(vec![
        ("id".to_string(), Value::String("7".to_string())),
        (
            "name".to_string(),
            Value::String("platform-ops".to_string()),
        ),
        (
            "email".to_string(),
            Value::String("platform@example.com".to_string()),
        ),
        ("memberRows".to_string(), Value::Array(members)),
    ])]);
    state.expand_selected();
    state.select_index(1);
    state
}

#[test]
fn search_prompt_treats_q_as_query_text() {
    let mut state = BrowserState::new(Vec::new());
    state.start_search(SearchDirection::Forward);

    handle_search_key(
        &mut state,
        &KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
    );

    assert_eq!(
        state
            .pending_search
            .as_ref()
            .map(|search| search.query.as_str()),
        Some("q")
    );
}

#[test]
fn load_rows_reads_local_team_bundle_without_live_requests() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("teams.json"),
        r#"{
            "kind":"grafana-utils-access-team-export-index",
            "version":1,
            "records":[
                {"name":"platform-team","email":"platform@example.com","members":["alice"],"admins":["bob"]}
            ]
        }"#,
    )
    .unwrap();
    let args = TeamBrowseArgs {
        common: common_args(None),
        input_dir: Some(temp.path().to_path_buf()),
        query: None,
        name: Some("platform-team".to_string()),
        with_members: true,
        page: 1,
        per_page: 100,
    };

    let rows = load_rows(
        |_method, _path, _params, _payload| {
            panic!("local team browse should not hit the request layer")
        },
        &args,
    )
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(map_get_text(&rows[0], "name"), "platform-team");
    assert_eq!(map_get_text(&rows[0], "members"), "alice,bob");
    assert!(matches!(rows[0].get("memberRows"), Some(Value::Array(values)) if values.len() == 2));
}

#[test]
fn member_row_edit_prompts_user_browse_instead_of_team_editor() {
    let mut state = selected_member_state(vec![member_row("alice", "Member")]);
    let args = live_browse_args();

    let mut request_json = |_method: Method,
                            _path: &str,
                            _params: &[(String, String)],
                            _payload: Option<&Value>|
     -> Result<Option<Value>> {
        panic!("member row edit should not call the request layer");
    };

    let action = handle_key(
        &mut request_json,
        &args,
        &mut state,
        &KeyEvent::new(
            crossterm::event::KeyCode::Char('e'),
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .unwrap();

    assert!(matches!(action, BrowseAction::Continue));
    assert!(state.pending_edit.is_none());
    assert!(state.status.contains("access user browse"));
}

#[test]
fn member_row_remove_updates_membership_and_keeps_parent_selected() {
    let mut state = selected_member_state(vec![
        member_row("alice", "Member"),
        member_row("bob", "Admin"),
    ]);
    let args = live_browse_args();
    let mut removed = false;
    let mut request_json = |method: Method,
                            path: &str,
                            _params: &[(String, String)],
                            payload: Option<&Value>|
     -> Result<Option<Value>> {
        match (method, path) {
            (Method::GET, "/api/teams/search") => Ok(Some(json!({
                "teams": [
                    {"id": "7", "name": "platform-ops", "email": "platform@example.com", "memberCount": 2}
                ]
            }))),
            (Method::GET, "/api/teams/7") => Ok(Some(json!({
                "id": "7",
                "name": "platform-ops",
                "email": "platform@example.com"
            }))),
            (Method::GET, "/api/teams/7/members") => {
                if removed {
                    Ok(Some(json!([
                        {"email": "bob@example.com", "login": "bob", "name": "Bob", "isAdmin": true, "userId": "43"}
                    ])))
                } else {
                    Ok(Some(json!([
                        {"email": "alice@example.com", "login": "alice", "name": "Alice", "isAdmin": false, "userId": "42"},
                        {"email": "bob@example.com", "login": "bob", "name": "Bob", "isAdmin": true, "userId": "43"}
                    ])))
                }
            }
            (Method::GET, "/api/org/users") => Ok(Some(json!([
                {"userId": "42", "login": "alice", "email": "alice@example.com", "name": "Alice"},
                {"userId": "43", "login": "bob", "email": "bob@example.com", "name": "Bob"}
            ]))),
            (Method::DELETE, "/api/teams/7/members/42") => {
                assert!(payload.is_none());
                removed = true;
                Ok(Some(json!({})))
            }
            other => panic!("unexpected request: {:?}", other),
        }
    };

    let action = handle_key(
        &mut request_json,
        &args,
        &mut state,
        &KeyEvent::new(
            crossterm::event::KeyCode::Char('r'),
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .unwrap();

    assert!(matches!(action, BrowseAction::Continue));
    assert!(state.pending_member_remove);
    assert_eq!(state.status, "Previewing team membership removal.");

    let action = handle_key(
        &mut request_json,
        &args,
        &mut state,
        &KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .unwrap();

    assert!(matches!(action, BrowseAction::Continue));
    assert!(state
        .status
        .contains("Removed alice from team platform-ops."));
    assert_eq!(state.selected_team_id().as_deref(), Some("7"));
    assert_eq!(state.rows.len(), 2);
    assert_eq!(map_get_text(&state.rows[1], "memberIdentity"), "bob");
}

#[test]
fn member_row_d_opens_membership_remove_confirmation() {
    let mut state = selected_member_state(vec![member_row("alice", "Member")]);
    let args = live_browse_args();

    let action = handle_key(
        &mut |_method, _path, _params, _payload| {
            panic!("member-row delete preview should not call Grafana before confirmation")
        },
        &args,
        &mut state,
        &KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .unwrap();

    assert!(matches!(action, BrowseAction::Continue));
    assert!(state.pending_member_remove);
    assert_eq!(state.status, "Previewing team membership removal.");
}

#[test]
fn member_row_toggle_admin_posts_the_team_admin_update_payload() {
    let mut state = selected_member_state(vec![member_row("alice", "Member")]);
    let args = live_browse_args();
    let mut admin_updated = false;
    let mut saw_payload = None::<Value>;
    let mut request_json = |method: Method,
                            path: &str,
                            _params: &[(String, String)],
                            payload: Option<&Value>|
     -> Result<Option<Value>> {
        match (method, path) {
            (Method::GET, "/api/teams/search") => Ok(Some(json!({
                "teams": [
                    {"id": "7", "name": "platform-ops", "email": "platform@example.com", "memberCount": 1}
                ]
            }))),
            (Method::GET, "/api/teams/7") => Ok(Some(json!({
                "id": "7",
                "name": "platform-ops",
                "email": "platform@example.com"
            }))),
            (Method::GET, "/api/teams/7/members") => {
                if admin_updated {
                    Ok(Some(json!([
                        {"email": "alice@example.com", "login": "alice", "name": "Alice", "isAdmin": true, "userId": "42"}
                    ])))
                } else {
                    Ok(Some(json!([
                        {"email": "alice@example.com", "login": "alice", "name": "Alice", "isAdmin": false, "userId": "42"}
                    ])))
                }
            }
            (Method::GET, "/api/org/users") => Ok(Some(json!([
                {"userId": "42", "login": "alice", "email": "alice@example.com", "name": "Alice"}
            ]))),
            (Method::PUT, "/api/teams/7/members") => {
                saw_payload = payload.cloned();
                admin_updated = true;
                Ok(Some(json!({})))
            }
            other => panic!("unexpected request: {:?}", other),
        }
    };

    let action = handle_key(
        &mut request_json,
        &args,
        &mut state,
        &KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .unwrap();

    assert!(matches!(action, BrowseAction::Continue));
    assert!(state
        .status
        .contains("Granted team admin to alice on platform-ops."));
    assert_eq!(
        saw_payload,
        Some(json!({
            "members": ["alice@example.com"],
            "admins": ["alice@example.com"]
        }))
    );
}
