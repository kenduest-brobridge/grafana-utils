#[cfg(feature = "tui")]
use std::io::{self, Stdout};

#[cfg(feature = "tui")]
use crossterm::execute;
#[cfg(feature = "tui")]
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
#[cfg(feature = "tui")]
use ratatui::backend::CrosstermBackend;
#[cfg(feature = "tui")]
use ratatui::Terminal;

#[cfg(feature = "tui")]
use crate::common::Result;

#[cfg(feature = "tui")]
pub(super) struct TerminalSession {
    pub(super) terminal: Terminal<CrosstermBackend<Stdout>>,
}

#[cfg(feature = "tui")]
impl TerminalSession {
    pub(super) fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        Ok(Self {
            terminal: Terminal::new(CrosstermBackend::new(stdout))?,
        })
    }
}

#[cfg(feature = "tui")]
impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
