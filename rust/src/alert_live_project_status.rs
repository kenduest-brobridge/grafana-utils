//! Alert live domain-status producer.
//!
//! Maintainer note:
//! - This module derives one alert-owned domain-status row from live alert
//!   list/read surfaces.
//! - Keep the producer conservative and document-driven; it should only rely on
//!   stable counts from the live API responses that the alert domain already
//!   knows how to read.
//! - Rule linkage stays the primary readiness signal; missing linked rules or
//!   policy coverage can block readiness, and empty supporting surfaces can add
//!   conservative coverage warnings when they are directly visible.

use serde_json::Value;

use super::value_to_string;
use crate::project_status::{
    status_finding, ProjectDomainStatus, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY,
};

const ALERT_DOMAIN_ID: &str = "alert";
const ALERT_SCOPE: &str = "live";
const ALERT_MODE: &str = "live-alert-surfaces";
const ALERT_REASON_READY: &str = PROJECT_STATUS_READY;
const ALERT_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const ALERT_REASON_BLOCKED_BY_BLOCKERS: &str = "blocked-by-blockers";

const ALERT_SOURCE_KINDS: &[&str] = &[
    "alert",
    "alert-contact-point",
    "alert-mute-timing",
    "alert-policy",
    "alert-template",
];
const ALERT_SIGNAL_KEYS: &[&str] = &[
    "live.alertRuleCount",
    "live.ruleLinkedCount",
    "live.ruleUnlinkedCount",
    "live.rulePanelMissingCount",
    "live.contactPointCount",
    "live.muteTimingCount",
    "live.policyCount",
    "live.templateCount",
];

const ALERT_EXPORT_AT_LEAST_ONE_ACTIONS: &[&str] = &["capture at least one live alert resource"];
const ALERT_LINK_AT_LEAST_ONE_RULE_ACTIONS: &[&str] = &[
    "link at least one live alert rule to a dashboard before re-running the live alert snapshot",
];
const ALERT_CAPTURE_POLICY_ACTIONS: &[&str] =
    &["capture at least one live alert policy before re-running the live alert snapshot"];
const ALERT_LINK_REMAINING_RULES_ACTIONS: &[&str] =
    &["link remaining live alert rules to dashboards before re-running the live alert snapshot"];
const ALERT_MISSING_CONTACT_POINTS_ACTIONS: &[&str] =
    &["capture at least one live contact point before re-running the live alert snapshot"];
const ALERT_MISSING_MUTE_TIMINGS_ACTIONS: &[&str] =
    &["capture at least one live mute timing before re-running the live alert snapshot"];
const ALERT_MISSING_TEMPLATES_ACTIONS: &[&str] =
    &["capture at least one live notification template before re-running the live alert snapshot"];
const ALERT_REFRESH_AFTER_CHANGES_ACTIONS: &[&str] =
    &["re-run the live alert snapshot after provisioning changes"];
const ALERT_WARNING_MISSING_CONTACT_POINTS: &str = "missing-contact-points";
const ALERT_WARNING_MISSING_MUTE_TIMINGS: &str = "missing-mute-timings";
const ALERT_WARNING_MISSING_TEMPLATES: &str = "missing-templates";
const ALERT_WARNING_MISSING_PANEL_LINKS: &str = "missing-panel-links";
const ALERT_BLOCKER_MISSING_LINKED_RULES: &str = "missing-linked-alert-rules";
const ALERT_BLOCKER_MISSING_POLICY: &str = "missing-alert-policy";

fn blocker_actions(kind: &str) -> &'static [&'static str] {
    match kind {
        ALERT_BLOCKER_MISSING_LINKED_RULES => ALERT_LINK_AT_LEAST_ONE_RULE_ACTIONS,
        ALERT_BLOCKER_MISSING_POLICY => ALERT_CAPTURE_POLICY_ACTIONS,
        _ => &[],
    }
}

fn array_count(document: &Value) -> usize {
    document.as_array().map(Vec::len).unwrap_or(0)
}

