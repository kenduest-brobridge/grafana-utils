#!/usr/bin/env python3
"""Generate roff manpages from docs/commands/en Markdown source."""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path

from docgen_command_docs import CommandDocPage, get_command_docs_dir, parse_command_page, parse_inline_subcommands
from docgen_common import REPO_ROOT, VERSION, check_outputs, print_written_outputs, write_outputs


MAN_DIR = REPO_ROOT / "docs" / "man"
DATE = "2026-04-03"


@dataclass(frozen=True)
class NamespaceSpec:
    """Describe one namespace-level manpage and where its content comes from."""

    stem: str
    cli_path: str
    title: str
    root_doc: str
    aliases: tuple[str, ...] = ()
    sub_docs: tuple[str, ...] = ()
    related_manpages: tuple[str, ...] = ()
    workflow_notes: tuple[str, ...] = ()


NAMESPACE_SPECS: tuple[NamespaceSpec, ...] = (
    NamespaceSpec(
        stem="grafana-util-dashboard",
        cli_path="grafana-util dashboard",
        title="dashboard browse, export, import, inspection, governance, and screenshot workflows",
        root_doc="dashboard.md",
        aliases=("grafana-util db",),
        sub_docs=(
            "dashboard-browse.md",
            "dashboard-get.md",
            "dashboard-clone-live.md",
            "dashboard-list.md",
            "dashboard-export.md",
            "dashboard-import.md",
            "dashboard-patch-file.md",
            "dashboard-review.md",
            "dashboard-publish.md",
            "dashboard-delete.md",
            "dashboard-diff.md",
            "dashboard-inspect-export.md",
            "dashboard-inspect-live.md",
            "dashboard-inspect-vars.md",
            "dashboard-governance-gate.md",
            "dashboard-topology.md",
            "dashboard-screenshot.md",
        ),
        related_manpages=(
            "grafana-util",
            "grafana-util-datasource",
            "grafana-util-status",
            "grafana-util-overview",
            "grafana-util-snapshot",
        ),
        workflow_notes=(
            "Dashboard export intentionally separates output lanes for different workflows. Treat the raw export tree as the canonical replay or import source unless a command explicitly asks for another lane.",
            "inspect-export and inspect-live are read-only analysis commands, not mutation paths.",
            "browse and screenshot operate against live Grafana state.",
        ),
    ),
    NamespaceSpec(
        stem="grafana-util-alert",
        cli_path="grafana-util alert",
        title="alert export, import, planning, apply, routing, and authoring workflows",
        root_doc="alert.md",
        sub_docs=(
            "alert-export.md",
            "alert-import.md",
            "alert-diff.md",
            "alert-plan.md",
            "alert-apply.md",
            "alert-delete.md",
            "alert-add-rule.md",
            "alert-clone-rule.md",
            "alert-add-contact-point.md",
            "alert-set-route.md",
            "alert-preview-route.md",
            "alert-new-rule.md",
            "alert-new-contact-point.md",
            "alert-new-template.md",
            "alert-list-rules.md",
            "alert-list-contact-points.md",
            "alert-list-mute-timings.md",
            "alert-list-templates.md",
        ),
        related_manpages=(
            "grafana-util",
            "grafana-util-change",
            "grafana-util-status",
            "grafana-util-overview",
        ),
        workflow_notes=(
            "The safest alert workflow is: author or update desired files, inspect the delta with alert plan, then execute only reviewed changes with alert apply.",
        ),
    ),
    NamespaceSpec(
        stem="grafana-util-datasource",
        cli_path="grafana-util datasource",
        title="datasource catalog, export, import, diff, browse, and mutation workflows",
        root_doc="datasource.md",
        aliases=("grafana-util ds",),
        sub_docs=(
            "datasource-types.md",
            "datasource-list.md",
            "datasource-browse.md",
            "datasource-inspect-export.md",
            "datasource-export.md",
            "datasource-import.md",
            "datasource-diff.md",
            "datasource-add.md",
            "datasource-modify.md",
            "datasource-delete.md",
        ),
        related_manpages=(
            "grafana-util",
            "grafana-util-dashboard",
            "grafana-util-status",
            "grafana-util-overview",
            "grafana-util-snapshot",
        ),
        workflow_notes=(
            "Datasource export follows a masked recovery contract. Treat the canonical export JSON as the replay source. Treat provisioning output as a derived projection for Grafana provisioning workflows.",
        ),
    ),
    NamespaceSpec(
        stem="grafana-util-access",
        cli_path="grafana-util access",
        title="access-management workflows for users, teams, orgs, and service accounts",
        root_doc="access.md",
        sub_docs=(
            "access-user.md",
            "access-org.md",
            "access-team.md",
            "access-service-account.md",
            "access-service-account-token.md",
        ),
        related_manpages=(
            "grafana-util",
            "grafana-util-profile",
            "grafana-util-status",
            "grafana-util-overview",
        ),
    ),
    NamespaceSpec(
        stem="grafana-util-profile",
        cli_path="grafana-util profile",
        title="manage repo-local grafana-util profile configuration",
        root_doc="profile.md",
        related_manpages=(
            "grafana-util",
            "grafana-util-status",
            "grafana-util-overview",
            "grafana-util-access",
        ),
    ),
    NamespaceSpec(
        stem="grafana-util-status",
        cli_path="grafana-util status",
        title="render shared staged or live project status",
        root_doc="status.md",
        related_manpages=(
            "grafana-util",
            "grafana-util-overview",
            "grafana-util-change",
            "grafana-util-profile",
            "grafana-util-dashboard",
            "grafana-util-alert",
            "grafana-util-datasource",
            "grafana-util-access",
            "grafana-util-snapshot",
        ),
    ),
    NamespaceSpec(
        stem="grafana-util-overview",
        cli_path="grafana-util overview",
        title="render project-wide staged or live overview summaries",
        root_doc="overview.md",
        related_manpages=(
            "grafana-util",
            "grafana-util-status",
            "grafana-util-change",
            "grafana-util-snapshot",
        ),
    ),
    NamespaceSpec(
        stem="grafana-util-change",
        cli_path="grafana-util change",
        title="review-first sync, preflight, audit, and apply workflows",
        root_doc="change.md",
        related_manpages=(
            "grafana-util",
            "grafana-util-status",
            "grafana-util-overview",
            "grafana-util-alert",
            "grafana-util-snapshot",
        ),
    ),
    NamespaceSpec(
        stem="grafana-util-snapshot",
        cli_path="grafana-util snapshot",
        title="export and review Grafana snapshot inventory bundles",
        root_doc="snapshot.md",
        related_manpages=(
            "grafana-util",
            "grafana-util-status",
            "grafana-util-overview",
            "grafana-util-dashboard",
            "grafana-util-datasource",
            "grafana-util-change",
        ),
    ),
)


