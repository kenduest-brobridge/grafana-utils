#![cfg(feature = "tui")]

use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};

use crate::common::Result;
use crate::http::JsonHttpClient;

use super::datasource_browse_input::{handle_browser_key, BrowserLoopAction};
use super::datasource_browse_render::render_datasource_browser_frame;
use super::datasource_browse_state::BrowserState;
use super::datasource_browse_support::load_datasource_browse_document;
use super::datasource_browse_terminal::TerminalSession;
use super::DatasourceBrowseArgs;

pub(crate) fn run_datasource_browser_tui(
    client: &JsonHttpClient,
    args: &DatasourceBrowseArgs,
) -> Result<usize> {
    let document = load_datasource_browse_document(client, args)?;
    let mut session = TerminalSession::enter()?;
    let mut state = BrowserState::new(document);

    loop {
        session
            .terminal
            .draw(|frame| render_datasource_browser_frame(frame, &mut state))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if let BrowserLoopAction::Exit = handle_browser_key(client, args, &mut state, &key)? {
            break;
        }
    }

    Ok(state.document.items.len())
}
