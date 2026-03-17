"""Unwired workbench for roadmap features that are not ready for CLI exposure.

This module exists to let roadmap work start in one isolated file before the
implementation is threaded through the current dashboard/datasource/alert
command paths. Keep it import-safe and side-effect free until the public
contract is ready.
"""

from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Sequence


@dataclass(frozen=True)
class WorkbenchTask:
    """One roadmap work item tracked in the temporary development workbench."""

    section: str
    name: str
    outcome: str
    candidate_modules: Sequence[str]


INSPECTION_AND_GOVERNANCE = "inspection-and-dependency-governance"
PROMOTION_AND_PREFLIGHT = "environment-promotion-and-preflight-safety"
DECLARATIVE_SYNC_AND_GITOPS = "declarative-sync-and-gitops"
SECRET_HANDLING_AND_REDACTION = "secret-handling-and-redaction"
DEPENDENCY_GRAPH_KIND = "grafana-utils-resource-dependency-graph"
DEPENDENCY_GRAPH_SCHEMA_VERSION = 1
PROMOTION_PLAN_KIND = "grafana-utils-promotion-plan"
PROMOTION_PLAN_SCHEMA_VERSION = 1
PREFLIGHT_CHECK_KIND = "grafana-utils-preflight-check"
PREFLIGHT_CHECK_SCHEMA_VERSION = 1


WORKBENCH_TASKS = (
    WorkbenchTask(
        section=INSPECTION_AND_GOVERNANCE,
        name="dashboard-datasource-dependency-summary",
        outcome="Summarize dashboard-to-datasource dependencies for operator review.",
        candidate_modules=(
            "grafana_utils.dashboards.inspection_summary",
            "grafana_utils.dashboards.inspection_governance",
            "grafana_utils.dashboards.inspection_report",
        ),
    ),
    WorkbenchTask(
        section=INSPECTION_AND_GOVERNANCE,
        name="blast-radius-and-orphan-detection",
        outcome="Report datasource usage hotspots and unused datasource inventory.",
        candidate_modules=(
            "grafana_utils.dashboards.inspection_governance",
            "grafana_utils.datasource_diff",
        ),
    ),
    WorkbenchTask(
        section=INSPECTION_AND_GOVERNANCE,
        name="resource-dependency-graph-export",
        outcome="Define one graph document that can later render JSON, DOT, or SVG.",
        candidate_modules=(
            "grafana_utils.dashboards.inspection_governance",
            "grafana_utils.dashboards.inspection_render",
        ),
    ),
    WorkbenchTask(
        section=INSPECTION_AND_GOVERNANCE,
        name="datasource-type-query-analyzers",
        outcome="Keep analyzers split by datasource family instead of bloating one parser.",
        candidate_modules=(
            "grafana_utils.dashboards.inspection_analyzers.dispatcher",
            "grafana_utils.dashboards.inspection_analyzers.prometheus",
            "grafana_utils.dashboards.inspection_analyzers.loki",
            "grafana_utils.dashboards.inspection_analyzers.flux",
            "grafana_utils.dashboards.inspection_analyzers.sql",
        ),
    ),
    WorkbenchTask(
        section=INSPECTION_AND_GOVERNANCE,
        name="static-html-inspection-report",
        outcome="Render inspection output as static HTML from the canonical report model.",
        candidate_modules=(
            "grafana_utils.dashboards.inspection_render",
            "grafana_utils.dashboards.inspection_report",
        ),
    ),
    WorkbenchTask(
        section=PROMOTION_AND_PREFLIGHT,
        name="promote-command-workflow",
        outcome="Wrap export/import/diff into one reviewable environment-promotion flow.",
        candidate_modules=(
            "grafana_utils.dashboard_cli",
            "grafana_utils.datasource_cli",
            "grafana_utils.dashboards.import_workflow",
        ),
    ),
    WorkbenchTask(
        section=PROMOTION_AND_PREFLIGHT,
        name="import-preflight-checks",
        outcome="Check target datasource, plugin, alert, contact-point, and library-panel prerequisites before mutation.",
        candidate_modules=(
            "grafana_utils.dashboards.import_support",
            "grafana_utils.datasource.workflows",
        ),
    ),
    WorkbenchTask(
        section=PROMOTION_AND_PREFLIGHT,
        name="uid-and-name-remap-rules",
        outcome="Make datasource/dashboard remap rules explicit and reviewable.",
        candidate_modules=(
            "grafana_utils.dashboards.import_support",
            "grafana_utils.datasource.workflows",
        ),
    ),
    WorkbenchTask(
        section=PROMOTION_AND_PREFLIGHT,
        name="reviewable-dry-run-and-diff-contract",
        outcome="Keep promotion dry-run and diff output stable enough for team review.",
        candidate_modules=(
            "grafana_utils.dashboards.import_support",
            "grafana_utils.datasource_diff",
        ),
    ),
    WorkbenchTask(
        section=DECLARATIVE_SYNC_AND_GITOPS,
        name="gitops-sync-plan-contract",
        outcome="Model declarative Grafana sync as a reviewable dry-run-first plan before live apply wiring.",
        candidate_modules=(
            "grafana_utils.gitops_sync",
            "grafana_utils.dashboard_cli",
            "grafana_utils.datasource_cli",
            "grafana_utils.alert_cli",
        ),
    ),
    WorkbenchTask(
        section=DECLARATIVE_SYNC_AND_GITOPS,
        name="restricted-state-bundle-shape",
        outcome="Define the managed Grafana state slice that declarative sync is allowed to own.",
        candidate_modules=(
            "grafana_utils.gitops_sync",
            "grafana_utils.datasource_contract",
            "grafana_utils.dashboards.export_inventory",
        ),
    ),
    WorkbenchTask(
        section=SECRET_HANDLING_AND_REDACTION,
        name="datasource-secret-placeholder-contract",
        outcome="Keep datasource secret inputs explicit, placeholder-based, and fail-closed before import wiring.",
        candidate_modules=(
            "grafana_utils.datasource_secret_workbench",
            "grafana_utils.datasource.workflows",
            "grafana_utils.datasource.parser",
        ),
    ),
    WorkbenchTask(
        section=SECRET_HANDLING_AND_REDACTION,
        name="python-rust-secret-flow-parity",
        outcome="Keep password and token handling semantics aligned before broader secret-provider support lands.",
        candidate_modules=(
            "grafana_utils.datasource_secret_workbench",
            "grafana_utils.auth_staging",
            "rust/src/datasource.rs",
            "rust/src/common.rs",
        ),
    ),
)