def roff_text(text: str) -> str:
    """Escape plain text for simple roff output."""
    escaped = text.replace("\\", r"\\").replace("-", r"\-")
    return re.sub(r"`([^`]+)`", lambda match: rf"\fB{match.group(1)}\fR", escaped)


def roff_name(text: str) -> str:
    return text.replace("\\", r"\\").replace("-", r"\-")


def roff_example_block(examples: tuple[str, ...]) -> list[str]:
    if not examples:
        return []
    lines: list[str] = []
    for example in examples:
        lines.extend([".EX", example, ".EE"])
    return lines


def emit_header(lines: list[str], stem: str, title: str, *, version: str = VERSION) -> None:
    lines.extend(
        [
            '.\\" Generated by scripts/generate_manpages.py from docs/commands/en/.',
            f'.TH {stem.upper()} 1 "{DATE}" "grafana-util {version}" "User Commands"',
            ".SH NAME",
            f"{roff_name(stem)} \\- {roff_text(title)}",
        ]
    )


def emit_see_also(lines: list[str], manpages: tuple[str, ...]) -> None:
    if not manpages:
        return
    lines.append(".SH SEE ALSO")
    lines.append(",\n".join(rf"\fB{stem}\fR(1)" for stem in manpages))


def emit_when(lines: list[str], page: CommandDocPage) -> None:
    bullet_lines = [line[2:] for line in page.when_lines if line.startswith("- ")]
    if bullet_lines and len(bullet_lines) == len(page.when_lines):
        for bullet in bullet_lines:
            lines.extend([".IP \\(bu 2", roff_text(bullet)])
        return
    if page.when:
        lines.extend([".PP", roff_text(page.when)])


