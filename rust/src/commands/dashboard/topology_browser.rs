//! Browser item builders for topology and impact inspection.
use std::collections::BTreeMap;

use crate::interactive_browser::BrowserItem;

use super::topology_build::compare_topology_nodes;
use super::{ImpactAlertResource, ImpactDashboard, ImpactDocument, TopologyDocument, TopologyNode};

fn sort_impact_resources(resources: &mut Vec<&ImpactAlertResource>) {
    resources.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then_with(|| left.identity.cmp(&right.identity))
            .then_with(|| left.source_path.cmp(&right.source_path))
    });
}

fn topology_node_display_label(node: &TopologyNode) -> String {
    if node.label.is_empty() {
        node.id.clone()
    } else {
        node.label.clone()
    }
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

pub(crate) fn build_topology_browser_items(document: &TopologyDocument) -> Vec<BrowserItem> {
    let mut nodes = document.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by(|left, right| compare_topology_nodes(left, right));
    let node_lookup = document
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.clone(),
                (topology_node_display_label(node), node.kind.clone()),
            )
        })
        .collect::<BTreeMap<String, (String, String)>>();
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
                            .map(|(label, _)| label.clone())
                            .unwrap_or_else(|| left.from.clone());
                        let right_label = node_lookup
                            .get(right.from.as_str())
                            .map(|(label, _)| label.clone())
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
                            .map(|(label, _)| label.clone())
                            .unwrap_or_else(|| left.to.clone());
                        let right_label = node_lookup
                            .get(right.to.as_str())
                            .map(|(label, _)| label.clone())
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
                    let (source_label, source_kind) = node_lookup
                        .get(edge.from.as_str())
                        .cloned()
                        .unwrap_or_else(|| (edge.from.clone(), "unknown".to_string()));
                    details.push(format!(
                        "  {} <- {} [{}]",
                        edge.relation, source_label, source_kind
                    ));
                }
            }
            if outbound_edges.is_empty() {
                details.push("Outbound edge summary: none".to_string());
            } else {
                details.push("Outbound edge summary:".to_string());
                for edge in &outbound_edges {
                    let (target_label, target_kind) = node_lookup
                        .get(edge.to.as_str())
                        .cloned()
                        .unwrap_or_else(|| (edge.to.clone(), "unknown".to_string()));
                    details.push(format!(
                        "  {} -> {} [{}]",
                        edge.relation, target_label, target_kind
                    ));
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