fn object_count(document: &Value) -> usize {
    if document.is_object() {
        1
    } else {
        0
    }
}

fn rule_linkage_panel_counts(document: &Value) -> (usize, usize, usize) {
    let Some(rules) = document.as_array() else {
        return (0, 0, 0);
    };

    let mut linked_rules = 0;
    let mut panel_linked_rules = 0;

    for rule in rules {
        let Some(rule) = rule.as_object() else {
            continue;
        };
        let Some(annotations) = rule.get("annotations").and_then(Value::as_object) else {
            continue;
        };

        let has_dashboard_uid = annotations
            .get("__dashboardUid__")
            .map(value_to_string)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if !has_dashboard_uid {
            continue;
        }

        linked_rules += 1;
        let has_panel_id = annotations
            .get("__panelId__")
            .map(value_to_string)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if has_panel_id {
            panel_linked_rules += 1;
        }
    }

    (linked_rules, panel_linked_rules, rules.len())
}

fn push_next_actions(next_actions: &mut Vec<String>, actions: &[&str]) {
    for action in actions {
        if !next_actions.iter().any(|item| item == action) {
            next_actions.push((*action).to_string());
        }
    }
}

fn add_missing_surface_signal(
    warnings: &mut Vec<crate::project_status::ProjectStatusFinding>,
    next_actions: &mut Vec<String>,
    has_linked_rules: bool,
    count: usize,
    warning_kind: &str,
    source_key: &str,
    action: &[&str],
) {
    if has_linked_rules && count == 0 {
        warnings.push(status_finding(warning_kind, 1, source_key));
        push_next_actions(next_actions, action);
    }
}

fn add_missing_panel_link_signal(
    warnings: &mut Vec<crate::project_status::ProjectStatusFinding>,
    next_actions: &mut Vec<String>,
    has_linked_rules: bool,
    linked_rules: usize,
    panel_linked_rules: usize,
) {
    if has_linked_rules && panel_linked_rules < linked_rules {
        warnings.push(status_finding(
            ALERT_WARNING_MISSING_PANEL_LINKS,
            linked_rules - panel_linked_rules,
            "live.rulePanelMissingCount",
        ));
        push_next_actions(
            next_actions,
            &["capture panel IDs for linked live alert rules before promotion handoff"],
        );
    }
}

#[derive(Debug, Clone, Default)]
pub struct AlertLiveProjectStatusInputs<'a> {
    pub rules_document: Option<&'a Value>,
    pub contact_points_document: Option<&'a Value>,
    pub mute_timings_document: Option<&'a Value>,
    pub policies_document: Option<&'a Value>,
    pub templates_document: Option<&'a Value>,
}

