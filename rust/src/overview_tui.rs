#![cfg(feature = "tui")]
#![cfg_attr(test, allow(dead_code))]

#[path = "overview_tui_render.rs"]
mod overview_tui_render;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;

use crate::common::Result;

use super::{OverviewDocument, OverviewSection, OverviewSectionItem, OverviewSectionView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverviewPane {
    ProjectHome,
    Sections,
    Views,
    Items,
    Details,
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

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

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

struct OverviewWorkbenchState {
    document: OverviewDocument,
    section_state: ListState,
    view_state: ListState,
    item_state: ListState,
    focus: OverviewPane,
    detail_scroll: u16,
    section_view_indexes: Vec<usize>,
}

impl OverviewWorkbenchState {
    fn new(document: OverviewDocument) -> Self {
        let mut section_state = ListState::default();
        let selected_section = document
            .selected_section_index
            .min(document.sections.len().saturating_sub(1));
        section_state.select((!document.sections.is_empty()).then_some(selected_section));
        let mut state = Self {
            section_view_indexes: vec![0; document.sections.len()],
            document,
            section_state,
            view_state: ListState::default(),
            item_state: ListState::default(),
            focus: OverviewPane::ProjectHome,
            detail_scroll: 0,
        };
        state.sync_view_selection();
        state.reset_items();
        state
    }

    fn current_section_index(&self) -> Option<usize> {
        self.section_state.selected()
    }

    fn current_section(&self) -> Option<&OverviewSection> {
        self.current_section_index()
            .and_then(|index| self.document.sections.get(index))
    }

    fn current_view_index(&self) -> Option<usize> {
        self.view_state.selected()
    }

    fn current_view(&self) -> Option<&OverviewSectionView> {
        self.current_section().and_then(|section| {
            self.current_view_index()
                .and_then(|index| section.views.get(index))
        })
    }

    fn current_view_label(&self) -> String {
        self.current_view()
            .map(|view| view.label.clone())
            .unwrap_or_else(|| "No view selected".to_string())
    }

    fn section_index_for_domain(&self, domain_id: &str) -> Option<usize> {
        let matches_kind = |predicate: fn(&str) -> bool| {
            self.document
                .sections
                .iter()
                .position(|section| predicate(section.kind.as_str()))
        };
        match domain_id {
            "dashboard" => matches_kind(|kind| kind == "dashboard-export"),
            "datasource" => matches_kind(|kind| kind == "datasource-export"),
            "alert" => matches_kind(|kind| kind == "alert-export"),
            "access" => matches_kind(|kind| kind.starts_with("grafana-utils-access-")),
            "sync" => matches_kind(|kind| kind == "sync-summary" || kind == "bundle-preflight"),
            "promotion" => matches_kind(|kind| kind == "promotion-preflight"),
            _ => None,
        }
    }

    fn project_home_target_section_index(&self) -> Option<usize> {
        self.document
            .project_status
            .domains
            .iter()
            .find_map(|domain| {
                let actionable = domain.blocker_count > 0 || domain.status == "blocked";
                if actionable {
                    self.section_index_for_domain(&domain.id)
                } else {
                    None
                }
            })
            .or_else(|| {
                self.document
                    .project_status
                    .domains
                    .iter()
                    .find_map(|domain| {
                        if !domain.next_actions.is_empty() {
                            self.section_index_for_domain(&domain.id)
                        } else {
                            None
                        }
                    })
            })
            .or_else(|| {
                if self.document.sections.is_empty() {
                    None
                } else {
                    Some(
                        self.section_state.selected().unwrap_or(
                            self.document
                                .selected_section_index
                                .min(self.document.sections.len().saturating_sub(1)),
                        ),
                    )
                }
            })
    }

    fn project_home_target_label(&self) -> Option<String> {
        self.project_home_target_section_index().and_then(|index| {
            self.document
                .sections
                .get(index)
                .map(|section| section.label.clone())
        })
    }

    fn project_home_domain(&self) -> Option<&crate::project_status::ProjectDomainStatus> {
        self.document.project_status.domains.iter().find(|domain| {
            domain.blocker_count > 0
                || domain.status == "blocked"
                || !domain.next_actions.is_empty()
        })
    }

    fn project_home_lines(&self) -> Vec<String> {
        let overall = &self.document.project_status.overall;
        let mut lines = vec![
            format!(
                "Overall: status={} scope={} domains={} present={} blocked={} blockers={} warnings={}",
                overall.status,
                self.document.project_status.scope,
                overall.domain_count,
                overall.present_count,
                overall.blocked_count,
                overall.blocker_count,
                overall.warning_count
            ),
            match self.project_home_target_label() {
                Some(label) => format!(
                    "Recommended handoff section: {label} | Project Home -> Sections -> Views -> Items -> Details"
                ),
                None => {
                    "Recommended handoff section: none | Project Home -> Sections -> Views -> Items -> Details"
                        .to_string()
                }
            },
        ];

        if let Some(domain) = self.project_home_domain() {
            let mut line = format!(
                "Top action: {} status={} reason={} primary={} blockers={} warnings={}",
                domain.id,
                domain.status,
                domain.reason_code,
                domain.primary_count,
                domain.blocker_count,
                domain.warning_count
            );
            if let Some(action) = domain.next_actions.first() {
                line.push_str(&format!(" next={action}"));
            }
            lines.push(line);
        } else {
            lines.push("Top action: no blocked or actionable domains".to_string());
        }

        lines.push(format!(
            "Domains: {}",
            self.document
                .project_status
                .domains
                .iter()
                .map(|domain| format!("{}={}", domain.id, domain.status))
                .collect::<Vec<String>>()
                .join(" | ")
        ));
        lines
    }

    fn status_focus_label(&self) -> &'static str {
        match self.focus {
            OverviewPane::ProjectHome => "Home",
            OverviewPane::Sections => "Sections",
            OverviewPane::Views => "Views",
            OverviewPane::Items => "Items",
            OverviewPane::Details => "Details",
        }
    }

    fn current_items(&self) -> &[OverviewSectionItem] {
        self.current_view()
            .map(|view| view.items.as_slice())
            .unwrap_or(&[])
    }

    fn selected_item(&self) -> Option<&OverviewSectionItem> {
        self.item_state
            .selected()
            .and_then(|index| self.current_items().get(index))
    }

    fn current_detail_lines(&self) -> Vec<String> {
        self.selected_item()
            .map(|item| {
                let mut lines = vec![
                    format!("Kind: {}", item.kind),
                    format!("Title: {}", item.title),
                ];
                if !item.meta.is_empty() {
                    lines.push(format!("Summary: {}", item.meta));
                }
                if !item.facts.is_empty() {
                    lines.push(String::new());
                    lines.extend(
                        item.facts
                            .iter()
                            .map(|fact| format!("{}: {}", fact.label, fact.value)),
                    );
                }
                if !item.details.is_empty() {
                    lines.push(String::new());
                    let summary_line = format!("Summary: {}", item.meta);
                    lines.extend(
                        item.details
                            .iter()
                            .filter(|line| line.as_str() != summary_line)
                            .cloned(),
                    );
                }
                if lines.len() == 2 {
                    lines.push("No detail lines available.".to_string());
                }
                lines
            })
            .unwrap_or_else(|| vec!["No item selected.".to_string()])
    }

    fn sync_view_selection(&mut self) {
        let Some(section_index) = self.current_section_index() else {
            self.view_state.select(None);
            return;
        };
        let Some(section) = self.document.sections.get(section_index) else {
            self.view_state.select(None);
            return;
        };
        if section.views.is_empty() {
            self.view_state.select(None);
            return;
        }
        let view_index = self
            .section_view_indexes
            .get(section_index)
            .copied()
            .unwrap_or(0)
            .min(section.views.len().saturating_sub(1));
        self.view_state.select(Some(view_index));
        if let Some(slot) = self.section_view_indexes.get_mut(section_index) {
            *slot = view_index;
        }
    }

    fn reset_items(&mut self) {
        self.item_state
            .select((!self.current_items().is_empty()).then_some(0));
        self.detail_scroll = 0;
    }

    fn focus_next(&mut self) {
        self.focus = match self.focus {
            OverviewPane::ProjectHome => OverviewPane::Sections,
            OverviewPane::Sections => OverviewPane::Views,
            OverviewPane::Views => OverviewPane::Items,
            OverviewPane::Items => OverviewPane::Details,
            OverviewPane::Details => OverviewPane::ProjectHome,
        };
    }

    fn focus_previous(&mut self) {
        self.focus = match self.focus {
            OverviewPane::ProjectHome => OverviewPane::Details,
            OverviewPane::Sections => OverviewPane::ProjectHome,
            OverviewPane::Views => OverviewPane::Sections,
            OverviewPane::Items => OverviewPane::Views,
            OverviewPane::Details => OverviewPane::Items,
        };
    }

    fn focus_project_home(&mut self) {
        self.focus = OverviewPane::ProjectHome;
    }

    fn handoff_from_home(&mut self) {
        let Some(section_index) = self.project_home_target_section_index() else {
            return;
        };
        self.section_state.select(Some(section_index));
        self.sync_view_selection();
        self.reset_items();
        self.focus = OverviewPane::Sections;
    }

    fn move_section_selection(&mut self, delta: isize) {
        let count = self.document.sections.len();
        if count == 0 {
            self.section_state.select(None);
            self.view_state.select(None);
            self.item_state.select(None);
            self.detail_scroll = 0;
            return;
        }
        let current = self.section_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, count.saturating_sub(1) as isize) as usize;
        self.section_state.select(Some(next));
        self.sync_view_selection();
        self.reset_items();
    }

    fn move_view_selection(&mut self, delta: isize) {
        let Some(section_index) = self.current_section_index() else {
            self.view_state.select(None);
            self.item_state.select(None);
            self.detail_scroll = 0;
            return;
        };
        let Some(section) = self.document.sections.get(section_index) else {
            self.view_state.select(None);
            self.item_state.select(None);
            self.detail_scroll = 0;
            return;
        };
        if section.views.is_empty() {
            self.view_state.select(None);
            self.item_state.select(None);
            self.detail_scroll = 0;
            return;
        }
        let current = self.view_state.selected().unwrap_or(0) as isize;
        let next =
            (current + delta).clamp(0, section.views.len().saturating_sub(1) as isize) as usize;
        self.view_state.select(Some(next));
        if let Some(slot) = self.section_view_indexes.get_mut(section_index) {
            *slot = next;
        }
        self.reset_items();
    }

    fn move_item_selection(&mut self, delta: isize) {
        let count = self.current_items().len();
        if count == 0 {
            self.item_state.select(None);
            self.detail_scroll = 0;
            return;
        }
        let current = self.item_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, count.saturating_sub(1) as isize) as usize;
        self.item_state.select(Some(next));
        self.detail_scroll = 0;
    }

    fn move_detail_scroll(&mut self, delta: isize, total_lines: usize) {
        let max_scroll = total_lines.saturating_sub(1) as u16;
        if delta.is_negative() {
            self.detail_scroll = self
                .detail_scroll
                .saturating_sub(delta.unsigned_abs() as u16);
        } else {
            self.detail_scroll = self.detail_scroll.saturating_add(delta as u16);
        }
        self.detail_scroll = self.detail_scroll.min(max_scroll);
    }
}

