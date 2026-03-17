"""Unwired secret-provider contract helpers for datasource imports.

Purpose:
- Stage an external secret-provider contract without wiring provider IO yet.
- Keep provider references explicit, reviewable, and fail-closed.

Caveats:
- This module does not fetch secrets from any remote system.
- It only validates provider references and shapes a later resolution plan.
"""

from dataclasses import dataclass
from typing import Iterable, Mapping, Optional, Sequence

from .dashboard_cli import GrafanaError

PROVIDER_REFERENCE_PREFIX = "${provider:"
PROVIDER_REFERENCE_SUFFIX = "}"


@dataclass(frozen=True)
class SecretProviderReference(object):
    """One staged provider-backed secret reference."""

    field_name: str
    provider_name: str
    secret_path: str
    raw_token: str


@dataclass(frozen=True)
class DatasourceSecretProviderPlan(object):
    """Reviewable staged plan for later provider-backed resolution."""

    datasource_uid: Optional[str]
    datasource_name: str
    datasource_type: str
    references: Sequence[SecretProviderReference]
    provider_kind: str
    action: str
    review_required: bool


def _normalize_text(value):
    """Internal helper for normalize text."""
    if value is None:
        return ""
    return str(value).strip()


def parse_provider_reference(value, field_name):
    """Parse one ${provider:name:path} token and reject opaque secret replay."""
    field_name = _normalize_text(field_name)
    if not field_name:
        raise GrafanaError("Secret provider field names must be non-empty strings.")
    if not isinstance(value, str):
        raise GrafanaError(
            "Secret provider field '%s' must use a placeholder string." % field_name
        )
    if not value.startswith(PROVIDER_REFERENCE_PREFIX) or not value.endswith(
        PROVIDER_REFERENCE_SUFFIX
    ):
        raise GrafanaError(
            "Secret provider field '%s' must use ${provider:NAME:PATH} references; opaque replay is not allowed."
            % field_name
        )
    inner = value[len(PROVIDER_REFERENCE_PREFIX) : -1]
    parts = inner.split(":", 1)
    if len(parts) != 2:
        raise GrafanaError(
            "Secret provider field '%s' must use ${provider:NAME:PATH} references."
            % field_name
        )
    provider_name = _normalize_text(parts[0])
    secret_path = _normalize_text(parts[1])
    if not provider_name or not secret_path:
        raise GrafanaError(
            "Secret provider field '%s' must include both provider name and secret path."
            % field_name
        )
    return SecretProviderReference(
        field_name=field_name,
        provider_name=provider_name,
        secret_path=secret_path,
        raw_token=value,
    )


def collect_provider_references(secure_json_data):
    """Normalize secureJsonData provider references in stable order."""
    if secure_json_data is None:
        return []
    if not isinstance(secure_json_data, dict):
        raise GrafanaError("Provider-backed secureJsonData input must be a JSON object.")
    references = []
    for field_name in sorted(secure_json_data):
        references.append(parse_provider_reference(secure_json_data[field_name], field_name))
    return references


def build_provider_plan(datasource_spec):
    """Build one staged provider-resolution plan without performing any lookup."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 44, 89

    if not isinstance(datasource_spec, dict):
        raise GrafanaError("Datasource provider plan input must be a mapping.")
    datasource_name = _normalize_text(datasource_spec.get("name"))
    datasource_type = _normalize_text(datasource_spec.get("type"))
    if not datasource_name:
        raise GrafanaError("Datasource provider plan requires a datasource name.")
    if not datasource_type:
        raise GrafanaError("Datasource provider plan requires a datasource type.")
    references = collect_provider_references(
        datasource_spec.get("secureJsonDataProviders")
    )
    return DatasourceSecretProviderPlan(
        datasource_uid=_normalize_text(datasource_spec.get("uid")) or None,
        datasource_name=datasource_name,
        datasource_type=datasource_type,
        references=tuple(references),
        provider_kind="external-provider-reference",
        action="resolve-provider-secrets",
        review_required=True,
    )


def summarize_provider_plan(plan):
    """Return a redacted provider plan summary suitable for review."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return {
        "datasourceUid": plan.datasource_uid,
        "datasourceName": plan.datasource_name,
        "datasourceType": plan.datasource_type,
        "providerKind": plan.provider_kind,
        "action": plan.action,
        "reviewRequired": plan.review_required,
        "providers": [
            {
                "fieldName": item.field_name,
                "providerName": item.provider_name,
                "secretPath": item.secret_path,
            }
            for item in plan.references
        ],
    }


def iter_provider_names(references):
    """Yield unique provider names in first-seen order."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    seen = set()
    for item in references:
        if item.provider_name in seen:
            continue
        seen.add(item.provider_name)
        yield item.provider_name


__all__ = [
    "PROVIDER_REFERENCE_PREFIX",
    "PROVIDER_REFERENCE_SUFFIX",
    "DatasourceSecretProviderPlan",
    "SecretProviderReference",
    "build_provider_plan",
    "collect_provider_references",
    "iter_provider_names",
    "parse_provider_reference",
    "summarize_provider_plan",
]
