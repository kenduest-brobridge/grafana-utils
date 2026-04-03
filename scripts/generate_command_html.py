#!/usr/bin/env python3
"""Generate the static HTML docs site from handbook and command Markdown."""

from __future__ import annotations

import argparse
import html
from pathlib import Path

from docgen_command_docs import RenderedHeading, render_markdown_document
from docgen_common import REPO_ROOT, VERSION, check_outputs, print_written_outputs, relative_href, write_outputs
from docgen_handbook import HANDBOOK_LOCALES, LOCALE_LABELS, build_handbook_pages, handbook_language_href


HTML_ROOT_DIR = REPO_ROOT / "docs" / "html"
COMMAND_DOCS_ROOT = REPO_ROOT / "docs" / "commands"
COMMAND_DOC_LOCALES = ("en", "zh-TW")

# Keep this mapping explicit so maintainers can see how command-reference pages
# jump back to the handbook chapters that explain the broader workflow.
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

PAGE_STYLE = """
:root {
  color-scheme: light dark;
  --bg: linear-gradient(180deg, #f7f3eb 0%, #fcfbf8 100%);
  --panel: rgba(255, 255, 255, 0.82);
  --panel-strong: rgba(255, 255, 255, 0.92);
  --text: #1f2933;
  --muted: #52606d;
  --heading: #102a43;
  --accent: #0b6e4f;
  --accent-soft: #e8f1eb;
  --border: #d9e2ec;
  --code-bg: #f0f4f8;
  --pre-bg: #0f1720;
  --pre-text: #e6edf3;
  --shadow: 0 18px 45px rgba(15, 23, 32, 0.08);
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: linear-gradient(180deg, #0b1220 0%, #111827 100%);
    --panel: rgba(15, 23, 32, 0.86);
    --panel-strong: rgba(15, 23, 32, 0.94);
    --text: #d9e2ec;
    --muted: #9fb3c8;
    --heading: #f0f4f8;
    --accent: #7bdcb5;
    --accent-soft: rgba(18, 53, 40, 0.9);
    --border: #243b53;
    --code-bg: #1f2933;
    --pre-bg: #081018;
    --pre-text: #e6edf3;
    --shadow: 0 18px 45px rgba(0, 0, 0, 0.32);
  }
}
html[data-theme="light"] {
  color-scheme: light;
}
html[data-theme="dark"] {
  color-scheme: dark;
}
html[data-theme="light"] body {
  --bg: linear-gradient(180deg, #f7f3eb 0%, #fcfbf8 100%);
  --panel: rgba(255, 255, 255, 0.82);
  --panel-strong: rgba(255, 255, 255, 0.92);
  --text: #1f2933;
  --muted: #52606d;
  --heading: #102a43;
  --accent: #0b6e4f;
  --accent-soft: #e8f1eb;
  --border: #d9e2ec;
  --code-bg: #f0f4f8;
  --pre-bg: #0f1720;
  --pre-text: #e6edf3;
  --shadow: 0 18px 45px rgba(15, 23, 32, 0.08);
}
html[data-theme="dark"] body {
  --bg: linear-gradient(180deg, #0b1220 0%, #111827 100%);
  --panel: rgba(15, 23, 32, 0.86);
  --panel-strong: rgba(15, 23, 32, 0.94);
  --text: #d9e2ec;
  --muted: #9fb3c8;
  --heading: #f0f4f8;
  --accent: #7bdcb5;
  --accent-soft: rgba(18, 53, 40, 0.9);
  --border: #243b53;
  --code-bg: #1f2933;
  --pre-bg: #081018;
  --pre-text: #e6edf3;
  --shadow: 0 18px 45px rgba(0, 0, 0, 0.32);
}
* { box-sizing: border-box; }
body {
  margin: 0;
  font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", serif;
  color: var(--text);
  background: var(--bg);
}
a { color: var(--accent); }
code {
  font: 0.92em ui-monospace, SFMono-Regular, Menlo, monospace;
  background: var(--code-bg);
  padding: 0.12em 0.35em;
  border-radius: 4px;
}
pre {
  overflow-x: auto;
  padding: 16px 18px;
  border-radius: 14px;
  background: var(--pre-bg);
  color: var(--pre-text);
}
pre code {
  background: transparent;
  color: inherit;
  padding: 0;
}
table {
  width: 100%;
  border-collapse: collapse;
  margin: 22px 0;
  font-size: 0.98rem;
}
th, td {
  border: 1px solid var(--border);
  padding: 10px 12px;
  vertical-align: top;
}
th {
  text-align: left;
  background: var(--accent-soft);
}
.site {
  max-width: 1220px;
  margin: 0 auto;
  padding: 28px 22px 68px;
}
.topbar {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: center;
  margin-bottom: 18px;
}
.topbar a {
  color: inherit;
  text-decoration: none;
}
.brand {
  font: 700 0.96rem/1.2 ui-monospace, SFMono-Regular, Menlo, monospace;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.themebar {
  display: flex;
  gap: 8px;
  align-items: center;
  color: var(--muted);
  font: 600 12px/1.2 ui-monospace, SFMono-Regular, Menlo, monospace;
}
.themebar select {
  border: 1px solid var(--border);
  background: var(--panel-strong);
  color: var(--text);
  padding: 6px 10px;
  border-radius: 10px;
}
.hero {
  padding: 26px 28px;
  border: 1px solid var(--border);
  border-radius: 24px;
  background: var(--panel-strong);
  box-shadow: var(--shadow);
}
.eyebrow {
  display: inline-block;
  margin-bottom: 14px;
  padding: 6px 10px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
  font: 700 12px/1.2 ui-monospace, SFMono-Regular, Menlo, monospace;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.hero h1 {
  margin: 0;
  font-size: clamp(2rem, 4vw, 3.2rem);
  line-height: 1.05;
  color: var(--heading);
}
.hero p {
  max-width: 74ch;
  margin: 14px 0 0;
  font-size: 1.06rem;
  line-height: 1.8;
  color: var(--muted);
}
.breadcrumbs {
  margin: 20px 0 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  list-style: none;
  font-size: 0.96rem;
  color: var(--muted);
}
.breadcrumbs li::after {
  content: "/";
  margin-left: 10px;
}
.breadcrumbs li:last-child::after {
  content: "";
  margin: 0;
}
.layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 290px;
  gap: 22px;
  margin-top: 22px;
}
.panel {
  border: 1px solid var(--border);
  border-radius: 22px;
  background: var(--panel);
  box-shadow: var(--shadow);
}
.article {
  padding: 30px;
}
.article h1,
.article h2,
.article h3 {
  color: var(--heading);
}
.article h1 {
  margin-top: 0;
}
.article h2 {
  margin-top: 40px;
  padding-top: 22px;
  border-top: 1px solid var(--border);
}
.article h3 {
  margin-top: 28px;
}
.article p,
.article li {
  font-size: 1.03rem;
  line-height: 1.8;
}
.sidebar {
  padding: 22px;
}
.sidebar section + section {
  margin-top: 20px;
}
.sidebar h2 {
  margin: 0 0 10px;
  font-size: 0.96rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--muted);
}
.sidebar ul {
  list-style: none;
  margin: 0;
  padding: 0;
}
.sidebar li + li {
  margin-top: 8px;
}
.sidebar a {
  text-decoration: none;
}
.link-card {
  display: block;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--panel-strong);
}
.footer-nav {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
  margin-top: 24px;
}
.footer-nav .link-card span {
  display: block;
  color: var(--muted);
  font-size: 0.84rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.site-footer {
  margin-top: 24px;
  padding: 18px 22px;
  border: 1px solid var(--border);
  border-radius: 18px;
  background: var(--panel);
  color: var(--muted);
  font-size: 0.94rem;
}
.landing-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
  margin-top: 22px;
}
.landing-card {
  padding: 22px;
  border: 1px solid var(--border);
  border-radius: 22px;
  background: var(--panel);
  box-shadow: var(--shadow);
}
.landing-card h2 {
  margin-top: 0;
  color: var(--heading);
}
.landing-card ul {
  margin: 0;
  padding-left: 20px;
}
@media (max-width: 980px) {
  .layout,
  .landing-grid,
  .footer-nav {
    grid-template-columns: 1fr;
  }
}
""".strip()