pub(crate) fn run_overview_interactive(document: OverviewDocument) -> Result<()> {
    let mut session = TerminalSession::enter()?;
    let mut state = OverviewWorkbenchState::new(document);

    loop {
        session
            .terminal
            .draw(|frame| overview_tui_render::render_overview_frame(frame, &mut state))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let detail_lines_len = state.current_detail_lines().len();
        match key.code {
            KeyCode::Tab => state.focus_next(),
            KeyCode::BackTab => state.focus_previous(),
            KeyCode::Char('h') => state.focus_project_home(),
            KeyCode::Up => match state.focus {
                OverviewPane::ProjectHome => {}
                OverviewPane::Sections => state.move_section_selection(-1),
                OverviewPane::Views => state.move_view_selection(-1),
                OverviewPane::Items => state.move_item_selection(-1),
                OverviewPane::Details => state.move_detail_scroll(-1, detail_lines_len),
            },
            KeyCode::Down => match state.focus {
                OverviewPane::ProjectHome => {}
                OverviewPane::Sections => state.move_section_selection(1),
                OverviewPane::Views => state.move_view_selection(1),
                OverviewPane::Items => state.move_item_selection(1),
                OverviewPane::Details => state.move_detail_scroll(1, detail_lines_len),
            },
            KeyCode::PageUp => {
                if state.focus == OverviewPane::Details {
                    state.move_detail_scroll(-10, detail_lines_len);
                }
            }
            KeyCode::PageDown => {
                if state.focus == OverviewPane::Details {
                    state.move_detail_scroll(10, detail_lines_len);
                }
            }
            KeyCode::Home => {
                match state.focus {
                    OverviewPane::ProjectHome => {}
                    OverviewPane::Sections => state.move_section_selection(
                        -(state.section_state.selected().unwrap_or(0) as isize),
                    ),
                    OverviewPane::Views => state
                        .move_view_selection(-(state.view_state.selected().unwrap_or(0) as isize)),
                    OverviewPane::Items => state
                        .move_item_selection(-(state.item_state.selected().unwrap_or(0) as isize)),
                    OverviewPane::Details => state.detail_scroll = 0,
                }
            }
            KeyCode::End => match state.focus {
                OverviewPane::ProjectHome => {}
                OverviewPane::Sections => state.move_section_selection(
                    state.document.sections.len().saturating_sub(1) as isize,
                ),
                OverviewPane::Views => {
                    let last = state
                        .current_section()
                        .map(|section| section.views.len())
                        .unwrap_or(0);
                    if last > 0 {
                        state.move_view_selection(last.saturating_sub(1) as isize);
                    }
                }
                OverviewPane::Items => {
                    let last = state.current_items().len();
                    if last > 0 {
                        state.move_item_selection(last.saturating_sub(1) as isize);
                    }
                }
                OverviewPane::Details => {
                    state.detail_scroll = detail_lines_len.saturating_sub(1) as u16;
                }
            },
            KeyCode::Enter => match state.focus {
                OverviewPane::ProjectHome => state.handoff_from_home(),
                _ => state.detail_scroll = 0,
            },
            KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overview::OverviewSummary;
    use crate::project_status::{ProjectDomainStatus, ProjectStatus, ProjectStatusOverall};

    fn test_section(kind: &str, label: &str, subtitle: &str) -> OverviewSection {
        OverviewSection {
            artifact_index: 0,
            kind: kind.to_string(),
            label: label.to_string(),
            subtitle: subtitle.to_string(),
            views: vec![OverviewSectionView {
                label: "Summary".to_string(),
                items: vec![OverviewSectionItem {
                    kind: "test".to_string(),
                    title: label.to_string(),
                    meta: "meta".to_string(),
                    facts: vec![],
                    details: vec![],
                }],
            }],
        }
    }

    fn test_document() -> OverviewDocument {
        OverviewDocument {
            kind: "grafana-utils-overview".to_string(),
            schema_version: 1,
            tool_version: crate::common::TOOL_VERSION.to_string(),
            summary: OverviewSummary::default(),
            project_status: ProjectStatus {
                schema_version: 1,
                tool_version: crate::common::TOOL_VERSION.to_string(),
                scope: "staged-only".to_string(),
                overall: ProjectStatusOverall {
                    status: "blocked".to_string(),
                    domain_count: 6,
                    present_count: 2,
                    blocked_count: 1,
                    blocker_count: 2,
                    warning_count: 0,
                    freshness: Default::default(),
                },
                domains: vec![
                    ProjectDomainStatus {
                        id: "dashboard".to_string(),
                        scope: "staged".to_string(),
                        mode: "artifact-summary".to_string(),
                        status: "ready".to_string(),
                        reason_code: "ready".to_string(),
                        primary_count: 1,
                        blocker_count: 0,
                        warning_count: 0,
                        source_kinds: vec!["dashboard-export".to_string()],
                        signal_keys: vec!["summary.dashboardCount".to_string()],
                        blockers: vec![],
                        warnings: vec![],
                        next_actions: vec![],
                        freshness: Default::default(),
                    },
                    ProjectDomainStatus {
                        id: "sync".to_string(),
                        scope: "staged".to_string(),
                        mode: "artifact-summary".to_string(),
                        status: "blocked".to_string(),
                        reason_code: "blocked-by-blockers".to_string(),
                        primary_count: 1,
                        blocker_count: 2,
                        warning_count: 0,
                        source_kinds: vec!["sync-summary".to_string()],
                        signal_keys: vec!["summary.blockingCount".to_string()],
                        blockers: vec![],
                        warnings: vec![],
                        next_actions: vec![
                            "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact".to_string(),
                        ],
                        freshness: Default::default(),
                    },
                ],
                top_blockers: vec![],
                next_actions: vec![],
            },
            artifacts: vec![],
            selected_section_index: 0,
            sections: vec![
                test_section("dashboard-export", "Dashboard export", "dashboards=1"),
                test_section("bundle-preflight", "Sync bundle preflight", "blocking=2"),
            ],
        }
    }

    #[test]
    fn project_home_defaults_to_home_and_hands_off_to_first_blocked_section() {
        let mut state = OverviewWorkbenchState::new(test_document());

        assert_eq!(state.focus, OverviewPane::ProjectHome);
        assert_eq!(
            state.project_home_target_label().as_deref(),
            Some("Sync bundle preflight")
        );

        state.focus_next();
        assert_eq!(state.focus, OverviewPane::Sections);
        state.focus_previous();
        assert_eq!(state.focus, OverviewPane::ProjectHome);

        state.handoff_from_home();
        assert_eq!(state.focus, OverviewPane::Sections);
        assert_eq!(state.section_state.selected(), Some(1));
        assert_eq!(
            state
                .current_section()
                .map(|section| section.label.as_str()),
            Some("Sync bundle preflight")
        );
    }

    #[test]
    fn project_home_lines_surface_status_and_next_action() {
        let state = OverviewWorkbenchState::new(test_document());
        let lines = state.project_home_lines().join("\n");

        assert!(lines.contains("Overall: status=blocked"));
        assert!(lines.contains("Recommended handoff section: Sync bundle preflight"));
        assert!(lines.contains("Top action: sync status=blocked reason=blocked-by-blockers"));
        assert!(lines.contains("next=resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"));
        assert!(!lines.contains("Navigation: Enter hands off from Home"));
        assert!(lines.contains("Domains: dashboard=ready | sync=blocked"));
    }

    #[test]
    fn interactive_render_starts_on_project_home_surface() {
        use ratatui::backend::TestBackend;

        let mut state = OverviewWorkbenchState::new(test_document());
        let mut terminal = Terminal::new(TestBackend::new(180, 40)).unwrap();

        terminal
            .draw(|frame| overview_tui_render::render_overview_frame(frame, &mut state))
            .unwrap();

        let screen = format!("{}", terminal.backend());
        assert!(screen.contains("Overview"));
        assert!(screen.contains("Recommended handoff section: Sync bundle preflight"));
        assert!(screen.contains("Status & Controls"));
        assert!(!screen.contains("Project Home [Focused]"));
        assert_eq!(state.focus, OverviewPane::ProjectHome);
    }
}
