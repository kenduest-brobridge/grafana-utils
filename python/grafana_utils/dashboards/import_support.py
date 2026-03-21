"""Dashboard import and diff helper functions."""

import copy
import difflib
import json
from dataclasses import dataclass
from collections import OrderedDict
from pathlib import Path
from typing import Any, Optional

from .common import GrafanaApiError, GrafanaError
from .export_inventory import (
    load_export_metadata as load_export_metadata_from_export,
    validate_export_metadata as validate_export_metadata_from_export,
)
from .transformer import (
    build_preserved_web_import_document,
    collect_datasource_refs,
    collect_panel_types,
    is_builtin_datasource_ref,
    lookup_datasource,
)


IMPORT_DRY_RUN_COLUMN_HEADERS = OrderedDict(
    [
        ("uid", "UID"),
        ("destination", "DESTINATION"),
        ("action", "ACTION"),
        ("folderPath", "FOLDER_PATH"),
        ("sourceFolderPath", "SOURCE_FOLDER_PATH"),
        ("destinationFolderPath", "DESTINATION_FOLDER_PATH"),
        ("reason", "REASON"),
        ("file", "FILE"),
    ]
)
IMPORT_DRY_RUN_COLUMN_ALIASES = {
    "uid": "uid",
    "destination": "destination",
    "action": "action",
    "folder_path": "folderPath",
    "source_folder_path": "sourceFolderPath",
    "destination_folder_path": "destinationFolderPath",
    "reason": "reason",
    "file": "file",
}


@dataclass(frozen=True)
class DashboardImportDependencyRecord:
    """One dashboard import dependency snapshot collected from disk."""

    file: str
    dashboard_uid: str
    dashboard_title: str
    datasource_refs: tuple[Any, ...]
    panel_types: tuple[str, ...]
    alert_datasource_uids: tuple[str, ...]
    alert_datasource_names: tuple[str, ...]
    alert_contact_points: tuple[str, ...]


def _normalize_dependency_text(value, default=""):
    """Internal helper for normalize dependency text."""
    if value is None:
        return default
    text = str(value).strip()
    if text:
        return text
    return default


def _is_builtin_import_datasource_ref(value):
    """Internal helper for raw-import builtin datasource refs."""
    if isinstance(value, str):
        return is_builtin_datasource_ref(value)
    if isinstance(value, dict):
        uid = value.get("uid")
        name = value.get("name")
        if isinstance(uid, str) and is_builtin_datasource_ref(uid):
            return True
        if isinstance(name, str) and is_builtin_datasource_ref(name):
            return True
    return False


def _collect_dashboard_alert_dependency_refs(
    node,
    alert_datasource_uids,
    alert_datasource_names,
    alert_contact_points,
):
    """Internal helper for collect alert dependency refs."""
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "datasourceUid":
                datasource_uid = _normalize_dependency_text(value)
                if datasource_uid and not is_builtin_datasource_ref(datasource_uid):
                    alert_datasource_uids.add(datasource_uid)
            elif key == "datasourceUids" and isinstance(value, (list, tuple, set)):
                for item in value:
                    datasource_uid = _normalize_dependency_text(item)
                    if datasource_uid and not is_builtin_datasource_ref(
                        datasource_uid
                    ):
                        alert_datasource_uids.add(datasource_uid)
            elif key == "datasourceName":
                datasource_name = _normalize_dependency_text(value)
                if datasource_name:
                    alert_datasource_names.add(datasource_name)
            elif key == "datasourceNames" and isinstance(value, (list, tuple, set)):
                for item in value:
                    datasource_name = _normalize_dependency_text(item)
                    if datasource_name:
                        alert_datasource_names.add(datasource_name)
            elif key == "contactPoints" and isinstance(value, (list, tuple, set)):
                for item in value:
                    contact_point = _normalize_dependency_text(item)
                    if contact_point:
                        alert_contact_points.add(contact_point)
            elif key == "receiver":
                receiver = _normalize_dependency_text(value)
                if receiver:
                    alert_contact_points.add(receiver)
            elif key == "notificationSettings" and isinstance(value, dict):
                receiver = _normalize_dependency_text(value.get("receiver"))
                if receiver:
                    alert_contact_points.add(receiver)
            _collect_dashboard_alert_dependency_refs(
                value,
                alert_datasource_uids,
                alert_datasource_names,
                alert_contact_points,
            )
    elif isinstance(node, list):
        for item in node:
            _collect_dashboard_alert_dependency_refs(
                item,
                alert_datasource_uids,
                alert_datasource_names,
                alert_contact_points,
            )


