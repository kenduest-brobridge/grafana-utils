#![cfg(feature = "tui")]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::Value;

use crate::common::{message, Result};

use crate::dashboard::browse_state::{BrowserState, SearchState};
use crate::dashboard::BrowseArgs;

pub(super) fn handle_search_dialog_key(state: &mut BrowserState, key: &KeyEvent) -> Result<()> {
    let mut search = state
        .pending_search
        .take()
        .ok_or_else(|| message("Dashboard browse search state is missing."))?;
    match key.code {
        KeyCode::Esc if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.status = "Cancelled dashboard search.".to_string();
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
                state.status = format!("Matched '{query}' at tree row {}.", index + 1);
            } else {
                state.last_search = Some(SearchState {
                    direction: search.direction,
                    query: query.clone(),
                });
                state.status = format!("No org, folder, or dashboard matched '{query}'.");
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
    Ok(())
}

pub(super) fn repeat_search<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(search) = state.last_search.clone() else {
        state.status = "No previous dashboard search. Use / or ? first.".to_string();
        return Ok(());
    };
    if let Some(index) = state.repeat_last_search() {
        state.select_index(index);
        super::ensure_selected_dashboard_view(request_json, args, state, false)?;
        state.status = format!(
            "Next match for '{}' at tree row {}.",
            search.query,
            index + 1
        );
    } else {
        state.status = format!("No more matches for '{}'.", search.query);
    }
    Ok(())
}
