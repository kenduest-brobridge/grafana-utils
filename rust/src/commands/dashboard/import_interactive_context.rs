#![cfg(feature = "tui")]

use ratatui::text::Line;

use super::import_interactive::{
    InteractiveImportContextView, InteractiveImportDiffDepth, InteractiveImportReviewState,
    InteractiveImportState, InteractiveImportSummaryCounts, InteractiveImportSummaryScope,
};

pub(crate) fn build_context_lines(state: &InteractiveImportState) -> Vec<Line<'static>> {
    match state.context_view {
        InteractiveImportContextView::Summary => build_summary_context_lines(state),
        InteractiveImportContextView::Destination => build_destination_context_lines(state),
        InteractiveImportContextView::Diff => build_diff_context_lines(state),
    }
}

fn build_summary_context_lines(state: &InteractiveImportState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match state.summary_scope {
        InteractiveImportSummaryScope::Focused => {
            let Some(item) = state.selected_item() else {
                return vec![Line::from("No dashboard selected.")];
            };
            lines.push(Line::from(format!(
                "Scope=focused   uid={}   title={}",
                item.uid, item.title
            )));
            lines.push(Line::from(format!(
                "Selected={}   Group={}",
                state.selected_paths.contains(&item.path),
                state.grouping.label()
            )));
            match &item.review {
                InteractiveImportReviewState::Pending => {
                    lines.push(Line::from("Review pending for the focused dashboard."));
                }
                InteractiveImportReviewState::Failed(error) => {
                    lines.push(Line::from("Review blocked for the focused dashboard."));
                    lines.push(Line::from(error.clone()));
                }
                InteractiveImportReviewState::Resolved(review) => {
                    lines.push(Line::from(format!(
                        "Action={}   Destination={}   Diff={}",
                        review.action_label, review.destination, review.diff_status
                    )));
                    if !review.reason.is_empty() {
                        lines.push(Line::from(format!("Reason={}", review.reason)));
                    }
                }
            }
        }
        InteractiveImportSummaryScope::Selected | InteractiveImportSummaryScope::All => {
            let counts = summary_counts_for_scope(state, state.summary_scope);
            lines.push(Line::from(format!(
                "Scope={}   total={}   selected={}   reviewed={}   pending={}",
                state.summary_scope.label(),
                counts.total,
                counts.selected,
                counts.reviewed,
                counts.pending
            )));
            lines.push(Line::from(format!(
                "create={}   update={}   skip-missing={}   skip-folder={}   blocked={}",
                counts.create,
                counts.update,
                counts.skip_missing,
                counts.skip_folder,
                counts.blocked
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Use `s` to switch focused / selected / all scope.",
    ));
    lines
}

fn build_destination_context_lines(state: &InteractiveImportState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match state.summary_scope {
        InteractiveImportSummaryScope::Focused => {
            let Some(item) = state.selected_item() else {
                return vec![Line::from("No dashboard selected.")];
            };
            lines.push(Line::from(format!("Scope=focused   uid={}", item.uid)));
            match &item.review {
                InteractiveImportReviewState::Pending => {
                    lines.push(Line::from("Destination status pending review."));
                }
                InteractiveImportReviewState::Failed(error) => {
                    lines.push(Line::from("Destination status blocked."));
                    lines.push(Line::from(error.clone()));
                }
                InteractiveImportReviewState::Resolved(review) => {
                    lines.push(Line::from(format!(
                        "Destination={}   Action={}",
                        review.destination, review.action_label
                    )));
                    lines.push(Line::from(format!("Target Folder={}", review.folder_path)));
                    if !review.destination_folder_path.is_empty() {
                        lines.push(Line::from(format!(
                            "Existing Folder={}",
                            review.destination_folder_path
                        )));
                    }
                    lines.push(Line::from(format!("Live Diff={}", review.diff_status)));
                }
            }
        }
        InteractiveImportSummaryScope::Selected | InteractiveImportSummaryScope::All => {
            let destination = destination_counts_for_scope(state, state.summary_scope);
            lines.push(Line::from(format!(
                "Scope={}   total={}   exists={}   missing={}   blocked={}",
                state.summary_scope.label(),
                destination.total,
                destination.exists,
                destination.missing,
                destination.blocked
            )));
            lines.push(Line::from(format!(
                "matches-live={}   changed={}   new-dashboard={}",
                destination.matches_live, destination.changed, destination.new_dashboard
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Use `s` to switch destination scope."));
    lines
}

fn build_diff_context_lines(state: &InteractiveImportState) -> Vec<Line<'static>> {
    let Some(item) = state.selected_item() else {
        return vec![Line::from("No dashboard selected.")];
    };
    let mut lines = vec![Line::from(format!(
        "Scope=focused   uid={}   diff-depth={}",
        item.uid,
        state.diff_depth.label()
    ))];
    match &item.review {
        InteractiveImportReviewState::Pending => {
            lines.push(Line::from("Diff pending review."));
        }
        InteractiveImportReviewState::Failed(error) => {
            lines.push(Line::from("Diff blocked because review failed."));
            lines.push(Line::from(error.clone()));
        }
        InteractiveImportReviewState::Resolved(review) => {
            lines.push(Line::from(format!("Diff status={}", review.diff_status)));
            let selected_lines = match state.diff_depth {
                InteractiveImportDiffDepth::Summary => &review.diff_summary_lines,
                InteractiveImportDiffDepth::Structural => &review.diff_structural_lines,
                InteractiveImportDiffDepth::Raw => &review.diff_raw_lines,
            };
            if selected_lines.is_empty() {
                lines.push(Line::from("No diff details available."));
            } else {
                lines.extend(selected_lines.iter().cloned().map(Line::from));
            }
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Use `d` to switch summary / structural / raw diff.",
    ));
    lines
}

fn summary_counts_for_scope(
    state: &InteractiveImportState,
    scope: InteractiveImportSummaryScope,
) -> InteractiveImportSummaryCounts {
    let mut counts = InteractiveImportSummaryCounts::default();
    for item in &state.items {
        let included = match scope {
            InteractiveImportSummaryScope::Focused => state
                .selected_item()
                .is_some_and(|focused| focused.path == item.path),
            InteractiveImportSummaryScope::Selected => state.selected_paths.contains(&item.path),
            InteractiveImportSummaryScope::All => true,
        };
        if !included {
            continue;
        }
        counts.total += 1;
        if state.selected_paths.contains(&item.path) {
            counts.selected += 1;
        }
        match &item.review {
            InteractiveImportReviewState::Pending => counts.pending += 1,
            InteractiveImportReviewState::Failed(_) => counts.blocked += 1,
            InteractiveImportReviewState::Resolved(review) => match review.action_label.as_str() {
                "create" => counts.create += 1,
                "update" => counts.update += 1,
                "skip-missing" => counts.skip_missing += 1,
                "skip-folder-mismatch" => counts.skip_folder += 1,
                "blocked-existing" => counts.blocked += 1,
                _ => {}
            },
        }
    }
    counts.reviewed = counts.total.saturating_sub(counts.pending);
    counts
}

#[derive(Default)]
struct DestinationCounts {
    total: usize,
    exists: usize,
    missing: usize,
    blocked: usize,
    matches_live: usize,
    changed: usize,
    new_dashboard: usize,
}

fn destination_counts_for_scope(
    state: &InteractiveImportState,
    scope: InteractiveImportSummaryScope,
) -> DestinationCounts {
    let mut counts = DestinationCounts::default();
    for item in &state.items {
        let included = match scope {
            InteractiveImportSummaryScope::Focused => state
                .selected_item()
                .is_some_and(|focused| focused.path == item.path),
            InteractiveImportSummaryScope::Selected => state.selected_paths.contains(&item.path),
            InteractiveImportSummaryScope::All => true,
        };
        if !included {
            continue;
        }
        counts.total += 1;
        match &item.review {
            InteractiveImportReviewState::Pending => {}
            InteractiveImportReviewState::Failed(_) => counts.blocked += 1,
            InteractiveImportReviewState::Resolved(review) => {
                if review.destination == "exists" {
                    counts.exists += 1;
                } else if review.destination == "missing" {
                    counts.missing += 1;
                }
                match review.diff_status.as_str() {
                    "matches live" => counts.matches_live += 1,
                    "changed" => counts.changed += 1,
                    "new dashboard" => counts.new_dashboard += 1,
                    _ => {}
                }
            }
        }
    }
    counts
}
