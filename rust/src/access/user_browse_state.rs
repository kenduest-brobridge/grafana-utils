//! Interactive browse workflows and terminal-driven state flow for Access entities.

use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use serde_json::{Map, Value};

use crate::access::render::map_get_text;
use crate::access::UserBrowseArgs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DisplayMode {
    GlobalAccounts,
    OrgMemberships,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PaneFocus {
    List,
    Facts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SearchPromptState {
    pub(super) direction: SearchDirection,
    pub(super) query: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SearchState {
    pub(super) direction: SearchDirection,
    pub(super) query: String,
}

pub(super) struct BrowserState {
    pub(super) base_rows: Vec<Map<String, Value>>,
    pub(super) rows: Vec<Map<String, Value>>,
    pub(super) list_state: ListState,
    pub(super) detail_cursor: usize,
    pub(super) focus: PaneFocus,
    pub(super) display_mode: DisplayMode,
    pub(super) expanded_user_ids: BTreeSet<String>,
    pub(super) show_numbers: bool,
    pub(super) status: String,
    pub(super) pending_delete: bool,
    pub(super) pending_member_remove: bool,
    pub(super) pending_edit: Option<super::user_browse_dialog::EditDialogState>,
    pub(super) pending_search: Option<SearchPromptState>,
    pub(super) last_search: Option<SearchState>,
}

impl BrowserState {
    pub(super) fn new(rows: Vec<Map<String, Value>>, display_mode: DisplayMode) -> Self {
        let mut list_state = ListState::default();
        let visible_rows = flatten_user_rows(&rows, display_mode, &BTreeSet::new());
        list_state.select((!visible_rows.is_empty()).then_some(0));
        Self {
            base_rows: rows,
            rows: visible_rows,
            list_state,
            detail_cursor: 0,
            focus: PaneFocus::List,
            display_mode,
            expanded_user_ids: BTreeSet::new(),
            show_numbers: true,
            status:
                "Loaded user browser. Use Enter/Right expand teams, Left collapse, c view mode."
                    .to_string(),
            pending_delete: false,
            pending_member_remove: false,
            pending_edit: None,
            pending_search: None,
            last_search: None,
        }
    }

    pub(super) fn selected_row(&self) -> Option<&Map<String, Value>> {
        self.list_state
            .selected()
            .and_then(|index| self.rows.get(index))
    }

    pub(super) fn replace_rows(&mut self, rows: Vec<Map<String, Value>>) {
        let selected_id = self.selected_user_id();
        self.base_rows = rows;
        self.pending_delete = false;
        self.pending_member_remove = false;
        self.pending_edit = None;
        self.pending_search = None;
        self.detail_cursor = 0;
        self.rows = flatten_user_rows(&self.base_rows, self.display_mode, &self.expanded_user_ids);
        let selected = selected_id
            .as_ref()
            .and_then(|id| {
                self.rows
                    .iter()
                    .position(|row| map_get_text(row, "id") == *id)
            })
            .or((!self.rows.is_empty()).then_some(0));
        self.list_state.select(selected);
    }

    pub(super) fn move_selection(&mut self, delta: isize) {
        if self.rows.is_empty() {
            self.list_state.select(None);
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, self.rows.len().saturating_sub(1) as isize) as usize;
        self.list_state.select(Some(next));
        self.detail_cursor = 0;
    }

    pub(super) fn select_first(&mut self) {
        self.list_state.select((!self.rows.is_empty()).then_some(0));
        self.detail_cursor = 0;
    }

    pub(super) fn select_last(&mut self) {
        self.list_state.select(self.rows.len().checked_sub(1));
        self.detail_cursor = 0;
    }

    pub(super) fn focus_next(&mut self) {
        self.focus = match self.focus {
            PaneFocus::List => PaneFocus::Facts,
            PaneFocus::Facts => PaneFocus::List,
        };
    }

    pub(super) fn focus_previous(&mut self) {
        self.focus = match self.focus {
            PaneFocus::List => PaneFocus::Facts,
            PaneFocus::Facts => PaneFocus::List,
        };
    }

    pub(super) fn start_search(&mut self, direction: SearchDirection) {
        self.pending_search = Some(SearchPromptState {
            direction,
            query: String::new(),
        });
        self.status = "Search users by login, email, name, role, or teams.".to_string();
    }

    pub(super) fn select_index(&mut self, index: usize) {
        if index < self.rows.len() {
            self.list_state.select(Some(index));
            self.detail_cursor = 0;
        }
    }

    pub(super) fn move_detail_cursor(&mut self, delta: isize, line_count: usize) {
        if line_count == 0 {
            self.detail_cursor = 0;
            return;
        }
        let current = self.detail_cursor as isize;
        self.detail_cursor =
            (current + delta).clamp(0, line_count.saturating_sub(1) as isize) as usize;
    }

    pub(super) fn set_detail_cursor(&mut self, index: usize, line_count: usize) {
        self.detail_cursor = if line_count == 0 {
            0
        } else {
            index.min(line_count.saturating_sub(1))
        };
    }

    pub(super) fn selected_user_id(&self) -> Option<String> {
        let row = self.selected_row()?;
        match row_kind(row) {
            "team" => Some(map_get_text(row, "parentUserId")),
            _ => Some(map_get_text(row, "id")),
        }
    }

    pub(super) fn selected_team_membership_row(&self) -> Option<&Map<String, Value>> {
        self.selected_row().filter(|row| row_kind(row) == "team")
    }

    pub(super) fn expand_selected(&mut self) {
        if self.display_mode != DisplayMode::GlobalAccounts {
            return;
        }
        if let Some(user_id) = self.selected_user_id() {
            self.expanded_user_ids.insert(user_id);
            self.rebuild_visible_rows();
        }
    }

    pub(super) fn collapse_selected(&mut self) {
        if self.display_mode != DisplayMode::GlobalAccounts {
            return;
        }
        if let Some(row) = self.selected_row() {
            if row_kind(row) == "team" {
                let parent_id = map_get_text(row, "parentUserId");
                self.expanded_user_ids.remove(&parent_id);
                self.rebuild_visible_rows_preserve(Some(parent_id));
                return;
            }
        }
        if let Some(user_id) = self.selected_user_id() {
            self.expanded_user_ids.remove(&user_id);
            self.rebuild_visible_rows_preserve(Some(user_id));
        }
    }

    pub(super) fn toggle_all_expanded(&mut self) {
        if self.display_mode != DisplayMode::GlobalAccounts {
            return;
        }
        if self.expanded_user_ids.len() == self.base_rows.len() {
            self.expanded_user_ids.clear();
        } else {
            self.expanded_user_ids = self
                .base_rows
                .iter()
                .map(|row| map_get_text(row, "id"))
                .collect::<BTreeSet<_>>();
        }
        self.rebuild_visible_rows();
    }

    fn rebuild_visible_rows(&mut self) {
        let selected_id = self.selected_row().map(|row| map_get_text(row, "id"));
        self.rebuild_visible_rows_preserve(selected_id);
    }

    fn rebuild_visible_rows_preserve(&mut self, selected_id: Option<String>) {
        self.rows = flatten_user_rows(&self.base_rows, self.display_mode, &self.expanded_user_ids);
        let selected = selected_id
            .as_ref()
            .and_then(|id| {
                self.rows
                    .iter()
                    .position(|row| map_get_text(row, "id") == *id)
            })
            .or((!self.rows.is_empty()).then_some(0));
        self.list_state.select(selected);
        self.detail_cursor = 0;
    }

    pub(super) fn find_match(&self, query: &str, direction: SearchDirection) -> Option<usize> {
        self.find_match_from(query, direction, self.list_state.selected())
    }

    pub(super) fn repeat_last_search(&self) -> Option<usize> {
        let search = self.last_search.as_ref()?;
        let next_start = self
            .list_state
            .selected()
            .map(|index| match search.direction {
                SearchDirection::Forward => index.saturating_add(1),
                SearchDirection::Backward => index.saturating_sub(1),
            });
        self.find_match_from(&search.query, search.direction, next_start)
    }

    fn find_match_from(
        &self,
        query: &str,
        direction: SearchDirection,
        start: Option<usize>,
    ) -> Option<usize> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() || self.rows.is_empty() {
            return None;
        }
        match direction {
            SearchDirection::Forward => {
                let start = start.unwrap_or(0).min(self.rows.len().saturating_sub(1));
                (start..self.rows.len()).find(|&index| row_matches(&self.rows[index], &needle))
            }
            SearchDirection::Backward => {
                let start = start.unwrap_or(self.rows.len().saturating_sub(1));
                (0..=start.min(self.rows.len().saturating_sub(1)))
                    .rev()
                    .find(|&index| row_matches(&self.rows[index], &needle))
            }
        }
    }
}

pub(super) fn row_matches(row: &Map<String, Value>, needle: &str) -> bool {
    [
        map_get_text(row, "rowKind"),
        map_get_text(row, "orgName"),
        map_get_text(row, "crossOrgMemberships"),
        map_get_text(row, "id"),
        map_get_text(row, "userId"),
        map_get_text(row, "login"),
        map_get_text(row, "email"),
        map_get_text(row, "name"),
        map_get_text(row, "orgRole"),
        map_get_text(row, "grafanaAdmin"),
        map_get_text(row, "roleSummary"),
        map_get_text(row, "orgMembershipCount"),
        map_get_text(row, "scope"),
        map_get_text(row, "teams"),
        map_get_text(row, "teamName"),
    ]
    .iter()
    .any(|value| value.to_ascii_lowercase().contains(needle))
}

pub(super) fn row_kind(row: &Map<String, Value>) -> &str {
    match row.get("rowKind").and_then(Value::as_str) {
        Some(kind) => kind,
        None => "user",
    }
}

fn flatten_user_rows(
    base_rows: &[Map<String, Value>],
    display_mode: DisplayMode,
    expanded_user_ids: &BTreeSet<String>,
) -> Vec<Map<String, Value>> {
    if display_mode != DisplayMode::GlobalAccounts {
        return base_rows.to_vec();
    }
    let mut rows = Vec::new();
    for row in base_rows {
        let user_id = map_get_text(row, "id");
        let mut parent = row.clone();
        parent.insert("rowKind".to_string(), Value::String("user".to_string()));
        parent.insert(
            "expanded".to_string(),
            Value::String(if expanded_user_ids.contains(&user_id) {
                "true".to_string()
            } else {
                "false".to_string()
            }),
        );
        rows.push(parent);
        if expanded_user_ids.contains(&user_id) {
            if let Some(Value::Array(team_rows)) = row.get("teamRows") {
                for (index, team) in team_rows.iter().enumerate() {
                    let Value::Object(team) = team else {
                        continue;
                    };
                    let team_name = map_get_text(team, "teamName");
                    if team_name.is_empty() {
                        continue;
                    }
                    rows.push(Map::from_iter(vec![
                        (
                            "id".to_string(),
                            Value::String(format!("{user_id}::team::{index}")),
                        ),
                        ("rowKind".to_string(), Value::String("team".to_string())),
                        ("parentUserId".to_string(), Value::String(user_id.clone())),
                        (
                            "parentLogin".to_string(),
                            Value::String(map_get_text(row, "login")),
                        ),
                        (
                            "parentTeamId".to_string(),
                            Value::String(map_get_text(team, "teamId")),
                        ),
                        (
                            "parentTeamName".to_string(),
                            Value::String(team_name.clone()),
                        ),
                        ("teamName".to_string(), Value::String(team_name.clone())),
                        ("name".to_string(), Value::String(team_name)),
                    ]));
                }
            } else if let Some(Value::Array(teams)) = row.get("teams") {
                for (index, team) in teams.iter().enumerate() {
                    let Some(team_name) = team.as_str() else {
                        continue;
                    };
                    rows.push(Map::from_iter(vec![
                        (
                            "id".to_string(),
                            Value::String(format!("{user_id}::team::{index}")),
                        ),
                        ("rowKind".to_string(), Value::String("team".to_string())),
                        ("parentUserId".to_string(), Value::String(user_id.clone())),
                        (
                            "parentLogin".to_string(),
                            Value::String(map_get_text(row, "login")),
                        ),
                        ("teamName".to_string(), Value::String(team_name.to_string())),
                        ("name".to_string(), Value::String(team_name.to_string())),
                    ]));
                }
            }
        }
    }
    rows
}

pub(super) fn row_matches_args(row: &Map<String, Value>, args: &UserBrowseArgs) -> bool {
    if let Some(query) = &args.query {
        let query = query.to_ascii_lowercase();
        if !row_matches(row, &query) {
            return false;
        }
    }
    if let Some(login) = &args.login {
        if map_get_text(row, "login") != *login {
            return false;
        }
    }
    if let Some(email) = &args.email {
        if map_get_text(row, "email") != *email {
            return false;
        }
    }
    if let Some(role) = &args.org_role {
        if map_get_text(row, "orgRole") != *role {
            return false;
        }
    }
    if let Some(admin) = args.grafana_admin {
        if map_get_text(row, "grafanaAdmin") != if admin { "true" } else { "false" } {
            return false;
        }
    }
    true
}

impl super::user_browse_dialog::EditDialogState {
    pub(super) fn handle_key(
        &mut self,
        key: &KeyEvent,
    ) -> super::user_browse_dialog::EditDialogAction {
        match key.code {
            KeyCode::Esc => super::user_browse_dialog::EditDialogAction::Cancel,
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                super::user_browse_dialog::EditDialogAction::Cancel
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                super::user_browse_dialog::EditDialogAction::Save
            }
            KeyCode::BackTab => {
                self.active_field = self.active_field.saturating_sub(1);
                super::user_browse_dialog::EditDialogAction::None
            }
            KeyCode::Tab => {
                self.active_field = (self.active_field + 1).min(4);
                super::user_browse_dialog::EditDialogAction::None
            }
            KeyCode::Backspace => {
                self.active_value_mut().pop();
                super::user_browse_dialog::EditDialogAction::None
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.active_value_mut().push(ch);
                super::user_browse_dialog::EditDialogAction::None
            }
            _ => super::user_browse_dialog::EditDialogAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{row_kind, BrowserState, DisplayMode};
    use serde_json::{Map, Value};

    #[test]
    fn user_browse_state_expand_selected_builds_team_child_rows_for_global_accounts() {
        let rows = vec![Map::from_iter(vec![
            ("id".to_string(), Value::String("7".to_string())),
            ("login".to_string(), Value::String("alice".to_string())),
            (
                "teams".to_string(),
                Value::Array(vec![
                    Value::String("platform-ops".to_string()),
                    Value::String("qa-observers".to_string()),
                ]),
            ),
        ])];
        let mut state = BrowserState::new(rows, DisplayMode::GlobalAccounts);
        state.expand_selected();
        assert_eq!(state.rows.len(), 3);
        assert_eq!(row_kind(&state.rows[0]), "user");
        assert_eq!(row_kind(&state.rows[1]), "team");
        assert_eq!(row_kind(&state.rows[2]), "team");
    }
}
