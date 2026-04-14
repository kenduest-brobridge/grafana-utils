use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ReferenceNodeKind {
    Dashboard,
    Datasource,
    Panel,
    Variable,
    AlertRule,
    ContactPoint,
    MuteTiming,
    NotificationPolicy,
    Template,
    Folder,
    Unknown(String),
}

impl ReferenceNodeKind {
    pub(crate) fn from_topology_kind(kind: &str) -> Self {
        match kind {
            "dashboard" => Self::Dashboard,
            "datasource" => Self::Datasource,
            "panel" => Self::Panel,
            "variable" => Self::Variable,
            "alert-rule" => Self::AlertRule,
            "contact-point" => Self::ContactPoint,
            "mute-timing" => Self::MuteTiming,
            "notification-policy" => Self::NotificationPolicy,
            "template" => Self::Template,
            "folder" => Self::Folder,
            other => Self::Unknown(other.to_string()),
        }
    }

    #[cfg(test)]
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Datasource => "datasource",
            Self::Panel => "panel",
            Self::Variable => "variable",
            Self::AlertRule => "alert-rule",
            Self::ContactPoint => "contact-point",
            Self::MuteTiming => "mute-timing",
            Self::NotificationPolicy => "notification-policy",
            Self::Template => "template",
            Self::Folder => "folder",
            Self::Unknown(kind) => kind.as_str(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ReferenceRelation {
    Feeds,
    FeedsVariable,
    BelongsTo,
    UsedBy,
    RoutesTo,
    UsesTemplate,
    AlertsOn,
    Backs,
    DependsOn,
    References,
    Other(String),
}

impl ReferenceRelation {
    pub(crate) fn from_topology_relation(relation: &str) -> Self {
        match relation {
            "feeds" => Self::Feeds,
            "feeds-variable" => Self::FeedsVariable,
            "belongs-to" => Self::BelongsTo,
            "used-by" => Self::UsedBy,
            "routes-to" => Self::RoutesTo,
            "uses-template" => Self::UsesTemplate,
            "alerts-on" => Self::AlertsOn,
            "backs" => Self::Backs,
            "depends-on" => Self::DependsOn,
            "references" => Self::References,
            other => Self::Other(other.to_string()),
        }
    }

    #[cfg(test)]
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Feeds => "feeds",
            Self::FeedsVariable => "feeds-variable",
            Self::BelongsTo => "belongs-to",
            Self::UsedBy => "used-by",
            Self::RoutesTo => "routes-to",
            Self::UsesTemplate => "uses-template",
            Self::AlertsOn => "alerts-on",
            Self::Backs => "backs",
            Self::DependsOn => "depends-on",
            Self::References => "references",
            Self::Other(relation) => relation.as_str(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReferenceNode {
    pub(crate) id: String,
    pub(crate) kind: ReferenceNodeKind,
    pub(crate) label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct ReferenceEdge {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) relation: ReferenceRelation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReferenceGraph {
    pub(crate) nodes: BTreeMap<String, ReferenceNode>,
    pub(crate) edges: BTreeSet<ReferenceEdge>,
}

impl ReferenceGraph {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[cfg(test)]
    pub(crate) fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub(crate) fn insert_node(&mut self, node: ReferenceNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    #[cfg(test)]
    pub(crate) fn ensure_node(
        &mut self,
        id: impl Into<String>,
        kind: ReferenceNodeKind,
        label: impl Into<String>,
    ) {
        let id = id.into();
        self.nodes.entry(id.clone()).or_insert(ReferenceNode {
            id,
            kind,
            label: label.into(),
            source_path: None,
        });
    }

    pub(crate) fn add_edge(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        relation: ReferenceRelation,
    ) {
        self.edges.insert(ReferenceEdge {
            from: from.into(),
            to: to.into(),
            relation,
        });
    }

    pub(crate) fn reachable_from(&self, root_id: &str) -> BTreeSet<String> {
        let mut adjacency = BTreeMap::<String, Vec<String>>::new();
        for edge in &self.edges {
            adjacency
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
        }

        let mut reachable = BTreeSet::<String>::new();
        let mut queue = VecDeque::from([root_id.to_string()]);
        while let Some(node_id) = queue.pop_front() {
            if !reachable.insert(node_id.clone()) {
                continue;
            }
            if let Some(targets) = adjacency.get(&node_id) {
                for target in targets {
                    queue.push_back(target.clone());
                }
            }
        }
        reachable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reference_graph_deduplicates_edges() {
        let mut graph = ReferenceGraph::new();
        graph.ensure_node(
            "datasource:prom-main",
            ReferenceNodeKind::Datasource,
            "Prometheus Main",
        );
        graph.ensure_node(
            "dashboard:cpu-main",
            ReferenceNodeKind::Dashboard,
            "CPU Main",
        );
        graph.add_edge(
            "datasource:prom-main",
            "dashboard:cpu-main",
            ReferenceRelation::Feeds,
        );
        graph.add_edge(
            "datasource:prom-main",
            "dashboard:cpu-main",
            ReferenceRelation::Feeds,
        );

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(
            graph.nodes["datasource:prom-main"].kind.as_str(),
            "datasource"
        );
        assert_eq!(ReferenceRelation::Feeds.as_str(), "feeds");
    }

    #[test]
    fn reference_graph_reachability_follows_topology_edges() {
        let mut graph = ReferenceGraph::new();
        graph.ensure_node(
            "datasource:prom-main",
            ReferenceNodeKind::Datasource,
            "Prometheus Main",
        );
        graph.ensure_node(
            "dashboard:cpu-main",
            ReferenceNodeKind::Dashboard,
            "CPU Main",
        );
        graph.ensure_node(
            "alert:alert-rule:cpu-high",
            ReferenceNodeKind::AlertRule,
            "CPU High",
        );
        graph.ensure_node(
            "alert:contact-point:pagerduty-primary",
            ReferenceNodeKind::ContactPoint,
            "PagerDuty Primary",
        );
        graph.add_edge(
            "datasource:prom-main",
            "dashboard:cpu-main",
            ReferenceRelation::Feeds,
        );
        graph.add_edge(
            "datasource:prom-main",
            "alert:alert-rule:cpu-high",
            ReferenceRelation::AlertsOn,
        );
        graph.add_edge(
            "alert:alert-rule:cpu-high",
            "alert:contact-point:pagerduty-primary",
            ReferenceRelation::RoutesTo,
        );

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.edge_count(), 3);
        assert_eq!(graph.nodes["dashboard:cpu-main"].kind.as_str(), "dashboard");
        assert_eq!(
            graph.reachable_from("datasource:prom-main"),
            [
                "alert:alert-rule:cpu-high".to_string(),
                "alert:contact-point:pagerduty-primary".to_string(),
                "dashboard:cpu-main".to_string(),
                "datasource:prom-main".to_string(),
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            graph.reachable_from("missing"),
            ["missing".to_string()].into_iter().collect()
        );
    }

    #[test]
    fn reference_graph_serializes_kinds_and_relations() {
        let mut graph = ReferenceGraph::new();
        graph.ensure_node(
            "dashboard:cpu-main",
            ReferenceNodeKind::Dashboard,
            "CPU Main",
        );
        graph.add_edge(
            "datasource:prom-main",
            "dashboard:cpu-main",
            ReferenceRelation::Feeds,
        );
        let value = serde_json::to_value(graph).unwrap();

        assert_eq!(
            value["nodes"]["dashboard:cpu-main"]["kind"],
            json!("dashboard")
        );
        assert_eq!(value["edges"][0]["relation"], json!("feeds"));
    }
}
