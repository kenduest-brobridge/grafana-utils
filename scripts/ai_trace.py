#!/usr/bin/env python3
"""Maintain the repo's AI trace Markdown files."""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from datetime import date
from pathlib import Path
from typing import Optional


REPO_ROOT = Path(__file__).resolve().parents[1]
STATUS_REL = Path("docs/internal/ai-status.md")
CHANGES_REL = Path("docs/internal/ai-changes.md")
ARCHIVE_REL = Path("docs/internal/archive")
ENTRY_HEADING_RE = re.compile(r"^## \d{4}-\d{2}-\d{2} - .+$")
DEFAULT_KEEP_STATUS = 6
DEFAULT_KEEP_CHANGES = 10


@dataclass(frozen=True)
class TraceDocument:
    preamble: str
    entries: list[str]


@dataclass(frozen=True)
class SizeLimit:
    relpath: Path
    keep_count: int


def resolve_repo_path(root: Path, relpath: Path) -> Path:
    return root / relpath


def ensure_single_trailing_newline(text: str) -> str:
    return text.rstrip() + "\n"


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(ensure_single_trailing_newline(text), encoding="utf-8")


def split_trace_document(text: str) -> TraceDocument:
    lines = text.splitlines(keepends=True)
    first_entry_index = None
    for index, line in enumerate(lines):
        if ENTRY_HEADING_RE.match(line.rstrip("\n")):
            first_entry_index = index
            break

    if first_entry_index is None:
        return TraceDocument(preamble=text, entries=[])

    preamble = "".join(lines[:first_entry_index])
    entries: list[str] = []
    current: list[str] = []
    for line in lines[first_entry_index:]:
        if ENTRY_HEADING_RE.match(line.rstrip("\n")) and current:
            entries.append("".join(current).rstrip() + "\n")
            current = []
        current.append(line)
    if current:
        entries.append("".join(current).rstrip() + "\n")
    return TraceDocument(preamble=preamble, entries=entries)


def render_trace_document(document: TraceDocument) -> str:
    body = "\n\n".join(entry.rstrip() for entry in document.entries)
    if body:
        return ensure_single_trailing_newline(document.preamble.rstrip() + "\n\n" + body)
    return ensure_single_trailing_newline(document.preamble)


def insert_entry(root: Path, relpath: Path, entry: str) -> None:
    path = resolve_repo_path(root, relpath)
    document = split_trace_document(read_text(path))
    updated = TraceDocument(
        preamble=document.preamble,
        entries=[ensure_single_trailing_newline(entry)] + document.entries,
    )
    write_text(path, render_trace_document(updated))


def render_status_entry(args: argparse.Namespace) -> str:
    return "\n".join(
        [
            f"## {args.date} - {args.title}",
            "- State: Done",
            f"- Scope: {args.scope}",
            f"- Current Update: {args.summary}",
            f"- Result: {args.impact}",
        ]
    )


def render_changes_entry(args: argparse.Namespace) -> str:
    return "\n".join(
        [
            f"## {args.date} - {args.title}",
            f"- Summary: {args.summary}",
            f"- Tests: {args.tests}",
            f"- Impact: {args.impact}",
            f"- Rollback/Risk: {args.risk}",
            f"- Follow-up: {args.follow_up}",
        ]
    )


def archive_relpath(label: str, archive_date: str) -> Path:
    return ARCHIVE_REL / f"{label}-archive-{archive_date}.md"


def archive_link_line(root: Path, relpath: Path) -> str:
    archive_path = resolve_repo_path(root, relpath)
    return f"- Older entries moved to [`{relpath.name}`]({archive_path})."


def ensure_archive_link(root: Path, document: TraceDocument, archive_path: Path) -> TraceDocument:
    archive_name = archive_path.name
    if archive_name in document.preamble:
        return document

    line = archive_link_line(root, archive_path)
    preamble = document.preamble.rstrip() + "\n" + line + "\n"
    return TraceDocument(preamble=preamble, entries=document.entries)


