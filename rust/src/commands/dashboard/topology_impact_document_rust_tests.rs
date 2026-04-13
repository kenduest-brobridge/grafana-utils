//! Rust regression coverage for Dashboard behavior at this module boundary.

use super::test_support::{
    build_impact_document, build_topology_document, render_impact_text, render_topology_dot,
    render_topology_mermaid,
};
use serde_json::json;

#[test]
fn build_topology_document_renders_mermaid_and_dot_edges() {
    let governance = json!({
        "dashboardGovernance": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "panelCount": 2,
                "queryCount": 3
            }
        ],
        "dashboardDatasourceEdges": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "datasourceUid": "prom-main",
                "datasource": "Prometheus Main",
                "family": "prometheus",
                "panelCount": 2,
                "queryCount": 3
            }
        ]
    });
    let alert_contract = json!({
        "resources": [
            {
                "kind": "grafana-alert-rule",
                "identity": "cpu-high",
                "title": "CPU High",
                "sourcePath": "rules/cpu-high.json",
                "references": ["prom-main", "cpu-main", "pagerduty-primary", "paging-policy", "slack.default"]
            },
            {
                "kind": "grafana-contact-point",
                "identity": "pagerduty-primary",
                "title": "PagerDuty Primary",
                "sourcePath": "contact-points/pagerduty-primary.json",
                "references": ["slack.default"]
            },
            {
                "kind": "grafana-mute-timing",
                "identity": "off-hours",
                "title": "Off Hours",
                "sourcePath": "mute-timings/off-hours.json",
                "references": []
            },
            {
                "kind": "grafana-notification-policies",
                "identity": "paging-policy",
                "title": "Paging Policy",
                "sourcePath": "policies/paging-policy.json",
                "references": ["slack.default"]
            },
            {
                "kind": "grafana-notification-template",
                "identity": "slack.default",
                "title": "Slack Default",
                "sourcePath": "templates/slack.default.json",
                "references": []
            }
        ]
    });

    let document = build_topology_document(&governance, Some(&alert_contract)).unwrap();
    assert_eq!(document.summary.datasource_count, 1);
    assert_eq!(document.summary.dashboard_count, 1);
    assert_eq!(document.summary.alert_resource_count, 5);
    assert_eq!(document.summary.alert_rule_count, 1);
    assert_eq!(document.summary.contact_point_count, 1);
    assert_eq!(document.summary.mute_timing_count, 1);
    assert_eq!(document.summary.notification_policy_count, 1);
    assert_eq!(document.summary.template_count, 1);
    assert_eq!(document.summary.node_count, 7);
    assert_eq!(document.summary.edge_count, 8);
    assert!(document
        .edges
        .iter()
        .any(|edge| edge.from == "datasource:prom-main" && edge.to == "dashboard:cpu-main"));
    assert!(document
        .edges
        .iter()
        .any(|edge| edge.from == "datasource:prom-main"
            && edge.to == "alert:grafana-alert-rule:cpu-high"));
    assert!(document.edges.iter().any(|edge| {
        edge.from == "dashboard:cpu-main"
            && edge.to == "alert:grafana-alert-rule:cpu-high"
            && edge.relation == "backs"
    }));
    assert!(document.edges.iter().any(|edge| {
        edge.from == "alert:grafana-alert-rule:cpu-high"
            && edge.to == "alert:grafana-contact-point:pagerduty-primary"
            && edge.relation == "routes-to"
    }));
    assert!(document.edges.iter().any(|edge| {
        edge.from == "alert:grafana-contact-point:pagerduty-primary"
            && edge.to == "alert:grafana-notification-template:slack.default"
            && edge.relation == "uses-template"
    }));
    assert!(document.edges.iter().any(|edge| {
        edge.from == "alert:grafana-notification-policies:paging-policy"
            && edge.to == "alert:grafana-notification-template:slack.default"
            && edge.relation == "uses-template"
    }));

    let mermaid = render_topology_mermaid(&document);
    assert!(mermaid.contains("graph TD"));
    assert!(mermaid.contains("datasource_prom_main"));
    assert!(mermaid.contains("alert_grafana_alert_rule_cpu_high"));
    assert!(mermaid.contains("alert_grafana_contact_point_pagerduty_primary"));
    assert!(mermaid.contains("alert_grafana_notification_policies_paging_policy"));
    assert!(mermaid.contains("alert_grafana_notification_template_slack_default"));
    assert!(mermaid.contains("alert_grafana_mute_timing_off_hours"));
    assert!(mermaid.contains("alert_grafana_alert_rule_cpu_high -->|routes-to| alert_grafana_contact_point_pagerduty_primary"));
    assert!(mermaid.contains("alert_grafana_contact_point_pagerduty_primary -->|uses-template| alert_grafana_notification_template_slack_default"));

    let dot = render_topology_dot(&document);
    assert!(dot.contains("digraph grafana_topology"));
    assert!(dot.contains("\"datasource:prom-main\" -> \"dashboard:cpu-main\""));
    assert!(dot.contains("\"alert:grafana-alert-rule:cpu-high\" [label=\"CPU High\\nalert-rule\"]"));
    assert!(dot.contains("\"alert:grafana-contact-point:pagerduty-primary\" [label=\"PagerDuty Primary\\ncontact-point\"]"));
    assert!(dot.contains("\"alert:grafana-notification-policies:paging-policy\" [label=\"Paging Policy\\nnotification-policy\"]"));
    assert!(dot.contains("\"alert:grafana-notification-template:slack.default\" [label=\"Slack Default\\ntemplate\"]"));
}

