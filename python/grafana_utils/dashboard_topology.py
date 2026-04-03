"""Build and render a deterministic dashboard topology graph.

Purpose:
- Parse governance and optional alert-contract JSON artifacts.
- Construct dashboard/datasource/panel/variable/alert nodes and their relations.
- Render topology documents as text, JSON, Mermaid, or DOT.

Behavior notes:
- This module mirrors the Rust dashboard topology behavior, including the same
  document and relationship shapes.
- Interactive rendering is intentionally unsupported in Python at this time.
"""

import argparse
import json
from pathlib import Path
from typing import Any

from .dashboards.import_support import load_json_file
from .dashboards.common import GrafanaError


TOPOLOGY_OUTPUT_FORMAT_CHOICES = ("text", "json", "mermaid", "dot")


def _string_field(record: Any, key: str) -> str:
    value = record.get(key) if isinstance(record, dict) else None
    if isinstance(value, str):
        value = value.strip()
        return value
    return ""


def _string_list_field(record: Any, key: str) -> list[str]:
    value = record.get(key) if isinstance(record, dict) else None
    if not isinstance(value, list):
        return []
    values: list[str] = []
    for item in value:
        if isinstance(item, str):
            item = item.strip()
            if item:
                values.append(item)
    return values


def _require_array(document: dict[str, Any], key: str, message: str) -> list[Any]:
    value = document.get(key)
    if not isinstance(value, list):
        raise GrafanaError(message)
    return value


def _normalize_alert_kind(kind: str) -> str:
    if kind == "grafana-alert-rule":
        return "alert-rule"
    if kind == "grafana-contact-point":
        return "contact-point"
    if kind == "grafana-mute-timing":
        return "mute-timing"
    if kind in ("grafana-notification-policies", "grafana-notification-policy"):
        return "notification-policy"
    if kind == "grafana-notification-template":
        return "template"
    return "alert-resource"


def _alert_resource_label(title: str, identity: str) -> str:
    return identity if not title else title


def _collect_alert_resources(document: dict[str, Any]) -> list[dict[str, Any]]:
    resources = document.get("resources")
    if not isinstance(resources, list):
        raise GrafanaError("Alert contract JSON must contain a resources array.")
    parsed_resources: list[dict[str, Any]] = []
    for resource in resources:
        if not isinstance(resource, dict):
            continue
        kind = _string_field(resource, "kind")
        identity = _string_field(resource, "identity")
        if not kind or not identity:
            continue
        parsed_resources.append(
            {
                "kind": kind,
                "identity": identity,
                "title": _string_field(resource, "title"),
                "source_path": _string_field(resource, "sourcePath"),
                "references": _string_list_field(resource, "references"),
            }
        )
    return parsed_resources


def _edge_relation_for_alert_reference(
    source_kind: str,
    target_kind: str,
) -> str | None:
    if source_kind == "alert-rule" and target_kind == "contact-point":
        return "routes-to"
    if source_kind == "alert-rule" and target_kind == "notification-policy":
        return "routes-to"
    if source_kind == "alert-rule" and target_kind == "template":
        return "uses-template"
    if source_kind == "contact-point" and target_kind == "template":
        return "uses-template"
    if source_kind == "notification-policy" and target_kind == "template":
        return "uses-template"
    return None


def _sort_nodes(nodes: list[dict[str, str]]) -> list[dict[str, str]]:
    return sorted(nodes, key=lambda item: (item["kind"], item["label"], item["id"]))


def _slug_for_mermaid(value: str) -> str:
    output = []
    for character in value:
        if character.isascii() and (character.isalnum() or character == "_"):
            output.append(character)
        else:
            output.append("_")
    slug = "".join(output)
    if not slug or slug[0].isdigit():
        slug = f"n{slug}"
    return slug


