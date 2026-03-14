"""Dashboard export/output helper functions."""

import json
import re
from pathlib import Path
from typing import Any, Optional


def sanitize_path_component(value: str) -> str:
    normalized = re.sub(r"[^\w.\- ]+", "_", value.strip(), flags=re.UNICODE)
    normalized = re.sub(r"\s+", "_", normalized)
    normalized = re.sub(r"_+", "_", normalized)
    normalized = normalized.strip("._")
    return normalized or "untitled"


def build_output_path(
    output_dir: Path,
    summary: dict[str, Any],
    flat: bool,
    default_folder_title: str,
    default_dashboard_title: str,
    default_unknown_uid: str,
) -> Path:
    folder_title = summary.get("folderTitle") or default_folder_title
    folder_name = sanitize_path_component(str(folder_title))
    title = sanitize_path_component(
        str(summary.get("title") or default_dashboard_title)
    )
    uid = sanitize_path_component(str(summary.get("uid") or default_unknown_uid))
    filename = "%s__%s.json" % (title, uid)
    if flat:
        return output_dir / filename
    return output_dir / folder_name / filename


def build_all_orgs_output_dir(
    output_dir: Path,
    org: dict[str, Any],
    default_unknown_uid: str,
) -> Path:
    """Return one org-prefixed export directory for multi-org dashboard exports."""
    org_id = sanitize_path_component(str(org.get("id") or default_unknown_uid))
    org_name = sanitize_path_component(str(org.get("name") or "org"))
    return output_dir / ("org_%s_%s" % (org_id, org_name))


def build_export_variant_dirs(
    output_dir: Path,
    raw_export_subdir: str,
    prompt_export_subdir: str,
) -> tuple[Path, Path]:
    """Return the raw/ and prompt/ export directories for one dashboard export root."""
    return output_dir / raw_export_subdir, output_dir / prompt_export_subdir


def ensure_dashboard_write_target(
    output_path: Path,
    overwrite: bool,
    error_cls: Any,
    create_parents: bool = True,
) -> None:
    """Create parent directories when needed and enforce the overwrite policy."""
    if create_parents:
        output_path.parent.mkdir(parents=True, exist_ok=True)
    if output_path.exists() and not overwrite:
        raise error_cls(
            "Refusing to overwrite existing file: %s. Use --overwrite." % output_path
        )


def write_dashboard(
    payload: dict[str, Any],
    output_path: Path,
    overwrite: bool,
    error_cls: Any,
) -> None:
    """Write one dashboard JSON file, creating parent directories as needed."""
    ensure_dashboard_write_target(output_path, overwrite, error_cls)
    output_path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def write_json_document(payload: Any, output_path: Path) -> None:
    """Write a JSON file with the formatting used by this repository."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def build_dashboard_index_item(
    summary: dict[str, Any],
    uid: str,
    default_org_name: str,
    default_org_id: str,
) -> dict[str, str]:
    """Build the shared root index metadata for one exported dashboard."""
    return {
        "uid": uid,
        "title": str(summary.get("title") or ""),
        "folder": str(summary.get("folderTitle") or ""),
        "org": str(summary.get("orgName") or default_org_name),
        "orgId": str(summary.get("orgId") or default_org_id),
    }


def build_variant_index(
    index_items: list[dict[str, str]],
    path_key: str,
    format_name: str,
) -> list[dict[str, str]]:
    """Build one variant-specific index file from the shared root index items."""
    return [
        {
            "uid": item["uid"],
            "title": item["title"],
            "folder": item["folder"],
            "org": item["org"],
            "orgId": item["orgId"],
            "path": item[path_key],
            "format": format_name,
        }
        for item in index_items
        if path_key in item
    ]


def build_root_export_index(
    index_items: list[dict[str, str]],
    raw_index_path: Optional[Path],
    prompt_index_path: Optional[Path],
    tool_schema_version: int,
    root_index_kind: str,
) -> dict[str, Any]:
    """Build the versioned root manifest for one dashboard export run."""
    return {
        "schemaVersion": tool_schema_version,
        "kind": root_index_kind,
        "items": index_items,
        "variants": {
            "raw": str(raw_index_path) if raw_index_path is not None else None,
            "prompt": str(prompt_index_path) if prompt_index_path is not None else None,
        },
    }


def build_export_metadata(
    variant: str,
    dashboard_count: int,
    tool_schema_version: int,
    root_index_kind: str,
    format_name: Optional[str] = None,
    folders_file: Optional[str] = None,
    datasources_file: Optional[str] = None,
) -> dict[str, Any]:
    """Describe one export directory in a small, versioned manifest."""
    metadata = {
        "schemaVersion": tool_schema_version,
        "kind": root_index_kind,
        "variant": variant,
        "dashboardCount": dashboard_count,
        "indexFile": "index.json",
    }
    if format_name:
        metadata["format"] = format_name
    if folders_file:
        metadata["foldersFile"] = folders_file
    if datasources_file:
        metadata["datasourcesFile"] = datasources_file
    return metadata
