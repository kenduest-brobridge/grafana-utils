#![cfg(feature = "tui")]
use crate::interactive_browser::BrowserItem;

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
pub(crate) struct InspectFullDetailState {
    pub(crate) open: bool,
    pub(crate) scroll: usize,
    pub(crate) active_logical: usize,
    pub(crate) wrapped: bool,
    pub(crate) row_logical_indexes: Vec<usize>,
    pub(crate) pending_anchor_logical: Option<usize>,
}

impl Default for InspectFullDetailState {
    fn default() -> Self {
        Self {
            open: false,
            scroll: 0,
            active_logical: 0,
            wrapped: true,
            row_logical_indexes: Vec::new(),
            pending_anchor_logical: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InspectWorkbenchModalState {
    pub(crate) full_detail: InspectFullDetailState,
    pub(crate) pending_search: Option<SearchPromptState>,
    pub(crate) last_search: Option<SearchState>,
}

impl InspectWorkbenchModalState {
    pub(crate) fn start_search(&mut self, direction: SearchDirection) {
        self.pending_search = Some(SearchPromptState {
            direction,
            query: String::new(),
        });
    }

    pub(crate) fn open_full_detail(&mut self) {
        self.full_detail.open = true;
        self.full_detail.scroll = 0;
        self.full_detail.active_logical = 0;
        self.full_detail.pending_anchor_logical = None;
    }

    pub(crate) fn close_full_detail(&mut self) {
        self.full_detail.open = false;
        self.full_detail.scroll = 0;
        self.full_detail.active_logical = 0;
        self.full_detail.row_logical_indexes.clear();
        self.full_detail.pending_anchor_logical = None;
    }

    pub(crate) fn toggle_full_detail_wrap(&mut self) {
        self.full_detail.pending_anchor_logical = Some(self.full_detail.active_logical);
        self.full_detail.wrapped = !self.full_detail.wrapped;
    }

    pub(crate) fn move_full_detail_focus(&mut self, line_count: usize, delta: isize) {
        if line_count == 0 {
            self.full_detail.active_logical = 0;
            return;
        }
        let current = self.full_detail.active_logical as isize;
        self.full_detail.active_logical =
            (current + delta).clamp(0, line_count.saturating_sub(1) as isize) as usize;
    }

    pub(crate) fn set_full_detail_focus(&mut self, line_count: usize, index: usize) {
        self.full_detail.active_logical = if line_count == 0 {
            0
        } else {
            index.min(line_count.saturating_sub(1))
        };
    }

    pub(crate) fn clamp_full_detail_scroll(&mut self, max_scroll: usize) {
        self.full_detail.scroll = self.full_detail.scroll.min(max_scroll);
    }

    pub(crate) fn sync_full_detail_row_mapping(&mut self, logical_indexes: Vec<usize>) {
        self.full_detail.row_logical_indexes = logical_indexes;
        if let Some(anchor) = self.full_detail.pending_anchor_logical.take() {
            self.full_detail.active_logical = anchor;
        }
    }

    pub(crate) fn ensure_full_detail_focus_visible(&mut self, viewport_height: usize) {
        let mut first_match = None;
        let mut last_match = None;
        for (index, logical_index) in self.full_detail.row_logical_indexes.iter().enumerate() {
            if *logical_index == self.full_detail.active_logical {
                first_match.get_or_insert(index);
                last_match = Some(index);
            }
        }
        let Some(first) = first_match else {
            self.full_detail.scroll = 0;
            return;
        };
        let last = last_match.unwrap_or(first);
        let viewport_height = viewport_height.max(1);
        if first < self.full_detail.scroll {
            self.full_detail.scroll = first;
            return;
        }
        let visible_end = self
            .full_detail
            .scroll
            .saturating_add(viewport_height.saturating_sub(1));
        if last > visible_end {
            self.full_detail.scroll = last.saturating_add(1).saturating_sub(viewport_height);
        }
    }

    pub(crate) fn find_match(
        &self,
        items: &[BrowserItem],
        query: &str,
        direction: SearchDirection,
        start: Option<usize>,
    ) -> Option<usize> {
        self.find_match_from(items, query, direction, start)
    }

    pub(crate) fn repeat_last_search(
        &self,
        items: &[BrowserItem],
        selected: Option<usize>,
    ) -> Option<usize> {
        let search = self.last_search.as_ref()?;
        let next_start = selected.map(|index| match search.direction {
            SearchDirection::Forward => index.saturating_add(1),
            SearchDirection::Backward => index.saturating_sub(1),
        });
        self.find_match_from(items, &search.query, search.direction, next_start)
    }

    fn find_match_from(
        &self,
        items: &[BrowserItem],
        query: &str,
        direction: SearchDirection,
        start: Option<usize>,
    ) -> Option<usize> {
        let normalized = query.trim().to_ascii_lowercase();
        if normalized.is_empty() || items.is_empty() {
            return None;
        }
        match direction {
            SearchDirection::Forward => {
                let start = start.unwrap_or(0);
                (start..items.len())
                    .find(|index| item_matches(&items[*index], &normalized))
                    .or_else(|| {
                        (0..start.min(items.len()))
                            .find(|index| item_matches(&items[*index], &normalized))
                    })
            }
            SearchDirection::Backward => {
                let start = start.unwrap_or_else(|| items.len().saturating_sub(1));
                (0..=start.min(items.len().saturating_sub(1)))
                    .rev()
                    .find(|index| item_matches(&items[*index], &normalized))
                    .or_else(|| {
                        ((start.saturating_add(1)).min(items.len())..items.len())
                            .rev()
                            .find(|index| item_matches(&items[*index], &normalized))
                    })
            }
        }
    }
}

fn item_matches(item: &BrowserItem, query: &str) -> bool {
    item.title.to_ascii_lowercase().contains(query)
        || item.meta.to_ascii_lowercase().contains(query)
        || item
            .details
            .iter()
            .any(|line| line.to_ascii_lowercase().contains(query))
}
