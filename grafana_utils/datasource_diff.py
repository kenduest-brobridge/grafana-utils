#!/usr/bin/env python3
"""Unwired datasource diff helpers for future CLI integration."""

import json
from pathlib import Path
from typing import Any, Optional

from .clients.dashboard_client import GrafanaClient
from .datasource_contract import normalize_datasource_record
from .datasource_contract import validate_datasource_contract_record
from .dashboard_cli import GrafanaError, build_datasource_inventory_record


DATASOURCE_EXPORT_FILENAME = "datasources.json"
EXPORT_METADATA_FILENAME = "export-metadata.json"
ROOT_INDEX_KIND = "grafana-utils-datasource-export-index"
TOOL_SCHEMA_VERSION = 1

COMPARE_FIELDS = (
    "uid",
    "name",
    "type",
    "access",
    "url",
    "isDefault",
    "org",
    "orgId",
)


def load_json_document(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc))
    except ValueError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc))


def load_datasource_diff_bundle(import_dir: Path) -> dict[str, Any]:
    if not import_dir.exists():
        raise GrafanaError("Diff directory does not exist: %s" % import_dir)
    if not import_dir.is_dir():
        raise GrafanaError("Diff path is not a directory: %s" % import_dir)

    metadata_path = import_dir / EXPORT_METADATA_FILENAME
    datasources_path = import_dir / DATASOURCE_EXPORT_FILENAME
    index_path = import_dir / "index.json"
    if not metadata_path.is_file():
        raise GrafanaError("Datasource diff metadata is missing: %s" % metadata_path)
    if not datasources_path.is_file():
        raise GrafanaError("Datasource diff file is missing: %s" % datasources_path)
    if not index_path.is_file():
        raise GrafanaError("Datasource diff index is missing: %s" % index_path)

    metadata = load_json_document(metadata_path)
    if not isinstance(metadata, dict):
        raise GrafanaError(
            "Datasource diff metadata must be a JSON object: %s" % metadata_path
        )
    if metadata.get("kind") != ROOT_INDEX_KIND:
        raise GrafanaError(
            "Unexpected datasource export manifest kind in %s: %r"
            % (metadata_path, metadata.get("kind"))
        )
    if metadata.get("schemaVersion") != TOOL_SCHEMA_VERSION:
        raise GrafanaError(
            "Unsupported datasource export schemaVersion %r in %s. Expected %s."
            % (metadata.get("schemaVersion"), metadata_path, TOOL_SCHEMA_VERSION)
        )
    if metadata.get("resource") != "datasource":
        raise GrafanaError(
            "Datasource diff metadata in %s does not describe datasource inventory."
            % metadata_path
        )

    raw_records = load_json_document(datasources_path)
    if not isinstance(raw_records, list):
        raise GrafanaError(
            "Datasource diff file must contain a JSON array: %s" % datasources_path
        )
    records = []
    for item in raw_records:
        if not isinstance(item, dict):
            raise GrafanaError(
                "Datasource diff entry must be a JSON object: %s" % datasources_path
            )
        try:
            validate_datasource_contract_record(
                item,
                "Datasource diff entry in %s" % datasources_path,
            )
        except ValueError as exc:
            raise GrafanaError(str(exc))
        records.append(normalize_datasource_record(item))

    index_document = load_json_document(index_path)
    if not isinstance(index_document, dict):
        raise GrafanaError(
            "Datasource diff index must be a JSON object: %s" % index_path
        )

    return {
        "metadata": metadata,
        "records": records,
        "index": index_document,
        "datasources_path": datasources_path,
    }


def build_live_datasource_diff_records(
    client: GrafanaClient,
) -> list[dict[str, str]]:
    org = client.fetch_current_org()
    return [
        normalize_datasource_record(build_datasource_inventory_record(item, org))
        for item in client.list_datasources()
    ]


def resolve_datasource_identity(record: dict[str, str]) -> str:
    uid = str(record.get("uid") or "").strip()
    if uid:
        return uid
    name = str(record.get("name") or "").strip()
    if name:
        return name
    return "<unnamed-datasource>"


def _index_records(
    records: list[dict[str, str]],
) -> dict[str, dict[str, list[tuple[int, dict[str, str]]]]]:
    by_uid = {}
    by_name = {}
    for index, record in enumerate(records):
        uid = str(record.get("uid") or "")
        name = str(record.get("name") or "")
        if uid:
            by_uid.setdefault(uid, []).append((index, record))
        if name:
            by_name.setdefault(name, []).append((index, record))
    return {"by_uid": by_uid, "by_name": by_name}


