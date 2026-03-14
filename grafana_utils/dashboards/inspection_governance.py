"""Dashboard inspection governance document helpers."""

from typing import Any, Iterable, Optional


def _unique_strings(values: Iterable[Any]) -> list[str]:
    seen = set()
    normalized = []
    for value in values:
        text = str(value or "").strip()
        if not text or text in seen:
            continue
        seen.add(text)
        normalized.append(text)
    return normalized


def _resolve_datasource_inventory(
    summary_document: dict[str, Any]
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    by_uid = {}
    by_name = {}
    for item in summary_document.get("datasourceInventory") or []:
        if not isinstance(item, dict):
            continue
        record = dict(item)
        uid = str(record.get("uid") or "").strip()
        name = str(record.get("name") or "").strip()
        if uid:
            by_uid[uid] = record
        if name:
            by_name[name] = record
    return by_uid, by_name


def _resolve_datasource_identity(
    query_record: dict[str, Any],
    datasource_by_uid: dict[str, dict[str, Any]],
    datasource_by_name: dict[str, dict[str, Any]],
) -> tuple[str, str, str]:
    datasource_uid = str(query_record.get("datasourceUid") or "").strip()
    datasource_label = str(query_record.get("datasource") or "").strip()
    inventory = None
    if datasource_uid:
        inventory = datasource_by_uid.get(datasource_uid)
    if inventory is None and datasource_label:
        inventory = datasource_by_uid.get(datasource_label) or datasource_by_name.get(
            datasource_label
        )
    if inventory is not None:
        return (
            str(inventory.get("uid") or datasource_uid or datasource_label),
            str(inventory.get("name") or datasource_label or datasource_uid),
            str(inventory.get("type") or "unknown"),
        )
    if datasource_uid:
        return datasource_uid, datasource_label or datasource_uid, "unknown"
    if datasource_label:
        return datasource_label, datasource_label, "unknown"
    return "unknown", "unknown", "unknown"


def _normalize_family_name(datasource_type: str) -> str:
    name = str(datasource_type or "").strip().lower()
    if not name:
        return "unknown"
    aliases = {
        "grafana-postgresql-datasource": "postgres",
        "grafana-mysql-datasource": "mysql",
        "influxdb": "influxdb",
        "loki": "loki",
        "prometheus": "prometheus",
        "postgres": "postgres",
    }
    return aliases.get(name, name)


def _build_query_analysis_state(record: dict[str, Any]) -> str:
    for field in ("metrics", "measurements", "buckets"):
        values = record.get(field)
        if isinstance(values, list) and values:
            return "ok"
    return "empty"


def _build_governance_risk_record(
    kind: str,
    severity: str,
    dashboard_uid: str,
    panel_id: str,
    datasource: str,
    detail: str,
) -> dict[str, str]:
    category = "coverage"
    recommendation = "Review this governance finding."
    if kind == "mixed-datasource-dashboard":
        category = "topology"
        recommendation = (
            "Split panel queries by datasource or document why this mixed datasource "
            "dashboard must stay combined."
        )
    elif kind == "orphaned-datasource":
        category = "inventory"
        recommendation = (
            "Remove the unused datasource or attach it to retained dashboards."
        )
    elif kind == "unknown-datasource-family":
        category = "coverage"
        recommendation = (
            "Normalize the datasource type mapping or add analyzer support for this "
            "plugin family."
        )
    elif kind == "empty-query-analysis":
        category = "coverage"
        recommendation = (
            "Inspect the query text and extend analyzer coverage if this datasource "
            "should emit metrics."
        )
    return {
        "kind": kind,
        "severity": severity,
        "category": category,
        "recommendation": recommendation,
        "dashboardUid": dashboard_uid,
        "panelId": panel_id,
        "datasource": datasource,
        "detail": detail,
    }


def build_datasource_family_coverage_records(
    summary_document: dict[str, Any],
    report_document: dict[str, Any],
) -> list[dict[str, Any]]:
    datasource_by_uid, datasource_by_name = _resolve_datasource_inventory(
        summary_document
    )
    coverage = {}
    for query in report_document.get("queries") or []:
        if not isinstance(query, dict):
            continue
        datasource_uid, datasource_name, datasource_type = _resolve_datasource_identity(
            query,
            datasource_by_uid,
            datasource_by_name,
        )
        family = _normalize_family_name(datasource_type)
        record = coverage.setdefault(
            family,
            {
                "family": family,
                "datasourceTypes": set(),
                "datasourceUids": set(),
                "datasourceNames": set(),
                "dashboardUids": set(),
                "panelKeys": set(),
                "queryCount": 0,
            },
        )
        record["datasourceTypes"].add(datasource_type)
        record["datasourceUids"].add(datasource_uid)
        record["datasourceNames"].add(datasource_name)
        record["dashboardUids"].add(str(query.get("dashboardUid") or ""))
        record["panelKeys"].add(
            "%s:%s" % (str(query.get("dashboardUid") or ""), str(query.get("panelId") or ""))
        )
        record["queryCount"] = int(record.get("queryCount") or 0) + 1

    rows = []
    for family in sorted(coverage):
        record = coverage[family]
        rows.append(
            {
                "family": family,
                "datasourceTypes": _unique_strings(record["datasourceTypes"]),
                "datasourceCount": len(record["datasourceUids"]),
                "dashboardCount": len([item for item in record["dashboardUids"] if item]),
                "panelCount": len([item for item in record["panelKeys"] if item != ":"]),
                "queryCount": int(record["queryCount"] or 0),
            }
        )
    return rows


def build_datasource_coverage_records(
    summary_document: dict[str, Any],
    report_document: dict[str, Any],
) -> list[dict[str, Any]]:
    datasource_by_uid, datasource_by_name = _resolve_datasource_inventory(
        summary_document
    )
    coverage = {}

    for datasource in summary_document.get("datasourceInventory") or []:
        if not isinstance(datasource, dict):
            continue
        uid = str(datasource.get("uid") or "").strip()
        name = str(datasource.get("name") or "").strip()
        key = uid or name or "unknown"
        coverage[key] = {
            "datasourceUid": uid or key,
            "datasource": name or uid or key,
            "family": _normalize_family_name(str(datasource.get("type") or "")),
            "queryFields": set(),
            "dashboardUids": set(),
            "panelKeys": set(),
            "queryCount": 0,
            "orphaned": int(datasource.get("referenceCount") or 0) == 0,
        }

    for query in report_document.get("queries") or []:
        if not isinstance(query, dict):
            continue
        datasource_uid, datasource_name, datasource_type = _resolve_datasource_identity(
            query,
            datasource_by_uid,
            datasource_by_name,
        )
        key = datasource_uid or datasource_name or "unknown"
        record = coverage.setdefault(
            key,
            {
                "datasourceUid": datasource_uid or key,
                "datasource": datasource_name or datasource_uid or key,
                "family": _normalize_family_name(datasource_type),
                "queryFields": set(),
                "dashboardUids": set(),
                "panelKeys": set(),
                "queryCount": 0,
                "orphaned": False,
            },
        )
        record["queryFields"].add(str(query.get("queryField") or ""))
        record["dashboardUids"].add(str(query.get("dashboardUid") or ""))
        record["panelKeys"].add(
            "%s:%s" % (str(query.get("dashboardUid") or ""), str(query.get("panelId") or ""))
        )
        record["queryCount"] = int(record.get("queryCount") or 0) + 1
        record["orphaned"] = False

    rows = []
    for key in sorted(coverage):
        record = coverage[key]
        rows.append(
            {
                "datasourceUid": record["datasourceUid"],
                "datasource": record["datasource"],
                "family": record["family"],
                "queryCount": int(record["queryCount"] or 0),
                "dashboardCount": len([item for item in record["dashboardUids"] if item]),
                "panelCount": len([item for item in record["panelKeys"] if item != ":"]),
                "queryFields": _unique_strings(record["queryFields"]),
                "orphaned": bool(record.get("orphaned")),
            }
        )
    return rows


def build_governance_risk_records(
    summary_document: dict[str, Any],
    report_document: dict[str, Any],
) -> list[dict[str, str]]:
    datasource_by_uid, datasource_by_name = _resolve_datasource_inventory(
        summary_document
    )
    records = []
    seen = set()

    for dashboard in summary_document.get("mixedDatasourceDashboards") or []:
        if not isinstance(dashboard, dict):
            continue
        record = _build_governance_risk_record(
            "mixed-datasource-dashboard",
            "medium",
            str(dashboard.get("uid") or ""),
            "",
            ",".join(_unique_strings(dashboard.get("datasources") or [])),
            str(dashboard.get("title") or ""),
        )
        key = tuple(record.items())
        if key not in seen:
            seen.add(key)
            records.append(record)

    for datasource in summary_document.get("orphanedDatasources") or []:
        if not isinstance(datasource, dict):
            continue
        record = _build_governance_risk_record(
            "orphaned-datasource",
            "low",
            "",
            "",
            str(datasource.get("uid") or datasource.get("name") or "unknown"),
            str(datasource.get("type") or ""),
        )
        key = tuple(record.items())
        if key not in seen:
            seen.add(key)
            records.append(record)

    for query in report_document.get("queries") or []:
        if not isinstance(query, dict):
            continue
        datasource_uid, datasource_name, datasource_type = _resolve_datasource_identity(
            query,
            datasource_by_uid,
            datasource_by_name,
        )
        if _normalize_family_name(datasource_type) == "unknown":
            record = _build_governance_risk_record(
                "unknown-datasource-family",
                "medium",
                str(query.get("dashboardUid") or ""),
                str(query.get("panelId") or ""),
                datasource_name or datasource_uid,
                str(query.get("queryField") or ""),
            )
            key = tuple(record.items())
            if key not in seen:
                seen.add(key)
                records.append(record)
        if _build_query_analysis_state(query) == "empty":
            record = _build_governance_risk_record(
                "empty-query-analysis",
                "low",
                str(query.get("dashboardUid") or ""),
                str(query.get("panelId") or ""),
                datasource_name or datasource_uid,
                str(query.get("queryField") or ""),
            )
            key = tuple(record.items())
            if key not in seen:
                seen.add(key)
                records.append(record)

    records.sort(
        key=lambda item: (
            item["severity"],
            item["kind"],
            item["dashboardUid"],
            item["panelId"],
            item["datasource"],
        )
    )
    return records


def build_export_inspection_governance_document(
    summary_document: dict[str, Any],
    report_document: dict[str, Any],
) -> dict[str, Any]:
    family_records = build_datasource_family_coverage_records(
        summary_document, report_document
    )
    datasource_records = build_datasource_coverage_records(
        summary_document, report_document
    )
    risk_records = build_governance_risk_records(summary_document, report_document)
    summary = summary_document.get("summary") or {}
    report_summary = report_document.get("summary") or {}
    return {
        "summary": {
            "dashboardCount": int(summary.get("dashboardCount") or 0),
            "queryRecordCount": int(report_summary.get("queryRecordCount") or 0),
            "datasourceInventoryCount": int(summary.get("datasourceInventoryCount") or 0),
            "datasourceFamilyCount": len(family_records),
            "datasourceCoverageCount": len(datasource_records),
            "mixedDatasourceDashboardCount": int(
                summary.get("mixedDatasourceDashboardCount") or 0
            ),
            "orphanedDatasourceCount": int(summary.get("orphanedDatasourceCount") or 0),
            "riskRecordCount": len(risk_records),
        },
        "datasourceFamilies": family_records,
        "datasources": datasource_records,
        "riskRecords": risk_records,
    }
