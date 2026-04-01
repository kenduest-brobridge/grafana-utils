//! Interactive browse workflows and terminal-driven state flow for Access entities.

use std::collections::BTreeSet;

use ratatui::widgets::ListState;
use serde_json::{Map, Value};

use crate::access::render::map_get_text;

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
    pub(super) team_rows: Vec<Map<String, Value>>,
    pub(super) rows: Vec<Map<String, Value>>,
    pub(super) list_state: ListState,
    pub(super) detail_cursor: usize,
    pub(super) focus: PaneFocus,
    pub(super) expanded_team_ids: BTreeSet<String>,
    pub(super) show_numbers: bool,
    pub(super) status: String,
    pub(super) pending_delete: bool,
    pub(super) pending_edit: Option<super::team_browse_dialog::EditDialogState>,
    pub(super) pending_search: Option<SearchPromptState>,
    pub(super) last_search: Option<SearchState>,
}

impl BrowserState {
    pub(super) fn new(team_rows: Vec<Map<String, Value>>) -> Self {
        let mut list_state = ListState::default();
        let rows = flatten_team_rows(&team_rows, &BTreeSet::new());
        list_state.select((!rows.is_empty()).then_some(0));
        Self {
            team_rows,
            rows,
            list_state,
            detail_cursor: 0,
            focus: PaneFocus::List,
            expanded_team_ids: BTreeSet::new(),
            show_numbers: true,
            status: "Loaded team browser. Use Enter/Right expand, Left collapse, c toggle all."
                .to_string(),
            pending_delete: false,
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
        let selected_id = self.selected_row().map(|row| map_get_text(row, "id"));
        self.team_rows = rows;
        self.pending_delete = false;
        self.pending_edit = None;
        self.pending_search = None;
        self.detail_cursor = 0;
        self.rows = flatten_team_rows(&self.team_rows, &self.expanded_team_ids);
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

    pub(super) fn toggle_focus(&mut self) {
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
        self.status = "Search teams by name, email, id, or members.".to_string();
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

    pub(super) fn selected_team_id(&self) -> Option<String> {
        let row = self.selected_row()?;
        match row_kind(row) {
            "member" => Some(map_get_text(row, "parentTeamId")),
            _ => Some(map_get_text(row, "id")),
        }
    }

    pub(super) fn expand_selected(&mut self) {
        if let Some(team_id) = self.selected_team_id() {
            self.expanded_team_ids.insert(team_id);
            self.rebuild_visible_rows();
        }
    }

    pub(super) fn collapse_selected(&mut self) {
        if let Some(row) = self.selected_row() {
            if row_kind(row) == "member" {
                let parent_id = map_get_text(row, "parentTeamId");
                self.expanded_team_ids.remove(&parent_id);
                self.rebuild_visible_rows_preserve(Some(parent_id));
                return;
            }
        }
        if let Some(team_id) = self.selected_team_id() {
            self.expanded_team_ids.remove(&team_id);
            self.rebuild_visible_rows_preserve(Some(team_id));
        }
    }

    pub(super) fn toggle_all_expanded(&mut self) {
        if self.expanded_team_ids.len() == self.team_rows.len() {
            self.expanded_team_ids.clear();
        } else {
            self.expanded_team_ids = self
                .team_rows
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
        self.rows = flatten_team_rows(&self.team_rows, &self.expanded_team_ids);
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

fn row_matches(row: &Map<String, Value>, needle: &str) -> bool {
    [
        map_get_text(row, "name"),
        map_get_text(row, "email"),
        map_get_text(row, "id"),
        map_get_text(row, "members"),
        map_get_text(row, "memberIdentity"),
        map_get_text(row, "memberRole"),
    ]
    .iter()
    .any(|value: &String| value.to_ascii_lowercase().contains(needle))
}

pub(super) fn row_kind(row: &Map<String, Value>) -> &str {
    match row.get("rowKind") {
        Some(Value::String(kind)) => kind,
        _ => "team",
    }
}

fn flatten_team_rows(
    team_rows: &[Map<String, Value>],
    expanded_team_ids: &BTreeSet<String>,
) -> Vec<Map<String, Value>> {
    let mut rows = Vec::new();
    for team in team_rows {
        let team_id = map_get_text(team, "id");
        let mut parent = team.clone();
        parent.insert("rowKind".to_string(), Value::String("team".to_string()));
        parent.insert(
            "expanded".to_string(),
            Value::String(if expanded_team_ids.contains(&team_id) {
                "true".to_string()
            } else {
                "false".to_string()
            }),
        );
        rows.push(parent);
        if expanded_team_ids.contains(&team_id) {
            if let Some(Value::Array(member_rows)) = team.get("memberRows") {
                for (index, member) in member_rows.iter().enumerate() {
                    let Value::Object(member) = member else {
                        continue;
                    };
                    rows.push(Map::from_iter(vec![
                        (
                            "id".to_string(),
                            Value::String(format!("{team_id}::member::{index}")),
                        ),
                        ("rowKind".to_string(), Value::String("member".to_string())),
                        ("parentTeamId".to_string(), Value::String(team_id.clone())),
                        (
                            "parentTeamName".to_string(),
                            Value::String(map_get_text(team, "name")),
                        ),
                        (
                            "name".to_string(),
                            Value::String(map_get_text(member, "memberIdentity")),
                        ),
                        (
                            "memberIdentity".to_string(),
                            Value::String(map_get_text(member, "memberIdentity")),
                        ),
                        (
                            "memberLogin".to_string(),
                            Value::String(map_get_text(member, "memberLogin")),
                        ),
                        (
                            "memberEmail".to_string(),
                            Value::String(map_get_text(member, "memberEmail")),
                        ),
                        (
                            "memberName".to_string(),
                            Value::String(map_get_text(member, "memberName")),
                        ),
                        (
                            "memberRole".to_string(),
                            Value::String(map_get_text(member, "memberRole")),
                        ),
                    ]));
                }
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::{row_kind, BrowserState};
    use serde_json::{Map, Value};

    #[test]
    fn team_browse_state_expand_selected_builds_member_child_rows() {
        let rows = vec![Map::from_iter(vec![
            ("id".to_string(), Value::String("7".to_string())),
            (
                "name".to_string(),
                Value::String("platform-ops".to_string()),
            ),
            (
                "memberRows".to_string(),
                Value::Array(vec![
                    Value::Object(Map::from_iter(vec![
                        (
                            "memberIdentity".to_string(),
                            Value::String("alice".to_string()),
                        ),
                        ("memberRole".to_string(), Value::String("Admin".to_string())),
                    ])),
                    Value::Object(Map::from_iter(vec![
                        (
                            "memberIdentity".to_string(),
                            Value::String("bob".to_string()),
                        ),
                        (
                            "memberRole".to_string(),
                            Value::String("Member".to_string()),
                        ),
                    ])),
                ]),
            ),
        ])];
        let mut state = BrowserState::new(rows);
        state.expand_selected();
        assert_eq!(state.rows.len(), 3);
        assert_eq!(row_kind(&state.rows[0]), "team");
        assert_eq!(row_kind(&state.rows[1]), "member");
        assert_eq!(row_kind(&state.rows[2]), "member");
    }
}