def collect_dashboard_import_dependency_records(
    dashboard_documents: list[tuple[Path, dict[str, Any]]],
) -> list[DashboardImportDependencyRecord]:
    """Collect dependency signals from raw dashboard import documents."""
    records = []
    for dashboard_file, document in dashboard_documents:
        dashboard = extract_dashboard_object(
            document, "Dashboard payload must be a JSON object."
        )
        datasource_refs = []
        collect_datasource_refs(dashboard, datasource_refs)
        normalized_datasource_refs = []
        seen_datasource_refs = set()
        for ref in datasource_refs:
            if ref is None or _is_builtin_import_datasource_ref(ref):
                continue
            key = json.dumps(ref, sort_keys=True, ensure_ascii=False, default=str)
            if key in seen_datasource_refs:
                continue
            seen_datasource_refs.add(key)
            normalized_datasource_refs.append(ref)

        panel_types = set()
        panels = dashboard.get("panels")
        if isinstance(panels, list):
            collect_panel_types(
                [item for item in panels if isinstance(item, dict)],
                panel_types,
            )
        panel_types.discard("row")

        alert_datasource_uids = set()
        alert_datasource_names = set()
        alert_contact_points = set()
        _collect_dashboard_alert_dependency_refs(
            dashboard,
            alert_datasource_uids,
            alert_datasource_names,
            alert_contact_points,
        )
        if (
            not normalized_datasource_refs
            and not panel_types
            and not alert_datasource_uids
            and not alert_datasource_names
            and not alert_contact_points
        ):
            continue
        records.append(
            DashboardImportDependencyRecord(
                file=str(dashboard_file),
                dashboard_uid=_normalize_dependency_text(dashboard.get("uid")),
                dashboard_title=_normalize_dependency_text(dashboard.get("title")),
                datasource_refs=tuple(normalized_datasource_refs),
                panel_types=tuple(sorted(panel_types)),
                alert_datasource_uids=tuple(sorted(alert_datasource_uids)),
                alert_datasource_names=tuple(sorted(alert_datasource_names)),
                alert_contact_points=tuple(sorted(alert_contact_points)),
            )
        )
    return records


def fetch_dashboard_import_dependency_availability(client):
    """Fetch live datasource, plugin, and contact-point availability from Grafana."""
    availability = {
        "datasourceUids": [],
        "datasourceNames": [],
        "pluginIds": [],
        "contactPoints": [],
    }
    for datasource in client.list_datasources():
        if not isinstance(datasource, dict):
            continue
        uid = _normalize_dependency_text(datasource.get("uid"))
        name = _normalize_dependency_text(datasource.get("name"))
        if uid:
            availability["datasourceUids"].append(uid)
        if name:
            availability["datasourceNames"].append(name)

    plugins = client.request_json("/api/plugins")
    if not isinstance(plugins, list):
        raise GrafanaError("Unexpected plugin list response from Grafana.")
    for plugin in plugins:
        if not isinstance(plugin, dict):
            continue
        plugin_id = _normalize_dependency_text(plugin.get("id"))
        if plugin_id:
            availability["pluginIds"].append(plugin_id)

    contact_points = client.request_json("/api/v1/provisioning/contact-points")
    if not isinstance(contact_points, list):
        raise GrafanaError("Unexpected contact-point list response from Grafana.")
    for item in contact_points:
        if not isinstance(item, dict):
            continue
        name = _normalize_dependency_text(item.get("name"))
        uid = _normalize_dependency_text(item.get("uid"))
        if name:
            availability["contactPoints"].append(name)
        if uid:
            availability["contactPoints"].append(uid)
    return availability


