#!/usr/bin/env python3
"""Non-destructive workspace noise discovery helper.

The new module returns candidate noise paths (temp dirs, scratch exports, editor
artefacts) so cleanup remains explicit and reviewable.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
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

    for path in root.rglob("*"):
        if path == root:
            continue
        name = path.name
        if path.is_dir() and path.name in NOISE_DIRECTORIES:
            noise.append(
                NoisePath(
                    path=path,
                    category="directory",
                    reason="known temporary or build directory",
                )
            )
            continue
        if path.is_file() and _contains_marker(path, NOISE_MARKERS):
            noise.append(
                NoisePath(
                    path=path,
                    category="artifact",
                    reason="filename or suffix matches common workspace noise",
                )
            )
            continue
        if path.is_file() and name in NOISE_FILENAMES:
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


def main() -> int:
    """CLI entry point for local manual workspace scans."""
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
    args = parser.parse_args()

    root = Path(args.root).resolve()
    if not root.is_dir():
        print(f"Invalid workspace root: {root}")
        return 2

    if args.json:
        from json import dumps
        payload = [
            {"path": str(item.path), "category": item.category, "reason": item.reason}
            for item in discover_noise_paths(root)
        ]
        print(dumps(payload, indent=2, sort_keys=False))
        return 0

    for line in render_noise_report(root):
        print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
