#!/usr/bin/env python3
"""Generate the static HTML docs site from handbook and command Markdown."""

from __future__ import annotations

import argparse
import html
from dataclasses import dataclass, replace
from pathlib import Path

from docgen_command_docs import RenderedHeading, render_markdown_document
from docgen_common import REPO_ROOT, VERSION, check_outputs, print_written_outputs, relative_href, write_outputs
from docgen_handbook import HANDBOOK_LOCALES, LOCALE_LABELS, build_handbook_pages, handbook_language_href
from generate_manpages import NAMESPACE_SPECS, generate_manpages


HTML_ROOT_DIR = REPO_ROOT / "docs" / "html"
COMMAND_DOCS_ROOT = REPO_ROOT / "docs" / "commands"
HANDBOOK_ROOT = REPO_ROOT / "docs" / "user-guide"
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
.article pre.manpage {
  white-space: pre-wrap;
  word-break: break-word;
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


def prefixed_output_rel(config: HtmlBuildConfig, relative_path: str) -> str:
    if not config.output_prefix:
        return relative_path
    return f"{config.output_prefix.strip('/')}/{relative_path}"


def render_version_links(output_rel: str, config: HtmlBuildConfig) -> str:
    if config.version_label is None and not config.version_links:
        return "<p>Current checkout</p>"
    items = []
    if config.version_label is not None:
        items.append((f"Current: {config.version_label}", "#"))
    for link in config.version_links:
        items.append((link.label, relative_href(output_rel, link.target_rel)))
    return html_list(items)


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
    version_html: str,
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
          <h2>Version</h2>
          {version_html}
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


def command_reference_root_for_stem(stem: str, config: HtmlBuildConfig) -> str | None:
    if stem == "grafana-util":
        return prefixed_output_rel(config, "commands/en/index.html")
    for spec in NAMESPACE_SPECS:
        if spec.stem == stem:
            return prefixed_output_rel(config, f"commands/en/{spec.root_doc[:-3]}.html")
    return None


def render_manpage_index_page(output_rel: str, manpage_names: list[str], config: HtmlBuildConfig) -> str:
    body_html = (
        "<p>This lane mirrors the checked-in generated manpages as browser-readable HTML for GitHub Pages and local HTML browsing.</p>"
        "<ul>"
        + "".join(
            f'<li><a href="{html.escape(relative_href(output_rel, prefixed_output_rel(config, f"man/{Path(name).stem}.html")))}">{html.escape(name)}</a></li>'
            for name in manpage_names
        )
        + "</ul>"
    )
    related_links = [
        ("English command reference", relative_href(output_rel, prefixed_output_rel(config, "commands/en/index.html"))),
        ("English handbook", relative_href(output_rel, prefixed_output_rel(config, "handbook/en/index.html"))),
    ]
    if config.raw_manpage_target_rel:
        related_links.append(("Top-level roff manpage", relative_href(output_rel, config.raw_manpage_target_rel)))
    return page_shell(
        page_title="Manpages",
        html_lang="en",
        home_href=relative_href(output_rel, prefixed_output_rel(config, "index.html")),
        hero_title="Generated Manpages",
        hero_summary="Browser-readable HTML mirrors of the checked-in generated manpages.",
        eyebrow=f"Manpages · grafana-util {html.escape(config.version)}",
        breadcrumbs=[
            ("Home", relative_href(output_rel, prefixed_output_rel(config, "index.html"))),
            ("Manpages", None),
        ],
        body_html=body_html,
        toc_html="<p>Open a generated manpage mirror or jump back to the command reference.</p>",
        related_html=html_list(related_links),
        version_html=render_version_links(output_rel, config),
        locale_html="<p>Manpages are generated from the English command-reference source.</p>",
        footer_nav_html="",
        footer_html='Generated from <code>docs/man/*.1</code> via <code>scripts/generate_manpages.py</code>.',
    )


def render_manpage_page(output_rel: str, name: str, roff_text_body: str, config: HtmlBuildConfig) -> str:
    stem = Path(name).stem
    command_root = command_reference_root_for_stem(stem, config)
    related_links = [
        ("Manpage index", relative_href(output_rel, prefixed_output_rel(config, "man/index.html"))),
    ]
    if config.raw_manpage_target_rel:
        related_links.append(("Raw roff source", relative_href(output_rel, prefixed_output_rel(config, f"man/{name}"))))
    if command_root:
        related_links.append(("Matching command reference", relative_href(output_rel, command_root)))
    body_html = (
        "<p>This page mirrors the generated roff manpage in HTML so it can be read from GitHub Pages. "
        "For deeper per-subcommand detail, prefer the command-reference pages.</p>"
        f'<pre class="manpage">{html.escape(roff_text_body)}</pre>'
    )
    return page_shell(
        page_title=name,
        html_lang="en",
        home_href=relative_href(output_rel, prefixed_output_rel(config, "index.html")),
        hero_title=name,
        hero_summary="HTML mirror of a generated roff manpage.",
        eyebrow=f"Manpage Mirror · {html.escape(config.version)}",
        breadcrumbs=[
            ("Home", relative_href(output_rel, prefixed_output_rel(config, "index.html"))),
            ("Manpages", relative_href(output_rel, prefixed_output_rel(config, "man/index.html"))),
            (name, None),
        ],
        body_html=body_html,
        toc_html="<p>This mirror preserves the generated manpage content as preformatted text.</p>",
        related_html=html_list(related_links),
        version_html=render_version_links(output_rel, config),
        locale_html="<p>English only. The manpage lane currently generates from English command docs.</p>",
        footer_nav_html="",
        footer_html=f'Source: <code>docs/man/{html.escape(name)}</code><br>Generated by <code>scripts/generate_command_html.py</code>.',
    )


def render_landing_page(config: HtmlBuildConfig) -> str:
    manpage_link = (
        f'<li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "man/index.html")))}">Manpages (HTML)</a></li>'
    )
    secondary_card_html = """
  <section class="landing-card">
    <div class="eyebrow">Maintainer</div>
    <h2>Repository and design docs</h2>
    <p>Maintainer-facing documentation stays separate from the public handbook and command-reference lanes.</p>
    <ul>
      <li><a href="../DEVELOPER.md">Maintainer entrypoint</a></li>
      <li><a href="../internal/maintainer-quickstart.md">Maintainer quickstart</a></li>
      <li><a href="../internal/README.md">Internal docs index</a></li>
    </ul>
  </section>
""".strip()
    if config.output_prefix:
        secondary_card_html = f"""
  <section class="landing-card">
    <div class="eyebrow">Version</div>
    <h2>{html.escape(config.version_label or config.version)}</h2>
    <p>This lane is a versioned snapshot for GitHub Pages. Use the version switcher to move between release and preview docs.</p>
    <ul>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "commands/en/index.html")))}">English command reference</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "commands/zh-TW/index.html")))}">繁體中文逐指令說明</a></li>
      {manpage_link}
    </ul>
  </section>
""".strip()
    body_html = f"""
<div class="landing-grid">
  <section class="landing-card">
    <div class="eyebrow">Language</div>
    <h2>Start by language</h2>
    <p>Choose the language lane first, then continue into the handbook, command reference, or role-specific entry pages.</p>
    <ul>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/en/index.html")))}">English handbook</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "commands/en/index.html")))}">English command reference</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/zh-TW/index.html")))}">繁體中文手冊</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "commands/zh-TW/index.html")))}">繁體中文逐指令說明</a></li>
    </ul>
  </section>
  {secondary_card_html}
</div>
<div class="landing-grid">
  <section class="landing-card">
    <div class="eyebrow">English</div>
    <h2>English handbook and reference</h2>
    <p>Use the English lane if you want the handbook, role guides, and one-page-per-command reference in English.</p>
    <ul>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/en/index.html")))}">Handbook index</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/en/role-new-user.html")))}">New user path</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/en/role-sre-ops.html")))}">SRE / operator path</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/en/role-automation-ci.html")))}">Automation / CI path</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "commands/en/index.html")))}">Command reference</a></li>
      {manpage_link}
    </ul>
  </section>
  <section class="landing-card">
    <div class="eyebrow">繁體中文</div>
    <h2>繁體中文手冊與逐指令說明</h2>
    <p>Use the Traditional Chinese lane if you want handbook chapters, role guides, and command pages without mixing locales in the same entry flow.</p>
    <ul>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/zh-TW/index.html")))}">手冊目錄</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/zh-TW/role-new-user.html")))}">新使用者路徑</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/zh-TW/role-sre-ops.html")))}">SRE / 維運路徑</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/zh-TW/role-automation-ci.html")))}">自動化 / CI 路徑</a></li>
      <li><a href="{html.escape(relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "commands/zh-TW/index.html")))}">逐指令說明</a></li>
      {manpage_link.replace("Manpages (HTML)", "Manpage HTML")}
    </ul>
  </section>
</div>
""".strip()
    footer_html = (
        "Source roots: <code>docs/user-guide/*</code> and <code>docs/commands/*</code>. "
        "Generated by <code>scripts/generate_command_html.py</code>."
    )
    related_links = [
        ("English handbook", relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/en/index.html"))),
        ("繁體中文手冊", relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "handbook/zh-TW/index.html"))),
        ("English command reference", relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "commands/en/index.html"))),
        ("繁體中文逐指令說明", relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "commands/zh-TW/index.html"))),
    ]
    if config.version_links:
        related_links.insert(0, ("Version portal", "#"))
    if config.include_raw_manpages or config.raw_manpage_target_rel:
        related_links.append(("Manpages (HTML)", relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "man/index.html"))))
    if config.raw_manpage_target_rel:
        related_links.append(("Top-level manpage source", relative_href(prefixed_output_rel(config, "index.html"), config.raw_manpage_target_rel)))
    if not config.output_prefix:
        related_links.append(("Maintainer entrypoint", "../DEVELOPER.md"))
    return page_shell(
        page_title="grafana-util HTML docs",
        html_lang="en",
        home_href=relative_href(prefixed_output_rel(config, "index.html"), prefixed_output_rel(config, "index.html")),
        hero_title="grafana-util HTML Docs",
        hero_summary="Generated manual-style HTML with separate handbook and command-reference entrypoints.",
        eyebrow=f"Generated HTML · grafana-util {html.escape(config.version)}",
        breadcrumbs=[("Home", None)],
        body_html=body_html,
        toc_html="<p>Start from one of the two main entrypoints.</p>",
        related_html=html_list(related_links),
        version_html=render_version_links(prefixed_output_rel(config, "index.html"), config),
        locale_html="<p>Choose a language first, then stay within that handbook or command-reference lane.</p>",
        footer_nav_html="",
        footer_html=footer_html,
    )


