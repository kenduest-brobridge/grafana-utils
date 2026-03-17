"""Dashboard inspection summary model and render helpers."""

from collections import OrderedDict
from pathlib import Path
from typing import Any

from .common import (
    DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE,
    DEFAULT_UNKNOWN_UID,
)
from .inspection_render import render_export_inspection_table_section
from .inspection_report import (
    describe_export_datasource_ref,
)


def summarize_datasource_inventory_usage(
    datasource: dict[str, str],
    usage_by_label: dict[str, dict[str, Any]],
) -> dict[str, int]:
    """Summarize one datasource inventory row against datasource usage records."""
    labels = []
    uid = str(datasource.get("uid") or "").strip()
    name = str(datasource.get("name") or "").strip()
    if uid:
        labels.append(uid)
    if name and name not in labels:
        labels.append(name)
    reference_count = 0
    dashboards: set[str] = set()
    for label in labels:
        usage = usage_by_label.get(label) or {}
        reference_count += int(usage.get("referenceCount") or 0)
        dashboards.update(usage.get("dashboards") or set())
    return {
        "referenceCount": reference_count,
        "dashboardCount": len(dashboards),
    }


def build_orphaned_datasource_record(record: dict[str, Any]) -> dict[str, str]:
    """Keep one stable orphaned datasource summary record for governance views."""
    return {
        "uid": str(record.get("uid") or ""),
        "name": str(record.get("name") or ""),
        "type": str(record.get("type") or ""),
        "access": str(record.get("access") or ""),
        "url": str(record.get("url") or ""),
        "isDefault": str(record.get("isDefault") or "false"),
        "org": str(record.get("org") or ""),
        "orgId": str(record.get("orgId") or ""),
    }


