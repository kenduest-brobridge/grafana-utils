//! State and focus semantics for the interactive dashboard import TUI.
//!
//! Ownership model:
//! - `items` is the durable backing store for all candidate dashboards and their review results.
//! - `list_state` points at a visible row in the current grouping; it is focus, not selection.
//! - `selected_paths` is the explicit batch-selection set used by Enter for dry-run/import.
//! - `context_view`, `summary_scope`, and `diff_depth` are presentation toggles only; they never
//!   change which dashboards are selected or reviewed.
//!
//! Review state follows focus. Moving the focused row marks that row as needing review again, but
//! previously resolved review data remains attached to the underlying item until recomputed.
#![cfg(feature = "tui")]

use std::collections::BTreeSet;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;
use crate::grafana_api::DashboardResourceClient;

use super::import_lookup::ImportLookupCache;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InteractiveImportItem {
    pub(crate) path: PathBuf,
    pub(crate) uid: String,
    pub(crate) title: String,
    pub(crate) folder_path: String,
    pub(crate) file_label: String,
    pub(crate) review: InteractiveImportReviewState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InteractiveImportAction {
    Continue,
    Confirm(Vec<PathBuf>),
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractiveImportGrouping {
    Folder,
    Action,
    Flat,
}

impl InteractiveImportGrouping {
    fn next(self) -> Self {
        match self {
            Self::Folder => Self::Action,
            Self::Action => Self::Flat,
            Self::Flat => Self::Folder,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Action => "action",
            Self::Flat => "flat",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractiveImportContextView {
    Summary,
    Destination,
    Diff,
}

impl InteractiveImportContextView {
    fn next(self) -> Self {
        match self {
            Self::Summary => Self::Destination,
            Self::Destination => Self::Diff,
            Self::Diff => Self::Summary,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Destination => "destination",
            Self::Diff => "diff",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractiveImportSummaryScope {
    Focused,
    Selected,
    All,
}

impl InteractiveImportSummaryScope {
    fn next(self) -> Self {
        match self {
            Self::Focused => Self::Selected,
            Self::Selected => Self::All,
            Self::All => Self::Focused,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Focused => "focused",
            Self::Selected => "selected",
            Self::All => "all",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractiveImportDiffDepth {
    Summary,
    Structural,
    Raw,
}

impl InteractiveImportDiffDepth {
    fn next(self) -> Self {
        match self {
            Self::Summary => Self::Structural,
            Self::Structural => Self::Raw,
            Self::Raw => Self::Summary,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Structural => "structural",
            Self::Raw => "raw",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InteractiveImportReview {
    pub(crate) action: String,
    pub(crate) destination: String,
    pub(crate) action_label: String,
    pub(crate) folder_path: String,
    pub(crate) source_folder_path: String,
    pub(crate) destination_folder_path: String,
    pub(crate) reason: String,
    pub(crate) diff_status: String,
    pub(crate) diff_summary_lines: Vec<String>,
    pub(crate) diff_structural_lines: Vec<String>,
    pub(crate) diff_raw_lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InteractiveImportReviewState {
    Pending,
    Resolved(Box<InteractiveImportReview>),
    Failed(String),
}

pub(crate) type InteractiveImportDiffData = (String, Vec<String>, Vec<String>, Vec<String>);

pub(crate) struct InteractiveImportState {
    pub(crate) items: Vec<InteractiveImportItem>,
    pub(crate) selected_paths: BTreeSet<PathBuf>,
    pub(crate) list_state: ListState,
    pub(crate) grouping: InteractiveImportGrouping,
    pub(crate) import_mode: String,
    pub(crate) dry_run: bool,
    pub(crate) context_view: InteractiveImportContextView,
    pub(crate) summary_scope: InteractiveImportSummaryScope,
    pub(crate) diff_depth: InteractiveImportDiffDepth,
    pub(crate) status: String,
    review_on_focus: bool,
}

#[derive(Default)]
pub(crate) struct InteractiveImportSummaryCounts {
    pub(crate) total: usize,
    pub(crate) selected: usize,
    pub(crate) pending: usize,
    pub(crate) reviewed: usize,
    pub(crate) blocked: usize,
    pub(crate) create: usize,
    pub(crate) update: usize,
    pub(crate) skip_missing: usize,
    pub(crate) skip_folder: usize,
}

impl InteractiveImportState {
    pub(crate) fn new(
        items: Vec<InteractiveImportItem>,
        import_mode: String,
        dry_run: bool,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select((!items.is_empty()).then_some(0));
        Self {
            items,
            selected_paths: BTreeSet::new(),
            list_state,
            grouping: InteractiveImportGrouping::Folder,
            import_mode,
            dry_run,
            context_view: InteractiveImportContextView::Summary,
            summary_scope: InteractiveImportSummaryScope::Focused,
            diff_depth: InteractiveImportDiffDepth::Summary,
            status: if dry_run {
                "Loaded local dashboards. Review follows focus; Enter runs dry-run for the selected dashboards.".to_string()
            } else {
                "Loaded local dashboards. Review follows focus; Enter imports the selected dashboards.".to_string()
            },
            review_on_focus: true,
        }
    }

    pub(crate) fn ordered_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.items.len()).collect();
        indices.sort_by_key(|index| self.sort_key(*index));
        indices
    }

    fn sort_key(&self, index: usize) -> (String, String, String, String) {
        let item = &self.items[index];
        match self.grouping {
            InteractiveImportGrouping::Folder => (
                item.folder_path.clone(),
                item.title.clone(),
                item.uid.clone(),
                item.file_label.clone(),
            ),
            InteractiveImportGrouping::Action => (
                self.action_group_title(item),
                item.folder_path.clone(),
                item.title.clone(),
                item.uid.clone(),
            ),
            InteractiveImportGrouping::Flat => (
                String::new(),
                item.title.clone(),
                item.uid.clone(),
                item.file_label.clone(),
            ),
        }
    }

    pub(crate) fn action_group_title(&self, item: &InteractiveImportItem) -> String {
        match &item.review {
            InteractiveImportReviewState::Pending => "Pending Review".to_string(),
            InteractiveImportReviewState::Failed(_) => "Blocked Review".to_string(),
            InteractiveImportReviewState::Resolved(review) => match review.action_label.as_str() {
                "create" => "Create".to_string(),
                "update" => "Update".to_string(),
                "skip-missing" => "Skip Missing".to_string(),
                "skip-folder-mismatch" => "Skip Folder Mismatch".to_string(),
                "blocked-existing" => "Blocked Existing".to_string(),
                _ => "Other".to_string(),
            },
        }
    }

    fn visible_count(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn review_summary_counts(&self) -> InteractiveImportSummaryCounts {
        let mut counts = InteractiveImportSummaryCounts {
            total: self.items.len(),
            selected: self.selected_paths.len(),
            ..InteractiveImportSummaryCounts::default()
        };
        for item in &self.items {
            match &item.review {
                InteractiveImportReviewState::Pending => counts.pending += 1,
                InteractiveImportReviewState::Failed(_) => counts.blocked += 1,
                InteractiveImportReviewState::Resolved(review) => {
                    match review.action_label.as_str() {
                        "create" => counts.create += 1,
                        "update" => counts.update += 1,
                        "skip-missing" => counts.skip_missing += 1,
                        "skip-folder-mismatch" => counts.skip_folder += 1,
                        "blocked-existing" => counts.blocked += 1,
                        _ => {}
                    }
                }
            }
        }
        counts.reviewed = counts.total.saturating_sub(counts.pending);
        counts
    }

    pub(crate) fn focused_group_summary(&self) -> Option<String> {
        let focused = self.selected_item()?;
        let group_label = match self.grouping {
            InteractiveImportGrouping::Folder => {
                if focused.folder_path.is_empty() {
                    "General".to_string()
                } else {
                    focused.folder_path.clone()
                }
            }
            InteractiveImportGrouping::Action => self.action_group_title(focused),
            InteractiveImportGrouping::Flat => return None,
        };
        let mut item_count = 0usize;
        let mut selected_count = 0usize;
        let mut reviewed_count = 0usize;
        for item in &self.items {
            let same_group = match self.grouping {
                InteractiveImportGrouping::Folder => {
                    let label = if item.folder_path.is_empty() {
                        "General"
                    } else {
                        item.folder_path.as_str()
                    };
                    label == group_label
                }
                InteractiveImportGrouping::Action => self.action_group_title(item) == group_label,
                InteractiveImportGrouping::Flat => false,
            };
            if !same_group {
                continue;
            }
            item_count += 1;
            if self.selected_paths.contains(&item.path) {
                selected_count += 1;
            }
            if !matches!(item.review, InteractiveImportReviewState::Pending) {
                reviewed_count += 1;
            }
        }
        Some(format!(
            "Group={}   Items={}   Reviewed={}   Selected={}",
            group_label, item_count, reviewed_count, selected_count
        ))
    }

    pub(crate) fn selected_item(&self) -> Option<&InteractiveImportItem> {
        let visible_index = self.list_state.selected()?;
        let ordered = self.ordered_indices();
        // `list_state` indexes the grouped/sorted presentation, not `items` directly.
        let item_index = *ordered.get(visible_index)?;
        self.items.get(item_index)
    }

    fn selected_item_index(&self) -> Option<usize> {
        let visible_index = self.list_state.selected()?;
        let ordered = self.ordered_indices();
        ordered.get(visible_index).copied()
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        let visible_count = self.visible_count();
        if visible_count == 0 {
            self.list_state.select(None);
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, visible_count.saturating_sub(1) as isize) as usize;
        self.list_state.select(Some(next));
        // Focus changes trigger lazy review for the newly focused row; batch selection is separate.
        self.review_on_focus = true;
    }

    pub(crate) fn select_first(&mut self) {
        self.list_state
            .select((!self.items.is_empty()).then_some(0));
        self.review_on_focus = true;
    }

    pub(crate) fn select_last(&mut self) {
        self.list_state.select(self.items.len().checked_sub(1));
        self.review_on_focus = true;
    }

    pub(crate) fn toggle_selected(&mut self) {
        let Some(path) = self.selected_item().map(|item| item.path.clone()) else {
            return;
        };
        if !self.selected_paths.remove(&path) {
            self.selected_paths.insert(path);
        }
        self.status = format!("Selected {} dashboard(s).", self.selected_paths.len());
    }

    pub(crate) fn toggle_select_all(&mut self) {
        if self.selected_paths.len() == self.items.len() {
            self.selected_paths.clear();
            self.status = "Cleared dashboard selection.".to_string();
            return;
        }
        self.selected_paths = self.items.iter().map(|item| item.path.clone()).collect();
        self.status = format!("Selected all {} dashboard(s).", self.selected_paths.len());
    }

    pub(crate) fn cycle_grouping(&mut self) {
        let focused_path = self.selected_item().map(|item| item.path.clone());
        self.grouping = self.grouping.next();
        if let Some(path) = focused_path {
            // Re-anchor on the same underlying dashboard after regrouping so focus-driven review
            // and context panes stay attached to the same item.
            self.select_path(&path);
        }
        self.status = format!(
            "Grouping is now {}. Review rows are still resolved on focus.",
            self.grouping.label()
        );
    }

    pub(crate) fn cycle_context_view(&mut self) {
        self.context_view = self.context_view.next();
        self.status = format!("Context view is now {}.", self.context_view.label());
    }

    pub(crate) fn cycle_summary_scope(&mut self) {
        self.summary_scope = self.summary_scope.next();
        self.status = format!("Summary scope is now {}.", self.summary_scope.label());
    }

    pub(crate) fn cycle_diff_depth(&mut self) {
        self.diff_depth = self.diff_depth.next();
        self.status = format!("Diff depth is now {}.", self.diff_depth.label());
    }

    fn select_path(&mut self, path: &PathBuf) {
        let ordered = self.ordered_indices();
        let next_index = ordered
            .iter()
            .position(|item_index| self.items[*item_index].path == *path);
        self.list_state.select(next_index);
    }

    pub(crate) fn focus_needs_review(&self) -> bool {
        self.review_on_focus
    }

    pub(crate) fn mark_focus_reviewed(&mut self) {
        self.review_on_focus = false;
    }

    pub(crate) fn resolve_focused_review_with_request<F>(
        &mut self,
        request_json: &mut F,
        lookup_cache: &mut ImportLookupCache,
        args: &super::ImportArgs,
    ) where
        F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
    {
        let Some(item_index) = self.selected_item_index() else {
            self.mark_focus_reviewed();
            return;
        };
        // Review work is cached on the backing item. Presentation toggles may move the focused row
        // around, but they should not force duplicate requests unless focus actually changed.
        if !matches!(
            self.items[item_index].review,
            InteractiveImportReviewState::Pending
        ) {
            self.mark_focus_reviewed();
            return;
        }
        let path = self.items[item_index].path.clone();
        let uid = self.items[item_index].uid.clone();
        let source_folder_path = self.items[item_index].folder_path.clone();
        let result = super::import_interactive_review::build_interactive_import_review_with_request(
            request_json,
            lookup_cache,
            args,
            &path,
            &uid,
            &source_folder_path,
        );
        let (status, review_state) = match result {
            Ok(review) => (
                format!(
                    "Reviewed {}: {} {}.",
                    uid, review.destination, review.action_label
                ),
                InteractiveImportReviewState::Resolved(Box::new(review)),
            ),
            Err(error) => (
                format!("Review blocked for {}: {}", uid, error),
                InteractiveImportReviewState::Failed(error.to_string()),
            ),
        };
        self.status = status;
        self.items[item_index].review = review_state;
        self.mark_focus_reviewed();
    }

    pub(crate) fn resolve_focused_review_with_client(
        &mut self,
        client: &DashboardResourceClient<'_>,
        lookup_cache: &mut ImportLookupCache,
        args: &super::ImportArgs,
    ) {
        let Some(item_index) = self.selected_item_index() else {
            self.mark_focus_reviewed();
            return;
        };
        if !matches!(
            self.items[item_index].review,
            InteractiveImportReviewState::Pending
        ) {
            self.mark_focus_reviewed();
            return;
        }
        let path = self.items[item_index].path.clone();
        let uid = self.items[item_index].uid.clone();
        let source_folder_path = self.items[item_index].folder_path.clone();
        let result = super::import_interactive_review::build_interactive_import_review_with_client(
            client,
            lookup_cache,
            args,
            &path,
            &uid,
            &source_folder_path,
        );
        let (status, review_state) = match result {
            Ok(review) => (
                format!(
                    "Reviewed {}: {} {}.",
                    uid, review.destination, review.action_label
                ),
                InteractiveImportReviewState::Resolved(Box::new(review)),
            ),
            Err(error) => (
                format!("Review blocked for {}: {}", uid, error),
                InteractiveImportReviewState::Failed(error.to_string()),
            ),
        };
        self.status = status;
        self.items[item_index].review = review_state;
        self.mark_focus_reviewed();
    }

    pub(crate) fn selected_files(&self) -> Vec<PathBuf> {
        self.items
            .iter()
            .filter(|item| self.selected_paths.contains(&item.path))
            .map(|item| item.path.clone())
            .collect()
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> InteractiveImportAction {
        match key.code {
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::Home => self.select_first(),
            KeyCode::End => self.select_last(),
            KeyCode::Char(' ') => self.toggle_selected(),
            KeyCode::Char('a') => self.toggle_select_all(),
            KeyCode::Char('g') => self.cycle_grouping(),
            KeyCode::Char('v') => self.cycle_context_view(),
            KeyCode::Char('s') => self.cycle_summary_scope(),
            KeyCode::Char('d') => self.cycle_diff_depth(),
            KeyCode::Enter => {
                let files = self.selected_files();
                if files.is_empty() {
                    self.status = if self.dry_run {
                        "Select at least one dashboard before running interactive dry-run."
                            .to_string()
                    } else {
                        "Select at least one dashboard before importing.".to_string()
                    };
                } else {
                    return InteractiveImportAction::Confirm(files);
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => return InteractiveImportAction::Cancel,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return InteractiveImportAction::Cancel;
            }
            _ => {}
        }
        InteractiveImportAction::Continue
    }
}