def command_language_switch_href(
    output_rel: str,
    locale: str,
    source_name: str,
    config: HtmlBuildConfig,
) -> tuple[str | None, str | None]:
    other_locale = next((candidate for candidate in COMMAND_DOC_LOCALES if candidate != locale), None)
    if other_locale is None:
        return None, None
    target_source = config.command_docs_root / other_locale / source_name
    if not target_source.exists():
        return None, None
    target_rel = prefixed_output_rel(config, f"commands/{other_locale}/{Path(source_name).with_suffix('.html').as_posix()}")
    return LOCALE_LABELS[other_locale], relative_href(output_rel, target_rel)


def command_handbook_context(
    locale: str,
    output_rel: str,
    source_name: str,
    config: HtmlBuildConfig,
) -> tuple[str, str] | None:
    stem = Path(source_name).stem
    root = stem.split("-", 1)[0]
    handbook_stem = HANDBOOK_CONTEXT_BY_COMMAND.get(stem) or HANDBOOK_CONTEXT_BY_COMMAND.get(root)
    if not handbook_stem:
        return None
    target_rel = prefixed_output_rel(config, f"handbook/{locale}/{handbook_stem}.html")
    return ("Matching handbook chapter", relative_href(output_rel, target_rel))


def rewrite_markdown_link(source_path: Path, output_rel: str, target: str, config: HtmlBuildConfig) -> str:
    """Rewrite source-relative Markdown links so they work from docs/html."""
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return target
    bare_target, fragment = (target.split("#", 1) + [""])[:2]
    resolved = (source_path.parent / bare_target).resolve()
    docs_root = config.source_root / "docs"
    try:
        docs_rel = resolved.relative_to(docs_root).as_posix()
    except ValueError:
        return target
    if docs_rel.startswith("commands/") and docs_rel.endswith(".md"):
        rewritten = relative_href(output_rel, prefixed_output_rel(config, f"{docs_rel[:-3]}.html"))
        return f"{rewritten}#{fragment}" if fragment else rewritten
    if docs_rel.startswith("user-guide/") and docs_rel.endswith(".md"):
        docs_rel = docs_rel.replace("user-guide/", "handbook/", 1)
        rewritten = relative_href(output_rel, prefixed_output_rel(config, f"{docs_rel[:-3]}.html"))
        return f"{rewritten}#{fragment}" if fragment else rewritten
    rewritten = relative_href(output_rel, docs_rel)
    return f"{rewritten}#{fragment}" if fragment else rewritten


