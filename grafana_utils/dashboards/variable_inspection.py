"""Helpers for inspecting live Grafana dashboard templating variables."""

import csv
import io
import json
from typing import Any, Optional
from urllib import parse

from .common import GrafanaError


VARIABLE_OUTPUT_FORMATS = ("table", "csv", "json")


def resolve_dashboard_uid(
    dashboard_uid: Optional[str] = None,
    dashboard_url: Optional[str] = None,
) -> str:
    """Resolve one dashboard UID from explicit input or a Grafana dashboard URL."""
    uid_value = str(dashboard_uid or "").strip()
    if uid_value:
        return uid_value
    url_value = str(dashboard_url or "").strip()
    if not url_value:
        raise GrafanaError("Set --dashboard-uid or pass --dashboard-url.")
    parsed = parse.urlparse(url_value)
    path_parts = [item for item in parsed.path.split("/") if item]
    if len(path_parts) >= 2 and path_parts[0] in ("d", "d-solo"):
        resolved = str(path_parts[1] or "").strip()
        if resolved:
            return resolved
    raise GrafanaError(
        "Unable to derive dashboard UID from --dashboard-url. "
        "Use a /d/... or /d-solo/... Grafana URL, or pass --dashboard-uid explicitly."
    )


def inspect_dashboard_variables_with_client(
    client: Any,
    dashboard_uid: Optional[str] = None,
    dashboard_url: Optional[str] = None,
    vars_query: Optional[str] = None,
) -> dict[str, Any]:
    """Fetch one dashboard and return a normalized variable inspection document."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 106, 15, 58

    resolved_uid = resolve_dashboard_uid(dashboard_uid=dashboard_uid, dashboard_url=dashboard_url)
    payload = client.fetch_dashboard(resolved_uid)
    if not isinstance(payload, dict):
        raise GrafanaError("Unexpected dashboard payload for UID %s." % resolved_uid)
    dashboard = payload.get("dashboard")
    if not isinstance(dashboard, dict):
        raise GrafanaError("Dashboard UID %s did not include a dashboard object." % resolved_uid)
    document = build_dashboard_variable_document(dashboard, dashboard_uid=resolved_uid)
    apply_vars_query_overrides(document["variables"], vars_query)
    document["variableCount"] = len(document["variables"])
    return document


def build_dashboard_variable_document(
    dashboard: dict[str, Any],
    dashboard_uid: Optional[str] = None,
) -> dict[str, Any]:
    """Normalize one dashboard JSON object into the variable inspection shape."""
    resolved_uid = str(dashboard_uid or dashboard.get("uid") or "").strip()
    variables = extract_dashboard_variables(dashboard)
    return {
        "dashboardUid": resolved_uid,
        "dashboardTitle": str(dashboard.get("title") or resolved_uid or "dashboard"),
        "variableCount": len(variables),
        "variables": variables,
    }


def extract_dashboard_variables(dashboard: dict[str, Any]) -> list[dict[str, Any]]:
    """Extract normalized templating rows from one dashboard document."""
    # Call graph: see callers/callees.
    #   Upstream callers: 58
    #   Downstream callees: 186, 208, 219

    templating = dashboard.get("templating")
    if not isinstance(templating, dict):
        return []
    raw_variables = templating.get("list")
    if not isinstance(raw_variables, list):
        return []
    rows = []
    for raw_item in raw_variables:
        if not isinstance(raw_item, dict):
            continue
        name = str(raw_item.get("name") or "").strip()
        if not name:
            continue
        options = _normalize_options(raw_item.get("options"))
        rows.append(
            {
                "name": name,
                "type": str(raw_item.get("type") or ""),
                "label": str(raw_item.get("label") or ""),
                "current": _format_current_value(raw_item.get("current")),
                "datasource": _format_compact_value(raw_item.get("datasource")),
                "query": _format_compact_value(raw_item.get("query")),
                "multi": bool(raw_item.get("multi")),
                "includeAll": bool(raw_item.get("includeAll")),
                "optionCount": len(options),
                "options": options,
            }
        )
    return rows


def apply_vars_query_overrides(
    rows: list[dict[str, Any]],
    vars_query: Optional[str],
) -> list[dict[str, Any]]:
    """Overlay one Grafana vars-query fragment onto normalized variable rows."""
    overrides = parse_vars_query(vars_query)
    if not overrides:
        return rows
    for row in rows:
        name = str(row.get("name") or "")
        if name in overrides:
            row["current"] = overrides[name]
    return rows


def parse_vars_query(vars_query: Optional[str]) -> dict[str, str]:
    """Parse one Grafana query fragment into variable-name overrides."""
    query_value = str(vars_query or "").strip()
    if not query_value:
        return {}
    parsed_pairs = parse.parse_qsl(
        query_value.lstrip("?"),
        keep_blank_values=True,
    )
    overrides = {}
    for key, value in parsed_pairs:
        if not key.startswith("var-"):
            continue
        name = str(key[4:] or "").strip()
        if not name:
            continue
        overrides[name] = value
    return overrides


def render_dashboard_variable_document(
    document: dict[str, Any],
    output_format: str = "table",
    include_header: bool = True,
) -> str:
    """Render one normalized variable inspection document."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 242, 255

    normalized_format = str(output_format or "table").strip().lower()
    if normalized_format not in VARIABLE_OUTPUT_FORMATS:
        raise GrafanaError(
            "Unsupported dashboard variable output format %r. "
            "Use one of: %s."
            % (output_format, ", ".join(VARIABLE_OUTPUT_FORMATS))
        )
    if normalized_format == "json":
        return json.dumps(document, indent=2, sort_keys=True)
    rows = [
        [
            str(item.get("name") or ""),
            str(item.get("type") or ""),
            str(item.get("label") or ""),
            str(item.get("current") or ""),
            str(item.get("datasource") or ""),
            _summarize_options(item),
        ]
        for item in list(document.get("variables") or [])
        if isinstance(item, dict)
    ]
    if normalized_format == "csv":
        output = io.StringIO()
        writer = csv.writer(output)
        if include_header:
            writer.writerow(
                ["name", "type", "label", "current", "datasource", "options"]
            )
        writer.writerows(rows)
        return output.getvalue()
    return "\n".join(
        _render_simple_table(
            ["NAME", "TYPE", "LABEL", "CURRENT", "DATASOURCE", "OPTIONS"],
            rows,
            include_header=include_header,
        )
    )


