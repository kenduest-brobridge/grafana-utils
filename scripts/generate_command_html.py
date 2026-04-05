#!/usr/bin/env python3
"""Generate the static HTML docs site from handbook and command Markdown."""

from __future__ import annotations

import argparse
import html
import json
import re
from dataclasses import dataclass, replace
from functools import lru_cache
from pathlib import Path
from string import Template

from docgen_command_docs import RenderedHeading, parse_command_page, render_markdown_document
from docgen_common import REPO_ROOT, VERSION, check_outputs, print_written_outputs, relative_href, write_outputs
from docgen_handbook import (
    HANDBOOK_LOCALES,
    HANDBOOK_NAV_GROUP_LABELS,
    HANDBOOK_NAV_GROUPS,
    LOCALE_LABELS,
    build_handbook_pages,
    existing_handbook_files,
    handbook_language_href,
)
from docgen_landing import LANDING_LOCALES, LANDING_UI_LABELS, LandingLink, LandingSection, LandingTask, load_landing_page
from generate_manpages import NAMESPACE_SPECS, generate_manpages

HTML_ROOT_DIR = REPO_ROOT / "docs" / "html"
COMMAND_DOCS_ROOT = REPO_ROOT / "docs" / "commands"
HANDBOOK_ROOT = REPO_ROOT / "docs" / "user-guide"
TEMPLATE_ROOT = REPO_ROOT / "scripts" / "templates"
COMMAND_DOC_LOCALES = ("en", "zh-TW")

@dataclass(frozen=True)
class VersionLink:
    label: str
    target_rel: str

@dataclass(frozen=True)
class HtmlBuildConfig:
    source_root: Path = REPO_ROOT
    command_docs_root: Path = COMMAND_DOCS_ROOT
    handbook_root: Path = HANDBOOK_ROOT
    output_prefix: str = ""
    version: str = VERSION
    version_label: str | None = None
    version_links: tuple[VersionLink, ...] = ()
    raw_manpage_target_rel: str | None = None
    include_raw_manpages: bool = False


@lru_cache(maxsize=None)
def load_template(name: str) -> Template:
    return Template((TEMPLATE_ROOT / name).read_text(encoding="utf-8"))


@lru_cache(maxsize=None)
def load_asset(name: str) -> str:
    return (TEMPLATE_ROOT / name).read_text(encoding="utf-8")


def render_template(name: str, **values: str) -> str:
    return load_template(name).substitute(**values)

HANDBOOK_CONTEXT_BY_COMMAND = {
    "index": "index",
    "dashboard": "dashboard",
    "datasource": "datasource",
    "alert": "alert",
    "access": "access",
    "change": "change-overview-status",
    "status": "change-overview-status",
    "overview": "change-overview-status",
    "snapshot": "change-overview-status",
    "profile": "getting-started",
}

COMMAND_AUDIENCE_HINTS = {
    "dashboard": {
        "en": "Best for SREs, Grafana operators, and responders working with dashboard inventory, migration, inspection, or screenshots.",
        "zh-TW": "適合 SRE、Grafana 維運人員，以及要處理 dashboard 盤點、搬遷、檢查或截圖的人。",
    },
    "datasource": {
        "en": "Best for operators who manage Grafana data source configuration, dependency checks, and recovery paths.",
        "zh-TW": "適合要管理 Grafana data source 設定、依賴檢查與復原流程的維運人員。",
    },
    "alert": {
        "en": "Best for operators who review alert rules, routes, contact points, and staged alert changes.",
        "zh-TW": "適合要檢查告警規則、通知路由、contact point 與 staged 變更的人。",
    },
    "access": {
        "en": "Best for administrators who work with org, user, team, service account, and token lifecycle operations.",
        "zh-TW": "適合要管理 org、使用者、team、service account 與 token 生命週期的管理者。",
    },
    "profile": {
        "en": "Best for anyone setting up repeatable connection defaults, secret handling, and non-interactive runs.",
        "zh-TW": "適合想整理可重複連線預設、secret 處理與非互動式執行方式的人。",
    },
    "status": {
        "en": "Best for operators who need fast live or staged readiness checks before they change anything.",
        "zh-TW": "適合想在動手前先做 live 或 staged readiness 檢查的人。",
    },
    "overview": {
        "en": "Best for readers who need a fast cross-surface inventory and health overview of the current Grafana estate.",
        "zh-TW": "適合想快速盤點目前 Grafana 環境、先看健康度與資產概況的人。",
    },
    "change": {
        "en": "Best for review-first workflows where you want summary, preflight, plan, and apply to stay explicit.",
        "zh-TW": "適合 review-first 流程，想把 summary、preflight、plan 與 apply 清楚分開的人。",
    },
    "snapshot": {
        "en": "Best for readers who need a local snapshot bundle for offline review, backup, or handoff work.",
        "zh-TW": "適合需要建立本機 snapshot bundle，做離線檢視、備份或交接的人。",
    },
}

PAGE_STYLE = load_asset("docs.css") + "\n" + load_asset("prism.css")

THEME_SCRIPT = load_asset("theme_script.js")

def strip_decorative_prefix(text: str) -> str:
    if not text:
        return text
    stripped = text.strip()
    while True:
        match = re.match(r"^([^\w\s\u4e00-\u9fffA-Za-z]+)\s*(.*)$", stripped, flags=re.UNICODE)
        if not match:
            break
        prefix = match.group(1)
        if any(char.isalnum() or ("\u4e00" <= char <= "\u9fff") for char in prefix):
            break
        stripped = match.group(2).lstrip()
    return stripped or text.strip()

