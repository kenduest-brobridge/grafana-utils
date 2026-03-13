"""Dashboard inspection report model and render helpers."""

import csv
import io
import re
from collections import OrderedDict
from pathlib import Path
from typing import Any, Dict, List, Optional, Set

from .common import (
    DATASOURCE_TYPE_ALIASES,
    DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE,
    DEFAULT_UNKNOWN_UID,
    GrafanaError,
)
from .transformer import is_builtin_datasource_ref, is_placeholder_string


REPORT_COLUMN_HEADERS = OrderedDict(
    [
        ("dashboardUid", "DASHBOARD_UID"),
        ("dashboardTitle", "DASHBOARD_TITLE"),
        ("folderPath", "FOLDER_PATH"),
        ("panelId", "PANEL_ID"),
        ("panelTitle", "PANEL_TITLE"),
        ("panelType", "PANEL_TYPE"),
        ("refId", "REF_ID"),
        ("datasource", "DATASOURCE"),
        ("queryField", "QUERY_FIELD"),
        ("metrics", "METRICS"),
        ("measurements", "MEASUREMENTS"),
        ("buckets", "BUCKETS"),
        ("query", "QUERY"),
        ("file", "FILE"),
    ]
)
OPTIONAL_REPORT_COLUMN_HEADERS = OrderedDict([("datasourceUid", "DATASOURCE_UID")])
REPORT_COLUMN_ALIASES = {
    "dashboard_uid": "dashboardUid",
    "dashboard_title": "dashboardTitle",
    "folder_path": "folderPath",
    "panel_id": "panelId",
    "panel_title": "panelTitle",
    "panel_type": "panelType",
    "ref_id": "refId",
    "query_field": "queryField",
    "datasource_uid": "datasourceUid",
}
SUPPORTED_REPORT_COLUMN_HEADERS = OrderedDict(
    list(REPORT_COLUMN_HEADERS.items()) + list(OPTIONAL_REPORT_COLUMN_HEADERS.items())
)
INSPECT_REPORT_FORMAT_CHOICES = ("table", "json", "csv", "tree", "tree-table")
NORMALIZED_QUERY_REPORT_FIELDS = (
    "dashboardUid",
    "dashboardTitle",
    "folderPath",
    "panelId",
    "panelTitle",
    "panelType",
    "refId",
    "datasource",
    "datasourceUid",
    "queryField",
    "query",
    "metrics",
    "measurements",
    "buckets",
    "file",
)
FLUX_DATASOURCE_FAMILIES = {"influxdb"}
SQL_DATASOURCE_FAMILIES = {"mysql", "postgres", "mssql"}
INSPECT_EXPORT_HELP_FULL_EXAMPLES = (
    "Extended examples:\n\n"
    "  Inspect one raw export as the default flat query table:\n"
    "    grafana-utils inspect-export --import-dir ./dashboards/raw --report\n\n"
    "  Inspect one raw export as dashboard-first grouped tables:\n"
    "    grafana-utils inspect-export --import-dir ./dashboards/raw --report tree-table\n\n"
    "  Narrow the report to one datasource and one panel id:\n"
    "    grafana-utils inspect-export --import-dir ./dashboards/raw --report tree-table "
    "--report-filter-datasource prom-main --report-filter-panel-id 7\n\n"
    "  Trim the per-query columns for flat or tree-table output:\n"
    "    grafana-utils inspect-export --import-dir ./dashboards/raw --report tree-table "
    "--report-columns panel_id,panel_title,datasource,query"
)
INSPECT_LIVE_HELP_FULL_EXAMPLES = (
    "Extended examples:\n\n"
    "  Inspect live dashboards as the default flat query table:\n"
    "    grafana-utils inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --report\n\n"
    "  Inspect live dashboards as dashboard-first grouped tables:\n"
    "    grafana-utils inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --report tree-table\n\n"
    "  Narrow live inspection to one datasource and one panel id:\n"
    "    grafana-utils inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --report tree-table --report-filter-datasource prom-main "
    "--report-filter-panel-id 7\n\n"
    "  Trim the per-query columns for flat or tree-table output:\n"
    "    grafana-utils inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --report tree-table "
    "--report-columns panel_id,panel_title,datasource,query"
)


