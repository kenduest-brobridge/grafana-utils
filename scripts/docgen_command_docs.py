"""Shared parsing helpers for generated docs Markdown source.

The manpage and HTML generators intentionally support only a small Markdown
subset. Keeping that parsing logic here makes the top-level generators easier
for maintainers to follow and safer to extend.
"""

from __future__ import annotations

import html
import re
from dataclasses import dataclass
from pathlib import Path

from docgen_common import REPO_ROOT


def get_command_docs_dir(repo_root: Path = REPO_ROOT) -> Path:
    return repo_root / "docs" / "commands" / "en"


COMMAND_DOCS_DIR = get_command_docs_dir()
ORDERED_LIST_ITEM_RE = re.compile(r"^\d+\.\s+(.*)$")
SECTION_ALIASES = {
    "Purpose": "Purpose",
    "目的": "Purpose",
    "用途": "Purpose",
    "When to use": "When to use",
    "使用時機": "When to use",
    "何時使用": "When to use",
    "Key flags": "Key flags",
    "主要旗標": "Key flags",
    "重點旗標": "Key flags",
    "Examples": "Examples",
    "範例": "Examples",
    "Related commands": "Related commands",
    "相關命令": "Related commands",
    "相關指令": "Related commands",
    "Auth notes": "Auth notes",
    "驗證說明": "Auth notes",
    "說明": "Description",
    "Description": "Description",
    "Workflow notes": "Workflow notes",
    "Command groups": "Command groups",
    "指令分類": "Command groups",
    "Workflow lanes": "Workflow lanes",
    "這一頁對應的工作流": "Workflow lanes",
    "Choose this page when": "Choose this page when",
    "從這裡開始": "Choose this page when",
    "Before / After": "Before / After",
    "採用前後對照": "Before / After",
    "What success looks like": "What success looks like",
    "成功判準": "What success looks like",
    "Failure checks": "Failure checks",
    "失敗時先檢查": "Failure checks",
    "Datasource resolution": "Datasource resolution",
    "Placeholder model": "Placeholder model",
    "Root": "Root",
}

LABEL_ALIASES = {
    "Purpose": "Purpose",
    "目的": "Purpose",
    "用途": "Purpose",
    "When to use": "When to use",
    "使用時機": "When to use",
    "何時使用": "When to use",
    "Key flags": "Key flags",
    "主要旗標": "Key flags",
    "重點旗標": "Key flags",
    "Examples": "Examples",
    "範例": "Examples",
    "Related commands": "Related commands",
    "相關命令": "Related commands",
    "相關指令": "Related commands",
    "Auth notes": "Auth notes",
    "驗證說明": "Auth notes",
    "Description": "Description",
    "說明": "Description",
    "Command groups": "Command groups",
    "指令分類": "Command groups",
    "Workflow lanes": "Workflow lanes",
    "這一頁對應的工作流": "Workflow lanes",
    "Choose this page when": "Choose this page when",
    "從這裡開始": "Choose this page when",
    "Before / After": "Before / After",
    "採用前後對照": "Before / After",
    "What success looks like": "What success looks like",
    "成功判準": "What success looks like",
    "Failure checks": "Failure checks",
    "失敗時先檢查": "Failure checks",
}

COMMAND_SUBSECTION_LABELS = frozenset(
    {
        "Purpose",
        "When to use",
        "Key flags",
        "Examples",
        "Related commands",
        "Auth notes",
        "Description",
    }
)

NON_COMMAND_SECTION_TITLES = frozenset(
    {
        "Before / After",
        "What success looks like",
        "Failure checks",
        "採用前後對照",
        "成功判準",
        "失敗時先檢查",
    }
)


@dataclass(frozen=True)
class CommandDocPage:
    title: str
    purpose: str
    when: str
    when_lines: tuple[str, ...]
    command_group_lines: tuple[str, ...]
    workflow_lines: tuple[str, ...]
    choose_lines: tuple[str, ...]
    before_after_lines: tuple[str, ...]
    key_flags: tuple[str, ...]
    examples: tuple[str, ...]
    success_lines: tuple[str, ...]
    failure_lines: tuple[str, ...]


@dataclass(frozen=True)
class RenderedHeading:
    level: int
    text: str
    anchor: str


@dataclass(frozen=True)
class RenderedMarkdownDocument:
    title: str
    body_html: str
    headings: tuple[RenderedHeading, ...]


