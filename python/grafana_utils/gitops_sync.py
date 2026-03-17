"""Unwired declarative Grafana sync planning helpers.

Purpose:
- Define one reviewable in-memory sync-plan contract before any CLI or live API
  wiring lands.
- Keep Git-managed desired state separate from apply-time execution so dry-run
  and human review stay first-class.

Caveats:
- This module is intentionally side-effect free and CLI-unwired.
- External secret providers are out of scope here. Datasource specs should
  carry only explicit placeholders or plain declarative values for now.
- Alert support is intentionally partial: callers must declare the managed
  fields they intend to own.
"""

from copy import deepcopy
from dataclasses import dataclass
from typing import Any, Dict, Mapping, Optional, Tuple

from .alert_sync_workbench import assess_alert_sync_specs
from .dashboard_cli import GrafanaError

RESOURCE_KINDS = ("dashboard", "datasource", "folder", "alert")
DEFAULT_REVIEW_TOKEN = "reviewed-sync-plan"
SYNC_SOURCE_BUNDLE_KIND = "grafana-utils-sync-source-bundle"
SYNC_SOURCE_BUNDLE_SCHEMA_VERSION = 1


@dataclass(frozen=True)
class SyncResourceSpec:
    """Normalized declarative resource owned by the future sync workflow."""

    kind: str
    identity: str
    title: str
    body: Dict[str, Any]
    managed_fields: Tuple[str, ...]
    source_path: str


@dataclass(frozen=True)
class SyncOperation:
    """One reviewable dry-run/apply candidate derived from desired state."""

    kind: str
    identity: str
    title: str
    action: str
    reason: str
    changed_fields: Tuple[str, ...]
    managed_fields: Tuple[str, ...]
    desired: Optional[Dict[str, Any]]
    live: Optional[Dict[str, Any]]
    source_path: str


@dataclass(frozen=True)
class SyncPlan:
    """Pure data container for a future dry-run/review/apply flow."""

    dry_run: bool
    review_required: bool
    reviewed: bool
    allow_prune: bool
    summary: Dict[str, int]
    operations: Tuple[SyncOperation, ...]


def _normalize_string(value):
    """Internal helper for normalize string."""
    if value is None:
        return ""
    return str(value).strip()


def _copy_mapping(value, label):
    """Internal helper for copy mapping."""
    if value is None:
        return {}
    if not isinstance(value, Mapping):
        raise GrafanaError("%s must be a JSON object." % label)
    return deepcopy(dict(value))


def _normalize_string_list(values, label):
    """Internal helper for normalize string list."""
    if values is None:
        return ()
    if not isinstance(values, (list, tuple)):
        raise GrafanaError("%s must be a list." % label)
    normalized = []
    for value in values:
        item = _normalize_string(value)
        if not item:
            raise GrafanaError("%s cannot contain empty values." % label)
        normalized.append(item)
    return tuple(normalized)


def _extract_identity(spec):
    """Internal helper for extract identity."""
    for field in ("uid", "name", "title", "path"):
        value = _normalize_string(spec.get(field))
        if value:
            return value
    return ""


def _extract_title(spec, fallback_identity):
    """Internal helper for extract title."""
    for field in ("title", "name", "uid", "path"):
        value = _normalize_string(spec.get(field))
        if value:
            return value
    return fallback_identity


def _normalize_body(spec):
    """Internal helper for normalize body."""
    body = _copy_mapping(spec.get("body"), "body")
    if not body:
        body = _copy_mapping(spec.get("spec"), "spec")
    return body


