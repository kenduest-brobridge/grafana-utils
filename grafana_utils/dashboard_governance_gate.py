"""Dashboard governance gate driven by inspect JSON artifacts."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


SQL_FAMILIES = {"mysql", "postgres", "mssql", "sql"}
SQL_TIME_FILTER_PATTERNS = (
    "$__timefilter(",
    "$__timefilter(",
    "$__unixepochfilter(",
    "$timefilter",
)
LOKI_BROAD_QUERY_PATTERNS = (
    '=~".*"',
    '=~".+"',
    '|~".*"',
    '|~".+"',
    "{}",
)


def _load_json_document(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise ValueError("JSON input not found: %s" % path) from error
    except json.JSONDecodeError as error:
        raise ValueError("Failed to parse JSON from %s: %s" % (path, error)) from error
    if not isinstance(data, dict):
        raise ValueError("JSON document at %s must be an object." % path)
    return data


def _normalize_string_set(values: Any) -> set[str]:
    normalized = set()
    for value in list(values or []):
        text = str(value or "").strip()
        if text:
            normalized.add(text)
    return normalized


def _normalize_bool(value: Any, default: bool = False) -> bool:
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    text = str(value).strip().lower()
    if text in {"1", "true", "yes", "on"}:
        return True
    if text in {"0", "false", "no", "off"}:
        return False
    return default


def _normalize_optional_int(value: Any) -> int | None:
    if value is None or value == "":
        return None
    return int(value)


def _dashboard_key(query: dict[str, Any]) -> tuple[str, str]:
    return (
        str(query.get("dashboardUid") or "").strip(),
        str(query.get("dashboardTitle") or "").strip(),
    )


def _panel_key(query: dict[str, Any]) -> tuple[str, str, str, str]:
    return (
        str(query.get("dashboardUid") or "").strip(),
        str(query.get("dashboardTitle") or "").strip(),
        str(query.get("panelId") or "").strip(),
        str(query.get("panelTitle") or "").strip(),
    )


def _build_finding(
    severity: str,
    code: str,
    message: str,
    query: dict[str, Any] | None = None,
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    record = {
        "severity": severity,
        "code": code,
        "message": message,
        "dashboardUid": "",
        "dashboardTitle": "",
        "panelId": "",
        "panelTitle": "",
        "refId": "",
        "datasource": "",
        "datasourceUid": "",
        "datasourceFamily": "",
    }
    if query:
        record.update(
            {
                "dashboardUid": str(query.get("dashboardUid") or ""),
                "dashboardTitle": str(query.get("dashboardTitle") or ""),
                "panelId": str(query.get("panelId") or ""),
                "panelTitle": str(query.get("panelTitle") or ""),
                "refId": str(query.get("refId") or ""),
                "datasource": str(query.get("datasource") or ""),
                "datasourceUid": str(query.get("datasourceUid") or ""),
                "datasourceFamily": str(query.get("datasourceFamily") or ""),
            }
        )
    if extra:
        record.update(extra)
    return record


def _query_family(query: dict[str, Any]) -> str:
    return str(query.get("datasourceFamily") or query.get("datasourceType") or "").strip()


def _query_text(query: dict[str, Any]) -> str:
    return str(query.get("query") or "").strip()


def _is_sql_query(query: dict[str, Any]) -> bool:
    return _query_family(query).lower() in SQL_FAMILIES


def _query_uses_time_filter(query: dict[str, Any]) -> bool:
    lowered = _query_text(query).lower()
    return any(pattern in lowered for pattern in SQL_TIME_FILTER_PATTERNS)


def _is_loki_broad_query(query: dict[str, Any]) -> bool:
    if _query_family(query).lower() != "loki":
        return False
    lowered = _query_text(query).lower()
    return any(pattern in lowered for pattern in LOKI_BROAD_QUERY_PATTERNS)


def _governance_risk_kinds(governance_document: dict[str, Any]) -> set[str]:
    kinds = set()
    for record in governance_document.get("riskRecords") or []:
        if not isinstance(record, dict):
            continue
        kind = str(record.get("kind") or "").strip()
        if kind:
            kinds.add(kind)
    return kinds


def evaluate_dashboard_governance_policy(
    policy_document: dict[str, Any],
    governance_document: dict[str, Any],
    query_document: dict[str, Any],
) -> dict[str, Any]:
    version = int(policy_document.get("version") or 1)
    if version != 1:
        raise ValueError("Unsupported dashboard governance policy version: %s" % version)

    datasource_policy = dict(policy_document.get("datasources") or {})
    query_policy = dict(policy_document.get("queries") or {})
    enforcement_policy = dict(policy_document.get("enforcement") or {})

    allowed_families = _normalize_string_set(datasource_policy.get("allowedFamilies"))
    allowed_uids = _normalize_string_set(datasource_policy.get("allowedUids"))
    forbid_unknown = _normalize_bool(datasource_policy.get("forbidUnknown"), default=False)
    forbid_mixed_families = _normalize_bool(
        datasource_policy.get("forbidMixedFamilies"), default=False
    )
    max_queries_per_dashboard = _normalize_optional_int(
        query_policy.get("maxQueriesPerDashboard")
    )
    max_queries_per_panel = _normalize_optional_int(query_policy.get("maxQueriesPerPanel"))
    forbid_select_star = _normalize_bool(query_policy.get("forbidSelectStar"), default=False)
    require_sql_time_filter = _normalize_bool(
        query_policy.get("requireSqlTimeFilter"), default=False
    )
    forbid_broad_loki_regex = _normalize_bool(
        query_policy.get("forbidBroadLokiRegex"), default=False
    )
    fail_on_warnings = _normalize_bool(enforcement_policy.get("failOnWarnings"), default=False)

    queries = [
        query
        for query in list(query_document.get("queries") or [])
        if isinstance(query, dict)
    ]
    governance_risk_kinds = _governance_risk_kinds(governance_document)

    violations = []
    warnings = []

    dashboard_counts = {}
    panel_counts = {}
    for query in queries:
        dashboard_counts[_dashboard_key(query)] = (
            int(dashboard_counts.get(_dashboard_key(query), 0)) + 1
        )
        panel_counts[_panel_key(query)] = int(panel_counts.get(_panel_key(query), 0)) + 1

        family = _query_family(query)
        datasource_uid = str(query.get("datasourceUid") or "").strip()
        query_text = _query_text(query)

        if forbid_unknown and (
            not family
            or family.lower() == "unknown"
            or not str(query.get("datasource") or "").strip()
        ):
            violations.append(
                _build_finding(
                    "error",
                    "DATASOURCE_UNKNOWN",
                    "Datasource identity could not be resolved for this query row.",
                    query,
                )
            )

        if allowed_families and family not in allowed_families:
            violations.append(
                _build_finding(
                    "error",
                    "DATASOURCE_FAMILY_NOT_ALLOWED",
                    "Datasource family %s is not allowed by policy." % (family or "unknown"),
                    query,
                )
            )

        if allowed_uids and datasource_uid and datasource_uid not in allowed_uids:
            violations.append(
                _build_finding(
                    "error",
                    "DATASOURCE_UID_NOT_ALLOWED",
                    "Datasource uid %s is not allowed by policy." % datasource_uid,
                    query,
                )
            )

        if forbid_select_star and _is_sql_query(query) and re.search(
            r"\bselect\s+\*", query_text, flags=re.IGNORECASE
        ):
            violations.append(
                _build_finding(
                    "error",
                    "SQL_SELECT_STAR",
                    "SQL query uses SELECT * and violates the policy.",
                    query,
                )
            )

        if require_sql_time_filter and _is_sql_query(query) and not _query_uses_time_filter(query):
            violations.append(
                _build_finding(
                    "error",
                    "SQL_MISSING_TIME_FILTER",
                    "SQL query does not include a Grafana time filter macro.",
                    query,
                )
            )

        if forbid_broad_loki_regex and _is_loki_broad_query(query):
            violations.append(
                _build_finding(
                    "error",
                    "LOKI_BROAD_REGEX",
                    "Loki query contains a broad match or empty selector.",
                    query,
                )
            )

    if max_queries_per_dashboard is not None:
        for key, query_count in sorted(dashboard_counts.items()):
            if query_count <= max_queries_per_dashboard:
                continue
            dashboard_uid, dashboard_title = key
            violations.append(
                _build_finding(
                    "error",
                    "QUERY_COUNT_TOO_HIGH",
                    "Dashboard query count %s exceeds policy maxQueriesPerDashboard=%s."
                    % (query_count, max_queries_per_dashboard),
                    extra={
                        "dashboardUid": dashboard_uid,
                        "dashboardTitle": dashboard_title,
                        "queryCount": query_count,
                    },
                )
            )

    if max_queries_per_panel is not None:
        for key, query_count in sorted(panel_counts.items()):
            if query_count <= max_queries_per_panel:
                continue
            dashboard_uid, dashboard_title, panel_id, panel_title = key
            violations.append(
                _build_finding(
                    "error",
                    "PANEL_QUERY_COUNT_TOO_HIGH",
                    "Panel query count %s exceeds policy maxQueriesPerPanel=%s."
                    % (query_count, max_queries_per_panel),
                    extra={
                        "dashboardUid": dashboard_uid,
                        "dashboardTitle": dashboard_title,
                        "panelId": panel_id,
                        "panelTitle": panel_title,
                        "queryCount": query_count,
                    },
                )
            )

    if forbid_mixed_families and "mixed-datasource-dashboard" in governance_risk_kinds:
        for record in governance_document.get("riskRecords") or []:
            if not isinstance(record, dict):
                continue
            if str(record.get("kind") or "").strip() != "mixed-datasource-dashboard":
                continue
            violations.append(
                _build_finding(
                    "error",
                    "MIXED_DATASOURCE_DASHBOARD",
                    "Dashboard mixes multiple datasources and violates policy.",
                    extra={
                        "dashboardUid": str(record.get("dashboardUid") or ""),
                        "panelId": str(record.get("panelId") or ""),
                        "datasource": str(record.get("datasource") or ""),
                    },
                )
            )

    for record in governance_document.get("riskRecords") or []:
        if not isinstance(record, dict):
            continue
        warnings.append(
            {
                "severity": "warning",
                "code": "GOVERNANCE_RISK",
                "message": str(record.get("recommendation") or str(record.get("detail") or "")).strip(),
                "riskKind": str(record.get("kind") or ""),
                "dashboardUid": str(record.get("dashboardUid") or ""),
                "panelId": str(record.get("panelId") or ""),
                "datasource": str(record.get("datasource") or ""),
            }
        )

    ok = not violations and not (fail_on_warnings and warnings)
    return {
        "ok": ok,
        "summary": {
            "dashboardCount": int(
                (query_document.get("summary") or {}).get("dashboardCount") or 0
            ),
            "queryRecordCount": int(
                (query_document.get("summary") or {}).get("queryRecordCount") or 0
            ),
            "violationCount": len(violations),
            "warningCount": len(warnings),
            "checkedRules": {
                "datasourceAllowedFamilies": sorted(allowed_families),
                "datasourceAllowedUids": sorted(allowed_uids),
                "forbidUnknown": forbid_unknown,
                "forbidMixedFamilies": forbid_mixed_families,
                "maxQueriesPerDashboard": max_queries_per_dashboard,
                "maxQueriesPerPanel": max_queries_per_panel,
                "forbidSelectStar": forbid_select_star,
                "requireSqlTimeFilter": require_sql_time_filter,
                "forbidBroadLokiRegex": forbid_broad_loki_regex,
                "failOnWarnings": fail_on_warnings,
            },
        },
        "violations": violations,
        "warnings": warnings,
    }


def render_dashboard_governance_check(result: dict[str, Any]) -> str:
    lines = [
        "Dashboard governance check: %s" % ("PASS" if result.get("ok") else "FAIL"),
        "Dashboards: %(dashboardCount)s  Queries: %(queryRecordCount)s  Violations: %(violationCount)s  Warnings: %(warningCount)s"
        % dict(result.get("summary") or {}),
    ]
    violations = list(result.get("violations") or [])
    warnings = list(result.get("warnings") or [])
    if violations:
        lines.append("")
        lines.append("Violations:")
        for record in violations:
            location = "dashboard=%s panel=%s ref=%s" % (
                str(record.get("dashboardUid") or "-"),
                str(record.get("panelId") or "-"),
                str(record.get("refId") or "-"),
            )
            lines.append(
                "  ERROR [%s] %s datasource=%s: %s"
                % (
                    str(record.get("code") or ""),
                    location,
                    str(record.get("datasourceUid") or record.get("datasource") or "-"),
                    str(record.get("message") or ""),
                )
            )
    if warnings:
        lines.append("")
        lines.append("Warnings:")
        for record in warnings:
            lines.append(
                "  WARN [%s] dashboard=%s panel=%s datasource=%s: %s"
                % (
                    str(record.get("riskKind") or record.get("code") or ""),
                    str(record.get("dashboardUid") or "-"),
                    str(record.get("panelId") or "-"),
                    str(record.get("datasource") or "-"),
                    str(record.get("message") or ""),
                )
            )
    return "\n".join(lines)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Evaluate dashboard governance policy rules against inspect-export JSON artifacts."
        )
    )
    parser.add_argument("--policy", required=True, help="Path to the governance policy JSON.")
    parser.add_argument(
        "--governance",
        required=True,
        help="Path to dashboard inspect governance-json output.",
    )
    parser.add_argument(
        "--queries",
        required=True,
        help="Path to dashboard inspect report json output.",
    )
    parser.add_argument(
        "--output-format",
        choices=("text", "json"),
        default="text",
        help="Render the gate result as text or JSON (default: text).",
    )
    parser.add_argument(
        "--json-output",
        default=None,
        help="Optional path to also write the normalized gate result JSON.",
    )
    return parser


def run_dashboard_governance_gate(args: argparse.Namespace) -> int:
    policy_document = _load_json_document(Path(args.policy))
    governance_document = _load_json_document(Path(args.governance))
    query_document = _load_json_document(Path(args.queries))
    result = evaluate_dashboard_governance_policy(
        policy_document,
        governance_document,
        query_document,
    )
    if args.json_output:
        Path(args.json_output).write_text(
            json.dumps(result, indent=2, sort_keys=False, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
    if args.output_format == "json":
        print(json.dumps(result, indent=2, sort_keys=False, ensure_ascii=False))
    else:
        print(render_dashboard_governance_check(result))
    return 0 if result.get("ok") else 1


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return run_dashboard_governance_gate(args)
    except (ValueError, OSError) as error:
        parser.exit(2, "error: %s\n" % error)


if __name__ == "__main__":
    sys.exit(main())
