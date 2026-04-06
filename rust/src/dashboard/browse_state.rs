//! State container for the dashboard browse TUI.
//!
//! `document` owns the current rendered tree snapshot while this module owns ephemeral UI state:
//! current row focus, fact scrolling, cached live detail fetches, and any pending modal dialog.
//! Selection is preserved across refreshes through a lightweight anchor so reloaded trees keep the
//! operator near the same org/folder/dashboard when possible.
#![cfg(feature = "tui")]
use std::collections::BTreeMap;

use ratatui::widgets::ListState;

use super::browse_edit_dialog::EditDialogState;
use super::browse_history_dialog::HistoryDialogState;
use super::browse_support::{
    DashboardBrowseDocument, DashboardBrowseNode, DashboardBrowseNodeKind,
};
use super::delete_support::DeletePlan;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaneFocus {
    Tree,
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
pub(crate) struct SelectionAnchor {
    kind: DashboardBrowseNodeKind,
    uid: Option<String>,
    path: String,
    org_id: String,
}

pub(crate) struct BrowserState {
    pub(crate) document: DashboardBrowseDocument,
    pub(crate) local_mode: bool,
    pub(crate) list_state: ListState,
    pub(crate) detail_scroll: u16,
    pub(crate) live_view_cache: BTreeMap<String, Vec<String>>,
    pub(crate) pending_delete: Option<DeletePlan>,
    pub(crate) pending_edit: Option<EditDialogState>,
    pub(crate) pending_history: Option<HistoryDialogState>,
    pub(crate) pending_search: Option<SearchPromptState>,
    pub(crate) last_search: Option<SearchState>,
    pub(crate) focus: PaneFocus,
    pub(crate) status: String,
}

impl BrowserState {
    #[cfg(test)]
    pub(crate) fn new(document: DashboardBrowseDocument) -> Self {
        Self::new_with_mode(document, false)
    }

    pub(crate) fn new_with_mode(document: DashboardBrowseDocument, local_mode: bool) -> Self {
        let mut list_state = ListState::default();
        list_state.select((!document.nodes.is_empty()).then_some(0));
        let status = if document.nodes.is_empty() {
            "No dashboards matched the current tree.".to_string()
        } else if local_mode {
            "Loaded local dashboard tree. Live actions are unavailable in browse mode.".to_string()
        } else {
            "Loaded dashboard tree. Use e for edit, E for raw JSON edit, v for live details, and d/D for delete.".to_string()
        };
        Self {
            document,
            local_mode,
            list_state,
            detail_scroll: 0,
            live_view_cache: BTreeMap::new(),
            pending_delete: None,
            pending_edit: None,
            pending_history: None,
            pending_search: None,
            last_search: None,
            focus: PaneFocus::Tree,
            status,
        }
    }

    pub(crate) fn selected_node(&self) -> Option<&DashboardBrowseNode> {
        if self.document.nodes.is_empty() {
            None
        } else {
            let index = self
                .list_state
                .selected()
                .unwrap_or(0)
                .min(self.document.nodes.len().saturating_sub(1));
            self.document.nodes.get(index)
        }
    }

    pub(crate) fn replace_document(&mut self, document: DashboardBrowseDocument) {
        let anchor = self.selection_anchor();
        self.document = document;
        // Cached live details belong to the old tree snapshot and may be stale after a refresh.
        self.live_view_cache.clear();
        self.pending_delete = None;
        self.pending_history = None;
        self.pending_search = None;
        // Restore the operator's position by identity first, then degrade to the containing folder.
        self.restore_selection(anchor.as_ref());
        self.detail_scroll = 0;
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        if self.document.nodes.is_empty() {
            self.list_state.select(None);
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, self.document.nodes.len().saturating_sub(1) as isize)
            as usize;
        self.list_state.select(Some(next));
    }

    pub(crate) fn select_first(&mut self) {
        self.list_state
            .select((!self.document.nodes.is_empty()).then_some(0));
    }

    pub(crate) fn select_last(&mut self) {
        self.list_state
            .select(self.document.nodes.len().checked_sub(1));
    }

    pub(crate) fn focus_next_pane(&mut self) {
        self.focus = match self.focus {
            PaneFocus::Tree => PaneFocus::Facts,
            PaneFocus::Facts => PaneFocus::Tree,
        };
    }

    pub(crate) fn focus_previous_pane(&mut self) {
        self.focus = match self.focus {
            PaneFocus::Tree => PaneFocus::Facts,
            PaneFocus::Facts => PaneFocus::Tree,
        };
    }

    pub(crate) fn focus_label(&self) -> &'static str {
        match self.focus {
            PaneFocus::Tree => "tree",
            PaneFocus::Facts => "facts",
        }
    }

    pub(crate) fn start_search(&mut self, direction: SearchDirection) {
        // Search is a transient modal layered over the current tree selection.
        self.pending_search = Some(SearchPromptState {
            direction,
            query: String::new(),
        });
        self.status = match direction {
            SearchDirection::Forward => "Search forward by org, folder, or dashboard.".to_string(),
            SearchDirection::Backward => {
                "Search backward by org, folder, or dashboard.".to_string()
            }
        };
    }

    pub(crate) fn select_index(&mut self, index: usize) {
        if index < self.document.nodes.len() {
            self.list_state.select(Some(index));
            self.detail_scroll = 0;
        }
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
                    SearchDirection::Backward => self.document.nodes.len().checked_sub(1),
                };
                self.find_match_from(&search.query, search.direction, wrapped_start)
            })
    }

    fn selection_anchor(&self) -> Option<SelectionAnchor> {
        self.selected_node().map(|node| SelectionAnchor {
            kind: node.kind.clone(),
            uid: node.uid.clone(),
            path: node.path.clone(),
            org_id: node.org_id.clone(),
        })
    }

    fn restore_selection(&mut self, anchor: Option<&SelectionAnchor>) {
        let selected_index = anchor
            .and_then(|item| {
                self.document.nodes.iter().position(|node| {
                    node.kind == item.kind
                        && node.org_id == item.org_id
                        && match item.kind {
                            DashboardBrowseNodeKind::Org => node.title == item.path,
                            DashboardBrowseNodeKind::Dashboard => node.uid == item.uid,
                            DashboardBrowseNodeKind::Folder => node.path == item.path,
                        }
                })
            })
            .or_else(|| {
                // Dashboard rows can disappear across refreshes; fall back to the enclosing folder
                // before giving up and jumping to the top of the tree.
                anchor.and_then(|item| {
                    self.document.nodes.iter().position(|node| {
                        node.kind == DashboardBrowseNodeKind::Folder
                            && node.org_id == item.org_id
                            && node.path == item.path
                    })
                })
            })
            .or((!self.document.nodes.is_empty()).then_some(0));
        self.list_state.select(selected_index);
    }

    fn find_match_from(
        &self,
        query: &str,
        direction: SearchDirection,
        start: Option<usize>,
    ) -> Option<usize> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() || self.document.nodes.is_empty() {
            return None;
        }
        match direction {
            SearchDirection::Forward => {
                let start_index = start
                    .unwrap_or(0)
                    .min(self.document.nodes.len().saturating_sub(1));
                (start_index..self.document.nodes.len())
                    .find(|&index| node_matches(&self.document.nodes[index], &needle))
            }
            SearchDirection::Backward => {
                let start_index = start.unwrap_or(self.document.nodes.len().saturating_sub(1));
                (0..=start_index.min(self.document.nodes.len().saturating_sub(1)))
                    .rev()
                    .find(|&index| node_matches(&self.document.nodes[index], &needle))
            }
        }
    }
}

fn node_matches(node: &DashboardBrowseNode, needle: &str) -> bool {
    // Search intentionally stays on stable identity/location fields so refreshed live metadata
    // does not change basic match semantics.
    [
        node.org_name.as_str(),
        node.title.as_str(),
        node.path.as_str(),
        node.uid.as_deref().unwrap_or(""),
    ]
    .iter()
    .any(|value| value.to_ascii_lowercase().contains(needle))
}
