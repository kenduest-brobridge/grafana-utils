#![cfg(feature = "tui")]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use super::inspect_workbench_support::{InspectWorkbenchDocument, InspectWorkbenchGroup};

#[path = "inspect_workbench_modal_state.rs"]
mod inspect_workbench_modal_state;

pub(crate) use inspect_workbench_modal_state::{
    InspectFullDetailState, InspectWorkbenchModalState, SearchDirection, SearchPromptState,
    SearchState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectPane {
    Groups,
    Items,
    Facts,
}

pub(crate) struct InspectWorkbenchState {
    pub(crate) document: InspectWorkbenchDocument,
    pub(crate) group_state: ListState,
    pub(crate) item_state: ListState,
    pub(crate) focus: InspectPane,
    pub(crate) item_horizontal_offset: usize,
    pub(crate) detail_cursor: usize,
    pub(crate) modal: InspectWorkbenchModalState,
    pub(crate) status: String,
    pub(crate) group_view_indexes: Vec<usize>,
}

impl InspectWorkbenchState {
    pub(crate) fn new(document: InspectWorkbenchDocument) -> Self {
        let mut group_state = ListState::default();
        group_state.select((!document.groups.is_empty()).then_some(0));
        let group_view_indexes = vec![0; document.groups.len()];
        let mut state = Self {
            document,
            group_state,
            item_state: ListState::default(),
            focus: InspectPane::Groups,
            item_horizontal_offset: 0,
            detail_cursor: 0,
            modal: InspectWorkbenchModalState::default(),
            status: "Loaded inspect workbench. Tab panes, / search, g modes, v mode view."
                .to_string(),
            group_view_indexes,
        };
        state.reset_items();
        state
    }

    pub(crate) fn current_group(&self) -> Option<&InspectWorkbenchGroup> {
        self.group_state
            .selected()
            .and_then(|index| self.document.groups.get(index))
    }

    pub(crate) fn current_view_label(&self) -> String {
        self.current_group()
            .and_then(|group| {
                let group_index = self.group_state.selected().unwrap_or(0);
                group
                    .views
                    .get(
                        self.group_view_indexes
                            .get(group_index)
                            .copied()
                            .unwrap_or(0),
                    )
                    .map(|view| view.label.clone())
            })
            .unwrap_or_else(|| "Default".to_string())
    }

    pub(crate) fn current_items(&self) -> &[crate::interactive_browser::BrowserItem] {
        self.current_group()
            .and_then(|group| {
                let group_index = self.group_state.selected().unwrap_or(0);
                group.views.get(
                    self.group_view_indexes
                        .get(group_index)
                        .copied()
                        .unwrap_or(0),
                )
            })
            .map(|view| view.items.as_slice())
            .unwrap_or(&[])
    }

    pub(crate) fn selected_item(&self) -> Option<&crate::interactive_browser::BrowserItem> {
        self.item_state
            .selected()
            .and_then(|index| self.current_items().get(index))
    }

    pub(crate) fn current_detail_lines(&self) -> Vec<String> {
        self.selected_item()
            .map(|item| {
                if item.details.is_empty() {
                    vec!["No facts available.".to_string()]
                } else {
                    item.details.clone()
                }
            })
            .unwrap_or_else(|| vec!["No item selected.".to_string()])
    }

    pub(crate) fn current_full_detail_lines(&self) -> Vec<String> {
        self.selected_item()
            .map(|item| {
                let mut lines = vec![
                    fact_line("Kind", &item.kind),
                    fact_line("Title", &item.title),
                ];
                if !item.meta.is_empty() {
                    lines.push(fact_line("Summary", &item.meta));
                }
                if !item.details.is_empty() {
                    lines.push(String::new());
                    lines.extend(item.details.clone());
                }
                lines
            })
            .unwrap_or_else(|| vec!["No item selected.".to_string()])
    }

    pub(crate) fn reset_items(&mut self) {
        self.item_state
            .select((!self.current_items().is_empty()).then_some(0));
        self.item_horizontal_offset = 0;
        self.detail_cursor = 0;
        self.modal.full_detail = InspectFullDetailState::default();
    }

    pub(crate) fn focus_next(&mut self) {
        self.focus = match self.focus {
            InspectPane::Groups => InspectPane::Items,
            InspectPane::Items => InspectPane::Facts,
            InspectPane::Facts => InspectPane::Groups,
        };
    }

    pub(crate) fn focus_previous(&mut self) {
        self.focus = match self.focus {
            InspectPane::Groups => InspectPane::Facts,
            InspectPane::Items => InspectPane::Groups,
            InspectPane::Facts => InspectPane::Items,
        };
    }

    pub(crate) fn move_group_selection(&mut self, delta: isize) {
        if self.document.groups.is_empty() {
            self.group_state.select(None);
            return;
        }
        let current = self.group_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, self.document.groups.len().saturating_sub(1) as isize)
            as usize;
        self.group_state.select(Some(next));
        self.reset_items();
    }

    pub(crate) fn cycle_group(&mut self) {
        if self.document.groups.is_empty() {
            self.group_state.select(None);
            return;
        }
        let current = self.group_state.selected().unwrap_or(0);
        let next = (current + 1) % self.document.groups.len();
        self.group_state.select(Some(next));
        self.reset_items();
        if let Some(group) = self.current_group() {
            self.status = format!("Focused {} mode.", group.label);
        }
    }

    pub(crate) fn move_item_selection(&mut self, delta: isize) {
        let items_len = self.current_items().len();
        if items_len == 0 {
            self.item_state.select(None);
            return;
        }
        let current = self.item_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, items_len.saturating_sub(1) as isize) as usize;
        self.item_state.select(Some(next));
        self.item_horizontal_offset = 0;
        self.detail_cursor = 0;
    }

    pub(crate) fn move_item_horizontal_offset(&mut self, delta: isize) {
        if delta.is_negative() {
            self.item_horizontal_offset = self
                .item_horizontal_offset
                .saturating_sub(delta.unsigned_abs());
        } else {
            self.item_horizontal_offset =
                self.item_horizontal_offset.saturating_add(delta as usize);
        }
    }

    pub(crate) fn clamp_item_horizontal_offset(&mut self, max_offset: usize) {
        self.item_horizontal_offset = self.item_horizontal_offset.min(max_offset);
    }

    pub(crate) fn move_detail_cursor(&mut self, delta: isize) {
        let line_count = self.current_detail_lines().len();
        if line_count == 0 {
            self.detail_cursor = 0;
            return;
        }
        let current = self.detail_cursor as isize;
        self.detail_cursor =
            (current + delta).clamp(0, line_count.saturating_sub(1) as isize) as usize;
    }

    pub(crate) fn set_detail_cursor(&mut self, index: usize) {
        let line_count = self.current_detail_lines().len();
        self.detail_cursor = if line_count == 0 {
            0
        } else {
            index.min(line_count.saturating_sub(1))
        };
    }

    pub(crate) fn start_search(&mut self, direction: SearchDirection) {
        self.modal.start_search(direction);
        self.status = "Search current inspect rows by title, meta, or facts.".to_string();
    }

    pub(crate) fn open_full_detail(&mut self) {
        self.modal.open_full_detail();
        self.status =
            "Opened full detail viewer. w toggles wrap; Esc, q, or Enter closes.".to_string();
    }

    pub(crate) fn close_full_detail(&mut self) {
        self.modal.close_full_detail();
        self.status = "Closed full detail viewer.".to_string();
    }

    pub(crate) fn toggle_full_detail_wrap(&mut self) {
        self.modal.toggle_full_detail_wrap();
        self.status = if self.modal.full_detail.wrapped {
            "Full detail viewer wrap enabled.".to_string()
        } else {
            "Full detail viewer wrap disabled.".to_string()
        };
    }

    pub(crate) fn move_full_detail_focus(&mut self, delta: isize) {
        let line_count = self.current_full_detail_lines().len();
        self.modal.move_full_detail_focus(line_count, delta);
    }

    pub(crate) fn set_full_detail_focus(&mut self, index: usize) {
        let line_count = self.current_full_detail_lines().len();
        self.modal.set_full_detail_focus(line_count, index);
    }

    pub(crate) fn clamp_full_detail_scroll(&mut self, max_scroll: usize) {
        self.modal.clamp_full_detail_scroll(max_scroll);
    }

    pub(crate) fn sync_full_detail_row_mapping(&mut self, logical_indexes: Vec<usize>) {
        self.modal.sync_full_detail_row_mapping(logical_indexes);
    }

    pub(crate) fn ensure_full_detail_focus_visible(&mut self, viewport_height: usize) {
        self.modal.ensure_full_detail_focus_visible(viewport_height);
    }

    pub(crate) fn find_match(&self, query: &str, direction: SearchDirection) -> Option<usize> {
        self.modal.find_match(
            self.current_items(),
            query,
            direction,
            self.item_state.selected(),
        )
    }

    pub(crate) fn repeat_last_search(&self) -> Option<usize> {
        self.modal
            .repeat_last_search(self.current_items(), self.item_state.selected())
    }

    pub(crate) fn cycle_group_view(&mut self) {
        let Some(group_index) = self.group_state.selected() else {
            return;
        };
        let Some(group) = self.document.groups.get(group_index) else {
            return;
        };
        let group_label = group.label.clone();
        if group.views.len() <= 1 {
            self.status = format!("{group_label} has one presentation only.");
            return;
        }
        let next = (self.group_view_indexes[group_index] + 1) % group.views.len();
        let view_label = group.views[next].label.clone();
        self.group_view_indexes[group_index] = next;
        self.reset_items();
        self.status = format!("{group_label} mode view: {view_label}.");
    }
}