def compact_trace_file(
    root: Path,
    relpath: Path,
    label: str,
    keep_count: int,
    archive_date: str,
) -> int:
    path = resolve_repo_path(root, relpath)
    document = split_trace_document(read_text(path))
    if len(document.entries) <= keep_count:
        return 0

    kept_entries = document.entries[:keep_count]
    moved_entries = document.entries[keep_count:]
    archive_path = archive_relpath(label, archive_date)
    document = ensure_archive_link(
        root,
        TraceDocument(preamble=document.preamble, entries=kept_entries),
        archive_path,
    )

    write_text(path, render_trace_document(document))

    archive_abs = resolve_repo_path(root, archive_path)
    archive_abs.parent.mkdir(parents=True, exist_ok=True)
    if archive_abs.exists() and archive_abs.read_text(encoding="utf-8").strip():
        prefix = archive_abs.read_text(encoding="utf-8").rstrip() + "\n\n"
    else:
        prefix = f"# {archive_path.stem}\n\n"
    archive_body = "\n\n".join(entry.rstrip() for entry in moved_entries)
    write_text(archive_abs, prefix + archive_body)
    return len(moved_entries)


def count_entries(root: Path, relpath: Path) -> int:
    path = resolve_repo_path(root, relpath)
    return len(split_trace_document(read_text(path)).entries)


def check_size(root: Path, keep_status: int, keep_changes: int) -> list[str]:
    limits = (
        SizeLimit(STATUS_REL, keep_status),
        SizeLimit(CHANGES_REL, keep_changes),
    )
    errors: list[str] = []
    for limit in limits:
        count = count_entries(root, limit.relpath)
        if count > limit.keep_count:
            errors.append(
                f"{limit.relpath} has {count} entries; keep at most {limit.keep_count}. "
                "Run: python3 scripts/ai_trace.py compact"
            )
    return errors


def handle_add(args: argparse.Namespace) -> int:
    root = args.root.resolve()
    insert_entry(root, STATUS_REL, render_status_entry(args))
    insert_entry(root, CHANGES_REL, render_changes_entry(args))
    print("ai_trace: added entries to docs/internal/ai-status.md and docs/internal/ai-changes.md")
    return 0


def handle_compact(args: argparse.Namespace) -> int:
    root = args.root.resolve()
    moved_status = compact_trace_file(
        root,
        STATUS_REL,
        "ai-status",
        args.keep_status,
        args.date,
    )
    moved_changes = compact_trace_file(
        root,
        CHANGES_REL,
        "ai-changes",
        args.keep_changes,
        args.date,
    )
    print(f"ai_trace: moved {moved_status} status entries and {moved_changes} change entries")
    return 0


def handle_check_size(args: argparse.Namespace) -> int:
    errors = check_size(args.root.resolve(), args.keep_status, args.keep_changes)
    if errors:
        for error in errors:
            print(f"error: {error}")
        return 1
    print("ai_trace: size check ok")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Maintain AI trace Markdown files.")
    parser.add_argument(
        "--root",
        type=Path,
        default=REPO_ROOT,
        help="Repository root. Defaults to this script's repository.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    add_parser = subparsers.add_parser("add", help="Add one structured trace entry.")
    add_parser.add_argument("--title", required=True)
    add_parser.add_argument("--scope", required=True)
    add_parser.add_argument("--summary", required=True)
    add_parser.add_argument("--tests", required=True)
    add_parser.add_argument("--impact", required=True)
    add_parser.add_argument("--risk", required=True)
    add_parser.add_argument("--follow-up", default="none")
    add_parser.add_argument("--date", default=date.today().isoformat())
    add_parser.set_defaults(handler=handle_add)

    compact_parser = subparsers.add_parser("compact", help="Archive older trace entries.")
    compact_parser.add_argument("--keep-status", type=int, default=DEFAULT_KEEP_STATUS)
    compact_parser.add_argument("--keep-changes", type=int, default=DEFAULT_KEEP_CHANGES)
    compact_parser.add_argument("--date", default=date.today().isoformat())
    compact_parser.set_defaults(handler=handle_compact)

    check_parser = subparsers.add_parser("check-size", help="Fail when trace files are too long.")
    check_parser.add_argument("--keep-status", type=int, default=DEFAULT_KEEP_STATUS)
    check_parser.add_argument("--keep-changes", type=int, default=DEFAULT_KEEP_CHANGES)
    check_parser.set_defaults(handler=handle_check_size)

    return parser


def main(argv: Optional[list[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.handler(args)


if __name__ == "__main__":
    raise SystemExit(main())