def render_handbook_page(page, config: HtmlBuildConfig) -> str:
    document = render_markdown_document(
        page.source_path.read_text(encoding="utf-8"),
        link_transform=lambda target: rewrite_markdown_link(page.source_path, page.output_rel, target, config),
    )
    page_title = title_only(document.title or page.title)
    breadcrumbs = [
        ("Home", relative_href(page.output_rel, "index.html")),
        ("Handbook", relative_href(page.output_rel, prefixed_output_rel(config, f"handbook/{page.locale}/index.html"))),
        (LOCALE_LABELS[page.locale], None),
        (page_title, None),
    ]
    related_links = [
        ("Handbook home", relative_href(page.output_rel, prefixed_output_rel(config, f"handbook/{page.locale}/index.html"))),
        ("Command reference index", relative_href(page.output_rel, prefixed_output_rel(config, f"commands/{page.locale}/index.html"))),
        ("Manpages (HTML)", relative_href(page.output_rel, prefixed_output_rel(config, "man/index.html"))),
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
        f'Source: <code>{html.escape(page.source_path.relative_to(config.source_root).as_posix())}</code><br>'
        'Generated by <code>scripts/generate_command_html.py</code>.'
    )
    return page_shell(
        page_title=page_title,
        html_lang=page.locale,
        home_href=relative_href(page.output_rel, prefixed_output_rel(config, "index.html")),
        hero_title=page_title,
        hero_summary=handbook_intro_text(page.locale),
        eyebrow=f"Handbook · {LOCALE_LABELS[page.locale]}",
        breadcrumbs=breadcrumbs,
        body_html=document.body_html,
        toc_html=render_toc(document.headings),
        related_html=html_list(related_links),
        version_html=render_version_links(page.output_rel, config),
        locale_html=render_language_links(LOCALE_LABELS[page.locale], locale_label, locale_href),
        footer_nav_html=render_footer_nav(previous_link, next_link),
        footer_html=footer_html,
    )