def _resolve_live_match(
    local_record: dict[str, str],
    live_index: dict[str, dict[str, list[tuple[int, dict[str, str]]]]],
) -> dict[str, Any]:
    uid = str(local_record.get("uid") or "")
    name = str(local_record.get("name") or "")
    if uid:
        uid_matches = live_index["by_uid"].get(uid) or []
        if len(uid_matches) > 1:
            return {"status": "ambiguous-live-uid"}
        if len(uid_matches) == 1:
            index, live_record = uid_matches[0]
            return {
                "status": "matched",
                "matchKey": "uid",
                "index": index,
                "record": live_record,
            }
    if name:
        name_matches = live_index["by_name"].get(name) or []
        if len(name_matches) > 1:
            return {"status": "ambiguous-live-name"}
        if len(name_matches) == 1:
            index, live_record = name_matches[0]
            return {
                "status": "matched",
                "matchKey": "name",
                "index": index,
                "record": live_record,
            }
    return {"status": "missing-live"}


def _resolve_compare_fields(
    local_record: Optional[dict[str, str]],
    match_key: Optional[str],
) -> tuple[str, ...]:
    fields = list(COMPARE_FIELDS)
    if (
        match_key == "name"
        and local_record is not None
        and not str(local_record.get("uid") or "").strip()
        and "uid" in fields
    ):
        fields.remove("uid")
    return tuple(fields)


def build_datasource_diff_item(
    local_record: Optional[dict[str, str]],
    live_record: Optional[dict[str, str]],
    status: str,
    match_key: Optional[str],
) -> dict[str, Any]:
    changed_fields = []
    local_values = {}
    live_values = {}
    if local_record is not None and live_record is not None:
        for field in _resolve_compare_fields(local_record, match_key):
            local_value = str(local_record.get(field) or "")
            live_value = str(live_record.get(field) or "")
            if local_value != live_value:
                changed_fields.append(field)
                local_values[field] = local_value
                live_values[field] = live_value

    identity_record = local_record if local_record is not None else live_record or {}
    return {
        "identity": resolve_datasource_identity(identity_record),
        "status": status,
        "matchKey": match_key or "",
        "changedFields": changed_fields,
        "local": local_record,
        "live": live_record,
        "localValues": local_values,
        "liveValues": live_values,
    }


def compare_datasource_inventory(
    bundle_records: list[dict[str, str]],
    live_records: list[dict[str, str]],
) -> dict[str, Any]:
    normalized_bundle = [normalize_datasource_record(item) for item in bundle_records]
    normalized_live = [normalize_datasource_record(item) for item in live_records]
    live_index = _index_records(normalized_live)
    matched_live_indexes: set[int] = set()
    items = []

    for local_record in normalized_bundle:
        resolution = _resolve_live_match(local_record, live_index)
        resolution_status = resolution["status"]
        if resolution_status == "matched":
            live_index_value = resolution["index"]
            matched_live_indexes.add(live_index_value)
            live_record = resolution["record"]
            item = build_datasource_diff_item(
                local_record,
                live_record,
                "different",
                resolution.get("matchKey"),
            )
            if not item["changedFields"]:
                item["status"] = "match"
            items.append(item)
            continue
        items.append(
            build_datasource_diff_item(local_record, None, resolution_status, None)
        )

    for live_index_value, live_record in enumerate(normalized_live):
        if live_index_value in matched_live_indexes:
            continue
        items.append(build_datasource_diff_item(None, live_record, "extra-live", None))

    summary = {
        "bundleCount": len(normalized_bundle),
        "liveCount": len(normalized_live),
        "matchCount": len([item for item in items if item["status"] == "match"]),
        "differentCount": len(
            [item for item in items if item["status"] == "different"]
        ),
        "missingLiveCount": len(
            [item for item in items if item["status"] == "missing-live"]
        ),
        "extraLiveCount": len(
            [item for item in items if item["status"] == "extra-live"]
        ),
        "ambiguousCount": len(
            [
                item
                for item in items
                if item["status"] in ("ambiguous-live-uid", "ambiguous-live-name")
            ]
        ),
        "diffCount": len([item for item in items if item["status"] != "match"]),
    }

    return {
        "summary": summary,
        "items": items,
    }


def compare_datasource_bundle_to_live(
    bundle: dict[str, Any],
    live_records: list[dict[str, str]],
) -> dict[str, Any]:
    return compare_datasource_inventory(bundle.get("records") or [], live_records)
