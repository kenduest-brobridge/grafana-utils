from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
REGISTRY_PATH = REPO_ROOT / "scripts" / "contracts" / "output-contracts.json"
SUPPORTED_VALUE_TYPES = {
    "object",
    "array",
    "non-empty-array",
    "string",
    "non-empty-string",
    "integer",
    "number",
    "boolean",
    "nullable-number",
}


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


def _parse_path_part(part: str) -> tuple[str, bool]:
    if part.endswith("[*]"):
        return part[:-3], True
    if part.endswith("[]"):
        return part[:-2], True
    return part, False


def _is_scalar_json_value(value: Any) -> bool:
    return value is None or isinstance(value, (str, int, float, bool))


def _collect_path_matches(value: Any, path: str) -> tuple[list[tuple[str, Any]], list[str]]:
    parts = path.split(".") if path else []
    if not parts:
        return [], ["path must not be empty"]

    matches: list[tuple[str, Any]] = [("<root>", value)]
    errors: list[str] = []

    for raw_part in parts:
        key, expects_array = _parse_path_part(raw_part)
        if not key:
            return [], [f"path segment {raw_part!r} must include a field name"]

        next_matches: list[tuple[str, Any]] = []
        for current_path, current_value in matches:
            if not isinstance(current_value, dict):
                errors.append(f"{current_path} must be an object before {raw_part!r}")
                continue

            next_path = f"{current_path}.{key}" if current_path != "<root>" else key
            if key not in current_value:
                errors.append(f"missing required path {next_path!r}")
                continue

            next_value = current_value[key]
            if not expects_array:
                next_matches.append((next_path, next_value))
                continue

            if not isinstance(next_value, list):
                errors.append(f"required path {next_path!r} must be an array")
                continue
            if not next_value:
                errors.append(f"required path {next_path!r} must be a non-empty array")
                continue

            for item_index, item in enumerate(next_value):
                next_matches.append((f"{next_path}[{item_index}]", item))

        matches = next_matches

    return matches, errors


def validate_required_path(value: Any, path: str) -> list[str]:
    _, errors = _collect_path_matches(value, path)
    return errors


def values_for_path(value: Any, path: str) -> tuple[list[Any], list[str]]:
    matches, errors = _collect_path_matches(value, path)
    if errors:
        return [], errors
    return [matched_value for _, matched_value in matches], []


def _collect_path_values(value: Any, path: str) -> tuple[list[tuple[str, Any]], list[str]]:
    return _collect_path_matches(value, path)