def _escape_label(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', "\\\"")


def build_topology_document(
    governance_document: dict[str, Any],
    alert_contract_document: dict[str, Any] | None,
) -> dict[str, Any]:
    dashboard_edges = _require_array(
        governance_document,
        "dashboardDatasourceEdges",
        "Dashboard governance JSON must contain a dashboardDatasourceEdges array.",
    )
    dashboards = _require_array(
        governance_document,
        "dashboardGovernance",
        "Dashboard governance JSON must contain a dashboardGovernance array.",
    )

    nodes: dict[str, dict[str, str]] = {}
    edges: set[tuple[str, str, str]] = set()
    alert_identity_to_node: dict[str, str] = {}
    alert_identity_to_kind: dict[str, str] = {}
    datasource_names_to_uid: dict[str, str] = {}

    for dashboard in dashboards:
        if not isinstance(dashboard, dict):
            continue
        dashboard_uid = _string_field(dashboard, "dashboardUid")
        if not dashboard_uid:
            continue
        dashboard_title = _string_field(dashboard, "dashboardTitle")
        nodes.setdefault(
            f"dashboard:{dashboard_uid}",
            {
                "id": f"dashboard:{dashboard_uid}",
                "kind": "dashboard",
                "label": dashboard_title or dashboard_uid,
            },
        )

    for edge in dashboard_edges:
        if not isinstance(edge, dict):
            continue
        datasource_uid = _string_field(edge, "datasourceUid")
        datasource_name = _string_field(edge, "datasource")
        dashboard_uid = _string_field(edge, "dashboardUid")
        if not datasource_uid or not dashboard_uid:
            continue
        if datasource_name:
            datasource_names_to_uid[datasource_name] = datasource_uid
        nodes.setdefault(
            f"datasource:{datasource_uid}",
            {
                "id": f"datasource:{datasource_uid}",
                "kind": "datasource",
                "label": datasource_name or datasource_uid,
            },
        )
        edges.add(
            (
                f"datasource:{datasource_uid}",
                f"dashboard:{dashboard_uid}",
                "feeds",
            )
        )
        for variable in _string_list_field(edge, "queryVariables"):
            variable_id = f"variable:{dashboard_uid}:{variable}"
            nodes.setdefault(
                variable_id,
                {
                    "id": variable_id,
                    "kind": "variable",
                    "label": variable,
                },
            )
            edges.add((f"datasource:{datasource_uid}", variable_id, "feeds-variable"))

    dashboard_dependencies = governance_document.get("dashboardDependencies", [])
    if isinstance(dashboard_dependencies, list):
        for dependency in dashboard_dependencies:
            if not isinstance(dependency, dict):
                continue
            dashboard_uid = _string_field(dependency, "dashboardUid")
            if not dashboard_uid:
                continue
            panel_ids = _string_list_field(dependency, "panelIds")
            if not panel_ids:
                continue
            variable_names = set(_string_list_field(dependency, "panelVariables"))
            variable_names.update(_string_list_field(dependency, "queryVariables"))
            for panel_id in panel_ids:
                panel_id = panel_id.strip()
                if not panel_id:
                    continue
                panel_id_node = f"panel:{dashboard_uid}:{panel_id}"
                nodes.setdefault(
                    panel_id_node,
                    {
                        "id": panel_id_node,
                        "kind": "panel",
                        "label": f"Panel {panel_id}",
                    },
                )
                edges.add((panel_id_node, f"dashboard:{dashboard_uid}", "belongs-to"))
                for variable in variable_names:
                    variable_node = f"variable:{dashboard_uid}:{variable}"
                    nodes.setdefault(
                        variable_node,
                        {
                            "id": variable_node,
                            "kind": "variable",
                            "label": variable,
                        },
                    )
                    edges.add((variable_node, panel_id_node, "used-by"))

    alert_resource_count = 0
    if alert_contract_document is not None:
        for resource in _collect_alert_resources(alert_contract_document):
            identity = resource["identity"]
            normalized_kind = _normalize_alert_kind(resource["kind"])
            node_id = f"alert:{normalized_kind}:{identity}"
            nodes.setdefault(
                node_id,
                {
                    "id": node_id,
                    "kind": normalized_kind,
                    "label": _alert_resource_label(resource["title"], identity),
                },
            )
            alert_identity_to_node[identity] = node_id
            alert_identity_to_kind[identity] = normalized_kind
            alert_resource_count += 1

        for resource in _collect_alert_resources(alert_contract_document):
            identity = resource["identity"]
            source_kind = _normalize_alert_kind(resource["kind"])
            source_node = f"alert:{source_kind}:{identity}"
            for reference in resource["references"]:
                reference = reference.strip()
                if not reference:
                    continue
                target_node = alert_identity_to_node.get(reference)
                if target_node:
                    target_kind = alert_identity_to_kind.get(reference)
                    if target_kind:
                        relation = _edge_relation_for_alert_reference(
                            source_kind,
                            target_kind,
                        )
                        if relation:
                            edges.add((source_node, target_node, relation))

                datasource_uid = datasource_names_to_uid.get(reference, reference)
                if source_kind == "alert-rule" and (
                    f"datasource:{datasource_uid}" in nodes
                ):
                    edges.add((f"datasource:{datasource_uid}", source_node, "alerts-on"))

                if source_kind == "alert-rule" and (
                    f"dashboard:{reference}" in nodes
                ):
                    edges.add((f"dashboard:{reference}", source_node, "backs"))

    node_list = _sort_nodes(list(nodes.values()))
    edge_list = [
        {"from": from_id, "to": to_id, "relation": relation}
        for from_id, to_id, relation in sorted(edges)
    ]

    summary = {
        "nodeCount": len(node_list),
        "edgeCount": len(edge_list),
        "datasourceCount": len([node for node in node_list if node["kind"] == "datasource"]),
        "dashboardCount": len([node for node in node_list if node["kind"] == "dashboard"]),
        "panelCount": len([node for node in node_list if node["kind"] == "panel"]),
        "variableCount": len([node for node in node_list if node["kind"] == "variable"]),
        "alertResourceCount": alert_resource_count,
        "alertRuleCount": len(
            [node for node in node_list if node["kind"] == "alert-rule"]
        ),
        "contactPointCount": len(
            [node for node in node_list if node["kind"] == "contact-point"]
        ),
        "muteTimingCount": len(
            [node for node in node_list if node["kind"] == "mute-timing"]
        ),
        "notificationPolicyCount": len(
            [node for node in node_list if node["kind"] == "notification-policy"]
        ),
        "templateCount": len([node for node in node_list if node["kind"] == "template"]),
    }

    return {
        "summary": summary,
        "nodes": node_list,
        "edges": edge_list,
    }


def render_topology_text(document: dict[str, Any]) -> str:
    lines = [
        "Dashboard topology: nodes={nodeCount} edges={edgeCount} datasources={datasourceCount} dashboards={dashboardCount} panels={panelCount} variables={variableCount} alert-resources={alertResourceCount} alert-rules={alertRuleCount} contact-points={contactPointCount} mute-timings={muteTimingCount} notification-policies={notificationPolicyCount} templates={templateCount}".format(
            **document["summary"]
        )
    ]
    for edge in document["edges"]:
        lines.append(
            f"  {edge['from']} --{edge['relation']}--> {edge['to']}"
        )
    return "\n".join(lines)


def render_topology_json(document: dict[str, Any]) -> str:
    return json.dumps(document, indent=2, ensure_ascii=False)


def render_topology_mermaid(document: dict[str, Any]) -> str:
    lines = ["graph TD"]
    for node in document["nodes"]:
        lines.append(
            f"  {_slug_for_mermaid(node['id'])}[\"{_escape_label(node['label'])}\"]"
        )
    for edge in document["edges"]:
        lines.append(
            f"  {_slug_for_mermaid(edge['from'])} -->|{edge['relation']}| {_slug_for_mermaid(edge['to'])}"
        )
    return "\n".join(lines)


def render_topology_dot(document: dict[str, Any]) -> str:
    lines = ["digraph grafana_topology {"]
    for node in document["nodes"]:
        lines.append(
            f"  \"{_escape_label(node['id'])}\" [label=\"{_escape_label(node['label'])}\\n{node['kind']}\"] ;"
        )
    for edge in document["edges"]:
        lines.append(
            f"  \"{_escape_label(edge['from'])}\" -> \"{_escape_label(edge['to'])}\" [label=\"{_escape_label(edge['relation'])}\"] ;"
        )
    lines.append("}")
    return "\n".join(lines)


def _render_topology(document: dict[str, Any], output_format: str) -> str:
    if output_format == "json":
        return render_topology_json(document)
    if output_format == "mermaid":
        return render_topology_mermaid(document)
    if output_format == "dot":
        return render_topology_dot(document)
    return render_topology_text(document)


def run_dashboard_topology(args: argparse.Namespace) -> int:
    """Build and render dashboard topology from governance artifacts."""
    governance_path = Path(args.governance)
    alert_contract = (
        load_json_file(Path(args.alert_contract))
        if getattr(args, "alert_contract", None)
        else None
    )
    topology_document = build_topology_document(
        load_json_file(governance_path),
        alert_contract,
    )
    if bool(getattr(args, "interactive", False)):
        raise GrafanaError(
            "Topology interactive mode is not supported in the Python CLI yet."
        )
    output_format = str(getattr(args, "output_format", "text"))
    rendered = _render_topology(topology_document, output_format)
    output_file = getattr(args, "output_file", None)
    if output_file:
        output_path = Path(output_file)
        if output_path.parent:
            output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(f"{rendered}\n", encoding="utf-8")
    print(rendered)
    return 0