#[test]
fn build_impact_document_summarizes_dashboards_and_alert_resources() {
    let governance = json!({
        "dashboardGovernance": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "panelCount": 2,
                "queryCount": 3
            },
            {
                "dashboardUid": "logs-main",
                "dashboardTitle": "Logs Main",
                "folderPath": "Platform",
                "panelCount": 1,
                "queryCount": 1
            }
        ],
        "dashboardDatasourceEdges": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "datasourceUid": "prom-main",
                "datasource": "Prometheus Main",
                "family": "prometheus",
                "panelCount": 2,
                "queryCount": 3
            },
            {
                "dashboardUid": "logs-main",
                "dashboardTitle": "Logs Main",
                "folderPath": "Platform",
                "datasourceUid": "logs-main",
                "datasource": "Logs Main",
                "family": "loki",
                "panelCount": 1,
                "queryCount": 1
            }
        ]
    });
    let alert_contract = json!({
        "resources": [
            {
                "kind": "grafana-alert-rule",
                "identity": "cpu-high",
                "title": "CPU High",
                "sourcePath": "rules/cpu-high.json",
                "references": ["prom-main"]
            },
            {
                "kind": "grafana-alert-rule",
                "identity": "logs-high",
                "title": "Logs High",
                "sourcePath": "rules/logs-high.json",
                "references": ["logs-main"]
            }
        ]
    });

    let document = build_impact_document(&governance, Some(&alert_contract), "prom-main").unwrap();
    assert_eq!(document.summary.dashboard_count, 1);
    assert_eq!(document.summary.alert_resource_count, 1);
    assert_eq!(document.dashboards[0].dashboard_uid, "cpu-main");
    assert_eq!(document.alert_resources[0].identity, "cpu-high");

    let rendered = render_impact_text(&document);
    assert!(rendered.contains("Datasource impact: prom-main"));
    assert!(rendered.contains("cpu-main"));
    assert!(rendered.contains("cpu-high"));
}

#[test]
fn build_dashboard_topology_document_renders_mermaid_and_dot() {
    let governance = json!({
        "dashboardGovernance": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main"
            }
        ],
        "dashboardDatasourceEdges": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "datasourceUid": "prom-main",
                "datasource": "Prometheus Main"
            }
        ]
    });
    let alert_contract = json!({
        "kind": "grafana-utils-sync-alert-contract",
        "resources": [
            {
                "kind": "grafana-alert-rule",
                "identity": "cpu-high",
                "title": "CPU High",
                "references": ["prom-main", "cpu-main"]
            }
        ]
    });

    let document = build_topology_document(&governance, Some(&alert_contract)).unwrap();
    assert_eq!(document.summary.datasource_count, 1);
    assert_eq!(document.summary.dashboard_count, 1);
    assert_eq!(document.summary.alert_resource_count, 1);
    assert_eq!(document.summary.edge_count, 3);
    assert_eq!(document.summary.node_count, 3);
    assert_eq!(document.nodes.len(), 3);
    assert_eq!(document.edges.len(), 3);

    let mermaid = render_topology_mermaid(&document);
    assert!(mermaid.starts_with("graph TD"));
    assert!(mermaid.contains("dashboard_cpu_main"));
    assert!(mermaid.contains("datasource_prom_main"));
    assert!(mermaid.contains("alert_grafana_alert_rule_cpu_high"));
    assert!(mermaid.contains("dashboard_cpu_main -->|backs| alert_grafana_alert_rule_cpu_high"));
    assert!(
        mermaid.contains("datasource_prom_main -->|alerts-on| alert_grafana_alert_rule_cpu_high")
    );
    assert!(mermaid.contains("datasource_prom_main -->|feeds| dashboard_cpu_main"));

    let dot = render_topology_dot(&document);
    assert!(dot.contains("digraph grafana_topology {"));
    assert!(dot.contains("\"dashboard:cpu-main\" [label=\"CPU Main\\ndashboard\"]"));
    assert!(dot.contains("\"datasource:prom-main\" [label=\"Prometheus Main\\ndatasource\"]"));
    assert!(dot.contains("\"alert:grafana-alert-rule:cpu-high\" [label=\"CPU High\\nalert-rule\"]"));
    assert!(dot.contains(
        "\"dashboard:cpu-main\" -> \"alert:grafana-alert-rule:cpu-high\" [label=\"backs\"]"
    ));
    assert!(dot.contains(
        "\"datasource:prom-main\" -> \"alert:grafana-alert-rule:cpu-high\" [label=\"alerts-on\"]"
    ));
    assert!(dot.contains("\"datasource:prom-main\" -> \"dashboard:cpu-main\" [label=\"feeds\"]"));
}

