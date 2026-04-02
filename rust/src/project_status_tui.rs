#![cfg(any(feature = "tui", test))]
#![cfg_attr(test, allow(dead_code))]

#[path = "project_status_tui_render.rs"]
mod project_status_tui_render;

use crate::project_status::{
    ProjectDomainStatus, ProjectStatus, ProjectStatusAction, ProjectStatusFinding,
    PROJECT_STATUS_BLOCKED,
};
#[cfg(feature = "tui")]
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
#[cfg(feature = "tui")]
use crossterm::execute;
#[cfg(feature = "tui")]
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
#[cfg(feature = "tui")]
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
#[cfg(feature = "tui")]
use ratatui::Terminal;
#[cfg(feature = "tui")]
use std::io::{self, Stdout};
#[cfg(feature = "tui")]
use std::time::Duration;

#[cfg(feature = "tui")]
use crate::common::Result;

pub(crate) use project_status_tui_render::render_project_status_frame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectStatusPane {
    Home,
    Domains,
    Details,
    Actions,
}

pub(crate) struct ProjectStatusTuiState {
    document: ProjectStatus,
    domain_state: ListState,
    action_state: ListState,
    focus: ProjectStatusPane,
    detail_scroll: u16,
}

#[cfg(feature = "tui")]
struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

#[cfg(feature = "tui")]
impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
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

impl ProjectStatusTuiState {
    pub(crate) fn new(document: ProjectStatus) -> Self {
        let mut domain_state = ListState::default();
        domain_state.select((!document.domains.is_empty()).then_some(0));
        let mut action_state = ListState::default();
        action_state.select((!document.next_actions.is_empty()).then_some(0));
        let mut state = Self {
            document,
            domain_state,
            action_state,
            focus: ProjectStatusPane::Home,
            detail_scroll: 0,
        };
        state.sync_action_selection();
        state
    }

    pub(crate) fn document(&self) -> &ProjectStatus {
        &self.document
    }

    pub(crate) fn focus(&self) -> ProjectStatusPane {
        self.focus
    }

    pub(crate) fn detail_scroll(&self) -> u16 {
        self.detail_scroll
    }

    pub(crate) fn domain_state_mut(&mut self) -> &mut ListState {
        &mut self.domain_state
    }

    pub(crate) fn action_state_mut(&mut self) -> &mut ListState {
        &mut self.action_state
    }

    pub(crate) fn current_domain_index(&self) -> Option<usize> {
        self.domain_state.selected()
    }

    pub(crate) fn current_action_index(&self) -> Option<usize> {
        self.action_state.selected()
    }

    pub(crate) fn current_domain(&self) -> Option<&ProjectDomainStatus> {
        self.current_domain_index()
            .and_then(|index| self.document.domains.get(index))
    }

    pub(crate) fn current_action(&self) -> Option<&ProjectStatusAction> {
        self.current_action_index()
            .and_then(|index| self.document.next_actions.get(index))
    }

    pub(crate) fn project_home_target_domain_index(&self) -> Option<usize> {
        self.document
            .domains
            .iter()
            .enumerate()
            .find_map(|(index, domain)| {
                if domain.status == PROJECT_STATUS_BLOCKED || domain.blocker_count > 0 {
                    Some(index)
                } else {
                    None
                }
            })
            .or_else(|| {
                self.document
                    .domains
                    .iter()
                    .enumerate()
                    .find_map(|(index, domain)| {
                        if !domain.next_actions.is_empty() {
                            Some(index)
                        } else {
                            None
                        }
                    })
            })
            .or_else(|| (!self.document.domains.is_empty()).then_some(0))
    }

    pub(crate) fn project_home_target_domain_label(&self) -> Option<String> {
        self.project_home_target_domain_index().and_then(|index| {
            self.document
                .domains
                .get(index)
                .map(|domain| domain.id.clone())
        })
    }

    pub(crate) fn project_home_target_action(&self) -> Option<&ProjectStatusAction> {
        let target_domain = self.project_home_target_domain_index()?;
        let domain_id = &self.document.domains.get(target_domain)?.id;
        self.document
            .next_actions
            .iter()
            .find(|action| &action.domain == domain_id)
            .or_else(|| self.document.next_actions.first())
    }

    pub(crate) fn project_home_target_action_label(&self) -> Option<String> {
        self.project_home_target_action()
            .map(|action| format!("{} -> {}", action.domain, action.action))
    }

    pub(crate) fn project_home_top_blocker_label(&self) -> Option<String> {
        let target_domain = self.project_home_target_domain_index()?;
        let domain = self.document.domains.get(target_domain)?;
        let blocker = domain.blockers.first()?;
        Some(format!(
            "{}: {}={} from {} (blockers={})",
            domain.id, blocker.kind, blocker.count, blocker.source, domain.blocker_count
        ))
    }

