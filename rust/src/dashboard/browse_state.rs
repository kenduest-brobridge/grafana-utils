use std::collections::BTreeMap;

use ratatui::widgets::ListState;

use super::browse_edit_dialog::EditDialogState;
use super::browse_history_dialog::HistoryDialogState;
use super::browse_support::{
    DashboardBrowseDocument, DashboardBrowseNode, DashboardBrowseNodeKind,
};
use super::delete_support::DeletePlan;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectionAnchor {
    kind: DashboardBrowseNodeKind,
    uid: Option<String>,
    path: String,
}

pub(crate) struct BrowserState {
    pub(crate) document: DashboardBrowseDocument,
    pub(crate) list_state: ListState,
    pub(crate) detail_scroll: u16,
    pub(crate) live_view_cache: BTreeMap<String, Vec<String>>,
    pub(crate) pending_delete: Option<DeletePlan>,
    pub(crate) pending_edit: Option<EditDialogState>,
    pub(crate) pending_history: Option<HistoryDialogState>,
    pub(crate) status: String,
}

impl BrowserState {
    pub(crate) fn new(document: DashboardBrowseDocument) -> Self {
        let mut list_state = ListState::default();
        list_state.select((!document.nodes.is_empty()).then_some(0));
        let status = if document.nodes.is_empty() {
            "No dashboards matched the current tree.".to_string()
        } else {
            "Loaded dashboard tree. Use e for edit, E for raw JSON edit, v for live details, and d/D for delete.".to_string()
        };
        Self {
            document,
            list_state,
            detail_scroll: 0,
            live_view_cache: BTreeMap::new(),
            pending_delete: None,
            pending_edit: None,
            pending_history: None,
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
        self.live_view_cache.clear();
        self.pending_delete = None;
        self.pending_history = None;
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

    fn selection_anchor(&self) -> Option<SelectionAnchor> {
        self.selected_node().map(|node| SelectionAnchor {
            kind: node.kind.clone(),
            uid: node.uid.clone(),
            path: node.path.clone(),
        })
    }

    fn restore_selection(&mut self, anchor: Option<&SelectionAnchor>) {
        let selected_index = anchor
            .and_then(|item| {
                self.document.nodes.iter().position(|node| {
                    node.kind == item.kind
                        && match item.kind {
                            DashboardBrowseNodeKind::Dashboard => node.uid == item.uid,
                            DashboardBrowseNodeKind::Folder => node.path == item.path,
                        }
                })
            })
            .or_else(|| {
                anchor.and_then(|item| {
                    self.document.nodes.iter().position(|node| {
                        node.kind == DashboardBrowseNodeKind::Folder && node.path == item.path
                    })
                })
            })
            .or((!self.document.nodes.is_empty()).then_some(0));
        self.list_state.select(selected_index);
    }
}
