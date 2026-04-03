from typing import Any

from .contract import (
    extract_buckets,
    extract_measurements,
    extract_string_values,
    normalize_query_analysis,
    unique_strings,
)


def extract_flux_pipeline_functions(query: str) -> list[str]:
    return unique_strings(
        extract_string_values(
            query,
            r"(?:^|\|>)\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(",
        )
    )


def analyze_query(
    panel: dict[str, Any], target: dict[str, Any], query_field: str, query_text: str
) -> dict[str, Any]:
    del panel, target, query_field
    return normalize_query_analysis(
        {
            "metrics": extract_flux_pipeline_functions(query_text),
            "measurements": extract_measurements(query_text),
            "buckets": extract_buckets(query_text),
        }
    )
