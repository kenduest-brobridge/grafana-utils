from typing import Any

from .contract import build_default_query_analysis


def analyze_query(
    panel: dict[str, Any],
    target: dict[str, Any],
    query_field: str,
    query_text: str,
) -> dict[str, Any]:
    del panel, query_field
    return build_default_query_analysis(target, query_text)
