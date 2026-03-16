"""Safer render helpers for unwired datasource live mutation dry-run output."""

import json

from ..dashboard_cli import GrafanaError

SAFE_DRY_RUN_COLUMN_HEADERS = {
    "operation": "OPERATION",
    "uid": "UID",
    "name": "NAME",
    "type": "TYPE",
    "match": "MATCH",
    "action": "ACTION",
    "targetId": "TARGET_ID",
}


def validate_columns(columns):
    selected = list(columns or [])
    unsupported = [column for column in selected if column not in SAFE_DRY_RUN_COLUMN_HEADERS]
    if unsupported:
        raise GrafanaError(
            "Unsupported live mutation dry-run column(s): %s."
            % ", ".join(unsupported)
        )
    return selected


def build_live_mutation_dry_run_record(operation, plan, spec=None, uid=None, name=None):
    spec = dict(spec or {})
    target = plan.get("target") or {}
    return {
        "operation": str(operation or "").strip(),
        "uid": str(spec.get("uid") or uid or target.get("uid") or ""),
        "name": str(spec.get("name") or name or target.get("name") or ""),
        "type": str(spec.get("type") or target.get("type") or ""),
        "match": str(plan.get("match") or ""),
        "action": str(plan.get("action") or ""),
        "targetId": str(target.get("id") or ""),
    }


def render_live_mutation_dry_run_table(records, include_header=True, columns=None):
    selected_columns = validate_columns(
        columns or ["operation", "uid", "name", "type", "match", "action", "targetId"]
    )
    headers = [SAFE_DRY_RUN_COLUMN_HEADERS[column] for column in selected_columns]
    rows = [[str(item.get(column) or "") for column in selected_columns] for item in records]
    widths = [len(value) for value in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def render_row(values):
        return "  ".join(values[index].ljust(widths[index]) for index in range(len(values)))

    lines = []
    if include_header:
        lines.append(render_row(headers))
        lines.append(render_row(["-" * width for width in widths]))
    for row in rows:
        lines.append(render_row(row))
    return lines


def render_live_mutation_dry_run_json(records):
    summary = {
        "itemCount": len(records),
        "createCount": len([item for item in records if item.get("action") == "would-create"]),
        "deleteCount": len([item for item in records if item.get("action") == "would-delete"]),
        "blockedCount": len([item for item in records if item.get("action", "").startswith("would-fail-")]),
    }
    return json.dumps({"items": records, "summary": summary}, indent=2, sort_keys=False)
