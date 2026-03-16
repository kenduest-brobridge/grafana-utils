"""Unwired bundle-level preflight helpers.

Purpose:
- Stage one combined preflight view across dashboards, datasources, and alerts.
- Reuse existing staged contracts without wiring them into current CLIs.

Caveats:
- This module is import-safe and side-effect free.
- It consumes staged documents and explicit availability hints only.
"""

from typing import Mapping

from .dashboard_cli import GrafanaError
from .alert_sync_workbench import assess_alert_sync_specs
from .datasource_secret_provider_workbench import (
    build_provider_plan,
    summarize_provider_plan,
)
from .datasource_secret_workbench import (
    collect_secret_placeholders,
    iter_secret_placeholder_names,
)
from .roadmap_workbench import build_preflight_check_document, build_promotion_plan_document
from .sync_preflight_workbench import build_sync_preflight_document

BUNDLE_PREFLIGHT_KIND = "grafana-utils-bundle-preflight"
BUNDLE_PREFLIGHT_SCHEMA_VERSION = 1


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


def _build_secret_assessment(datasources, availability):
    available_secret_names = set(
        _require_string_list(
            availability.get("secretPlaceholderNames") or availability.get("secretNames"),
            "secretPlaceholderNames",
        )
    )
    plans = []
    checks = []
    for item in datasources:
        if not isinstance(item, Mapping):
            continue
        placeholders = collect_secret_placeholders(item.get("secureJsonDataPlaceholders"))
        if not placeholders:
            continue
        placeholder_names = list(iter_secret_placeholder_names(placeholders))
        plans.append(
            {
                "datasourceUid": _normalize_text(item.get("uid")) or None,
                "datasourceName": _normalize_text(item.get("name"), "unknown"),
                "datasourceType": _normalize_text(item.get("type"), "unknown"),
                "providerKind": "inline-placeholder-map",
                "action": "inject-secrets",
                "reviewRequired": True,
                "secretFields": sorted(
                    [placeholder.field_name for placeholder in placeholders]
                ),
                "placeholderNames": placeholder_names,
            }
        )
        for placeholder_name in placeholder_names:
            blocking = placeholder_name not in available_secret_names
            checks.append(
                {
                    "kind": "secret-placeholder",
                    "datasourceName": _normalize_text(item.get("name"), "unknown"),
                    "identity": "%s->%s"
                    % (
                        _normalize_text(item.get("uid"))
                        or _normalize_text(item.get("name"), "unknown"),
                        placeholder_name,
                    ),
                    "placeholderName": placeholder_name,
                    "status": "missing" if blocking else "ok",
                    "blocking": blocking,
                }
            )
    return {
        "summary": {
            "datasourceCount": len(plans),
            "referenceCount": len(checks),
            "blockingCount": len([item for item in checks if item.get("blocking")]),
        },
        "plans": plans,
        "checks": checks,
    }


def _build_provider_assessment(datasources, availability):
    available_provider_names = set(
        _require_string_list(
            availability.get("providerNames") or availability.get("secretProviderNames"),
            "providerNames",
        )
    )
    plans = []
    checks = []
    for item in datasources:
        if not isinstance(item, Mapping):
            continue
        if not item.get("secureJsonDataProviders"):
            continue
        plan = build_provider_plan(dict(item))
        plan_summary = summarize_provider_plan(plan)
        plans.append(plan_summary)
        seen_provider_names = []
        for provider in plan_summary.get("providers") or []:
            provider_name = _normalize_text(provider.get("providerName"))
            if not provider_name or provider_name in seen_provider_names:
                continue
            seen_provider_names.append(provider_name)
            blocking = provider_name not in available_provider_names
            checks.append(
                {
                    "kind": "secret-provider",
                    "datasourceName": plan_summary.get("datasourceName"),
                    "identity": "%s->%s"
                    % (
                        plan_summary.get("datasourceUid")
                        or plan_summary.get("datasourceName")
                        or "unknown",
                        provider_name,
                    ),
                    "providerName": provider_name,
                    "status": "missing" if blocking else "ok",
                    "blocking": blocking,
                }
            )
    return {
        "summary": {
            "datasourceCount": len(plans),
            "referenceCount": len(
                [
                    provider
                    for plan in plans
                    for provider in (plan.get("providers") or [])
                ]
            ),
            "blockingCount": len([item for item in checks if item.get("blocking")]),
        },
        "plans": plans,
        "checks": checks,
    }


