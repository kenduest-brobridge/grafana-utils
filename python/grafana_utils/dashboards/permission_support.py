"""Dashboard export permission helper functions."""

from typing import Any

from ..dashboard_permission_workbench import build_permission_export_document


def collect_permission_export_documents(
    client: Any,
    summaries: list[dict[str, Any]],
    folder_inventory: list[dict[str, str]],
) -> list[dict[str, Any]]:
    """Collect permission export documents for the exported dashboards and folders."""
    documents = []
    seen_folders = set()
    for folder in folder_inventory:
        folder_uid = str(folder.get("uid") or "").strip()
        if not folder_uid or folder_uid in seen_folders:
            continue
        seen_folders.add(folder_uid)
        document = build_permission_export_document(
            "folder",
            folder_uid,
            str(folder.get("title") or folder_uid).strip(),
            client.fetch_folder_permissions(folder_uid),
        )
        document["org"] = str(folder.get("org") or "").strip()
        document["orgId"] = str(folder.get("orgId") or "").strip()
        documents.append(document)
    for summary in summaries:
        dashboard_uid = str(summary.get("uid") or "").strip()
        if not dashboard_uid:
            continue
        document = build_permission_export_document(
            "dashboard",
            dashboard_uid,
            str(summary.get("title") or dashboard_uid).strip(),
            client.fetch_dashboard_permissions(dashboard_uid),
        )
        document["org"] = str(summary.get("orgName") or "").strip()
        document["orgId"] = str(summary.get("orgId") or "").strip()
        documents.append(document)
    return documents
