#[allow(dead_code)]
#[path = "../src/common.rs"]
mod common;
#[path = "../src/project_status.rs"]
mod project_status;
#[path = "../src/project_status_tui.rs"]
mod project_status_tui;

use project_status::{
    build_project_status, status_finding, ProjectDomainStatus, ProjectStatusFinding,
    ProjectStatusFreshness, PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY,
};
use project_status_tui::{ProjectStatusPane, ProjectStatusTuiState};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn domain(
    id: &str,
    status: &str,
    reason_code: &str,
    blocker_count: usize,
    warning_count: usize,
    next_actions: Vec<&str>,
    blockers: Vec<ProjectStatusFinding>,
    warnings: Vec<ProjectStatusFinding>,
) -> ProjectDomainStatus {
    ProjectDomainStatus {
        id: id.to_string(),
        scope: "staged".to_string(),
        mode: format!("{id}-mode"),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: 1,
        blocker_count,
        warning_count,
        source_kinds: vec![format!("{id}-export")],
        signal_keys: vec![format!("{id}.signals")],
        blockers,
        warnings,
        next_actions: next_actions
            .into_iter()
            .map(|action| action.to_string())
            .collect(),
        freshness: ProjectStatusFreshness {
            status: "current".to_string(),
            source_count: 1,
            newest_age_seconds: Some(60),
            oldest_age_seconds: Some(120),
        },
    }
}

fn sample_project_status() -> project_status::ProjectStatus {
    build_project_status(
        "staged",
        3,
        ProjectStatusFreshness {
            status: "current".to_string(),
            source_count: 3,
            newest_age_seconds: Some(30),
            oldest_age_seconds: Some(120),
        },
        vec![
            domain(
                "dashboard",
                PROJECT_STATUS_READY,
                PROJECT_STATUS_READY,
                0,
                1,
                vec!["review dashboard governance warnings before promotion or apply"],
                Vec::new(),
                vec![status_finding("risk-records", 1, "summary.riskRecordCount")],
            ),
            domain(
                "sync",
                PROJECT_STATUS_BLOCKED,
                "blocked-by-blockers",
                3,
                0,
                vec!["resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"],
                vec![status_finding("sync-blocking", 3, "summary.syncBlockingCount")],
                Vec::new(),
            ),
            domain(
                "promotion",
                PROJECT_STATUS_PARTIAL,
                "blocked-by-warnings",
                0,
                2,
                vec!["review promotion preflight warnings before handoff"],
                Vec::new(),
                vec![status_finding("promotion-warning", 2, "summary.promotionWarningCount")],
            ),
        ],
    )
}

#[test]
fn project_status_tui_hands_off_from_home_to_blocked_domain() {
    let state = ProjectStatusTuiState::new(sample_project_status());

    assert_eq!(state.focus(), ProjectStatusPane::Home);
    assert_eq!(
        state.project_home_target_domain_label().as_deref(),
        Some("sync")
    );
    assert_eq!(
        state
            .project_home_target_action()
            .map(|action| action.domain.as_str()),
        Some("sync")
    );
}

#[test]
fn project_status_tui_home_lines_make_the_handoff_path_explicit() {
    let state = ProjectStatusTuiState::new(sample_project_status());
    let home_lines = state.home_lines().join("\n");

    assert!(home_lines.contains("Recommended handoff domain: sync"));
    assert!(home_lines.contains(
        "Recommended handoff action: sync -> resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"
    ));
    assert!(
        home_lines.contains(
            "Navigation: Enter hands off from Home to the recommended domain and preselects the matching action"
        )
    );
    assert!(state.status_line().contains(
        "Home handoff: sync -> resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"
    ));
}

#[test]
fn project_status_tui_domain_navigation_keeps_action_selection_in_sync() {
    let mut state = ProjectStatusTuiState::new(sample_project_status());

    state.handoff_from_home();
    assert_eq!(state.focus(), ProjectStatusPane::Domains);
    assert_eq!(
        state.current_domain().map(|domain| domain.id.as_str()),
        Some("sync")
    );
    assert_eq!(
        state.current_action().map(|action| action.domain.as_str()),
        Some("sync")
    );

    state.move_domain_selection(1);
    assert_eq!(
        state.current_domain().map(|domain| domain.id.as_str()),
        Some("promotion")
    );
    assert_eq!(
        state.current_action().map(|action| action.domain.as_str()),
        Some("promotion")
    );

    let detail_lines = state.current_domain_lines().join("\n");
    assert!(detail_lines.contains("Domain: promotion"));
    assert!(detail_lines.contains("Next actions:"));
    assert!(detail_lines.contains("review promotion preflight warnings before handoff"));
}

#[test]
fn project_status_tui_render_smoke_renders_the_tui_surfaces() {
    let mut state = ProjectStatusTuiState::new(sample_project_status());
    let backend = TestBackend::new(120, 32);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame| project_status_tui::render_project_status_frame(frame, &mut state))
        .expect("render frame");

    let rendered = format!("{}", terminal.backend());
    assert!(rendered.contains("Project Status Workbench"));
    assert!(rendered.contains("Project Home"));
    assert!(rendered.contains("Domains"));
    assert!(rendered.contains("Domain Detail"));
    assert!(rendered.contains("Actions"));
    assert!(rendered.contains("Recommended handoff domain: sync"));
    assert!(rendered.contains("Enter"));
    assert!(rendered.contains("resolve sync workflow blockers"));
}
