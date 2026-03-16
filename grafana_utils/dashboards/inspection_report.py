"""Dashboard inspection report model and document helpers."""

from collections import OrderedDict
from pathlib import Path
from typing import Any, Optional

from .common import (
    DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE,
    DEFAULT_UNKNOWN_UID,
    GrafanaError,
)
from .inspection_analyzers import build_query_field_and_text, dispatch_query_analysis
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
INSPECT_REPORT_FORMAT_CHOICES = (
    "table",
    "json",
    "csv",
    "tree",
    "tree-table",
    "governance",
    "governance-json",
)
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
INSPECT_EXPORT_HELP_FULL_EXAMPLES = (
    "Extended examples:\n\n"
    "  Inspect one raw export as the default flat query table:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format report-table\n\n"
    "  Inspect one raw export as datasource governance tables:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format governance\n\n"
    "  Inspect one raw export as datasource governance JSON:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format governance-json\n\n"
    "  Inspect one raw export as dashboard-first grouped tables:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format report-tree-table\n\n"
    "  Narrow the report to one datasource and one panel id:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format report-tree-table "
    "--report-filter-datasource prom-main --report-filter-panel-id 7\n\n"
    "  Trim the per-query columns for flat or tree-table output:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format report-tree-table "
    "--report-columns panel_id,panel_title,datasource,query"
)
INSPECT_LIVE_HELP_FULL_EXAMPLES = (
    "Extended examples:\n\n"
    "  Inspect live dashboards as the default flat query table:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --output-format report-table\n\n"
    "  Inspect live dashboards as datasource governance tables:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --output-format governance\n\n"
    "  Inspect live dashboards as datasource governance JSON:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --output-format governance-json\n\n"
    "  Inspect live dashboards as dashboard-first grouped tables:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --output-format report-tree-table\n\n"
    "  Narrow live inspection to one datasource and one panel id:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --output-format report-tree-table --report-filter-datasource prom-main "
    "--report-filter-panel-id 7\n\n"
    "  Trim the per-query columns for flat or tree-table output:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin "
    "--basic-password admin --output-format report-tree-table "
    "--report-columns panel_id,panel_title,datasource,query"
)


def build_export_inspection_report_document(
    import_dir: Path,
    deps: dict[str, Any],
) -> dict[str, Any]:
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
    datasources_by_uid: dict[str, dict[str, str]],
    datasources_by_name: dict[str, dict[str, str]],
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
    panel: dict[str, Any],
    target: dict[str, Any],
    datasources_by_uid: dict[str, dict[str, str]],
    datasources_by_name: dict[str, dict[str, str]],
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
    panel: dict[str, Any],
    target: dict[str, Any],
    datasources_by_name: dict[str, dict[str, str]],
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
def build_query_report_record(
    dashboard: dict[str, Any],
    folder_path: str,
    panel: dict[str, Any],
    target: dict[str, Any],
    dashboard_file: Path,
    datasources_by_uid: dict[str, dict[str, str]],
    datasources_by_name: dict[str, dict[str, str]],
) -> dict[str, Any]:
    """Build one canonical per-query inspection row."""
    query_field, query_text = build_query_field_and_text(target)
    analysis = dispatch_query_analysis(
        panel,
        target,
        query_field,
        query_text,
        datasources_by_uid,
        datasources_by_name,
    )
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


def parse_report_columns(value: Optional[str]) -> Optional[list[str]]:
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
    document: dict[str, Any],
    datasource_label: Optional[str] = None,
    panel_id: Optional[str] = None,
) -> dict[str, Any]:
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


def build_grouped_export_inspection_report_document(
    document: dict[str, Any]
) -> dict[str, Any]:
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


from .inspection_render import (  # noqa: E402
    format_report_column_value,
    render_export_inspection_grouped_report,
    render_export_inspection_report_csv,
    render_export_inspection_report_tables,
    render_export_inspection_table_section,
    render_export_inspection_tree_tables,
)
