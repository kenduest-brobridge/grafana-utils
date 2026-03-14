"""Dashboard folder inventory and import-folder helper functions."""

from pathlib import Path
from typing import Any, Optional

from .common import (
    DEFAULT_FOLDER_TITLE,
    DEFAULT_FOLDER_UID,
    DEFAULT_ORG_ID,
    DEFAULT_ORG_NAME,
    GrafanaError,
)
from .export_inventory import (
    build_folder_inventory_lookup as build_folder_inventory_lookup_from_export,
    build_import_dashboard_folder_path as build_import_dashboard_folder_path_from_export,
    load_datasource_inventory as load_datasource_inventory_from_export,
    load_folder_inventory as load_folder_inventory_from_export,
    resolve_folder_inventory_record_for_dashboard as resolve_folder_inventory_record_for_dashboard_from_export,
)
from .listing import build_folder_path


def build_folder_inventory_record(
    folder: dict[str, Any],
    org: dict[str, Any],
    fallback_title: str,
) -> dict[str, str]:
    uid = str(folder.get("uid") or "")
    title = str(folder.get("title") or fallback_title or uid or DEFAULT_FOLDER_TITLE)
    parents = folder.get("parents")
    parent_uid = ""
    if isinstance(parents, list) and parents:
        last_parent = parents[-1]
        if isinstance(last_parent, dict):
            parent_uid = str(last_parent.get("uid") or "")
    return {
        "uid": uid,
        "title": title,
        "parentUid": parent_uid,
        "path": build_folder_path(folder, title),
        "org": str(org.get("name") or DEFAULT_ORG_NAME),
        "orgId": str(org.get("id") or DEFAULT_ORG_ID),
    }


def collect_folder_inventory(
    client: Any,
    org: dict[str, Any],
    summaries: list[dict[str, Any]],
) -> list[dict[str, str]]:
    folders_by_uid = {}
    pending = []
    for summary in summaries:
        folder_uid = str(summary.get("folderUid") or "").strip()
        folder_title = str(summary.get("folderTitle") or DEFAULT_FOLDER_TITLE)
        if folder_uid:
            pending.append({"uid": folder_uid, "title": folder_title})

    while pending:
        item = pending.pop()
        folder_uid = item["uid"]
        if not folder_uid or folder_uid in folders_by_uid:
            continue
        folder = client.fetch_folder_if_exists(folder_uid)
        if not folder:
            continue
        folders_by_uid[folder_uid] = build_folder_inventory_record(folder, org, item["title"])
        parents = folder.get("parents")
        if isinstance(parents, list):
            for parent in parents:
                if isinstance(parent, dict):
                    parent_uid = str(parent.get("uid") or "").strip()
                    parent_title = str(parent.get("title") or parent_uid or "folder")
                    if parent_uid and parent_uid not in folders_by_uid:
                        pending.append({"uid": parent_uid, "title": parent_title})

    return sorted(
        folders_by_uid.values(),
        key=lambda item: (item["orgId"], item["path"], item["uid"]),
    )


def load_folder_inventory(
    import_dir: Path,
    folder_inventory_filename: str,
    metadata: Optional[dict[str, Any]] = None,
) -> list[dict[str, str]]:
    return load_folder_inventory_from_export(
        import_dir,
        folder_inventory_filename,
        metadata=metadata,
    )


def load_datasource_inventory(
    import_dir: Path,
    datasource_inventory_filename: str,
    metadata: Optional[dict[str, Any]] = None,
) -> list[dict[str, str]]:
    return load_datasource_inventory_from_export(
        import_dir,
        datasource_inventory_filename,
        metadata=metadata,
    )


def ensure_folder_inventory(
    client: Any,
    folders: list[dict[str, str]],
) -> int:
    created_count = 0
    sorted_folders = sorted(
        folders,
        key=lambda item: (
            item.get("path", "").count(" / "),
            item.get("path", ""),
            item.get("uid", ""),
        ),
    )
    for folder in sorted_folders:
        uid = folder.get("uid") or ""
        title = folder.get("title") or uid
        parent_uid = folder.get("parentUid") or None
        if not uid:
            continue
        if client.fetch_folder_if_exists(uid) is not None:
            continue
        client.create_folder(uid=uid, title=title, parent_uid=parent_uid)
        created_count += 1
    return created_count


def inspect_folder_inventory(
    client: Any,
    folders: list[dict[str, str]],
) -> list[dict[str, str]]:
    records = []
    sorted_folders = sorted(
        folders,
        key=lambda item: (
            item.get("path", "").count(" / "),
            item.get("path", ""),
            item.get("uid", ""),
        ),
    )
    for folder in sorted_folders:
        uid = str(folder.get("uid") or "")
        if not uid:
            continue
        expected_path = str(folder.get("path") or "")
        status = determine_folder_inventory_status(client, folder)
        live_folder = build_live_folder_inventory_record(client, uid)
        if live_folder is None:
            records.append(
                {
                    "uid": uid,
                    "destination": "missing",
                    "status": "missing",
                    "reason": "would-create",
                    "expected_path": expected_path,
                    "actual_path": "",
                }
            )
            continue
        records.append(
            {
                "uid": uid,
                "destination": "exists",
                "status": status.get("status") or "unknown",
                "reason": status.get("details") or "",
                "expected_path": expected_path,
                "actual_path": str(live_folder.get("path") or ""),
            }
        )
    return records