def validate_value_type(value: Any, expected_type: str) -> bool:
    if expected_type == "object":
        return isinstance(value, dict)
    if expected_type == "array":
        return isinstance(value, list)
    if expected_type == "non-empty-array":
        return isinstance(value, list) and bool(value)
    if expected_type == "string":
        return isinstance(value, str)
    if expected_type == "non-empty-string":
        return isinstance(value, str) and bool(value)
    if expected_type == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected_type == "number":
        return (isinstance(value, int) or isinstance(value, float)) and not isinstance(value, bool)
    if expected_type == "boolean":
        return isinstance(value, bool)
    if expected_type == "nullable-number":
        return value is None or validate_value_type(value, "number")
    return False


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
        if "requiredPaths" in entry:
            required_paths = entry["requiredPaths"]
            if not isinstance(required_paths, list):
                errors.append(f"contracts[{index}].requiredPaths must be a list")
            else:
                for path_index, path in enumerate(required_paths):
                    if not isinstance(path, str) or not path:
                        errors.append(
                            f"contracts[{index}].requiredPaths[{path_index}] must be a non-empty string"
                        )
        if "pathTypes" in entry:
            path_types = entry["pathTypes"]
            if not isinstance(path_types, dict) or not path_types:
                errors.append(f"contracts[{index}].pathTypes must be a non-empty object")
            else:
                for path, expected_type in path_types.items():
                    if not isinstance(path, str) or not path:
                        errors.append(
                            f"contracts[{index}].pathTypes keys must be non-empty strings"
                        )
                    if not isinstance(expected_type, str) or expected_type not in SUPPORTED_VALUE_TYPES:
                        errors.append(
                            f"contracts[{index}].pathTypes[{path!r}] has unsupported type {expected_type!r}"
                        )
        if "arrayItemTypes" in entry:
            array_item_types = entry["arrayItemTypes"]
            if not isinstance(array_item_types, dict) or not array_item_types:
                errors.append(f"contracts[{index}].arrayItemTypes must be a non-empty object")
            else:
                for path, expected_type in array_item_types.items():
                    if not isinstance(path, str) or not path:
                        errors.append(
                            f"contracts[{index}].arrayItemTypes keys must be non-empty strings"
                        )
                    if not isinstance(expected_type, str) or expected_type not in SUPPORTED_VALUE_TYPES:
                        errors.append(
                            f"contracts[{index}].arrayItemTypes[{path!r}] has unsupported type {expected_type!r}"
                        )
        if "minimumItems" in entry:
            minimum_items = entry["minimumItems"]
            if not isinstance(minimum_items, dict) or not minimum_items:
                errors.append(f"contracts[{index}].minimumItems must be a non-empty object")
            else:
                for path, minimum in minimum_items.items():
                    if not isinstance(path, str) or not path:
                        errors.append(
                            f"contracts[{index}].minimumItems keys must be non-empty strings"
                        )
                    if not isinstance(minimum, int) or isinstance(minimum, bool) or minimum < 0:
                        errors.append(
                            f"contracts[{index}].minimumItems[{path!r}] must be a non-negative integer"
                        )
        if "enumValues" in entry:
            enum_values = entry["enumValues"]
            if not isinstance(enum_values, dict) or not enum_values:
                errors.append(f"contracts[{index}].enumValues must be a non-empty object")
            else:
                for path, allowed_values in enum_values.items():
                    if not isinstance(path, str) or not path:
                        errors.append(
                            f"contracts[{index}].enumValues keys must be non-empty strings"
                        )
                    if not isinstance(allowed_values, list) or not allowed_values:
                        errors.append(
                            f"contracts[{index}].enumValues[{path!r}] must be a non-empty list"
                        )
                        continue
                    if any(not _is_scalar_json_value(value) for value in allowed_values):
                        errors.append(
                            f"contracts[{index}].enumValues[{path!r}] must contain only scalar JSON values"
                        )
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

        for path in entry.get("requiredPaths", []):
            for error in validate_required_path(document, path):
                errors.append(f"{name}: {error}")

        for path, expected_type in entry.get("pathTypes", {}).items():
            matches, path_errors = _collect_path_values(document, path)
            for error in path_errors:
                errors.append(f"{name}: {error}")
            for _, value in matches:
                if not validate_value_type(value, expected_type):
                    errors.append(
                        f"{name}: path {path!r} expected {expected_type} but found {type(value).__name__}"
                    )

        for path, expected_type in entry.get("arrayItemTypes", {}).items():
            matches, path_errors = _collect_path_values(document, path)
            for error in path_errors:
                errors.append(f"{name}: {error}")
            for matched_path, value in matches:
                if not isinstance(value, list):
                    errors.append(
                        f"{name}: path {matched_path!r} expected an array for arrayItemTypes but found {type(value).__name__}"
                    )
                    continue
                for item_index, item in enumerate(value):
                    if not validate_value_type(item, expected_type):
                        errors.append(
                            f"{name}: path {matched_path}[{item_index}] expected {expected_type} but found {type(item).__name__}"
                        )

        for path, minimum_items in entry.get("minimumItems", {}).items():
            matches, path_errors = _collect_path_values(document, path)
            for error in path_errors:
                errors.append(f"{name}: {error}")
            for matched_path, value in matches:
                if not isinstance(value, list):
                    errors.append(
                        f"{name}: path {matched_path!r} expected an array for minimumItems but found {type(value).__name__}"
                    )
                    continue
                if len(value) < minimum_items:
                    errors.append(
                        f"{name}: path {matched_path!r} expected at least {minimum_items} items but found {len(value)}"
                    )

        for path, allowed_values in entry.get("enumValues", {}).items():
            matches, path_errors = _collect_path_values(document, path)
            for error in path_errors:
                errors.append(f"{name}: {error}")
            allowed_set = set(allowed_values)
            for matched_path, value in matches:
                if not _is_scalar_json_value(value) or value not in allowed_set:
                    errors.append(
                        f"{name}: path {matched_path!r} expected one of {allowed_values!r} but found {value!r}"
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