def list_workbench_sections() -> List[str]:
    """Return the known roadmap workbench sections in stable order."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return [
        INSPECTION_AND_GOVERNANCE,
        PROMOTION_AND_PREFLIGHT,
        DECLARATIVE_SYNC_AND_GITOPS,
        SECRET_HANDLING_AND_REDACTION,
    ]


def list_workbench_tasks(section: str | None = None) -> List[WorkbenchTask]:
    """Return all workbench tasks, optionally filtered to one roadmap section."""
    if section is None:
        return list(WORKBENCH_TASKS)
    return [task for task in WORKBENCH_TASKS if task.section == section]


def build_workbench_index() -> Dict[str, List[str]]:
    """Return a simple section-to-task-name index for later CLI wiring."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    index: Dict[str, List[str]] = {}
    for task in WORKBENCH_TASKS:
        index.setdefault(task.section, []).append(task.name)
    return index


def iter_candidate_modules(section: str | None = None) -> Iterable[str]:
    """Yield unique candidate module names in first-seen order."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 177

    seen = set()
    for task in list_workbench_tasks(section=section):
        for module_name in task.candidate_modules:
            if module_name in seen:
                continue
            seen.add(module_name)
            yield module_name


def _normalize_text(value: Any, default: str = "") -> str:
    """Internal helper for normalize text."""
    text = str(value or "").strip()
    if text:
        return text
    return default


def _build_dashboard_node_id(dashboard_uid: str) -> str:
    """Internal helper for build dashboard node id."""
    return "dashboard:%s" % dashboard_uid


def _build_panel_node_id(dashboard_uid: str, panel_id: str) -> str:
    """Internal helper for build panel node id."""
    return "panel:%s:%s" % (dashboard_uid, panel_id)


def _build_datasource_node_id(datasource_uid: str) -> str:
    """Internal helper for build datasource node id."""
    return "datasource:%s" % datasource_uid


def _resolve_datasource_inventory(
    summary_document: dict[str, Any]
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    """Internal helper for resolve datasource inventory."""
    by_uid = {}
    by_name = {}
    for item in summary_document.get("datasourceInventory") or []:
        if not isinstance(item, dict):
            continue
        record = dict(item)
        uid = _normalize_text(record.get("uid"))
        name = _normalize_text(record.get("name"))
        if uid:
            by_uid[uid] = record
        if name:
            by_name[name] = record
    return by_uid, by_name


def _resolve_query_datasource_record(
    query_record: dict[str, Any],
    datasource_by_uid: dict[str, dict[str, Any]],
    datasource_by_name: dict[str, dict[str, Any]],
) -> dict[str, str]:
    """Internal helper for resolve query datasource record."""
    datasource_uid = _normalize_text(query_record.get("datasourceUid"))
    datasource_label = _normalize_text(query_record.get("datasource"))
    inventory = None
    if datasource_uid:
        inventory = datasource_by_uid.get(datasource_uid)
    if inventory is None and datasource_label:
        inventory = datasource_by_uid.get(datasource_label) or datasource_by_name.get(
            datasource_label
        )
    if inventory is not None:
        return {
            "uid": _normalize_text(inventory.get("uid"), datasource_uid or datasource_label),
            "name": _normalize_text(
                inventory.get("name"), datasource_label or datasource_uid or "Unknown datasource"
            ),
            "type": _normalize_text(inventory.get("type"), "unknown"),
        }
    return {
        "uid": datasource_uid or datasource_label or "unknown",
        "name": datasource_label or datasource_uid or "Unknown datasource",
        "type": "unknown",
    }


def build_dependency_graph_document(
    summary_document: dict[str, Any],
    report_document: dict[str, Any],
) -> dict[str, Any]:
    """Build an unwired dependency graph JSON contract from inspection documents."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 203, 211, 216, 221, 226, 245, 286, 312

    datasource_by_uid, datasource_by_name = _resolve_datasource_inventory(
        summary_document
    )
    nodes_by_id: Dict[str, dict[str, Any]] = {}
    edges_by_key: Dict[str, dict[str, Any]] = {}

    def ensure_node(node_id: str, node_type: str, label: str, **attrs: Any) -> None:
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        record = nodes_by_id.get(node_id)
        if record is None:
            record = {"id": node_id, "type": node_type, "label": label}
            nodes_by_id[node_id] = record
        for key, value in attrs.items():
            if value in (None, "", [], ()):
                continue
            if key == "queryCount":
                record[key] = int(record.get(key) or 0) + int(value or 0)
                continue
            if key == "datasourceTypes":
                seen_values = set(record.get(key) or [])
                for item in value:
                    text = _normalize_text(item)
                    if text and text not in seen_values:
                        seen_values.add(text)
                record[key] = sorted(seen_values)
                continue
            if key not in record:
                record[key] = value

    def ensure_edge(
        source: str, target: str, relation: str, dashboard_uid: str, panel_id: str
    ) -> None:
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        edge_id = "%s|%s|%s" % (source, relation, target)
        record = edges_by_key.get(edge_id)
        if record is None:
            record = {
                "source": source,
                "target": target,
                "relation": relation,
                "dashboardUids": [],
                "panelIds": [],
                "queryCount": 0,
            }
            edges_by_key[edge_id] = record
        if dashboard_uid and dashboard_uid not in record["dashboardUids"]:
            record["dashboardUids"].append(dashboard_uid)
        if panel_id and panel_id not in record["panelIds"]:
            record["panelIds"].append(panel_id)
        record["queryCount"] = int(record.get("queryCount") or 0) + 1

    for datasource in summary_document.get("datasourceInventory") or []:
        if not isinstance(datasource, dict):
            continue
        datasource_uid = _normalize_text(datasource.get("uid"), "unknown")
        ensure_node(
            _build_datasource_node_id(datasource_uid),
            "datasource",
            _normalize_text(datasource.get("name"), datasource_uid),
            uid=datasource_uid,
            datasourceType=_normalize_text(datasource.get("type"), "unknown"),
            orgId=_normalize_text(datasource.get("orgId")),
            referenceCount=int(datasource.get("referenceCount") or 0),
        )

    for query in report_document.get("queries") or []:
        if not isinstance(query, dict):
            continue
        dashboard_uid = _normalize_text(query.get("dashboardUid"), "unknown-dashboard")
        dashboard_title = _normalize_text(query.get("dashboardTitle"), dashboard_uid)
        panel_id = _normalize_text(query.get("panelId"), "unknown-panel")
        panel_title = _normalize_text(query.get("panelTitle"), panel_id)
        folder_path = _normalize_text(query.get("folderPath"))
        datasource = _resolve_query_datasource_record(
            query, datasource_by_uid, datasource_by_name
        )
        datasource_uid = _normalize_text(datasource.get("uid"), "unknown")
        datasource_name = _normalize_text(datasource.get("name"), datasource_uid)
        datasource_type = _normalize_text(datasource.get("type"), "unknown")
        dashboard_node_id = _build_dashboard_node_id(dashboard_uid)
        panel_node_id = _build_panel_node_id(dashboard_uid, panel_id)
        datasource_node_id = _build_datasource_node_id(datasource_uid)

        ensure_node(
            dashboard_node_id,
            "dashboard",
            dashboard_title,
            uid=dashboard_uid,
            folderPath=folder_path,
        )
        ensure_node(
            panel_node_id,
            "panel",
            panel_title,
            dashboardUid=dashboard_uid,
            panelId=panel_id,
            panelType=_normalize_text(query.get("panelType")),
            queryCount=1,
        )
        ensure_node(
            datasource_node_id,
            "datasource",
            datasource_name,
            uid=datasource_uid,
            datasourceType=datasource_type,
        )

        ensure_edge(dashboard_node_id, panel_node_id, "contains-panel", dashboard_uid, panel_id)
        ensure_edge(
            panel_node_id, datasource_node_id, "queries-datasource", dashboard_uid, panel_id
        )

    nodes = sorted(nodes_by_id.values(), key=lambda item: (item["type"], item["id"]))
    edges = sorted(
        edges_by_key.values(),
        key=lambda item: (item["relation"], item["source"], item["target"]),
    )
    return {
        "kind": DEPENDENCY_GRAPH_KIND,
        "schemaVersion": DEPENDENCY_GRAPH_SCHEMA_VERSION,
        "section": INSPECTION_AND_GOVERNANCE,
        "summary": {
            "nodeCount": len(nodes),
            "edgeCount": len(edges),
            "dashboardCount": len([node for node in nodes if node["type"] == "dashboard"]),
            "panelCount": len([node for node in nodes if node["type"] == "panel"]),
            "datasourceCount": len([node for node in nodes if node["type"] == "datasource"]),
        },
        "nodes": nodes,
        "edges": edges,
    }