def _check_datasource_ref_available(
    ref,
    available_datasources_by_uid: dict[str, dict[str, Any]],
    available_datasources_by_name: dict[str, dict[str, Any]],
):
    """Internal helper for check datasource ref available."""
    if ref is None:
        return True
    if isinstance(ref, str):
        if is_builtin_datasource_ref(ref):
            return True
        return (
            lookup_datasource(
                available_datasources_by_uid,
                available_datasources_by_name,
                uid=ref,
                name=ref,
            )
            is not None
        )
    if isinstance(ref, dict):
        uid = _normalize_dependency_text(ref.get("uid"))
        name = _normalize_dependency_text(ref.get("name"))
        if uid and is_builtin_datasource_ref(uid):
            return True
        if name and is_builtin_datasource_ref(name):
            return True
        return (
            lookup_datasource(
                available_datasources_by_uid,
                available_datasources_by_name,
                uid=uid,
                name=name,
            )
            is not None
        )
    text = _normalize_dependency_text(ref)
    if not text or is_builtin_datasource_ref(text):
        return True
    return (
        lookup_datasource(
            available_datasources_by_uid,
            available_datasources_by_name,
            uid=text,
            name=text,
        )
        is not None
    )


def validate_dashboard_import_dependencies(
    dependency_records: list[DashboardImportDependencyRecord],
    availability: dict[str, Any],
) -> None:
    """Reject dashboard imports whose live dependencies are not available."""
    available_datasources_by_uid = {
        str(item): {"uid": str(item)}
        for item in availability.get("datasourceUids") or []
        if _normalize_dependency_text(item)
    }
    available_datasources_by_name = {
        str(item): {"name": str(item)}
        for item in availability.get("datasourceNames") or []
        if _normalize_dependency_text(item)
    }
    available_plugins = {
        _normalize_dependency_text(item)
        for item in availability.get("pluginIds") or []
        if _normalize_dependency_text(item)
    }
    available_contact_points = {
        _normalize_dependency_text(item)
        for item in availability.get("contactPoints") or []
        if _normalize_dependency_text(item)
    }

    missing = []
    for record in dependency_records:
        dashboard_label = record.dashboard_uid or record.dashboard_title or "-"
        for ref in record.datasource_refs:
            if _check_datasource_ref_available(
                ref,
                available_datasources_by_uid,
                available_datasources_by_name,
            ):
                continue
            missing.append(
                "file=%s dashboard=%s datasource=%s"
                % (
                    record.file,
                    dashboard_label,
                    json.dumps(ref, sort_keys=True, ensure_ascii=False, default=str),
                )
            )
        for plugin_id in record.panel_types:
            if plugin_id in available_plugins:
                continue
            missing.append(
                "file=%s dashboard=%s panel-plugin=%s"
                % (
                    record.file,
                    dashboard_label,
                    plugin_id,
                )
            )
        for datasource_uid in record.alert_datasource_uids:
            if datasource_uid in available_datasources_by_uid:
                continue
            missing.append(
                "file=%s dashboard=%s alert-datasource-uid=%s"
                % (
                    record.file,
                    dashboard_label,
                    datasource_uid,
                )
            )
        for datasource_name in record.alert_datasource_names:
            if datasource_name in available_datasources_by_name:
                continue
            missing.append(
                "file=%s dashboard=%s alert-datasource-name=%s"
                % (
                    record.file,
                    dashboard_label,
                    datasource_name,
                )
            )
        for contact_point in record.alert_contact_points:
            if contact_point in available_contact_points:
                continue
            missing.append(
                "file=%s dashboard=%s alert-contact-point=%s"
                % (
                    record.file,
                    dashboard_label,
                    contact_point,
                )
            )
    if missing:
        lines = [
            "Dashboard import dependency preflight failed.",
            "Missing dependencies detected before Grafana mutation:",
        ]
        lines.extend(["- %s" % item for item in missing])
        raise GrafanaError("\n".join(lines))


