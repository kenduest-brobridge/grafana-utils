"""Dashboard raw export inventory and file-discovery helpers."""

import json
from pathlib import Path
from typing import Any, Optional

from .common import DEFAULT_FOLDER_TITLE, DEFAULT_FOLDER_UID, GrafanaError


def discover_dashboard_files(
    import_dir: Path,
    raw_export_subdir: str,
    prompt_export_subdir: str,
    export_metadata_filename: str,
    folder_inventory_filename: str,
    datasource_inventory_filename: str,
) -> list[Path]:
    """Find dashboard JSON files for import and reject ambiguous combined roots."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if not import_dir.exists():
        raise GrafanaError(f"Import directory does not exist: {import_dir}")
    if not import_dir.is_dir():
        raise GrafanaError(f"Import path is not a directory: {import_dir}")
    if (import_dir / raw_export_subdir).is_dir() and (
        import_dir / prompt_export_subdir
    ).is_dir():
        raise GrafanaError(
            f"Import path {import_dir} looks like the combined export root. "
            f"Point --import-dir at {import_dir / raw_export_subdir}."
        )

    files = [
        path
        for path in sorted(import_dir.rglob("*.json"))
        if path.name
        not in {
            "index.json",
            export_metadata_filename,
            folder_inventory_filename,
            datasource_inventory_filename,
        }
    ]
    if not files:
        raise GrafanaError(f"No dashboard JSON files found in {import_dir}")
    return files


def discover_org_raw_export_dirs(import_dir: Path, raw_export_subdir: str) -> list[Path]:
    """Find per-org raw export directories under one combined multi-org export root."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if not import_dir.exists():
        raise GrafanaError(f"Import directory does not exist: {import_dir}")
    if not import_dir.is_dir():
        raise GrafanaError(f"Import path is not a directory: {import_dir}")
    org_raw_dirs = []
    for child in sorted(import_dir.iterdir()):
        if not child.is_dir() or not child.name.startswith("org_"):
            continue
        raw_dir = child / raw_export_subdir
        if raw_dir.is_dir():
            org_raw_dirs.append(raw_dir)
    if not org_raw_dirs:
        raise GrafanaError(
            "Import path %s does not contain any org-scoped %s/ exports. "
            "Point --import-dir at a combined multi-org export root created with --all-orgs."
            % (import_dir, raw_export_subdir)
        )
    return org_raw_dirs


def load_folder_inventory(
    import_dir: Path,
    default_filename: str,
    metadata: Optional[dict[str, Any]] = None,
) -> list[dict[str, str]]:
    """Load folder inventory implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    folders_file = default_filename
    if isinstance(metadata, dict):
        folders_file = str(metadata.get("foldersFile") or default_filename)
    path = import_dir / folders_file
    if not path.is_file():
        return []
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc)) from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc)) from exc
    if not isinstance(raw, list):
        raise GrafanaError("Folder inventory file must contain a JSON array: %s" % path)
    records = []
    for item in raw:
        if not isinstance(item, dict):
            raise GrafanaError("Folder inventory entry must be a JSON object: %s" % path)
        records.append(
            {
                "uid": str(item.get("uid") or ""),
                "title": str(item.get("title") or ""),
                "parentUid": str(item.get("parentUid") or ""),
                "path": str(item.get("path") or ""),
                "org": str(item.get("org") or ""),
                "orgId": str(item.get("orgId") or ""),
            }
        )
    return records


def load_datasource_inventory(
    import_dir: Path,
    default_filename: str,
    metadata: Optional[dict[str, Any]] = None,
) -> list[dict[str, str]]:
    """Load datasource inventory implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    datasources_file = default_filename
    if isinstance(metadata, dict):
        datasources_file = str(metadata.get("datasourcesFile") or default_filename)
    path = import_dir / datasources_file
    if not path.is_file():
        return []
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc)) from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc)) from exc
    if not isinstance(raw, list):
        raise GrafanaError(
            "Datasource inventory file must contain a JSON array: %s" % path
        )
    records = []
    for item in raw:
        if not isinstance(item, dict):
            raise GrafanaError(
                "Datasource inventory entry must be a JSON object: %s" % path
            )
        records.append(
            {
                "uid": str(item.get("uid") or ""),
                "name": str(item.get("name") or ""),
                "type": str(item.get("type") or ""),
                "access": str(item.get("access") or ""),
                "url": str(item.get("url") or ""),
                "isDefault": str(item.get("isDefault") or "false"),
                "org": str(item.get("org") or ""),
                "orgId": str(item.get("orgId") or ""),
            }
        )
    return records


