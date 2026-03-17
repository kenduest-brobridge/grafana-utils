"""Unwired datasource secret planning helpers.

This module keeps placeholder-based datasource secret handling isolated until a
reviewable CLI or bundle contract is ready. It is intentionally import-safe and
side-effect free.

External secret providers are explicitly out of scope for this scaffold. The
only supported resolution input is an explicit in-memory placeholder map.
"""

from dataclasses import dataclass
from typing import Dict, Iterable, List, Mapping, Optional, Sequence

from .dashboard_cli import GrafanaError


SECRET_PLACEHOLDER_PREFIX = "${secret:"
SECRET_PLACEHOLDER_SUFFIX = "}"


@dataclass(frozen=True)
class SecretPlaceholder(object):
    """One declared placeholder reference for a secure datasource field."""

    field_name: str
    placeholder_name: str
    placeholder_token: str


@dataclass(frozen=True)
class DatasourceSecretPlan(object):
    """Resolved unwired secret plan ready for later reviewable wiring."""

    datasource_uid: Optional[str]
    datasource_name: str
    datasource_type: str
    placeholders: Sequence[SecretPlaceholder]
    resolved_secure_json_data: Mapping[str, str]
    action: str
    review_required: bool
    provider_kind: str


def build_placeholder_token(name):
    """Return the canonical placeholder token for one secret name."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if not isinstance(name, str) or not name.strip():
        raise GrafanaError("Secret placeholder names must be non-empty strings.")
    return "%s%s%s" % (
        SECRET_PLACEHOLDER_PREFIX,
        name.strip(),
        SECRET_PLACEHOLDER_SUFFIX,
    )


def parse_secret_placeholder(value, field_name):
    """Parse one secure field placeholder token and fail closed on raw secrets."""
    if not isinstance(field_name, str) or not field_name:
        raise GrafanaError("Secret field names must be non-empty strings.")
    if not isinstance(value, str):
        raise GrafanaError(
            "Secret field '%s' must use a placeholder string, not %s."
            % (field_name, type(value).__name__)
        )
    if not value.startswith(SECRET_PLACEHOLDER_PREFIX) or not value.endswith(
        SECRET_PLACEHOLDER_SUFFIX
    ):
        raise GrafanaError(
            "Secret field '%s' must use ${secret:...} placeholders; opaque replay is not allowed."
            % field_name
        )
    placeholder_name = value[len(SECRET_PLACEHOLDER_PREFIX) : -1].strip()
    if not placeholder_name:
        raise GrafanaError(
            "Secret field '%s' must not use an empty placeholder name." % field_name
        )
    return SecretPlaceholder(
        field_name=field_name,
        placeholder_name=placeholder_name,
        placeholder_token=value,
    )


def collect_secret_placeholders(secure_json_data):
    """Normalize secureJsonData placeholder declarations in stable key order."""
    if secure_json_data is None:
        return []
    if not isinstance(secure_json_data, dict):
        raise GrafanaError("secureJsonData placeholder input must be a JSON object.")
    placeholders = []
    for field_name in sorted(secure_json_data):
        placeholders.append(
            parse_secret_placeholder(secure_json_data[field_name], field_name)
        )
    return placeholders


def resolve_secret_placeholders(placeholders, provided_secrets):
    """Resolve placeholder declarations from an explicit in-memory secret map."""
    if not isinstance(provided_secrets, dict):
        raise GrafanaError("Provided datasource secrets must be a mapping.")
    resolved = {}
    for placeholder in placeholders:
        if placeholder.placeholder_name not in provided_secrets:
            raise GrafanaError(
                "Missing datasource secret placeholder '%s'."
                % placeholder.placeholder_name
            )
        secret_value = provided_secrets[placeholder.placeholder_name]
        if not isinstance(secret_value, str) or not secret_value:
            raise GrafanaError(
                "Resolved datasource secret '%s' must be a non-empty string."
                % placeholder.placeholder_name
            )
        resolved[placeholder.field_name] = secret_value
    return resolved


def iter_secret_placeholder_names(placeholders):
    """Yield unique placeholder names in first-seen order."""
    seen = set()
    for placeholder in placeholders:
        if placeholder.placeholder_name in seen:
            continue
        seen.add(placeholder.placeholder_name)
        yield placeholder.placeholder_name


def build_datasource_secret_plan(datasource_spec, provided_secrets):
    """Build one resolved datasource secret plan from placeholder input."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 83, 97

    if not isinstance(datasource_spec, dict):
        raise GrafanaError("Datasource secret plan input must be a mapping.")
    datasource_name = datasource_spec.get("name")
    datasource_type = datasource_spec.get("type")
    if not isinstance(datasource_name, str) or not datasource_name.strip():
        raise GrafanaError("Datasource secret plan requires a datasource name.")
    if not isinstance(datasource_type, str) or not datasource_type.strip():
        raise GrafanaError("Datasource secret plan requires a datasource type.")
    placeholders = collect_secret_placeholders(
        datasource_spec.get("secureJsonDataPlaceholders")
    )
    resolved_secure_json_data = resolve_secret_placeholders(
        placeholders,
        provided_secrets,
    )
    return DatasourceSecretPlan(
        datasource_uid=datasource_spec.get("uid"),
        datasource_name=datasource_name,
        datasource_type=datasource_type,
        placeholders=tuple(placeholders),
        resolved_secure_json_data=resolved_secure_json_data,
        action="inject-secrets",
        review_required=True,
        provider_kind="inline-placeholder-map",
    )


def summarize_secret_plan(plan):
    """Return a small review-friendly summary without exposing secret values."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 118

    return {
        "datasourceUid": plan.datasource_uid,
        "datasourceName": plan.datasource_name,
        "datasourceType": plan.datasource_type,
        "action": plan.action,
        "reviewRequired": plan.review_required,
        "providerKind": plan.provider_kind,
        "secretFields": sorted(plan.resolved_secure_json_data),
        "placeholderNames": list(iter_secret_placeholder_names(plan.placeholders)),
    }
