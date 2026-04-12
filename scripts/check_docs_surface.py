#!/usr/bin/env python3
"""Validate docs command examples and local links against the Rust CLI surface."""

from __future__ import annotations

import argparse
import json
import re
import shlex
import sys
import subprocess
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SURFACE_PATH = REPO_ROOT / "scripts" / "contracts" / "command-surface.json"
DEBUG_BINARY = REPO_ROOT / "rust" / "target" / "debug" / "grafana-util"
COMMAND_DOC_ROOT = REPO_ROOT / "docs" / "commands"
DEFAULT_COMMAND_DOC_LOCALES = ("en", "zh-TW")
DOC_ROOTS = (
    REPO_ROOT / "README.md",
    REPO_ROOT / "README.zh-TW.md",
    REPO_ROOT / "docs" / "landing",
    REPO_ROOT / "docs" / "user-guide",
    REPO_ROOT / "docs" / "commands",
)
INTERNAL_DOCS = (
    REPO_ROOT / "docs" / "internal" / "README.md",
    REPO_ROOT / "docs" / "internal" / "maintainer-quickstart.md",
    REPO_ROOT / "docs" / "internal" / "generated-docs-architecture.md",
    REPO_ROOT / "docs" / "internal" / "generated-docs-playbook.md",
    REPO_ROOT / "docs" / "internal" / "docs-architecture-guardrails.md",
    REPO_ROOT / "docs" / "internal" / "ai-workflow-note.md",
)
SKIP_PARTS = {
    "archive",
    "html",
    "man",
}
MARKDOWN_LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
HEADING_COMMAND_RE = re.compile(r"^# `(?P<command>grafana-util(?: [^`]+)?)`$")
HEADING_PLAIN_COMMAND_RE = re.compile(r"^# (?P<command>grafana-util .+)$")
HEADING_BARE_PATH_RE = re.compile(r"^# (?P<command>[a-z0-9][a-z0-9 -]*)$")
HELP_CACHE: dict[tuple[tuple[str, ...], str], bool] = {}


@dataclass(frozen=True)
class Finding:
    path: Path
    line: int
    message: str

    def render(self) -> str:
        return f"{self.path.relative_to(REPO_ROOT)}:{self.line}: {self.message}"


def load_surface() -> dict[str, object]:
    return json.loads(SURFACE_PATH.read_text(encoding="utf-8"))


def command_doc_locales(surface: dict[str, object]) -> tuple[str, ...]:
    locales = surface.get("command_doc_locales", list(DEFAULT_COMMAND_DOC_LOCALES))
    if not isinstance(locales, list) or not all(isinstance(locale, str) for locale in locales):
        raise TypeError("command_doc_locales must be a list of strings")
    return tuple(locales)


def iter_markdown_files() -> list[Path]:
    files: list[Path] = []
    for root in DOC_ROOTS:
        if root.is_file():
            files.append(root)
            continue
        for path in root.rglob("*.md"):
            rel_parts = path.relative_to(REPO_ROOT).parts
            if any(part in SKIP_PARTS for part in rel_parts):
                continue
            files.append(path)
    files.extend(path for path in INTERNAL_DOCS if path.exists())
    return sorted(set(files))


def iter_command_docs_by_locale(locales: tuple[str, ...]) -> dict[str, dict[str, Path]]:
    docs_by_locale: dict[str, dict[str, Path]] = {}
    for locale in locales:
        locale_root = COMMAND_DOC_ROOT / locale
        docs_by_locale[locale] = {
            path.relative_to(locale_root).as_posix(): path for path in sorted(locale_root.rglob("*.md"))
        }
    return docs_by_locale


def is_public_doc(path: Path) -> bool:
    rel = path.relative_to(REPO_ROOT).as_posix()
    return (
        rel.startswith("README")
        or rel.startswith("docs/landing/")
        or rel.startswith("docs/user-guide/")
        or rel.startswith("docs/commands/")
    )


def joined_command_lines(text: str) -> list[tuple[int, str]]:
    results: list[tuple[int, str]] = []
    pending = ""
    pending_line = 0
    inside_fence = False
    fence_language = ""
    for line_number, raw in enumerate(text.splitlines(), start=1):
        stripped = raw.strip()
        if stripped.startswith("```"):
            if inside_fence:
                inside_fence = False
                fence_language = ""
            else:
                inside_fence = True
                fence_language = stripped[3:].strip().lower()
            continue
        if not inside_fence or fence_language not in {"bash", "sh", "zsh", "shell", "console"}:
            continue
        line = raw.strip()
        if line.startswith("#"):
            continue
        if not pending and "grafana-util" not in raw:
            continue
        if not pending:
            pending_line = line_number
        if line.endswith("\\"):
            pending += line[:-1].strip() + " "
            continue
        command = pending + line
        pending = ""
        results.append((pending_line or line_number, command.strip()))
    if pending:
        results.append((pending_line, pending.strip()))
    return results


