#!/usr/bin/env python3
"""Grafana datasource list/export utility."""

import argparse
import csv
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional
from urllib import parse

from .clients.dashboard_client import GrafanaClient
from .dashboard_cli import (
    DEFAULT_TIMEOUT,
    DEFAULT_URL,
    GrafanaError,
    HelpFullAction,
    add_common_cli_args,
    build_client as build_dashboard_client,
    build_data_source_record,
    build_datasource_inventory_record,
    render_data_source_table,
    resolve_auth,
    write_json_document,
)


DEFAULT_EXPORT_DIR = "datasources"
DATASOURCE_EXPORT_FILENAME = "datasources.json"
EXPORT_METADATA_FILENAME = "export-metadata.json"
ROOT_INDEX_KIND = "grafana-utils-datasource-export-index"
TOOL_SCHEMA_VERSION = 1

HELP_FULL_EXAMPLES = (
    "Extended Examples:\n\n"
    "  Export datasource inventory for the current org:\n"
    "    grafana-utils datasource export --url http://localhost:3000 "
    "--basic-user admin --basic-password admin --export-dir ./datasources --overwrite\n\n"
    "  Dry-run datasource import for the current org:\n"
    "    grafana-utils datasource import --url http://localhost:3000 "
    "--token \"$GRAFANA_API_TOKEN\" --import-dir ./datasources --dry-run --table\n\n"
    "  List datasource inventory as JSON for scripting:\n"
    "    grafana-utils datasource list --url http://localhost:3000 "
    "--token \"$GRAFANA_API_TOKEN\" --json"
)


def add_list_cli_args(parser: argparse.ArgumentParser) -> None:
    output_group = parser.add_mutually_exclusive_group()
    output_group.add_argument(
        "--table",
        action="store_true",
        help="Render datasource summaries as a table.",
    )
    output_group.add_argument(
        "--csv",
        action="store_true",
        help="Render datasource summaries as CSV.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render datasource summaries as JSON.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="Do not print table headers when rendering the default table output.",
    )


def add_export_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--export-dir",
        default=DEFAULT_EXPORT_DIR,
        help=(
            "Directory to write exported datasource inventory into. Export writes "
            "datasources.json plus index/manifest files at that root."
        ),
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Replace existing export files in the target directory instead of failing.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview the datasource export files that would be written without changing disk.",
    )


def add_import_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--import-dir",
        required=True,
        help=(
            "Import datasource inventory from this directory. Point this to the "
            "datasource export root that contains datasources.json and export-metadata.json."
        ),
    )
    parser.add_argument(
        "--org-id",
        default=None,
        help=(
            "Import datasources into this explicit Grafana organization ID instead "
            "of the current org context. Requires Basic auth."
        ),
    )
    parser.add_argument(
        "--require-matching-export-org",
        action="store_true",
        help=(
            "Require the datasource export's recorded orgId to match the target "
            "Grafana org before dry-run or live import."
        ),
    )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="Update an existing destination datasource when the imported datasource already exists.",
    )
    parser.add_argument(
        "--update-existing-only",
        action="store_true",
        help="Only update existing destination datasources. Missing datasources are skipped instead of created.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview what datasource import would do without changing Grafana.",
    )
    parser.add_argument(
        "--table",
        action="store_true",
        help="For --dry-run only, render a compact table instead of per-datasource log lines.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="For --dry-run only, render one JSON document with mode, actions, and summary counts.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="For --dry-run --table only, omit the table header row.",
    )
    parser.add_argument(
        "--progress",
        action="store_true",
        help="Show concise per-datasource import progress in <current>/<total> form while processing records.",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show detailed per-datasource import output. Overrides --progress output.",
    )