#[test]
fn build_dashboard_impact_document_reports_reachable_dashboards_and_alerts() {
    let governance = json!({
        "dashboardGovernance": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main"
            }
        ],
        "dashboardDatasourceEdges": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "datasourceUid": "prom-main",
                "datasource": "Prometheus Main"
            }
        ]
    });
    let alert_contract = json!({
        "kind": "grafana-utils-sync-alert-contract",
        "resources": [
            {
                "kind": "grafana-alert-rule",
                "identity": "cpu-high",
                "title": "CPU High",
                "references": ["prom-main", "cpu-main", "pagerduty-primary", "paging-policy", "slack.default"]
            },
            {
                "kind": "grafana-contact-point",
                "identity": "pagerduty-primary",
                "title": "PagerDuty Primary",
                "references": ["slack.default"]
            },
            {
                "kind": "grafana-notification-policies",
                "identity": "paging-policy",
                "title": "Paging Policy",
                "references": ["slack.default"]
            },
            {
                "kind": "grafana-notification-template",
                "identity": "slack.default",
                "title": "Slack Default",
                "references": []
            },
            {
                "kind": "grafana-alert-rule",
                "identity": "logs-high",
                "title": "Logs High",
                "references": ["logs-main"]
            }
        ]
    });

    let document = build_impact_document(&governance, Some(&alert_contract), "prom-main").unwrap();
    assert_eq!(document.summary.datasource_uid, "prom-main");
    assert_eq!(document.summary.dashboard_count, 1);
    assert_eq!(document.summary.alert_resource_count, 4);
    assert_eq!(document.summary.alert_rule_count, 1);
    assert_eq!(document.summary.contact_point_count, 1);
    assert_eq!(document.summary.mute_timing_count, 0);
    assert_eq!(document.summary.notification_policy_count, 1);
    assert_eq!(document.summary.template_count, 1);
    assert_eq!(document.dashboards[0].dashboard_uid, "cpu-main");
    assert_eq!(document.alert_resources.len(), 4);
    assert_eq!(document.alert_resources[0].identity, "cpu-high");
    assert_eq!(document.alert_resources[1].identity, "pagerduty-primary");
    assert_eq!(document.alert_resources[2].identity, "paging-policy");
    assert_eq!(document.alert_resources[3].identity, "slack.default");
    assert_eq!(document.affected_contact_points.len(), 1);
    assert_eq!(
        document.affected_contact_points[0].identity,
        "pagerduty-primary"
    );
    assert_eq!(document.affected_policies.len(), 1);
    assert_eq!(document.affected_policies[0].identity, "paging-policy");
    assert_eq!(document.affected_templates.len(), 1);
    assert_eq!(document.affected_templates[0].identity, "slack.default");

    let output = render_impact_text(&document);
    assert!(output.contains("Datasource impact"));
    assert!(output.contains(
        "Datasource impact: prom-main dashboards=1 alert-resources=4 alert-rules=1 contact-points=1 mute-timings=0 notification-policies=1 templates=1"
    ));
    assert!(output.contains("Alert resources:"));
    assert!(output.contains("alert-rule:cpu-high"));
    assert!(output.contains("Affected contact points:"));
    assert!(output.contains("Affected policies:"));
    assert!(output.contains("Affected templates:"));
}