def strip_heading_decorations(body_html: str) -> str:
    def replace_heading(match: re.Match[str]) -> str:
        level = match.group(1)
        attrs = match.group(2)
        content = match.group(3)
        cleaned = re.sub(r"^([^\w\s\u4e00-\u9fffA-Za-z<]+)\s*", "", content, flags=re.UNICODE)
        return f"<h{level}{attrs}>{cleaned}</h{level}>"
    return re.sub(r"<h([1-6])([^>]*)>(.*?)</h\1>", replace_heading, body_html, flags=re.DOTALL)

def title_only(text: str) -> str:
    return strip_decorative_prefix(text.replace("`", "")) if text else "grafana-util docs"

def html_list(items: list[tuple[str, str]]) -> str:
    if not items:
        return '<p class="sidebar-meta-text">No related links.</p>'
    rendered: list[str] = []
    for label, href in items:
        classes = "sidebar-meta-link"
        if href == "#":
            classes += " sidebar-meta-current"
        rendered.append(f'<li><a href="{html.escape(href)}" class="{classes}">{html.escape(label)}</a></li>')
    return '<ul class="sidebar-meta-list">' + "".join(rendered) + "</ul>"

def render_breadcrumbs(items: list[tuple[str, str | None]]) -> str:
    if not items: return ""
    links = []
    for l, h in items:
        if h: links.append(f'<a href="{html.escape(h)}">{html.escape(l)}</a>')
        else: links.append(html.escape(l))
    return render_template("breadcrumbs.html.tmpl", items_html=" / ".join(links))

def split_display_title(title: str) -> tuple[str, str | None]:
    title = strip_decorative_prefix(title)
    match = re.fullmatch(r"(.+?)\s*\(([^()]+)\)\s*", title)
    if not match:
        return title, None
    main = match.group(1).strip()
    secondary = match.group(2).strip()
    if re.search(r"[\u4e00-\u9fff]", main) and re.search(r"[A-Za-z]", secondary):
        return main, secondary
    return title, None

def strip_leading_h1(body_html: str) -> str:
    return re.sub(r'^\s*<h1\b[^>]*>.*?</h1>\s*', "", body_html, count=1, flags=re.DOTALL)

def split_leading_symbol(label: str) -> tuple[str | None, str]:
    match = re.match(r"^\s*([^\w\s]+)\s+(.+)$", label, flags=re.UNICODE)
    if not match:
        return None, label
    return match.group(1), match.group(2).strip()

def render_toc(headings: tuple[RenderedHeading, ...]) -> str:
    entries = [(h.level, strip_decorative_prefix(h.text), f"#{h.anchor}") for h in headings if h.level in (2, 3)]
    if not entries:
        return "<p style='color:var(--muted); font-size:0.85rem;'>No subsection anchors.</p>"
    items: list[str] = []
    for level, label, href in entries:
        label_html = f'<span class="toc-label">{html.escape(label)}</span>'
        items.append(
            f'<li class="toc-level-{level}"><a href="{html.escape(href)}">{label_html}</a></li>'
        )
    return '<ul class="toc-list">' + "".join(items) + "</ul>"


def render_section_index(
    headings: tuple[RenderedHeading, ...],
    *,
    title: str,
    summary: str = "",
    levels: tuple[int, ...] = (2,),
) -> str:
    entries = [
        (strip_decorative_prefix(h.text), f"#{h.anchor}")
        for h in headings
        if h.level in levels
    ]
    if not entries:
        return ""
    intro_html = f'<p class="section-index-summary">{html.escape(summary)}</p>' if summary else ""
    items_html = "".join(
        f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>'
        for label, href in entries
    )
    return (
        '<section class="section-index">'
        f'<h2>{html.escape(title)}</h2>'
        f"{intro_html}"
        f'<ul class="section-index-list">{items_html}</ul>'
        "</section>"
    )

def prefixed_output_rel(config: HtmlBuildConfig, rel: str) -> str:
    return f"{config.output_prefix.strip('/')}/{rel}" if config.output_prefix else rel

def render_version_links(output_rel: str, config: HtmlBuildConfig) -> str:
    items = []
    if config.version_label: items.append((f"Current: {config.version_label}", "#"))
    for link in config.version_links: items.append((link.label, relative_href(output_rel, link.target_rel)))
    return html_list(items) if items else '<p class="sidebar-meta-text">Current checkout</p>'

def handbook_nav_titles(locale: str, config: HtmlBuildConfig) -> dict[str, str]:
    pages = build_handbook_pages(locale, handbook_root=config.handbook_root)
    titles: dict[str, str] = {}
    for page in pages:
        stem = Path(page.output_rel).stem
        titles[stem] = format_handbook_nav_label(page.title, locale, stem)
    return titles


def handbook_nav_groups(locale: str, config: HtmlBuildConfig) -> list[tuple[str, list[tuple[str, str, str]]]]:
    titles = handbook_nav_titles(locale, config)
    grouped: list[tuple[str, list[tuple[str, str, str]]]] = []
    labels = HANDBOOK_NAV_GROUP_LABELS[locale]
    for group_key, filenames in HANDBOOK_NAV_GROUPS:
        items: list[tuple[str, str, str]] = []
        for filename in filenames:
            stem = Path(filename).stem
            if stem not in titles:
                continue
            target = prefixed_output_rel(config, f"handbook/{locale}/{stem}.html")
            items.append((stem, titles[stem], target))
        if items:
            grouped.append((labels[group_key], items))
    return grouped