def build_export_inspection_report_document(
    import_dir: Path,
    deps: Dict[str, Any],
) -> Dict[str, Any]:
    """Analyze one raw export directory and emit one per-query inspection record."""
    metadata = deps["load_export_metadata"](
        import_dir, expected_variant=deps["RAW_EXPORT_SUBDIR"]
    )
    dashboard_files = deps["discover_dashboard_files"](import_dir)
    folder_inventory = deps["load_folder_inventory"](import_dir, metadata)
    datasource_inventory = deps["load_datasource_inventory"](import_dir, metadata)
    folder_lookup = deps["build_folder_inventory_lookup"](folder_inventory)
    datasources_by_uid = {}
    datasources_by_name = {}
    for datasource in datasource_inventory:
        uid = str(datasource.get("uid") or "").strip()
        name = str(datasource.get("name") or "").strip()
        if uid:
            datasources_by_uid[uid] = dict(datasource)
        if name:
            datasources_by_name[name] = dict(datasource)
    records = []

    for dashboard_file in dashboard_files:
        document = deps["load_json_file"](dashboard_file)
        dashboard = deps["extract_dashboard_object"](
            document, "Dashboard payload must be a JSON object."
        )
        folder_record = deps["resolve_folder_inventory_record_for_dashboard"](
            document,
            dashboard_file,
            import_dir,
            folder_lookup,
        )
        folder_path = str(
            (folder_record or {}).get("path")
            or (folder_record or {}).get("title")
            or DEFAULT_FOLDER_TITLE
        ).strip() or DEFAULT_FOLDER_TITLE
        for panel in deps["iter_dashboard_panels"](dashboard.get("panels")):
            targets = panel.get("targets")
            if not isinstance(targets, list):
                continue
            for target in targets:
                if not isinstance(target, dict):
                    continue
                records.append(
                    build_query_report_record(
                        dashboard,
                        folder_path,
                        panel,
                        target,
                        dashboard_file,
                        datasources_by_uid,
                        datasources_by_name,
                    )
                )

    records.sort(
        key=lambda item: (
            item["folderPath"],
            item["dashboardTitle"],
            item["dashboardUid"],
            item["panelId"],
            item["refId"],
        )
    )
    return {
        "summary": {
            "dashboardCount": len(
                set(record["dashboardUid"] for record in records)
            ),
            "queryRecordCount": len(records),
        },
        "queries": records,
    }


def describe_export_datasource_ref(
    ref: Any,
    datasources_by_uid: Dict[str, Dict[str, str]],
    datasources_by_name: Dict[str, Dict[str, str]],
) -> str:
    """Render one exported datasource reference into a stable label."""
    if ref is None:
        return ""
    if isinstance(ref, str):
        label = ref.strip()
        if not label:
            return ""
        if is_builtin_datasource_ref(label):
            return ""
        datasource = datasources_by_name.get(label)
        if datasource is not None:
            return str(datasource.get("uid") or label)
        return label
    if not isinstance(ref, dict):
        return str(ref).strip()
    uid = str(ref.get("uid") or "").strip()
    name = str(ref.get("name") or "").strip()
    ref_type = str(ref.get("type") or "").strip()
    if uid:
        if is_builtin_datasource_ref(uid):
            return ""
        datasource = datasources_by_uid.get(uid)
        if datasource is not None:
            return str(datasource.get("uid") or uid)
        return uid
    if name:
        datasource = datasources_by_name.get(name)
        if datasource is not None:
            return str(datasource.get("uid") or name)
        return name
    return ref_type


def describe_panel_datasource(
    panel: Dict[str, Any],
    target: Dict[str, Any],
    datasources_by_uid: Dict[str, Dict[str, str]],
    datasources_by_name: Dict[str, Dict[str, str]],
) -> str:
    """Resolve one panel/query datasource label from target or panel scope."""
    target_ref = target.get("datasource")
    panel_ref = panel.get("datasource")
    label = describe_export_datasource_ref(
        target_ref,
        datasources_by_uid,
        datasources_by_name,
    )
    if label:
        return label
    return describe_export_datasource_ref(
        panel_ref,
        datasources_by_uid,
        datasources_by_name,
    )


