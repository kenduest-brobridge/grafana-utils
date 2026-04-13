#![cfg(feature = "tui")]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::Value;

use crate::common::{message, Result};
use crate::http::JsonHttpClient;

use super::datasource_browse_edit_dialog::{EditDialogAction, EditDialogState};
use super::datasource_browse_state::{
    BrowserState, PaneFocus, PendingDelete, SearchDirection, SearchState,
};
use super::datasource_browse_support::{
    build_modify_updates_from_browse, fetch_datasource_by_uid, load_datasource_browse_document,
};
use super::DatasourceBrowseArgs;

pub(crate) enum BrowserLoopAction {
    Continue,
    Exit,
}

pub(crate) fn handle_browser_key(
    client: &JsonHttpClient,
    args: &DatasourceBrowseArgs,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<BrowserLoopAction> {
    if state.pending_edit.is_some() {
        return handle_edit_key(client, args, state, key);
    }
    if state.pending_search.is_some() {
        return handle_search_key(state, key);
    }
    if state.pending_delete.is_some() {
        return handle_delete_key(client, args, state, key);
    }

    match key.code {
        KeyCode::BackTab if state.pending_delete.is_none() => {
            state.focus_previous_pane();
            state.status = format!("Focused {} pane.", state.focus_label());
        }
        KeyCode::Tab if state.pending_delete.is_none() => {
            state.focus_next_pane();
            state.status = format!("Focused {} pane.", state.focus_label());
        }
        KeyCode::Up if state.pending_delete.is_none() => {
            if state.focus == PaneFocus::List {
                state.move_selection(-1);
                state.detail_scroll = 0;
            } else {
                state.detail_scroll = state.detail_scroll.saturating_sub(1);
            }
        }
        KeyCode::Down if state.pending_delete.is_none() => {
            if state.focus == PaneFocus::List {
                state.move_selection(1);
                state.detail_scroll = 0;
            } else {
                state.detail_scroll = state.detail_scroll.saturating_add(1);
            }
        }
        KeyCode::Home if state.pending_delete.is_none() => {
            if state.focus == PaneFocus::List {
                state.select_first();
                state.detail_scroll = 0;
            } else {
                state.detail_scroll = 0;
            }
        }
        KeyCode::End if state.pending_delete.is_none() => {
            if state.focus == PaneFocus::List {
                state.select_last();
                state.detail_scroll = 0;
            } else {
                state.detail_scroll = u16::MAX.saturating_sub(32);
            }
        }
        KeyCode::PageUp => state.detail_scroll = state.detail_scroll.saturating_sub(10),
        KeyCode::PageDown => state.detail_scroll = state.detail_scroll.saturating_add(10),
        KeyCode::Char('l') => refresh_browser_document(client, args, state)?,
        KeyCode::Char('/') => state.start_search(SearchDirection::Forward),
        KeyCode::Char('?') => state.start_search(SearchDirection::Backward),
        KeyCode::Char('n') => repeat_search(state),
        KeyCode::Char('e') => start_modify_selected(state)?,
        KeyCode::Char('d') => start_delete_selected(state)?,
        KeyCode::Esc | KeyCode::Char('q') => return Ok(BrowserLoopAction::Exit),
        _ => {}
    }

    Ok(BrowserLoopAction::Continue)
}

fn handle_search_key(state: &mut BrowserState, key: &KeyEvent) -> Result<BrowserLoopAction> {
    let mut search = state
        .pending_search
        .take()
        .ok_or_else(|| message("Datasource browse search state is missing."))?;
    match key.code {
        KeyCode::Esc if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.status = "Cancelled datasource search.".to_string();
        }
        KeyCode::Enter => {
            let query = search.query.trim().to_string();
            if query.is_empty() {
                state.status = "Search query is empty.".to_string();
            } else if let Some(index) = state.find_match(&query, search.direction) {
                state.select_index(index);
                state.last_search = Some(SearchState {
                    direction: search.direction,
                    query: query.clone(),
                });
                state.status = format!("Matched '{query}' at row {}.", index + 1);
            } else {
                state.status = format!("No datasource or org matched '{query}'.");
                state.last_search = Some(SearchState {
                    direction: search.direction,
                    query,
                });
            }
        }
        KeyCode::Backspace => {
            search.query.pop();
            state.pending_search = Some(search);
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            search.query.push(ch);
            state.pending_search = Some(search);
        }
        _ => {
            state.pending_search = Some(search);
        }
    }
    Ok(BrowserLoopAction::Continue)
}