def format_handbook_nav_label(title: str, locale: str, stem: str) -> str:
    if stem == "index":
        return "Overview" if locale == "en" else "概觀"
    clean = re.sub(r"^[^\w\u4e00-\u9fffA-Za-z]+", "", title).strip()
    main, secondary = split_display_title(clean)
    if locale == "zh-TW" and secondary:
        return main
    return clean or title

def command_namespace_label(spec) -> str:
    root = Path(spec.root_doc).stem.replace("-", " ")
    return " ".join(part.capitalize() for part in root.split())

def render_jump_select_options(output_rel: str, locale: str, config: HtmlBuildConfig) -> str:
    handbook_label = "Handbook" if locale == "en" else "使用手冊"
    commands_label = "Commands" if locale == "en" else "指令參考"
    prompt_label = "Jump to..." if locale == "en" else "快速跳轉..."
    handbook_titles = handbook_nav_titles(locale, config)
    sections = [f'<option value="" selected>{html.escape(prompt_label)}</option>', f'<optgroup label="{handbook_label}">']
    for name in existing_handbook_files(locale, handbook_root=config.handbook_root):
        stem = Path(name).stem
        label = handbook_titles.get(stem, (stem.replace("-", " ").title()) if stem != "index" else ("Overview" if locale == "en" else "概觀"))
        sections.append(f'<option value="{html.escape(relative_href(output_rel, prefixed_output_rel(config, f"handbook/{locale}/{stem}.html")))}">{html.escape(label)}</option>')
    sections.append(f'</optgroup><optgroup label="{commands_label}">')
    for spec in NAMESPACE_SPECS:
        sections.append(
            f'<option value="{html.escape(relative_href(output_rel, prefixed_output_rel(config, f"commands/{locale}/{spec.root_doc[:-3]}.html")))}">{html.escape(command_namespace_label(spec))}</option>'
        )
    sections.append('</optgroup>')
    return "".join(sections)

def render_jump_select(output_rel: str, locale: str, config: HtmlBuildConfig) -> str:
    return f'<select id="jump-select" aria-label="Jump">{render_jump_select_options(output_rel, locale, config)}</select>'

def render_landing_locale_select(current_locale: str = "en") -> str:
    selected_auto = ' selected' if current_locale == "auto" else ""
    selected_en = ' selected' if current_locale == "en" else ""
    selected_zh = ' selected' if current_locale == "zh-TW" else ""
    return (
        '<select id="locale-select" aria-label="Language">'
        f'<option value="auto"{selected_auto}>Auto</option>'
        f'<option value="en"{selected_en}>English</option>'
        f'<option value="zh-TW"{selected_zh}>繁體中文</option>'
        "</select>"
    )


def render_page_locale_select(current_label: str, switch_label: str | None = None, switch_href: str | None = None) -> str:
    options = [f'<option value="" selected>Language: {html.escape(current_label)}</option>']
    if switch_label and switch_href:
        options.append(f'<option value="{html.escape(switch_href)}">Switch to {html.escape(switch_label)}</option>')
    return f'<select id="page-locale-select" aria-label="Language switch">{"".join(options)}</select>'

def render_global_nav(output_rel: str, locale: str, config: HtmlBuildConfig) -> str:
    if not output_rel or "index.html" in output_rel and "/" not in output_rel.replace("index.html", ""): return ""
    handbook_label = "Guide Map" if locale == "en" else "手冊導覽"
    commands_label = "Command Entry" if locale == "en" else "指令入口"
    sections = []
    group_html: list[str] = []
    for group_label, items in handbook_nav_groups(locale, config):
        group_items: list[str] = []
        for stem, label, target in items:
            href = relative_href(output_rel, target)
            is_active = output_rel == target
            active = " active" if is_active else ""
            group_items.append(f'<li class="nav-tree-node"><a href="{html.escape(href)}" class="nav-item{active}">{html.escape(label)}</a></li>')
        group_html.append(f'<div class="nav-group-block"><div class="nav-group-title">{html.escape(group_label)}</div><ul class="nav-tree">{"".join(group_items)}</ul></div>')
    sections.append(f'<section class="nav-section"><h2>{handbook_label}</h2>{"".join(group_html)}</section>')
    command_target = prefixed_output_rel(config, f"commands/{locale}/index.html")
    command_href = relative_href(output_rel, command_target)
    command_active = " active" if output_rel == command_target or f"commands/{locale}/" in output_rel else ""
    command_label = "Open command docs" if locale == "en" else "開啟指令參考"
    sections.append(
        f'<section class="nav-section"><h2>{commands_label}</h2><a href="{html.escape(command_href)}" class="nav-command-entry{command_active}">{html.escape(command_label)}</a></section>'
    )
    return render_template("nav_sidebar.html.tmpl", sections_html="".join(sections))

