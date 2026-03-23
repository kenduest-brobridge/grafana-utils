//! Artifact-driven topology and impact analysis for dashboards and alert contracts.
use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::common::{message, Result};
use crate::interactive_browser::BrowserItem;

use super::{
    write_json_document, ImpactArgs, ImpactOutputFormat, TopologyArgs, TopologyOutputFormat,
};

#[cfg(not(test))]
use super::impact_tui::run_impact_interactive;
#[cfg(not(test))]
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

fn normalize_alert_kind(kind: &str) -> &str {
    match kind {
        "grafana-alert-rule" => "alert-rule",
        "grafana-contact-point" => "contact-point",
        "grafana-mute-timing" => "mute-timing",
        "grafana-notification-policies" | "grafana-notification-policy" => "notification-policy",
        "grafana-notification-template" => "template",
        _ => "alert-resource",
    }
}

fn alert_resource_label(title: &str, identity: &str) -> String {
    if title.is_empty() {
        identity.to_string()
    } else {
        title.to_string()
    }
}

fn collect_alert_resources(alert_contract: &Value) -> Result<Vec<ParsedAlertResource>> {
    let resources = alert_contract
        .get("resources")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Alert contract JSON must contain a resources array."))?;
    let mut parsed_resources = Vec::new();
    for resource in resources {
        let kind = string_field(resource, "kind");
        let identity = string_field(resource, "identity");
        let title = string_field(resource, "title");
        if kind.is_empty() || identity.is_empty() {
            continue;
        }
        let references = resource
            .get("references")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        parsed_resources.push(ParsedAlertResource {
            node_id: format!("alert:{kind}:{identity}"),
            normalized_kind: normalize_alert_kind(&kind).to_string(),
            identity,
            title,
            source_path: string_field(resource, "sourcePath"),
            references,
        });
    }
    Ok(parsed_resources)
}

fn edge_relation_for_alert_reference(source_kind: &str, target_kind: &str) -> Option<&'static str> {
    match (source_kind, target_kind) {
        ("alert-rule", "contact-point") => Some("routes-to"),
        ("alert-rule", "notification-policy") => Some("routes-to"),
        ("alert-rule", "template") => Some("uses-template"),
        ("contact-point", "template") => Some("uses-template"),
        ("notification-policy", "template") => Some("uses-template"),
        _ => None,
    }
}

fn load_object(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&raw)?;
    if !value.is_object() {
        return Err(message(format!(
            "JSON document at {} must be an object.",
            path.display()
        )));
    }
    Ok(value)
}

fn string_field(record: &Value, key: &str) -> String {
    record
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

fn string_list_field(record: &Value, key: &str) -> Vec<String> {
    record
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn sort_impact_resources(resources: &mut Vec<&ImpactAlertResource>) {
    resources.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then_with(|| left.identity.cmp(&right.identity))
            .then_with(|| left.source_path.cmp(&right.source_path))
    });
}

fn push_unique_node(
    nodes: &mut BTreeMap<String, TopologyNode>,
    id: String,
    kind: &str,
    label: String,
) {
    nodes.entry(id.clone()).or_insert(TopologyNode {
        id,
        kind: kind.to_string(),
        label,
    });
}

fn push_unique_edge(
    edges: &mut BTreeSet<(String, String, String)>,
    from: String,
    to: String,
    relation: &str,
) {
    edges.insert((from, to, relation.to_string()));
}