    pub(crate) fn home_lines(&self) -> Vec<String> {
        let overall = &self.document.overall;
        let mut lines = vec![
            "Project Home: review the top blocker, then hand off to the recommended domain and matching action.".to_string(),
            format!(
                "Overall: status={} scope={} domains={} present={} blocked={} blockers={} warnings={}",
                overall.status,
                self.document.scope,
                overall.domain_count,
                overall.present_count,
                overall.blocked_count,
                overall.blocker_count,
                overall.warning_count
            ),
            match self.project_home_target_domain_label() {
                Some(label) => format!(
                    "Recommended handoff domain: {label} | Project Home -> Domains -> Actions"
                ),
                None => "Recommended handoff domain: none | Project Home -> Domains -> Actions"
                    .to_string(),
            },
        ];

        lines.push(match self.project_home_top_blocker_label() {
            Some(label) => format!("Current top blocker: {label}"),
            None => "Current top blocker: none".to_string(),
        });

        lines.push(match self.project_home_target_action() {
            Some(action) => format!(
                "Recommended handoff action: {} -> {} reason={} | Project Home -> Domains -> Actions",
                action.domain, action.action, action.reason_code
            ),
            None => "Recommended handoff action: none | Project Home -> Domains -> Actions"
                .to_string(),
        });

        lines.push(
            "Navigation: Enter hands off from Home to the recommended domain and preselects the matching action. Path: Home -> Domains -> Actions.".to_string(),
        );
        lines.push(format!(
            "Domains: {}",
            self.document
                .domains
                .iter()
                .map(|domain| format!("{}={}", domain.id, domain.status))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
        lines
    }

    pub(crate) fn current_domain_lines(&self) -> Vec<String> {
        self.current_domain()
            .map(|domain| {
                let mut lines = vec![
                    format!("Domain: {}", domain.id),
                    format!("Scope: {}   Mode: {}", domain.scope, domain.mode),
                    format!(
                        "Status: {} reason={} primary={} blockers={} warnings={}",
                        domain.status,
                        domain.reason_code,
                        domain.primary_count,
                        domain.blocker_count,
                        domain.warning_count
                    ),
                    format!("Sources: {}", join_or_none(&domain.source_kinds)),
                    format!("Signals: {}", join_or_none(&domain.signal_keys)),
                    format!(
                        "Freshness: status={} sources={} newest={} oldest={}",
                        domain.freshness.status,
                        domain.freshness.source_count,
                        optional_age(domain.freshness.newest_age_seconds),
                        optional_age(domain.freshness.oldest_age_seconds)
                    ),
                ];
                lines.push(render_finding_block("Blockers", &domain.blockers));
                lines.push(render_finding_block("Warnings", &domain.warnings));
                if domain.next_actions.is_empty() {
                    lines.push("Next actions: none".to_string());
                } else {
                    lines.push("Next actions:".to_string());
                    lines.extend(
                        domain
                            .next_actions
                            .iter()
                            .map(|action| format!("- {action}")),
                    );
                }
                lines
            })
            .unwrap_or_else(|| vec!["No domain selected.".to_string()])
    }

    pub(crate) fn status_line(&self) -> String {
        let focus = match self.focus {
            ProjectStatusPane::Home => "Home",
            ProjectStatusPane::Domains => "Domains",
            ProjectStatusPane::Details => "Details",
            ProjectStatusPane::Actions => "Actions",
        };
        let domain = self
            .current_domain()
            .map(|domain| domain.id.as_str())
            .unwrap_or("No domain");
        let action = self
            .current_action()
            .map(|action| action.action.as_str())
            .unwrap_or("No action");
        let handoff = self
            .project_home_target_action_label()
            .or_else(|| self.project_home_target_domain_label())
            .map(|label| format!("   Home handoff: {label} | Home -> Domains -> Actions"))
            .unwrap_or_default();
        format!(
            "Focus {focus}{handoff}   Domain {}/{}: {}   Action {}/{}: {}",
            self.domain_state
                .selected()
                .map(|index| index + 1)
                .unwrap_or(0),
            self.document.domains.len(),
            domain,
            self.action_state
                .selected()
                .map(|index| index + 1)
                .unwrap_or(0),
            self.document.next_actions.len(),
            action
        )
    }

    pub(crate) fn focus_next(&mut self) {
        self.focus = match self.focus {
            ProjectStatusPane::Home => ProjectStatusPane::Domains,
            ProjectStatusPane::Domains => ProjectStatusPane::Details,
            ProjectStatusPane::Details => ProjectStatusPane::Actions,
            ProjectStatusPane::Actions => ProjectStatusPane::Home,
        };
    }

    pub(crate) fn focus_previous(&mut self) {
        self.focus = match self.focus {
            ProjectStatusPane::Home => ProjectStatusPane::Actions,
            ProjectStatusPane::Domains => ProjectStatusPane::Home,
            ProjectStatusPane::Details => ProjectStatusPane::Domains,
            ProjectStatusPane::Actions => ProjectStatusPane::Details,
        };
    }

    pub(crate) fn focus_home(&mut self) {
        self.focus = ProjectStatusPane::Home;
    }

    pub(crate) fn handoff_from_home(&mut self) {
        let Some(index) = self.project_home_target_domain_index() else {
            return;
        };
        self.select_domain(index);
        self.focus = ProjectStatusPane::Domains;
    }

    pub(crate) fn move_domain_selection(&mut self, delta: isize) {
        let count = self.document.domains.len();
        if count == 0 {
            self.domain_state.select(None);
            self.action_state.select(None);
            self.detail_scroll = 0;
            return;
        }
        let current = self.domain_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, count.saturating_sub(1) as isize) as usize;
        self.select_domain(next);
    }

    pub(crate) fn move_action_selection(&mut self, delta: isize) {
        let count = self.document.next_actions.len();
        if count == 0 {
            self.action_state.select(None);
            self.detail_scroll = 0;
            return;
        }
        let current = self.action_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, count.saturating_sub(1) as isize) as usize;
        self.action_state.select(Some(next));
        self.detail_scroll = 0;
    }

