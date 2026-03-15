"""Unwired sync preflight helpers for later Grafana-aware validation.

Purpose:
- Keep richer declarative sync preflight checks isolated from the current CLI.
- Model alert policy and dependency checks without forcing live apply support.

Caveats:
- This module is intentionally import-safe and side-effect free.
- It does not talk to Grafana directly; callers pass desired state and
  availability hints explicitly.
"""

from dataclasses import dataclass
from typing import Mapping

from .dashboard_cli import GrafanaError
from .gitops_sync import normalize_resource_spec

SYNC_PREFLIGHT_KIND = "grafana-utils-sync-preflight"
SYNC_PREFLIGHT_SCHEMA_VERSION = 1


@dataclass(frozen=True)
class SyncPreflightCheck:
    """One staged preflight result for a declarative sync resource."""

    kind: str
    identity: str
    status: str
    detail: str
    blocking: bool


def _normalize_text(value, default=""):
    if value is None:
        return default
    text = str(value).strip()
    if text:
        return text
    return default


def _require_mapping(value, label):
    if value is None:
        return {}
    if not isinstance(value, Mapping):
        raise GrafanaError("%s must be a JSON object." % label)
    return dict(value)


def _require_string_list(values, label):
    if values is None:
        return []
    if not isinstance(values, (list, tuple, set)):
        raise GrafanaError("%s must be a list." % label)
    normalized = []
    for value in values:
        item = _normalize_text(value)
        if item:
            normalized.append(item)
    return normalized


def _build_datasource_checks(spec, availability):
    checks = []
    available_uids = set(
        _require_string_list(availability.get("datasourceUids"), "datasourceUids")
    )
    required_plugins = set(
        _require_string_list(availability.get("pluginIds"), "pluginIds")
    )
    datasource_type = _normalize_text(spec.body.get("type"), "unknown")

    if spec.identity in available_uids:
        checks.append(
            SyncPreflightCheck(
                kind="datasource",
                identity=spec.identity,
                status="ok",
                detail="Datasource already exists in the destination inventory.",
                blocking=False,
            )
        )
    else:
        checks.append(
            SyncPreflightCheck(
                kind="datasource",
                identity=spec.identity,
                status="create-planned",
                detail="Datasource is absent and would be created by sync.",
                blocking=False,
            )
        )

    if datasource_type and datasource_type not in required_plugins:
        checks.append(
            SyncPreflightCheck(
                kind="plugin",
                identity=datasource_type,
                status="missing",
                detail="Datasource plugin type is not listed in destination plugin availability.",
                blocking=True,
            )
        )
    else:
        checks.append(
            SyncPreflightCheck(
                kind="plugin",
                identity=datasource_type or "unknown",
                status="ok",
                detail="Datasource plugin type is available.",
                blocking=False,
            )
        )
    return checks


def _build_dashboard_checks(spec, availability):
    checks = []
    body = _require_mapping(spec.body, "dashboard body")
    datasource_uids = _require_string_list(
        body.get("datasourceUids"), "dashboard datasourceUids"
    )
    available_uids = set(
        _require_string_list(availability.get("datasourceUids"), "datasourceUids")
    )
    for datasource_uid in datasource_uids:
        status = "ok" if datasource_uid in available_uids else "missing"
        checks.append(
            SyncPreflightCheck(
                kind="dashboard-datasource",
                identity="%s->%s" % (spec.identity, datasource_uid),
                status=status,
                detail=(
                    "Referenced datasource is available for dashboard sync."
                    if status == "ok"
                    else "Referenced datasource is missing for dashboard sync."
                ),
                blocking=status != "ok",
            )
        )
    return checks


def _build_alert_checks(spec, availability):
    checks = [
        SyncPreflightCheck(
            kind="alert-live-apply",
            identity=spec.identity,
            status="blocked",
            detail="Alert sync stays plan-only until partial ownership and live-apply semantics are explicitly wired.",
            blocking=True,
        )
    ]
    body = _require_mapping(spec.body, "alert body")
    contact_points = _require_string_list(
        body.get("contactPoints"), "alert contactPoints"
    )
    available_contact_points = set(
        _require_string_list(availability.get("contactPoints"), "contactPoints")
    )
    for contact_point in contact_points:
        status = "ok" if contact_point in available_contact_points else "missing"
        checks.append(
            SyncPreflightCheck(
                kind="alert-contact-point",
                identity="%s->%s" % (spec.identity, contact_point),
                status=status,
                detail=(
                    "Alert contact point is available."
                    if status == "ok"
                    else "Alert contact point is missing."
                ),
                blocking=status != "ok",
            )
        )
    return checks


def build_sync_preflight_document(desired_specs, availability=None):
    """Build a staged sync preflight document from desired state and hints."""
    availability = _require_mapping(availability, "availability")
    checks = []
    for raw_spec in desired_specs:
        spec = normalize_resource_spec(raw_spec)
        if spec.kind == "datasource":
            checks.extend(_build_datasource_checks(spec, availability))
        elif spec.kind == "dashboard":
            checks.extend(_build_dashboard_checks(spec, availability))
        elif spec.kind == "alert":
            checks.extend(_build_alert_checks(spec, availability))
        elif spec.kind == "folder":
            checks.append(
                SyncPreflightCheck(
                    kind="folder",
                    identity=spec.identity,
                    status="ok",
                    detail="Folder sync does not require extra staged preflight checks.",
                    blocking=False,
                )
            )
        else:
            raise GrafanaError("Unsupported sync preflight kind %s." % spec.kind)
    return {
        "kind": SYNC_PREFLIGHT_KIND,
        "schemaVersion": SYNC_PREFLIGHT_SCHEMA_VERSION,
        "summary": {
            "checkCount": len(checks),
            "okCount": len([item for item in checks if item.status == "ok"]),
            "blockingCount": len([item for item in checks if item.blocking]),
        },
        "checks": [
            {
                "kind": item.kind,
                "identity": item.identity,
                "status": item.status,
                "detail": item.detail,
                "blocking": item.blocking,
            }
            for item in checks
        ],
    }


def render_sync_preflight_text(document):
    """Render one deterministic text summary for later CLI wiring."""
    if _normalize_text(document.get("kind")) != SYNC_PREFLIGHT_KIND:
        raise GrafanaError("Sync preflight document kind is not supported.")
    summary = _require_mapping(document.get("summary"), "summary")
    lines = [
        "Sync preflight summary",
        "Checks: %s total, %s ok, %s blocking"
        % (
            int(summary.get("checkCount") or 0),
            int(summary.get("okCount") or 0),
            int(summary.get("blockingCount") or 0),
        ),
        "",
        "# Checks",
    ]
    for item in document.get("checks") or []:
        if not isinstance(item, Mapping):
            continue
        lines.append(
            "- %s identity=%s status=%s detail=%s"
            % (
                _normalize_text(item.get("kind"), "check"),
                _normalize_text(item.get("identity"), "unknown"),
                _normalize_text(item.get("status"), "unknown"),
                _normalize_text(item.get("detail")),
            )
        )
    return lines


__all__ = [
    "SYNC_PREFLIGHT_KIND",
    "SYNC_PREFLIGHT_SCHEMA_VERSION",
    "SyncPreflightCheck",
    "build_sync_preflight_document",
    "render_sync_preflight_text",
]
