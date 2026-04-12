#!/usr/bin/env python3
"""Non-destructive workspace noise discovery helper.

The new module returns candidate noise paths (temp dirs, scratch exports, editor
artefacts) so cleanup remains explicit and reviewable.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from subprocess import CalledProcessError, run
from typing import Iterable


DEFAULT_NOISE_MARKERS: list[str] = [
    ".tmp",
    ".temp",
    ".cache",
    ".idea",
    ".vscode",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    ".mypy_cache",
    ".DS_Store",
]

NOISE_DIRECTORIES: list[str] = [
    ".eggs",
    "build",
    "dist",
    "tmp",
    "coverage",
    ".tox",
    ".venv",
    "target",
    "node_modules",
]

NOISE_FILENAMES: list[str] = [
    ".gitkeep",
]

ALLOWED_NOISE_LIKE_SUFFIXES: tuple[str, ...] = (
    ".tmpl",
)


@dataclass(frozen=True)
class NoisePath:
    """One candidate path reported by the auditor."""

    path: Path
    category: str
    reason: str


def _contains_marker(path: Path, markers: Iterable[str]) -> bool:
    """Return true when a basename contains any configured marker."""
    basename = path.name
    for marker in markers:
        if marker and marker in basename:
            return True
    return False


def discover_noise_paths(root: Path) -> list[NoisePath]:
    """Discover likely scratch/noise paths under a repository root."""
    noise: list[NoisePath] = []
    if not root.exists():
        return noise

    pending_dirs = [root]
    while pending_dirs:
        current = pending_dirs.pop()
        try:
            children = list(current.iterdir())
        except OSError:
            continue

        for path in children:
            name = path.name
            if path.is_dir():
                if name in NOISE_DIRECTORIES:
                    noise.append(
                        NoisePath(
                            path=path,
                            category="directory",
                            reason="known temporary or build directory",
                        )
                    )
                    continue
                pending_dirs.append(path)
                continue

            if not path.is_file():
                continue
            if path.suffix in ALLOWED_NOISE_LIKE_SUFFIXES:
                continue
            if _contains_marker(path, DEFAULT_NOISE_MARKERS):
                noise.append(
                    NoisePath(
                        path=path,
                        category="artifact",
                        reason="filename or suffix matches common workspace noise",
                    )
                )
                continue
            if name in NOISE_FILENAMES:
                noise.append(
                    NoisePath(
                        path=path,
                        category="ignored-file",
                        reason="explicit ignore marker entry",
                    )
                )
    return noise


def render_noise_report(root: Path) -> list[str]:
    """Build a plain text report for review workflows or CI checks."""
    lines: list[str] = []
    for item in discover_noise_paths(root):
        lines.append(f"{item.category}\t{item.reason}\t{item.path}")
    return lines


def discover_noise_paths_from_git_status(root: Path) -> list[NoisePath]:
    """Report only noise paths that currently show up in git status."""
    tracked_noise = {item.path.resolve(): item for item in discover_noise_paths(root)}
    if not tracked_noise:
        return []

    try:
        result = run(
            ["git", "status", "--short", "--untracked-files=all"],
            check=True,
            capture_output=True,
            text=True,
            cwd=root,
        )
    except (OSError, CalledProcessError):
        return []

    visible_noise: list[NoisePath] = []
    seen_paths: set[Path] = set()
    for raw_line in result.stdout.splitlines():
        if len(raw_line) < 4:
            continue
        path_text = raw_line[3:].strip()
        if " -> " in path_text:
            path_text = path_text.split(" -> ", 1)[1]
        candidate = (root / path_text).resolve()
        noise = tracked_noise.get(candidate)
        if noise is None or candidate in seen_paths:
            continue
        visible_noise.append(noise)
        seen_paths.add(candidate)
    return visible_noise


def render_git_status_noise_report(root: Path) -> list[str]:
    """Build a plain text report for noise currently visible in git status."""
    return [
        f"{item.category}\t{item.reason}\t{item.path}"
        for item in discover_noise_paths_from_git_status(root)
    ]


def main() -> int:
    """CLI entry point for local manual workspace scans."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 102, 63

    from argparse import ArgumentParser

    parser = ArgumentParser(description="Scan for likely workspace noise files.")
    parser.add_argument(
        "--root",
        default=".",
        help="Repository/workspace root to scan.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render a machine-readable JSON payload.",
    )
    parser.add_argument(
        "--check-git-status",
        action="store_true",
        help="Fail when likely scratch/noise paths currently appear in git status.",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    if not root.is_dir():
        print(f"Invalid workspace root: {root}")
        return 2

    if args.json:
        from json import dumps
        items = (
            discover_noise_paths_from_git_status(root)
            if args.check_git_status
            else discover_noise_paths(root)
        )
        payload = [
            {"path": str(item.path), "category": item.category, "reason": item.reason}
            for item in items
        ]
        print(dumps(payload, indent=2, sort_keys=False))
        return 1 if args.check_git_status and payload else 0

    lines = (
        render_git_status_noise_report(root)
        if args.check_git_status
        else render_noise_report(root)
    )
    for line in lines:
        print(line)
    return 1 if args.check_git_status and lines else 0


if __name__ == "__main__":
    raise SystemExit(main())
