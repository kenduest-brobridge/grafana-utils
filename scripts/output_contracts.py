from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
REGISTRY_PATH = REPO_ROOT / "scripts" / "contracts" / "output-contracts.json"


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def iter_field_paths(value: Any, path: tuple[str, ...] = ()):
    if isinstance(value, dict):
        for key, item in value.items():
            current = path + (key,)
            yield current
            yield from iter_field_paths(item, current)
        return
    if isinstance(value, list):
        for index, item in enumerate(value):
            yield from iter_field_paths(item, path + (str(index),))


def validate_registry(registry: dict[str, Any]) -> list[str]:
    errors: list[str] = []

    if registry.get("schema") != 1:
        errors.append("registry.schema must be 1")

    contracts = registry.get("contracts")
    if not isinstance(contracts, list) or not contracts:
        errors.append("registry.contracts must be a non-empty list")
        return errors

    seen_names: set[str] = set()
    for index, entry in enumerate(contracts):
        if not isinstance(entry, dict):
            errors.append(f"contracts[{index}] must be an object")
            continue

        name = entry.get("name")
        kind = entry.get("kind")
        fixture = entry.get("fixture")
        required_fields = entry.get("requiredFields")
        schema_version = entry.get("schemaVersion")

        if not name:
            errors.append(f"contracts[{index}].name must be set")
        elif str(name) in seen_names:
            errors.append(f"contracts[{index}].name duplicates {name!r}")
        else:
            seen_names.add(str(name))

        if not kind:
            errors.append(f"contracts[{index}].kind must be set")
        if schema_version != 1:
            errors.append(f"contracts[{index}].schemaVersion must be 1")
        if not fixture:
            errors.append(f"contracts[{index}].fixture must be set")
        elif not isinstance(fixture, str):
            errors.append(f"contracts[{index}].fixture must be a string")
        if not isinstance(required_fields, list) or not required_fields:
            errors.append(f"contracts[{index}].requiredFields must be a non-empty list")
        if "requiredValues" in entry and not isinstance(entry["requiredValues"], dict):
            errors.append(f"contracts[{index}].requiredValues must be an object")
        if "forbiddenFields" in entry and not isinstance(entry["forbiddenFields"], list):
            errors.append(f"contracts[{index}].forbiddenFields must be a list")

    return errors


def validate_contract_fixtures(
    registry: dict[str, Any],
    registry_path: Path = REGISTRY_PATH,
) -> list[str]:
    errors = validate_registry(registry)
    contracts = registry.get("contracts") if isinstance(registry.get("contracts"), list) else []

    for index, entry in enumerate(contracts):
        if not isinstance(entry, dict):
            continue

        name = str(entry.get("name") or f"contract-{index}")
        fixture_name = entry.get("fixture")
        if not isinstance(fixture_name, str):
            continue

        fixture_path = registry_path.parent / fixture_name
        if not fixture_path.is_file():
            errors.append(f"{name}: missing fixture {fixture_name}")
            continue

        try:
            document = load_json(fixture_path)
        except json.JSONDecodeError as exc:
            errors.append(f"{name}: invalid JSON fixture {fixture_name}: {exc}")
            continue

        if document.get("kind") != entry.get("kind"):
            errors.append(
                f"{name}: fixture kind {document.get('kind')!r} does not match registry kind {entry.get('kind')!r}"
            )
        if document.get("schemaVersion") != entry.get("schemaVersion"):
            errors.append(
                f"{name}: fixture schemaVersion {document.get('schemaVersion')!r} does not match registry schemaVersion {entry.get('schemaVersion')!r}"
            )

        for field in entry.get("requiredFields", []):
            if field not in document:
                errors.append(f"{name}: missing required field {field!r}")

        for field, expected in entry.get("requiredValues", {}).items():
            actual = document.get(field)
            if actual != expected:
                errors.append(
                    f"{name}: field {field!r} expected {expected!r} but found {actual!r}"
                )

        forbidden_fields = set(entry.get("forbiddenFields", []))
        if forbidden_fields:
            forbidden_paths = [
                ".".join(path)
                for path in iter_field_paths(document)
                if path and path[-1] in forbidden_fields
            ]
            if forbidden_paths:
                errors.append(
                    f"{name}: forbidden fields present at {', '.join(forbidden_paths)}"
                )

    return errors


def check_output_contracts(registry_path: Path = REGISTRY_PATH) -> list[str]:
    registry = load_json(registry_path)
    return validate_contract_fixtures(registry, registry_path)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate core JSON output contracts against golden fixtures."
    )
    parser.add_argument(
        "--registry",
        default=str(REGISTRY_PATH),
        help="Path to the output contract registry JSON file.",
    )
    args = parser.parse_args(argv)

    registry_path = Path(args.registry)
    errors = check_output_contracts(registry_path)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        print(f"checked registry: {registry_path}", file=sys.stderr)
        return 1

    print(f"check_output_contracts: ok ({registry_path})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