def load_json_file(path: Path) -> dict[str, Any]:
    """Read one dashboard document from disk and require a top-level JSON object."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc)) from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc)) from exc

    if not isinstance(raw, dict):
        raise GrafanaError("Dashboard file must contain a JSON object: %s" % path)
    return raw


def extract_dashboard_object(document: dict[str, Any], error_message: str) -> dict[str, Any]:
    """Return the dashboard object from either the wrapped or plain export shape."""
    dashboard = document.get("dashboard", document)
    if not isinstance(dashboard, dict):
        raise GrafanaError(error_message)
    return dashboard


def build_import_payload(
    document: dict[str, Any],
    folder_uid_override: Optional[str],
    replace_existing: bool,
    message: str,
) -> dict[str, Any]:
    """Build the POST /api/dashboards/db payload from either export shape we write."""
    if "__inputs" in document:
        raise GrafanaError(
            "Dashboard file contains Grafana web-import placeholders (__inputs). "
            "Import it through the Grafana web UI after choosing datasources."
        )

    dashboard = copy.deepcopy(
        extract_dashboard_object(document, "Dashboard payload must be a JSON object.")
    )
    dashboard["id"] = None

    meta = document.get("meta", {})
    folder_uid = folder_uid_override
    if folder_uid is None and isinstance(meta, dict):
        folder_uid = meta.get("folderUid")

    payload = {
        "dashboard": dashboard,
        "overwrite": replace_existing,
        "message": message,
    }
    if folder_uid:
        payload["folderUid"] = folder_uid
    return payload


def load_export_metadata(
    import_dir: Path,
    export_metadata_filename: str,
    root_index_kind: str,
    tool_schema_version: int,
    expected_variant: Optional[str] = None,
) -> Optional[dict[str, Any]]:
    """Load the optional export manifest and validate its schema version when present."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return load_export_metadata_from_export(
        import_dir,
        export_metadata_filename,
        root_index_kind,
        tool_schema_version,
        expected_variant=expected_variant,
    )


def validate_export_metadata(
    metadata: dict[str, Any],
    metadata_path: Path,
    root_index_kind: str,
    tool_schema_version: int,
    expected_variant: Optional[str] = None,
) -> None:
    """Reject dashboard export manifests this implementation does not understand."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    validate_export_metadata_from_export(
        metadata,
        metadata_path,
        root_index_kind,
        tool_schema_version,
        expected_variant=expected_variant,
    )


def build_compare_document(
    dashboard: dict[str, Any],
    folder_uid: Optional[str],
) -> dict[str, Any]:
    """Build the normalized comparison shape shared by import dry-run and diff."""
    compare_document: dict[str, Any] = {"dashboard": copy.deepcopy(dashboard)}
    if folder_uid:
        compare_document["folderUid"] = folder_uid
    return compare_document


def build_local_compare_document(
    document: dict[str, Any],
    folder_uid_override: Optional[str],
) -> dict[str, Any]:
    """Normalize one local raw export into the shape compared against Grafana."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 131, 64

    payload = build_import_payload(
        document=document,
        folder_uid_override=folder_uid_override,
        replace_existing=False,
        message="",
    )
    return build_compare_document(payload["dashboard"], payload.get("folderUid"))


def build_remote_compare_document(
    payload: dict[str, Any],
    folder_uid_override: Optional[str],
) -> dict[str, Any]:
    """Normalize one live dashboard wrapper into the same diff shape as local files."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 131

    dashboard = build_preserved_web_import_document(payload)
    return build_compare_document(dashboard, folder_uid_override)


def serialize_compare_document(document: dict[str, Any]) -> str:
    """Serialize normalized compare data so nested JSON can be compared stably."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def build_compare_diff_lines(
    remote_compare: dict[str, Any],
    local_compare: dict[str, Any],
    uid: str,
    dashboard_file: Path,
    context_lines: int,
) -> list[str]:
    """Render a unified diff for one dashboard comparison."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    remote_lines = json.dumps(
        remote_compare,
        indent=2,
        sort_keys=True,
        ensure_ascii=False,
    ).splitlines()
    local_lines = json.dumps(
        local_compare,
        indent=2,
        sort_keys=True,
        ensure_ascii=False,
    ).splitlines()
    return list(
        difflib.unified_diff(
            remote_lines,
            local_lines,
            fromfile="grafana:%s" % uid,
            tofile=str(dashboard_file),
            lineterm="",
            n=max(context_lines, 0),
        )
    )


def resolve_dashboard_uid_for_import(document: dict[str, Any]) -> str:
    """Return the stable dashboard UID used by dry-run and diff workflows."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 64

    payload = build_import_payload(
        document=document,
        folder_uid_override=None,
        replace_existing=False,
        message="",
    )
    uid = str(payload["dashboard"].get("uid") or "")
    if not uid:
        raise GrafanaError("Dashboard import document is missing dashboard.uid.")
    return uid


