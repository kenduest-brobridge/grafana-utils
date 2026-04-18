#!/usr/bin/env python3
"""Validate the repo-owned Rust feature matrix policy."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_MANIFEST = REPO_ROOT / "rust" / "Cargo.toml"
POLICY_DOCS = [
    REPO_ROOT / "docs" / "internal" / "rust-architecture-guardrails.md",
    REPO_ROOT / "docs" / "internal" / "generated-docs-playbook.md",
]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def manifest_default_features(text: str) -> list[str]:
    match = re.search(r"(?m)^default\s*=\s*\[(?P<features>[^\]]*)\]", text)
    if not match:
        return []
    return re.findall(r'"([^"]+)"', match.group("features"))


def validate_policy_text() -> list[str]:
    errors: list[str] = []
    manifest = read_text(RUST_MANIFEST)
    default_features = manifest_default_features(manifest)
    if default_features != ["tui"]:
        errors.append(
            "rust/Cargo.toml default features must stay exactly [\"tui\"] for the supported default artifact"
        )
    if 'browser = ["dep:font8x8", "dep:headless_chrome", "dep:image"]' not in manifest:
        errors.append("rust/Cargo.toml must keep browser as an explicit opt-in feature")

    required_phrases = [
        "Supported Rust Feature Matrix",
        "`--no-default-features` is not a supported release surface",
    ]
    for doc in POLICY_DOCS:
        text = read_text(doc)
        for phrase in required_phrases:
            if phrase not in text:
                errors.append(f"{doc.relative_to(REPO_ROOT)} must state: {phrase}")

    return errors


def run_cargo_check(args: list[str]) -> int:
    command = ["cargo", "check", "--manifest-path", str(RUST_MANIFEST), "--quiet", *args]
    return subprocess.run(command, cwd=REPO_ROOT, check=False).returncode


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Check the maintained Rust feature matrix policy."
    )
    parser.add_argument(
        "--check-cargo",
        action="store_true",
        help="Also compile-check the supported default and browser feature surfaces.",
    )
    parser.add_argument(
        "--probe-no-default-features",
        action="store_true",
        help="Also run the explicit unsupported no-default-features probe.",
    )
    args = parser.parse_args(argv)

    errors = validate_policy_text()

    if args.check_cargo:
        if run_cargo_check([]) != 0:
            errors.append("default Rust feature check failed")
        else:
            print("check_rust_feature_matrix: default Rust feature check: ok")
        if run_cargo_check(["--features", "browser"]) != 0:
            errors.append("browser-enabled Rust feature check failed")
        else:
            print("check_rust_feature_matrix: browser-enabled Rust feature check: ok")

    if args.probe_no_default_features:
        no_default_features_rc = subprocess.run(
            [
                "cargo",
                "check",
                "--manifest-path",
                str(RUST_MANIFEST),
                "--quiet",
                "--no-default-features",
            ],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
        ).returncode
        if no_default_features_rc == 0:
            errors.append("--no-default-features probe unexpectedly succeeded")
        else:
            print(
                "check_rust_feature_matrix: no-default-features probe: expected unsupported"
            )

    if errors:
        for error in errors:
            print(f"check_rust_feature_matrix: {error}", file=sys.stderr)
        return 1

    print("check_rust_feature_matrix: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
