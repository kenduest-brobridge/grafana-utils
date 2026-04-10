#!/usr/bin/env python3
"""Check a few high-signal AI workflow drift rules for the current change set."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]

TRACE_FILES = {
    "docs/internal/ai-status.md",
    "docs/internal/ai-changes.md",
}

HTML_SOURCE_PREFIXES = (
    "docs/landing/",
    "docs/user-guide/",
    "docs/commands/",
    "scripts/templates/",
)

HTML_SOURCE_FILES = {
    "docs/DEVELOPER.md",
    "scripts/generate_command_html.py",
    "scripts/docgen_command_docs.py",
    "scripts/docgen_common.py",
    "scripts/docgen_handbook.py",
    "scripts/docgen_landing.py",
    "scripts/generate_manpages.py",
}

MAN_SOURCE_PREFIXES = ("docs/commands/en/",)

MAN_SOURCE_FILES = {
    "VERSION",
    "scripts/generate_manpages.py",
    "scripts/docgen_command_docs.py",
    "scripts/docgen_common.py",
}

WORKSPACE_NOISE_PREFIXES = (
    ".codex/",
    ".workspace-noise/",
    "notes/",
    "scratch/",
    "test-results/",
)

MEANINGFUL_INTERNAL_DOC_FILES = {
    "docs/DEVELOPER.md",
    "docs/internal/ai-workflow-note.md",
    "docs/internal/maintainer-quickstart.md",
    "docs/internal/maintainer-role-map.md",
    "docs/internal/contract-doc-map.md",
    "docs/internal/generated-docs-architecture.md",
    "docs/internal/generated-docs-playbook.md",
    "docs/internal/profile-secret-storage-architecture.md",
}

MEANINGFUL_INTERNAL_DOC_SUFFIXES = (
    "-contract.md",
    "-architecture.md",
    "-policy.md",
)


def relpath(path: str | Path) -> str:
    return Path(path).as_posix().lstrip("./")


def matches_any_prefix(path: str, prefixes: tuple[str, ...]) -> bool:
    return any(path.startswith(prefix) for prefix in prefixes)


def is_meaningful_internal_doc(path: str) -> bool:
    if path in MEANINGFUL_INTERNAL_DOC_FILES:
        return True
    if not path.startswith("docs/internal/"):
        return False
    return path.endswith(MEANINGFUL_INTERNAL_DOC_SUFFIXES)


def is_workspace_noise_path(path: str) -> bool:
    return any(path.startswith(prefix) for prefix in WORKSPACE_NOISE_PREFIXES)


def detect_changed_files() -> list[str]:
    diff_cmd = [
        "git",
        "diff",
        "--name-only",
        "--relative",
        "--diff-filter=ACMR",
        "HEAD",
    ]
    result = subprocess.run(
        diff_cmd,
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def validate_paths(paths: list[str]) -> list[str]:
    normalized = [relpath(path) for path in paths]
    path_set = set(normalized)
    errors: list[str] = []

    touched_html = any(path.startswith("docs/html/") for path in normalized)
    touched_man = any(path.startswith("docs/man/") for path in normalized)
    touched_meaningful_internal = any(is_meaningful_internal_doc(path) for path in normalized)
    touched_trace = TRACE_FILES.issubset(path_set)
    touched_workspace_noise = [path for path in normalized if is_workspace_noise_path(path)]

    if touched_html:
        has_html_source = any(
            matches_any_prefix(path, HTML_SOURCE_PREFIXES) or path in HTML_SOURCE_FILES
            for path in normalized
        )
        if not has_html_source:
            errors.append(
                "changed docs/html output without changing its source Markdown, templates, or HTML generator files"
            )

    if touched_man:
        has_man_source = any(
            matches_any_prefix(path, MAN_SOURCE_PREFIXES) or path in MAN_SOURCE_FILES
            for path in normalized
        )
        if not has_man_source:
            errors.append(
                "changed docs/man output without changing docs/commands/en or the manpage generator files"
            )

    if touched_meaningful_internal and not touched_trace:
        errors.append(
            "changed meaningful maintainer/contract/architecture docs without updating both docs/internal/ai-status.md and docs/internal/ai-changes.md"
        )

    if touched_workspace_noise:
        errors.append(
            "changed local workspace noise paths that should stay out of review and commit paths: "
            + ", ".join(touched_workspace_noise)
        )

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Check a few high-signal AI workflow drift rules."
    )
    parser.add_argument(
        "paths",
        nargs="*",
        help="Optional repo-relative paths to validate. If omitted, validate the current git diff against HEAD.",
    )
    args = parser.parse_args(argv)

    changed = [relpath(path) for path in args.paths] if args.paths else detect_changed_files()

    if not changed:
        print("check_ai_workflow: no changed paths to validate")
        return 0

    errors = validate_paths(changed)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        print("checked paths:", file=sys.stderr)
        for path in changed:
            print(f"  - {path}", file=sys.stderr)
        return 1

    print("check_ai_workflow: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