def page_shell(*, page_title, html_lang, home_href, hero_title, hero_summary, breadcrumbs, body_html, toc_html, related_html, version_html, locale_html, footer_nav_html, footer_html, jump_html="", nav_html="", is_landing=False):
    header_html = ""
    if hero_title and not is_landing:
        summary_html = f'<p class="hero-summary">{hero_summary}</p>' if hero_summary else ""
        title_main, title_secondary = split_display_title(hero_title)
        title_html = f'<span class="hero-title-main">{html.escape(title_main)}</span>'
        if title_secondary:
            title_html += f'<span class="hero-title-secondary">{html.escape(title_secondary)}</span>'
        header_html = render_template("page_header.html.tmpl", title_html=title_html, summary_html=summary_html)
    
    sidebar_html = ""
    if not is_landing:
        sidebar_sections: list[str] = []
        if toc_html:
            sidebar_sections.append(f'<section class="sidebar-section"><h2>On This Page</h2>{toc_html}</section>')
        if related_html:
            sidebar_sections.append(f'<section class="sidebar-section"><h2>Related</h2>{related_html}</section>')
        if version_html:
            sidebar_sections.append(f'<section class="sidebar-section"><h2>Version</h2>{version_html}</section>')
        if locale_html:
            sidebar_sections.append(f'<section class="sidebar-section"><h2>Language</h2>{locale_html}</section>')
        if sidebar_sections:
            sidebar_html = render_template(
                "right_sidebar.html.tmpl",
                sections_html="".join(sidebar_sections),
            )
    
    content = ""
    if is_landing:
        content = body_html
    else:
        layout_class = "layout"
        if not nav_html:
            layout_class += " layout-no-nav"
        if not sidebar_html:
            layout_class += " layout-no-sidebar"
        content = render_template(
            "article_layout.html.tmpl",
            layout_class=layout_class,
            nav_html=nav_html,
            breadcrumbs_html=render_breadcrumbs(breadcrumbs),
            header_html=header_html,
            body_html=body_html,
            footer_nav_html=footer_nav_html,
            sidebar_html=sidebar_html,
        )

    controls_html = jump_html
    topbar_html = render_template(
        "topbar.html.tmpl",
        home_href=html.escape(home_href),
        controls_html=controls_html,
    )

    return render_template(
        "base.html.tmpl",
        html_lang=html.escape(html_lang),
        page_title=html.escape(page_title),
        page_style=PAGE_STYLE,
        landing_shell_class=" landing-shell" if is_landing else "",
        topbar_html=topbar_html,
        content=content,
        footer_html=footer_html,
        theme_script=THEME_SCRIPT,
    )

def command_intro_labels(locale: str) -> dict[str, str]:
    if locale == "zh-TW":
        return {
            "what": "這頁在說什麼",
            "when": "什麼時候看這頁",
            "who": "適合誰",
        }
    return {
        "what": "What this page covers",
        "when": "When to open this page",
        "who": "Who this page is for",
    }


def command_audience_hint(locale: str, source_path: Path) -> str:
    root = source_path.stem.split("-", 1)[0]
    hints = COMMAND_AUDIENCE_HINTS.get(root)
    if not hints:
        return ""
    return hints.get(locale, "")


def render_command_intro(locale: str, source_path: Path) -> str:
    try:
        parsed = parse_command_page(source_path, "grafana-util " + source_path.stem.replace("-", " "))
    except Exception:
        return ""
    labels = command_intro_labels(locale)
    blocks: list[str] = []
    if parsed.purpose:
        blocks.append(render_template(
            "command_intro_block.html.tmpl",
            label=html.escape(labels["what"]),
            content_html=f'<p class="command-intro-text">{html.escape(parsed.purpose)}</p>',
        )
        )
    when_html = ""
    if parsed.when_lines:
        items = "".join(f"<li>{html.escape(line.removeprefix('- ').strip())}</li>" for line in parsed.when_lines)
        when_html = f'<ul class="command-intro-list">{items}</ul>'
    elif parsed.when:
        when_html = f'<p class="command-intro-text">{html.escape(parsed.when)}</p>'
    if when_html:
        blocks.append(render_template(
            "command_intro_block.html.tmpl",
            label=html.escape(labels["when"]),
            content_html=when_html,
        )
        )
    audience = command_audience_hint(locale, source_path)
    if audience:
        blocks.append(render_template(
            "command_intro_block.html.tmpl",
            label=html.escape(labels["who"]),
            content_html=f'<p class="command-intro-text">{html.escape(audience)}</p>',
        )
        )
    if not blocks:
        return ""
    return render_template("command_intro.html.tmpl", blocks_html="".join(blocks))

def render_footer_nav(prev, nxt):
    cards = []
    if prev: cards.append(f'<a class="link-card" href="{html.escape(prev[1])}"><span>Previous</span>{html.escape(prev[0])}</a>')
    if nxt: cards.append(f'<a class="link-card" href="{html.escape(nxt[1])}"><span>Next</span>{html.escape(nxt[0])}</a>')
    return '<nav class="footer-nav">' + "".join(cards) + "</nav>" if cards else ""

def render_language_links(cur, sw_l, sw_h):
    items = [(f"Current: {cur}", "#")]
    if sw_l and sw_h: items.append((f"Switch to {sw_l}", sw_h))
    return html_list(items)

def command_reference_root_for_stem(stem, config):
    if stem == "grafana-util": return prefixed_output_rel(config, "commands/en/index.html")
    for spec in NAMESPACE_SPECS:
        if spec.stem == stem: return prefixed_output_rel(config, f"commands/en/{spec.root_doc[:-3]}.html")
    return None

