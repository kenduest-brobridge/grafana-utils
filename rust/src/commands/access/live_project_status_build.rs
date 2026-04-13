use crate::project_status::{ProjectDomainStatus, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY};

use super::{
    LiveReviewSignalGroup, LiveScopeReading, ACCESS_DOMAIN_ID, ACCESS_MODE,
    ACCESS_NO_DATA_NEXT_ACTIONS, ACCESS_READY_NEXT_ACTIONS, ACCESS_REASON_PARTIAL_LIVE_SCOPES,
    ACCESS_REASON_PARTIAL_NO_DATA, ACCESS_REASON_READY, ACCESS_SCOPE, ACCESS_SIGNAL_KEYS,
};

fn build_review_next_actions(readings: &[LiveScopeReading]) -> Vec<String> {
    let import_review_labels = readings
        .iter()
        .flat_map(|reading| reading.review_signals.iter())
        .filter(|signal| matches!(signal.group, LiveReviewSignalGroup::ImportReview))
        .map(|signal| signal.label)
        .collect::<Vec<&str>>();
    let drift_severity_labels = readings
        .iter()
        .flat_map(|reading| reading.review_signals.iter())
        .filter(|signal| matches!(signal.group, LiveReviewSignalGroup::DriftSeverity))
        .map(|signal| signal.label)
        .collect::<Vec<&str>>();
    let mut next_actions = Vec::new();
    if !import_review_labels.is_empty() {
        next_actions.push(format!(
            "review live access import-review signals: {}",
            import_review_labels.join(", ")
        ));
    }
    if !drift_severity_labels.is_empty() {
        next_actions.push(format!(
            "review live access drift-severity signals: {}",
            drift_severity_labels.join(", ")
        ));
    }
    next_actions
}

fn build_next_actions(readings: &[LiveScopeReading], total_count: usize) -> Vec<String> {
    let unreadable_labels = readings
        .iter()
        .filter(|reading| !reading.is_readable())
        .map(|reading| reading.label)
        .collect::<Vec<&str>>();
    let mut next_actions = Vec::new();
    if !unreadable_labels.is_empty() {
        next_actions.push(format!(
            "restore access to unreadable live scopes: {}",
            unreadable_labels.join(", ")
        ));
    }
    next_actions.extend(build_review_next_actions(readings));
    if total_count == 0 {
        next_actions.extend(
            ACCESS_NO_DATA_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    } else if unreadable_labels.is_empty() {
        next_actions.extend(
            ACCESS_READY_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }
    next_actions
}

pub(super) fn build_access_live_domain_status_from_readings(
    readings: &[LiveScopeReading],
) -> Option<ProjectDomainStatus> {
    let mut source_kinds = Vec::new();
    let mut warnings = Vec::new();
    let mut total_count = 0usize;
    let mut unreadable_count = 0usize;

    for reading in readings {
        if let Some(source_kind) = reading.source_kind {
            source_kinds.push(source_kind.to_string());
            total_count += reading.count;
        } else {
            unreadable_count += 1;
        }
        warnings.push(reading.finding());
        warnings.extend(reading.review_signals.iter().map(|signal| signal.finding()));
    }

    let (status, reason_code) = if unreadable_count > 0 {
        (PROJECT_STATUS_PARTIAL, ACCESS_REASON_PARTIAL_LIVE_SCOPES)
    } else if total_count == 0 {
        (PROJECT_STATUS_PARTIAL, ACCESS_REASON_PARTIAL_NO_DATA)
    } else {
        (PROJECT_STATUS_READY, ACCESS_REASON_READY)
    };

    Some(ProjectDomainStatus {
        id: ACCESS_DOMAIN_ID.to_string(),
        scope: ACCESS_SCOPE.to_string(),
        mode: ACCESS_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: total_count,
        blocker_count: 0,
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds,
        signal_keys: ACCESS_SIGNAL_KEYS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        blockers: Vec::new(),
        warnings,
        next_actions: build_next_actions(readings, total_count),
        freshness: Default::default(),
    })
}
