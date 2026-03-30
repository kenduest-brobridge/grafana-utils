"""Render helpers for unwired datasource live add/delete dry-run output."""

import json

ADD_DELETE_DRY_RUN_COLUMN_HEADERS = {
    "operation": "OPERATION",
    "uid": "UID",
    "name": "NAME",
    "type": "TYPE",
    "match": "MATCH",
    "action": "ACTION",
    "targetId": "TARGET_ID",
    "secretSummary": "SECRET",
}


def _render_rows(records, columns):
    """Internal helper for render rows."""
    headers = [ADD_DELETE_DRY_RUN_COLUMN_HEADERS[column] for column in columns]
    rows = [[str(item.get(column) or "") for column in columns] for item in records]
    widths = [len(value) for value in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def render_row(values):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(
            values[index].ljust(widths[index]) for index in range(len(values))
        )

    lines = [render_row(headers), render_row(["-" * width for width in widths])]
    for row in rows:
        lines.append(render_row(row))
    return lines


def build_live_mutation_dry_run_record(operation, plan, spec=None, uid=None, name=None):
    """Build live mutation dry run record implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    spec = dict(spec or {})
    target = plan.get("target") or {}
    secure_json_data = spec.get("secureJsonData")
    secret_fields = []
    if isinstance(secure_json_data, dict):
        secret_fields = sorted(
            str(field).strip() for field in secure_json_data if str(field).strip()
        )
    return {
        "operation": str(operation or "").strip(),
        "uid": str(spec.get("uid") or uid or target.get("uid") or ""),
        "name": str(spec.get("name") or name or target.get("name") or ""),
        "type": str(spec.get("type") or target.get("type") or ""),
        "match": str(plan.get("match") or ""),
        "action": str(plan.get("action") or ""),
        "targetId": str(target.get("id") or ""),
        "secretFields": secret_fields,
        "secretSummary": "fields=%s" % ", ".join(secret_fields) if secret_fields else "",
    }


def render_live_mutation_dry_run_table(records, include_header=True, columns=None):
    """Render live mutation dry run table implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 16

    selected_columns = list(
        columns
        or [
            "operation",
            "uid",
            "name",
            "type",
            "match",
            "action",
            "targetId",
            "secretSummary",
        ]
    )
    lines = _render_rows(records, selected_columns)
    if include_header:
        return lines
    return lines[2:]


def render_live_mutation_dry_run_json(records):
    """Render live mutation dry run json implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    summary = {
        "itemCount": len(records),
        "createCount": len(
            [item for item in records if item.get("action") == "would-create"]
        ),
        "deleteCount": len(
            [item for item in records if item.get("action") == "would-delete"]
        ),
        "blockedCount": len(
            [
                item
                for item in records
                if item.get("action")
                in ("would-fail-existing", "would-fail-missing", "would-fail-ambiguous")
            ]
        ),
    }
    return json.dumps(
        {
            "items": records,
            "summary": summary,
        },
        indent=2,
        sort_keys=False,
    )
