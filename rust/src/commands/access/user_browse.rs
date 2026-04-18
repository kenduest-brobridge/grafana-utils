//! Interactive TUI browser for Grafana users with edit/delete support.

#[cfg(feature = "tui")]
use std::time::Duration;

#[cfg(feature = "tui")]
use crossterm::event::{self, Event, KeyEventKind};
use reqwest::Method;
use serde_json::Value;

use crate::access::UserBrowseArgs;
use crate::common::Result;

#[cfg(feature = "tui")]
use super::browse_support::default_team_browse_args_from_user;
use super::browse_support::BrowseSwitch;
#[cfg(feature = "tui")]
use super::browse_terminal::TerminalSession;
#[cfg(feature = "tui")]
#[path = "user_browse_dialog.rs"]
mod user_browse_dialog;
#[cfg(feature = "tui")]
#[path = "user_browse_input.rs"]
mod user_browse_input;
#[cfg(feature = "tui")]
#[path = "user_browse_mutation.rs"]
mod user_browse_mutation;
#[cfg(feature = "tui")]
#[path = "user_browse_render.rs"]
mod user_browse_render;
#[cfg(feature = "tui")]
#[path = "user_browse_state.rs"]
mod user_browse_state;
#[cfg(feature = "tui")]
use user_browse_input::{handle_key, load_rows};
#[cfg(feature = "tui")]
use user_browse_render::render_frame;
#[cfg(feature = "tui")]
use user_browse_state::{BrowserState, DisplayMode};

#[cfg(feature = "tui")]
#[allow(dead_code)]
pub(crate) fn browse_users_with_request<F>(
    mut request_json: F,
    args: &UserBrowseArgs,
) -> Result<BrowseSwitch>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut session = TerminalSession::enter()?;
    browse_users_in_session(&mut session, &mut request_json, args)
}

#[cfg(feature = "tui")]
pub(super) fn browse_users_in_session<F>(
    session: &mut TerminalSession,
    mut request_json: F,
    args: &UserBrowseArgs,
) -> Result<BrowseSwitch>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let display_mode = if args.input_dir.is_some() {
        DisplayMode::GlobalAccounts
    } else if args.all_orgs {
        DisplayMode::OrgMemberships
    } else {
        DisplayMode::GlobalAccounts
    };
    let rows = load_rows(&mut request_json, args, display_mode)?;
    let mut state = BrowserState::new(rows, display_mode);

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
            user_browse_input::BrowseAction::Continue => {}
            user_browse_input::BrowseAction::Exit => return Ok(BrowseSwitch::Exit),
            user_browse_input::BrowseAction::JumpToTeam => {
                return Ok(BrowseSwitch::ToTeam(default_team_browse_args_from_user(
                    args,
                )))
            }
        }
    }
}

#[cfg(not(feature = "tui"))]
pub(crate) fn browse_users_with_request<F>(
    _request_json: F,
    _args: &UserBrowseArgs,
) -> Result<BrowseSwitch>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    Err(crate::common::tui(
        "Access user browse requires the `tui` feature.",
    ))
}
