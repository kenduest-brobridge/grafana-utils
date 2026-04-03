"""Dashboard export and import progress rendering helpers."""

from pathlib import Path
from typing import Any, Optional


def print_dashboard_export_progress(
    args: Any,
    index: int,
    total: int,
    uid: str,
    variant: str,
    path: Path,
    dry_run: bool,
) -> None:
    """Render one export progress update in concise or verbose form."""
    if getattr(args, "verbose", False):
        print(
            "%s %s    %s -> %s"
            % ("Would export" if dry_run else "Exported", variant, uid, path)
        )


def print_dashboard_export_progress_summary(
    args: Any,
    index: int,
    total: int,
    uid: str,
    dry_run: bool,
) -> None:
    """Render one concise export progress update per dashboard."""
    if getattr(args, "verbose", False):
        return
    if getattr(args, "progress", False):
        print(
            "%s dashboard %s/%s: %s"
            % ("Would export" if dry_run else "Exporting", index, total, uid)
        )


def print_dashboard_import_progress(
    args: Any,
    index: int,
    total: int,
    dashboard_file: Path,
    uid: str,
    action: Optional[str] = None,
    status: Optional[str] = None,
    folder_status: Optional[str] = None,
    folder_details: Optional[str] = None,
    folder_path: Optional[str] = None,
    dry_run: bool = False,
) -> None:
    """Render one import progress update in concise or verbose form."""
    destination = None
    action_label = action or "unknown"
    if action:
        if action == "would-create":
            destination = "missing"
            action_label = "create"
        elif action == "would-skip-missing":
            destination = "missing"
            action_label = "skip-missing"
        elif action == "would-skip-folder-mismatch":
            destination = "exists"
            action_label = "skip-folder-mismatch"
        elif action in ("would-update", "would-fail-existing"):
            destination = "exists"
            if action == "would-update":
                action_label = "update"
            else:
                action_label = "blocked-existing"
        else:
            destination = "unknown"
    folder_segment = ""
    if dry_run and folder_path:
        folder_segment = " folderPath=%s" % folder_path
    if getattr(args, "verbose", False):
        if dry_run:
            print(
                "Dry-run import uid=%s dest=%s action=%s%s file=%s"
                % (
                    uid,
                    destination or "unknown",
                    action_label,
                    folder_segment,
                    dashboard_file,
                )
            )
        else:
            print(
                "Imported %s -> uid=%s status=%s"
                % (dashboard_file, uid, status or "unknown")
            )
        return
    if getattr(args, "progress", False):
        if dry_run:
            print(
                "Dry-run dashboard %s/%s: %s dest=%s action=%s%s"
                % (
                    index,
                    total,
                    uid,
                    destination or "unknown",
                    action_label,
                    folder_segment,
                )
            )
        else:
            print("Importing dashboard %s/%s: %s" % (index, total, uid))
