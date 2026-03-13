#!/usr/bin/env python3
"""Grafana datasource list/export utility."""

import argparse
import csv
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional

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


def build_parser(prog: Optional[str] = None) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=prog or "grafana-utils datasource",
        description="List or export Grafana datasource inventory.",
        epilog=(
            "Examples:\n\n"
            "  grafana-utils datasource list --url http://localhost:3000 --json\n"
            "  grafana-utils datasource export --url http://localhost:3000 "
            "--export-dir ./datasources --overwrite"
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


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    try:
        if args.command == "list":
            return list_datasources(args)
        return export_datasources(args)
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
    "list_datasources",
    "main",
    "parse_args",
    "render_data_source_csv",
    "render_data_source_json",
]
