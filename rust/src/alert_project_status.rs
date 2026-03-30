//! Shared alert domain-status producer.
//!
//! Maintainer note:
//! - This module derives one alert-owned domain-status row from the staged
//!   alert export summary document.
//! - Keep the producer document-driven and conservative; it should rely only on
//!   stable summary counts that already exist in the overview artifact.
//! - Alert rule/contact-point/policy counts drive readiness; mute timing and
//!   template counts stay as passive coverage signals.

use serde_json::Value;

use crate::project_status::{ProjectDomainStatus, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY};

const ALERT_DOMAIN_ID: &str = "alert";
const ALERT_SCOPE: &str = "staged";
const ALERT_MODE: &str = "artifact-summary";
const ALERT_REASON_READY: &str = PROJECT_STATUS_READY;
const ALERT_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";

const ALERT_SOURCE_KINDS: &[&str] = &["alert-export"];
const ALERT_SIGNAL_KEYS: &[&str] = &[
    "summary.ruleCount",
    "summary.contactPointCount",
    "summary.policyCount",
    "summary.muteTimingCount",
    "summary.templateCount",
];

const ALERT_EXPORT_AT_LEAST_ONE_ACTIONS: &[&str] =
    &["export at least one alert rule, contact point, or policy"];
const ALERT_REEXPORT_AFTER_CHANGES_ACTIONS: &[&str] =
    &["re-run alert export after alerting changes"];
const ALERT_MISSING_CONTACT_POINTS_ACTIONS: &[&str] =
    &["capture at least one contact point before promotion handoff"];
const ALERT_MISSING_POLICIES_ACTIONS: &[&str] =
    &["capture at least one notification policy before promotion handoff"];
const ALERT_MISSING_MUTE_TIMINGS_ACTIONS: &[&str] =
    &["capture at least one mute timing before promotion handoff"];
const ALERT_MISSING_TEMPLATES_ACTIONS: &[&str] =
    &["capture at least one notification template before promotion handoff"];
const ALERT_WARNING_MISSING_CONTACT_POINTS: &str = "missing-contact-points";
const ALERT_WARNING_MISSING_POLICIES: &str = "missing-policies";
const ALERT_WARNING_MISSING_MUTE_TIMINGS: &str = "missing-mute-timings";
const ALERT_WARNING_MISSING_TEMPLATES: &str = "missing-templates";