def tokenize_command(raw: str) -> list[str] | None:
    if "<" in raw or "..." in raw:
        return None
    try:
        tokens = shlex.split(raw, comments=False, posix=True)
    except ValueError:
        return None
    if "grafana-util" not in tokens:
        return None
    start = tokens.index("grafana-util")
    return tokens[start + 1 :]


def command_tokens_before_flags(tokens: list[str]) -> list[str]:
    path_tokens: list[str] = []
    iterator = iter(tokens)
    for token in iterator:
        if token in {"|", "&&", ";"}:
            break
        if token.startswith("-"):
            if token in {"--color"}:
                next(iterator, None)
                continue
            break
        if token.startswith("$") or "=" in token:
            break
        path_tokens.append(token)
    return path_tokens


def first_heading(text: str) -> tuple[int, str] | None:
    for line_number, raw in enumerate(text.splitlines(), start=1):
        stripped = raw.strip()
        if stripped.startswith("# "):
            return line_number, stripped
    return None


def heading_command_tokens(text: str) -> tuple[int, tuple[str, ...]] | None:
    heading = first_heading(text)
    if heading is None:
        return None
    line_number, line = heading
    for pattern, strip_binary in (
        (HEADING_COMMAND_RE, True),
        (HEADING_PLAIN_COMMAND_RE, True),
        (HEADING_BARE_PATH_RE, False),
    ):
        match = pattern.match(line)
        if match is None:
            continue
        command = match.group("command")
        parts = tuple(command.split())
        return line_number, (parts[1:] if strip_binary else parts)
    return None


def executable_command_paths(text: str) -> set[str]:
    paths: set[str] = set()
    for _, raw in joined_command_lines(text):
        tokens = tokenize_command(raw)
        if tokens is None:
            continue
        path_tokens = command_tokens_before_flags(tokens)
        if not path_tokens:
            continue
        paths.add(" ".join(path_tokens))
    return paths


def validate_command_doc_locale_parity(surface: dict[str, object]) -> list[Finding]:
    findings: list[Finding] = []
    locales = command_doc_locales(surface)
    docs_by_locale = iter_command_docs_by_locale(locales)
    file_sets = {locale: set(docs_by_locale[locale]) for locale in locales}
    first_locale = locales[0] if locales else None
    if first_locale is None:
        return findings
    reference_files = file_sets[first_locale]
    for locale in locales[1:]:
        missing = sorted(reference_files - file_sets[locale])
        extra = sorted(file_sets[locale] - reference_files)
        if missing or extra:
            message = []
            if missing:
                message.append(f"missing mirrored docs: {', '.join(missing)}")
            if extra:
                message.append(f"extra docs: {', '.join(extra)}")
            findings.append(
                Finding(
                    COMMAND_DOC_ROOT / locale,
                    1,
                    f"command docs under docs/commands/{locale} are not mirrored with docs/commands/{first_locale}: {'; '.join(message)}",
                )
            )
    shared_files = sorted(set.intersection(*file_sets.values())) if file_sets else []
    for rel_path in shared_files:
        texts = {locale: docs_by_locale[locale][rel_path].read_text(encoding="utf-8") for locale in locales}
        if rel_path != "index.md":
            heading_info = {locale: heading_command_tokens(text) for locale, text in texts.items()}
            heading_paths = {locale: value[1] if value is not None else None for locale, value in heading_info.items()}
            for locale, value in heading_info.items():
                if value is None:
                    findings.append(
                        Finding(
                            docs_by_locale[locale][rel_path],
                            1,
                            "command doc heading must identify a grafana-util command path",
                        )
                    )
            if len({path_tokens for path_tokens in heading_paths.values() if path_tokens is not None}) > 1:
                findings.append(
                    Finding(
                        docs_by_locale[first_locale][rel_path],
                        1,
                        f"command doc heading differs across locales for `{rel_path}`",
                    )
                )
            for locale, value in heading_info.items():
                if value is None:
                    continue
                line_number, path_tokens = value
                resolved = resolve_cli_path(list(path_tokens))
                if resolved is None or resolved != path_tokens:
                    findings.append(
                        Finding(
                            docs_by_locale[locale][rel_path],
                            line_number,
                            f"command doc heading does not resolve to a CLI path: `grafana-util {' '.join(path_tokens)}`",
                        )
                    )
            parsed_paths = [path_tokens for path_tokens in heading_paths.values() if path_tokens is not None]
            if parsed_paths:
                expected_prefix = parsed_paths[0]
                for locale, text in texts.items():
                    example_paths = executable_command_paths(text)
                    for example_path in sorted(example_paths):
                        tokens = tuple(example_path.split())
                        if tokens[: len(expected_prefix)] != expected_prefix:
                            findings.append(
                                Finding(
                                    docs_by_locale[locale][rel_path],
                                    1,
                                    f"example command path escapes page command surface for `{rel_path}`: `grafana-util {example_path}`",
                                )
                            )

    doc_pages = surface.get("doc_pages", {})
    if isinstance(doc_pages, dict):
        for cli_path, filename in sorted(doc_pages.items()):
            if not isinstance(filename, str):
                continue
            for locale in locales:
                doc_path = COMMAND_DOC_ROOT / locale / filename
                if not doc_path.exists():
                    findings.append(
                        Finding(
                            doc_path,
                            1,
                            f"command surface maps `grafana-util {cli_path}` to missing `{locale}` doc `{filename}`",
                        )
                    )
    return findings