def render_manpage_index_page(output_rel, names, config):
    body = (
        "<p>This lane mirrors the checked-in generated manpages as browser-readable HTML for local browsing and GitHub Pages.</p>"
        + "<ul>"
        + "".join(
            f'<li><a href="{html.escape(relative_href(output_rel, prefixed_output_rel(config, f"man/{Path(n).stem}.html")))}">{html.escape(n)}</a></li>'
            for n in names
        )
        + "</ul>"
    )
    related = [
        ("English handbook", relative_href(output_rel, prefixed_output_rel(config, "handbook/en/index.html"))),
        ("English command reference", relative_href(output_rel, prefixed_output_rel(config, "commands/en/index.html"))),
    ]
    if config.raw_manpage_target_rel:
        related.append(("Top-level roff manpage", relative_href(output_rel, config.raw_manpage_target_rel)))
    return page_shell(
        page_title="Manpages",
        html_lang="en",
        home_href=relative_href(output_rel, prefixed_output_rel(config, "index.html")),
        hero_title="Generated Manpages",
        hero_summary="Browser-readable HTML mirrors of the generated roff pages.",
        breadcrumbs=[("Home", relative_href(output_rel, prefixed_output_rel(config, "index.html"))), ("Manpages", None)],
        body_html=body,
        toc_html="<p>Open a generated manpage mirror or jump back to the command-reference lane.</p>",
        related_html=html_list(related),
        version_html=render_version_links(output_rel, config),
        locale_html="<p>English only. This lane currently generates from English command docs.</p>",
        footer_nav_html="",
        footer_html='Generated from <code>docs/man/*.1</code> via <code>scripts/generate_manpages.py</code>.',
        jump_html=render_jump_select(output_rel, "en", config),
        nav_html=render_global_nav(output_rel, "en", config),
    )


ROFF_FONT_TOKEN_RE = re.compile(r"\\f[BRI]|\\fR")


def normalize_roff_text(text: str) -> str:
    return text.replace(r"\-", "-").replace(r"\(bu", "•")


def render_roff_inline(text: str) -> str:
    pieces = []
    stack = []
    cursor = 0
    for match in ROFF_FONT_TOKEN_RE.finditer(text):
        if match.start() > cursor:
            pieces.append(html.escape(normalize_roff_text(text[cursor:match.start()])))
        token = match.group(0)
        if token == r"\fB":
            pieces.append("<strong>")
            stack.append("strong")
        elif token == r"\fI":
            pieces.append("<em>")
            stack.append("em")
        elif token == r"\fR" and stack:
            pieces.append(f"</{stack.pop()}>")
        cursor = match.end()
    if cursor < len(text):
        pieces.append(html.escape(normalize_roff_text(text[cursor:])))
    while stack:
        pieces.append(f"</{stack.pop()}>")
    return "".join(pieces)


def render_roff_macro_text(line: str) -> str:
    if line.startswith(".B "):
        return f"<strong>{render_roff_inline(line[3:])}</strong>"
    if line.startswith(".I "):
        return f"<em>{render_roff_inline(line[3:])}</em>"
    return render_roff_inline(line)


def render_roff_manpage_html(roff_text_body: str) -> str:
    body_parts = []
    section_parts = []
    paragraph_lines = []
    bullet_items = []
    definition_items = []
    definition_term = None
    definition_desc = []
    code_lines = []
    current_heading = None
    in_code_block = False
    pending_bullet = False
    expecting_definition_term = False

    def flush_paragraph():
        nonlocal paragraph_lines
        if paragraph_lines:
            section_parts.append(
                "<p>"
                + " ".join(
                    render_roff_macro_text(line) if line.startswith((".B ", ".I ")) else render_roff_inline(line)
                    for line in paragraph_lines
                )
                + "</p>"
            )
            paragraph_lines = []

    def flush_bullets():
        nonlocal bullet_items
        if bullet_items:
            section_parts.append('<ul class="man-bullets">' + "".join(f"<li>{item}</li>" for item in bullet_items) + "</ul>")
            bullet_items = []

    def flush_definition():
        nonlocal definition_term, definition_desc
        if definition_term is not None:
            definition_items.append((definition_term, " ".join(render_roff_inline(line) for line in definition_desc).strip()))
            definition_term = None
            definition_desc = []

    def flush_definitions():
        nonlocal definition_items
        flush_definition()
        if definition_items:
            section_parts.append(
                '<dl class="man-definitions">'
                + "".join(f"<dt>{term}</dt><dd>{desc}</dd>" for term, desc in definition_items)
                + "</dl>"
            )
            definition_items = []

    def flush_code():
        nonlocal code_lines
        if code_lines:
            section_parts.append(f'<pre class="man-example"><code>{html.escape(chr(10).join(code_lines))}</code></pre>')
            code_lines = []

    def flush_section_content():
        flush_paragraph()
        flush_bullets()
        flush_definitions()
        flush_code()

    def emit_section():
        nonlocal section_parts
        flush_section_content()
        if current_heading is not None:
            body_parts.append(f'<section class="man-section"><h2>{html.escape(current_heading)}</h2>{"".join(section_parts)}</section>')
            section_parts = []

    for raw_line in roff_text_body.splitlines():
        line = raw_line.rstrip()
        if in_code_block:
            if line == ".EE":
                in_code_block = False
                flush_code()
            else:
                code_lines.append(line)
            continue
        if pending_bullet:
            bullet_items.append(render_roff_inline(line))
            pending_bullet = False
            continue
        if expecting_definition_term:
            definition_term = render_roff_macro_text(line)
            definition_desc = []
            expecting_definition_term = False
            continue
        if line.startswith('.\\"') or line.startswith(".TH"):
            continue
        if line.startswith(".SH "):
            emit_section()
            current_heading = normalize_roff_text(line[4:])
            continue
        if line == ".PP":
            flush_paragraph()
            flush_bullets()
            flush_definitions()
            continue
        if line.startswith(".IP "):
            flush_paragraph()
            flush_definitions()
            pending_bullet = True
            continue
        if line == ".TP":
            flush_paragraph()
            flush_bullets()
            flush_definition()
            expecting_definition_term = True
            continue
        if line == ".EX":
            flush_paragraph()
            flush_bullets()
            flush_definitions()
            in_code_block = True
            code_lines = []
            continue
        if definition_term is not None:
            definition_desc.append(line)
        else:
            paragraph_lines.append(line)

    emit_section()
    if not body_parts and section_parts:
        body_parts.extend(section_parts)
    return '<div class="manpage-rendered">' + "".join(body_parts) + "</div>"