def determine_dashboard_import_action(
    client: Any,
    payload: dict[str, Any],
    replace_existing: bool,
    update_existing_only: bool = False,
) -> str:
    """Predict whether one dashboard import would create, update, or fail."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    uid = str(payload["dashboard"].get("uid") or "")
    if not uid:
        return "would-create"

    try:
        existing_payload = client.fetch_dashboard_if_exists(uid)
    except GrafanaApiError as exc:
        if exc.status_code != 404:
            raise
        existing_payload = None
    if existing_payload is None:
        if update_existing_only:
            return "would-skip-missing"
        return "would-create"

    if replace_existing or update_existing_only:
        return "would-update"
    return "would-fail-existing"


def determine_import_folder_uid_override(
    client: Any,
    uid: str,
    folder_uid_override: Optional[str],
    preserve_existing_folder: bool,
) -> Optional[str]:
    """Prefer an explicit override, otherwise keep the destination folder for updates."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if folder_uid_override is not None:
        return folder_uid_override
    if not preserve_existing_folder or not uid:
        return None
    existing_payload = client.fetch_dashboard_if_exists(uid)
    if existing_payload is None:
        return None
    meta = existing_payload.get("meta")
    if not isinstance(meta, dict):
        return ""
    return str(meta.get("folderUid") or "")


def describe_dashboard_import_mode(
    replace_existing: bool,
    update_existing_only: bool,
) -> str:
    """Return the operator-facing import mode label."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if update_existing_only:
        return "update-or-skip-missing"
    if replace_existing:
        return "create-or-update"
    return "create-only"


def build_dashboard_import_dry_run_record(
    dashboard_file: Path,
    uid: str,
    action: str,
    folder_path: Optional[str] = None,
    source_folder_path: Optional[str] = None,
    destination_folder_path: Optional[str] = None,
    reason: Optional[str] = None,
) -> dict[str, str]:
    """Build dashboard import dry run record implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    destination = "unknown"
    action_label = action or "unknown"
    if action == "would-create":
        destination = "missing"
        action_label = "create"
    elif action == "would-skip-missing":
        destination = "missing"
        action_label = "skip-missing"
    elif action == "would-update":
        destination = "exists"
        action_label = "update"
    elif action == "would-fail-existing":
        destination = "exists"
        action_label = "blocked-existing"
    elif action == "would-skip-folder-mismatch":
        destination = "exists"
        action_label = "skip-folder-mismatch"
    return {
        "uid": uid,
        "destination": destination,
        "action": action_label,
        "folderPath": str(folder_path or ""),
        "sourceFolderPath": str(source_folder_path or ""),
        "destinationFolderPath": str(destination_folder_path or ""),
        "reason": str(reason or ""),
        "file": str(dashboard_file),
    }