THEME_SCRIPT = """
<script>
(() => {
  const storageKey = "grafana-util-docs-theme";
  const root = document.documentElement;
  const select = document.getElementById("theme-select");
  const saved = localStorage.getItem(storageKey) || "auto";
  const applyTheme = (value) => {
    if (value === "auto") {
      root.removeAttribute("data-theme");
    } else {
      root.setAttribute("data-theme", value);
    }
  };
  applyTheme(saved);
  if (select) {
    select.value = saved;
    select.addEventListener("change", (event) => {
      const value = event.target.value;
      localStorage.setItem(storageKey, value);
      applyTheme(value);
    });
  }
})();
</script>
""".strip()


def title_only(text: str) -> str:
    if not text:
        return "grafana-util docs"
    return text.replace("`", "")


def html_list(items: list[tuple[str, str]]) -> str:
    if not items:
        return "<p>No related links for this page.</p>"
    return "<ul>" + "".join(
        f'<li><a class="link-card" href="{html.escape(href)}">{html.escape(label)}</a></li>' for label, href in items
    ) + "</ul>"


def render_breadcrumbs(items: list[tuple[str, str | None]]) -> str:
    rendered = []
    for label, href in items:
        if href:
            rendered.append(f'<li><a href="{html.escape(href)}">{html.escape(label)}</a></li>')
        else:
            rendered.append(f"<li>{html.escape(label)}</li>")
    return '<ol class="breadcrumbs">' + "".join(rendered) + "</ol>"