def describe_panel_datasource_uid(
    panel: Dict[str, Any],
    target: Dict[str, Any],
    datasources_by_name: Dict[str, Dict[str, str]],
) -> str:
    """Resolve one best-effort datasource uid for a panel/query target."""
    for ref in (target.get("datasource"), panel.get("datasource")):
        if isinstance(ref, dict):
            uid = str(ref.get("uid") or "").strip()
            if uid:
                return uid
            name = str(ref.get("name") or "").strip()
            if name and datasources_by_name.get(name):
                return str(datasources_by_name[name].get("uid") or "")
        elif isinstance(ref, str):
            name = ref.strip()
            if name and datasources_by_name.get(name):
                return str(datasources_by_name[name].get("uid") or "")
    return ""


def extract_string_values(query: str, pattern: str) -> List[str]:
    """Extract one stable list of string matches from a query text."""
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


def unique_strings(values: List[str]) -> List[str]:
    """Keep first-seen order while dropping empty duplicates."""
    seen = set()  # type: Set[str]
    ordered = []
    for value in values:
        text = str(value or "").strip()
        if not text or text in seen:
            continue
        seen.add(text)
        ordered.append(text)
    return ordered


def build_query_field_and_text(target: Dict[str, Any]) -> List[str]:
    """Choose the most relevant query field and raw text from one target."""
    for field in (
        "expr",
        "query",
        "rawSql",
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


def resolve_query_datasource_family(
    panel: Dict[str, Any],
    target: Dict[str, Any],
    datasources_by_uid: Dict[str, Dict[str, str]],
    datasources_by_name: Dict[str, Dict[str, str]],
) -> str:
    """Resolve one canonical datasource family from target or panel scope."""
    for ref in (target.get("datasource"), panel.get("datasource")):
        if isinstance(ref, dict):
            ref_type = normalize_query_datasource_family(ref.get("type"))
            if ref_type:
                return ref_type
            uid = str(ref.get("uid") or "").strip()
            name = str(ref.get("name") or "").strip()
            datasource = None
            if uid:
                datasource = datasources_by_uid.get(uid)
            if datasource is None and name:
                datasource = datasources_by_name.get(name)
            if datasource is not None:
                return normalize_query_datasource_family(datasource.get("type"))
        elif isinstance(ref, str):
            name = ref.strip()
            normalized = normalize_query_datasource_family(name)
            if normalized:
                return normalized
            datasource = datasources_by_name.get(name)
            if datasource is not None:
                return normalize_query_datasource_family(datasource.get("type"))
    return ""


def normalize_query_datasource_family(value: Any) -> str:
    """Normalize datasource types into one stable inspection family id."""
    normalized = str(
        DATASOURCE_TYPE_ALIASES.get(
            str(value or "").strip().lower(),
            str(value or "").strip().lower(),
        )
    )
    if not normalized:
        return ""
    if normalized in FLUX_DATASOURCE_FAMILIES or normalized in SQL_DATASOURCE_FAMILIES:
        return normalized
    if "influx" in normalized:
        return "influxdb"
    if "postgres" in normalized:
        return "postgres"
    if "mysql" in normalized:
        return "mysql"
    if "mssql" in normalized or "sqlserver" in normalized:
        return "mssql"
    return normalized


def strip_sql_comments(query: str) -> str:
    """Drop obvious SQL comments before heuristic extraction."""
    if not query:
        return ""
    query = re.sub(r"/\*.*?\*/", " ", query, flags=re.DOTALL)
    return re.sub(r"--[^\n]*", " ", query)


def extract_metric_names(query: str) -> List[str]:
    """Extract best-effort metric identifiers with the existing generic rules."""
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


def extract_prometheus_metric_names(query: str) -> List[str]:
    """Extract Prometheus metric identifiers conservatively."""
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


def extract_measurements(query: str) -> List[str]:
    """Extract best-effort measurement identifiers from Flux/Influx-style queries."""
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


def extract_flux_pipeline_functions(query: str) -> List[str]:
    """Extract Flux source and pipeline functions in execution order."""
    return unique_strings(
        extract_string_values(
            query,
            r'(?:^|\|>)\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(',
        )
    )


def extract_buckets(query: str) -> List[str]:
    """Extract best-effort bucket identifiers from Flux/Influx-style queries."""
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


def normalize_sql_identifier(value: str) -> str:
    """Normalize one quoted SQL identifier into a compact dotted name."""
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


def extract_sql_source_references(query: str) -> List[str]:
    """Extract best-effort SQL table/source references conservatively."""
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


def extract_sql_query_shape_hints(query: str) -> List[str]:
    """Extract coarse SQL query-shape hints."""
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


def extract_sql_function_names(query: str) -> List[str]:
    """Extract best-effort SQL function calls for the shared metrics field."""
    query = strip_sql_comments(query)
    if not query:
        return []
    ignored = {
        "as",
        "case",
        "cast",
        "distinct",
        "else",
        "end",
        "from",
        "in",
        "join",
        "not",
        "on",
        "over",
        "select",
        "then",
        "when",
        "where",
    }
    values = []
    for name in extract_string_values(
        query,
        r"\b([A-Za-z_][A-Za-z0-9_$]*)\s*\(",
    ):
        if name.lower() in ignored:
            continue
        values.append(name)
    return unique_strings(values)


def looks_like_flux_query(query_text: str) -> bool:
    """Return whether query text looks like Flux."""
    return bool(
        re.search(r"\bfrom\s*\(\s*bucket\s*:", query_text)
        or re.search(r"\|>\s*[A-Za-z_][A-Za-z0-9_]*\s*\(", query_text)
    )


def looks_like_sql_query(query_text: str, query_field: str) -> bool:
    """Return whether query text looks like SQL."""
    if query_field in ("rawSql", "sql"):
        return True
    return bool(
        re.search(
            r"(?is)^\s*(?:with\b.+?\bselect\b|select\b|insert\b|update\b|delete\b)",
            query_text,
        )
    )


def analyze_query_text(
    query_text: str,
    datasource_family: str,
    query_field: str,
) -> Dict[str, List[str]]:
    """Extract conservative datasource-family inspection details."""
    if datasource_family == "loki":
        return {"metrics": [], "measurements": [], "buckets": []}
    if datasource_family in FLUX_DATASOURCE_FAMILIES or looks_like_flux_query(query_text):
        return {
            "metrics": extract_flux_pipeline_functions(query_text),
            "measurements": extract_measurements(query_text),
            "buckets": extract_buckets(query_text),
        }
    if datasource_family in SQL_DATASOURCE_FAMILIES or looks_like_sql_query(
        query_text, query_field
    ):
        return {
            "metrics": extract_sql_query_shape_hints(query_text),
            "measurements": extract_sql_source_references(query_text),
            "buckets": [],
        }
    metrics = extract_metric_names(query_text)
    if datasource_family == "prometheus":
        metrics = extract_prometheus_metric_names(query_text)
    return {
        "metrics": metrics,
        "measurements": extract_measurements(query_text),
        "buckets": extract_buckets(query_text),
    }


def build_query_report_record(
    dashboard: Dict[str, Any],
    folder_path: str,
    panel: Dict[str, Any],
    target: Dict[str, Any],
    dashboard_file: Path,
    datasources_by_uid: Dict[str, Dict[str, str]],
    datasources_by_name: Dict[str, Dict[str, str]],
) -> Dict[str, Any]:
    """Build one canonical per-query inspection row."""
    query_field, query_text = build_query_field_and_text(target)
    datasource_family = resolve_query_datasource_family(
        panel,
        target,
        datasources_by_uid,
        datasources_by_name,
    )
    analysis = analyze_query_text(query_text, datasource_family, query_field)
    record = {
        "dashboardUid": str(dashboard.get("uid") or DEFAULT_UNKNOWN_UID),
        "dashboardTitle": str(dashboard.get("title") or DEFAULT_DASHBOARD_TITLE),
        "folderPath": str(folder_path or DEFAULT_FOLDER_TITLE),
        "panelId": str(panel.get("id") or ""),
        "panelTitle": str(panel.get("title") or ""),
        "panelType": str(panel.get("type") or ""),
        "refId": str(target.get("refId") or ""),
        "datasource": describe_panel_datasource(
            panel,
            target,
            datasources_by_uid,
            datasources_by_name,
        ),
        "datasourceUid": describe_panel_datasource_uid(
            panel,
            target,
            datasources_by_name,
        ),
        "queryField": query_field,
        "query": query_text,
        "metrics": analysis["metrics"],
        "measurements": analysis["measurements"],
        "buckets": analysis["buckets"],
        "file": str(dashboard_file),
    }
    normalized = {}
    for field in NORMALIZED_QUERY_REPORT_FIELDS:
        value = record.get(field)
        if isinstance(value, list):
            normalized[field] = list(value)
        else:
            normalized[field] = str(value or "")
    return normalized


def parse_report_columns(value: Optional[str]) -> Optional[List[str]]:
    """Parse one report column list into canonical inspection field ids."""
    if value is None:
        return None
    columns = []
    for item in value.split(","):
        column = item.strip()
        if column:
            columns.append(REPORT_COLUMN_ALIASES.get(column, column))
    if not columns:
        raise GrafanaError(
            "--report-columns requires one or more comma-separated column ids."
        )
    unknown = [
        column for column in columns if column not in SUPPORTED_REPORT_COLUMN_HEADERS
    ]
    if unknown:
        raise GrafanaError(
            "Unsupported report column(s): %s. Supported values: %s."
            % (
                ", ".join(unknown),
                ", ".join(
                    list(REPORT_COLUMN_ALIASES.keys())
                    + [
                        "datasourceUid",
                        "datasource",
                        "metrics",
                        "measurements",
                        "buckets",
                        "query",
                        "file",
                    ]
                ),
            )
        )
    return columns


def filter_export_inspection_report_document(
    document: Dict[str, Any],
    datasource_label: Optional[str] = None,
    panel_id: Optional[str] = None,
) -> Dict[str, Any]:
    """Filter one flat inspection report document to narrower query rows."""
    if not datasource_label and not panel_id:
        return document
    filtered_records = [
        dict(record)
        for record in list(document.get("queries") or [])
        if (
            (not datasource_label or str(record.get("datasource") or "") == datasource_label)
            and (not panel_id or str(record.get("panelId") or "") == panel_id)
        )
    ]
    return {
        "summary": {
            "dashboardCount": len(
                set(str(record.get("dashboardUid") or "") for record in filtered_records)
            ),
            "queryRecordCount": len(filtered_records),
        },
        "queries": filtered_records,
    }


def format_report_column_value(record: Dict[str, Any], column_id: str) -> str:
    """Format one report cell from the canonical inspection row model."""
    value = record.get(column_id)
    if isinstance(value, list):
        return ",".join(str(item) for item in value)
    return str(value or "")


def render_export_inspection_report_csv(
    document: Dict[str, Any],
    selected_columns: Optional[List[str]] = None,
    include_header: bool = True,
) -> str:
    """Render one full per-query inspection report as CSV."""
    selected_columns = list(selected_columns or REPORT_COLUMN_HEADERS.keys())
    rows = []
    if include_header:
        rows.append(
            [
                REPORT_COLUMN_ALIASES.get(
                    column_id,
                    re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", column_id).lower(),
                )
                for column_id in selected_columns
            ]
        )
    for record in list(document.get("queries") or []):
        rows.append(
            [
                format_report_column_value(record, column_id)
                for column_id in selected_columns
            ]
        )
    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerows(rows)
    return output.getvalue()


def render_export_inspection_table_section(
    headers: List[str],
    rows: List[List[str]],
    include_header: bool = True,
) -> List[str]:
    """Render one simple left-aligned table section."""
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: List[str]) -> str:
        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.append(format_row(headers))
        lines.append(format_row(["-" * width for width in widths]))
    lines.extend(format_row(row) for row in rows)
    return lines


