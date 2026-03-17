"""Shared datasource import/export normalization helpers.

Purpose:
- Define the source-of-truth schema checks and normalization used by datasource
  export/import/diff workflows.

Data contract notes:
- Normalize values at this boundary so importer/exporter code can rely on stable
  string fields for comparisons and diff output.
- Keep the contract strict: only expected keys are allowed so unknown fields are
  rejected before export or diff logic runs.

Caveats:
- This contract is intentionally strict; adding fields requires explicit schema
  updates plus corresponding test coverage.
"""

from typing import Any

DATASOURCE_CONTRACT_FIELDS = (
    "uid",
    "name",
    "type",
    "access",
    "url",
    "isDefault",
    "org",
    "orgId",
)


def normalize_datasource_string(value: Any) -> str:
    """Return a stable string representation for contract normalization."""
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value).strip()


def normalize_datasource_bool(value: Any) -> bool:
    """Interpret common datasource bool string formats before normalization."""
    normalized = normalize_datasource_string(value).lower()
    return normalized in ("true", "1", "yes")


def normalize_datasource_record(record: dict[str, Any]) -> dict[str, str]:
    """Normalize a datasource record into the strict contract field set."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 32, 41

    return {
        "uid": normalize_datasource_string(record.get("uid")),
        "name": normalize_datasource_string(record.get("name")),
        "type": normalize_datasource_string(record.get("type")),
        "access": normalize_datasource_string(record.get("access")),
        "url": normalize_datasource_string(record.get("url")),
        "isDefault": (
            "true" if normalize_datasource_bool(record.get("isDefault")) else "false"
        ),
        "org": normalize_datasource_string(record.get("org")),
        "orgId": normalize_datasource_string(record.get("orgId")),
    }


def validate_datasource_contract_record(
    record: dict[str, Any],
    context_label: str,
) -> None:
    """Reject unsupported datasource fields before import/diff workflows."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    extra_fields = sorted(
        key for key in record.keys() if key not in DATASOURCE_CONTRACT_FIELDS
    )
    if extra_fields:
        raise ValueError(
            "%s contains unsupported datasource field(s): %s. Supported fields: %s."
            % (
                context_label,
                ", ".join(extra_fields),
                ", ".join(DATASOURCE_CONTRACT_FIELDS),
            )
        )
