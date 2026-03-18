"""
Prometheus analyzer for dashboard query inspection.
"""

import re
from typing import Any

from .contract import (
    PROMETHEUS_RESERVED_WORDS,
    extract_measurements,
    extract_range_windows,
    extract_string_values,
    normalize_query_analysis,
    unique_strings,
)


def extract_prometheus_metric_names(query: str) -> list[str]:
    """Collect metric-like identifiers from a Prometheus query."""
    if not query:
        return []
    values = extract_string_values(
        query,
        r'__name__\s*=\s*"([A-Za-z_:][A-Za-z0-9_:]*)"',
    )
    sanitized_query = re.sub(r'"(?:\\.|[^"\\])*"', '""', query)
    sanitized_query = re.sub(
        r"\b(?:by|without|on|ignoring)\s*\(\s*[^)]*\)",
        " ",
        sanitized_query,
    )
    sanitized_query = re.sub(
        r"\b(?:group_left|group_right)\s*(?:\(\s*[^)]*\))?",
        " ",
        sanitized_query,
    )
    sanitized_query = re.sub(r"\{[^{}]*\}", "{}", sanitized_query)
    candidates = re.finditer(
        r"(?<![A-Za-z0-9_:])([A-Za-z_:][A-Za-z0-9_:]*)",
        sanitized_query,
    )
    for matched in candidates:
        candidate = matched.group(1)
        if candidate.lower() in PROMETHEUS_RESERVED_WORDS:
            continue
        if candidate.startswith("$"):
            continue
        trailing = sanitized_query[matched.end() :].lstrip()
        if trailing.startswith("("):
            continue
        if trailing.startswith(("=", "!=", "=~", "!~")):
            continue
        values.append(candidate)
    return unique_strings(values)


def extract_prometheus_functions(query: str) -> list[str]:
    """Collect function-like identifiers from a Prometheus query."""
    if not query:
        return []
    values = []
    for name in re.findall(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(", query):
        if name in ("by", "without", "on", "ignoring", "group_left", "group_right"):
            continue
        values.append(name)
    return unique_strings(values)


def analyze_query(panel: dict[str, Any], target: dict[str, Any], query_field: str, query_text: str) -> dict[str, Any]:
    """Build normalized analysis output for a Prometheus query."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 18

    del panel, target, query_field
    return normalize_query_analysis(
        {
            "metrics": extract_prometheus_metric_names(query_text),
            "functions": extract_prometheus_functions(query_text),
            "measurements": extract_measurements(query_text),
            "buckets": extract_range_windows(query_text),
        }
    )