def clean_markdown(text: str) -> str:
    """Remove the few inline Markdown constructs used in command docs."""
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    text = re.sub(r"`([^`]+)`", r"\1", text)
    text = re.sub(r"\*\*([^*]+)\*\*", r"\1", text)
    text = re.sub(r"\*([^*]+)\*", r"\1", text)
    return " ".join(text.strip().split())


def canonical_command_section(label: str) -> str:
    cleaned = clean_markdown(label)
    return SECTION_ALIASES.get(cleaned, cleaned)


def split_markdown_sections(text: str) -> tuple[str, dict[str, list[str]]]:
    """Split a command-doc page by second-level headings."""
    lines = text.splitlines()
    title = ""
    sections: dict[str, list[str]] = {}
    current: str | None = None
    inside_fence = False
    for line in lines:
        if line.strip().startswith("```"):
            inside_fence = not inside_fence
            if current is not None:
                sections[current].append(line)
            continue
        if inside_fence:
            if current is not None:
                sections[current].append(line)
            continue
        if line.startswith("# "):
            title = clean_markdown(line[2:].strip())
            continue
        if line.startswith("## "):
            current = canonical_command_section(line[3:].strip())
            sections[current] = []
            continue
        if current is not None:
            sections[current].append(line)
    return title, sections


def clean_command_name(title: str, cli_path: str) -> str:
    """Convert a page title like 'grafana-util alert plan' to 'plan'."""
    cleaned = clean_markdown(title)
    prefix = cli_path.removeprefix("grafana-util ").strip()
    if cleaned.startswith("grafana-util "):
        cleaned = cleaned.removeprefix("grafana-util ").strip()
    if cleaned == prefix:
        return prefix.split()[-1]
    if cleaned.startswith(prefix + " "):
        return cleaned[len(prefix) + 1 :]
    return cleaned


def extract_paragraph(lines: list[str]) -> str:
    current: list[str] = []
    for raw in lines:
        stripped = raw.strip()
        if not stripped:
            if current:
                break
            continue
        if stripped.startswith("```"):
            break
        current.append(clean_markdown(stripped))
    return " ".join(current)


def extract_text_lines(lines: list[str]) -> list[str]:
    results: list[str] = []
    for raw in lines:
        stripped = raw.strip()
        if not stripped or stripped.startswith("```"):
            continue
        if stripped.startswith("- "):
            results.append("- " + clean_markdown(stripped[2:]))
        else:
            results.append(clean_markdown(stripped))
    return results


def extract_bullets(lines: list[str]) -> list[str]:
    return [clean_markdown(raw.strip()[2:]) for raw in lines if raw.strip().startswith("- ")]


def extract_codeblock(lines: list[str]) -> list[str]:
    inside = False
    code: list[str] = []
    for raw in lines:
        stripped = raw.strip()
        if stripped.startswith("```"):
            if inside:
                break
            inside = True
            continue
        if inside:
            code.append(raw.rstrip())
    cleaned = [line for line in code if line.strip()]
    if cleaned and cleaned[0].lstrip().startswith("# "):
        cleaned = cleaned[1:]
    return cleaned


def parse_labeled_section(lines: list[str]) -> dict[str, list[str]]:
    labels: dict[str, list[str]] = {}
    current: str | None = None
    inside_fence = False
    for raw in lines:
        stripped = raw.strip()
        if stripped.startswith("```"):
            inside_fence = not inside_fence
            continue
        if inside_fence:
            if current is not None:
                labels[current].append(raw)
            continue
        match = re.match(r"^([^:：]{1,80})[:：]\s*(.*)$", raw)
        if match:
            current = LABEL_ALIASES.get(clean_markdown(match.group(1).strip()), clean_markdown(match.group(1).strip()))
            labels[current] = [match.group(2)] if match.group(2) else []
            continue
        if current is not None:
            labels[current].append(raw)
    return labels


