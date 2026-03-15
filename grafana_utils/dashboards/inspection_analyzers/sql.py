import re
from typing import Any

from .contract import extract_string_values, normalize_query_analysis, unique_strings


def strip_sql_comments(query: str) -> str:
    if not query:
        return ""
    query = re.sub(r"/\*.*?\*/", " ", query, flags=re.DOTALL)
    return re.sub(r"--[^\n]*", " ", query)


def normalize_sql_identifier(value: str) -> str:
    parts = []
    for part in re.split(r"\s*\.\s*", str(value or "").strip()):
        normalized = part.strip()
        if len(normalized) >= 2 and (
            (normalized[0] == normalized[-1] and normalized[0] in ('"', "'", "`"))
            or (normalized[0] == "[" and normalized[-1] == "]")
        ):
            normalized = normalized[1:-1]
        normalized = normalized.strip()
        if normalized:
            parts.append(normalized)
    return ".".join(parts)


def extract_sql_source_references(query: str) -> list[str]:
    query = strip_sql_comments(query)
    if not query:
        return []
    cte_names = {
        str(name).strip().lower()
        for name in extract_string_values(
            query,
            r"(?i)\bwith\s+([A-Za-z_][A-Za-z0-9_$]*)\s+as\s*\(",
        )
    }
    references = []
    for value in extract_string_values(
        query,
        (
            r"(?i)\b(?:from|join|update|into|delete\s+from)\s+"
            r"("
            r"(?:[A-Za-z_][A-Za-z0-9_$]*|\"[^\"]+\"|`[^`]+`|\[[^\]]+\])"
            r"(?:\s*\.\s*(?:[A-Za-z_][A-Za-z0-9_$]*|\"[^\"]+\"|`[^`]+`|\[[^\]]+\])){0,2}"
            r")"
        ),
    ):
        normalized = normalize_sql_identifier(value)
        if normalized and normalized.lower() not in cte_names:
            references.append(normalized)
    return unique_strings(references)


def extract_sql_query_shape_hints(query: str) -> list[str]:
    lowered = strip_sql_comments(query).lower()
    hints = []
    for hint, pattern in (
        ("with", r"\bwith\b"),
        ("select", r"\bselect\b"),
        ("insert", r"\binsert\s+into\b"),
        ("update", r"\bupdate\b"),
        ("delete", r"\bdelete\s+from\b"),
        ("distinct", r"\bdistinct\b"),
        ("join", r"\bjoin\b"),
        ("where", r"\bwhere\b"),
        ("group_by", r"\bgroup\s+by\b"),
        ("having", r"\bhaving\b"),
        ("order_by", r"\border\s+by\b"),
        ("limit", r"\blimit\b"),
        ("top", r"\btop\s+\d+\b"),
        ("union", r"\bunion(?:\s+all)?\b"),
        ("window", r"\bover\s*\("),
        ("subquery", r"\b(?:from|join)\s*\("),
    ):
        if re.search(pattern, lowered):
            hints.append(hint)
    return unique_strings(hints)


def analyze_query(
    panel: dict[str, Any], target: dict[str, Any], query_field: str, query_text: str
) -> dict[str, Any]:
    del panel, target, query_field
    return normalize_query_analysis(
        {
            "metrics": extract_sql_query_shape_hints(query_text),
            "measurements": extract_sql_source_references(query_text),
            "buckets": [],
        }
    )