def render_toc(headings: tuple[RenderedHeading, ...]) -> str:
    entries = [(heading.text, f"#{heading.anchor}") for heading in headings if heading.level in (2, 3)]
    return html_list(entries) if entries else "<p>This page has no subsection anchors.</p>"


def page_shell(
    *,
    page_title: str,
    html_lang: str,
    home_href: str,
    hero_title: str,
    hero_summary: str,
    eyebrow: str,
    breadcrumbs: list[tuple[str, str | None]],
    body_html: str,
    toc_html: str,
    related_html: str,
    locale_html: str,
    footer_nav_html: str,
    footer_html: str,
) -> str:
    return f"""<!DOCTYPE html>
<html lang="{html.escape(html_lang)}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{html.escape(page_title)} · grafana-util docs</title>
  <style>{PAGE_STYLE}</style>
</head>
<body>
  <div class="site">
    <div class="topbar">
      <a class="brand" href="{html.escape(home_href)}">grafana-util docs</a>
      <div class="themebar">
        <label for="theme-select">Theme</label>
        <select id="theme-select" aria-label="Theme">
          <option value="auto">Auto</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </div>
    </div>
    <header class="hero">
      <div class="eyebrow">{html.escape(eyebrow)}</div>
      <h1>{html.escape(hero_title)}</h1>
      <p>{hero_summary}</p>
      {render_breadcrumbs(breadcrumbs)}
    </header>
    <div class="layout">
      <article class="panel article">
        {body_html}
        {footer_nav_html}
      </article>
      <aside class="panel sidebar">
        <section>
          <h2>On This Page</h2>
          {toc_html}
        </section>
        <section>
          <h2>Related</h2>
          {related_html}
        </section>
        <section>
          <h2>Language</h2>
          {locale_html}
        </section>
      </aside>
    </div>
    <footer class="site-footer">{footer_html}</footer>
  </div>
  {THEME_SCRIPT}
</body>
</html>
"""


def handbook_intro_text(locale: str) -> str:
    if locale == "zh-TW":
        return "敘事式手冊頁面，適合照工作流閱讀，再跳到逐指令頁查精確命令面。"
    return "Narrative handbook chapters for learning the workflow first, then jumping to command-reference pages for exact syntax."


def command_intro_text(locale: str) -> str:
    if locale == "zh-TW":
        return "逐指令 reference，適合快速查 flags、examples、相鄰命令與對應手冊章節。"
    return "Stable command-reference pages for exact flags, examples, nearby commands, and the matching handbook context."


