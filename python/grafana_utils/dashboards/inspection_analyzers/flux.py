"""
Flux analyzer for dashboard query inspection.
"""

from typing import Any

from .contract import (
    extract_buckets,
    extract_influxql_select_functions,
    extract_influxql_select_metrics,
    extract_influxql_time_buckets,
    extract_measurements,
    extract_string_values,
    normalize_query_analysis,
    unique_strings,
)


def extract_flux_pipeline_functions(query: str) -> list[str]:
    """Extract function names from Flux pipeline stages."""
    return unique_strings(
        extract_string_values(
            query,
            r'(?:^|\|>)\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(',
        )
    )


def analyze_query(panel: dict[str, Any], target: dict[str, Any], query_field: str, query_text: str) -> dict[str, Any]:
    """Build normalized analysis output for a Flux query."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 16

    del panel, target, query_field
    stripped = str(query_text or "").lstrip()
    return normalize_query_analysis(
        {
            "metrics": (
                extract_influxql_select_metrics(query_text)
            ),
            "functions": (
                extract_flux_pipeline_functions(query_text)
                if (
                    stripped.startswith("from(")
                    or stripped.startswith("from (")
                    or "|>" in str(query_text or "")
                )
                else []
            )
            + extract_influxql_select_functions(query_text),
            "measurements": extract_measurements(query_text),
            "buckets": extract_buckets(query_text) + extract_influxql_time_buckets(query_text),
        }
    )