def build_export_inspection_document(
    import_dir: Path,
    deps: dict[str, Any],
) -> dict[str, Any]:
    """Analyze one raw export directory and summarize dashboard structure."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 18, 42

    metadata = deps["load_export_metadata"](
        import_dir, expected_variant=deps["RAW_EXPORT_SUBDIR"]
    )
    dashboard_files = deps["discover_dashboard_files"](import_dir)
    folder_inventory = deps["load_folder_inventory"](import_dir, metadata)
    datasource_inventory = deps["load_datasource_inventory"](import_dir, metadata)
    datasources_by_uid, datasources_by_name = deps["build_datasource_catalog"](
        datasource_inventory
    )
    folder_lookup = deps["build_folder_inventory_lookup"](folder_inventory)
    folder_paths = OrderedDict()
    datasource_usage: dict[str, dict[str, Any]] = {}
    dashboards: list[dict[str, Any]] = []
    total_panels = 0
    total_queries = 0
    mixed_dashboards = []

    for folder in sorted(folder_inventory, key=lambda item: str(item.get("path") or "")):
        path = str(folder.get("path") or str(folder.get("title") or "")).strip()
        if path:
            folder_paths[path] = 0

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
            folder_record.get("path")
            or folder_record.get("title")
            or DEFAULT_FOLDER_TITLE
        ).strip() or DEFAULT_FOLDER_TITLE
        folder_paths[folder_path] = int(folder_paths.get(folder_path) or 0) + 1

        panels = deps["iter_dashboard_panels"](dashboard.get("panels"))
        panel_count = len(panels)
        query_count = 0
        datasource_refs: list[Any] = []
        deps["collect_datasource_refs"](dashboard, datasource_refs)
        datasource_labels = []
        for ref in datasource_refs:
            label = describe_export_datasource_ref(
                ref,
                datasources_by_uid,
                datasources_by_name,
            )
            if label:
                datasource_labels.append(label)
        unique_datasources = sorted(set(datasource_labels))
        is_mixed = False
        for panel in panels:
            targets = panel.get("targets")
            if isinstance(targets, list):
                query_count += len(
                    [target for target in targets if isinstance(target, dict)]
                )
            panel_datasource = panel.get("datasource")
            if isinstance(panel_datasource, dict) and str(
                panel_datasource.get("uid") or ""
            ) == "-- Mixed --":
                is_mixed = True
        if len(unique_datasources) > 1:
            is_mixed = True

        for label in datasource_labels:
            usage = datasource_usage.setdefault(
                label,
                {"name": label, "referenceCount": 0, "dashboards": set()},
            )
            usage["referenceCount"] = int(usage.get("referenceCount") or 0) + 1
            usage["dashboards"].add(str(dashboard.get("uid") or DEFAULT_UNKNOWN_UID))

        total_panels += panel_count
        total_queries += query_count
        dashboard_record = {
            "uid": str(dashboard.get("uid") or DEFAULT_UNKNOWN_UID),
            "title": str(dashboard.get("title") or DEFAULT_DASHBOARD_TITLE),
            "folderPath": folder_path,
            "panelCount": panel_count,
            "queryCount": query_count,
            "datasources": unique_datasources,
            "mixedDatasource": is_mixed,
            "file": str(dashboard_file),
        }
        dashboards.append(dashboard_record)
        if is_mixed:
            mixed_dashboards.append(
                {
                    "uid": dashboard_record["uid"],
                    "title": dashboard_record["title"],
                    "folderPath": folder_path,
                    "datasources": unique_datasources,
                }
            )

    datasource_records = []
    for label in sorted(datasource_usage):
        usage = datasource_usage[label]
        datasource_records.append(
            {
                "name": label,
                "referenceCount": int(usage.get("referenceCount") or 0),
                "dashboardCount": len(usage.get("dashboards") or []),
            }
        )

    datasource_inventory_records = []
    orphaned_datasource_records = []
    for datasource in sorted(
        datasource_inventory,
        key=lambda item: (
            str(item.get("orgId") or ""),
            str(item.get("name") or ""),
            str(item.get("uid") or ""),
        ),
    ):
        usage = summarize_datasource_inventory_usage(datasource, datasource_usage)
        datasource_inventory_records.append(
            {
                "uid": str(datasource.get("uid") or ""),
                "name": str(datasource.get("name") or ""),
                "type": str(datasource.get("type") or ""),
                "access": str(datasource.get("access") or ""),
                "url": str(datasource.get("url") or ""),
                "isDefault": str(datasource.get("isDefault") or "false"),
                "org": str(datasource.get("org") or ""),
                "orgId": str(datasource.get("orgId") or ""),
                "referenceCount": usage["referenceCount"],
                "dashboardCount": usage["dashboardCount"],
            }
        )
        if usage["referenceCount"] == 0 and usage["dashboardCount"] == 0:
            orphaned_datasource_records.append(
                build_orphaned_datasource_record(datasource_inventory_records[-1])
            )

    folder_records = [
        {"path": path, "dashboardCount": count} for path, count in folder_paths.items()
    ]
    dashboards.sort(key=lambda item: (item["folderPath"], item["title"], item["uid"]))
    mixed_dashboards.sort(
        key=lambda item: (item["folderPath"], item["title"], item["uid"])
    )
    return {
        "summary": {
            "dashboardCount": len(dashboards),
            "folderCount": len(folder_records),
            "panelCount": total_panels,
            "queryCount": total_queries,
            "mixedDatasourceDashboardCount": len(mixed_dashboards),
            "datasourceInventoryCount": len(datasource_inventory_records),
            "orphanedDatasourceCount": len(orphaned_datasource_records),
        },
        "folders": folder_records,
        "datasources": datasource_records,
        "datasourceInventory": datasource_inventory_records,
        "orphanedDatasources": orphaned_datasource_records,
        "mixedDatasourceDashboards": mixed_dashboards,
        "dashboards": dashboards,
    }


def render_export_inspection_summary(
    document: dict[str, Any],
    import_dir: Path,
) -> list[str]:
    """Render a compact human-readable export inspection summary."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    summary = document.get("summary") or {}
    folder_records = list(document.get("folders") or [])
    datasource_records = list(document.get("datasources") or [])
    datasource_inventory = list(document.get("datasourceInventory") or [])
    orphaned_datasources = list(document.get("orphanedDatasources") or [])
    mixed_dashboards = list(document.get("mixedDatasourceDashboards") or [])
    lines = [
        "Export inspection: %s" % import_dir,
        "Dashboards: %s" % int(summary.get("dashboardCount") or 0),
        "Folders: %s" % int(summary.get("folderCount") or 0),
        "Panels: %s" % int(summary.get("panelCount") or 0),
        "Queries: %s" % int(summary.get("queryCount") or 0),
        "Datasource inventory: %s"
        % int(summary.get("datasourceInventoryCount") or 0),
        "Orphaned datasources: %s"
        % int(summary.get("orphanedDatasourceCount") or 0),
        "Mixed datasource dashboards: %s"
        % int(summary.get("mixedDatasourceDashboardCount") or 0),
    ]
    if folder_records:
        lines.append("")
        lines.append("Folder paths:")
        for record in folder_records:
            lines.append(
                "- %s (%s dashboards)"
                % (
                    str(record.get("path") or DEFAULT_FOLDER_TITLE),
                    int(record.get("dashboardCount") or 0),
                )
            )
    if datasource_records:
        lines.append("")
        lines.append("Datasource usage:")
        for record in datasource_records:
            lines.append(
                "- %s (%s refs across %s dashboards)"
                % (
                    str(record.get("name") or ""),
                    int(record.get("referenceCount") or 0),
                    int(record.get("dashboardCount") or 0),
                )
            )
    if datasource_inventory:
        lines.append("")
        lines.append("Datasource inventory:")
        for record in datasource_inventory:
            lines.append(
                "- [%s] %s uid=%s type=%s access=%s url=%s isDefault=%s refs=%s dashboards=%s"
                % (
                    str(record.get("orgId") or ""),
                    str(record.get("name") or ""),
                    str(record.get("uid") or ""),
                    str(record.get("type") or ""),
                    str(record.get("access") or ""),
                    str(record.get("url") or ""),
                    str(record.get("isDefault") or "false"),
                    int(record.get("referenceCount") or 0),
                    int(record.get("dashboardCount") or 0),
                )
            )
    if orphaned_datasources:
        lines.append("")
        lines.append("Orphaned datasources:")
        for record in orphaned_datasources:
            lines.append(
                "- [%s] %s uid=%s type=%s access=%s url=%s isDefault=%s"
                % (
                    str(record.get("orgId") or ""),
                    str(record.get("name") or ""),
                    str(record.get("uid") or ""),
                    str(record.get("type") or ""),
                    str(record.get("access") or ""),
                    str(record.get("url") or ""),
                    str(record.get("isDefault") or "false"),
                )
            )
    if mixed_dashboards:
        lines.append("")
        lines.append("Mixed datasource dashboards:")
        for record in mixed_dashboards:
            lines.append(
                "- %s (%s) path=%s datasources=%s"
                % (
                    str(record.get("title") or ""),
                    str(record.get("uid") or ""),
                    str(record.get("folderPath") or DEFAULT_FOLDER_TITLE),
                    ",".join(record.get("datasources") or []),
                )
            )
    return lines