fn handle_edit_key(
    client: &JsonHttpClient,
    args: &DatasourceBrowseArgs,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<BrowserLoopAction> {
    let mut edit_state = state
        .pending_edit
        .take()
        .ok_or_else(|| message("Datasource browse edit state is missing."))?;
    let action = edit_state.handle_key(key)?;
    match action {
        EditDialogAction::None => {}
        EditDialogAction::Cancel => {
            state.status = "Cancelled datasource edit.".to_string();
        }
        EditDialogAction::Save => return save_edit(client, args, state, edit_state),
    }
    if matches!(action, EditDialogAction::None) {
        state.pending_edit = Some(edit_state);
    }
    Ok(BrowserLoopAction::Continue)
}

fn handle_delete_key(
    client: &JsonHttpClient,
    args: &DatasourceBrowseArgs,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<BrowserLoopAction> {
    match key.code {
        KeyCode::Char('y') => confirm_delete(client, args, state)?,
        KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
            state.pending_delete = None;
            state.status = "Cancelled datasource delete.".to_string();
        }
        _ => {}
    }
    Ok(BrowserLoopAction::Continue)
}

fn refresh_browser_document(
    client: &JsonHttpClient,
    args: &DatasourceBrowseArgs,
    state: &mut BrowserState,
) -> Result<()> {
    let document = load_datasource_browse_document(client, args)?;
    state.replace_document(document);
    state.status = "Refreshed datasource browser from live Grafana.".to_string();
    Ok(())
}

fn repeat_search(state: &mut BrowserState) {
    let Some(search) = state.last_search.clone() else {
        state.status = "No previous datasource search. Use / or ? first.".to_string();
        return;
    };
    if let Some(index) = state.repeat_last_search() {
        state.select_index(index);
        state.status = format!("Next match for '{}' at row {}.", search.query, index + 1);
    } else {
        state.status = format!("No more matches for '{}'.", search.query);
    }
}

fn start_modify_selected(state: &mut BrowserState) -> Result<()> {
    let item = state
        .selected_item()
        .ok_or_else(|| message("Datasource browse has no selected datasource to modify."))?
        .clone();
    if item.is_org_row() {
        state.status = format!("Select a datasource row under org {} to edit.", item.org_id);
        return Ok(());
    }
    state.pending_edit = Some(EditDialogState::new(&item));
    state.status = format!("Editing datasource {}.", item.uid);
    Ok(())
}

fn start_delete_selected(state: &mut BrowserState) -> Result<()> {
    let item = state
        .selected_item()
        .ok_or_else(|| message("Datasource browse has no selected datasource to delete."))?
        .clone();
    if item.is_org_row() {
        state.status = format!(
            "Select a datasource row under org {} to delete.",
            item.org_id
        );
        return Ok(());
    }
    state.pending_delete = Some(PendingDelete {
        uid: item.uid.clone(),
        name: item.name.clone(),
        id: item.id,
    });
    state.status = format!("Previewing datasource delete for {}.", item.uid);
    Ok(())
}

fn save_edit(
    client: &JsonHttpClient,
    args: &DatasourceBrowseArgs,
    state: &mut BrowserState,
    edit_state: EditDialogState,
) -> Result<BrowserLoopAction> {
    let selected = state
        .selected_item()
        .ok_or_else(|| message("Datasource browse lost the selected datasource."))?
        .clone();
    let updates = build_modify_updates_from_browse(
        &selected,
        &edit_state.name,
        &edit_state.url,
        &edit_state.access,
        edit_state.is_default,
    );
    if updates.is_empty() {
        state.status = format!("No datasource changes detected for {}.", selected.uid);
        return Ok(BrowserLoopAction::Continue);
    }
    let existing = fetch_datasource_by_uid(client, &selected.uid)?;
    let payload = super::build_modify_payload(&existing, &updates)?;
    let target_id = existing
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Datasource browse edit requires a live datasource id."))?;
    client.request_json(
        Method::PUT,
        &format!("/api/datasources/{target_id}"),
        &[],
        Some(&payload),
    )?;
    let document = load_datasource_browse_document(client, args)?;
    state.replace_document(document);
    state.status = format!("Updated datasource {}.", selected.uid);
    Ok(BrowserLoopAction::Continue)
}

fn confirm_delete(
    client: &JsonHttpClient,
    args: &DatasourceBrowseArgs,
    state: &mut BrowserState,
) -> Result<()> {
    let pending = state
        .pending_delete
        .take()
        .ok_or_else(|| message("Datasource browse delete preview is missing."))?;
    client.request_json(
        Method::DELETE,
        &format!("/api/datasources/{}", pending.id),
        &[],
        None,
    )?;
    let document = load_datasource_browse_document(client, args)?;
    state.replace_document(document);
    state.status = format!("Deleted datasource {}.", pending.uid);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::datasource_browse_support::DatasourceBrowseDocument;
    use super::*;

    fn empty_document() -> DatasourceBrowseDocument {
        DatasourceBrowseDocument {
            scope_label: "current-org".to_string(),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
            items: Vec::new(),
            org_count: 1,
            datasource_count: 0,
        }
    }

    #[test]
    fn search_prompt_treats_q_as_query_text() {
        let mut state = BrowserState::new(empty_document());
        state.start_search(SearchDirection::Forward);

        let action = handle_search_key(
            &mut state,
            &KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        )
        .expect("search key should succeed");

        assert!(matches!(action, BrowserLoopAction::Continue));
        assert_eq!(
            state
                .pending_search
                .as_ref()
                .map(|search| search.query.as_str()),
            Some("q")
        );
    }
}
