from __future__ import annotations

import html
import json
from functools import lru_cache
from pathlib import Path

from docgen_common import REPO_ROOT
from docsite_html_common import render_template
from docsite_html_nav import render_landing_locale_select
from generate_command_html import page_shell


PORTAL_CONTRACT_PATH = REPO_ROOT / "scripts" / "contracts" / "versioned-docs-portal.json"


@lru_cache(maxsize=1)
def load_versioned_docs_portal() -> dict:
    raw = json.loads(PORTAL_CONTRACT_PATH.read_text(encoding="utf-8"))
    if raw.get("schema") != 1:
        raise ValueError(f"Unsupported portal schema in {PORTAL_CONTRACT_PATH}")
    locales = raw.get("locales")
    if not isinstance(locales, dict):
        raise TypeError(f"{PORTAL_CONTRACT_PATH} must define a locale map")
    return raw


def _portal_locale(locale: str) -> dict:
    locales = load_versioned_docs_portal()["locales"]
    selected = locales.get(locale) or locales.get("en")
    if selected is None:
        raise KeyError(f"No portal locale found for {locale}")
    return selected


def _render_links(items: list[tuple[str, str]]) -> str:
    return "".join(
        f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>'
        for label, href in items
    )


def _render_panel(title: str, summary: str, links: list[tuple[str, str]]) -> str:
    return render_template(
        "landing_panel.html.tmpl",
        title=html.escape(title),
        summary=html.escape(summary),
        links_html=_render_links(links),
    )


def _render_section(title: str, summary: str, tasks: list[tuple[str, str, list[tuple[str, str]]]]) -> str:
    tasks_html = "".join(
        render_template(
            "landing_task.html.tmpl",
            title=html.escape(task_title),
            summary=html.escape(task_summary),
            links_html=_render_links(task_links),
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


def _portal_copy(locale: str, *, latest_lane: str | None, version_lanes: list[str], has_dev: bool) -> tuple[dict, list[tuple[str, str]]]:
    copy = _portal_locale(locale)
    lane_links: list[tuple[str, str]] = []
    lane_labels = copy["lane_labels"]
    if latest_lane:
        lane_links.append((lane_labels["latest_release"].format(latest_lane=latest_lane), "latest/index.html"))
    if has_dev:
        lane_links.append((lane_labels["dev_preview"], "dev/index.html"))
    lane_links.extend((label, f"{label}/index.html") for label in version_lanes)
    return copy, lane_links


def render_version_portal(*, latest_lane: str | None, version_lanes: list[str], has_dev: bool) -> str:
    portal_data: dict[str, dict[str, str]] = {}
    default_locale = "en"

    for locale in ("en", "zh-TW"):
        copy, lane_links = _portal_copy(locale, latest_lane=latest_lane, version_lanes=version_lanes, has_dev=has_dev)
        release_section = copy["sections"]["release_lanes"]
        outputs_section = copy["sections"]["available_outputs"]
        outputs_tasks = outputs_section["tasks"]
        latest_target = "latest/index.html" if latest_lane else "dev/index.html"
        portal_data[locale] = {
            "lang": locale,
            "hero_title": copy["hero_title"],
            "hero_summary": copy["hero_summary"],
            "hero_links_html": "".join(
                f'<a class="landing-hero-link" href="{html.escape(href)}">{html.escape(label)}</a>'
                for label, href in lane_links[:2]
            ),
            "sections_html": "".join(
                [
                    _render_section(
                        release_section["title"],
                        release_section["summary"],
                        [
                            (
                                release_section["tasks"]["latest_release"]["title"],
                                release_section["tasks"]["latest_release"]["summary"],
                                lane_links[:1] if latest_lane else [],
                            ),
                            (
                                release_section["tasks"]["dev_preview"]["title"],
                                release_section["tasks"]["dev_preview"]["summary"],
                                lane_links[1:2] if has_dev and latest_lane else lane_links[:1] if has_dev else [],
                            ),
                            (
                                release_section["tasks"]["older_release_lines"]["title"],
                                release_section["tasks"]["older_release_lines"]["summary"],
                                [(label, href) for label, href in lane_links if href not in {"latest/index.html", "dev/index.html"}],
                            ),
                        ],
                    ),
                    _render_section(
                        outputs_section["title"],
                        outputs_section["summary"],
                        [
                            (
                                outputs_tasks["handbook_html"]["title"],
                                outputs_tasks["handbook_html"]["summary"],
                                [(outputs_section["open_lane_label"], latest_target)],
                            ),
                            (
                                outputs_tasks["command_reference_html"]["title"],
                                outputs_tasks["command_reference_html"]["summary"],
                                [(outputs_section["open_lane_label"], latest_target)],
                            ),
                            (
                                outputs_tasks["manpage_html"]["title"],
                                outputs_tasks["manpage_html"]["summary"],
                                [(outputs_section["open_lane_label"], latest_target)],
                            ),
                        ],
                    ),
                ]
            ),
            "meta_html": "".join(
                [
                    _render_panel(
                        copy["meta"]["how_to_use"]["title"],
                        copy["meta"]["how_to_use"]["summary"],
                        lane_links[:2],
                    ),
                    _render_panel(
                        copy["meta"]["formats"]["title"],
                        copy["meta"]["formats"]["summary"],
                        [(label, "#outputs") for label in copy["meta"]["formats"]["links"]],
                    ),
                ]
            ),
            "jump_options_html": (
                f'<option value="" selected>{html.escape(copy["jump_prompt"])}</option>'
                + "".join(
                    f'<option value="{html.escape(href)}">{html.escape(label)}</option>'
                    for label, href in lane_links
                )
            ),
        }

    body_html = (
        '<div class="landing-page portal-page">'
        '<section class="landing-hero">'
        '<div class="landing-hero-inner">'
        f'<h1 id="landing-title" class="landing-title">{html.escape(portal_data[default_locale]["hero_title"])}</h1>'
        f'<p id="landing-summary" class="landing-summary">{html.escape(portal_data[default_locale]["hero_summary"])}</p>'
        f'<div id="landing-hero-links">{portal_data[default_locale]["hero_links_html"]}</div>'
        '</div>'
        '</section>'
        f'<div id="landing-sections" class="landing-sections">{portal_data[default_locale]["sections_html"]}</div>'
        f'<div id="landing-meta" class="landing-meta">{portal_data[default_locale]["meta_html"]}</div>'
        f'<script id="landing-i18n" type="application/json">{json.dumps(portal_data, ensure_ascii=False)}</script>'
        '</div>'
    )
    copy = _portal_locale(default_locale)
    jump_html = render_landing_locale_select("auto") + (
        f'<select id="jump-select" aria-label="Jump"><option value="" selected>{html.escape(copy["jump_prompt"])}</option>'
        + "".join(
            f'<option value="{html.escape(href)}">{html.escape(label)}</option>'
            for label, href in _portal_copy(default_locale, latest_lane=latest_lane, version_lanes=version_lanes, has_dev=has_dev)[1]
        )
        + "</select>"
    )
    return page_shell(
        page_title=copy["page_title"],
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
        jump_html=jump_html,
        nav_html="",
        is_landing=True,
    )
