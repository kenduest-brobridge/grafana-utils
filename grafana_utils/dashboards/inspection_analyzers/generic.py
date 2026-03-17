"""
Generic analyzer helpers for query-family inspection parsing.
"""

from typing import Any

from .contract import build_default_query_analysis


def analyze_query(
    panel: dict[str, Any],
    target: dict[str, Any],
    query_field: str,
    query_text: str,
) -> dict[str, Any]:
    """Fallback analyzer for unsupported query families."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    del panel, query_field
    return build_default_query_analysis(target, query_text)
