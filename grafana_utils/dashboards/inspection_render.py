"""Dashboard inspection report render helpers."""

import csv
import io
import re
from pathlib import Path
from typing import Any, Optional

from .common import (
    DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE,
    DEFAULT_UNKNOWN_UID,
)
from .inspection_report import (
    REPORT_COLUMN_ALIASES,
    REPORT_COLUMN_HEADERS,
    SUPPORTED_REPORT_COLUMN_HEADERS,
)


def format_report_column_value(record: dict[str, Any], column_id: str) -> str:
    """Format one report cell from the canonical inspection row model."""
    value = record.get(column_id)
    if isinstance(value, list):
        return ",".join(str(item) for item in value)
    return str(value or "")


def render_export_inspection_report_csv(
    document: dict[str, Any],
    selected_columns: Optional[list[str]] = None,
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
    headers: list[str],
    rows: list[list[str]],
    include_header: bool = True,
) -> list[str]:
    """Render one simple left-aligned table section."""
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: list[str]) -> str:
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
    document: dict[str, Any],
    import_dir: Path,
    include_header: bool = True,
    selected_columns: Optional[list[str]] = None,
) -> list[str]:
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


def render_export_inspection_grouped_report(
    document: dict[str, Any],
    import_dir: Path,
) -> list[str]:
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
    document: dict[str, Any],
    import_dir: Path,
    include_header: bool = True,
    selected_columns: Optional[list[str]] = None,
) -> list[str]:
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