fn slug_for_mermaid(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn escape_label(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn panel_node_id(dashboard_uid: &str, panel_id: &str) -> String {
    format!("panel:{dashboard_uid}:{panel_id}")
}

fn variable_node_id(dashboard_uid: &str, variable: &str) -> String {
    format!("variable:{dashboard_uid}:{variable}")
}

fn compare_topology_nodes(left: &TopologyNode, right: &TopologyNode) -> Ordering {
    left.kind
        .cmp(&right.kind)
        .then_with(|| left.label.cmp(&right.label))
        .then_with(|| left.id.cmp(&right.id))
}

fn topology_node_display_label(node: &TopologyNode) -> String {
    format!(
        "{} [{}]",
        if node.label.is_empty() {
            node.id.as_str()
        } else {
            node.label.as_str()
        },
        node.kind
    )
}

pub(crate) fn build_topology_document(
    governance_document: &Value,
    alert_contract_document: Option<&Value>,
) -> Result<TopologyDocument> {
    let dashboard_edges = governance_document
        .get("dashboardDatasourceEdges")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            message("Dashboard governance JSON must contain a dashboardDatasourceEdges array.")
        })?;
    let dashboards = governance_document
        .get("dashboardGovernance")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            message("Dashboard governance JSON must contain a dashboardGovernance array.")
        })?;

    let mut nodes = BTreeMap::<String, TopologyNode>::new();
    let mut edges = BTreeSet::<(String, String, String)>::new();
    let mut alert_identity_to_node = BTreeMap::<String, String>::new();
    let mut alert_identity_to_kind = BTreeMap::<String, String>::new();
    let mut datasource_names_to_uid = BTreeMap::<String, String>::new();

    for dashboard in dashboards {
        let dashboard_uid = string_field(dashboard, "dashboardUid");
        if dashboard_uid.is_empty() {
            continue;
        }
        let dashboard_title = string_field(dashboard, "dashboardTitle");
        push_unique_node(
            &mut nodes,
            format!("dashboard:{dashboard_uid}"),
            "dashboard",
            if dashboard_title.is_empty() {
                dashboard_uid.clone()
            } else {
                dashboard_title
            },
        );
    }

    for edge in dashboard_edges {
        let datasource_uid = string_field(edge, "datasourceUid");
        let datasource_name = string_field(edge, "datasource");
        let dashboard_uid = string_field(edge, "dashboardUid");
        if datasource_uid.is_empty() || dashboard_uid.is_empty() {
            continue;
        }
        datasource_names_to_uid.insert(datasource_name.clone(), datasource_uid.clone());
        push_unique_node(
            &mut nodes,
            format!("datasource:{datasource_uid}"),
            "datasource",
            if datasource_name.is_empty() {
                datasource_uid.clone()
            } else {
                datasource_name
            },
        );
        push_unique_edge(
            &mut edges,
            format!("datasource:{datasource_uid}"),
            format!("dashboard:{dashboard_uid}"),
            "feeds",
        );
        for variable in string_list_field(edge, "queryVariables") {
            let variable_id = variable_node_id(&dashboard_uid, &variable);
            push_unique_node(
                &mut nodes,
                variable_id.clone(),
                "variable",
                variable.clone(),
            );
            push_unique_edge(
                &mut edges,
                format!("datasource:{datasource_uid}"),
                variable_id,
                "feeds-variable",
            );
        }
    }

    let empty: &[Value] = &[];
    let dashboard_dependencies = governance_document
        .get("dashboardDependencies")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(empty);
    for dependency in dashboard_dependencies {
        let dashboard_uid = string_field(dependency, "dashboardUid");
        if dashboard_uid.is_empty() {
            continue;
        }
        let panel_ids = string_list_field(dependency, "panelIds");
        if panel_ids.is_empty() {
            continue;
        }
        let mut variable_names = BTreeSet::<String>::new();
        for variable in string_list_field(dependency, "panelVariables") {
            variable_names.insert(variable);
        }
        for variable in string_list_field(dependency, "queryVariables") {
            variable_names.insert(variable);
        }
        let dashboard_node_id = format!("dashboard:{dashboard_uid}");
        for panel_id in panel_ids {
            let panel_id = panel_id.trim();
            if panel_id.is_empty() {
                continue;
            }
            let panel_id_string = panel_id.to_string();
            let panel_node = panel_node_id(&dashboard_uid, &panel_id_string);
            push_unique_node(
                &mut nodes,
                panel_node.clone(),
                "panel",
                format!("Panel {panel_id_string}"),
            );
            push_unique_edge(
                &mut edges,
                panel_node.clone(),
                dashboard_node_id.clone(),
                "belongs-to",
            );
            for variable in &variable_names {
                let variable_id = variable_node_id(&dashboard_uid, variable);
                push_unique_node(
                    &mut nodes,
                    variable_id.clone(),
                    "variable",
                    variable.clone(),
                );
                push_unique_edge(&mut edges, variable_id, panel_node.clone(), "used-by");
            }
        }
    }

    let mut alert_resource_count = 0usize;
    if let Some(alert_contract) = alert_contract_document {
        let parsed_alert_resources = collect_alert_resources(alert_contract)?;
        for resource in &parsed_alert_resources {
            alert_resource_count += 1;
            alert_identity_to_node.insert(resource.identity.clone(), resource.node_id.clone());
            alert_identity_to_kind
                .insert(resource.identity.clone(), resource.normalized_kind.clone());
            push_unique_node(
                &mut nodes,
                resource.node_id.clone(),
                &resource.normalized_kind,
                alert_resource_label(&resource.title, &resource.identity),
            );
        }
        for resource in &parsed_alert_resources {
            for reference in &resource.references {
                if let Some(target_node) = alert_identity_to_node.get(reference) {
                    if let Some(target_kind) = alert_identity_to_kind.get(reference) {
                        if let Some(relation) = edge_relation_for_alert_reference(
                            &resource.normalized_kind,
                            target_kind,
                        ) {
                            push_unique_edge(
                                &mut edges,
                                resource.node_id.clone(),
                                target_node.clone(),
                                relation,
                            );
                        }
                    }
                }
                let datasource_uid = datasource_names_to_uid
                    .get(reference)
                    .cloned()
                    .unwrap_or_else(|| reference.clone());
                if nodes.contains_key(&format!("datasource:{datasource_uid}"))
                    && resource.normalized_kind == "alert-rule"
                {
                    push_unique_edge(
                        &mut edges,
                        format!("datasource:{datasource_uid}"),
                        resource.node_id.clone(),
                        "alerts-on",
                    );
                }
                if nodes.contains_key(&format!("dashboard:{reference}"))
                    && resource.normalized_kind == "alert-rule"
                {
                    push_unique_edge(
                        &mut edges,
                        format!("dashboard:{reference}"),
                        resource.node_id.clone(),
                        "backs",
                    );
                }
            }
        }
    }

    let nodes = nodes.into_values().collect::<Vec<_>>();
    let edges = edges
        .into_iter()
        .map(|(from, to, relation)| TopologyEdge { from, to, relation })
        .collect::<Vec<_>>();
    let datasource_count = nodes
        .iter()
        .filter(|node| node.kind == "datasource")
        .count();
    let dashboard_count = nodes.iter().filter(|node| node.kind == "dashboard").count();
    let panel_count = nodes.iter().filter(|node| node.kind == "panel").count();
    let variable_count = nodes.iter().filter(|node| node.kind == "variable").count();
    let alert_rule_count = nodes
        .iter()
        .filter(|node| node.kind == "alert-rule")
        .count();
    let contact_point_count = nodes
        .iter()
        .filter(|node| node.kind == "contact-point")
        .count();
    let mute_timing_count = nodes
        .iter()
        .filter(|node| node.kind == "mute-timing")
        .count();
    let notification_policy_count = nodes
        .iter()
        .filter(|node| node.kind == "notification-policy")
        .count();
    let template_count = nodes.iter().filter(|node| node.kind == "template").count();

    Ok(TopologyDocument {
        summary: TopologySummary {
            node_count: nodes.len(),
            edge_count: edges.len(),
            datasource_count,
            dashboard_count,
            panel_count,
            variable_count,
            alert_resource_count,
            alert_rule_count,
            contact_point_count,
            mute_timing_count,
            notification_policy_count,
            template_count,
        },
        nodes,
        edges,
    })
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