def build_folder_inventory_lookup(
    folders: list[dict[str, str]],
) -> dict[str, dict[str, str]]:
    """Build folder inventory lookup implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    lookup = {}
    for folder in folders:
        uid = str(folder.get("uid") or "")
        if uid:
            lookup[uid] = dict(folder)
    return lookup


def build_import_dashboard_folder_path(dashboard_file: Path, import_dir: Path) -> str:
    """Build import dashboard folder path implementation."""
    relative_path = dashboard_file.relative_to(import_dir)
    parts = list(relative_path.parts[:-1])
    return " / ".join(parts)


def resolve_folder_inventory_record_for_dashboard(
    document: dict[str, Any],
    dashboard_file: Path,
    import_dir: Path,
    folder_lookup: dict[str, dict[str, str]],
    default_folder_uid: str = DEFAULT_FOLDER_UID,
    default_folder_title: str = DEFAULT_FOLDER_TITLE,
) -> Optional[dict[str, str]]:
    """Resolve folder inventory record for dashboard implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 161, 177

    def build_general_record() -> dict[str, str]:
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return {
            "uid": default_folder_uid,
            "title": default_folder_title,
            "parentUid": "",
            "path": default_folder_title,
            "builtin": "true",
        }

    meta = document.get("meta")
    if isinstance(meta, dict):
        folder_uid = str(meta.get("folderUid") or "")
        if folder_uid and folder_uid in folder_lookup:
            return dict(folder_lookup[folder_uid])
        if folder_uid == default_folder_uid:
            return build_general_record()

    folder_path = build_import_dashboard_folder_path(dashboard_file, import_dir)
    if not folder_path:
        return None
    if folder_path == default_folder_title:
        return build_general_record()
    if " / " not in folder_path:
        title_matches = []
        for record in folder_lookup.values():
            if str(record.get("title") or "") == folder_path:
                title_matches.append(dict(record))
        if len(title_matches) == 1:
            return title_matches[0]
    for record in folder_lookup.values():
        if str(record.get("path") or "") == folder_path:
            return dict(record)
    return None


def validate_export_metadata(
    metadata: dict[str, Any],
    metadata_path: Path,
    root_index_kind: str,
    tool_schema_version: int,
    expected_variant: Optional[str] = None,
) -> None:
    """Reject dashboard export manifests this implementation does not understand."""
    if metadata.get("kind") != root_index_kind:
        raise GrafanaError(
            f"Unexpected dashboard export manifest kind in {metadata_path}: "
            f"{metadata.get('kind')!r}"
        )

    schema_version = metadata.get("schemaVersion")
    if schema_version != tool_schema_version:
        raise GrafanaError(
            f"Unsupported dashboard export schemaVersion {schema_version!r} in "
            f"{metadata_path}. Expected {tool_schema_version}."
        )

    if expected_variant is not None and metadata.get("variant") != expected_variant:
        raise GrafanaError(
            f"Dashboard export manifest {metadata_path} describes variant "
            f"{metadata.get('variant')!r}. Point this command at the "
            f"{expected_variant}/ directory."
        )


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
    #   Downstream callees: 216

    metadata_path = import_dir / export_metadata_filename
    if not metadata_path.is_file():
        return None
    try:
        raw = json.loads(metadata_path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError(f"Failed to read {metadata_path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError(f"Invalid JSON in {metadata_path}: {exc}") from exc
    if not isinstance(raw, dict):
        raise GrafanaError(
            "Dashboard export metadata must be a JSON object: %s" % metadata_path
        )
    validate_export_metadata(
        raw,
        metadata_path=metadata_path,
        root_index_kind=root_index_kind,
        tool_schema_version=tool_schema_version,
        expected_variant=expected_variant,
    )
    return raw


def resolve_export_org_id(
    import_dir: Path,
    folder_inventory_filename: str,
    datasource_inventory_filename: str,
    metadata: Optional[dict[str, Any]] = None,
) -> Optional[str]:
    """Resolve one stable source export orgId from the raw export directory."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 308

    return _resolve_export_identity_field(
        import_dir,
        folder_inventory_filename,
        datasource_inventory_filename,
        field_name="orgId",
        metadata=metadata,
    )


def resolve_export_org_name(
    import_dir: Path,
    folder_inventory_filename: str,
    datasource_inventory_filename: str,
    metadata: Optional[dict[str, Any]] = None,
) -> Optional[str]:
    """Resolve one stable source export org name from the raw export directory."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 308

    return _resolve_export_identity_field(
        import_dir,
        folder_inventory_filename,
        datasource_inventory_filename,
        field_name="org",
        metadata=metadata,
    )


def _resolve_export_identity_field(
    import_dir: Path,
    folder_inventory_filename: str,
    datasource_inventory_filename: str,
    field_name: str,
    metadata: Optional[dict[str, Any]] = None,
) -> Optional[str]:
    """Resolve one stable export identity field from the raw export directory."""
    org_ids = set()
    index_file = "index.json"
    folders_file = folder_inventory_filename
    datasources_file = datasource_inventory_filename
    if isinstance(metadata, dict):
        index_file = str(metadata.get("indexFile") or index_file)
        folders_file = str(metadata.get("foldersFile") or folder_inventory_filename)
        datasources_file = str(
            metadata.get("datasourcesFile") or datasource_inventory_filename
        )
    for path in [
        import_dir / index_file,
        import_dir / folders_file,
        import_dir / datasources_file,
    ]:
        if not path.is_file():
            continue
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
        except OSError as exc:
            raise GrafanaError("Failed to read %s: %s" % (path, exc)) from exc
        except ValueError as exc:
            raise GrafanaError("Invalid JSON in %s: %s" % (path, exc)) from exc
        if isinstance(raw, dict):
            items = raw.get("items") or []
        elif isinstance(raw, list):
            items = raw
        else:
            items = []
        for item in items:
            if not isinstance(item, dict):
                continue
            value = str(item.get(field_name) or "").strip()
            if value:
                org_ids.add(value)
    if not org_ids:
        return None
    if len(org_ids) > 1:
        raise GrafanaError(
            "Raw export metadata in %s spans multiple %s values (%s). "
            "Point --import-dir at one org-specific raw export."
            % (import_dir, field_name, ", ".join(sorted(org_ids)))
        )
    return list(org_ids)[0]
