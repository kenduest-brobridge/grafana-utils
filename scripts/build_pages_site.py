#!/usr/bin/env python3
"""Assemble a versioned GitHub Pages docs site from tags and branch refs."""

from __future__ import annotations

import argparse
import html
import json
import re
import shutil
import subprocess
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path

from docgen_common import REPO_ROOT, write_outputs
from generate_command_html import HtmlBuildConfig, VersionLink, generate_outputs, page_shell, html_list
from generate_command_html import render_landing_locale_select, render_template


SEMVER_TAG_RE = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")


@dataclass(frozen=True, order=True)
class SemverTag:
    major: int
    minor: int
    patch: int
    raw: str

    @property
    def minor_label(self) -> str:
        return f"v{self.major}.{self.minor}"


def run_git(args: list[str]) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def parse_semver_tag(text: str) -> SemverTag | None:
    match = SEMVER_TAG_RE.fullmatch(text.strip())
    if not match:
        return None
    return SemverTag(
        major=int(match.group(1)),
        minor=int(match.group(2)),
        patch=int(match.group(3)),
        raw=text.strip(),
    )


def list_release_tags() -> list[SemverTag]:
    tags = [parse_semver_tag(line) for line in run_git(["tag", "--list", "v*"]).splitlines() if line.strip()]
    return sorted((tag for tag in tags if tag is not None), reverse=True)


def select_latest_tags_per_minor(tags: list[SemverTag]) -> list[SemverTag]:
    by_minor: dict[tuple[int, int], SemverTag] = {}
    for tag in tags:
        key = (tag.major, tag.minor)
        current = by_minor.get(key)
        if current is None or tag.patch > current.patch:
            by_minor[key] = tag
    return sorted(by_minor.values(), reverse=True)