def emit_common_options(lines: list[str], key_flags: tuple[str, ...]) -> None:
    if not key_flags:
        return
    lines.append(".SH COMMON OPTIONS")
    for flag in key_flags:
        if ":" in flag:
            name, description = flag.split(":", 1)
            lines.extend([".TP", rf".B {roff_text(name.strip())}", roff_text(description.strip())])
        else:
            lines.extend([".IP \\(bu 2", roff_text(flag)])


def load_subcommands(spec: NamespaceSpec, command_docs_dir: Path) -> list[CommandDocPage]:
    if spec.sub_docs:
        return [parse_command_page(command_docs_dir / filename, spec.cli_path) for filename in spec.sub_docs]
    return parse_inline_subcommands(command_docs_dir / spec.root_doc, spec.cli_path)


def generate_namespace_manpage(
    spec: NamespaceSpec,
    *,
    command_docs_dir: Path,
    version: str = VERSION,
) -> str:
    """Build one namespace-level manpage from the command-doc source pages."""
    root_page = parse_command_page(command_docs_dir / spec.root_doc, spec.cli_path)
    subcommands = load_subcommands(spec, command_docs_dir)

    lines: list[str] = []
    emit_header(lines, spec.stem, spec.title, version=version)
    lines.extend([".SH SYNOPSIS", rf".B {spec.cli_path} [\fISUBCOMMAND\fR] [\fIOPTIONS\fR]"])
    for alias in spec.aliases:
        lines.extend([".PP", rf".B {alias} [\fISUBCOMMAND\fR] [\fIOPTIONS\fR]"])
    lines.extend([".SH DESCRIPTION", roff_text(root_page.purpose)])
    emit_when(lines, root_page)
    lines.append(".SH SUBCOMMANDS")
    for page in subcommands:
        lines.extend([".TP", rf".B {roff_text(page.title)}", roff_text(page.purpose)])
    emit_common_options(lines, root_page.key_flags)
    if spec.workflow_notes:
        lines.append(".SH WORKFLOW NOTES")
        for note in spec.workflow_notes:
            lines.extend([".PP", roff_text(note)])

    examples = list(root_page.examples)
    for page in subcommands:
        for example in page.examples:
            if example not in examples:
                examples.append(example)
            if len(examples) >= 6:
                break
        if len(examples) >= 6:
            break
    if examples:
        lines.append(".SH EXAMPLES")
        lines.extend(roff_example_block(tuple(examples)))
    emit_see_also(lines, spec.related_manpages)
    return "\n".join(lines) + "\n"


