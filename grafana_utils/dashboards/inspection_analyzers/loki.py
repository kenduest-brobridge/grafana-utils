"""
Loki analyzer for dashboard query inspection.
"""

import re
from typing import Any

from .contract import normalize_query_analysis, unique_strings


def extract_stream_matchers(query: str) -> list[str]:
    """Collect stream-matchers from Loki selectors."""
    if not query:
        return []
    values = []
    for selector in re.findall(r"\{([^{}]+)\}", query):
        for matcher in re.findall(
            r'([A-Za-z_][A-Za-z0-9_]*)\s*(=~|!~|=|!=)\s*("(?:\\.|[^"\\])*")',
            selector,
        ):
            values.append("%s%s%s" % matcher)
    return unique_strings(values)


def extract_pipeline_stage_names(query: str) -> list[str]:
    """Collect pipeline stage names used in a Loki query."""
    if not query:
        return []
    values = []
    filter_stage_map = {
        "|=": "filter_eq",
        "!=": "filter_neq",
        "|~": "filter_regex",
        "!~": "filter_not_regex",
    }
    for operator in re.findall(r"\s(\|=|!=|\|~|!~)\s", query):
        stage_name = filter_stage_map.get(operator)
        if stage_name:
            values.append(stage_name)
    for stage in re.findall(r"\s\|\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:\(|\b)", query):
        values.append(stage)
    return unique_strings(values)


def extract_range_and_aggregation_functions(query: str) -> list[str]:
    """Collect Loki aggregation and range function usage."""
    if not query:
        return []
    allowed = {
        "rate",
        "bytes_rate",
        "count_over_time",
        "rate_counter",
        "bytes_over_time",
        "avg_over_time",
        "min_over_time",
        "max_over_time",
        "sum_over_time",
        "quantile_over_time",
        "absent_over_time",
        "first_over_time",
        "last_over_time",
        "stddev_over_time",
        "stdvar_over_time",
        "sum",
        "avg",
        "min",
        "max",
        "count",
        "topk",
        "bottomk",
        "sort",
        "sort_desc",
        "quantile",
    }
    values = []
    for name in re.findall(
        r"\b([A-Za-z_][A-Za-z0-9_]*)\b(?=\s*\(|\s+(?:by|without)\b)",
        query,
    ):
        if name.lower() in allowed:
            values.append(name)
    return unique_strings(values)


def analyze_query(
    panel: dict[str, Any],
    target: dict[str, Any],
    query_field: str,
    query_text: str,
) -> dict[str, Any]:
    """Build normalized analysis output for a Loki query."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 11, 25, 45

    del panel, target, query_field
    return normalize_query_analysis(
        {
            "metrics": extract_range_and_aggregation_functions(query_text)
            + extract_pipeline_stage_names(query_text),
            "measurements": extract_stream_matchers(query_text),
            "buckets": [],
        }
    )
