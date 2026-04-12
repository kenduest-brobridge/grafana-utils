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
from docsite_version_portal import render_version_portal
from generate_command_html import HtmlBuildConfig, VersionLink, generate_outputs


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


def build_version_links(version_lanes: list[str], *, include_dev: bool = False) -> tuple[VersionLink, ...]:
    links = [
        VersionLink("Version portal", "index.html"),
        VersionLink("Latest release", "latest/index.html"),
    ]
    if include_dev:
        links.append(VersionLink("Dev preview", "dev/index.html"))
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


def render_redirect_page(target_href: str) -> str:
    escaped_target = html.escape(target_href, quote=True)
    return (
        "<!DOCTYPE html>\n"
        '<html lang="en">\n'
        "<head>\n"
        '  <meta charset="utf-8">\n'
        f'  <meta http-equiv="refresh" content="0; url={escaped_target}">\n'
        f'  <link rel="canonical" href="{escaped_target}">\n'
        "  <title>Redirecting...</title>\n"
        "</head>\n"
        "<body>\n"
        f'  <p>Redirecting to <a href="{escaped_target}">{escaped_target}</a>.</p>\n'
        "</body>\n"
        "</html>\n"
    )


def add_legacy_html_manpage_redirects(outputs: dict[str, str]) -> None:
    """Keep stale release-lane links such as latest/html/man/name.html working."""
    for path in sorted(tuple(outputs)):
        path_obj = Path(path)
        parts = path_obj.parts
        if len(parts) < 3 or parts[-2] != "man" or path_obj.suffix != ".html":
            continue
        legacy_path = Path(*parts[:-2], "html", "man", parts[-1]).as_posix()
        if legacy_path == path or legacy_path in outputs:
            continue
        outputs[legacy_path] = render_redirect_page(f"../../man/{parts[-1]}")


def assemble_site(output_dir: Path, *, include_dev: bool = False) -> None:
    release_tags = select_latest_tags_per_minor(list_release_tags())
    outputs: dict[str, str] = {".nojekyll": ""}
    supported_release_tags: list[SemverTag] = []
    dev_ref = resolve_ref(["origin/dev", "dev"]) if include_dev else None

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

        latest_lane = supported_release_tags[0].minor_label if supported_release_tags else None
        version_lanes = [tag.minor_label for tag in supported_release_tags if tag.minor_label != latest_lane]
        version_links = build_version_links(version_lanes, include_dev=dev_ref is not None)

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
        latest_lane=latest_lane,
        version_lanes=version_lanes,
        has_dev=dev_ref is not None and "dev/index.html" in outputs,
    )
    add_legacy_html_manpage_redirects(outputs)
    if output_dir.exists():
        shutil.rmtree(output_dir)
    write_outputs(output_dir, outputs)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Assemble a multi-version GitHub Pages docs site.")
    parser.add_argument("--output-dir", required=True, help="Directory to write the assembled Pages site into.")
    parser.add_argument(
        "--include-dev",
        action="store_true",
        help="Include the dev preview lane in the generated output. Use this for preview validation, not published release docs.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    assemble_site(Path(args.output_dir).resolve(), include_dev=args.include_dev)
    print(f"Wrote versioned docs site to {args.output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