fn summary_number(document: &Value, key: &str) -> usize {
    document
        .get("summary")
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn push_next_actions(next_actions: &mut Vec<String>, actions: &[&str]) {
    for action in actions {
        if !next_actions.iter().any(|item| item == action) {
            next_actions.push((*action).to_string());
        }
    }
}

fn add_missing_supporting_surface_signal(
    warnings: &mut Vec<crate::project_status::ProjectStatusFinding>,
    next_actions: &mut Vec<String>,
    primary_count: usize,
    count: usize,
    warning_kind: &str,
    source_key: &str,
    action: &[&str],
) {
    if primary_count > 0 && count == 0 {
        warnings.push(crate::project_status::status_finding(
            warning_kind,
            1,
            source_key,
        ));
        push_next_actions(next_actions, action);
    }
}

pub(crate) fn build_alert_project_status_domain(
    summary_document: Option<&Value>,
) -> Option<ProjectDomainStatus> {
    let document = summary_document?;
    let rules = summary_number(document, "ruleCount");
    let contact_points = summary_number(document, "contactPointCount");
    let policies = summary_number(document, "policyCount");

    let primary_count = rules.max(contact_points).max(policies);
    let is_partial = primary_count == 0;
    let (status, reason_code, next_actions) = if is_partial {
        (
            PROJECT_STATUS_PARTIAL,
            ALERT_REASON_PARTIAL_NO_DATA,
            ALERT_EXPORT_AT_LEAST_ONE_ACTIONS,
        )
    } else {
        (
            PROJECT_STATUS_READY,
            ALERT_REASON_READY,
            ALERT_REEXPORT_AFTER_CHANGES_ACTIONS,
        )
    };

    let mut warnings = Vec::new();
    let mut next_actions = next_actions
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<String>>();
    add_missing_supporting_surface_signal(
        &mut warnings,
        &mut next_actions,
        rules,
        contact_points,
        ALERT_WARNING_MISSING_CONTACT_POINTS,
        "summary.contactPointCount",
        ALERT_MISSING_CONTACT_POINTS_ACTIONS,
    );
    add_missing_supporting_surface_signal(
        &mut warnings,
        &mut next_actions,
        rules,
        policies,
        ALERT_WARNING_MISSING_POLICIES,
        "summary.policyCount",
        ALERT_MISSING_POLICIES_ACTIONS,
    );
    add_missing_supporting_surface_signal(
        &mut warnings,
        &mut next_actions,
        primary_count,
        summary_number(document, "muteTimingCount"),
        ALERT_WARNING_MISSING_MUTE_TIMINGS,
        "summary.muteTimingCount",
        ALERT_MISSING_MUTE_TIMINGS_ACTIONS,
    );
    add_missing_supporting_surface_signal(
        &mut warnings,
        &mut next_actions,
        primary_count,
        summary_number(document, "templateCount"),
        ALERT_WARNING_MISSING_TEMPLATES,
        "summary.templateCount",
        ALERT_MISSING_TEMPLATES_ACTIONS,
    );

    Some(ProjectDomainStatus {
        id: ALERT_DOMAIN_ID.to_string(),
        scope: ALERT_SCOPE.to_string(),
        mode: ALERT_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count,
        blocker_count: 0,
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds: ALERT_SOURCE_KINDS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        signal_keys: ALERT_SIGNAL_KEYS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        blockers: Vec::new(),
        warnings,
        next_actions,
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod alert_project_status_rust_tests {
    use super::build_alert_project_status_domain;
    use crate::project_status::{PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY};
    use serde_json::json;

    #[test]
    fn build_alert_project_status_domain_is_partial_without_core_counts() {
        let summary_document = json!({
            "summary": {
                "ruleCount": 0,
                "contactPointCount": 0,
                "policyCount": 0,
                "muteTimingCount": 2,
                "templateCount": 1
            }
        });
        let domain = build_alert_project_status_domain(Some(&summary_document)).unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!(PROJECT_STATUS_PARTIAL));
        assert_eq!(value["reasonCode"], json!("partial-no-data"));
        assert_eq!(value["primaryCount"], json!(0));
        assert_eq!(value["warningCount"], json!(0));
        assert_eq!(value["warnings"], json!([]));
        assert_eq!(
            value["nextActions"],
            json!(["export at least one alert rule, contact point, or policy"])
        );
    }

    #[test]
    fn build_alert_project_status_domain_is_ready_from_core_counts() {
        let summary_document = json!({
            "summary": {
                "ruleCount": 4,
                "contactPointCount": 2,
                "policyCount": 3,
                "muteTimingCount": 1,
                "templateCount": 5
            }
        });
        let domain = build_alert_project_status_domain(Some(&summary_document)).unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["reasonCode"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["primaryCount"], json!(4));
        assert_eq!(value["warningCount"], json!(0));
        assert_eq!(
            value["nextActions"],
            json!(["re-run alert export after alerting changes"])
        );
    }

    #[test]
    fn build_alert_project_status_domain_adds_supporting_surface_warnings() {
        let summary_document = json!({
            "summary": {
                "ruleCount": 2,
                "contactPointCount": 0,
                "policyCount": 0,
                "muteTimingCount": 0,
                "templateCount": 0
            }
        });
        let domain = build_alert_project_status_domain(Some(&summary_document)).unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["reasonCode"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["primaryCount"], json!(2));
        assert_eq!(value["warningCount"], json!(4));
        assert_eq!(
            value["warnings"],
            json!([
                {
                    "kind": "missing-contact-points",
                    "count": 1,
                    "source": "summary.contactPointCount"
                },
                {
                    "kind": "missing-policies",
                    "count": 1,
                    "source": "summary.policyCount"
                },
                {
                    "kind": "missing-mute-timings",
                    "count": 1,
                    "source": "summary.muteTimingCount"
                },
                {
                    "kind": "missing-templates",
                    "count": 1,
                    "source": "summary.templateCount"
                }
            ])
        );
        assert_eq!(
            value["nextActions"],
            json!([
                "re-run alert export after alerting changes",
                "capture at least one contact point before promotion handoff",
                "capture at least one notification policy before promotion handoff",
                "capture at least one mute timing before promotion handoff",
                "capture at least one notification template before promotion handoff"
            ])
        );
    }
}