def resolve_folder_inventory_requirements(
    args: Any,
    import_dir: Path,
    metadata: Optional[dict[str, Any]],
    folder_inventory_filename: str,
) -> list[dict[str, str]]:
    """Load the optional folder inventory and enforce explicit operator intent."""
    folder_inventory = load_folder_inventory(
        import_dir,
        folder_inventory_filename,
        metadata=metadata,
    )
    if getattr(args, "import_folder_uid", None) is not None:
        return folder_inventory
    if getattr(args, "ensure_folders", False) and not folder_inventory:
        folders_file = folder_inventory_filename
        if isinstance(metadata, dict):
            folders_file = str(metadata.get("foldersFile") or folder_inventory_filename)
        raise GrafanaError(
            "Folder inventory file not found for --ensure-folders: %s. "
            "Re-export dashboards with raw folder inventory or omit --ensure-folders."
            % (import_dir / folders_file)
        )
    return folder_inventory


def build_folder_inventory_lookup(
    folders: list[dict[str, str]],
) -> dict[str, dict[str, str]]:
    return build_folder_inventory_lookup_from_export(folders)


def build_import_dashboard_folder_path(dashboard_file: Path, import_dir: Path) -> str:
    return build_import_dashboard_folder_path_from_export(dashboard_file, import_dir)


def resolve_folder_inventory_record_for_dashboard(
    document: dict[str, Any],
    dashboard_file: Path,
    import_dir: Path,
    folder_lookup: dict[str, dict[str, str]],
) -> Optional[dict[str, str]]:
    return resolve_folder_inventory_record_for_dashboard_from_export(
        document,
        dashboard_file,
        import_dir,
        folder_lookup,
        default_folder_uid=DEFAULT_FOLDER_UID,
        default_folder_title=DEFAULT_FOLDER_TITLE,
    )


def build_live_folder_inventory_record(
    client: Any,
    uid: str,
) -> Optional[dict[str, str]]:
    if not uid:
        return None
    folder = client.fetch_folder_if_exists(uid)
    if folder is None:
        return None
    title = str(folder.get("title") or uid)
    parents = folder.get("parents")
    if isinstance(parents, list):
        parent_uid = ""
        if parents:
            last_parent = parents[-1]
            if isinstance(last_parent, dict):
                parent_uid = str(last_parent.get("uid") or "")
        return {
            "uid": uid,
            "title": title,
            "parentUid": parent_uid,
            "path": build_folder_path(folder, title),
        }

    parent_uid = str(folder.get("parentUid") or "")
    path_titles = [title]
    seen = set([uid])
    current_parent_uid = parent_uid
    while current_parent_uid:
        if current_parent_uid in seen:
            break
        seen.add(current_parent_uid)
        parent = client.fetch_folder_if_exists(current_parent_uid)
        if parent is None:
            break
        parent_title = str(parent.get("title") or current_parent_uid)
        path_titles.append(parent_title)
        current_parent_uid = str(parent.get("parentUid") or "")
    path_titles.reverse()
    return {
        "uid": uid,
        "title": title,
        "parentUid": parent_uid,
        "path": " / ".join(path_titles),
    }


def determine_folder_inventory_status(
    client: Any,
    expected_folder: Optional[dict[str, str]],
) -> dict[str, str]:
    if expected_folder is None:
        return {"status": "unknown", "details": ""}
    if str(expected_folder.get("builtin") or "") == "true":
        return {"status": "general", "details": "default-grafana"}

    uid = str(expected_folder.get("uid") or "")
    live_folder = build_live_folder_inventory_record(client, uid)
    if live_folder is None:
        return {"status": "missing", "details": ""}

    mismatch_fields = []
    for field in ("title", "parentUid", "path"):
        if str(expected_folder.get(field) or "") != str(live_folder.get(field) or ""):
            mismatch_fields.append(field)
    if mismatch_fields:
        return {"status": "mismatch", "details": ",".join(mismatch_fields)}
    return {"status": "match", "details": ""}


def resolve_dashboard_import_folder_path(
    client: Any,
    payload: dict[str, Any],
    document: dict[str, Any],
    dashboard_file: Path,
    import_dir: Path,
    folder_inventory_lookup: dict[str, dict[str, str]],
) -> str:
    """Resolve the effective destination folder path for one dashboard import."""
    folder_uid = str(payload.get("folderUid") or "").strip()
    if not folder_uid or folder_uid == DEFAULT_FOLDER_UID:
        return DEFAULT_FOLDER_TITLE

    live_folder = client.fetch_folder_if_exists(folder_uid)
    if isinstance(live_folder, dict):
        return build_folder_path(live_folder, str(live_folder.get("title") or folder_uid))

    inventory_record = folder_inventory_lookup.get(folder_uid)
    if inventory_record is None:
        inventory_record = resolve_folder_inventory_record_for_dashboard(
            document,
            dashboard_file,
            import_dir,
            folder_inventory_lookup,
        )
        if (
            inventory_record is None
            or str(inventory_record.get("uid") or "").strip() != folder_uid
        ):
            inventory_record = None
    if inventory_record is not None:
        path = str(inventory_record.get("path") or "").strip()
        if path:
            return path
        title = str(inventory_record.get("title") or folder_uid).strip()
        if title:
            return title
    return folder_uid
