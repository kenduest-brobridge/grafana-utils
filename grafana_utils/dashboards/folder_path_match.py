"""Helpers for optional dashboard import folder-path matching guards.

This module is intentionally standalone so the matching logic can be developed
and tested before it is wired into the existing import workflow.
"""

from pathlib import Path
from typing import Any, Optional

from .common import DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID
from .folder_support import (
    build_import_dashboard_folder_path,
    build_live_folder_inventory_record,
    resolve_folder_inventory_record_for_dashboard,
)

FOLDER_PATH_MISMATCH_ACTION = "would-skip-folder-mismatch"
FOLDER_PATH_UNKNOWN_REASON = "folder-path-unknown"
FOLDER_PATH_MISMATCH_REASON = "folder-path-mismatch"


def normalize_folder_path(
    folder_path: Optional[str],
    default_folder_title: str = DEFAULT_FOLDER_TITLE,
) -> str:
    """Normalize an operator-facing folder path into a stable comparison string."""
    path = str(folder_path or "").strip()
    if path:
        return path
    return default_folder_title


def resolve_source_dashboard_folder_path(
    document: dict[str, Any],
    dashboard_file: Path,
    import_dir: Path,
    folder_inventory_lookup: dict[str, dict[str, str]],
    default_folder_title: str = DEFAULT_FOLDER_TITLE,
) -> str:
    """Resolve the raw dashboard's source folder path from inventory or file layout."""
    inventory_record = resolve_folder_inventory_record_for_dashboard(
        document,
        dashboard_file,
        import_dir,
        folder_inventory_lookup,
    )
    if inventory_record is not None:
        path = str(inventory_record.get("path") or "").strip()
        if path:
            return path
        title = str(inventory_record.get("title") or "").strip()
        if title:
            return title

    relative_folder_path = build_import_dashboard_folder_path(
        dashboard_file, import_dir
    )
    return normalize_folder_path(relative_folder_path, default_folder_title)


def resolve_existing_dashboard_folder_path(
    client: Any,
    dashboard_uid: str,
    default_folder_uid: str = DEFAULT_FOLDER_UID,
    default_folder_title: str = DEFAULT_FOLDER_TITLE,
) -> Optional[str]:
    """Resolve the live destination folder path for one existing dashboard UID."""
    uid = str(dashboard_uid or "").strip()
    if not uid:
        return None

    existing_payload = client.fetch_dashboard_if_exists(uid)
    if not isinstance(existing_payload, dict):
        return None

    meta = existing_payload.get("meta")
    if not isinstance(meta, dict):
        return None

    folder_uid = str(meta.get("folderUid") or "").strip()
    if not folder_uid or folder_uid == default_folder_uid:
        return default_folder_title

    live_folder = build_live_folder_inventory_record(client, folder_uid)
    if live_folder is None:
        return None

    folder_path = str(live_folder.get("path") or "").strip()
    if folder_path:
        return folder_path
    title = str(live_folder.get("title") or "").strip()
    if title:
        return title
    return None


def build_folder_path_match_result(
    source_folder_path: Optional[str],
    destination_folder_path: Optional[str],
    destination_exists: bool,
    require_matching_folder_path: bool,
    default_folder_title: str = DEFAULT_FOLDER_TITLE,
) -> dict[str, Any]:
    """Compare source and destination folder paths for an optional update guard."""
    normalized_source = normalize_folder_path(source_folder_path, default_folder_title)
    normalized_destination = None
    if destination_folder_path is not None:
        normalized_destination = normalize_folder_path(
            destination_folder_path,
            default_folder_title,
        )

    if not require_matching_folder_path:
        return {
            "matches": True,
            "reason": "",
            "source_folder_path": normalized_source,
            "destination_folder_path": normalized_destination,
            "destination_exists": bool(destination_exists),
        }

    if not destination_exists:
        return {
            "matches": True,
            "reason": "",
            "source_folder_path": normalized_source,
            "destination_folder_path": normalized_destination,
            "destination_exists": False,
        }

    if normalized_destination is None:
        return {
            "matches": False,
            "reason": FOLDER_PATH_UNKNOWN_REASON,
            "source_folder_path": normalized_source,
            "destination_folder_path": None,
            "destination_exists": True,
        }

    if normalized_source == normalized_destination:
        return {
            "matches": True,
            "reason": "",
            "source_folder_path": normalized_source,
            "destination_folder_path": normalized_destination,
            "destination_exists": True,
        }

    return {
        "matches": False,
        "reason": FOLDER_PATH_MISMATCH_REASON,
        "source_folder_path": normalized_source,
        "destination_folder_path": normalized_destination,
        "destination_exists": True,
    }


def apply_folder_path_guard_to_action(
    action: str,
    match_result: dict[str, Any],
) -> str:
    """Rewrite update actions when a required folder-path comparison does not match."""
    if action != "would-update":
        return action
    if bool(match_result.get("matches")):
        return action
    return FOLDER_PATH_MISMATCH_ACTION
