use grafana_utils_rust::alert::{
    build_alert_live_project_status_domain, AlertLiveProjectStatusInputs,
};
use serde_json::json;

#[test]
fn build_alert_live_project_status_domain_is_ready_from_live_counts() {
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
    let policies = json!({"receiver": "grafana-default-email"});
    let templates = json!([{"name": "slack.default"}]);

    let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
        rules_document: Some(&rules),
        contact_points_document: Some(&contact_points),
        mute_timings_document: Some(&mute_timings),
        policies_document: Some(&policies),
        templates_document: Some(&templates),
    })
    .unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["scope"], json!("live"));
    assert_eq!(value["mode"], json!("live-alert-surfaces"));
    assert_eq!(value["primaryCount"], json!(5));
    assert_eq!(value["status"], json!("ready"));
    assert_eq!(value["reasonCode"], json!("ready"));
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
    assert_eq!(value["warnings"], json!([]));
    assert_eq!(
        value["nextActions"],
        json!(["re-run the live alert snapshot after provisioning changes"])
    );
}

#[test]
fn build_alert_live_project_status_domain_warns_when_linked_rules_missing_panel_ids() {
    let rules = json!([
        {
            "uid": "cpu-high",
            "annotations": {
                "__dashboardUid__": "dash-uid"
            }
        },
        {
            "uid": "mem-high",
            "annotations": {
                "__dashboardUid__": "dash-uid-2",
                "__panelId__": "8"
            }
        }
    ]);
    let contact_points = json!([{"uid": "cp-main"}]);
    let mute_timings = json!([{"name": "off-hours"}]);
    let policies = json!({"receiver": "grafana-default-email"});
    let templates = json!([{"name": "slack.default"}]);

    let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
        rules_document: Some(&rules),
        contact_points_document: Some(&contact_points),
        mute_timings_document: Some(&mute_timings),
        policies_document: Some(&policies),
        templates_document: Some(&templates),
    })
    .unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["status"], json!("ready"));
    assert_eq!(value["reasonCode"], json!("ready"));
    assert_eq!(value["warningCount"], json!(1));
    assert_eq!(
        value["warnings"],
        json!([
            {
                "kind": "missing-panel-links",
                "count": 1,
                "source": "live.rulePanelMissingCount",
            }
        ])
    );
    assert_eq!(
        value["nextActions"],
        json!([
            "re-run the live alert snapshot after provisioning changes",
            "capture panel IDs for linked live alert rules before promotion handoff"
        ])
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

    assert_eq!(value["status"], json!("blocked"));
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
    let contact_points = json!([{"uid": "cp-main"}]);
    let policies = json!({"receiver": "grafana-default-email"});

    let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
        rules_document: Some(&rules),
        contact_points_document: Some(&contact_points),
        mute_timings_document: None,
        policies_document: Some(&policies),
        templates_document: None,
    })
    .unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["status"], json!("ready"));
    assert_eq!(value["reasonCode"], json!("ready"));
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

    assert_eq!(value["status"], json!("blocked"));
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
        json!(["capture at least one live alert policy before re-running the live alert snapshot"])
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
    let policies = json!({"receiver": "grafana-default-email"});
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

    assert_eq!(value["status"], json!("ready"));
    assert_eq!(value["reasonCode"], json!("ready"));
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

    assert_eq!(value["status"], json!("partial"));
    assert_eq!(value["reasonCode"], json!("partial-no-data"));
    assert_eq!(value["primaryCount"], json!(0));
    assert_eq!(value["warningCount"], json!(0));
    assert_eq!(
        value["nextActions"],
        json!(["capture at least one live alert resource"])
    );
}