def normalize_resource_spec(spec):
    """Normalize one declarative resource specification."""
    # Call graph: see callers/callees.
    #   Upstream callers: 200, 336
    #   Downstream callees: 101, 110, 119, 70, 86

    if not isinstance(spec, Mapping):
        raise GrafanaError("Sync resource spec must be a JSON object.")

    kind = _normalize_string(spec.get("kind")).lower()
    if kind not in RESOURCE_KINDS:
        raise GrafanaError(
            "Unsupported sync resource kind %r. Expected one of %s."
            % (kind, ", ".join(RESOURCE_KINDS))
        )

    identity = _extract_identity(spec)
    if not identity:
        raise GrafanaError("Sync resource spec requires uid, name, title, or path.")

    source_path = _normalize_string(spec.get("sourcePath"))
    body = _normalize_body(spec)
    managed_fields = _normalize_string_list(spec.get("managedFields"), "managedFields")

    if kind == "alert" and not managed_fields:
        raise GrafanaError(
            "Alert sync specs must declare managedFields to keep partial ownership explicit."
        )

    title = _extract_title(spec, identity)
    return SyncResourceSpec(
        kind=kind,
        identity=identity,
        title=title,
        body=body,
        managed_fields=managed_fields,
        source_path=source_path,
    )


def build_resource_index(specs):
    """Index normalized resources by stable kind/identity pairs."""
    index = {}
    for spec in specs:
        key = (spec.kind, spec.identity)
        if key in index:
            raise GrafanaError(
                "Duplicate sync identity detected for %s %s."
                % (spec.kind, spec.identity)
            )
        index[key] = spec
    return index


def _body_subset_for_comparison(spec, live_body):
    """Internal helper for body subset for comparison."""
    live_copy = _copy_mapping(live_body, "live body")
    if not spec.managed_fields:
        return live_copy
    subset = {}
    for field in spec.managed_fields:
        if field in live_copy:
            subset[field] = deepcopy(live_copy[field])
    return subset


def _compare_body(spec, live_body):
    """Internal helper for compare body."""
    desired_body = deepcopy(spec.body)
    comparable_live_body = _body_subset_for_comparison(spec, live_body)
    field_names = sorted(set(desired_body.keys()) | set(comparable_live_body.keys()))
    changed = []
    for field in field_names:
        if desired_body.get(field) != comparable_live_body.get(field):
            changed.append(field)
    return tuple(changed)


def _normalize_live_specs(live_specs):
    """Internal helper for normalize live specs."""
    normalized = []
    for spec in live_specs:
        normalized.append(normalize_resource_spec(spec))
    return normalized


def _build_operation(spec, live_spec=None):
    """Internal helper for build operation."""
    if live_spec is None:
        return SyncOperation(
            kind=spec.kind,
            identity=spec.identity,
            title=spec.title,
            action="would-create",
            reason="missing-live",
            changed_fields=tuple(sorted(spec.body.keys())),
            managed_fields=tuple(spec.managed_fields),
            desired=deepcopy(spec.body),
            live=None,
            source_path=spec.source_path,
        )

    changed_fields = _compare_body(spec, live_spec.body)
    if changed_fields:
        return SyncOperation(
            kind=spec.kind,
            identity=spec.identity,
            title=spec.title,
            action="would-update",
            reason="drift-detected",
            changed_fields=changed_fields,
            managed_fields=tuple(spec.managed_fields),
            desired=deepcopy(spec.body),
            live=deepcopy(live_spec.body),
            source_path=spec.source_path,
        )
    return SyncOperation(
        kind=spec.kind,
        identity=spec.identity,
        title=spec.title,
        action="noop",
        reason="in-sync",
        changed_fields=(),
        managed_fields=tuple(spec.managed_fields),
        desired=deepcopy(spec.body),
        live=deepcopy(live_spec.body),
        source_path=spec.source_path,
    )


def _build_prune_operation(spec, allow_prune):
    """Internal helper for build prune operation."""
    if allow_prune:
        return SyncOperation(
            kind=spec.kind,
            identity=spec.identity,
            title=spec.title,
            action="would-delete",
            reason="missing-from-desired-state",
            changed_fields=(),
            managed_fields=tuple(spec.managed_fields),
            desired=None,
            live=deepcopy(spec.body),
            source_path=spec.source_path,
        )
    return SyncOperation(
        kind=spec.kind,
        identity=spec.identity,
        title=spec.title,
        action="unmanaged",
        reason="prune-disabled",
        changed_fields=(),
        managed_fields=tuple(spec.managed_fields),
        desired=None,
        live=deepcopy(spec.body),
        source_path=spec.source_path,
    )