fn fact_line(label: &str, value: &str) -> String {
    format!("{label:<16}: {value}")
}

pub(crate) fn handle_search_key(state: &mut InspectWorkbenchState, key: &KeyEvent) {
    let Some(search) = state.modal.pending_search.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Esc => {
            state.modal.pending_search = None;
            state.status = "Cancelled search.".to_string();
        }
        KeyCode::Enter => {
            let query = search.query.trim().to_string();
            let direction = search.direction;
            state.modal.pending_search = None;
            if query.is_empty() {
                state.status = "Search query is empty.".to_string();
                return;
            }
            state.modal.last_search = Some(SearchState {
                direction,
                query: query.clone(),
            });
            if let Some(index) = state.find_match(&query, direction) {
                state.item_state.select(Some(index));
                state.detail_cursor = 0;
                state.status = format!("Matched inspect row for {query}.");
            } else {
                state.status = format!("No inspect rows matched {query}.");
            }
        }
        KeyCode::Backspace => {
            search.query.pop();
        }
        KeyCode::Char(ch)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT) =>
        {
            search.query.push(ch);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{InspectWorkbenchState, SearchDirection, SearchState};
    use crate::dashboard::inspect_workbench_support::build_inspect_workbench_document;
    use crate::dashboard::test_support::{inspect_governance, make_core_family_report_row};
    use crate::dashboard::test_support::{
        ExportInspectionQueryReport, ExportInspectionSummary, QueryReportSummary,
    };

    fn sample_state() -> InspectWorkbenchState {
        let summary = ExportInspectionSummary {
            import_dir: "/tmp/raw".to_string(),
            export_org: Some("Main Org.".to_string()),
            export_org_id: Some("1".to_string()),
            dashboard_count: 1,
            folder_count: 1,
            panel_count: 1,
            query_count: 1,
            datasource_inventory_count: 1,
            orphaned_datasource_count: 0,
            mixed_dashboard_count: 0,
            folder_paths: Vec::new(),
            datasource_usage: Vec::new(),
            datasource_inventory: Vec::new(),
            orphaned_datasources: Vec::new(),
            mixed_dashboards: Vec::new(),
        };
        let report = ExportInspectionQueryReport {
            import_dir: "/tmp/raw".to_string(),
            summary: QueryReportSummary {
                dashboard_count: 1,
                panel_count: 1,
                query_count: 1,
                report_row_count: 1,
            },
            queries: vec![make_core_family_report_row(
                "cpu-main",
                "7",
                "A",
                "prom-main",
                "Prometheus Main",
                "prometheus",
                "prometheus",
                "up",
                &[],
            )],
        };
        let governance = inspect_governance::ExportInspectionGovernanceDocument {
            summary: inspect_governance::GovernanceSummary {
                dashboard_count: 1,
                query_record_count: 1,
                datasource_inventory_count: 1,
                datasource_family_count: 1,
                datasource_coverage_count: 1,
                dashboard_datasource_edge_count: 1,
                datasource_risk_coverage_count: 1,
                dashboard_risk_coverage_count: 1,
                mixed_datasource_dashboard_count: 0,
                orphaned_datasource_count: 0,
                risk_record_count: 1,
                query_audit_count: 1,
                dashboard_audit_count: 0,
            },
            datasource_families: Vec::new(),
            dashboard_dependencies: Vec::new(),
            dashboard_governance: vec![inspect_governance::DashboardGovernanceRow {
                dashboard_uid: "cpu-main".to_string(),
                dashboard_title: "CPU Main".to_string(),
                folder_path: "General".to_string(),
                panel_count: 1,
                query_count: 1,
                datasource_count: 1,
                datasource_family_count: 1,
                datasources: vec!["prom-main".to_string()],
                datasource_families: vec!["prometheus".to_string()],
                mixed_datasource: false,
                risk_count: 1,
                risk_kinds: vec!["prometheus-query-cost-score".to_string()],
            }],
            dashboard_datasource_edges: Vec::new(),
            datasource_governance: Vec::new(),
            datasources: Vec::new(),
            risk_records: vec![inspect_governance::GovernanceRiskRow {
                kind: "prometheus-query-cost-score".to_string(),
                severity: "high".to_string(),
                category: "cost".to_string(),
                dashboard_uid: "cpu-main".to_string(),
                panel_id: "7".to_string(),
                datasource: "Prometheus Main".to_string(),
                detail: "cost=3".to_string(),
                recommendation: "Reduce expensive Prometheus query shapes before broad rollout."
                    .to_string(),
            }],
            query_audits: vec![inspect_governance::QueryAuditRow {
                dashboard_uid: "cpu-main".to_string(),
                dashboard_title: "CPU Main".to_string(),
                folder_path: "General".to_string(),
                panel_id: "7".to_string(),
                panel_title: "CPU".to_string(),
                ref_id: "A".to_string(),
                datasource: "Prometheus Main".to_string(),
                datasource_uid: "prom-main".to_string(),
                datasource_family: "prometheus".to_string(),
                aggregation_depth: 0,
                regex_matcher_count: 0,
                estimated_series_risk: "low".to_string(),
                query_cost_score: 3,
                score: 2,
                severity: "medium".to_string(),
                reasons: vec!["prometheus-query-cost-score".to_string()],
                recommendations: vec!["Trim costly aggregation and range windows.".to_string()],
            }],
            dashboard_audits: Vec::new(),
        };
        let document =
            build_inspect_workbench_document("live snapshot", &summary, &governance, &report);
        InspectWorkbenchState::new(document)
    }

    #[test]
    fn cycle_group_wraps_back_to_first_mode() {
        let mut state = sample_state();
        state
            .group_state
            .select(Some(state.document.groups.len() - 1));

        state.cycle_group();

        assert_eq!(state.group_state.selected(), Some(0));
        assert_eq!(
            state.current_group().map(|group| group.label.as_str()),
            Some("Overview")
        );
    }

    #[test]
    fn full_detail_viewer_wrap_toggle_flips_state() {
        let mut state = sample_state();

        state.open_full_detail();
        assert!(state.modal.full_detail.open);
        assert_eq!(state.modal.full_detail.scroll, 0);
        assert!(state.modal.full_detail.wrapped);

        state.toggle_full_detail_wrap();
        assert!(!state.modal.full_detail.wrapped);

        state.toggle_full_detail_wrap();
        assert!(state.modal.full_detail.wrapped);
    }

    #[test]
    fn full_detail_viewer_scroll_moves_independently() {
        let mut state = sample_state();

        state.open_full_detail();
        state.move_full_detail_focus(3);
        assert_eq!(state.modal.full_detail.active_logical, 3);

        state.move_full_detail_focus(-1);
        assert_eq!(state.modal.full_detail.active_logical, 2);

        state.set_full_detail_focus(0);
        assert_eq!(state.modal.full_detail.active_logical, 0);
    }

    #[test]
    fn repeat_last_search_uses_the_nested_modal_search_state() {
        let mut state = sample_state();
        state.item_state.select(Some(0));
        state.modal.last_search = Some(SearchState {
            direction: SearchDirection::Forward,
            query: "cpu".to_string(),
        });

        assert_eq!(state.repeat_last_search(), Some(0));
    }

    #[test]
    fn item_horizontal_offset_resets_when_item_selection_changes() {
        let mut state = sample_state();

        state.move_item_horizontal_offset(12);
        assert_eq!(state.item_horizontal_offset, 12);

        state.move_item_selection(1);
        assert_eq!(state.item_horizontal_offset, 0);
    }

    #[test]
    fn item_horizontal_offset_is_clamped_to_max_visible_range() {
        let mut state = sample_state();

        state.move_item_horizontal_offset(120);
        state.clamp_item_horizontal_offset(9);

        assert_eq!(state.item_horizontal_offset, 9);
    }
}