def render_footer_nav(previous_link: tuple[str, str] | None, next_link: tuple[str, str] | None) -> str:
    cards: list[str] = []
    if previous_link:
        cards.append(
            f'<a class="link-card" href="{html.escape(previous_link[1])}"><span>Previous</span>{html.escape(previous_link[0])}</a>'
        )
    if next_link:
        cards.append(f'<a class="link-card" href="{html.escape(next_link[1])}"><span>Next</span>{html.escape(next_link[0])}</a>')
    if not cards:
        return ""
    return '<nav class="footer-nav">' + "".join(cards) + "</nav>"


def render_language_links(current_label: str, switch_label: str | None, switch_href: str | None) -> str:
    items = [(f"Current: {current_label}", "#")]
    if switch_label and switch_href:
        items.append((f"Switch to {switch_label}", switch_href))
    return html_list(items)


def render_landing_page() -> str:
    body_html = """
<div class="landing-grid">
  <section class="landing-card">
    <div class="eyebrow">Choose Your Path</div>
    <h2>Start by role, not by file tree</h2>
    <p>Pick the path that matches your job to be done, then drop into the handbook, command reference, or maintainer docs from there.</p>
    <ul>
      <li><a href="./handbook/en/role-new-user.html">New user</a> · first-run setup, auth, and safe starter commands</li>
      <li><a href="./handbook/en/role-sre-ops.html">SRE / operator</a> · day-to-day estate operations, review-first change flows, and troubleshooting</li>
      <li><a href="./handbook/en/role-automation-ci.html">Automation / CI</a> · profile-backed automation, output formats, and scripting surfaces</li>
      <li><a href="../DEVELOPER.md">Maintainer / architect</a> · repo routing, generator design, contracts, and release flow</li>
    </ul>
  </section>
  <section class="landing-card">
    <div class="eyebrow">Locale Paths</div>
    <h2>Role guides in English and zh-TW</h2>
    <p>The public handbook keeps mirrored role-based entry pages in both locales so onboarding and operator guidance stay parallel.</p>
    <ul>
      <li><a href="./handbook/en/role-new-user.html">English role guides</a></li>
      <li><a href="./handbook/zh-TW/role-new-user.html">繁體中文角色導覽</a></li>
      <li><a href="./handbook/en/index.html">English handbook index</a></li>
      <li><a href="./handbook/zh-TW/index.html">繁體中文手冊目錄</a></li>
    </ul>
  </section>
</div>
<div class="landing-grid">
  <section class="landing-card">
    <div class="eyebrow">Handbook</div>
    <h2>Read the manual by workflow</h2>
    <p>Use the handbook when you want the operator story, the recommended order, and the why behind the command families.</p>
    <ul>
      <li><a href="./handbook/en/index.html">English handbook</a></li>
      <li><a href="./handbook/zh-TW/index.html">繁體中文手冊</a></li>
    </ul>
  </section>
  <section class="landing-card">
    <div class="eyebrow">Command Reference</div>
    <h2>Look up the exact CLI surface</h2>
    <p>Use the command reference when you want one page per command or subcommand, plus the generated manpage family.</p>
    <ul>
      <li><a href="./commands/en/index.html">English command reference</a></li>
      <li><a href="./commands/zh-TW/index.html">繁體中文逐指令說明</a></li>
      <li><a href="../man/grafana-util.1">Top-level manpage source</a></li>
    </ul>
  </section>
</div>
""".strip()
    footer_html = (
        "Source roots: <code>docs/user-guide/*</code> and <code>docs/commands/*</code>. "
        "Generated by <code>scripts/generate_command_html.py</code>."
    )
    return page_shell(
        page_title="grafana-util HTML docs",
        html_lang="en",
        home_href="./index.html",
        hero_title="grafana-util HTML Docs",
        hero_summary="Generated manual-style HTML with separate handbook and command-reference entrypoints.",
        eyebrow=f"Generated HTML · grafana-util {html.escape(VERSION)}",
        breadcrumbs=[("Home", None)],
        body_html=body_html,
        toc_html="<p>Start from one of the two main entrypoints.</p>",
        related_html=html_list(
            [
                ("New user role path", "./handbook/en/role-new-user.html"),
                ("SRE / operator role path", "./handbook/en/role-sre-ops.html"),
                ("Automation / CI role path", "./handbook/en/role-automation-ci.html"),
                ("English handbook", "./handbook/en/index.html"),
                ("繁體中文手冊", "./handbook/zh-TW/index.html"),
                ("English command reference", "./commands/en/index.html"),
                ("繁體中文逐指令說明", "./commands/zh-TW/index.html"),
                ("Maintainer entrypoint", "../DEVELOPER.md"),
            ]
        ),
        locale_html="<p>Each handbook and command-reference lane has English and zh-TW entrypoints.</p>",
        footer_nav_html="",
        footer_html=footer_html,
    )