def build_parser(prog: Optional[str] = None) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=prog or "grafana-utils datasource",
        description="List, export, or import Grafana datasource inventory.",
        epilog=(
            "Examples:\n\n"
            "  grafana-utils datasource list --url http://localhost:3000 --json\n"
            "  grafana-utils datasource export --url http://localhost:3000 "
            "--export-dir ./datasources --overwrite\n"
            "  grafana-utils datasource import --url http://localhost:3000 "
            "--import-dir ./datasources --dry-run --table"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="command")
    subparsers.required = True

    list_parser = subparsers.add_parser(
        "list",
        help="List live Grafana datasource inventory.",
    )
    add_common_cli_args(list_parser)
    add_list_cli_args(list_parser)
    list_parser.set_defaults(_help_full_examples=HELP_FULL_EXAMPLES)
    list_parser.add_argument(
        "--help-full",
        nargs=0,
        action=HelpFullAction,
        help="Show normal help plus extended datasource examples.",
    )

    export_parser = subparsers.add_parser(
        "export",
        help="Export live Grafana datasource inventory as normalized JSON files.",
    )
    add_common_cli_args(export_parser)
    add_export_cli_args(export_parser)
    export_parser.set_defaults(_help_full_examples=HELP_FULL_EXAMPLES)
    export_parser.add_argument(
        "--help-full",
        nargs=0,
        action=HelpFullAction,
        help="Show normal help plus extended datasource examples.",
    )

    import_parser = subparsers.add_parser(
        "import",
        help="Import datasource inventory JSON through the Grafana API.",
    )
    add_common_cli_args(import_parser)
    add_import_cli_args(import_parser)
    import_parser.set_defaults(_help_full_examples=HELP_FULL_EXAMPLES)
    import_parser.add_argument(
        "--help-full",
        nargs=0,
        action=HelpFullAction,
        help="Show normal help plus extended datasource examples.",
    )

    return parser


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    return build_parser().parse_args(argv)


def build_client(args: argparse.Namespace) -> GrafanaClient:
    """Build the datasource API client from parsed CLI arguments."""
    return build_dashboard_client(args)


def build_export_index(
    datasource_records: List[Dict[str, str]],
    datasources_file: str,
) -> Dict[str, Any]:
    return {
        "kind": ROOT_INDEX_KIND,
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "datasourcesFile": datasources_file,
        "count": len(datasource_records),
        "items": [
            {
                "uid": record.get("uid") or "",
                "name": record.get("name") or "",
                "type": record.get("type") or "",
                "org": record.get("org") or "",
                "orgId": record.get("orgId") or "",
            }
            for record in datasource_records
        ],
    }


def build_export_metadata(
    datasource_count: int,
    datasources_file: str,
) -> Dict[str, Any]:
    return {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "kind": ROOT_INDEX_KIND,
        "variant": "root",
        "resource": "datasource",
        "datasourceCount": datasource_count,
        "datasourcesFile": datasources_file,
        "indexFile": "index.json",
        "format": "grafana-datasource-inventory-v1",
    }


def build_export_records(
    client: GrafanaClient,
) -> List[Dict[str, str]]:
    org = client.fetch_current_org()
    return [
        build_datasource_inventory_record(item, org)
        for item in client.list_datasources()
    ]


def fetch_datasource_by_uid_if_exists(
    client: GrafanaClient,
    uid: str,
) -> Optional[Dict[str, Any]]:
    if not uid:
        return None
    try:
        data = client.request_json(
            "/api/datasources/uid/%s" % parse.quote(uid, safe="")
        )
    except Exception as exc:
        if isinstance(exc, exporter_api_error_type()):
            if exc.status_code == 404:
                return None
        raise
    if not isinstance(data, dict):
        raise GrafanaError("Unexpected datasource payload for UID %s." % uid)
    return data


def exporter_api_error_type():
    from .dashboards.common import GrafanaApiError

    return GrafanaApiError


def normalize_datasource_record(record: Dict[str, Any]) -> Dict[str, str]:
    return {
        "uid": str(record.get("uid") or ""),
        "name": str(record.get("name") or ""),
        "type": str(record.get("type") or ""),
        "access": str(record.get("access") or ""),
        "url": str(record.get("url") or ""),
        "isDefault": str(record.get("isDefault") or "false"),
        "org": str(record.get("org") or ""),
        "orgId": str(record.get("orgId") or ""),
    }