pub fn build_alert_live_project_status_domain(
    inputs: AlertLiveProjectStatusInputs<'_>,
) -> Option<ProjectDomainStatus> {
    let rules = inputs.rules_document.map(array_count).unwrap_or(0);
    let (linked_rules, panel_linked_rules, rule_count) = inputs
        .rules_document
        .map(rule_linkage_panel_counts)
        .unwrap_or((0, 0, 0));
    let contact_points = inputs.contact_points_document.map(array_count).unwrap_or(0);
    let mute_timings = inputs.mute_timings_document.map(array_count).unwrap_or(0);
    let policies = inputs.policies_document.map(object_count).unwrap_or(0);
    let templates = inputs.templates_document.map(array_count).unwrap_or(0);

    let source_kinds = [
        inputs.rules_document.map(|_| ALERT_SOURCE_KINDS[0]),
        inputs
            .contact_points_document
            .map(|_| ALERT_SOURCE_KINDS[1]),
        inputs.mute_timings_document.map(|_| ALERT_SOURCE_KINDS[2]),
        inputs.policies_document.map(|_| ALERT_SOURCE_KINDS[3]),
        inputs.templates_document.map(|_| ALERT_SOURCE_KINDS[4]),
    ]
    .into_iter()
    .flatten()
    .map(|item| item.to_string())
    .collect::<Vec<String>>();

    if source_kinds.is_empty() {
        return None;
    }

    let primary_count = rules + contact_points + mute_timings + policies + templates;
    let unlinked_rules = rule_count.saturating_sub(linked_rules);
    let has_linked_rules = linked_rules > 0;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    if rules > 0 && !has_linked_rules {
        blockers.push(status_finding(
            ALERT_BLOCKER_MISSING_LINKED_RULES,
            1,
            "live.ruleLinkedCount",
        ));
    } else if unlinked_rules > 0 {
        warnings.push(status_finding(
            "unlinked-alert-rules",
            unlinked_rules,
            "live.ruleUnlinkedCount",
        ));
    }
    if has_linked_rules && policies == 0 {
        blockers.push(status_finding(
            ALERT_BLOCKER_MISSING_POLICY,
            1,
            "live.policyCount",
        ));
    }

    let (status, reason_code, mut next_actions) = if primary_count == 0 {
        (
            PROJECT_STATUS_PARTIAL,
            ALERT_REASON_PARTIAL_NO_DATA,
            ALERT_EXPORT_AT_LEAST_ONE_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    } else if !blockers.is_empty() {
        (
            crate::project_status::PROJECT_STATUS_BLOCKED,
            ALERT_REASON_BLOCKED_BY_BLOCKERS,
            blockers
                .iter()
                .flat_map(|finding| blocker_actions(finding.kind.as_str()))
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    } else if unlinked_rules > 0 {
        (
            PROJECT_STATUS_READY,
            ALERT_REASON_READY,
            ALERT_LINK_REMAINING_RULES_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    } else {
        (
            PROJECT_STATUS_READY,
            ALERT_REASON_READY,
            ALERT_REFRESH_AFTER_CHANGES_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    };

    add_missing_surface_signal(
        &mut warnings,
        &mut next_actions,
        has_linked_rules,
        contact_points,
        ALERT_WARNING_MISSING_CONTACT_POINTS,
        "live.contactPointCount",
        ALERT_MISSING_CONTACT_POINTS_ACTIONS,
    );
    add_missing_surface_signal(
        &mut warnings,
        &mut next_actions,
        has_linked_rules,
        mute_timings,
        ALERT_WARNING_MISSING_MUTE_TIMINGS,
        "live.muteTimingCount",
        ALERT_MISSING_MUTE_TIMINGS_ACTIONS,
    );
    add_missing_surface_signal(
        &mut warnings,
        &mut next_actions,
        has_linked_rules,
        templates,
        ALERT_WARNING_MISSING_TEMPLATES,
        "live.templateCount",
        ALERT_MISSING_TEMPLATES_ACTIONS,
    );
    add_missing_panel_link_signal(
        &mut warnings,
        &mut next_actions,
        has_linked_rules,
        linked_rules,
        panel_linked_rules,
    );

    Some(ProjectDomainStatus {
        id: ALERT_DOMAIN_ID.to_string(),
        scope: ALERT_SCOPE.to_string(),
        mode: ALERT_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count,
        blocker_count: blockers.iter().map(|item| item.count).sum(),
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds,
        signal_keys: ALERT_SIGNAL_KEYS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        blockers,
        warnings,
        next_actions,
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod alert_live_project_status_rust_tests {
    use super::{build_alert_live_project_status_domain, AlertLiveProjectStatusInputs};
    use crate::project_status::{
        PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY,
    };
    use serde_json::json;

    #[test]
    fn build_alert_live_project_status_domain_returns_none_without_any_surfaces() {
        assert!(
            build_alert_live_project_status_domain(AlertLiveProjectStatusInputs::default())
                .is_none()
        );
    }

    #[test]
    fn build_alert_live_project_status_domain_tracks_live_surface_counts() {
        let rules = json!([
            {
                "uid": "cpu-high",
                "annotations": {
                    "__dashboardUid__": "dash-uid",
                    "__panelId__": "7"
                }
            },
            {
                "uid": "mem-high",
                "annotations": {
                    "__dashboardUid__": "dash-uid-2",
                    "__panelId__": "11"
                }
            }
        ]);
        let contact_points = json!([
            {"uid": "cp-main"}
        ]);
        let mute_timings = json!([
            {"name": "off-hours"}
        ]);
        let policies = json!({
            "receiver": "grafana-default-email"
        });
        let templates = json!([
            {"name": "slack.default"},
            {"name": "email.default"},
            {"name": "pagerduty.default"}
        ]);

        let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
            rules_document: Some(&rules),
            contact_points_document: Some(&contact_points),
            mute_timings_document: Some(&mute_timings),
            policies_document: Some(&policies),
            templates_document: Some(&templates),
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["id"], json!("alert"));
        assert_eq!(value["scope"], json!("live"));
        assert_eq!(value["mode"], json!("live-alert-surfaces"));
        assert_eq!(value["status"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["reasonCode"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["primaryCount"], json!(8));
        assert_eq!(value["blockerCount"], json!(0));
        assert_eq!(value["warningCount"], json!(0));
        assert_eq!(
            value["sourceKinds"],
            json!([
                "alert",
                "alert-contact-point",
                "alert-mute-timing",
                "alert-policy",
                "alert-template"
            ])
        );
        assert_eq!(
            value["signalKeys"],
            json!([
                "live.alertRuleCount",
                "live.ruleLinkedCount",
                "live.ruleUnlinkedCount",
                "live.rulePanelMissingCount",
                "live.contactPointCount",
                "live.muteTimingCount",
                "live.policyCount",
                "live.templateCount",
            ])
        );
        assert_eq!(value["blockers"], json!([]));
        assert_eq!(value["warnings"], json!([]));
        assert_eq!(
            value["nextActions"],
            json!(["re-run the live alert snapshot after provisioning changes"])
        );
    }

    #[test]
    fn build_alert_live_project_status_domain_blocks_when_no_rules_are_linked() {
        let rules = json!([
            {"uid": "cpu-high"},
            {"uid": "mem-high"}
        ]);

        let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
            rules_document: Some(&rules),
            contact_points_document: None,
            mute_timings_document: None,
            policies_document: None,
            templates_document: None,
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!(PROJECT_STATUS_BLOCKED));
        assert_eq!(value["reasonCode"], json!("blocked-by-blockers"));
        assert_eq!(value["primaryCount"], json!(2));
        assert_eq!(value["blockerCount"], json!(1));
        assert_eq!(value["warningCount"], json!(0));
        assert_eq!(
            value["blockers"],
            json!([
                {
                    "kind": "missing-linked-alert-rules",
                    "count": 1,
                    "source": "live.ruleLinkedCount",
                }
            ])
        );
        assert_eq!(value["warnings"], json!([]));
        assert_eq!(
            value["nextActions"],
            json!([
                "link at least one live alert rule to a dashboard before re-running the live alert snapshot"
            ])
        );
    }

    #[test]
    fn build_alert_live_project_status_domain_warns_when_some_rules_are_unlinked() {
        let rules = json!([
            {
                "uid": "cpu-high",
                "annotations": {
                    "__dashboardUid__": "dash-uid",
                    "__panelId__": "7"
                }
            },
            {"uid": "mem-high"}
        ]);
        let contact_points = json!([
            {"uid": "cp-main"}
        ]);
        let policies = json!({
            "receiver": "grafana-default-email"
        });

        let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
            rules_document: Some(&rules),
            contact_points_document: Some(&contact_points),
            mute_timings_document: None,
            policies_document: Some(&policies),
            templates_document: None,
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["reasonCode"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["primaryCount"], json!(4));
        assert_eq!(value["warningCount"], json!(3));
        assert_eq!(
            value["warnings"],
            json!([
                {
                    "kind": "unlinked-alert-rules",
                    "count": 1,
                    "source": "live.ruleUnlinkedCount",
                },
                {
                    "kind": "missing-mute-timings",
                    "count": 1,
                    "source": "live.muteTimingCount",
                },
                {
                    "kind": "missing-templates",
                    "count": 1,
                    "source": "live.templateCount",
                }
            ])
        );
        assert_eq!(
            value["nextActions"],
            json!([
                "link remaining live alert rules to dashboards before re-running the live alert snapshot",
                "capture at least one live mute timing before re-running the live alert snapshot",
                "capture at least one live notification template before re-running the live alert snapshot"
            ])
        );
    }

    #[test]
    fn build_alert_live_project_status_domain_blocks_when_policy_surface_is_missing() {
        let rules = json!([
            {
                "uid": "cpu-high",
                "annotations": {
                    "__dashboardUid__": "dash-uid",
                    "__panelId__": "7"
                }
            }
        ]);
        let contact_points = json!([{"uid": "cp-main"}]);
        let mute_timings = json!([{"name": "off-hours"}]);
        let templates = json!([{"name": "slack.default"}]);

        let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
            rules_document: Some(&rules),
            contact_points_document: Some(&contact_points),
            mute_timings_document: Some(&mute_timings),
            policies_document: None,
            templates_document: Some(&templates),
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!(PROJECT_STATUS_BLOCKED));
        assert_eq!(value["reasonCode"], json!("blocked-by-blockers"));
        assert_eq!(value["blockerCount"], json!(1));
        assert_eq!(
            value["blockers"],
            json!([
                {
                    "kind": "missing-alert-policy",
                    "count": 1,
                    "source": "live.policyCount",
                }
            ])
        );
        assert_eq!(
            value["nextActions"],
            json!([
                "capture at least one live alert policy before re-running the live alert snapshot"
            ])
        );
    }

    #[test]
    fn build_alert_live_project_status_domain_adds_support_surface_warnings_for_linked_rules() {
        let rules = json!([
            {
                "uid": "cpu-high",
                "annotations": {
                    "__dashboardUid__": "dash-uid",
                    "__panelId__": "7"
                }
            }
        ]);
        let contact_points = json!([]);
        let mute_timings = json!([]);
        let policies = json!({
            "receiver": "grafana-default-email"
        });
        let templates = json!([]);

        let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
            rules_document: Some(&rules),
            contact_points_document: Some(&contact_points),
            mute_timings_document: Some(&mute_timings),
            policies_document: Some(&policies),
            templates_document: Some(&templates),
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["reasonCode"], json!(PROJECT_STATUS_READY));
        assert_eq!(value["warningCount"], json!(3));
        assert_eq!(
            value["warnings"],
            json!([
                {
                    "kind": "missing-contact-points",
                    "count": 1,
                    "source": "live.contactPointCount",
                },
                {
                    "kind": "missing-mute-timings",
                    "count": 1,
                    "source": "live.muteTimingCount",
                },
                {
                    "kind": "missing-templates",
                    "count": 1,
                    "source": "live.templateCount",
                }
            ])
        );
        assert_eq!(
            value["nextActions"],
            json!([
                "re-run the live alert snapshot after provisioning changes",
                "capture at least one live contact point before re-running the live alert snapshot",
                "capture at least one live mute timing before re-running the live alert snapshot",
                "capture at least one live notification template before re-running the live alert snapshot"
            ])
        );
    }

    #[test]
    fn build_alert_live_project_status_domain_is_partial_without_live_data() {
        let rules = json!([]);
        let contact_points = json!([]);
        let mute_timings = json!([]);
        let templates = json!([]);

        let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
            rules_document: Some(&rules),
            contact_points_document: Some(&contact_points),
            mute_timings_document: Some(&mute_timings),
            policies_document: None,
            templates_document: Some(&templates),
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!(PROJECT_STATUS_PARTIAL));
        assert_eq!(value["reasonCode"], json!("partial-no-data"));
        assert_eq!(value["primaryCount"], json!(0));
        assert_eq!(
            value["nextActions"],
            json!(["capture at least one live alert resource"])
        );
    }
}