def command_language_switch_href(output_rel: str, locale: str, source_name: str) -> tuple[str | None, str | None]:
    other_locale = next((candidate for candidate in COMMAND_DOC_LOCALES if candidate != locale), None)
    if other_locale is None:
        return None, None
    target_source = COMMAND_DOCS_ROOT / other_locale / source_name
    if not target_source.exists():
        return None, None
    target_rel = f"commands/{other_locale}/{Path(source_name).with_suffix('.html').as_posix()}"
    return LOCALE_LABELS[other_locale], relative_href(output_rel, target_rel)


def command_handbook_context(locale: str, output_rel: str, source_name: str) -> tuple[str, str] | None:
    stem = Path(source_name).stem
    root = stem.split("-", 1)[0]
    handbook_stem = HANDBOOK_CONTEXT_BY_COMMAND.get(stem) or HANDBOOK_CONTEXT_BY_COMMAND.get(root)
    if not handbook_stem:
        return None
    target_rel = f"handbook/{locale}/{handbook_stem}.html"
    return ("Matching handbook chapter", relative_href(output_rel, target_rel))


def rewrite_markdown_link(source_path: Path, output_rel: str, target: str) -> str:
    """Rewrite source-relative Markdown links so they work from docs/html."""
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return target
    bare_target, fragment = (target.split("#", 1) + [""])[:2]
    resolved = (source_path.parent / bare_target).resolve()
    docs_root = REPO_ROOT / "docs"
    try:
        docs_rel = resolved.relative_to(docs_root).as_posix()
    except ValueError:
        return target
    if docs_rel.startswith("commands/") and docs_rel.endswith(".md"):
        rewritten = relative_href(f"html/{output_rel}", f"html/{docs_rel[:-3]}.html")
        return f"{rewritten}#{fragment}" if fragment else rewritten
    if docs_rel.startswith("user-guide/") and docs_rel.endswith(".md"):
        docs_rel = docs_rel.replace("user-guide/", "handbook/", 1)
        rewritten = relative_href(f"html/{output_rel}", f"html/{docs_rel[:-3]}.html")
        return f"{rewritten}#{fragment}" if fragment else rewritten
    rewritten = relative_href(f"html/{output_rel}", docs_rel)
    return f"{rewritten}#{fragment}" if fragment else rewritten


def render_handbook_page(page) -> str:
    document = render_markdown_document(
        page.source_path.read_text(encoding="utf-8"),
        link_transform=lambda target: rewrite_markdown_link(page.source_path, page.output_rel, target),
    )
    page_title = title_only(document.title or page.title)
    breadcrumbs = [
        ("Home", relative_href(page.output_rel, "index.html")),
        ("Handbook", relative_href(page.output_rel, f"handbook/{page.locale}/index.html")),
        (LOCALE_LABELS[page.locale], None),
        (page_title, None),
    ]
    related_links = [
        ("Handbook home", relative_href(page.output_rel, f"handbook/{page.locale}/index.html")),
        ("Command reference index", relative_href(page.output_rel, f"commands/{page.locale}/index.html")),
    ]
    locale_href = handbook_language_href(page)
    locale_label = None
    if locale_href:
        other_locale = next(candidate for candidate in HANDBOOK_LOCALES if candidate != page.locale)
        locale_label = LOCALE_LABELS[other_locale]
    previous_link = None
    if page.previous_output_rel:
        previous_link = (title_only(page.previous_title or "Previous"), relative_href(page.output_rel, page.previous_output_rel))
    next_link = None
    if page.next_output_rel:
        next_link = (title_only(page.next_title or "Next"), relative_href(page.output_rel, page.next_output_rel))
    footer_html = (
        f'Source: <code>{html.escape(page.source_path.relative_to(REPO_ROOT).as_posix())}</code><br>'
        'Generated by <code>scripts/generate_command_html.py</code>.'
    )
    return page_shell(
        page_title=page_title,
        html_lang=page.locale,
        home_href=relative_href(page.output_rel, "index.html"),
        hero_title=page_title,
        hero_summary=handbook_intro_text(page.locale),
        eyebrow=f"Handbook · {LOCALE_LABELS[page.locale]}",
        breadcrumbs=breadcrumbs,
        body_html=document.body_html,
        toc_html=render_toc(document.headings),
        related_html=html_list(related_links),
        locale_html=render_language_links(LOCALE_LABELS[page.locale], locale_label, locale_href),
        footer_nav_html=render_footer_nav(previous_link, next_link),
        footer_html=footer_html,
    )