def render_manpage_page(output_rel, name, roff, config):
    stem = Path(name).stem
    command_root = command_reference_root_for_stem(stem, config)
    related = [("Manpage index", relative_href(output_rel, prefixed_output_rel(config, "man/index.html")))]
    if config.raw_manpage_target_rel:
        related.append(("Raw roff source", relative_href(output_rel, prefixed_output_rel(config, f"man/{name}"))))
    if command_root:
        related.append(("Matching command reference", relative_href(output_rel, command_root)))
    body = render_template(
        "manpage_intro.html.tmpl",
        summary_html=(
            "This page renders the generated roff manpage into readable HTML for browser use. "
            "For richer per-subcommand detail, prefer the command-reference pages."
        ),
        body_html=render_roff_manpage_html(roff),
    )
    return page_shell(
        page_title=name,
        html_lang="en",
        home_href=relative_href(output_rel, prefixed_output_rel(config, "index.html")),
        hero_title=name,
        hero_summary="Browser-readable rendering of a generated roff manpage.",
        breadcrumbs=[
            ("Home", relative_href(output_rel, prefixed_output_rel(config, "index.html"))),
            ("Manpages", relative_href(output_rel, prefixed_output_rel(config, "man/index.html"))),
            (name, None),
        ],
        body_html=body,
        toc_html="<p>This page renders the generated manpage into readable HTML sections.</p>",
        related_html=html_list(related),
        version_html=render_version_links(output_rel, config),
        locale_html="<p>English only. The manpage lane currently generates from English command docs.</p>",
        footer_nav_html="",
        footer_html=f'Source: <code>docs/man/{html.escape(name)}</code><br>Generated by <code>scripts/generate_command_html.py</code>.',
        jump_html=render_jump_select(output_rel, "en", config),
        nav_html=render_global_nav(output_rel, "en", config),
    )