def render_export_inspection_tables(
    document: dict[str, Any],
    import_dir: Path,
    include_header: bool = True,
) -> list[str]:
    """Render export inspection as multiple compact table sections."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    summary = document.get("summary") or {}
    folder_records = list(document.get("folders") or [])
    datasource_records = list(document.get("datasources") or [])
    datasource_inventory = list(document.get("datasourceInventory") or [])
    orphaned_datasources = list(document.get("orphanedDatasources") or [])
    mixed_dashboards = list(document.get("mixedDatasourceDashboards") or [])
    lines = ["Export inspection: %s" % import_dir, ""]

    lines.append("# Summary")
    lines.extend(
        render_export_inspection_table_section(
            ["METRIC", "VALUE"],
            [
                ["dashboard_count", str(int(summary.get("dashboardCount") or 0))],
                ["folder_count", str(int(summary.get("folderCount") or 0))],
                ["panel_count", str(int(summary.get("panelCount") or 0))],
                ["query_count", str(int(summary.get("queryCount") or 0))],
                [
                    "datasource_inventory_count",
                    str(int(summary.get("datasourceInventoryCount") or 0)),
                ],
                [
                    "orphaned_datasource_count",
                    str(int(summary.get("orphanedDatasourceCount") or 0)),
                ],
                [
                    "mixed_datasource_dashboard_count",
                    str(int(summary.get("mixedDatasourceDashboardCount") or 0)),
                ],
            ],
            include_header=include_header,
        )
    )

    if folder_records:
        lines.append("")
        lines.append("# Folder paths")
        lines.extend(
            render_export_inspection_table_section(
                ["FOLDER_PATH", "DASHBOARDS"],
                [
                    [
                        str(record.get("path") or DEFAULT_FOLDER_TITLE),
                        str(int(record.get("dashboardCount") or 0)),
                    ]
                    for record in folder_records
                ],
                include_header=include_header,
            )
        )

    if datasource_records:
        lines.append("")
        lines.append("# Datasource usage")
        lines.extend(
            render_export_inspection_table_section(
                ["DATASOURCE", "REFS", "DASHBOARDS"],
                [
                    [
                        str(record.get("name") or ""),
                        str(int(record.get("referenceCount") or 0)),
                        str(int(record.get("dashboardCount") or 0)),
                    ]
                    for record in datasource_records
                ],
                include_header=include_header,
            )
        )

    if datasource_inventory:
        lines.append("")
        lines.append("# Datasource inventory")
        lines.extend(
            render_export_inspection_table_section(
                [
                    "ORG_ID",
                    "UID",
                    "NAME",
                    "TYPE",
                    "ACCESS",
                    "URL",
                    "IS_DEFAULT",
                    "REFS",
                    "DASHBOARDS",
                ],
                [
                    [
                        str(record.get("orgId") or ""),
                        str(record.get("uid") or ""),
                        str(record.get("name") or ""),
                        str(record.get("type") or ""),
                        str(record.get("access") or ""),
                        str(record.get("url") or ""),
                        str(record.get("isDefault") or "false"),
                        str(int(record.get("referenceCount") or 0)),
                        str(int(record.get("dashboardCount") or 0)),
                    ]
                    for record in datasource_inventory
                ],
                include_header=include_header,
            )
        )

    if orphaned_datasources:
        lines.append("")
        lines.append("# Orphaned datasources")
        lines.extend(
            render_export_inspection_table_section(
                [
                    "ORG_ID",
                    "UID",
                    "NAME",
                    "TYPE",
                    "ACCESS",
                    "URL",
                    "IS_DEFAULT",
                ],
                [
                    [
                        str(record.get("orgId") or ""),
                        str(record.get("uid") or ""),
                        str(record.get("name") or ""),
                        str(record.get("type") or ""),
                        str(record.get("access") or ""),
                        str(record.get("url") or ""),
                        str(record.get("isDefault") or "false"),
                    ]
                    for record in orphaned_datasources
                ],
                include_header=include_header,
            )
        )

    if mixed_dashboards:
        lines.append("")
        lines.append("# Mixed datasource dashboards")
        lines.extend(
            render_export_inspection_table_section(
                ["UID", "TITLE", "FOLDER_PATH", "DATASOURCES"],
                [
                    [
                        str(record.get("uid") or ""),
                        str(record.get("title") or ""),
                        str(record.get("folderPath") or DEFAULT_FOLDER_TITLE),
                        ",".join(record.get("datasources") or []),
                    ]
                    for record in mixed_dashboards
                ],
                include_header=include_header,
            )
        )
    return lines