def render_command_page(locale: str, source_path: Path, output_rel: str) -> str:
    document = render_markdown_document(
        source_path.read_text(encoding="utf-8"),
        link_transform=lambda target: rewrite_markdown_link(source_path, output_rel, target),
    )
    page_title = title_only(document.title or source_path.stem)
    breadcrumbs = [
        ("Home", relative_href(output_rel, "index.html")),
        ("Command Reference", relative_href(output_rel, f"commands/{locale}/index.html")),
        (LOCALE_LABELS[locale], None),
        (page_title, None),
    ]
    related_links = [
        ("Command reference home", relative_href(output_rel, f"commands/{locale}/index.html")),
    ]
    handbook_link = command_handbook_context(locale, output_rel, source_path.name)
    if handbook_link:
        related_links.append(handbook_link)
    if locale == "en":
        related_links.append(("Top-level manpage", relative_href(f"html/{output_rel}", "man/grafana-util.1")))
    switch_label, switch_href = command_language_switch_href(output_rel, locale, source_path.name)
    footer_html = (
        f'Source: <code>{html.escape(source_path.relative_to(REPO_ROOT).as_posix())}</code><br>'
        'Generated by <code>scripts/generate_command_html.py</code>.'
    )
    return page_shell(
        page_title=page_title,
        html_lang=locale,
        home_href=relative_href(output_rel, "index.html"),
        hero_title=page_title,
        hero_summary=command_intro_text(locale),
        eyebrow=f"Command Reference · {LOCALE_LABELS[locale]}",
        breadcrumbs=breadcrumbs,
        body_html=document.body_html,
        toc_html=render_toc(document.headings),
        related_html=html_list(related_links),
        locale_html=render_language_links(LOCALE_LABELS[locale], switch_label, switch_href),
        footer_nav_html="",
        footer_html=footer_html,
    )


def generate_outputs() -> dict[str, str]:
    """Return docs/html-relative output paths and generated HTML contents."""
    outputs: dict[str, str] = {"index.html": render_landing_page(), ".nojekyll": ""}
    for locale in COMMAND_DOC_LOCALES:
        for source_path in sorted((COMMAND_DOCS_ROOT / locale).glob("*.md")):
            output_rel = f"commands/{locale}/{source_path.with_suffix('.html').name}"
            outputs[output_rel] = render_command_page(locale, source_path, output_rel)
    for locale in HANDBOOK_LOCALES:
        for page in build_handbook_pages(locale):
            outputs[page.output_rel] = render_handbook_page(page)
    return outputs


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Generate static HTML docs from docs/user-guide/* and docs/commands/* Markdown source."
    )
    parser.add_argument("--write", action="store_true", help="Write generated HTML to docs/html/.")
    parser.add_argument("--check", action="store_true", help="Fail if checked-in HTML is out of date.")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    outputs = generate_outputs()
    if args.check:
        return check_outputs(
            HTML_ROOT_DIR,
            outputs,
            "HTML docs",
            "python3 scripts/generate_command_html.py --write",
        )
    write_outputs(HTML_ROOT_DIR, outputs)
    print_written_outputs(
        HTML_ROOT_DIR,
        outputs,
        "HTML docs",
        "docs/user-guide/*/*.md and docs/commands/*/*.md",
        "docs/html/**/*.html plus docs/html/.nojekyll",
        "docs/html/index.html",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