def resolve_ref(candidates: list[str]) -> str | None:
    for candidate in candidates:
        if not candidate:
            continue
        result = subprocess.run(
            ["git", "rev-parse", "--verify", candidate],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            return candidate
    return None


def export_ref_tree(ref: str, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    archive_path = destination / "source.tar"
    with archive_path.open("wb") as handle:
        subprocess.run(["git", "archive", "--format=tar", ref], cwd=REPO_ROOT, check=True, stdout=handle)
    with tarfile.open(archive_path) as tar:
        tar.extractall(destination, filter="data")
    archive_path.unlink()


def has_modern_docs_source(source_root: Path) -> bool:
    required_paths = (
        source_root / "VERSION",
        source_root / "docs" / "commands" / "en" / "dashboard.md",
        source_root / "docs" / "commands" / "zh-TW" / "index.md",
        source_root / "docs" / "user-guide" / "en" / "index.md",
        source_root / "docs" / "user-guide" / "zh-TW" / "index.md",
    )
    return all(path.exists() for path in required_paths)


def build_version_links(version_lanes: list[str]) -> tuple[VersionLink, ...]:
    links = [
        VersionLink("Version portal", "index.html"),
        VersionLink("Latest release", "latest/index.html"),
        VersionLink("Dev preview", "dev/index.html"),
    ]
    links.extend(VersionLink(label, f"{label}/index.html") for label in version_lanes)
    return tuple(links)


def lane_config(
    *,
    source_root: Path,
    output_prefix: str,
    version_label: str,
    version_links: tuple[VersionLink, ...],
) -> HtmlBuildConfig:
    version_value = (source_root / "VERSION").read_text(encoding="utf-8").strip()
    return HtmlBuildConfig(
        source_root=source_root,
        command_docs_root=source_root / "docs" / "commands",
        handbook_root=source_root / "docs" / "user-guide",
        output_prefix=output_prefix,
        version=version_value,
        version_label=version_label,
        version_links=version_links,
        raw_manpage_target_rel=f"{output_prefix}/man/grafana-util.1",
        include_raw_manpages=True,
    )


def render_version_portal(
    *,
    latest_lane: str | None,
    version_lanes: list[str],
    has_dev: bool,
) -> str:
    lane_links_en: list[tuple[str, str]] = []
    lane_links_zh: list[tuple[str, str]] = []
    if latest_lane:
        lane_links_en.append((f"Latest release ({latest_lane})", "latest/index.html"))
        lane_links_zh.append((f"最新版本（{latest_lane}）", "latest/index.html"))
    if has_dev:
        lane_links_en.append(("Dev preview", "dev/index.html"))
        lane_links_zh.append(("開發預覽", "dev/index.html"))
    lane_links_en.extend((label, f"{label}/index.html") for label in version_lanes)
    lane_links_zh.extend((label, f"{label}/index.html") for label in version_lanes)

    def render_links(items: list[tuple[str, str]]) -> str:
        return "".join(
            f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>'
            for label, href in items
        )

    def render_panel(title: str, summary: str, links: list[tuple[str, str]]) -> str:
        return render_template(
            "landing_panel.html.tmpl",
            title=html.escape(title),
            summary=html.escape(summary),
            links_html=render_links(links),
        )

    def render_section(title: str, summary: str, tasks: list[tuple[str, str, list[tuple[str, str]]]]) -> str:
        tasks_html = "".join(
            render_template(
                "landing_task.html.tmpl",
                title=html.escape(task_title),
                summary=html.escape(task_summary),
                links_html=render_links(task_links),
            )
            for task_title, task_summary, task_links in tasks
        )
        return render_template(
            "landing_section.html.tmpl",
            title=html.escape(title),
            summary=html.escape(summary),
            inline_html="",
            tasks_html=tasks_html,
        )

    outputs_links_en = [
        ("Handbook HTML", "#outputs"),
        ("Command reference HTML", "#outputs"),
        ("Manpage HTML mirrors", "#outputs"),
    ]
    outputs_links_zh = [
        ("手冊 HTML", "#outputs"),
        ("指令說明 HTML", "#outputs"),
        ("Manpage HTML 鏡像", "#outputs"),
    ]

    portal_data = {
        "en": {
            "lang": "en",
            "hero_title": "grafana-util Versioned Docs",
            "hero_summary": "Pick the release line you need, switch into the right language, and open handbook, command, or manpage outputs from one portal.",
            "search_heading": "Choose a docs lane",
            "search_copy": "Use quick jump for the latest release, the dev preview, or a specific release line.",
            "search_placeholder": "Jump to latest, dev, or a release line",
            "search_button": "Open",
            "sections_html": "".join(
                [
                    render_section(
                        "Release lanes",
                        "Published docs stay grouped by release line so you can read the right behavior for your deployed version.",
                        [
                            ("Latest release", "Open the latest published handbook, command docs, and manpage mirrors.", lane_links_en[:1] if latest_lane else []),
                            ("Dev preview", "Open the current development docs before the next release ships.", lane_links_en[1:2] if has_dev and latest_lane else lane_links_en[:1] if has_dev else []),
                            ("Older release lines", "Browse older published lines when you need version-specific behavior or migration context.", [(label, href) for label, href in lane_links_en if href not in {"latest/index.html", "dev/index.html"}]),
                        ],
                    ),
                    render_section(
                        "Available outputs",
                        "Each lane exposes the same three generated surfaces so operators and maintainers can switch format without leaving the version context.",
                        [
                            ("Handbook HTML", "Task and workflow guidance for operators and maintainers.", [("Open a docs lane first", "latest/index.html" if latest_lane else "dev/index.html")]),
                            ("Command reference HTML", "Per-command and per-subcommand reference pages.", [("Open a docs lane first", "latest/index.html" if latest_lane else "dev/index.html")]),
                            ("Manpage HTML mirrors", "Browser-readable mirrors of the generated manpages.", [("Open a docs lane first", "latest/index.html" if latest_lane else "dev/index.html")]),
                        ],
                    ),
                ]
            ),
            "meta_html": "".join(
                [
                    render_panel(
                        "How to use this portal",
                        "Choose a release line first. Once you enter that lane, use the built-in language switch and jump menu inside the versioned docs.",
                        lane_links_en[:2],
                    ),
                    render_panel(
                        "Formats",
                        "All release lanes expose the same generated outputs.",
                        outputs_links_en,
                    ),
                ]
            ),
            "jump_options_html": (
                '<option value="" selected>Jump to a docs lane...</option>'
                + "".join(
                    f'<option value="{html.escape(href)}">{html.escape(label)}</option>'
                    for label, href in lane_links_en
                )
            ),
        },
        "zh-TW": {
            "lang": "zh-TW",
            "hero_title": "grafana-util 版本文件入口",
            "hero_summary": "先選版本線，再進入對應語言的手冊、指令說明或 manpage HTML，不用在首頁自己猜路徑。",
            "search_heading": "選擇文件版本線",
            "search_copy": "可直接跳到最新版本、開發預覽，或指定的 release line。",
            "search_placeholder": "快速跳到最新版本、開發預覽或指定版本",
            "search_button": "開啟",
            "sections_html": "".join(
                [
                    render_section(
                        "版本線",
                        "已發佈文件會依版本線整理，方便你直接查看目前部署版本對應的行為與說明。",
                        [
                            ("最新版本", "直接開啟最新發佈的手冊、指令說明與 manpage HTML。", lane_links_zh[:1] if latest_lane else []),
                            ("開發預覽", "查看下一個版本尚未發佈前的最新文件。", lane_links_zh[1:2] if has_dev and latest_lane else lane_links_zh[:1] if has_dev else []),
                            ("舊版本線", "需要比對舊版行為、升級差異或回看舊文件時，從這裡進入。", [(label, href) for label, href in lane_links_zh if href not in {"latest/index.html", "dev/index.html"}]),
                        ],
                    ),
                    render_section(
                        "可用輸出",
                        "每條版本線都提供同一組生成文件，方便依工作情境切換閱讀形式。",
                        [
                            ("手冊 HTML", "工作流程、操作順序與使用情境的完整說明。", [("先開啟任一版本線", "latest/index.html" if latest_lane else "dev/index.html")]),
                            ("指令說明 HTML", "每個 command 與 subcommand 的參數、用途與範例。", [("先開啟任一版本線", "latest/index.html" if latest_lane else "dev/index.html")]),
                            ("Manpage HTML", "可在瀏覽器閱讀的 manpage 鏡像。", [("先開啟任一版本線", "latest/index.html" if latest_lane else "dev/index.html")]),
                        ],
                    ),
                ]
            ),
            "meta_html": "".join(
                [
                    render_panel(
                        "如何使用這個入口",
                        "先選版本線。進入該版本後，再用頁面內建的語言切換與快速跳轉找你要看的內容。",
                        lane_links_zh[:2],
                    ),
                    render_panel(
                        "輸出形式",
                        "每條版本線都提供相同的生成輸出。",
                        outputs_links_zh,
                    ),
                ]
            ),
            "jump_options_html": (
                '<option value="" selected>快速跳到版本線...</option>'
                + "".join(
                    f'<option value="{html.escape(href)}">{html.escape(label)}</option>'
                    for label, href in lane_links_zh
                )
            ),
        },
    }
    default_locale = "en"
    body_html = (
        '<div class="landing-page portal-page">'
        '<section class="landing-hero">'
        '<div class="landing-hero-inner">'
        f'<h1 id="landing-title" class="landing-title">{html.escape(portal_data[default_locale]["hero_title"])}</h1>'
        f'<p id="landing-summary" class="landing-summary">{html.escape(portal_data[default_locale]["hero_summary"])}</p>'
        '</div>'
        '<section class="landing-search-panel">'
        f'<h2 id="landing-search-heading">{html.escape(portal_data[default_locale]["search_heading"])}</h2>'
        f'<p id="landing-search-copy">{html.escape(portal_data[default_locale]["search_copy"])}</p>'
        '<form id="landing-search-form" class="landing-search-form">'
        f'<input id="landing-search" class="landing-search-input" type="search" placeholder="{html.escape(portal_data[default_locale]["search_placeholder"])}" aria-label="{html.escape(portal_data[default_locale]["search_placeholder"])}" />'
        f'<button id="landing-search-button" class="landing-search-button" type="submit">{html.escape(portal_data[default_locale]["search_button"])}</button>'
        '</form>'
        '</section>'
        '</section>'
        f'<div id="landing-sections" class="landing-sections">{portal_data[default_locale]["sections_html"]}</div>'
        f'<div id="landing-meta" class="landing-meta">{portal_data[default_locale]["meta_html"]}</div>'
        f'<script id="landing-i18n" type="application/json">{json.dumps(portal_data, ensure_ascii=False)}</script>'
        '</div>'
    )
    return page_shell(
        page_title="grafana-util versioned docs",
        html_lang="en",
        home_href="index.html",
        hero_title="",
        hero_summary="",
        breadcrumbs=[("Home", None)],
        body_html=body_html,
        toc_html="",
        related_html="",
        version_html="",
        locale_html="",
        footer_nav_html="",
        footer_html="Generated by <code>scripts/build_pages_site.py</code>.",
        jump_html=render_landing_locale_select("auto") + '<select id="jump-select" aria-label="Jump"><option value="" selected>Jump to a docs lane...</option>' + "".join(
            f'<option value="{html.escape(href)}">{html.escape(label)}</option>'
            for label, href in lane_links_en
        ) + "</select>",
        nav_html="",
        is_landing=True,
    )


def assemble_site(output_dir: Path) -> None:
    release_tags = select_latest_tags_per_minor(list_release_tags())
    outputs: dict[str, str] = {".nojekyll": ""}
    supported_release_tags: list[SemverTag] = []
    dev_ref = resolve_ref(["origin/dev", "dev"])

    with tempfile.TemporaryDirectory(prefix="grafana-util-pages-") as tmp_root_text:
        tmp_root = Path(tmp_root_text)

        for tag in release_tags:
            source_root = tmp_root / tag.raw
            export_ref_tree(tag.raw, source_root)
            if not has_modern_docs_source(source_root):
                continue
            try:
                generate_outputs(
                    lane_config(
                        source_root=source_root,
                        output_prefix=tag.minor_label,
                        version_label=tag.minor_label,
                        version_links=(),
                    )
                )
            except FileNotFoundError as exc:
                print(f"Skipping {tag.raw}: {exc}")
                continue
            supported_release_tags.append(tag)

        version_lanes = [tag.minor_label for tag in supported_release_tags]
        version_links = build_version_links(version_lanes)

        for tag in supported_release_tags:
            source_root = tmp_root / tag.raw
            outputs.update(
                generate_outputs(
                    lane_config(
                        source_root=source_root,
                        output_prefix=tag.minor_label,
                        version_label=tag.minor_label,
                        version_links=version_links,
                    )
                )
            )

        if supported_release_tags:
            latest_source_root = tmp_root / "latest"
            export_ref_tree(supported_release_tags[0].raw, latest_source_root)
            outputs.update(
                generate_outputs(
                    lane_config(
                        source_root=latest_source_root,
                        output_prefix="latest",
                        version_label=f"Latest release ({supported_release_tags[0].minor_label})",
                        version_links=version_links,
                    )
                )
            )

        if dev_ref:
            dev_source_root = tmp_root / "dev"
            export_ref_tree(dev_ref, dev_source_root)
            if has_modern_docs_source(dev_source_root):
                try:
                    outputs.update(
                        generate_outputs(
                            lane_config(
                                source_root=dev_source_root,
                                output_prefix="dev",
                                version_label="Dev preview",
                                version_links=version_links,
                            )
                        )
                    )
                except FileNotFoundError as exc:
                    print(f"Skipping dev preview: {exc}")

    outputs["index.html"] = render_version_portal(
        latest_lane=supported_release_tags[0].minor_label if supported_release_tags else None,
        version_lanes=version_lanes,
        has_dev=dev_ref is not None and "dev/index.html" in outputs,
    )
    if output_dir.exists():
        shutil.rmtree(output_dir)
    write_outputs(output_dir, outputs)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Assemble a multi-version GitHub Pages docs site.")
    parser.add_argument("--output-dir", required=True, help="Directory to write the assembled Pages site into.")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    assemble_site(Path(args.output_dir).resolve())
    print(f"Wrote versioned docs site to {args.output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