def parse_dashboard_import_dry_run_columns(
    value: Optional[str],
) -> Optional[list[str]]:
    """Parse one import dry-run column list into canonical dashboard import field ids."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if value is None:
        return None
    columns = []
    for item in str(value).split(","):
        column = item.strip()
        if column:
            columns.append(IMPORT_DRY_RUN_COLUMN_ALIASES.get(column, column))
    if not columns:
        raise GrafanaError(
            "--output-columns requires one or more comma-separated import dry-run column ids."
        )
    unsupported = [
        column for column in columns if column not in IMPORT_DRY_RUN_COLUMN_HEADERS
    ]
    if unsupported:
        raise GrafanaError(
            "Unsupported import dry-run column(s): %s. Supported values: %s."
            % (
                ", ".join(unsupported),
                ", ".join(sorted(IMPORT_DRY_RUN_COLUMN_ALIASES.keys())),
            )
        )
    return columns


def _render_table(headers: list[str], rows: list[list[str]], include_header: bool) -> list[str]:
    """Internal helper for render table."""
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: list[str]) -> str:
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.extend([format_row(headers), format_row(["-" * width for width in widths])])
    lines.extend(format_row(row) for row in rows)
    return lines


def render_dashboard_import_dry_run_table(
    records: list[dict[str, str]],
    include_header: bool = True,
    selected_columns: Optional[list[str]] = None,
) -> list[str]:
    """Render dashboard import dry run table implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 343

    columns = list(selected_columns or ["uid", "destination", "action"])
    if selected_columns is None:
        if any(record.get("folderPath") for record in records):
            columns.append("folderPath")
        if any(record.get("sourceFolderPath") for record in records):
            columns.append("sourceFolderPath")
        if any(record.get("destinationFolderPath") for record in records):
            columns.append("destinationFolderPath")
        if any(record.get("reason") for record in records):
            columns.append("reason")
        columns.append("file")
    headers = [IMPORT_DRY_RUN_COLUMN_HEADERS[column] for column in columns]
    rows = []
    for record in records:
        row = [record.get(column) or "" for column in columns]
        rows.append(row)
    return _render_table(headers, rows, include_header)


def render_dashboard_import_dry_run_json(
    mode: str,
    folder_records: list[dict[str, str]],
    dashboard_records: list[dict[str, str]],
    import_dir: Path,
    skipped_missing_count: int,
    skipped_folder_mismatch_count: int,
) -> str:
    """Render one JSON document for dry-run import output."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    payload = {
        "mode": mode,
        "folders": [
            {
                "uid": record.get("uid") or "",
                "destination": record.get("destination") or "",
                "status": record.get("status") or "",
                "reason": record.get("reason") or "",
                "expectedPath": record.get("expected_path") or "",
                "actualPath": record.get("actual_path") or "",
            }
            for record in folder_records
        ],
        "dashboards": [
            {
                "uid": record.get("uid") or "",
                "destination": record.get("destination") or "",
                "action": record.get("action") or "",
                "folderPath": record.get("folderPath") or "",
                "sourceFolderPath": record.get("sourceFolderPath") or "",
                "destinationFolderPath": record.get("destinationFolderPath") or "",
                "reason": record.get("reason") or "",
                "file": record.get("file") or "",
            }
            for record in dashboard_records
        ],
        "summary": {
            "importDir": str(import_dir),
            "folderCount": len(folder_records),
            "missingFolders": len(
                [record for record in folder_records if record.get("status") == "missing"]
            ),
            "mismatchedFolders": len(
                [record for record in folder_records if record.get("status") == "mismatch"]
            ),
            "dashboardCount": len(dashboard_records),
            "missingDashboards": len(
                [
                    record
                    for record in dashboard_records
                    if record.get("destination") == "missing"
                ]
            ),
            "skippedMissingDashboards": skipped_missing_count,
            "skippedFolderMismatchDashboards": skipped_folder_mismatch_count,
        },
    }
    return json.dumps(payload, indent=2, sort_keys=False, ensure_ascii=False)


def render_folder_inventory_dry_run_table(
    records: list[dict[str, str]],
    include_header: bool = True,
) -> list[str]:
    """Render folder inventory dry run table implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 343

    headers = ["UID", "DESTINATION", "STATUS", "REASON", "EXPECTED_PATH", "ACTUAL_PATH"]
    rows = []
    for record in records:
        rows.append(
            [
                record["uid"],
                record["destination"],
                record["status"],
                record["reason"],
                record["expected_path"],
                record["actual_path"],
            ]
        )
    return _render_table(headers, rows, include_header)