def parse_command_page(path: Path, cli_path: str) -> CommandDocPage:
    """Parse one Markdown command-doc page into the fields used by generators."""
    title, sections = split_markdown_sections(path.read_text(encoding="utf-8"))
    if "Purpose" in sections:
        purpose_source = sections
    elif "Root" in sections:
        purpose_source = parse_labeled_section(sections["Root"])
    else:
        raise ValueError(f"Unsupported command doc format: {path}")
    return CommandDocPage(
        title=clean_command_name(title, cli_path),
        purpose=extract_paragraph(purpose_source.get("Purpose", [])),
        when=extract_paragraph(purpose_source.get("When to use", [])),
        when_lines=tuple(extract_text_lines(purpose_source.get("When to use", []))),
        command_group_lines=tuple(extract_text_lines(sections.get("Command groups", []))),
        workflow_lines=tuple(extract_text_lines(sections.get("Workflow lanes", []))),
        choose_lines=tuple(extract_text_lines(sections.get("Choose this page when", []))),
        before_after_lines=tuple(extract_text_lines(sections.get("Before / After", []))),
        key_flags=tuple(extract_bullets(purpose_source.get("Key flags", []))),
        examples=tuple(extract_codeblock(purpose_source.get("Examples", []))),
        success_lines=tuple(extract_text_lines(sections.get("What success looks like", []))),
        failure_lines=tuple(extract_text_lines(sections.get("Failure checks", []))),
    )


def parse_inline_subcommands(path: Path, cli_path: str) -> list[CommandDocPage]:
    """Parse pages like profile.md where subcommands live in one Markdown file."""
    _, sections = split_markdown_sections(path.read_text(encoding="utf-8"))
    parsed: list[CommandDocPage] = []
    for heading, body in sections.items():
        if heading == "Root":
            continue
        if clean_markdown(heading) in NON_COMMAND_SECTION_TITLES:
            continue
        labels = parse_labeled_section(body)
        if not COMMAND_SUBSECTION_LABELS.intersection(labels):
            continue
        parsed.append(
            CommandDocPage(
                title=clean_markdown(heading).strip('"'),
                purpose=extract_paragraph(labels.get("Purpose", [])),
                when=extract_paragraph(labels.get("When to use", [])),
                when_lines=tuple(extract_text_lines(labels.get("When to use", []))),
                command_group_lines=tuple(extract_text_lines(labels.get("Command groups", []))),
                workflow_lines=tuple(extract_text_lines(labels.get("Workflow lanes", []))),
                choose_lines=tuple(extract_text_lines(labels.get("Choose this page when", []))),
                before_after_lines=tuple(extract_text_lines(labels.get("Before / After", []))),
                key_flags=tuple(extract_bullets(labels.get("Key flags", []))),
                examples=tuple(extract_codeblock(labels.get("Examples", []))),
                success_lines=tuple(extract_text_lines(labels.get("What success looks like", []))),
                failure_lines=tuple(extract_text_lines(labels.get("Failure checks", []))),
            )
        )
    return parsed


def inline_html(text: str, link_transform=None) -> str:
    """Render the tiny inline Markdown subset used by the generated docs."""
    escaped = html.escape(text)
    escaped = re.sub(
        r"`([^`]+)`",
        lambda match: f"<code>{match.group(1)}</code>",
        escaped,
    )

    def replace_link(match: re.Match[str]) -> str:
        label = match.group(1)
        target = match.group(2)
        if link_transform is not None:
            target = link_transform(target)
        elif target.endswith(".md"):
            target = target[:-3] + ".html"
        return f'<a href="{html.escape(target)}">{label}</a>'

    escaped = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", replace_link, escaped)
    escaped = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", escaped)
    escaped = re.sub(r"\*([^*]+)\*", r"<em>\1</em>", escaped)
    return escaped


def slugify_heading(text: str) -> str:
    slug = re.sub(r"[^\w\s-]", "", text, flags=re.UNICODE).strip().lower()
    slug = re.sub(r"[-\s]+", "-", slug)
    return slug or "section"


def split_table_row(row: str) -> list[str]:
    return [cell.strip() for cell in row.strip().strip("|").split("|")]


def is_table_separator(row: str) -> bool:
    cells = split_table_row(row)
    return bool(cells) and all(re.fullmatch(r":?-{3,}:?", cell) for cell in cells)


def render_table(table_lines: list[str], link_transform=None) -> str:
    rows = [split_table_row(line) for line in table_lines]
    if len(rows) >= 2 and is_table_separator(table_lines[1]):
        header = rows[0]
        body = rows[2:]
    else:
        header = rows[0]
        body = rows[1:]
    head_html = "".join(f"<th>{inline_html(cell, link_transform=link_transform)}</th>" for cell in header)
    body_html = "".join(
        "<tr>"
        + "".join(f"<td>{inline_html(cell, link_transform=link_transform)}</td>" for cell in row)
        + "</tr>"
        for row in body
    )
    if body_html:
        body_html = f"<tbody>{body_html}</tbody>"
    return f"<table><thead><tr>{head_html}</tr></thead>{body_html}</table>"


