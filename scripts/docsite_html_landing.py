from __future__ import annotations

import html
import json
from pathlib import Path

from docgen_common import relative_href
from docgen_landing import LANDING_LOCALES, LandingLink, LandingSection, LandingTask, load_landing_page
from docsite_html_common import prefixed_output_rel, render_template
from docsite_html_nav import render_jump_select, render_jump_select_options, render_landing_locale_select
from docsite_html_page_shell import page_shell
from docsite_ui_text import ui_text


def landing_panel_html(title: str, summary: str, links: list[tuple[str, str]]) -> str:
    links_html = "".join(f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>' for label, href in links)
    return render_template("landing_panel.html.tmpl", title=html.escape(title), summary=html.escape(summary), links_html=links_html)


def landing_link_is_available(source_path: Path, target: str, config) -> bool:
    if target.startswith(("http", "mailto:", "#")):
        return True
    bare = target.split("#", 1)[0]
    resolved = (source_path.parent / bare).resolve()
    try:
        relative = resolved.relative_to(config.source_root / "docs")
    except Exception:
        return True
    relative_path = Path(relative)
    if relative_path.parts[:1] == ("user-guide",) and relative_path.suffix == ".md":
        return config.handbook_root.joinpath(*relative_path.parts[1:]).exists()
    if relative_path.parts[:1] == ("commands",) and relative_path.suffix == ".md":
        return config.command_docs_root.joinpath(*relative_path.parts[1:]).exists()
    return True


def render_landing_links(source_path: Path, output_rel: str, links: tuple[LandingLink, ...], config, rewrite_markdown_link) -> list[tuple[str, str]]:
    rendered: list[tuple[str, str]] = []
    for link in links:
        if not landing_link_is_available(source_path, link.target, config):
            continue
        rendered.append((link.label, rewrite_markdown_link(source_path, output_rel, link.target, config)))
    return rendered


def render_landing_task(source_path: Path, output_rel: str, task: LandingTask, config, rewrite_markdown_link) -> str:
    rendered_links = render_landing_links(source_path, output_rel, task.links, config, rewrite_markdown_link)
    links_html = "".join(f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>' for label, href in rendered_links)
    return render_template("landing_task.html.tmpl", title=html.escape(task.title), summary=html.escape(task.summary), links_html=links_html)


def render_landing_section(source_path: Path, output_rel: str, section: LandingSection, config, rewrite_markdown_link) -> str:
    inline_html = ""
    inline_links = render_landing_links(source_path, output_rel, section.links, config, rewrite_markdown_link)
    if inline_links:
        inline_html = '<ul class="landing-inline-links">' + "".join(f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>' for label, href in inline_links) + "</ul>"
    return render_template("landing_section.html.tmpl", title=html.escape(section.title), summary=html.escape(section.summary), inline_html=inline_html, tasks_html="".join(render_landing_task(source_path, output_rel, task, config, rewrite_markdown_link) for task in section.tasks))


def render_landing_hero_links(source_path: Path, output_rel: str, links: tuple[LandingLink, ...], config, rewrite_markdown_link) -> str:
    rendered_links = render_landing_links(source_path, output_rel, links, config, rewrite_markdown_link)
    if not rendered_links:
        return ""
    links_html = "".join(
        f'<a class="landing-hero-link" href="{html.escape(href)}">{html.escape(label)}</a>'
        for label, href in rendered_links
    )
    return f'<div class="landing-hero-actions">{links_html}</div>'


def build_landing_locale_data(config, rewrite_markdown_link) -> dict[str, dict[str, str]]:
    landing_rel = prefixed_output_rel(config, "index.html")
    landing_root = config.source_root / "docs" / "landing"
    version_links = [(config.version_label or config.version, "#")]
    version_links.extend((link.label, relative_href(landing_rel, link.target_rel)) for link in config.version_links)
    if config.include_raw_manpages or config.raw_manpage_target_rel:
        version_links.append(("Manpages", relative_href(landing_rel, prefixed_output_rel(config, "man/index.html"))))
    has_version_switch = bool(config.version_links)
    landing_data: dict[str, dict[str, str]] = {}
    for locale in LANDING_LOCALES:
        page = load_landing_page(locale, landing_root=landing_root)
        maintainer_links = render_landing_links(page.source_path, landing_rel, page.maintainer.links, config, rewrite_markdown_link)
        version_title = ui_text(locale, "landing_version_switch_title")
        version_summary = ui_text(locale, "landing_version_switch_summary")
        if not has_version_switch:
            version_title = ui_text(locale, "landing_current_version_title")
            version_summary = ui_text(locale, "landing_current_version_summary")
        meta_html = "".join((
            landing_panel_html(page.maintainer.title, page.maintainer.summary, maintainer_links),
            landing_panel_html(version_title, version_summary, version_links),
        ))
        landing_data[locale] = {
            "lang": locale,
            "hero_title": page.title,
            "hero_summary": page.summary,
            "hero_links_html": render_landing_hero_links(page.source_path, landing_rel, page.hero_links, config, rewrite_markdown_link),
            "sections_html": "".join(render_landing_section(page.source_path, landing_rel, section, config, rewrite_markdown_link) for section in page.sections),
            "meta_html": meta_html,
            "jump_options_html": render_jump_select_options(landing_rel, locale, config),
        }
    return landing_data


def render_landing_page(config, rewrite_markdown_link):
    landing_data = build_landing_locale_data(config, rewrite_markdown_link)
    copy = landing_data["en"]
    body = f"""
<div class="landing-page">
  <section class="landing-hero">
    <div class="landing-hero-inner">
      <h1 id="landing-title" class="landing-title">{html.escape(copy["hero_title"])}</h1>
      <p id="landing-summary" class="landing-summary">{html.escape(copy["hero_summary"])}</p>
      <div id="landing-hero-links">{copy["hero_links_html"]}</div>
    </div>
  </section>
  <div id="landing-sections" class="landing-sections">{copy["sections_html"]}</div>
  <div id="landing-meta" class="landing-meta">{copy["meta_html"]}</div>
  <script id="landing-i18n" type="application/json">{json.dumps(landing_data, ensure_ascii=False)}</script>
</div>
"""
    return page_shell(page_title="Grafana Documentation", html_lang="en", home_href=relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "index.html")), hero_title="", hero_summary="", breadcrumbs=[], body_html=body, toc_html="", related_html="", version_html="", locale_html="", footer_nav_html="", footer_html="Generated by scripts/generate_command_html.py", jump_html=render_landing_locale_select("auto") + render_jump_select(prefixed_output_rel(config, "index.html"), "en", config), nav_html="", is_landing=True)