def summarize_alert_operations(operations):
    """Return staged alert assessment derived from desired alert operations."""
    alert_specs = []
    for operation in operations:
        if operation.kind != "alert" or operation.desired is None:
            continue
        alert_specs.append(
            {
                "kind": "alert",
                "uid": operation.identity,
                "title": operation.title,
                "managedFields": list(operation.managed_fields),
                "body": deepcopy(operation.desired),
            }
        )
    if not alert_specs:
        return {
            "summary": {
                "alertCount": 0,
                "candidateCount": 0,
                "planOnlyCount": 0,
                "blockedCount": 0,
            },
            "alerts": [],
        }
    document = assess_alert_sync_specs(alert_specs)
    return {
        "summary": deepcopy(document.get("summary") or {}),
        "alerts": deepcopy(document.get("alerts") or []),
    }


def summarize_operations(operations):
    """Return stable summary counts for review output and future renderers."""
    summary = {
        "would_create": 0,
        "would_update": 0,
        "would_delete": 0,
        "noop": 0,
        "unmanaged": 0,
    }
    for operation in operations:
        if operation.action == "would-create":
            summary["would_create"] += 1
        elif operation.action == "would-update":
            summary["would_update"] += 1
        elif operation.action == "would-delete":
            summary["would_delete"] += 1
        elif operation.action == "noop":
            summary["noop"] += 1
        elif operation.action == "unmanaged":
            summary["unmanaged"] += 1
    return summary


def build_sync_plan(
    desired_specs,
    live_specs,
    allow_prune=False,
    dry_run=True,
    review_required=True,
):
    """Build a dry-run-first declarative sync plan without mutating Grafana."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 127, 163, 200, 208, 252, 313

    desired_index = build_resource_index(
        [normalize_resource_spec(spec) for spec in desired_specs]
    )
    live_index = build_resource_index(_normalize_live_specs(live_specs))

    operations = []

    for key in sorted(desired_index.keys()):
        desired_spec = desired_index[key]
        operations.append(_build_operation(desired_spec, live_index.get(key)))

    for key in sorted(live_index.keys()):
        if key in desired_index:
            continue
        operations.append(_build_prune_operation(live_index[key], allow_prune))

    return SyncPlan(
        dry_run=bool(dry_run),
        review_required=bool(review_required),
        reviewed=not review_required,
        allow_prune=bool(allow_prune),
        summary=summarize_operations(operations),
        operations=tuple(operations),
    )


def mark_plan_reviewed(plan, review_token=DEFAULT_REVIEW_TOKEN):
    """Return a reviewed plan only when the caller presents the expected token."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 70

    normalized_token = _normalize_string(review_token)
    if normalized_token != DEFAULT_REVIEW_TOKEN:
        raise GrafanaError("Sync plan review token rejected.")
    return SyncPlan(
        dry_run=plan.dry_run,
        review_required=plan.review_required,
        reviewed=True,
        allow_prune=plan.allow_prune,
        summary=deepcopy(plan.summary),
        operations=tuple(plan.operations),
    )


def build_apply_intent(plan, approve=False):
    """Gate non-dry-run execution behind explicit review and approval."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if plan.dry_run:
        return {
            "mode": "dry-run",
            "reviewed": plan.reviewed,
            "operations": list(plan.operations),
        }
    if plan.review_required and not plan.reviewed:
        raise GrafanaError(
            "Refusing live sync intent before the reviewable plan is marked reviewed."
        )
    if not approve:
        raise GrafanaError("Refusing live sync intent without explicit approval.")
    return {
        "mode": "apply",
        "reviewed": plan.reviewed,
        "operations": [
            operation
            for operation in plan.operations
            if operation.action in ("would-create", "would-update", "would-delete")
        ],
    }


def plan_to_document(plan):
    """Render one JSON-safe document for future CLI/table/json wiring."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 281

    alert_assessment = summarize_alert_operations(plan.operations)
    summary = deepcopy(plan.summary)
    summary["alert_candidate"] = int(
        (alert_assessment.get("summary") or {}).get("candidateCount") or 0
    )
    summary["alert_plan_only"] = int(
        (alert_assessment.get("summary") or {}).get("planOnlyCount") or 0
    )
    summary["alert_blocked"] = int(
        (alert_assessment.get("summary") or {}).get("blockedCount") or 0
    )
    return {
        "dryRun": plan.dry_run,
        "reviewRequired": plan.review_required,
        "reviewed": plan.reviewed,
        "allowPrune": plan.allow_prune,
        "summary": summary,
        "alertAssessment": alert_assessment,
        "operations": [
            {
                "kind": operation.kind,
                "identity": operation.identity,
                "title": operation.title,
                "action": operation.action,
                "reason": operation.reason,
                "changedFields": list(operation.changed_fields),
                "managedFields": list(operation.managed_fields),
                "desired": deepcopy(operation.desired),
                "live": deepcopy(operation.live),
                "sourcePath": operation.source_path,
            }
            for operation in plan.operations
        ],
    }


