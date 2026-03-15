import re
from typing import Any


DATASOURCE_FAMILY_PROMETHEUS = "prometheus"
DATASOURCE_FAMILY_LOKI = "loki"
DATASOURCE_FAMILY_FLUX = "flux"
DATASOURCE_FAMILY_SQL = "sql"
DATASOURCE_FAMILY_UNKNOWN = "unknown"
QUERY_ANALYSIS_FIELDS = ("metrics", "measurements", "buckets")


def extract_string_values(query: str, pattern: str) -> list[str]:
    if not query:
        return []
    values = []
    for match in re.findall(pattern, query):
        if isinstance(match, tuple):
            for item in match:
                if item:
                    values.append(str(item))
                    break
        elif match:
            values.append(str(match))
    return values


def unique_strings(values: list[str]) -> list[str]:
    seen: set[str] = set()
    ordered = []
    for value in values:
        text = str(value or "").strip()
        if not text or text in seen:
            continue
        seen.add(text)
        ordered.append(text)
    return ordered


def normalize_query_analysis(result: dict[str, Any]) -> dict[str, list[str]]:
    normalized = {}
    for field in QUERY_ANALYSIS_FIELDS:
        value = (result or {}).get(field)
        if isinstance(value, list):
            normalized[field] = unique_strings([str(item) for item in value])
        elif value is None:
            normalized[field] = []
        else:
            normalized[field] = unique_strings([str(value)])
    return normalized


def build_query_field_and_text(target: dict[str, Any]) -> list[str]:
    for field in (
        "expr",
        "expression",
        "query",
        "rawSql",
        "sql",
        "rawQuery",
        "jql",
        "logql",
        "search",
        "definition",
        "command",
    ):
        value = target.get(field)
        if value is None:
            continue
        text = str(value).strip()
        if text:
            return [field, text]
    return ["", ""]


PROMETHEUS_RESERVED_WORDS = {
    "and",
    "bool",
    "by",
    "ignoring",
    "group_left",
    "group_right",
    "on",
    "offset",
    "or",
    "unless",
    "without",
    "sum",
    "min",
    "max",
    "avg",
    "count",
    "stddev",
    "stdvar",
    "bottomk",
    "topk",
    "quantile",
    "count_values",
    "rate",
    "irate",
    "increase",
    "delta",
    "idelta",
    "deriv",
    "predict_linear",
    "holt_winters",
    "sort",
    "sort_desc",
    "label_replace",
    "label_join",
    "histogram_quantile",
    "clamp_max",
    "clamp_min",
    "abs",
    "absent",
    "ceil",
    "floor",
    "ln",
    "log2",
    "log10",
    "round",
    "scalar",
    "vector",
    "year",
    "month",
    "day_of_month",
    "day_of_week",
    "hour",
    "minute",
    "time",
}


def extract_metric_names(query: str) -> list[str]:
    if not query:
        return []
    sanitized_query = re.sub(r'"[^"]*"', '""', query)
    candidates = re.finditer(
        r"(?<![A-Za-z0-9_:])([A-Za-z_:][A-Za-z0-9_:]*)",
        sanitized_query,
    )
    values = []
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


def extract_measurements(query: str) -> list[str]:
    return unique_strings(
        extract_string_values(
            query,
            r'_measurement\s*==\s*"([^"]+)"',
        )
        + extract_string_values(
            query,
            r'from\s*\(\s*measurement\s*:\s*"([^"]+)"',
        )
    )


def extract_buckets(query: str) -> list[str]:
    return unique_strings(
        extract_string_values(
            query,
            r'from\s*\(\s*bucket\s*:\s*"([^"]+)"',
        )
        + extract_string_values(
            query,
            r'from\(bucket:\s*"([^"]+)"',
        )
    )


def build_default_query_analysis(
    target: dict[str, Any], query_text: str
) -> dict[str, list[str]]:
    del target
    return normalize_query_analysis(
        {
            "metrics": extract_metric_names(query_text),
            "measurements": extract_measurements(query_text),
            "buckets": extract_buckets(query_text),
        }
    )
