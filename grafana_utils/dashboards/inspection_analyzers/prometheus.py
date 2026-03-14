import re
from typing import Any

from .contract import (
    PROMETHEUS_RESERVED_WORDS,
    extract_buckets,
    extract_measurements,
    extract_string_values,
    normalize_query_analysis,
    unique_strings,
)


def extract_prometheus_metric_names(query: str) -> list[str]:
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


def analyze_query(panel: dict[str, Any], target: dict[str, Any], query_field: str, query_text: str) -> dict[str, Any]:
    del panel, target, query_field
    return normalize_query_analysis(
        {
            "metrics": extract_prometheus_metric_names(query_text),
            "measurements": extract_measurements(query_text),
            "buckets": extract_buckets(query_text),
        }
    )
