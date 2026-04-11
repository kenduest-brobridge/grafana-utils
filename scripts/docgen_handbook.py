"""Shared handbook metadata for generated HTML manual pages.

This file keeps the handbook chapter order and locale paths explicit so the
HTML generator can stay focused on rendering.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from docgen_common import REPO_ROOT, relative_href


def get_handbook_root(repo_root: Path = REPO_ROOT) -> Path:
    return repo_root / "docs" / "user-guide"


HANDBOOK_ROOT = get_handbook_root()
HANDBOOK_LOCALES = ("en", "zh-TW")
HANDBOOK_ORDER = (
    "index.md",
    "what-is-grafana-util.md",
    "getting-started.md",
    "role-new-user.md",
    "role-sre-ops.md",
    "role-automation-ci.md",
    "architecture.md",
    "dashboard.md",
    "datasource.md",
    "alert.md",
    "access.md",
    "status-workspace.md",
    "scenarios.md",
    "recipes.md",
    "reference.md",
    "troubleshooting.md",
)
HANDBOOK_NAV_GROUPS = (
    ("start", ("index.md", "what-is-grafana-util.md", "getting-started.md")),
    ("role-paths", ("role-new-user.md", "role-sre-ops.md", "role-automation-ci.md")),
    ("core-operations", ("dashboard.md", "datasource.md", "alert.md", "access.md")),
    ("governance", ("architecture.md", "status-workspace.md")),
    ("scenarios-reference", ("scenarios.md", "recipes.md", "reference.md", "troubleshooting.md")),
)
HANDBOOK_NAV_GROUP_LABELS = {
    "en": {
        "start": "Start",
        "role-paths": "Role Paths",
        "core-operations": "Core Operations",
        "governance": "Governance",
        "scenarios-reference": "Scenarios & Reference",
    },
    "zh-TW": {
        "start": "開始",
        "role-paths": "角色路徑",
        "core-operations": "核心操作",
        "governance": "治理",
        "scenarios-reference": "實戰與參考",
    },
}
LOCALE_LABELS = {
    "en": "English",
    "zh-TW": "繁體中文",
}


@dataclass(frozen=True)
class HandbookPage:
    locale: str
    source_path: Path
    output_rel: str
    stem: str
    title: str
    previous_output_rel: str | None
    previous_title: str | None
    next_output_rel: str | None
    next_title: str | None
    language_switch_rel: str | None


def _validate_handbook_nav_groups() -> None:
    nav_files = [filename for _, filenames in HANDBOOK_NAV_GROUPS for filename in filenames]
    if len(nav_files) != len(set(nav_files)):
        raise ValueError("HANDBOOK_NAV_GROUPS must not contain duplicate handbook files")
    if set(nav_files) != set(HANDBOOK_ORDER):
        raise ValueError("HANDBOOK_NAV_GROUPS must cover HANDBOOK_ORDER exactly")


def parse_title(path: Path) -> str:
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return path.stem.replace("-", " ").title()


def existing_handbook_files(locale: str, handbook_root: Path = HANDBOOK_ROOT) -> list[str]:
    if locale not in HANDBOOK_LOCALES:
        raise ValueError(f"Unsupported handbook locale: {locale}")
    locale_dir = handbook_root / locale
    return [filename for filename in HANDBOOK_ORDER if (locale_dir / filename).exists()]


def build_handbook_pages(locale: str, handbook_root: Path = HANDBOOK_ROOT) -> list[HandbookPage]:
    """Build the ordered handbook page list for one locale."""
    if locale not in HANDBOOK_LOCALES:
        raise ValueError(f"Unsupported handbook locale: {locale}")
    locale_dir = handbook_root / locale
    filenames = existing_handbook_files(locale, handbook_root)
    output_rels = [f"handbook/{locale}/{Path(name).with_suffix('.html').as_posix()}" for name in filenames]
    titles = [parse_title(locale_dir / filename) for filename in filenames]
    pages: list[HandbookPage] = []
    for index, filename in enumerate(filenames):
        source_path = locale_dir / filename
        output_rel = output_rels[index]
        other_locale = next((candidate for candidate in HANDBOOK_LOCALES if candidate != locale), None)
        other_output_rel = None
        if other_locale is not None:
            other_source_path = handbook_root / other_locale / filename
            if other_source_path.exists():
                other_output_rel = f"handbook/{other_locale}/{Path(filename).with_suffix('.html').as_posix()}"
        pages.append(
            HandbookPage(
                locale=locale,
                source_path=source_path,
                output_rel=output_rel,
                stem=Path(filename).stem,
                title=titles[index],
                previous_output_rel=output_rels[index - 1] if index > 0 else None,
                previous_title=titles[index - 1] if index > 0 else None,
                next_output_rel=output_rels[index + 1] if index + 1 < len(output_rels) else None,
                next_title=titles[index + 1] if index + 1 < len(output_rels) else None,
                language_switch_rel=other_output_rel,
            )
        )
    return pages


def handbook_language_href(page: HandbookPage) -> str | None:
    if page.language_switch_rel is None:
        return None
    return relative_href(page.output_rel, page.language_switch_rel)


_validate_handbook_nav_groups()