    pub(crate) fn move_detail_scroll(&mut self, delta: isize) {
        if delta.is_negative() {
            self.detail_scroll = self
                .detail_scroll
                .saturating_sub(delta.unsigned_abs() as u16);
        } else {
            self.detail_scroll = self.detail_scroll.saturating_add(delta as u16);
        }
    }

    fn select_domain(&mut self, index: usize) {
        self.domain_state.select(Some(index));
        self.sync_action_selection();
        self.detail_scroll = 0;
    }

    fn sync_action_selection(&mut self) {
        if self.document.next_actions.is_empty() {
            self.action_state.select(None);
            return;
        }
        let selected_domain = self.current_domain().map(|domain| domain.id.as_str());
        let action_index = selected_domain.and_then(|domain_id| {
            self.document
                .next_actions
                .iter()
                .position(|action| action.domain == domain_id)
        });
        self.action_state.select(Some(action_index.unwrap_or(0)));
    }
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn optional_age(value: Option<u64>) -> String {
    value
        .map(|seconds| seconds.to_string())
        .unwrap_or_else(|| "n/a".to_string())
}

fn render_finding_block(label: &str, findings: &[ProjectStatusFinding]) -> String {
    if findings.is_empty() {
        format!("{label}: none")
    } else {
        let rendered = findings
            .iter()
            .map(|finding| format!("{}={} from {}", finding.kind, finding.count, finding.source))
            .collect::<Vec<_>>()
            .join(" | ");
        format!("{label}: {rendered}")
    }
}

#[cfg(feature = "tui")]
pub(crate) fn run_project_status_interactive(document: ProjectStatus) -> Result<()> {
    let mut session = TerminalSession::enter()?;
    let mut state = ProjectStatusTuiState::new(document);

    loop {
        session
            .terminal
            .draw(|frame| render_project_status_frame(frame, &mut state))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let detail_lines_len = state.current_domain_lines().len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Tab => state.focus_next(),
            KeyCode::BackTab => state.focus_previous(),
            KeyCode::Char('h') => state.focus_home(),
            KeyCode::Enter => {
                if state.focus() == ProjectStatusPane::Home {
                    state.handoff_from_home();
                }
            }
            KeyCode::Up => match state.focus() {
                ProjectStatusPane::Home => {}
                ProjectStatusPane::Domains => state.move_domain_selection(-1),
                ProjectStatusPane::Details => state.move_detail_scroll(-1),
                ProjectStatusPane::Actions => state.move_action_selection(-1),
            },
            KeyCode::Down => match state.focus() {
                ProjectStatusPane::Home => {}
                ProjectStatusPane::Domains => state.move_domain_selection(1),
                ProjectStatusPane::Details => state.move_detail_scroll(1),
                ProjectStatusPane::Actions => state.move_action_selection(1),
            },
            KeyCode::PageUp => {
                if state.focus() == ProjectStatusPane::Details {
                    state.move_detail_scroll(-10);
                }
            }
            KeyCode::PageDown => {
                if state.focus() == ProjectStatusPane::Details {
                    state.move_detail_scroll(10);
                }
            }
            KeyCode::Home => match state.focus() {
                ProjectStatusPane::Home => {}
                ProjectStatusPane::Domains => {
                    let current = state.current_domain_index().unwrap_or(0) as isize;
                    state.move_domain_selection(-current);
                }
                ProjectStatusPane::Details => state.detail_scroll = 0,
                ProjectStatusPane::Actions => {
                    let current = state.current_action_index().unwrap_or(0) as isize;
                    state.move_action_selection(-current);
                }
            },
            KeyCode::End => match state.focus() {
                ProjectStatusPane::Home => {}
                ProjectStatusPane::Domains => {
                    let len = state.document().domains.len();
                    if len > 0 {
                        let current = state.current_domain_index().unwrap_or(0) as isize;
                        state.move_domain_selection(len.saturating_sub(1) as isize - current);
                    }
                }
                ProjectStatusPane::Details => {
                    state.detail_scroll = detail_lines_len.saturating_sub(1) as u16;
                }
                ProjectStatusPane::Actions => {
                    let len = state.document().next_actions.len();
                    if len > 0 {
                        let current = state.current_action_index().unwrap_or(0) as isize;
                        state.move_action_selection(len.saturating_sub(1) as isize - current);
                    }
                }
            },
            _ => {}
        }
    }
}