def render_export_inspection_report_tables(
    document: Dict[str, Any],
    import_dir: Path,
    include_header: bool = True,
    selected_columns: Optional[List[str]] = None,
) -> List[str]:
    """Render one full per-query inspection report as a table."""
    summary = document.get("summary") or {}
    query_records = list(document.get("queries") or [])
    selected_columns = list(selected_columns or REPORT_COLUMN_HEADERS.keys())
    lines = ["Export inspection report: %s" % import_dir, ""]

    lines.append("# Summary")
    lines.extend(
        render_export_inspection_table_section(
            ["METRIC", "VALUE"],
            [
                ["dashboard_count", str(int(summary.get("dashboardCount") or 0))],
                ["query_record_count", str(int(summary.get("queryRecordCount") or 0))],
            ],
            include_header=include_header,
        )
    )

    if query_records:
        lines.append("")
        lines.append("# Query report")
        lines.extend(
            render_export_inspection_table_section(
                [
                    SUPPORTED_REPORT_COLUMN_HEADERS[column_id]
                    for column_id in selected_columns
                ],
                [
                    [
                        format_report_column_value(record, column_id)
                        for column_id in selected_columns
                    ]
                    for record in query_records
                ],
                include_header=include_header,
            )
        )
    return lines


def build_grouped_export_inspection_report_document(
    document: Dict[str, Any]
) -> Dict[str, Any]:
    """Normalize one flat inspection report into dashboard-first grouped form."""
    query_records = list(document.get("queries") or [])
    dashboards = OrderedDict()

    for record in query_records:
        dashboard_key = (
            str(record.get("folderPath") or DEFAULT_FOLDER_TITLE),
            str(record.get("dashboardTitle") or DEFAULT_DASHBOARD_TITLE),
            str(record.get("dashboardUid") or DEFAULT_UNKNOWN_UID),
        )
        dashboard_entry = dashboards.get(dashboard_key)
        if dashboard_entry is None:
            dashboard_entry = {
                "dashboardUid": dashboard_key[2],
                "dashboardTitle": dashboard_key[1],
                "folderPath": dashboard_key[0],
                "file": str(record.get("file") or ""),
                "queryCount": 0,
                "panels": OrderedDict(),
            }
            dashboards[dashboard_key] = dashboard_entry
        dashboard_entry["queryCount"] = int(dashboard_entry.get("queryCount") or 0) + 1

        panel_key = (
            str(record.get("panelId") or ""),
            str(record.get("panelTitle") or ""),
            str(record.get("panelType") or ""),
        )
        panel_entry = dashboard_entry["panels"].get(panel_key)
        if panel_entry is None:
            panel_entry = {
                "panelId": panel_key[0],
                "panelTitle": panel_key[1],
                "panelType": panel_key[2],
                "datasources": [],
                "queryCount": 0,
                "queries": [],
            }
            dashboard_entry["panels"][panel_key] = panel_entry
        datasource_label = str(record.get("datasource") or "")
        if datasource_label and datasource_label not in panel_entry["datasources"]:
            panel_entry["datasources"].append(datasource_label)
        panel_entry["queryCount"] = int(panel_entry.get("queryCount") or 0) + 1
        panel_entry["queries"].append(dict(record))

    dashboard_records = []
    panel_count = 0
    for dashboard_entry in dashboards.values():
        panels = []
        for panel_entry in dashboard_entry["panels"].values():
            panel_entry["datasources"].sort()
            panels.append(panel_entry)
        panel_count += len(panels)
        dashboard_records.append(
            {
                "dashboardUid": dashboard_entry["dashboardUid"],
                "dashboardTitle": dashboard_entry["dashboardTitle"],
                "folderPath": dashboard_entry["folderPath"],
                "file": dashboard_entry["file"],
                "panelCount": len(panels),
                "queryCount": int(dashboard_entry.get("queryCount") or 0),
                "panels": panels,
            }
        )

    return {
        "summary": {
            "dashboardCount": len(dashboard_records),
            "panelCount": panel_count,
            "queryRecordCount": len(query_records),
        },
        "dashboards": dashboard_records,
    }


