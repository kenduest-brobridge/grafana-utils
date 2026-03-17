"""Unwired staged alert sync helpers.

Purpose:
- Stage alert-specific sync ownership and mutation policy before live wiring.
- Keep partial alert ownership explicit and reviewable.

Caveats:
- This module is import-safe and side-effect free.
- It does not call Grafana APIs or mutate alert documents directly.
"""

from dataclasses import dataclass
from typing import Mapping, Sequence

from .dashboard_cli import GrafanaError

ALERT_SYNC_KIND = "grafana-utils-alert-sync-plan"
ALERT_SYNC_SCHEMA_VERSION = 1
ALERT_ALLOWED_MANAGED_FIELDS = (
    "condition",
    "labels",
    "annotations",
    "contactPoints",
    "for",
    "noDataState",
    "execErrState",
)


@dataclass(frozen=True)
class AlertSyncAssessment(object):
    """One staged assessment row for an alert sync resource."""

    identity: str
    title: str
    managed_fields: Sequence[str]
    status: str
    live_apply_allowed: bool
    detail: str


def _normalize_text(value, default=""):
    """Internal helper for normalize text."""
    if value is None:
        return default
    text = str(value).strip()
    if text:
        return text
    return default


def _require_mapping(value, label):
    """Internal helper for require mapping."""
    if not isinstance(value, Mapping):
        raise GrafanaError("%s must be a JSON object." % label)
    return dict(value)


def _normalize_managed_fields(values):
    """Internal helper for normalize managed fields."""
    if not isinstance(values, (list, tuple)):
        raise GrafanaError("Alert managedFields must be a list.")
    fields = []
    for value in values:
        item = _normalize_text(value)
        if not item:
            raise GrafanaError("Alert managedFields cannot contain empty values.")
        if item not in ALERT_ALLOWED_MANAGED_FIELDS:
            raise GrafanaError(
                "Unsupported alert managed field %r. Allowed values: %s."
                % (item, ", ".join(ALERT_ALLOWED_MANAGED_FIELDS))
            )
        fields.append(item)
    return tuple(fields)


def assess_alert_sync_specs(alert_specs):
    """Assess alert sync specs for staged plan-only vs future live apply support."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 42, 52, 59

    assessments = []
    for raw_spec in alert_specs:
        spec = _require_mapping(raw_spec, "Alert sync spec")
        if _normalize_text(spec.get("kind")).lower() != "alert":
            raise GrafanaError("Alert sync assessment only supports kind=alert.")
        identity = _normalize_text(
            spec.get("uid") or spec.get("name") or spec.get("title")
        )
        if not identity:
            raise GrafanaError("Alert sync spec requires uid, name, or title.")
        title = _normalize_text(spec.get("title") or spec.get("name"), identity)
        managed_fields = _normalize_managed_fields(spec.get("managedFields") or [])
        body = _require_mapping(
            spec.get("body") or spec.get("spec") or {}, "Alert body"
        )

        if "condition" not in managed_fields:
            assessments.append(
                AlertSyncAssessment(
                    identity=identity,
                    title=title,
                    managed_fields=managed_fields,
                    status="blocked",
                    live_apply_allowed=False,
                    detail="Alert sync must manage condition explicitly before live apply can be considered.",
                )
            )
            continue

        if any(field in managed_fields for field in ("contactPoints", "annotations")):
            assessments.append(
                AlertSyncAssessment(
                    identity=identity,
                    title=title,
                    managed_fields=managed_fields,
                    status="plan-only",
                    live_apply_allowed=False,
                    detail="Alert sync includes linked routing or annotation fields and stays plan-only until mutation semantics settle.",
                )
            )
            continue

        if not _normalize_text(body.get("condition")):
            assessments.append(
                AlertSyncAssessment(
                    identity=identity,
                    title=title,
                    managed_fields=managed_fields,
                    status="blocked",
                    live_apply_allowed=False,
                    detail="Alert sync body must include a non-empty condition.",
                )
            )
            continue

        assessments.append(
            AlertSyncAssessment(
                identity=identity,
                title=title,
                managed_fields=managed_fields,
                status="candidate",
                live_apply_allowed=True,
                detail="Alert sync scope is narrow enough for future controlled live-apply experiments.",
            )
        )
    return {
        "kind": ALERT_SYNC_KIND,
        "schemaVersion": ALERT_SYNC_SCHEMA_VERSION,
        "summary": {
            "alertCount": len(assessments),
            "candidateCount": len(
                [item for item in assessments if item.status == "candidate"]
            ),
            "planOnlyCount": len(
                [item for item in assessments if item.status == "plan-only"]
            ),
            "blockedCount": len(
                [item for item in assessments if item.status == "blocked"]
            ),
        },
        "alerts": [
            {
                "identity": item.identity,
                "title": item.title,
                "managedFields": list(item.managed_fields),
                "status": item.status,
                "liveApplyAllowed": item.live_apply_allowed,
                "detail": item.detail,
            }
            for item in assessments
        ],
    }


def render_alert_sync_assessment_text(document):
    """Render one deterministic text summary for later wiring."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 42, 52

    if _normalize_text(document.get("kind")) != ALERT_SYNC_KIND:
        raise GrafanaError("Alert sync assessment document kind is not supported.")
    summary = _require_mapping(document.get("summary"), "summary")
    lines = [
        "Alert sync assessment",
        "Alerts: %s total, %s candidate, %s plan-only, %s blocked"
        % (
            int(summary.get("alertCount") or 0),
            int(summary.get("candidateCount") or 0),
            int(summary.get("planOnlyCount") or 0),
            int(summary.get("blockedCount") or 0),
        ),
        "",
        "# Alerts",
    ]
    for item in document.get("alerts") or []:
        if not isinstance(item, Mapping):
            continue
        lines.append(
            "- %s status=%s liveApplyAllowed=%s detail=%s"
            % (
                _normalize_text(item.get("identity"), "unknown"),
                _normalize_text(item.get("status"), "unknown"),
                "true" if bool(item.get("liveApplyAllowed")) else "false",
                _normalize_text(item.get("detail")),
            )
        )
    return lines


__all__ = [
    "ALERT_ALLOWED_MANAGED_FIELDS",
    "ALERT_SYNC_KIND",
    "ALERT_SYNC_SCHEMA_VERSION",
    "AlertSyncAssessment",
    "assess_alert_sync_specs",
    "render_alert_sync_assessment_text",
]