def render_markdown_document(md_text: str, link_transform=None) -> RenderedMarkdownDocument:
    """Render the Markdown subset used by handbook and command docs."""
    lines = md_text.splitlines()
    blocks: list[str] = []
    paragraph: list[str] = []
    list_items: list[str] = []
    list_tag: str | None = None
    code_lines: list[str] = []
    table_lines: list[str] = []
    headings: list[RenderedHeading] = []
    in_code = False
    code_lang = ""
    title = ""

    def flush_paragraph() -> None:
        nonlocal paragraph
        if paragraph:
            joined = " ".join(part.strip() for part in paragraph)
            blocks.append(f"<p>{inline_html(joined, link_transform=link_transform)}</p>")
            paragraph = []

    def flush_list() -> None:
        nonlocal list_items, list_tag
        if list_items:
            items = "".join(f"<li>{inline_html(item, link_transform=link_transform)}</li>" for item in list_items)
            blocks.append(f"<{list_tag}>{items}</{list_tag}>")
            list_items = []
            list_tag = None

    def flush_code() -> None:
        nonlocal code_lines
        joined = chr(10).join(code_lines)
        blocks.append(f"<pre><code>{html.escape(joined)}</code></pre>")
        code_lines = []

    def flush_table() -> None:
        nonlocal table_lines
        if table_lines:
            blocks.append(render_table(table_lines, link_transform=link_transform))
            table_lines = []

    for raw in lines:
        stripped = raw.strip()
        if stripped.startswith("```"):
            flush_paragraph()
            flush_list()
            flush_table()
            if in_code:
                flush_code()
                in_code = False
                code_lang = ""
            else:
                in_code = True
                code_lang = stripped[3:].strip().lower()
            continue
        if in_code:
            code_lines.append(raw.rstrip())
            continue
        if not stripped:
            flush_paragraph()
            flush_list()
            flush_table()
            continue
        if stripped == "---":
            flush_paragraph()
            flush_list()
            flush_table()
            continue
        if stripped.startswith("# "):
            flush_paragraph()
            flush_list()
            flush_table()
            title = stripped[2:].strip()
            anchor = slugify_heading(title)
            headings.append(RenderedHeading(level=1, text=title, anchor=anchor))
            blocks.append(f'<h1 id="{anchor}">{inline_html(title, link_transform=link_transform)}</h1>')
            continue
        if stripped.startswith("## "):
            flush_paragraph()
            flush_list()
            flush_table()
            heading_text = stripped[3:].strip()
            anchor = slugify_heading(heading_text)
            headings.append(RenderedHeading(level=2, text=heading_text, anchor=anchor))
            blocks.append(f'<h2 id="{anchor}">{inline_html(heading_text, link_transform=link_transform)}</h2>')
            continue
        if stripped.startswith("### "):
            flush_paragraph()
            flush_list()
            flush_table()
            heading_text = stripped[4:].strip()
            anchor = slugify_heading(heading_text)
            headings.append(RenderedHeading(level=3, text=heading_text, anchor=anchor))
            blocks.append(f'<h3 id="{anchor}">{inline_html(heading_text, link_transform=link_transform)}</h3>')
            continue
        ordered_match = ORDERED_LIST_ITEM_RE.match(stripped)
        if stripped.startswith(("- ", "* ")) or ordered_match:
            flush_paragraph()
            flush_table()
            next_tag = "ol" if ordered_match else "ul"
            if list_tag and list_tag != next_tag:
                flush_list()
            list_tag = next_tag
            list_items.append(ordered_match.group(1).strip() if ordered_match else stripped[2:].strip())
            continue
        if stripped.startswith("|") and stripped.endswith("|"):
            flush_paragraph()
            flush_list()
            table_lines.append(stripped)
            continue
        flush_table()
        paragraph.append(stripped)

    flush_paragraph()
    flush_list()
    flush_table()
    if in_code:
        flush_code()
    return RenderedMarkdownDocument(
        title=clean_markdown(title) if title else "",
        body_html="\n".join(blocks),
        headings=tuple(headings),
    )


def render_markdown_subset(md_text: str, link_transform=None) -> str:
    """Backward-compatible wrapper for callers that only need body HTML."""
    return render_markdown_document(md_text, link_transform=link_transform).body_html
