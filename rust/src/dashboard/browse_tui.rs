use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;

use super::browse_actions::refresh_browser_document;
use super::browse_input::{ensure_selected_dashboard_view, handle_browser_key, BrowserLoopAction};
use super::browse_render::render_dashboard_browser_frame;
use super::browse_state::BrowserState;
use super::browse_terminal::TerminalSession;
use super::BrowseArgs;

pub(crate) fn run_dashboard_browser_tui<F>(mut request_json: F, args: &BrowseArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let document = refresh_browser_document(&mut request_json, args)?;
    let mut session = TerminalSession::enter()?;
    let mut state = BrowserState::new(document);
    ensure_selected_dashboard_view(&mut request_json, &mut state, false)?;

    loop {
        session
            .terminal
            .draw(|frame| render_dashboard_browser_frame(frame, &mut state))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if let BrowserLoopAction::Exit =
            handle_browser_key(&mut request_json, args, &mut session, &mut state, &key)?
        {
            break;
        }
    }

    Ok(state.document.summary.dashboard_count)
}