def load_json_document(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc))
    except ValueError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc))


def load_import_bundle(import_dir: Path) -> Dict[str, Any]:
    if not import_dir.exists():
        raise GrafanaError("Import directory does not exist: %s" % import_dir)
    if not import_dir.is_dir():
        raise GrafanaError("Import path is not a directory: %s" % import_dir)
    metadata_path = import_dir / EXPORT_METADATA_FILENAME
    datasources_path = import_dir / DATASOURCE_EXPORT_FILENAME
    index_path = import_dir / "index.json"
    if not metadata_path.is_file():
        raise GrafanaError("Datasource import metadata is missing: %s" % metadata_path)
    if not datasources_path.is_file():
        raise GrafanaError("Datasource import file is missing: %s" % datasources_path)
    if not index_path.is_file():
        raise GrafanaError("Datasource import index is missing: %s" % index_path)
    metadata = load_json_document(metadata_path)
    if not isinstance(metadata, dict):
        raise GrafanaError(
            "Datasource import metadata must be a JSON object: %s" % metadata_path
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
            "Datasource import metadata in %s does not describe datasource inventory."
            % metadata_path
        )
    raw_records = load_json_document(datasources_path)
    if not isinstance(raw_records, list):
        raise GrafanaError(
            "Datasource import file must contain a JSON array: %s" % datasources_path
        )
    records = []
    for item in raw_records:
        if not isinstance(item, dict):
            raise GrafanaError(
                "Datasource import entry must be a JSON object: %s" % datasources_path
            )
        records.append(normalize_datasource_record(item))
    index_document = load_json_document(index_path)
    if not isinstance(index_document, dict):
        raise GrafanaError(
            "Datasource import index must be a JSON object: %s" % index_path
        )
    return {
        "metadata": metadata,
        "records": records,
        "index": index_document,
        "datasources_path": datasources_path,
    }


def resolve_export_org_id(bundle: Dict[str, Any]) -> Optional[str]:
    org_ids = set()
    index_document = bundle.get("index")
    if isinstance(index_document, dict):
        for item in index_document.get("items") or []:
            if isinstance(item, dict):
                org_id = str(item.get("orgId") or "").strip()
                if org_id:
                    org_ids.add(org_id)
    for record in bundle.get("records") or []:
        org_id = str(record.get("orgId") or "").strip()
        if org_id:
            org_ids.add(org_id)
    if not org_ids:
        return None
    if len(org_ids) > 1:
        raise GrafanaError(
            "Datasource export metadata spans multiple orgIds (%s). Remove "
            "--require-matching-export-org or point --import-dir at one org-specific export."
            % ", ".join(sorted(org_ids))
        )
    return list(org_ids)[0]


def build_effective_import_client(
    args: argparse.Namespace,
    client: GrafanaClient,
) -> GrafanaClient:
    org_id = getattr(args, "org_id", None)
    auth_header = client.headers.get("Authorization", "")
    if org_id and not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Datasource org switching requires Basic auth. Use --basic-user and --basic-password."
        )
    if org_id:
        return client.with_org_id(str(org_id))
    return client


def validate_export_org_match(
    args: argparse.Namespace,
    client: GrafanaClient,
    bundle: Dict[str, Any],
) -> str:
    target_org = client.fetch_current_org()
    target_org_id = str(target_org.get("id") or "").strip()
    if not target_org_id:
        raise GrafanaError("Grafana did not return a usable target org id.")
    if not bool(getattr(args, "require_matching_export_org", False)):
        return target_org_id
    source_org_id = resolve_export_org_id(bundle)
    if not source_org_id:
        raise GrafanaError(
            "Could not determine one source export orgId while "
            "--require-matching-export-org is active."
        )
    if source_org_id != target_org_id:
        raise GrafanaError(
            "Raw export orgId %s does not match target Grafana org id %s. "
            "Remove --require-matching-export-org to allow cross-org import."
            % (source_org_id, target_org_id)
        )
    return target_org_id