pub(crate) fn build_impact_document(
    governance_document: &Value,
    alert_contract_document: Option<&Value>,
    datasource_uid: &str,
) -> Result<ImpactDocument> {
    let dashboards = governance_document
        .get("dashboardGovernance")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            message("Dashboard governance JSON must contain a dashboardGovernance array.")
        })?;
    let mut dashboard_lookup = BTreeMap::<String, ImpactDashboard>::new();
    for dashboard in dashboards {
        let dashboard_uid = string_field(dashboard, "dashboardUid");
        if dashboard_uid.is_empty() {
            continue;
        }
        dashboard_lookup.insert(
            dashboard_uid.clone(),
            ImpactDashboard {
                dashboard_uid,
                dashboard_title: string_field(dashboard, "dashboardTitle"),
                folder_path: string_field(dashboard, "folderPath"),
                panel_count: dashboard
                    .get("panelCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
                query_count: dashboard
                    .get("queryCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
            },
        );
    }

    let topology = build_topology_document(governance_document, alert_contract_document)?;
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    for edge in &topology.edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    let mut reachable = BTreeSet::<String>::new();
    let mut stack = vec![format!("datasource:{datasource_uid}")];
    while let Some(node_id) = stack.pop() {
        if !reachable.insert(node_id.clone()) {
            continue;
        }
        if let Some(targets) = adjacency.get(&node_id) {
            stack.extend(targets.iter().cloned());
        }
    }

    let mut affected_dashboards = BTreeMap::<String, ImpactDashboard>::new();
    for node in &topology.nodes {
        if node.kind != "dashboard" || !reachable.contains(&node.id) {
            continue;
        }
        let dashboard_uid = node.id.strip_prefix("dashboard:").unwrap_or(&node.id);
        if let Some(dashboard) = dashboard_lookup.get(dashboard_uid) {
            affected_dashboards.insert(dashboard_uid.to_string(), dashboard.clone());
        }
    }

    let mut alert_resources = BTreeMap::<String, ImpactAlertResource>::new();
    if let Some(alert_contract) = alert_contract_document {
        for resource in collect_alert_resources(alert_contract)? {
            if !reachable.contains(&resource.node_id) {
                continue;
            }
            alert_resources.insert(
                resource.node_id.clone(),
                ImpactAlertResource {
                    kind: resource.normalized_kind,
                    identity: resource.identity,
                    title: resource.title,
                    source_path: resource.source_path,
                },
            );
        }
    }

    let mut affected_contact_points = Vec::new();
    let mut affected_policies = Vec::new();
    let mut affected_templates = Vec::new();
    let mut alert_rule_count = 0usize;
    let mut contact_point_count = 0usize;
    let mut mute_timing_count = 0usize;
    let mut notification_policy_count = 0usize;
    let mut template_count = 0usize;
    for resource in alert_resources.values() {
        match resource.kind.as_str() {
            "alert-rule" => alert_rule_count += 1,
            "contact-point" => {
                contact_point_count += 1;
                affected_contact_points.push(resource.clone());
            }
            "mute-timing" => mute_timing_count += 1,
            "notification-policy" => {
                notification_policy_count += 1;
                affected_policies.push(resource.clone());
            }
            "template" => {
                template_count += 1;
                affected_templates.push(resource.clone());
            }
            _ => {}
        }
    }

    Ok(ImpactDocument {
        summary: ImpactSummary {
            datasource_uid: datasource_uid.to_string(),
            dashboard_count: affected_dashboards.len(),
            alert_resource_count: alert_resources.len(),
            alert_rule_count,
            contact_point_count,
            mute_timing_count,
            notification_policy_count,
            template_count,
        },
        dashboards: affected_dashboards.into_values().collect(),
        alert_resources: alert_resources.into_values().collect(),
        affected_contact_points,
        affected_policies,
        affected_templates,
    })
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

pub(crate) fn build_impact_browser_items(document: &ImpactDocument) -> Vec<BrowserItem> {
    let mut items = Vec::new();

    let mut dashboards = document.dashboards.iter().collect::<Vec<_>>();
    dashboards.sort_by(|left, right| {
        left.folder_path
            .cmp(&right.folder_path)
            .then_with(|| left.dashboard_title.cmp(&right.dashboard_title))
            .then_with(|| left.dashboard_uid.cmp(&right.dashboard_uid))
    });
    items.extend(dashboards.into_iter().map(build_impact_dashboard_item));

    let mut alert_rules = document
        .alert_resources
        .iter()
        .filter(|resource| resource.kind == "alert-rule")
        .collect::<Vec<_>>();
    sort_impact_resources(&mut alert_rules);
    items.extend(alert_rules.into_iter().map(|resource| {
        build_impact_resource_item(resource, &document.summary.datasource_uid, "Alert rules")
    }));

    let mut mute_timings = document
        .alert_resources
        .iter()
        .filter(|resource| resource.kind == "mute-timing")
        .collect::<Vec<_>>();
    sort_impact_resources(&mut mute_timings);
    items.extend(mute_timings.into_iter().map(|resource| {
        build_impact_resource_item(resource, &document.summary.datasource_uid, "Mute timings")
    }));

    let mut alert_resources = document
        .alert_resources
        .iter()
        .filter(|resource| {
            !matches!(
                resource.kind.as_str(),
                "alert-rule" | "mute-timing" | "contact-point" | "notification-policy" | "template"
            )
        })
        .collect::<Vec<_>>();
    sort_impact_resources(&mut alert_resources);
    items.extend(alert_resources.into_iter().map(|resource| {
        build_impact_resource_item(
            resource,
            &document.summary.datasource_uid,
            "Alert resources",
        )
    }));

    let mut contact_points = document.affected_contact_points.iter().collect::<Vec<_>>();
    sort_impact_resources(&mut contact_points);
    items.extend(contact_points.into_iter().map(|resource| {
        build_impact_resource_item(resource, &document.summary.datasource_uid, "Contact points")
    }));

    let mut policies = document.affected_policies.iter().collect::<Vec<_>>();
    sort_impact_resources(&mut policies);
    items.extend(policies.into_iter().map(|resource| {
        build_impact_resource_item(resource, &document.summary.datasource_uid, "Policies")
    }));

    let mut templates = document.affected_templates.iter().collect::<Vec<_>>();
    sort_impact_resources(&mut templates);
    items.extend(templates.into_iter().map(|resource| {
        build_impact_resource_item(resource, &document.summary.datasource_uid, "Templates")
    }));

    items
}

fn impact_item_title(title: &str, fallback: &str) -> String {
    if title.is_empty() {
        fallback.to_string()
    } else {
        title.to_string()
    }
}

fn impact_display_value(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn build_impact_dashboard_item(dashboard: &ImpactDashboard) -> BrowserItem {
    let title = impact_item_title(&dashboard.dashboard_title, &dashboard.dashboard_uid);
    let classification = if dashboard.folder_path.is_empty() {
        "unfiled"
    } else {
        "folder-scoped"
    };
    BrowserItem {
        kind: "dashboard".to_string(),
        title,
        meta: format!(
            "folder={} | uid={} | p={} q={}",
            impact_display_value(&dashboard.folder_path),
            dashboard.dashboard_uid,
            dashboard.panel_count,
            dashboard.query_count
        ),
        details: vec![
            format!("UID: {}", dashboard.dashboard_uid),
            format!(
                "Folder path: {}",
                impact_display_value(&dashboard.folder_path)
            ),
            format!("Scope: {}", classification),
            format!("Panels: {}", dashboard.panel_count),
            format!("Queries: {}", dashboard.query_count),
        ],
    }
}

fn build_impact_resource_item(
    resource: &ImpactAlertResource,
    datasource_uid: &str,
    section_label: &str,
) -> BrowserItem {
    let title = impact_item_title(&resource.title, &resource.identity);
    BrowserItem {
        kind: resource.kind.clone(),
        title,
        meta: format!("group={} | id={}", section_label, resource.identity),
        details: vec![
            format!("Kind: {}", resource.kind),
            format!("Group: {}", section_label),
            format!("Identity: {}", resource.identity),
            format!(
                "Title: {}",
                impact_item_title(&resource.title, &resource.identity)
            ),
            format!(
                "Source path: {}",
                impact_display_value(&resource.source_path)
            ),
            format!("Datasource UID: {}", datasource_uid),
        ],
    }
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

pub(crate) fn build_topology_browser_items(document: &TopologyDocument) -> Vec<BrowserItem> {
    let mut nodes = document.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by(|left, right| compare_topology_nodes(left, right));
    let node_lookup = document
        .nodes
        .iter()
        .map(|node| (node.id.clone(), topology_node_display_label(node)))
        .collect::<BTreeMap<String, String>>();
    nodes
        .into_iter()
        .map(|node| {
            let display_label = if node.label.is_empty() {
                node.id.clone()
            } else {
                node.label.clone()
            };
            let mut inbound_edges = document
                .edges
                .iter()
                .filter(|edge| edge.to == node.id)
                .collect::<Vec<_>>();
            inbound_edges.sort_by(|left, right| {
                left.relation
                    .cmp(&right.relation)
                    .then_with(|| {
                        let left_label = node_lookup
                            .get(left.from.as_str())
                            .cloned()
                            .unwrap_or_else(|| left.from.clone());
                        let right_label = node_lookup
                            .get(right.from.as_str())
                            .cloned()
                            .unwrap_or_else(|| right.from.clone());
                        left_label.cmp(&right_label)
                    })
                    .then_with(|| left.from.cmp(&right.from))
            });

            let mut outbound_edges = document
                .edges
                .iter()
                .filter(|edge| edge.from == node.id)
                .collect::<Vec<_>>();
            outbound_edges.sort_by(|left, right| {
                left.relation
                    .cmp(&right.relation)
                    .then_with(|| {
                        let left_label = node_lookup
                            .get(left.to.as_str())
                            .cloned()
                            .unwrap_or_else(|| left.to.clone());
                        let right_label = node_lookup
                            .get(right.to.as_str())
                            .cloned()
                            .unwrap_or_else(|| right.to.clone());
                        left_label.cmp(&right_label)
                    })
                    .then_with(|| left.to.cmp(&right.to))
            });

            let mut details = vec![
                format!("Node ID: {}", node.id),
                format!("Kind: {}", node.kind),
                format!("Label: {}", display_label),
                format!("Inbound edges: {}", inbound_edges.len()),
                format!("Outbound edges: {}", outbound_edges.len()),
            ];
            let inbound_count = inbound_edges.len();
            let outbound_count = outbound_edges.len();
            if inbound_edges.is_empty() {
                details.push("Inbound edge summary: none".to_string());
            } else {
                details.push("Inbound edge summary:".to_string());
                for edge in &inbound_edges {
                    let source_label = node_lookup
                        .get(edge.from.as_str())
                        .cloned()
                        .unwrap_or_else(|| edge.from.clone());
                    details.push(format!("  {} <- {}", edge.relation, source_label));
                }
            }
            if outbound_edges.is_empty() {
                details.push("Outbound edge summary: none".to_string());
            } else {
                details.push("Outbound edge summary:".to_string());
                for edge in &outbound_edges {
                    let target_label = node_lookup
                        .get(edge.to.as_str())
                        .cloned()
                        .unwrap_or_else(|| edge.to.clone());
                    details.push(format!("  {} -> {}", edge.relation, target_label));
                }
            }
            BrowserItem {
                kind: node.kind.clone(),
                title: display_label,
                meta: format!(
                    "id={} | in={} out={}",
                    node.id, inbound_count, outbound_count
                ),
                details,
            }
        })
        .collect()
}

pub(crate) fn run_dashboard_topology(args: &TopologyArgs) -> Result<()> {
    let governance = load_object(&args.governance)?;
    let alert_contract = match args.alert_contract.as_ref() {
        Some(path) => Some(load_object(path)?),
        None => None,
    };
    let document = build_topology_document(&governance, alert_contract.as_ref())?;
    if args.interactive {
        #[cfg(not(test))]
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
    }
    let rendered = match args.output_format {
        TopologyOutputFormat::Text => render_topology_text(&document),
        TopologyOutputFormat::Json => serde_json::to_string_pretty(&document)?,
        TopologyOutputFormat::Mermaid => render_topology_mermaid(&document),
        TopologyOutputFormat::Dot => render_topology_dot(&document),
    };
    if let Some(output_file) = args.output_file.as_ref() {
        if matches!(args.output_format, TopologyOutputFormat::Json) {
            write_json_document(&document, output_file)?;
        } else {
            fs::write(output_file, &rendered)?;
        }
    }
    println!("{rendered}");
    Ok(())
}

pub(crate) fn run_dashboard_impact(args: &ImpactArgs) -> Result<()> {
    let governance = load_object(&args.governance)?;
    let alert_contract = match args.alert_contract.as_ref() {
        Some(path) => Some(load_object(path)?),
        None => None,
    };
    let document =
        build_impact_document(&governance, alert_contract.as_ref(), &args.datasource_uid)?;
    if args.interactive {
        #[cfg(not(test))]
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
    }
    match args.output_format {
        ImpactOutputFormat::Text => println!("{}", render_impact_text(&document)),
        ImpactOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&document)?),
    }
    Ok(())
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
