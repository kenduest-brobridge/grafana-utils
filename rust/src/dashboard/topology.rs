//! Topology and impact analysis for dashboards and alert contracts.
//! Direct live/local analysis is the common path; saved artifacts stay available for advanced reuse.
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

use crate::common::{
    emit_plain_output, load_json_object_file, render_json_value, should_print_stdout, Result,
};

use super::analysis_source::{resolve_dashboard_analysis_artifacts, DashboardAnalysisSourceArgs};
use super::{
    write_json_document, ImpactArgs, ImpactOutputFormat, TopologyArgs, TopologyOutputFormat,
};
#[path = "topology_build.rs"]
mod topology_build;
pub(crate) use topology_build::{build_impact_document, build_topology_document};
#[cfg(any(feature = "tui", test))]
#[path = "topology_browser.rs"]
mod topology_browser;
#[cfg(any(feature = "tui", test))]
pub(crate) use topology_browser::{build_impact_browser_items, build_topology_browser_items};

#[cfg(all(feature = "tui", not(test)))]
use super::impact_tui::run_impact_interactive;
#[cfg(all(feature = "tui", not(test)))]
use super::topology_tui::run_topology_interactive;
#[cfg(test)]
use crate::interactive_browser::run_interactive_browser;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct TopologySummary {
    #[serde(rename = "nodeCount")]
    pub(crate) node_count: usize,
    #[serde(rename = "edgeCount")]
    pub(crate) edge_count: usize,
    #[serde(rename = "datasourceCount")]
    pub(crate) datasource_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "variableCount")]
    pub(crate) variable_count: usize,
    #[serde(rename = "alertResourceCount")]
    pub(crate) alert_resource_count: usize,
    #[serde(rename = "alertRuleCount")]
    pub(crate) alert_rule_count: usize,
    #[serde(rename = "contactPointCount")]
    pub(crate) contact_point_count: usize,
    #[serde(rename = "muteTimingCount")]
    pub(crate) mute_timing_count: usize,
    #[serde(rename = "notificationPolicyCount")]
    pub(crate) notification_policy_count: usize,
    #[serde(rename = "templateCount")]
    pub(crate) template_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct TopologyNode {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct TopologyEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) relation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct TopologyDocument {
    pub(crate) summary: TopologySummary,
    pub(crate) nodes: Vec<TopologyNode>,
    pub(crate) edges: Vec<TopologyEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ImpactSummary {
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "alertResourceCount")]
    pub(crate) alert_resource_count: usize,
    #[serde(rename = "alertRuleCount")]
    pub(crate) alert_rule_count: usize,
    #[serde(rename = "contactPointCount")]
    pub(crate) contact_point_count: usize,
    #[serde(rename = "muteTimingCount")]
    pub(crate) mute_timing_count: usize,
    #[serde(rename = "notificationPolicyCount")]
    pub(crate) notification_policy_count: usize,
    #[serde(rename = "templateCount")]
    pub(crate) template_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ImpactDashboard {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ImpactAlertResource {
    pub(crate) kind: String,
    pub(crate) identity: String,
    pub(crate) title: String,
    #[serde(rename = "sourcePath")]
    pub(crate) source_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ImpactDocument {
    pub(crate) summary: ImpactSummary,
    pub(crate) dashboards: Vec<ImpactDashboard>,
    #[serde(rename = "alertResources")]
    pub(crate) alert_resources: Vec<ImpactAlertResource>,
    #[serde(rename = "affectedContactPoints")]
    pub(crate) affected_contact_points: Vec<ImpactAlertResource>,
    #[serde(rename = "affectedPolicies")]
    pub(crate) affected_policies: Vec<ImpactAlertResource>,
    #[serde(rename = "affectedTemplates")]
    pub(crate) affected_templates: Vec<ImpactAlertResource>,
}

#[derive(Clone, Debug)]
struct ParsedAlertResource {
    normalized_kind: String,
    identity: String,
    title: String,
    source_path: String,
    references: Vec<String>,
    node_id: String,
}

fn slug_for_mermaid(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            slug.push(character);
        } else {
            slug.push('_');
        }
    }
    if slug
        .chars()
        .next()
        .map(|character| character.is_ascii_digit())
        .unwrap_or(true)
    {
        slug.insert(0, 'n');
    }
    slug
}