def build_existing_datasource_lookups(
    client: GrafanaClient,
) -> Dict[str, Dict[str, List[Dict[str, Any]]]]:
    by_uid = {}
    by_name = {}
    for datasource in client.list_datasources():
        uid = str(datasource.get("uid") or "")
        name = str(datasource.get("name") or "")
        if uid:
            by_uid.setdefault(uid, []).append(datasource)
        if name:
            by_name.setdefault(name, []).append(datasource)
    return {"by_uid": by_uid, "by_name": by_name}


def resolve_datasource_match(
    record: Dict[str, str],
    lookups: Dict[str, Dict[str, List[Dict[str, Any]]]],
) -> Dict[str, Any]:
    uid = str(record.get("uid") or "")
    name = str(record.get("name") or "")
    if uid:
        matches = lookups["by_uid"].get(uid) or []
        if len(matches) > 1:
            return {"state": "ambiguous", "target": None}
        if len(matches) == 1:
            return {"state": "exists-uid", "target": matches[0]}
    if name:
        matches = lookups["by_name"].get(name) or []
        if len(matches) > 1:
            return {"state": "ambiguous", "target": None}
        if len(matches) == 1:
            return {"state": "exists-name", "target": matches[0]}
    return {"state": "missing", "target": None}


def determine_import_mode(args: argparse.Namespace) -> str:
    if bool(getattr(args, "update_existing_only", False)):
        return "update-or-skip-missing"
    if bool(getattr(args, "replace_existing", False)):
        return "create-or-update"
    return "create-only"


def determine_datasource_action(
    args: argparse.Namespace,
    record: Dict[str, str],
    match: Dict[str, Any],
) -> str:
    state = match["state"]
    existing = match.get("target")
    if state == "ambiguous":
        return "would-fail-ambiguous"
    if existing is not None:
        existing_type = str(existing.get("type") or "")
        incoming_type = str(record.get("type") or "")
        if existing_type and incoming_type and existing_type != incoming_type:
            return "would-fail-plugin-type-change"
    if state == "missing":
        if bool(getattr(args, "update_existing_only", False)):
            return "would-skip-missing"
        return "would-create"
    if bool(getattr(args, "replace_existing", False)) or bool(
        getattr(args, "update_existing_only", False)
    ):
        return "would-update"
    return "would-fail-existing"