def render_command_page(locale: str, source_path: Path, output_rel: str, config: HtmlBuildConfig) -> str:
    document = render_markdown_document(
        source_path.read_text(encoding="utf-8"),
        link_transform=lambda target: rewrite_markdown_link(source_path, output_rel, target, config),
    )
    page_title = title_only(document.title or source_path.stem)
    breadcrumbs = [
        ("Home", relative_href(output_rel, prefixed_output_rel(config, "index.html"))),
        ("Command Reference", relative_href(output_rel, prefixed_output_rel(config, f"commands/{locale}/index.html"))),
        (LOCALE_LABELS[locale], None),
        (page_title, None),
    ]
    related_links = [
        ("Command reference home", relative_href(output_rel, prefixed_output_rel(config, f"commands/{locale}/index.html"))),
    ]
    handbook_link = command_handbook_context(locale, output_rel, source_path.name, config)
    if handbook_link:
        related_links.append(handbook_link)
    if locale == "en":
        related_links.append(("Manpages (HTML)", relative_href(output_rel, prefixed_output_rel(config, "man/index.html"))))
        if config.raw_manpage_target_rel:
            related_links.append(("Top-level manpage", relative_href(output_rel, config.raw_manpage_target_rel)))
    switch_label, switch_href = command_language_switch_href(output_rel, locale, source_path.name, config)
    footer_html = (
        f'Source: <code>{html.escape(source_path.relative_to(config.source_root).as_posix())}</code><br>'
        'Generated by <code>scripts/generate_command_html.py</code>.'
    )
    return page_shell(
        page_title=page_title,
        html_lang=locale,
        home_href=relative_href(output_rel, prefixed_output_rel(config, "index.html")),
        hero_title=page_title,
        hero_summary=command_intro_text(locale),
        eyebrow=f"Command Reference · {LOCALE_LABELS[locale]}",
        breadcrumbs=breadcrumbs,
        body_html=document.body_html,
        toc_html=render_toc(document.headings),
        related_html=html_list(related_links),
        version_html=render_version_links(output_rel, config),
        locale_html=render_language_links(LOCALE_LABELS[locale], switch_label, switch_href),
        footer_nav_html="",
        footer_html=footer_html,
    )