def build_sync_source_bundle_document(
    dashboards=None,
    datasources=None,
    folders=None,
    alerting=None,
    metadata=None,
):
    """Build one portable local source bundle document for later review/preflight."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 77

    dashboards = list(dashboards or ())
    datasources = list(datasources or ())
    folders = list(folders or ())
    alerting = _copy_mapping(alerting, "alerting")
    metadata = _copy_mapping(metadata, "metadata")
    alerting_summary = _copy_mapping(alerting.get("summary"), "alerting.summary")
    return {
        "kind": SYNC_SOURCE_BUNDLE_KIND,
        "schemaVersion": SYNC_SOURCE_BUNDLE_SCHEMA_VERSION,
        "summary": {
            "dashboardCount": len(dashboards),
            "datasourceCount": len(datasources),
            "folderCount": len(folders),
            "alertRuleCount": int(alerting_summary.get("ruleCount") or 0),
            "contactPointCount": int(alerting_summary.get("contactPointCount") or 0),
            "muteTimingCount": int(alerting_summary.get("muteTimingCount") or 0),
            "policyCount": int(alerting_summary.get("policyCount") or 0),
            "templateCount": int(alerting_summary.get("templateCount") or 0),
        },
        "dashboards": deepcopy(dashboards),
        "datasources": deepcopy(datasources),
        "folders": deepcopy(folders),
        "alerts": [],
        "alerting": deepcopy(alerting),
        "metadata": deepcopy(metadata),
    }


def render_sync_source_bundle_text(document):
    """Render one portable source bundle document as concise operator text."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 77

    if not isinstance(document, Mapping):
        raise GrafanaError("Sync source bundle document must be a JSON object.")
    if document.get("kind") != SYNC_SOURCE_BUNDLE_KIND:
        raise GrafanaError("Sync source bundle document kind is not supported.")
    summary = _copy_mapping(document.get("summary"), "summary")
    return [
        "Sync source bundle",
        "Dashboards: %s" % int(summary.get("dashboardCount") or 0),
        "Datasources: %s" % int(summary.get("datasourceCount") or 0),
        "Folders: %s" % int(summary.get("folderCount") or 0),
        "Alerting: rules=%s contact-points=%s mute-timings=%s policies=%s templates=%s"
        % (
            int(summary.get("alertRuleCount") or 0),
            int(summary.get("contactPointCount") or 0),
            int(summary.get("muteTimingCount") or 0),
            int(summary.get("policyCount") or 0),
            int(summary.get("templateCount") or 0),
        ),
    ]


__all__ = [
    "DEFAULT_REVIEW_TOKEN",
    "RESOURCE_KINDS",
    "SYNC_SOURCE_BUNDLE_KIND",
    "SYNC_SOURCE_BUNDLE_SCHEMA_VERSION",
    "SyncOperation",
    "SyncPlan",
    "SyncResourceSpec",
    "build_apply_intent",
    "build_resource_index",
    "build_sync_source_bundle_document",
    "build_sync_plan",
    "mark_plan_reviewed",
    "normalize_resource_spec",
    "plan_to_document",
    "render_sync_source_bundle_text",
    "summarize_alert_operations",
    "summarize_operations",
]