def render_export_inspection_grouped_report(
    document: Dict[str, Any],
    import_dir: Path,
) -> List[str]:
    """Render one per-query inspection report grouped by dashboard and panel."""
    summary = document.get("summary") or {}
    dashboard_records = list(document.get("dashboards") or [])
    lines = ["Export inspection tree report: %s" % import_dir, ""]

    lines.append("# Summary")
    lines.extend(
        render_export_inspection_table_section(
            ["METRIC", "VALUE"],
            [
                ["dashboard_count", str(int(summary.get("dashboardCount") or 0))],
                ["panel_count", str(int(summary.get("panelCount") or 0))],
                ["query_record_count", str(int(summary.get("queryRecordCount") or 0))],
            ],
            include_header=True,
        )
    )

    if dashboard_records:
        lines.append("")
        lines.append("# Dashboard tree")
        for index, dashboard in enumerate(dashboard_records, 1):
            lines.append(
                "[%s] Dashboard %s title=%s path=%s panels=%s queries=%s"
                % (
                    index,
                    str(dashboard.get("dashboardUid") or DEFAULT_UNKNOWN_UID),
                    str(dashboard.get("dashboardTitle") or DEFAULT_DASHBOARD_TITLE),
                    str(dashboard.get("folderPath") or DEFAULT_FOLDER_TITLE),
                    int(dashboard.get("panelCount") or 0),
                    int(dashboard.get("queryCount") or 0),
                )
            )
            for panel in list(dashboard.get("panels") or []):
                datasource_text = ",".join(panel.get("datasources") or []) or "-"
                lines.append(
                    "  Panel %s title=%s type=%s datasources=%s queries=%s"
                    % (
                        str(panel.get("panelId") or ""),
                        str(panel.get("panelTitle") or ""),
                        str(panel.get("panelType") or ""),
                        datasource_text,
                        int(panel.get("queryCount") or 0),
                    )
                )
                for query in list(panel.get("queries") or []):
                    detail_parts = [
                        "datasource=%s" % str(query.get("datasource") or "-"),
                        "field=%s" % str(query.get("queryField") or "-"),
                    ]
                    metrics = format_report_column_value(query, "metrics")
                    measurements = format_report_column_value(query, "measurements")
                    buckets = format_report_column_value(query, "buckets")
                    if metrics:
                        detail_parts.append("metrics=%s" % metrics)
                    if measurements:
                        detail_parts.append("measurements=%s" % measurements)
                    if buckets:
                        detail_parts.append("buckets=%s" % buckets)
                    lines.append(
                        "    Query %s %s"
                        % (
                            str(query.get("refId") or ""),
                            " ".join(detail_parts),
                        )
                    )
                    lines.append("      %s" % str(query.get("query") or ""))
    return lines


