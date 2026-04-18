#!/usr/bin/env python3
"""Classify a docs-related diff into source, generated, contract, and CLI buckets."""

from __future__ import annotations

import argparse
import csv
import json
import io
import subprocess
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]

SOURCE_DOC_PREFIXES = (
    "docs/landing/",
    "docs/user-guide/",
    "docs/commands/",
)
SOURCE_DOC_FILES = {
    "README.md",
    "README.zh-TW.md",
}
GENERATED_DOC_PREFIXES = (
    "docs/html/",
    "docs/man/",
)
COMMAND_CONTRACT_PREFIX = "scripts/contracts/"
COMMAND_CONTRACT_SKIP_PREFIX = "scripts/contracts/output-fixtures/"
PUBLIC_CLI_PREFIXES = (
    "rust/src/bin/",
    "rust/src/cli/",
    "rust/src/commands/",
)


@dataclass(frozen=True)
class ClassifiedPath:
    path: str
    category: str


def relpath(path: str | Path) -> str:
    return Path(path).as_posix().lstrip("./")


def detect_changed_files(ref: str) -> list[str]:
    result = subprocess.run(
        [
            "git",
            "diff",
            "--name-only",
            "--relative",
            "--diff-filter=ACMR",
            ref,
        ],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def classify_path(path: str) -> str:
    if path in SOURCE_DOC_FILES or any(path.startswith(prefix) for prefix in SOURCE_DOC_PREFIXES):
        return "source_docs"
    if any(path.startswith(prefix) for prefix in GENERATED_DOC_PREFIXES):
        return "generated_docs"
    if path.startswith(COMMAND_CONTRACT_PREFIX) and not path.startswith(COMMAND_CONTRACT_SKIP_PREFIX):
        return "command_contracts"
    if any(path.startswith(prefix) for prefix in PUBLIC_CLI_PREFIXES):
        return "public_cli"
    return "other"


def classify_paths(paths: list[str]) -> list[ClassifiedPath]:
    return [ClassifiedPath(path=relpath(path), category=classify_path(relpath(path))) for path in paths]


def summarize(classifications: list[ClassifiedPath]) -> dict[str, object]:
    buckets: dict[str, list[str]] = {
        "source_docs": [],
        "generated_docs": [],
        "command_contracts": [],
        "public_cli": [],
        "other": [],
    }
    for item in classifications:
        buckets.setdefault(item.category, []).append(item.path)
    for paths in buckets.values():
        paths.sort()

    source_docs = buckets["source_docs"]
    generated_docs = buckets["generated_docs"]
    command_contracts = buckets["command_contracts"]
    public_cli = buckets["public_cli"]
    missing_generated_updates = bool(source_docs or command_contracts) and not bool(generated_docs)
    stale_generated_updates = bool(generated_docs) and not bool(source_docs or command_contracts)

    return {
        "counts": {name: len(paths) for name, paths in buckets.items()},
        "paths": buckets,
        "missing_generated_updates": missing_generated_updates,
        "stale_generated_updates": stale_generated_updates,
        "generated_update_status": (
            "missing_generated_updates"
            if missing_generated_updates
            else "stale_generated_updates"
            if stale_generated_updates
            else "balanced"
        ),
        "public_cli_touched": bool(public_cli),
    }


def render_table(summary: dict[str, object]) -> str:
    counts = summary["counts"]  # type: ignore[index]
    paths = summary["paths"]  # type: ignore[index]
    lines = [
        "docs-diff-classify summary:",
        (
            "  source_docs={source_docs} generated_docs={generated_docs} "
            "command_contracts={command_contracts} public_cli={public_cli} other={other}"
        ).format(**counts),
        f"  generated_update_status={summary['generated_update_status']}",
    ]

    for title in ("source_docs", "generated_docs", "command_contracts", "public_cli", "other"):
        items = paths[title]
        lines.append(f"{title}:")
        if items:
            lines.extend(f"  - {item}" for item in items)
        else:
            lines.append("  - (none)")

    if summary["missing_generated_updates"]:
        lines.append("note: source docs or command contracts changed without generated docs updates")
    if summary["stale_generated_updates"]:
        lines.append("note: generated docs changed without source docs or command contracts")
    return "\n".join(lines)


def render_json(summary: dict[str, object]) -> str:
    return json.dumps(summary, indent=2, sort_keys=True)


def render_csv(summary: dict[str, object]) -> str:
    buffer = io.StringIO()
    writer = csv.writer(buffer)
    writer.writerow(["category", "path"])
    paths = summary["paths"]  # type: ignore[index]
    for category in ("source_docs", "generated_docs", "command_contracts", "public_cli", "other"):
        for path in paths[category]:
            writer.writerow([category, path])
    return buffer.getvalue().rstrip("\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "paths",
        nargs="*",
        help="Optional repo-relative paths to classify. If omitted, classify the current git diff against the selected ref.",
    )
    parser.add_argument(
        "--ref",
        default="HEAD",
        help="Git ref or diff expression to classify when no explicit paths are supplied.",
    )
    parser.add_argument(
        "--format",
        choices=("table", "json", "csv"),
        default="table",
        help="Output format.",
    )
    args = parser.parse_args()

    changed = [relpath(path) for path in args.paths] if args.paths else detect_changed_files(args.ref)
    classifications = classify_paths(changed)
    summary = summarize(classifications)

    if not changed:
        print("docs-diff-classify: no changed paths to classify")
        return 0

    if args.format == "json":
        print(render_json(summary))
    elif args.format == "csv":
        print(render_csv(summary))
    else:
        print(render_table(summary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