def generate_outputs(config: HtmlBuildConfig = HtmlBuildConfig()) -> dict[str, str]:
    """Return docs/html-relative output paths and generated HTML contents."""
    lane_index_rel = prefixed_output_rel(config, "index.html")
    outputs: dict[str, str] = {lane_index_rel: render_landing_page(config), ".nojekyll": ""}
    manpage_outputs = generate_manpages(
        command_docs_dir=config.command_docs_root / "en",
        version=config.version,
    )
    man_index_rel = prefixed_output_rel(config, "man/index.html")
    outputs[man_index_rel] = render_manpage_index_page(man_index_rel, sorted(manpage_outputs), config)
    for name, roff_text_body in sorted(manpage_outputs.items()):
        html_rel = prefixed_output_rel(config, f"man/{Path(name).stem}.html")
        outputs[html_rel] = render_manpage_page(html_rel, name, roff_text_body, config)
        if config.include_raw_manpages:
            outputs[prefixed_output_rel(config, f"man/{name}")] = roff_text_body
    for locale in COMMAND_DOC_LOCALES:
        for source_path in sorted((config.command_docs_root / locale).glob("*.md")):
            output_rel = prefixed_output_rel(config, f"commands/{locale}/{source_path.with_suffix('.html').name}")
            outputs[output_rel] = render_command_page(locale, source_path, output_rel, config)
    for locale in HANDBOOK_LOCALES:
        for page in build_handbook_pages(locale, handbook_root=config.handbook_root):
            page = replace(
                page,
                output_rel=prefixed_output_rel(config, page.output_rel),
                previous_output_rel=prefixed_output_rel(config, page.previous_output_rel) if page.previous_output_rel else None,
                next_output_rel=prefixed_output_rel(config, page.next_output_rel) if page.next_output_rel else None,
                language_switch_rel=prefixed_output_rel(config, page.language_switch_rel) if page.language_switch_rel else None,
            )
            outputs[page.output_rel] = render_handbook_page(page, config)
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