def landing_panel_html(title: str, summary: str, links: list[tuple[str, str]]) -> str:
    links_html = "".join(f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>' for label, href in links)
    return render_template(
        "landing_panel.html.tmpl",
        title=html.escape(title),
        summary=html.escape(summary),
        links_html=links_html,
    )

def render_landing_links(source_path: Path, output_rel: str, links: tuple[LandingLink, ...], config: HtmlBuildConfig) -> list[tuple[str, str]]:
    return [(link.label, rewrite_markdown_link(source_path, output_rel, link.target, config)) for link in links]

def render_landing_task(source_path: Path, output_rel: str, task: LandingTask, config: HtmlBuildConfig) -> str:
    rendered_links = render_landing_links(source_path, output_rel, task.links, config)
    links_html = "".join(f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>' for label, href in rendered_links)
    return render_template(
        "landing_task.html.tmpl",
        title=html.escape(task.title),
        summary=html.escape(task.summary),
        links_html=links_html,
    )

def render_landing_section(source_path: Path, output_rel: str, section: LandingSection, config: HtmlBuildConfig) -> str:
    inline_html = ""
    inline_links = render_landing_links(source_path, output_rel, section.links, config)
    if inline_links:
        inline_html = (
            '<ul class="landing-inline-links">'
            + "".join(f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>' for label, href in inline_links)
            + "</ul>"
        )
    return render_template(
        "landing_section.html.tmpl",
        title=html.escape(section.title),
        summary=html.escape(section.summary),
        inline_html=inline_html,
        tasks_html="".join(render_landing_task(source_path, output_rel, task, config) for task in section.tasks),
    )

def build_landing_locale_data(config: HtmlBuildConfig) -> dict[str, dict[str, str]]:
    landing_rel = prefixed_output_rel(config, "index.html")
    landing_root = config.source_root / "docs" / "landing"
    version_links = [(config.version_label or config.version, "#")]
    version_links.extend((link.label, relative_href(landing_rel, link.target_rel)) for link in config.version_links)
    if config.include_raw_manpages or config.raw_manpage_target_rel:
        version_links.append(("Manpages", relative_href(landing_rel, prefixed_output_rel(config, "man/index.html"))))

    landing_data: dict[str, dict[str, str]] = {}
    for locale in LANDING_LOCALES:
        page = load_landing_page(locale, landing_root=landing_root)
        ui_labels = LANDING_UI_LABELS[locale]
        maintainer_links = render_landing_links(page.source_path, landing_rel, page.maintainer.links, config)
        meta_html = "".join(
            (
                landing_panel_html(page.maintainer.title, page.maintainer.summary, maintainer_links),
                landing_panel_html("Version" if locale == "en" else "版本", "Version switching is secondary here. Pick a language first, then jump release context if you need it." if locale == "en" else "版本切換是次要操作。先選語言，再視需要跳去特定版本內容。", version_links),
            )
        )
        landing_data[locale] = {
            "lang": locale,
            "hero_title": page.title,
            "hero_summary": page.summary,
            "search_heading": page.search.title,
            "search_copy": page.search.summary,
            "search_placeholder": ui_labels["search_placeholder"],
            "search_button": ui_labels["search_button"],
            "sections_html": "".join(render_landing_section(page.source_path, landing_rel, section, config) for section in page.sections),
            "meta_html": meta_html,
            "jump_options_html": render_jump_select_options(landing_rel, locale, config),
        }
    return landing_data

def render_landing_page(config):
    landing_data = build_landing_locale_data(config)
    default_locale = "en"
    copy = landing_data[default_locale]
    body = f"""
<div class="landing-page">
  <section class="landing-hero">
    <div class="landing-hero-inner">
      <h1 id="landing-title" class="landing-title">{html.escape(copy["hero_title"])}</h1>
      <p id="landing-summary" class="landing-summary">{html.escape(copy["hero_summary"])}</p>
    </div>
    <section class="landing-search-panel">
      <h2 id="landing-search-heading">{html.escape(copy["search_heading"])}</h2>
      <p id="landing-search-copy">{html.escape(copy["search_copy"])}</p>
      <form id="landing-search-form" class="landing-search-form">
        <input id="landing-search" class="landing-search-input" type="search" placeholder="{html.escape(copy['search_placeholder'])}" aria-label="{html.escape(copy['search_placeholder'])}" />
        <button id="landing-search-button" class="landing-search-button" type="submit">{html.escape(copy["search_button"])}</button>
      </form>
    </section>
  </section>
  <div id="landing-sections" class="landing-sections">{copy["sections_html"]}</div>
  <div id="landing-meta" class="landing-meta">{copy["meta_html"]}</div>
  <script id="landing-i18n" type="application/json">{json.dumps(landing_data, ensure_ascii=False)}</script>
</div>
"""
    return page_shell(
        page_title="grafana-util Docs",
        html_lang="en",
        home_href=relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "index.html")),
        hero_title="",
        hero_summary="",
        breadcrumbs=[],
        body_html=body,
        toc_html="",
        related_html="",
        version_html="",
        locale_html="",
        footer_nav_html="",
        footer_html="Generated by scripts/generate_command_html.py",
        jump_html=render_landing_locale_select("auto") + render_jump_select(prefixed_output_rel(config, "index.html"), default_locale, config),
        nav_html="",
        is_landing=True,
    )

def render_developer_page(config):
    dev_path = config.source_root / "docs" / "DEVELOPER.md"
    if not dev_path.exists(): return ""
    doc = render_markdown_document(dev_path.read_text(encoding="utf-8"), link_transform=lambda t: t)
    section_index = render_section_index(
        doc.headings,
        title="Guide Map",
        summary="Jump straight to the repo concern you are touching instead of scanning the whole maintainer guide.",
    )
    body_html = section_index + strip_heading_decorations(strip_leading_h1(doc.body_html))
    return page_shell(page_title="Developer Guide", html_lang="en", home_href=relative_href("developer.html", prefixed_output_rel(config, "index.html")), hero_title="Developer Guide", hero_summary="Maintainer routing for runtime, docs, contracts, and release work.", breadcrumbs=[("Home", "index.html"), ("Developer Guide", None)], body_html=body_html, toc_html=render_toc(doc.headings), related_html="", version_html="", locale_html="", footer_nav_html="", footer_html="Source: docs/DEVELOPER.md", jump_html=render_page_locale_select("English") + render_jump_select("developer.html", "en", config), nav_html="")

def command_language_switch_href(output_rel, locale, source_name, config):
    other = "zh-TW" if locale == "en" else "en"
    target = config.command_docs_root / other / source_name
    if not target.exists(): return None, None
    return LOCALE_LABELS[other], relative_href(output_rel, prefixed_output_rel(config, f"commands/{other}/{Path(source_name).with_suffix('.html').as_posix()}"))

def command_handbook_context(locale, output_rel, source_name, config):
    stem = Path(source_name).stem
    root = stem.split("-", 1)[0]
    h_stem = HANDBOOK_CONTEXT_BY_COMMAND.get(stem) or HANDBOOK_CONTEXT_BY_COMMAND.get(root)
    if not h_stem: return None
    return ("Matching handbook chapter", relative_href(output_rel, prefixed_output_rel(config, f"handbook/{locale}/{h_stem}.html")))

def rewrite_markdown_link(source_path, output_rel, target, config):
    if target.startswith(("http", "mailto:", "#")): return target
    bare, frag = (target.split("#", 1) + [""])[:2]
    res = (source_path.parent / bare).resolve()
    if res == (config.source_root / "docs" / "DEVELOPER.md").resolve():
        rel = "developer.html"
    else:
        try:
            rel = res.relative_to(config.source_root / "docs").as_posix()
        except Exception:
            return target
    if rel.startswith("commands/") and rel.endswith(".md"): rel = rel[:-3] + ".html"
    elif rel.startswith("user-guide/") and rel.endswith(".md"): rel = rel.replace("user-guide/", "handbook/", 1)[:-3] + ".html"
    return f"{relative_href(output_rel, prefixed_output_rel(config, rel))}#{frag}" if frag else relative_href(output_rel, prefixed_output_rel(config, rel))

def render_handbook_page(page, config):
    doc = render_markdown_document(page.source_path.read_text(encoding="utf-8"), link_transform=lambda t: rewrite_markdown_link(page.source_path, page.output_rel, t, config))
    title = title_only(doc.title or page.title)
    locale_href = handbook_language_href(page)
    locale_label = LOCALE_LABELS["zh-TW" if page.locale=="en" else "en"] if locale_href else None
    prev = (title_only(page.previous_title), relative_href(page.output_rel, page.previous_output_rel)) if page.previous_output_rel else None
    nxt = (title_only(page.next_title), relative_href(page.output_rel, page.next_output_rel)) if page.next_output_rel else None
    return page_shell(page_title=title, html_lang=page.locale, home_href=relative_href(page.output_rel, prefixed_output_rel(config, "index.html")), hero_title=title, hero_summary="", breadcrumbs=[("Home", relative_href(page.output_rel, "index.html")), ("Handbook", None), (title, None)], body_html=strip_heading_decorations(strip_leading_h1(doc.body_html)), toc_html=render_toc(doc.headings), related_html="", version_html=render_version_links(page.output_rel, config), locale_html=render_language_links(LOCALE_LABELS[page.locale], locale_label, locale_href), footer_nav_html=render_footer_nav(prev, nxt), footer_html="Source: " + page.source_path.name, jump_html=render_page_locale_select(LOCALE_LABELS[page.locale], locale_label, locale_href) + render_jump_select(page.output_rel, page.locale, config), nav_html=render_global_nav(page.output_rel, page.locale, config))

def render_command_page(locale, source_path, output_rel, config):
    doc = render_markdown_document(source_path.read_text(encoding="utf-8"), link_transform=lambda t: rewrite_markdown_link(source_path, output_rel, t, config))
    title = title_only(doc.title or source_path.stem)
    sw_l, sw_h = command_language_switch_href(output_rel, locale, source_path.name, config)
    h_link = command_handbook_context(locale, output_rel, source_name=source_path.name, config=config)
    body_html = render_command_intro(locale, source_path) + strip_heading_decorations(strip_leading_h1(doc.body_html))
    return page_shell(page_title=title, html_lang=locale, home_href=relative_href(output_rel, prefixed_output_rel(config, "index.html")), hero_title=title, hero_summary="", breadcrumbs=[("Home", relative_href(output_rel, prefixed_output_rel(config, "index.html"))), ("Commands", None), (title, None)], body_html=body_html, toc_html=render_toc(doc.headings), related_html=html_list([h_link]) if h_link else "", version_html=render_version_links(output_rel, config), locale_html=render_language_links(LOCALE_LABELS[locale], sw_l, sw_h), footer_nav_html="", footer_html="Source: " + source_path.name, jump_html=render_page_locale_select(LOCALE_LABELS[locale], sw_l, sw_h) + render_jump_select(output_rel, locale, config), nav_html=render_global_nav(output_rel, locale, config))

def generate_outputs(config=HtmlBuildConfig()):
    outputs = {prefixed_output_rel(config, "index.html"): render_landing_page(config), ".nojekyll": ""}
    dev_html = render_developer_page(config)
    if dev_html: outputs[prefixed_output_rel(config, "developer.html")] = dev_html
    manpage_outputs = generate_manpages(command_docs_dir=config.command_docs_root / "en", version=config.version)
    outputs[prefixed_output_rel(config, "man/index.html")] = render_manpage_index_page(prefixed_output_rel(config, "man/index.html"), sorted(manpage_outputs), config)
    for name, roff in sorted(manpage_outputs.items()):
        outputs[prefixed_output_rel(config, f"man/{Path(name).stem}.html")] = render_manpage_page(prefixed_output_rel(config, f"man/{Path(name).stem}.html"), name, roff, config)
        if config.include_raw_manpages:
            outputs[prefixed_output_rel(config, f"man/{name}")] = roff
    for loc in COMMAND_DOC_LOCALES:
        for src in sorted((config.command_docs_root / loc).glob("*.md")):
            out_rel = prefixed_output_rel(config, f"commands/{loc}/{src.with_suffix('.html').name}")
            outputs[out_rel] = render_command_page(loc, src, out_rel, config)
    for loc in HANDBOOK_LOCALES:
        for page in build_handbook_pages(loc, handbook_root=config.handbook_root):
            page = replace(page, output_rel=prefixed_output_rel(config, page.output_rel), previous_output_rel=prefixed_output_rel(config, page.previous_output_rel) if page.previous_output_rel else None, next_output_rel=prefixed_output_rel(config, page.next_output_rel) if page.next_output_rel else None, language_switch_rel=prefixed_output_rel(config, page.language_switch_rel) if page.language_switch_rel else None)
            outputs[page.output_rel] = render_handbook_page(page, config)
    return outputs

def build_parser():
    parser = argparse.ArgumentParser(); parser.add_argument("--write", action="store_true"); parser.add_argument("--check", action="store_true"); return parser

def main(argv=None):
    args = build_parser().parse_args(argv)
    outputs = generate_outputs()
    if args.check: return check_outputs(HTML_ROOT_DIR, outputs, "HTML docs", "python3 scripts/generate_command_html.py --write")
    write_outputs(HTML_ROOT_DIR, outputs)
    print_written_outputs(HTML_ROOT_DIR, outputs, "HTML docs", "docs/*", "docs/html/*", "docs/html/index.html")
    return 0

if __name__ == "__main__": raise SystemExit(main())
