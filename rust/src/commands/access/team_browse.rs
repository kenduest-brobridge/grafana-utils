//! Interactive TUI browser for Grafana teams with edit/delete support.

#[cfg(feature = "tui")]
use std::time::Duration;

#[cfg(feature = "tui")]
use crossterm::event::{self, Event, KeyEventKind};
use reqwest::Method;
use serde_json::Value;

use crate::access::TeamBrowseArgs;
use crate::common::Result;

#[cfg(feature = "tui")]
use super::browse_support::default_user_browse_args_from_team;
use super::browse_support::BrowseSwitch;
#[cfg(feature = "tui")]
use super::browse_terminal::TerminalSession;
#[cfg(feature = "tui")]
#[path = "team_browse_actions.rs"]
mod team_browse_actions;
#[cfg(feature = "tui")]
#[path = "team_browse_confirmation.rs"]
mod team_browse_confirmation;
#[cfg(feature = "tui")]
#[path = "team_browse_dialog.rs"]
mod team_browse_dialog;
#[cfg(feature = "tui")]
#[path = "team_browse_dispatch.rs"]
mod team_browse_dispatch;
#[cfg(feature = "tui")]
#[path = "team_browse_input.rs"]
mod team_browse_input;
#[cfg(feature = "tui")]
#[path = "team_browse_reload.rs"]
mod team_browse_reload;
#[cfg(feature = "tui")]
#[path = "team_browse_render.rs"]
mod team_browse_render;
#[cfg(feature = "tui")]
#[path = "team_browse_state.rs"]
mod team_browse_state;
#[cfg(feature = "tui")]
use team_browse_input::{handle_key, load_rows};
#[cfg(feature = "tui")]
use team_browse_render::render_frame;
#[cfg(feature = "tui")]
use team_browse_state::BrowserState;

#[cfg(feature = "tui")]
#[allow(dead_code)]
pub(crate) fn browse_teams_with_request<F>(
    mut request_json: F,
    args: &TeamBrowseArgs,
) -> Result<BrowseSwitch>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut session = TerminalSession::enter()?;
    browse_teams_in_session(&mut session, &mut request_json, args)
}

#[cfg(feature = "tui")]
pub(super) fn browse_teams_in_session<F>(
    session: &mut TerminalSession,
    mut request_json: F,
    args: &TeamBrowseArgs,
) -> Result<BrowseSwitch>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let rows = load_rows(&mut request_json, args)?;
    let mut state = BrowserState::new(rows);

    loop {
        session
            .terminal
            .draw(|frame| render_frame(frame, &mut state, args))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match handle_key(&mut request_json, args, &mut state, &key)? {
            team_browse_input::BrowseAction::Continue => {}
            team_browse_input::BrowseAction::Exit => return Ok(BrowseSwitch::Exit),
            team_browse_input::BrowseAction::JumpToUser => {
                return Ok(BrowseSwitch::ToUser(default_user_browse_args_from_team(
                    args,
                )))
            }
        }
    }
}

#[cfg(not(feature = "tui"))]
pub(crate) fn browse_teams_with_request<F>(
    _request_json: F,
    _args: &TeamBrowseArgs,
) -> Result<BrowseSwitch>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    Err(crate::common::tui(
        "Access team browse requires the `tui` feature.",
    ))
}