def run_help(path_tokens: tuple[str, ...], help_flag: str) -> bool:
    if DEBUG_BINARY.exists():
        command = [str(DEBUG_BINARY), *path_tokens, help_flag]
    else:
        command = [
            "cargo",
            "run",
            "--manifest-path",
            "rust/Cargo.toml",
            "--quiet",
            "--bin",
            "grafana-util",
            "--",
            *path_tokens,
            help_flag,
        ]
    try:
        result = subprocess.run(
            command,
            cwd=REPO_ROOT,
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            text=True,
            timeout=5,
        )
    except subprocess.TimeoutExpired:
        return False
    return result.returncode == 0


def resolve_cli_path(path_tokens: list[str]) -> tuple[str, ...] | None:
    if not path_tokens:
        return ()
    for length in range(len(path_tokens), 0, -1):
        candidate = tuple(path_tokens[:length])
        key = (candidate, "--help")
        if key not in HELP_CACHE:
            HELP_CACHE[key] = run_help(candidate, "--help")
        if HELP_CACHE[key]:
            return candidate
    return None


def validate_commands(path: Path, text: str, surface: dict[str, object]) -> list[Finding]:
    findings: list[Finding] = []
    help_flat_supported = bool(surface.get("help_flat_supported", False))
    help_full_supported = set(surface.get("help_full_supported", []))
    candidates = joined_command_lines(text)
    for line_number, raw in candidates:
        tokens = tokenize_command(raw)
        if tokens is None:
            continue
        if "--help-flat" in tokens:
            if not help_flat_supported:
                findings.append(
                    Finding(
                        path,
                        line_number,
                        "`--help-flat` is not documented as supported in command-surface.json",
                    )
                )
            continue
        path_tokens = command_tokens_before_flags(tokens)
        if not path_tokens:
            continue
        resolved = resolve_cli_path(path_tokens)
        if resolved is None:
            findings.append(
                Finding(path, line_number, f"command path is not accepted by Rust CLI help: `grafana-util {' '.join(path_tokens)}`")
            )
            continue
        if "--help-full" in tokens:
            resolved_key = " ".join(resolved)
            if resolved_key not in help_full_supported:
                findings.append(
                    Finding(
                        path,
                        line_number,
                        f"`--help-full` is not documented as supported for `grafana-util {resolved_key}`",
                    )
                )
    return findings


def validate_links(path: Path, text: str) -> list[Finding]:
    findings: list[Finding] = []
    for line_number, raw in enumerate(text.splitlines(), start=1):
        if is_public_doc(path) and "/Users/" in raw:
            findings.append(Finding(path, line_number, "public docs must not contain local absolute paths"))
        for match in MARKDOWN_LINK_RE.finditer(raw):
            target = match.group(1).split("#", 1)[0]
            if not target or re.match(r"^[a-z]+:", target) or target.startswith("#"):
                continue
            if target.startswith("/"):
                if is_public_doc(path):
                    findings.append(Finding(path, line_number, f"public docs must not link to absolute local path `{target}`"))
                continue
            resolved = (path.parent / target).resolve()
            if (
                not resolved.exists()
                and path.parts[-3:-1] == ("docs", "landing")
                and target.startswith("../man/")
                and target.endswith(".html")
            ):
                generated_manpage = REPO_ROOT / "docs" / "html" / "man" / Path(target).name
                if generated_manpage.exists():
                    continue
            if not resolved.exists():
                findings.append(Finding(path, line_number, f"local Markdown link target does not exist: `{target}`"))
    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    surface = load_surface()
    findings: list[Finding] = validate_command_doc_locale_parity(surface)
    files = iter_markdown_files()
    for index, path in enumerate(files, start=1):
        if index == 1 or index % 25 == 0:
            print(f"[docs-surface] {index}/{len(files)} {path.relative_to(REPO_ROOT)}", file=sys.stderr)
        text = path.read_text(encoding="utf-8")
        findings.extend(validate_commands(path, text, surface))
        findings.extend(validate_links(path, text))
    if findings:
        print("Docs surface check failed:")
        for finding in findings:
            print(f"  {finding.render()}")
        return 1
    print("Docs surface check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
