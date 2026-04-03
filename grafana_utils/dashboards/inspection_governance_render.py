"""Dashboard inspection governance render helpers."""

from typing import Any, Iterable


def _stringify_cell(value: Any) -> str:
    if isinstance(value, list):
        return ",".join(str(item or "") for item in value if str(item or "").strip())
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value or "")


def _render_table(headers: list[str], rows: list[list[str]]) -> list[str]:
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))
    rendered = [
        "  ".join(headers[index].ljust(widths[index]) for index in range(len(headers)))
    ]
    rendered.append("  ".join("-" * widths[index] for index in range(len(headers))))
    for row in rows:
        rendered.append(
            "  ".join(row[index].ljust(widths[index]) for index in range(len(headers)))
        )
    return rendered


def _render_named_section(
    title: str,
    headers: list[str],
    records: Iterable[dict[str, Any]],
    columns: list[str],
) -> list[str]:
    rows = []
    for record in records:
        rows.append([_stringify_cell(record.get(column)) for column in columns])
    lines = [title]
    if not rows:
        lines.append("(none)")
        return lines
    lines.extend(_render_table(headers, rows))
    return lines


def render_export_inspection_governance_tables(
    document: dict[str, Any], import_dir: str
) -> list[str]:
    """Render one governance document as compact table sections."""
    summary = document.get("summary") or {}
    lines = ["Export inspection governance: %s" % import_dir, ""]
    lines.extend(
        _render_named_section(
            "# Summary",
            ["DASHBOARDS", "QUERIES", "FAMILIES", "DATASOURCES", "RISKS"],
            [
                {
                    "dashboardCount": summary.get("dashboardCount"),
                    "queryRecordCount": summary.get("queryRecordCount"),
                    "datasourceFamilyCount": summary.get("datasourceFamilyCount"),
                    "datasourceCoverageCount": summary.get("datasourceCoverageCount"),
                    "riskRecordCount": summary.get("riskRecordCount"),
                }
            ],
            [
                "dashboardCount",
                "queryRecordCount",
                "datasourceFamilyCount",
                "datasourceCoverageCount",
                "riskRecordCount",
            ],
        )
    )
    lines.append("")
    lines.extend(
        _render_named_section(
            "# Datasource Families",
            ["FAMILY", "TYPES", "DATASOURCES", "DASHBOARDS", "PANELS", "QUERIES"],
            document.get("datasourceFamilies") or [],
            [
                "family",
                "datasourceTypes",
                "datasourceCount",
                "dashboardCount",
                "panelCount",
                "queryCount",
            ],
        )
    )
    lines.append("")
    lines.extend(
        _render_named_section(
            "# Datasources",
            ["UID", "DATASOURCE", "FAMILY", "QUERIES", "DASHBOARDS", "ORPHANED"],
            document.get("datasources") or [],
            [
                "datasourceUid",
                "datasource",
                "family",
                "queryCount",
                "dashboardCount",
                "orphaned",
            ],
        )
    )
    lines.append("")
    lines.extend(
        _render_named_section(
            "# Risks",
            [
                "SEVERITY",
                "CATEGORY",
                "KIND",
                "DASHBOARD_UID",
                "PANEL_ID",
                "DATASOURCE",
                "DETAIL",
                "RECOMMENDATION",
            ],
            document.get("riskRecords") or [],
            [
                "severity",
                "category",
                "kind",
                "dashboardUid",
                "panelId",
                "datasource",
                "detail",
                "recommendation",
            ],
        )
    )
    return lines