def _escape_dot_string(value: Any) -> str:
    """Internal helper for escape dot string."""
    text = str(value or "")
    return text.replace("\\", "\\\\").replace('"', '\\"')


def render_dependency_graph_dot(document: dict[str, Any]) -> str:
    """Render the dependency graph contract as a deterministic DOT document."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 419

    if str(document.get("kind") or "").strip() != DEPENDENCY_GRAPH_KIND:
        raise ValueError("Dependency graph document kind is not supported for DOT rendering.")
    lines = [
        "digraph grafana_dependency_graph {",
        '  rankdir="LR";',
        '  graph [label="Grafana Resource Dependency Graph", labelloc="t"];',
        '  node [shape="box"];',
    ]
    shape_by_type = {
        "dashboard": "folder",
        "panel": "box",
        "datasource": "cylinder",
    }
    for node in document.get("nodes") or []:
        if not isinstance(node, dict):
            continue
        node_id = _escape_dot_string(node.get("id"))
        label = _escape_dot_string(node.get("label") or node.get("id") or "")
        node_type = str(node.get("type") or "").strip()
        shape = shape_by_type.get(node_type, "box")
        lines.append(
            '  "%s" [label="%s", shape="%s"];' % (node_id, label, shape)
        )
    for edge in document.get("edges") or []:
        if not isinstance(edge, dict):
            continue
        source = _escape_dot_string(edge.get("source"))
        target = _escape_dot_string(edge.get("target"))
        relation = _escape_dot_string(edge.get("relation"))
        query_count = int(edge.get("queryCount") or 0)
        edge_label = relation
        if query_count > 1:
            edge_label = "%s (%s)" % (relation, query_count)
        lines.append(
            '  "%s" -> "%s" [label="%s"];' % (
                source,
                target,
                _escape_dot_string(edge_label),
            )
        )
    lines.append("}")
    return "\n".join(lines)


def build_dependency_graph_governance_summary(
    document: dict[str, Any],
) -> dict[str, Any]:
    """Summarize blast radius and orphan signals from one graph document."""
    if str(document.get("kind") or "").strip() != DEPENDENCY_GRAPH_KIND:
        raise ValueError(
            "Dependency graph document kind is not supported for governance summary."
        )
    dashboard_nodes = []
    panel_nodes = []
    datasource_nodes = []
    panel_to_datasource = {}
    dashboard_to_panels: Dict[str, set[str]] = {}
    datasource_to_panels: Dict[str, set[str]] = {}
    datasource_to_dashboards: Dict[str, set[str]] = {}

    for node in document.get("nodes") or []:
        if not isinstance(node, dict):
            continue
        node_type = str(node.get("type") or "").strip()
        if node_type == "dashboard":
            dashboard_nodes.append(node)
        elif node_type == "panel":
            panel_nodes.append(node)
        elif node_type == "datasource":
            datasource_nodes.append(node)

    for edge in document.get("edges") or []:
        if not isinstance(edge, dict):
            continue
        relation = str(edge.get("relation") or "").strip()
        source = _normalize_text(edge.get("source"))
        target = _normalize_text(edge.get("target"))
        if relation == "contains-panel":
            dashboard_to_panels.setdefault(source, set()).add(target)
            continue
        if relation == "queries-datasource":
            panel_to_datasource.setdefault(source, set()).add(target)

    panel_to_dashboard = {}
    for dashboard_id, panel_ids in dashboard_to_panels.items():
        for panel_id in panel_ids:
            panel_to_dashboard[panel_id] = dashboard_id

    for panel_id, datasource_ids in panel_to_datasource.items():
        dashboard_id = panel_to_dashboard.get(panel_id, "")
        for datasource_id in datasource_ids:
            datasource_to_panels.setdefault(datasource_id, set()).add(panel_id)
            if dashboard_id:
                datasource_to_dashboards.setdefault(datasource_id, set()).add(dashboard_id)

    orphaned_datasources = []
    blast_radius = []
    for node in datasource_nodes:
        datasource_id = _normalize_text(node.get("id"))
        impacted_panels = sorted(datasource_to_panels.get(datasource_id, set()))
        impacted_dashboards = sorted(datasource_to_dashboards.get(datasource_id, set()))
        record = {
            "datasourceNodeId": datasource_id,
            "datasourceUid": _normalize_text(node.get("uid"), datasource_id),
            "datasource": _normalize_text(node.get("label"), datasource_id),
            "datasourceType": _normalize_text(node.get("datasourceType"), "unknown"),
            "panelCount": len(impacted_panels),
            "dashboardCount": len(impacted_dashboards),
            "panelNodeIds": impacted_panels,
            "dashboardNodeIds": impacted_dashboards,
        }
        blast_radius.append(record)
        if len(impacted_panels) == 0 and len(impacted_dashboards) == 0:
            orphaned_datasources.append(record)

    blast_radius.sort(
        key=lambda item: (
            -int(item.get("dashboardCount") or 0),
            -int(item.get("panelCount") or 0),
            item.get("datasource"),
        )
    )
    orphaned_datasources.sort(key=lambda item: item.get("datasource"))
    return {
        "summary": {
            "dashboardCount": len(dashboard_nodes),
            "panelCount": len(panel_nodes),
            "datasourceCount": len(datasource_nodes),
            "orphanedDatasourceCount": len(orphaned_datasources),
            "blastRadiusRecordCount": len(blast_radius),
        },
        "orphanedDatasources": orphaned_datasources,
        "datasourceBlastRadius": blast_radius,
    }


def render_dependency_graph_governance_text(document: dict[str, Any]) -> List[str]:
    """Render graph-level governance summary as deterministic text."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 203, 471

    summary_document = build_dependency_graph_governance_summary(document)
    summary = summary_document.get("summary") or {}
    lines = [
        "Dependency graph governance summary",
        "Counts: %s dashboards, %s panels, %s datasources, %s orphaned"
        % (
            int(summary.get("dashboardCount") or 0),
            int(summary.get("panelCount") or 0),
            int(summary.get("datasourceCount") or 0),
            int(summary.get("orphanedDatasourceCount") or 0),
        ),
        "",
        "# Datasource Blast Radius",
    ]
    for item in summary_document.get("datasourceBlastRadius") or []:
        if not isinstance(item, dict):
            continue
        lines.append(
            "- %s uid=%s dashboards=%s panels=%s"
            % (
                _normalize_text(item.get("datasource"), "unknown"),
                _normalize_text(item.get("datasourceUid"), "unknown"),
                int(item.get("dashboardCount") or 0),
                int(item.get("panelCount") or 0),
            )
        )
    lines.append("")
    lines.append("# Orphaned Datasources")
    for item in summary_document.get("orphanedDatasources") or []:
        if not isinstance(item, dict):
            continue
        lines.append(
            "- %s uid=%s type=%s"
            % (
                _normalize_text(item.get("datasource"), "unknown"),
                _normalize_text(item.get("datasourceUid"), "unknown"),
                _normalize_text(item.get("datasourceType"), "unknown"),
            )
        )
    return lines