def build_import_payload(
    record: Dict[str, str],
    existing: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    payload = {
        "name": record.get("name") or "",
        "type": record.get("type") or "",
        "access": record.get("access") or "",
        "url": record.get("url") or "",
        "isDefault": str(record.get("isDefault") or "").lower() == "true",
    }
    uid = record.get("uid") or ""
    if uid:
        payload["uid"] = uid
    if existing is not None:
        datasource_id = existing.get("id")
        if datasource_id is not None:
            payload["id"] = datasource_id
    return payload


def render_import_dry_run_table(
    records: List[Dict[str, str]],
    include_header: bool,
) -> List[str]:
    headers = ["UID", "NAME", "TYPE", "DESTINATION", "ACTION", "ORG_ID", "FILE"]
    rows = [
        [
            item.get("uid") or "",
            item.get("name") or "",
            item.get("type") or "",
            item.get("destination") or "",
            item.get("action") or "",
            item.get("orgId") or "",
            item.get("file") or "",
        ]
        for item in records
    ]
    widths = [len(value) for value in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def render_row(values: List[str]) -> str:
        return "  ".join(values[index].ljust(widths[index]) for index in range(len(values)))

    lines = []
    if include_header:
        lines.append(render_row(headers))
        lines.append(render_row(["-" * width for width in widths]))
    for row in rows:
        lines.append(render_row(row))
    return lines


def render_import_dry_run_json(
    mode: str,
    records: List[Dict[str, str]],
    target_org_id: str,
) -> str:
    summary = {
        "datasourceCount": len(records),
        "createCount": len([item for item in records if item["action"] == "would-create"]),
        "updateCount": len([item for item in records if item["action"] == "would-update"]),
        "skipCount": len(
            [item for item in records if item["action"] == "would-skip-missing"]
        ),
        "blockedCount": len(
            [
                item
                for item in records
                if item["action"]
                in ("would-fail-existing", "would-fail-ambiguous", "would-fail-plugin-type-change")
            ]
        ),
    }
    source_org_id = ""
    if records:
        source_org_id = str(records[0].get("sourceOrgId") or "")
    return json.dumps(
        {
            "mode": mode,
            "sourceOrgId": source_org_id,
            "targetOrgId": target_org_id,
            "datasources": records,
            "summary": summary,
        },
        indent=2,
        sort_keys=False,
    )


def render_data_source_csv(datasources: List[Dict[str, Any]]) -> None:
    writer = csv.DictWriter(
        sys.stdout,
        fieldnames=["uid", "name", "type", "url", "isDefault"],
        lineterminator="\n",
    )
    writer.writeheader()
    for datasource in datasources:
        writer.writerow(build_data_source_record(datasource))


def render_data_source_json(datasources: List[Dict[str, Any]]) -> str:
    return json.dumps(
        [build_data_source_record(item) for item in datasources],
        indent=2,
        sort_keys=False,
    )


def list_datasources(args: argparse.Namespace) -> int:
    client = build_client(args)
    datasources = client.list_datasources()
    if args.csv:
        render_data_source_csv(datasources)
        return 0
    if args.json:
        print(render_data_source_json(datasources))
        return 0
    for line in render_data_source_table(
        datasources,
        include_header=not bool(getattr(args, "no_header", False)),
    ):
        print(line)
    print("")
    print(f"Listed {len(datasources)} data source(s) from {args.url}")
    return 0


def export_datasources(args: argparse.Namespace) -> int:
    client = build_client(args)
    records = build_export_records(client)
    output_dir = Path(args.export_dir)
    datasources_path = output_dir / DATASOURCE_EXPORT_FILENAME
    index_path = output_dir / "index.json"
    metadata_path = output_dir / EXPORT_METADATA_FILENAME

    existing_paths = [path for path in [datasources_path, index_path, metadata_path] if path.exists()]
    if existing_paths and not args.overwrite:
        raise GrafanaError(
            "Refusing to overwrite existing file: %s. Use --overwrite."
            % existing_paths[0]
        )

    index_document = build_export_index(records, DATASOURCE_EXPORT_FILENAME)
    metadata_document = build_export_metadata(
        datasource_count=len(records),
        datasources_file=DATASOURCE_EXPORT_FILENAME,
    )
    if not args.dry_run:
        write_json_document(records, datasources_path)
        write_json_document(index_document, index_path)
        write_json_document(metadata_document, metadata_path)
    summary_verb = "Would export" if args.dry_run else "Exported"
    print(
        "%s %s datasource(s). Datasources: %s Index: %s Manifest: %s"
        % (
            summary_verb,
            len(records),
            datasources_path,
            index_path,
            metadata_path,
        )
    )
    return 0


def import_datasources(args: argparse.Namespace) -> int:
    if getattr(args, "table", False) and not args.dry_run:
        raise GrafanaError("--table is only supported with --dry-run for datasource import.")
    if getattr(args, "json", False) and not args.dry_run:
        raise GrafanaError("--json is only supported with --dry-run for datasource import.")
    if getattr(args, "table", False) and getattr(args, "json", False):
        raise GrafanaError("--table and --json are mutually exclusive for datasource import.")
    if getattr(args, "no_header", False) and not getattr(args, "table", False):
        raise GrafanaError(
            "--no-header is only supported with --dry-run --table for datasource import."
        )
    client = build_effective_import_client(args, build_client(args))
    bundle = load_import_bundle(Path(args.import_dir))
    target_org_id = validate_export_org_match(args, client, bundle)
    lookups = build_existing_datasource_lookups(client)
    mode = determine_import_mode(args)
    records = []
    imported_count = 0
    skipped_missing_count = 0
    total = len(bundle["records"])
    if not getattr(args, "json", False):
        print("Import mode: %s" % mode)
    for index, record in enumerate(bundle["records"], 1):
        match = resolve_datasource_match(record, lookups)
        action = determine_datasource_action(args, record, match)
        dry_run_record = {
            "uid": record.get("uid") or "",
            "name": record.get("name") or "",
            "type": record.get("type") or "",
            "destination": match["state"],
            "action": action,
            "orgId": target_org_id,
            "sourceOrgId": record.get("orgId") or "",
            "file": "%s#%s" % (bundle["datasources_path"], index - 1),
        }
        if args.dry_run:
            records.append(dry_run_record)
            if getattr(args, "table", False) or getattr(args, "json", False):
                continue
            print(
                "Dry-run datasource uid=%s name=%s dest=%s action=%s file=%s"
                % (
                    dry_run_record["uid"] or "-",
                    dry_run_record["name"] or "-",
                    dry_run_record["destination"],
                    dry_run_record["action"],
                    dry_run_record["file"],
                )
            )
            continue
        if action == "would-skip-missing":
            skipped_missing_count += 1
            if getattr(args, "verbose", False):
                print(
                    "Skipped datasource uid=%s name=%s dest=missing action=skip-missing"
                    % (record.get("uid") or "-", record.get("name") or "-")
                )
            elif getattr(args, "progress", False):
                print(
                    "Skipping datasource %s/%s: %s"
                    % (index, total, record.get("uid") or record.get("name") or "-")
                )
            continue
        if action in (
            "would-fail-existing",
            "would-fail-ambiguous",
            "would-fail-plugin-type-change",
        ):
            raise GrafanaError(
                "Datasource import blocked for uid=%s name=%s action=%s"
                % (record.get("uid") or "-", record.get("name") or "-", action)
            )
        payload = build_import_payload(record, match.get("target"))
        if action == "would-update":
            datasource_id = payload.get("id")
            if datasource_id is None:
                raise GrafanaError(
                    "Datasource import could not determine destination datasource id for update."
                )
            client.request_json(
                "/api/datasources/%s" % datasource_id,
                method="PUT",
                payload=payload,
            )
        else:
            client.request_json("/api/datasources", method="POST", payload=payload)
        imported_count += 1
        if getattr(args, "verbose", False):
            print(
                "Imported datasource uid=%s name=%s action=%s"
                % (
                    record.get("uid") or "-",
                    record.get("name") or "-",
                    "update" if action == "would-update" else "create",
                )
            )
        elif getattr(args, "progress", False):
            print(
                "Importing datasource %s/%s: %s"
                % (index, total, record.get("uid") or record.get("name") or "-")
            )
    if args.dry_run:
        if getattr(args, "json", False):
            print(render_import_dry_run_json(mode, records, target_org_id))
            return 0
        if getattr(args, "table", False):
            for line in render_import_dry_run_table(
                records, include_header=not bool(getattr(args, "no_header", False))
            ):
                print(line)
        print(
            "Dry-run checked %s datasource(s) from %s"
            % (len(records), args.import_dir)
        )
        return 0
    if skipped_missing_count:
        print(
            "Imported %s datasource(s) from %s; skipped %s missing destination datasources"
            % (imported_count, args.import_dir, skipped_missing_count)
        )
    else:
        print("Imported %s datasource(s) from %s" % (imported_count, args.import_dir))
    return 0


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    try:
        if args.command == "list":
            return list_datasources(args)
        if args.command == "export":
            return export_datasources(args)
        return import_datasources(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


__all__ = [
    "DATASOURCE_EXPORT_FILENAME",
    "DEFAULT_EXPORT_DIR",
    "EXPORT_METADATA_FILENAME",
    "ROOT_INDEX_KIND",
    "TOOL_SCHEMA_VERSION",
    "build_client",
    "build_export_index",
    "build_export_metadata",
    "build_export_records",
    "build_parser",
    "export_datasources",
    "import_datasources",
    "list_datasources",
    "main",
    "parse_args",
    "render_data_source_csv",
    "render_data_source_json",
]
