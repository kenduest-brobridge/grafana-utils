#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any


PLACEHOLDER_RE = re.compile(r"^\$\{?[^}]+\}?$")


def iter_prompt_files(root: Path, *, require_prompt_segment: bool) -> dict[str, Path]:
    files: dict[str, Path] = {}
    for path in sorted(root.rglob("*.json")):
        if path.name in {"index.json", "export-metadata.json"} or path.name.endswith(
            ".summary.json"
        ):
            continue
        parts = list(path.relative_to(root).parts)
        if require_prompt_segment and "prompt" not in parts:
            continue
        relative = (
            [part for part in parts if part != "prompt"]
            if require_prompt_segment
            else parts
        )
        files[str(Path(*relative))] = path
    return files


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def normalize_plugin_name(value: str | None) -> str:
    return (value or "").strip().lower()


def classify_uid(uid: str | None) -> str:
    if not uid:
        return "missing"
    if uid == "-- Mixed --":
        return "mixed"
    if PLACEHOLDER_RE.match(uid):
        return "placeholder"
    return "concrete"


def collect_datasource_refs(node: Any, refs: list[tuple[str, str]]) -> None:
    if isinstance(node, dict):
        datasource = node.get("datasource")
        if isinstance(datasource, dict):
            refs.append(
                (
                    normalize_plugin_name(datasource.get("type")),
                    classify_uid(datasource.get("uid")),
                )
            )
        elif isinstance(datasource, str):
            refs.append(("", classify_uid(datasource)))
        for value in node.values():
            collect_datasource_refs(value, refs)
    elif isinstance(node, list):
        for item in node:
            collect_datasource_refs(item, refs)


def summarize_templating(document: dict[str, Any]) -> Counter[str]:
    counter: Counter[str] = Counter()
    templating = document.get("templating")
    if not isinstance(templating, dict):
        return counter
    variables = templating.get("list")
    if not isinstance(variables, list):
        return counter
    for item in variables:
        if not isinstance(item, dict):
            continue
        counter[f"type:{item.get('type', '')}"] += 1
        datasource = item.get("datasource")
        if isinstance(datasource, dict):
            counter[
                f"datasource:{normalize_plugin_name(datasource.get('type'))}:{classify_uid(datasource.get('uid'))}"
            ] += 1
        elif isinstance(datasource, str):
            counter[f"datasource:string:{classify_uid(datasource)}"] += 1
    return counter


def summarize_document(document: dict[str, Any]) -> dict[str, Any]:
    inputs = document.get("__inputs", [])
    requires = document.get("__requires", [])

    input_plugins = Counter()
    for item in inputs:
        if isinstance(item, dict):
            input_plugins[normalize_plugin_name(item.get("pluginId"))] += 1

    datasource_requires = Counter()
    panel_requires = Counter()
    for item in requires:
        if not isinstance(item, dict):
            continue
        item_type = item.get("type")
        item_id = normalize_plugin_name(item.get("id"))
        if item_type == "datasource":
            datasource_requires[item_id] += 1
        elif item_type == "panel":
            panel_requires[item_id] += 1

    refs: list[tuple[str, str]] = []
    collect_datasource_refs(document, refs)
    ref_counts = Counter(f"{plugin}:{kind}" for plugin, kind in refs)

    return {
        "input_plugins": dict(input_plugins),
        "datasource_requires": dict(datasource_requires),
        "panel_requires": dict(panel_requires),
        "datasource_ref_counts": dict(ref_counts),
        "templating": dict(summarize_templating(document)),
    }


def compare_summary(expected: dict[str, Any], generated: dict[str, Any]) -> list[str]:
    mismatches: list[str] = []
    for key in [
        "input_plugins",
        "datasource_requires",
        "panel_requires",
        "datasource_ref_counts",
        "templating",
    ]:
        if expected[key] != generated[key]:
            mismatches.append(key)
    return mismatches


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--expected-root", required=True)
    parser.add_argument("--generated-root", required=True)
    parser.add_argument("--show-limit", type=int, default=20)
    args = parser.parse_args()

    expected_files = iter_prompt_files(
        Path(args.expected_root), require_prompt_segment=True
    )
    generated_files = iter_prompt_files(
        Path(args.generated_root), require_prompt_segment=False
    )

    expected_only = sorted(set(expected_files) - set(generated_files))
    generated_only = sorted(set(generated_files) - set(expected_files))

    compared = 0
    identical = 0
    mismatched: list[tuple[str, list[str]]] = []

    for relative_path in sorted(set(expected_files) & set(generated_files)):
        compared += 1
        expected_doc = load_json(expected_files[relative_path])
        generated_doc = load_json(generated_files[relative_path])
        mismatches = compare_summary(
            summarize_document(expected_doc), summarize_document(generated_doc)
        )
        if mismatches:
            mismatched.append((relative_path, mismatches))
        else:
            identical += 1

    print("Prompt semantic comparison")
    print(f"  expected files: {len(expected_files)}")
    print(f"  generated files: {len(generated_files)}")
    print(f"  compared: {compared}")
    print(f"  semantic matches: {identical}")
    print(f"  semantic mismatches: {len(mismatched)}")
    print(f"  expected-only files: {len(expected_only)}")
    print(f"  generated-only files: {len(generated_only)}")

    if expected_only:
        print("\nExpected-only files:")
        for item in expected_only[: args.show_limit]:
            print(f"  - {item}")

    if generated_only:
        print("\nGenerated-only files:")
        for item in generated_only[: args.show_limit]:
            print(f"  - {item}")

    if mismatched:
        print("\nSemantic mismatches:")
        for item, fields in mismatched[: args.show_limit]:
            print(f"  - {item}: {', '.join(fields)}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