def render_export_inspection_tree_tables(
    document: Dict[str, Any],
    import_dir: Path,
    include_header: bool = True,
    selected_columns: Optional[List[str]] = None,
) -> List[str]:
    """Render one grouped report as dashboard-first sections with per-dashboard tables."""
    summary = document.get("summary") or {}
    dashboard_records = list(document.get("dashboards") or [])
    selected_columns = list(selected_columns or REPORT_COLUMN_HEADERS.keys())
    lines = ["Export inspection tree-table report: %s" % import_dir, ""]

    lines.append("# Summary")
    lines.extend(
        render_export_inspection_table_section(
            ["METRIC", "VALUE"],
            [
                ["dashboard_count", str(int(summary.get("dashboardCount") or 0))],
                ["panel_count", str(int(summary.get("panelCount") or 0))],
                ["query_record_count", str(int(summary.get("queryRecordCount") or 0))],
            ],
            include_header=include_header,
        )
    )

    if dashboard_records:
        lines.append("")
        lines.append("# Dashboard sections")
        for index, dashboard in enumerate(dashboard_records, 1):
            lines.append(
                "[%s] Dashboard %s title=%s path=%s panels=%s queries=%s"
                % (
                    index,
                    str(dashboard.get("dashboardUid") or DEFAULT_UNKNOWN_UID),
                    str(dashboard.get("dashboardTitle") or DEFAULT_DASHBOARD_TITLE),
                    str(dashboard.get("folderPath") or DEFAULT_FOLDER_TITLE),
                    int(dashboard.get("panelCount") or 0),
                    int(dashboard.get("queryCount") or 0),
                )
            )
            query_records = []
            for panel in list(dashboard.get("panels") or []):
                for query in list(panel.get("queries") or []):
                    query_records.append(query)
            if query_records:
                lines.extend(
                    render_export_inspection_table_section(
                        [
                            SUPPORTED_REPORT_COLUMN_HEADERS[column_id]
                            for column_id in selected_columns
                        ],
                        [
                            [
                                format_report_column_value(record, column_id)
                                for column_id in selected_columns
                            ]
                            for record in query_records
                        ],
                        include_header=include_header,
                    )
                )
            else:
                lines.append("(no query rows)")
            lines.append("")
        if lines[-1] == "":
            lines.pop()
    return lines