fn escape_label(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn load_object(path: &Path) -> Result<Value> {
    load_json_object_file(path, "Topology/impact JSON")
}

pub(crate) fn render_topology_text(document: &TopologyDocument) -> String {
    let mut lines = vec![format!(
        "Dashboard topology: nodes={} edges={} datasources={} dashboards={} panels={} variables={} alert-resources={} alert-rules={} contact-points={} mute-timings={} notification-policies={} templates={}",
        document.summary.node_count,
        document.summary.edge_count,
        document.summary.datasource_count,
        document.summary.dashboard_count,
        document.summary.panel_count,
        document.summary.variable_count,
        document.summary.alert_resource_count,
        document.summary.alert_rule_count,
        document.summary.contact_point_count,
        document.summary.mute_timing_count,
        document.summary.notification_policy_count,
        document.summary.template_count
    )];
    for edge in &document.edges {
        lines.push(format!(
            "  {} --{}--> {}",
            edge.from, edge.relation, edge.to
        ));
    }
    lines.join("\n")
}

pub(crate) fn render_topology_mermaid(document: &TopologyDocument) -> String {
    let mut lines = vec!["graph TD".to_string()];
    for node in &document.nodes {
        lines.push(format!(
            "  {}[\"{}\"]",
            slug_for_mermaid(&node.id),
            escape_label(&node.label)
        ));
    }
    for edge in &document.edges {
        lines.push(format!(
            "  {} -->|{}| {}",
            slug_for_mermaid(&edge.from),
            edge.relation,
            slug_for_mermaid(&edge.to)
        ));
    }
    lines.join("\n")
}

pub(crate) fn render_topology_dot(document: &TopologyDocument) -> String {
    let mut lines = vec!["digraph grafana_topology {".to_string()];
    for node in &document.nodes {
        lines.push(format!(
            "  \"{}\" [label=\"{}\\n{}\"] ;",
            node.id,
            escape_label(&node.label),
            node.kind
        ));
    }
    for edge in &document.edges {
        lines.push(format!(
            "  \"{}\" -> \"{}\" [label=\"{}\"] ;",
            escape_label(&edge.from),
            escape_label(&edge.to),
            escape_label(&edge.relation)
        ));
    }
    lines.push("}".to_string());
    lines.join("\n")
}

pub(crate) fn render_impact_text(document: &ImpactDocument) -> String {
    let mut lines = vec![format!(
        "Datasource impact: {} dashboards={} alert-resources={} alert-rules={} contact-points={} mute-timings={} notification-policies={} templates={}",
        document.summary.datasource_uid,
        document.summary.dashboard_count,
        document.summary.alert_resource_count,
        document.summary.alert_rule_count,
        document.summary.contact_point_count,
        document.summary.mute_timing_count,
        document.summary.notification_policy_count,
        document.summary.template_count
    )];
    if !document.dashboards.is_empty() {
        lines.push("Dashboards:".to_string());
        for dashboard in &document.dashboards {
            lines.push(format!(
                "  {} ({}) panels={} queries={}",
                dashboard.dashboard_uid,
                dashboard.folder_path,
                dashboard.panel_count,
                dashboard.query_count
            ));
        }
    }
    if !document.alert_resources.is_empty() {
        lines.push("Alert resources:".to_string());
        for resource in &document.alert_resources {
            lines.push(format!(
                "  {}:{} {}",
                resource.kind, resource.identity, resource.title
            ));
        }
    }
    if !document.affected_contact_points.is_empty() {
        lines.push("Affected contact points:".to_string());
        for resource in &document.affected_contact_points {
            lines.push(format!(
                "  {}:{} {}",
                resource.kind, resource.identity, resource.title
            ));
        }
    }
    if !document.affected_policies.is_empty() {
        lines.push("Affected policies:".to_string());
        for resource in &document.affected_policies {
            lines.push(format!(
                "  {}:{} {}",
                resource.kind, resource.identity, resource.title
            ));
        }
    }
    if !document.affected_templates.is_empty() {
        lines.push("Affected templates:".to_string());
        for resource in &document.affected_templates {
            lines.push(format!(
                "  {}:{} {}",
                resource.kind, resource.identity, resource.title
            ));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
fn build_impact_summary_lines(document: &ImpactDocument) -> Vec<String> {
    vec![
        format!(
            "Datasource {} impact: dashboards={}; alert assets={} (alert rules={}, contact points={}, policies={}, templates={}, mute timings={}).",
            document.summary.datasource_uid,
            document.summary.dashboard_count,
            document.summary.alert_resource_count,
            document.summary.alert_rule_count,
            document.summary.contact_point_count,
            document.summary.notification_policy_count,
            document.summary.template_count,
            document.summary.mute_timing_count
        ),
        "Blast radius first: inspect dashboards, then follow alert rules into routing assets."
            .to_string(),
    ]
}

#[cfg(test)]
fn build_impact_interactive_summary(document: &ImpactDocument) -> Vec<String> {
    build_impact_summary_lines(document)
}

pub(crate) fn run_dashboard_topology(args: &TopologyArgs) -> Result<()> {
    let artifacts = resolve_dashboard_analysis_artifacts(&DashboardAnalysisSourceArgs {
        common: &args.common,
        page_size: args.page_size,
        org_id: args.org_id,
        all_orgs: args.all_orgs,
        import_dir: args.import_dir.as_deref(),
        input_format: args.input_format,
        input_type: args.input_type,
        governance: args.governance.as_deref(),
        queries: args.queries.as_deref(),
        require_queries: false,
    })?;
    let alert_contract = match args.alert_contract.as_ref() {
        Some(path) => Some(load_object(path)?),
        None => None,
    };
    let document = build_topology_document(&artifacts.governance, alert_contract.as_ref())?;
    if args.interactive {
        #[cfg(all(feature = "tui", not(test)))]
        {
            return run_topology_interactive(&document);
        }
        #[cfg(test)]
        {
            let summary = vec![
            format!(
                "Topology: {} nodes, {} edges, {} dashboards, {} datasources, {} panels, {} variables, {} alert resources.",
                document.summary.node_count,
                document.summary.edge_count,
                document.summary.dashboard_count,
                document.summary.datasource_count,
                document.summary.panel_count,
                document.summary.variable_count,
                document.summary.alert_resource_count,
            ),
            "Browse nodes by kind on the left; each detail pane shows inbound and outbound edge summaries.".to_string(),
        ];
            return run_interactive_browser(
                "Dashboard Topology",
                &summary,
                &build_topology_browser_items(&document),
            );
        }
        #[cfg(not(feature = "tui"))]
        {
            return super::tui_not_built("topology --interactive");
        }
    }
    let rendered = match args.output_format {
        TopologyOutputFormat::Text => render_topology_text(&document),
        TopologyOutputFormat::Json => render_json_value(&document)?,
        TopologyOutputFormat::Mermaid => render_topology_mermaid(&document),
        TopologyOutputFormat::Dot => render_topology_dot(&document),
    };
    if let Some(output_file) = args.output_file.as_ref() {
        if matches!(args.output_format, TopologyOutputFormat::Json) {
            write_json_document(&document, output_file)?;
        }
    }
    if matches!(args.output_format, TopologyOutputFormat::Json) {
        if should_print_stdout(args.output_file.as_deref(), args.also_stdout) {
            print!("{rendered}");
        }
    } else {
        emit_plain_output(&rendered, args.output_file.as_deref(), args.also_stdout)?;
    }
    Ok(())
}

pub(crate) fn run_dashboard_impact(args: &ImpactArgs) -> Result<()> {
    let artifacts = resolve_dashboard_analysis_artifacts(&DashboardAnalysisSourceArgs {
        common: &args.common,
        page_size: args.page_size,
        org_id: args.org_id,
        all_orgs: args.all_orgs,
        import_dir: args.import_dir.as_deref(),
        input_format: args.input_format,
        input_type: args.input_type,
        governance: args.governance.as_deref(),
        queries: args.queries.as_deref(),
        require_queries: false,
    })?;
    let alert_contract = match args.alert_contract.as_ref() {
        Some(path) => Some(load_object(path)?),
        None => None,
    };
    let document = build_impact_document(
        &artifacts.governance,
        alert_contract.as_ref(),
        &args.datasource_uid,
    )?;
    if args.interactive {
        #[cfg(all(feature = "tui", not(test)))]
        {
            return run_impact_interactive(&document);
        }
        #[cfg(test)]
        {
            return run_interactive_browser(
                "Dashboard Impact",
                &build_impact_interactive_summary(&document),
                &build_impact_browser_items(&document),
            );
        }
        #[cfg(not(feature = "tui"))]
        {
            return super::tui_not_built("impact --interactive");
        }
    }
    match args.output_format {
        ImpactOutputFormat::Text => println!("{}", render_impact_text(&document)),
        ImpactOutputFormat::Json => print!("{}", render_json_value(&document)?),
    }
    Ok(())
}

#[cfg(test)]
mod output_contract_tests {
    use super::*;
    use crate::common::CliColorChoice;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    fn make_common_args() -> crate::dashboard::CommonCliArgs {
        crate::dashboard::CommonCliArgs {
            color: CliColorChoice::Never,
            profile: None,
            url: "http://127.0.0.1:3000".to_string(),
            api_token: None,
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        }
    }

    #[test]
    fn run_dashboard_topology_writes_plain_text_output_file_and_keeps_also_stdout_enabled() {
        let temp = tempdir().unwrap();
        let governance = temp.path().join("governance.json");
        let output_file = temp.path().join("topology.txt");
        fs::write(
            &governance,
            serde_json::to_string_pretty(&json!({
                "dashboardGovernance": [
                    {
                        "dashboardUid": "cpu-main",
                        "dashboardTitle": "CPU Main",
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
                        "panelCount": 1,
                        "queryCount": 1
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        run_dashboard_topology(&TopologyArgs {
            common: make_common_args(),
            page_size: 500,
            org_id: None,
            all_orgs: false,
            import_dir: None,
            input_format: crate::dashboard::DashboardImportInputFormat::Raw,
            input_type: None,
            governance: Some(governance),
            queries: None,
            alert_contract: None,
            output_format: TopologyOutputFormat::Text,
            output_file: Some(output_file.clone()),
            also_stdout: true,
            interactive: false,
        })
        .unwrap();

        let raw = fs::read_to_string(output_file).unwrap();
        assert_eq!(
            raw,
            "Dashboard topology: nodes=2 edges=1 datasources=1 dashboards=1 panels=0 variables=0 alert-resources=0 alert-rules=0 contact-points=0 mute-timings=0 notification-policies=0 templates=0\n  datasource:prom-main --feeds--> dashboard:cpu-main\n"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_topology_document() -> TopologyDocument {
        TopologyDocument {
            summary: TopologySummary {
                node_count: 4,
                edge_count: 3,
                datasource_count: 1,
                dashboard_count: 1,
                panel_count: 1,
                variable_count: 1,
                alert_resource_count: 0,
                alert_rule_count: 0,
                contact_point_count: 0,
                mute_timing_count: 0,
                notification_policy_count: 0,
                template_count: 0,
            },
            nodes: vec![
                TopologyNode {
                    id: "panel:p1".to_string(),
                    kind: "panel".to_string(),
                    label: "Panel 1".to_string(),
                },
                TopologyNode {
                    id: "dashboard:db".to_string(),
                    kind: "dashboard".to_string(),
                    label: "Overview".to_string(),
                },
                TopologyNode {
                    id: "variable:db:env".to_string(),
                    kind: "variable".to_string(),
                    label: "Env".to_string(),
                },
                TopologyNode {
                    id: "datasource:ds".to_string(),
                    kind: "datasource".to_string(),
                    label: "Primary".to_string(),
                },
            ],
            edges: vec![
                TopologyEdge {
                    from: "panel:p1".to_string(),
                    to: "dashboard:db".to_string(),
                    relation: "belongs-to".to_string(),
                },
                TopologyEdge {
                    from: "datasource:ds".to_string(),
                    to: "dashboard:db".to_string(),
                    relation: "feeds".to_string(),
                },
                TopologyEdge {
                    from: "dashboard:db".to_string(),
                    to: "variable:db:env".to_string(),
                    relation: "uses".to_string(),
                },
            ],
        }
    }

    fn sample_impact_document() -> ImpactDocument {
        ImpactDocument {
            summary: ImpactSummary {
                datasource_uid: "ds-1".to_string(),
                dashboard_count: 1,
                alert_resource_count: 5,
                alert_rule_count: 1,
                contact_point_count: 1,
                mute_timing_count: 1,
                notification_policy_count: 1,
                template_count: 1,
            },
            dashboards: vec![
                ImpactDashboard {
                    dashboard_uid: "db-z".to_string(),
                    dashboard_title: "Zulu".to_string(),
                    folder_path: "team/z".to_string(),
                    panel_count: 3,
                    query_count: 5,
                },
                ImpactDashboard {
                    dashboard_uid: "db-a".to_string(),
                    dashboard_title: "Alpha".to_string(),
                    folder_path: "team/a".to_string(),
                    panel_count: 1,
                    query_count: 2,
                },
            ],
            alert_resources: vec![
                ImpactAlertResource {
                    kind: "alert-rule".to_string(),
                    identity: "rule-1".to_string(),
                    title: "Rule A".to_string(),
                    source_path: "/rules/a".to_string(),
                },
                ImpactAlertResource {
                    kind: "mute-timing".to_string(),
                    identity: "mute-1".to_string(),
                    title: "Mute A".to_string(),
                    source_path: "/mutes/a".to_string(),
                },
            ],
            affected_contact_points: vec![ImpactAlertResource {
                kind: "contact-point".to_string(),
                identity: "cp-1".to_string(),
                title: "PagerDuty".to_string(),
                source_path: "/contact/pagerduty".to_string(),
            }],
            affected_policies: vec![ImpactAlertResource {
                kind: "policy".to_string(),
                identity: "policy-1".to_string(),
                title: "Default policy".to_string(),
                source_path: "/policies/default".to_string(),
            }],
            affected_templates: vec![ImpactAlertResource {
                kind: "template".to_string(),
                identity: "tmpl-1".to_string(),
                title: "Alert template".to_string(),
                source_path: "/templates/alert".to_string(),
            }],
        }
    }

    #[test]
    fn build_impact_interactive_summary_is_concise_and_decision_oriented() {
        let document = sample_impact_document();
        assert_eq!(
            build_impact_interactive_summary(&document),
            vec![
                "Datasource ds-1 impact: dashboards=1; alert assets=5 (alert rules=1, contact points=1, policies=1, templates=1, mute timings=1).".to_string(),
                "Blast radius first: inspect dashboards, then follow alert rules into routing assets."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn build_impact_browser_items_groups_operator_surface() {
        let document = sample_impact_document();
        let items = build_impact_browser_items(&document);
        let kinds = items
            .iter()
            .map(|item| item.kind.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                "dashboard",
                "dashboard",
                "alert-rule",
                "mute-timing",
                "contact-point",
                "policy",
                "template",
            ]
        );
        assert!(items[0].meta.starts_with("folder=team/a | uid=db-a"));
        assert_eq!(items[0].details[2], "Scope: folder-scoped");
        assert!(items[2].meta.contains("group=Alert rules"));
        assert_eq!(items[2].details[1], "Group: Alert rules");
    }

    #[test]
    fn build_topology_browser_items_sorts_by_kind_then_label_and_summarizes_edges() {
        let document = sample_topology_document();
        let items = build_topology_browser_items(&document);
        let titles = items
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>();
        assert_eq!(titles, vec!["Overview", "Primary", "Panel 1", "Env"]);
        assert!(items[0].meta.contains("id=dashboard:db | in=2 out=1"));
        assert!(items[0]
            .details
            .iter()
            .any(|line| line == "Inbound edge summary:"));
        assert!(items[0]
            .details
            .iter()
            .any(|line| line == "  belongs-to <- Panel 1 [panel]"));
        assert!(items[0]
            .details
            .iter()
            .any(|line| line == "  feeds <- Primary [datasource]"));
        assert!(items[0]
            .details
            .iter()
            .any(|line| line == "  uses -> Env [variable]"));
    }
}