def build_bundle_preflight_document(source_bundle, target_inventory, availability=None):
    """Build one staged preflight document from a multi-resource bundle shape."""
    source_bundle = _require_mapping(source_bundle, "source bundle")
    target_inventory = _require_mapping(target_inventory, "target inventory")
    availability = _require_mapping(availability, "availability")

    sync_specs = []
    for item in source_bundle.get("dashboards") or []:
        if isinstance(item, Mapping):
            sync_specs.append(
                {
                    "kind": "dashboard",
                    "uid": item.get("uid"),
                    "title": item.get("title"),
                    "body": dict(item),
                }
            )
    for item in source_bundle.get("datasources") or []:
        if isinstance(item, Mapping):
            sync_specs.append(
                {
                    "kind": "datasource",
                    "uid": item.get("uid"),
                    "name": item.get("name"),
                    "title": item.get("name") or item.get("uid"),
                    "body": dict(item),
                }
            )
    for item in source_bundle.get("folders") or []:
        if isinstance(item, Mapping):
            sync_specs.append(dict(item))

    promotion_plan = build_promotion_plan_document(
        source_bundle,
        target_inventory,
        options={"requirePreflight": True},
    )
    promotion_preflight = build_preflight_check_document(
        promotion_plan,
        availability=availability,
    )
    sync_preflight = build_sync_preflight_document(
        sync_specs,
        availability=availability,
    )
    alert_assessment = assess_alert_sync_specs(source_bundle.get("alerts") or [])
    datasources = source_bundle.get("datasources") or []
    provider_assessment = _build_provider_assessment(datasources, availability)
    secret_assessment = _build_secret_assessment(datasources, availability)

    return {
        "kind": BUNDLE_PREFLIGHT_KIND,
        "schemaVersion": BUNDLE_PREFLIGHT_SCHEMA_VERSION,
        "summary": {
            "promotionBlockingCount": int(
                (promotion_preflight.get("summary") or {}).get("blockingCount") or 0
            ),
            "syncBlockingCount": int(
                (sync_preflight.get("summary") or {}).get("blockingCount") or 0
            ),
            "alertBlockedCount": int(
                (alert_assessment.get("summary") or {}).get("blockedCount") or 0
            ),
            "alertPlanOnlyCount": int(
                (alert_assessment.get("summary") or {}).get("planOnlyCount") or 0
            ),
            "providerBlockingCount": int(
                (provider_assessment.get("summary") or {}).get("blockingCount") or 0
            ),
            "secretBlockingCount": int(
                (secret_assessment.get("summary") or {}).get("blockingCount") or 0
            ),
        },
        "promotionPlan": promotion_plan,
        "promotionPreflight": promotion_preflight,
        "syncPreflight": sync_preflight,
        "alertAssessment": alert_assessment,
        "providerAssessment": provider_assessment,
        "secretAssessment": secret_assessment,
    }


def render_bundle_preflight_text(document):
    """Render one deterministic bundle preflight summary."""
    if _normalize_text(document.get("kind")) != BUNDLE_PREFLIGHT_KIND:
        raise GrafanaError("Bundle preflight document kind is not supported.")
    summary = _require_mapping(document.get("summary"), "summary")
    return [
        "Bundle preflight summary",
        "Promotion blocking: %s" % int(summary.get("promotionBlockingCount") or 0),
        "Sync blocking: %s" % int(summary.get("syncBlockingCount") or 0),
        "Alert blocked: %s" % int(summary.get("alertBlockedCount") or 0),
        "Alert plan-only: %s" % int(summary.get("alertPlanOnlyCount") or 0),
        "Provider blocking: %s" % int(summary.get("providerBlockingCount") or 0),
        "Secret blocking: %s" % int(summary.get("secretBlockingCount") or 0),
    ]


__all__ = [
    "BUNDLE_PREFLIGHT_KIND",
    "BUNDLE_PREFLIGHT_SCHEMA_VERSION",
    "build_bundle_preflight_document",
    "render_bundle_preflight_text",
]