def _build_dashboard_bundle_lookup(
    bundle_document: dict[str, Any]
) -> Dict[str, dict[str, Any]]:
    """Internal helper for build dashboard bundle lookup."""
    lookup: Dict[str, dict[str, Any]] = {}
    for item in bundle_document.get("dashboards") or []:
        if not isinstance(item, dict):
            continue
        uid = _normalize_text(item.get("uid"))
        if not uid:
            continue
        lookup[uid] = item
    return lookup


def _build_datasource_bundle_lookup(
    bundle_document: dict[str, Any]
) -> Dict[str, dict[str, Any]]:
    """Internal helper for build datasource bundle lookup."""
    lookup: Dict[str, dict[str, Any]] = {}
    for item in bundle_document.get("datasources") or []:
        if not isinstance(item, dict):
            continue
        uid = _normalize_text(item.get("uid"))
        name = _normalize_text(item.get("name"))
        key = uid or name
        if not key:
            continue
        lookup[key] = item
    return lookup


def build_promotion_plan_document(
    source_bundle: dict[str, Any],
    target_inventory: dict[str, Any],
    options: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build a staged promotion/dry-run plan from source and target snapshots."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 203, 607, 622

    options = dict(options or {})
    source_env = _normalize_text(source_bundle.get("environment"), "source")
    target_env = _normalize_text(target_inventory.get("environment"), "target")
    dashboard_uid_map = dict(options.get("dashboardUidMap") or {})
    dashboard_name_map = dict(options.get("dashboardNameMap") or {})
    datasource_uid_map = dict(options.get("datasourceUidMap") or {})
    datasource_name_map = dict(options.get("datasourceNameMap") or {})
    require_preflight = bool(options.get("requirePreflight", True))

    source_dashboards = _build_dashboard_bundle_lookup(source_bundle)
    target_dashboards = _build_dashboard_bundle_lookup(target_inventory)
    source_datasources = _build_datasource_bundle_lookup(source_bundle)
    target_datasources = _build_datasource_bundle_lookup(target_inventory)

    plan_items = []

    for source_uid, source_dashboard in sorted(source_dashboards.items()):
        target_uid = _normalize_text(dashboard_uid_map.get(source_uid), source_uid)
        target_name = _normalize_text(
            dashboard_name_map.get(source_uid), _normalize_text(source_dashboard.get("title"))
        )
        target_dashboard = target_dashboards.get(target_uid)
        action = "create"
        if target_dashboard is not None:
            action = "update"
        plan_items.append(
            {
                "resourceType": "dashboard",
                "sourceUid": source_uid,
                "sourceName": _normalize_text(source_dashboard.get("title"), source_uid),
                "targetUid": target_uid,
                "targetName": target_name or target_uid,
                "action": action,
                "requiresPreflight": require_preflight,
                "folderPath": _normalize_text(source_dashboard.get("folderPath")),
            }
        )

    for source_key, source_datasource in sorted(source_datasources.items()):
        source_uid = _normalize_text(source_datasource.get("uid"), source_key)
        source_name = _normalize_text(source_datasource.get("name"), source_key)
        target_uid = _normalize_text(datasource_uid_map.get(source_uid), source_uid)
        target_name = _normalize_text(datasource_name_map.get(source_name), source_name)
        target_datasource = (
            target_datasources.get(target_uid)
            or target_datasources.get(target_name)
            or target_datasources.get(source_uid)
            or target_datasources.get(source_name)
        )
        action = "create"
        if target_datasource is not None:
            action = "update"
        plan_items.append(
            {
                "resourceType": "datasource",
                "sourceUid": source_uid,
                "sourceName": source_name,
                "targetUid": target_uid or source_uid,
                "targetName": target_name or source_name or target_uid,
                "action": action,
                "requiresPreflight": require_preflight,
                "datasourceType": _normalize_text(source_datasource.get("type"), "unknown"),
            }
        )

    summary = {
        "sourceEnvironment": source_env,
        "targetEnvironment": target_env,
        "itemCount": len(plan_items),
        "createCount": len([item for item in plan_items if item["action"] == "create"]),
        "updateCount": len([item for item in plan_items if item["action"] == "update"]),
        "preflightRequiredCount": len(
            [item for item in plan_items if item["requiresPreflight"]]
        ),
    }
    return {
        "kind": PROMOTION_PLAN_KIND,
        "schemaVersion": PROMOTION_PLAN_SCHEMA_VERSION,
        "section": PROMOTION_AND_PREFLIGHT,
        "summary": summary,
        "options": {
            "requirePreflight": require_preflight,
            "dashboardUidMap": dashboard_uid_map,
            "dashboardNameMap": dashboard_name_map,
            "datasourceUidMap": datasource_uid_map,
            "datasourceNameMap": datasource_name_map,
        },
        "planItems": plan_items,
    }


def build_preflight_check_document(
    plan_document: dict[str, Any],
    availability: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build a staged preflight summary from a promotion plan and availability hints."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 203

    if str(plan_document.get("kind") or "").strip() != PROMOTION_PLAN_KIND:
        raise ValueError("Promotion plan document kind is not supported for preflight checks.")
    availability = dict(availability or {})
    available_datasource_uids = set(availability.get("datasourceUids") or [])
    available_plugin_ids = set(availability.get("pluginIds") or [])
    available_contact_points = set(availability.get("contactPoints") or [])
    available_library_panels = set(availability.get("libraryPanels") or [])
    required_plugins = set(availability.get("requiredPluginIds") or [])
    required_contact_points = set(availability.get("requiredContactPoints") or [])
    required_library_panels = set(availability.get("requiredLibraryPanels") or [])

    checks = []
    for item in plan_document.get("planItems") or []:
        if not isinstance(item, dict):
            continue
        if item.get("resourceType") == "datasource":
            datasource_uid = _normalize_text(item.get("targetUid"))
            status = "ok"
            detail = "Target datasource is already mapped or will be created by this plan."
            if item.get("action") != "create" and datasource_uid not in available_datasource_uids:
                status = "missing"
                detail = "Target datasource is not available in the destination inventory."
            checks.append(
                {
                    "kind": "datasource",
                    "resourceType": item.get("resourceType"),
                    "resourceUid": datasource_uid,
                    "status": status,
                    "detail": detail,
                }
            )

    for plugin_id in sorted(required_plugins):
        status = "ok" if plugin_id in available_plugin_ids else "missing"
        checks.append(
            {
                "kind": "plugin",
                "resourceType": "plugin",
                "resourceUid": plugin_id,
                "status": status,
                "detail": (
                    "Plugin is available in the destination environment."
                    if status == "ok"
                    else "Plugin is required by the promotion inputs but missing from the destination environment."
                ),
            }
        )

    for contact_point in sorted(required_contact_points):
        status = "ok" if contact_point in available_contact_points else "missing"
        checks.append(
            {
                "kind": "contact-point",
                "resourceType": "alert-contact-point",
                "resourceUid": contact_point,
                "status": status,
                "detail": (
                    "Contact point is available in the destination environment."
                    if status == "ok"
                    else "Contact point is required by the promotion inputs but missing from the destination environment."
                ),
            }
        )

    for library_panel in sorted(required_library_panels):
        status = "ok" if library_panel in available_library_panels else "missing"
        checks.append(
            {
                "kind": "library-panel",
                "resourceType": "library-panel",
                "resourceUid": library_panel,
                "status": status,
                "detail": (
                    "Library panel is available in the destination environment."
                    if status == "ok"
                    else "Library panel is required by the promotion inputs but missing from the destination environment."
                ),
            }
        )

    return {
        "kind": PREFLIGHT_CHECK_KIND,
        "schemaVersion": PREFLIGHT_CHECK_SCHEMA_VERSION,
        "section": PROMOTION_AND_PREFLIGHT,
        "summary": {
            "checkCount": len(checks),
            "okCount": len([item for item in checks if item["status"] == "ok"]),
            "missingCount": len([item for item in checks if item["status"] == "missing"]),
            "blockingCount": len([item for item in checks if item["status"] != "ok"]),
        },
        "checks": checks,
    }


def render_promotion_plan_text(document: dict[str, Any]) -> List[str]:
    """Render a staged promotion plan as a deterministic text summary."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 203

    if str(document.get("kind") or "").strip() != PROMOTION_PLAN_KIND:
        raise ValueError("Promotion plan document kind is not supported for text rendering.")
    summary = document.get("summary") or {}
    lines = [
        "Promotion plan: %s -> %s"
        % (
            _normalize_text(summary.get("sourceEnvironment"), "source"),
            _normalize_text(summary.get("targetEnvironment"), "target"),
        ),
        "Items: %s total, %s create, %s update, %s preflight-required"
        % (
            int(summary.get("itemCount") or 0),
            int(summary.get("createCount") or 0),
            int(summary.get("updateCount") or 0),
            int(summary.get("preflightRequiredCount") or 0),
        ),
        "",
        "# Plan Items",
    ]
    for item in document.get("planItems") or []:
        if not isinstance(item, dict):
            continue
        lines.append(
            "- %s uid=%s target=%s action=%s"
            % (
                _normalize_text(item.get("resourceType"), "resource"),
                _normalize_text(item.get("sourceUid"), "unknown"),
                _normalize_text(item.get("targetUid"), "unknown"),
                _normalize_text(item.get("action"), "unknown"),
            )
        )
    return lines


def render_preflight_check_text(document: dict[str, Any]) -> List[str]:
    """Render staged preflight results as a deterministic text summary."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 203

    if str(document.get("kind") or "").strip() != PREFLIGHT_CHECK_KIND:
        raise ValueError("Preflight check document kind is not supported for text rendering.")
    summary = document.get("summary") or {}
    lines = [
        "Promotion preflight summary",
        "Checks: %s total, %s ok, %s missing, %s blocking"
        % (
            int(summary.get("checkCount") or 0),
            int(summary.get("okCount") or 0),
            int(summary.get("missingCount") or 0),
            int(summary.get("blockingCount") or 0),
        ),
        "",
        "# Checks",
    ]
    for item in document.get("checks") or []:
        if not isinstance(item, dict):
            continue
        lines.append(
            "- %s uid=%s status=%s detail=%s"
            % (
                _normalize_text(item.get("kind"), "check"),
                _normalize_text(item.get("resourceUid"), "unknown"),
                _normalize_text(item.get("status"), "unknown"),
                _normalize_text(item.get("detail")),
            )
        )
    return lines