def _normalize_options(raw_options: Any) -> list[str]:
    """Internal helper for normalize options."""
    if not isinstance(raw_options, list):
        return []
    values = []
    for item in raw_options:
        rendered = _format_option_value(item)
        if rendered:
            values.append(rendered)
    return values


def _format_option_value(value: Any) -> str:
    """Internal helper for format option value."""
    if isinstance(value, dict):
        if value.get("text") not in (None, ""):
            return str(value.get("text"))
        if value.get("value") not in (None, ""):
            return _format_compact_value(value.get("value"))
    return _format_compact_value(value)


def _format_current_value(value: Any) -> str:
    """Internal helper for format current value."""
    if isinstance(value, dict):
        if value.get("text") not in (None, ""):
            return _format_compact_value(value.get("text"))
        if value.get("value") not in (None, ""):
            return _format_compact_value(value.get("value"))
        return ""
    return _format_compact_value(value)


def _format_compact_value(value: Any) -> str:
    """Internal helper for format compact value."""
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (str, int, float)):
        return str(value)
    if isinstance(value, list):
        return ",".join(_format_compact_value(item) for item in value)
    if isinstance(value, dict):
        if value.get("uid") not in (None, ""):
            return str(value.get("uid"))
        if value.get("name") not in (None, ""):
            return str(value.get("name"))
        if value.get("text") not in (None, ""):
            return _format_compact_value(value.get("text"))
        if value.get("value") not in (None, ""):
            return _format_compact_value(value.get("value"))
        return json.dumps(value, sort_keys=True)
    return str(value)


def _summarize_options(row: dict[str, Any]) -> str:
    """Internal helper for summarize options."""
    options = list(row.get("options") or [])
    if not options:
        return ""
    if len(options) <= 3:
        return ",".join(str(item) for item in options)
    return "%s (+%s more)" % (
        ",".join(str(item) for item in options[:3]),
        len(options) - 3,
    )


def _render_simple_table(
    headers: list[str],
    rows: list[list[str]],
    include_header: bool = True,
) -> list[str]:
    """Internal helper for render simple table."""
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: list[str]) -> str:
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.append(format_row(headers))
        lines.append(format_row(["-" * width for width in widths]))
    lines.extend(format_row(row) for row in rows)
    return lines


__all__ = [
    "VARIABLE_OUTPUT_FORMATS",
    "apply_vars_query_overrides",
    "build_dashboard_variable_document",
    "extract_dashboard_variables",
    "inspect_dashboard_variables_with_client",
    "parse_vars_query",
    "render_dashboard_variable_document",
    "resolve_dashboard_uid",
]
