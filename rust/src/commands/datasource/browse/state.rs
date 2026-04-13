#![cfg(feature = "tui")]

use ratatui::widgets::ListState;

use super::datasource_browse_edit_dialog::EditDialogState;
use super::datasource_browse_support::{
    DatasourceBrowseDocument, DatasourceBrowseItem, DatasourceBrowseItemKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaneFocus {
    List,
    Facts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchPromptState {
    pub(crate) direction: SearchDirection,
    pub(crate) query: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchState {
    pub(crate) direction: SearchDirection,
    pub(crate) query: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingDelete {
    pub(crate) uid: String,
    pub(crate) name: String,
    pub(crate) id: i64,
}

pub(crate) struct BrowserState {
    pub(crate) document: DatasourceBrowseDocument,
    pub(crate) list_state: ListState,
    pub(crate) detail_scroll: u16,
    pub(crate) status: String,
    pub(crate) pending_delete: Option<PendingDelete>,
    pub(crate) pending_edit: Option<EditDialogState>,
    pub(crate) pending_search: Option<SearchPromptState>,
    pub(crate) last_search: Option<SearchState>,
    pub(crate) focus: PaneFocus,
}

impl BrowserState {
    pub(crate) fn new(document: DatasourceBrowseDocument) -> Self {
        let mut list_state = ListState::default();
        list_state.select((!document.items.is_empty()).then_some(0));
        let status = if document.items.is_empty() {
            "No datasources found in the selected browse scope.".to_string()
        } else {
            "Loaded datasource browser. Use e for edit, d for delete, and l for refresh."
                .to_string()
        };
        Self {
            document,
            list_state,
            detail_scroll: 0,
            status,
            pending_delete: None,
            pending_edit: None,
            pending_search: None,
            last_search: None,
            focus: PaneFocus::List,
        }
    }

    pub(crate) fn selected_item(&self) -> Option<&DatasourceBrowseItem> {
        if self.document.items.is_empty() {
            None
        } else {
            let index = self
                .list_state
                .selected()
                .unwrap_or(0)
                .min(self.document.items.len().saturating_sub(1));
            self.document.items.get(index)
        }
    }

    pub(crate) fn replace_document(&mut self, document: DatasourceBrowseDocument) {
        let selected_anchor = self.selected_item().map(selection_anchor);
        self.document = document;
        self.pending_delete = None;
        self.pending_edit = None;
        self.pending_search = None;
        self.detail_scroll = 0;
        let selected = selected_anchor
            .as_ref()
            .and_then(|anchor| {
                self.document
                    .items
                    .iter()
                    .position(|item| selection_anchor(item) == *anchor)
            })
            .or((!self.document.items.is_empty()).then_some(0));
        self.list_state.select(selected);
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        if self.document.items.is_empty() {
            self.list_state.select(None);
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, self.document.items.len().saturating_sub(1) as isize)
            as usize;
        self.list_state.select(Some(next));
    }

    pub(crate) fn select_first(&mut self) {
        self.list_state
            .select((!self.document.items.is_empty()).then_some(0));
    }

    pub(crate) fn select_last(&mut self) {
        self.list_state
            .select(self.document.items.len().checked_sub(1));
    }

    pub(crate) fn focus_next_pane(&mut self) {
        self.focus = match self.focus {
            PaneFocus::List => PaneFocus::Facts,
            PaneFocus::Facts => PaneFocus::List,
        };
    }

    pub(crate) fn focus_previous_pane(&mut self) {
        self.focus = match self.focus {
            PaneFocus::List => PaneFocus::Facts,
            PaneFocus::Facts => PaneFocus::List,
        };
    }

    pub(crate) fn focus_label(&self) -> &'static str {
        match self.focus {
            PaneFocus::List => "list",
            PaneFocus::Facts => "facts",
        }
    }

    pub(crate) fn start_search(&mut self, direction: SearchDirection) {
        self.pending_search = Some(SearchPromptState {
            direction,
            query: String::new(),
        });
        self.status = match direction {
            SearchDirection::Forward => "Search forward by org or datasource name.".to_string(),
            SearchDirection::Backward => "Search backward by org or datasource name.".to_string(),
        };
    }

    pub(crate) fn find_match(&self, query: &str, direction: SearchDirection) -> Option<usize> {
        self.find_match_from(query, direction, self.list_state.selected())
    }

    pub(crate) fn repeat_last_search(&self) -> Option<usize> {
        let search = self.last_search.as_ref()?;
        let next_start = self
            .list_state
            .selected()
            .map(|index| match search.direction {
                SearchDirection::Forward => index.saturating_add(1),
                SearchDirection::Backward => index.saturating_sub(1),
            });
        self.find_match_from(&search.query, search.direction, next_start)
            .or_else(|| {
                let wrapped_start = match search.direction {
                    SearchDirection::Forward => Some(0),
                    SearchDirection::Backward => self.document.items.len().checked_sub(1),
                };
                self.find_match_from(&search.query, search.direction, wrapped_start)
            })
    }

    pub(crate) fn select_index(&mut self, index: usize) {
        if index < self.document.items.len() {
            self.list_state.select(Some(index));
            self.detail_scroll = 0;
        }
    }

    fn find_match_from(
        &self,
        query: &str,
        direction: SearchDirection,
        start: Option<usize>,
    ) -> Option<usize> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() || self.document.items.is_empty() {
            return None;
        }

        match direction {
            SearchDirection::Forward => {
                let start_index = start
                    .unwrap_or(0)
                    .min(self.document.items.len().saturating_sub(1));
                (start_index..self.document.items.len())
                    .find(|&index| item_matches(&self.document.items[index], &needle))
            }
            SearchDirection::Backward => {
                let start_index = start.unwrap_or(self.document.items.len().saturating_sub(1));
                (0..=start_index.min(self.document.items.len().saturating_sub(1)))
                    .rev()
                    .find(|&index| item_matches(&self.document.items[index], &needle))
            }
        }
    }
}

fn selection_anchor(item: &DatasourceBrowseItem) -> (DatasourceBrowseItemKind, String, String) {
    match item.kind {
        DatasourceBrowseItemKind::Org => (
            item.kind,
            item.org_id.clone(),
            item.org.to_ascii_lowercase(),
        ),
        DatasourceBrowseItemKind::Datasource => (
            item.kind,
            item.org_id.clone(),
            item.uid.to_ascii_lowercase(),
        ),
    }
}

fn item_matches(item: &DatasourceBrowseItem, needle: &str) -> bool {
    [
        item.org.as_str(),
        item.name.as_str(),
        item.uid.as_str(),
        item.datasource_type.as_str(),
    ]
    .iter()
    .any(|value| value.to_ascii_lowercase().contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;

    fn org_row(org: &str, org_id: &str, count: usize) -> DatasourceBrowseItem {
        DatasourceBrowseItem {
            kind: DatasourceBrowseItemKind::Org,
            depth: 0,
            id: 0,
            uid: String::new(),
            name: org.to_string(),
            datasource_type: "org".to_string(),
            access: String::new(),
            url: String::new(),
            is_default: false,
            org: org.to_string(),
            org_id: org_id.to_string(),
            details: Map::new(),
            datasource_count: count,
        }
    }

    fn ds_row(org: &str, org_id: &str, name: &str, uid: &str) -> DatasourceBrowseItem {
        DatasourceBrowseItem {
            kind: DatasourceBrowseItemKind::Datasource,
            depth: 1,
            id: 1,
            uid: uid.to_string(),
            name: name.to_string(),
            datasource_type: "prometheus".to_string(),
            access: "proxy".to_string(),
            url: "http://prom".to_string(),
            is_default: false,
            org: org.to_string(),
            org_id: org_id.to_string(),
            details: Map::new(),
            datasource_count: 0,
        }
    }

    fn state() -> BrowserState {
        BrowserState::new(DatasourceBrowseDocument {
            scope_label: "All visible orgs".to_string(),
            org: "All visible orgs".to_string(),
            org_id: "-".to_string(),
            items: vec![
                org_row("Audit Org", "4", 1),
                ds_row("Audit Org", "4", "Audit Prometheus", "audit-prom"),
                org_row("Main Org.", "1", 2),
                ds_row("Main Org.", "1", "Smoke Loki", "smoke-loki"),
                ds_row("Main Org.", "1", "Smoke Prometheus", "smoke-prom"),
            ],
            org_count: 2,
            datasource_count: 3,
        })
    }

    #[test]
    fn browser_search_matches_org_rows_by_name() {
        let state = state();
        assert_eq!(
            state.find_match("main org", SearchDirection::Forward),
            Some(2)
        );
    }

    #[test]
    fn browser_repeat_last_search_wraps_to_next_match() {
        let mut state = state();
        state.last_search = Some(SearchState {
            direction: SearchDirection::Forward,
            query: "smoke".to_string(),
        });
        state.select_index(3);
        assert_eq!(state.repeat_last_search(), Some(4));
    }
}