def generate_top_level_manpage(*, command_docs_dir: Path, version: str = VERSION) -> str:
    """Build the top-level grafana-util(1) page from namespace metadata."""
    lines: list[str] = []
    emit_header(
        lines,
        "grafana-util",
        "unified CLI for Grafana dashboards, alerts, datasources, access, status, and sync workflows",
        version=version,
    )
    lines.extend(
        [
            ".SH SYNOPSIS",
            r".B grafana-util [\fB\-\-help\fR] [\fB\-\-version\fR]",
            ".PP",
            r".B grafana-util \fICOMMAND\fR [\fISUBCOMMAND\fR] [\fIOPTIONS\fR]",
        ]
    )
    for spec in NAMESPACE_SPECS:
        lines.extend([".PP", rf".B {spec.cli_path} [\fISUBCOMMAND\fR] [\fIOPTIONS\fR]"])
    lines.extend(
        [
            ".SH DESCRIPTION",
            "grafana-util is a unified command-line interface for operating Grafana estates with one executable and one namespaced command shape.",
            ".PP",
            "The checked-in English command reference pages under docs/commands/en/ are the higher-level maintainer source for the generated manpage family under docs/man/.",
            ".SH TOP-LEVEL COMMANDS",
        ]
    )
    for spec in NAMESPACE_SPECS:
        root_page = parse_command_page(command_docs_dir / spec.root_doc, spec.cli_path)
        lines.extend([".TP", rf".B {spec.cli_path.removeprefix('grafana-util ')}", roff_text(root_page.purpose)])
    lines.extend(
        [
            ".TP",
            ".B sync",
            "Declarative sync planning and gated apply workflows. Use command-local --help because this repo does not yet carry a generated sync namespace manpage.",
            ".SH COMMON CONNECTION AND AUTH PATTERN",
            "Many live Grafana commands accept a shared connection pattern. Prefer repo-local profiles for repeatable work, use direct Basic auth for bootstrap or admin-heavy flows, and use direct tokens for scoped automation where the permission envelope is already understood.",
        ]
    )
    for name, description in (
        ("--url", "Grafana base URL."),
        ("--token", "Grafana API token."),
        ("--basic-user", "HTTP basic-auth username."),
        ("--basic-password", "HTTP basic-auth password."),
        ("--prompt-password", "Prompt interactively for the basic-auth password."),
        ("--prompt-token", "Prompt interactively for the API token."),
        ("--profile", "Load defaults from the selected repo-local profile in grafana-util.yaml."),
        ("--timeout", "Override request timeout where supported."),
        ("--verify-ssl", "Enable or disable TLS certificate verification where supported."),
    ):
        lines.extend([".TP", rf".B {roff_text(name)}", roff_text(description)])
    lines.extend(
        [
            ".PP",
            "For environment-backed secrets, the usual pattern is to store them in grafana-util.yaml via password_env or token_env, then run the live command with --profile rather than repeating secrets on every command line.",
            ".PP",
            "Cross-org inventory such as --all-orgs, plus org or user administration, is usually safest with an admin-backed profile or direct Basic auth. Narrow API tokens may only see a subset of orgs or may be rejected entirely for broader administration surfaces.",
            ".SH CONFIGURATION",
            "grafana-util uses repo-local profile configuration by design.",
            ".TP",
            ".I grafana-util.yaml",
            "Primary profile configuration file. By default it is resolved in the current working directory.",
            ".TP",
            ".B GRAFANA_UTIL_CONFIG",
            "Overrides the default config file path.",
            ".TP",
            ".I .grafana-util.secrets.yaml",
            "Optional encrypted secret store used by profile-backed secret resolution.",
            ".TP",
            ".I .grafana-util.secrets.key",
            "Optional local key file used by the encrypted-file secret store mode.",
            ".SH DOCUMENTATION",
            "To render a checked-in manpage from the repo, run man ./docs/man/<name>.1 on BSD or macOS systems, or man -l docs/man/<name>.1 on GNU/Linux systems whose man implementation supports -l.",
            ".PP",
            "This repository currently provides top-level and namespace-level generated manpages. For deeper per-subcommand detail, use the matching Markdown command pages under docs/commands/en/ plus command-local --help.",
            ".SH EXAMPLES",
        ]
    )
    lines.extend(
        roff_example_block(
            (
                "grafana-util --help",
                "grafana-util dashboard --help",
                "grafana-util status live --profile prod --output yaml",
                "grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output yaml",
                "grafana-util overview live --url http://localhost:3000 --token $GRAFANA_API_TOKEN --output json",
                "grafana-util dashboard export --url http://localhost:3000 --export-dir ./dashboards",
                "grafana-util alert plan --desired-dir ./alerts/desired --prune --output json",
                "grafana-util datasource export --url http://localhost:3000 --export-dir ./datasources --overwrite",
            )
        )
    )
    emit_see_also(lines, tuple(spec.stem for spec in NAMESPACE_SPECS) + ("man",))
    return "\n".join(lines) + "\n"


def generate_manpages(*, command_docs_dir: Path | None = None, version: str = VERSION) -> dict[str, str]:
    """Return docs/man-relative output paths and generated roff contents."""
    resolved_command_docs_dir = command_docs_dir or get_command_docs_dir()
    outputs = {"grafana-util.1": generate_top_level_manpage(command_docs_dir=resolved_command_docs_dir, version=version)}
    for spec in NAMESPACE_SPECS:
        outputs[f"{spec.stem}.1"] = generate_namespace_manpage(
            spec,
            command_docs_dir=resolved_command_docs_dir,
            version=version,
        )
    return outputs


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Generate roff manpages from docs/commands/en Markdown source."
    )
    parser.add_argument("--write", action="store_true", help="Write generated manpages into docs/man/.")
    parser.add_argument("--check", action="store_true", help="Fail if checked-in docs/man output is out of date.")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    outputs = generate_manpages()
    if args.check:
        return check_outputs(MAN_DIR, outputs, "manpages", "python3 scripts/generate_manpages.py --write")
    write_outputs(MAN_DIR, outputs)
    print_written_outputs(
        MAN_DIR,
        outputs,
        "manpages",
        "docs/commands/en/*.md",
        "docs/man/*.1",
        "docs/man/grafana-util.1",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
