"""Argparse wiring for the Python datasource CLI.

Purpose:
- Centralize datasource parser and help wiring so facade and tests share one source
  of parser truth.

Architecture:
- Keep parser construction centralized in one module so CLI arguments, aliases,
  and help examples stay stable.
- `datasource_cli.py` performs parse/normalize and delegates execution to
  `datasource.workflows`.

Caveats:
- This module should stay parser-only; do not add import/export business logic
  here.
"""

import argparse
from collections import OrderedDict

from ..dashboard_cli import (
    HelpFullAction,
    add_common_cli_args,
)

DEFAULT_EXPORT_DIR = "datasources"
DATASOURCE_EXPORT_FILENAME = "datasources.json"
EXPORT_METADATA_FILENAME = "export-metadata.json"
ROOT_INDEX_KIND = "grafana-utils-datasource-export-index"
TOOL_SCHEMA_VERSION = 1
LIST_OUTPUT_FORMAT_CHOICES = ("table", "csv", "json")
IMPORT_DRY_RUN_OUTPUT_FORMAT_CHOICES = ("text", "table", "json")
LIVE_MUTATION_DRY_RUN_OUTPUT_FORMAT_CHOICES = ("text", "table", "json")
IMPORT_DRY_RUN_COLUMN_HEADERS = OrderedDict(
    [
        ("uid", "UID"),
        ("name", "NAME"),
        ("type", "TYPE"),
        ("destination", "DESTINATION"),
        ("action", "ACTION"),
        ("orgId", "ORG_ID"),
        ("file", "FILE"),
    ]
)
IMPORT_DRY_RUN_COLUMN_ALIASES = {
    "uid": "uid",
    "name": "name",
    "type": "type",
    "destination": "destination",
    "action": "action",
    "org_id": "orgId",
    "file": "file",
}

HELP_FULL_EXAMPLES = (
    "Extended Examples:\n\n"
    "  Export datasource inventory for the current org:\n"
    "    grafana-util datasource export --url http://localhost:3000 "
    "--basic-user admin --basic-password admin --export-dir ./datasources --overwrite\n\n"
    "  Dry-run a live datasource create without changing Grafana:\n"
    "    grafana-util datasource add --url http://localhost:3000 "
    "--token \"$GRAFANA_API_TOKEN\" --name prometheus-main --type prometheus "
    "--datasource-url http://prometheus:9090 --dry-run --table\n\n"
    "  Dry-run a live datasource delete by UID:\n"
    "    grafana-util datasource delete --url http://localhost:3000 "
    "--token \"$GRAFANA_API_TOKEN\" --uid prom-main --dry-run --json\n\n"
    "  Dry-run datasource import for the current org:\n"
    "    grafana-util datasource import --url http://localhost:3000 "
    "--token \"$GRAFANA_API_TOKEN\" --import-dir ./datasources --dry-run --table\n\n"
    "  Compare an exported datasource inventory against live Grafana:\n"
    "    grafana-util datasource diff --url http://localhost:3000 "
    "--token \"$GRAFANA_API_TOKEN\" --diff-dir ./datasources\n\n"
    "  List datasource inventory as JSON for scripting:\n"
    "    grafana-util datasource list --url http://localhost:3000 "
    "--token \"$GRAFANA_API_TOKEN\" --json"
)


def add_list_cli_args(parser):
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
    parser.add_argument(
        "--output-format",
        choices=LIST_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for datasource list output. "
            "Use table, csv, or json. This cannot be combined with --table, "
            "--csv, or --json."
        ),
    )


def add_export_cli_args(parser):
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


def add_import_cli_args(parser):
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
        "--output-format",
        choices=IMPORT_DRY_RUN_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for datasource import "
            "dry-run output. Use text, table, or json. This cannot be "
            "combined with --table or --json."
        ),
    )
    parser.add_argument(
        "--output-columns",
        default=None,
        help=(
            "For --dry-run --table only, render only these comma-separated columns. "
            "Supported values: uid, name, type, destination, action, org_id, file."
        ),
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


def add_diff_cli_args(parser):
    parser.add_argument(
        "--diff-dir",
        required=True,
        help=(
            "Compare datasource inventory from this directory against live Grafana. "
            "Point this to the datasource export root that contains datasources.json "
            "and export-metadata.json."
        ),
    )


def add_add_cli_args(parser):
    parser.add_argument(
        "--uid",
        default=None,
        help="Datasource UID to create. Optional but recommended for stable identity.",
    )
    parser.add_argument(
        "--name",
        required=True,
        help="Datasource name to create.",
    )
    parser.add_argument(
        "--type",
        required=True,
        help="Grafana datasource plugin type id to create.",
    )
    parser.add_argument(
        "--access",
        default=None,
        help="Datasource access mode such as proxy or direct.",
    )
    parser.add_argument(
        "--datasource-url",
        dest="datasource_url",
        default=None,
        help="Datasource target URL to store in Grafana.",
    )
    parser.add_argument(
        "--default",
        dest="is_default",
        action="store_true",
        help="Mark the new datasource as the default datasource.",
    )
    parser.add_argument(
        "--basic-auth",
        action="store_true",
        help="Enable basic auth for the datasource.",
    )
    parser.add_argument(
        "--basic-auth-user",
        default=None,
        help="Username for datasource basic auth.",
    )
    parser.add_argument(
        "--basic-auth-password",
        default=None,
        help="Password for datasource basic auth. Stored in secureJsonData.",
    )
    parser.add_argument(
        "--user",
        default=None,
        help="Datasource user/login field where the plugin supports it.",
    )
    parser.add_argument(
        "--password",
        default=None,
        help="Datasource password field where the plugin supports it. Stored in secureJsonData.",
    )
    parser.add_argument(
        "--with-credentials",
        action="store_true",
        help="Send browser credentials such as cookies for supported datasource types.",
    )
    parser.add_argument(
        "--http-header",
        action="append",
        default=None,
        metavar="NAME=VALUE",
        help=(
            "Add one custom HTTP header for supported datasource types. May be "
            "specified multiple times."
        ),
    )
    parser.add_argument(
        "--tls-skip-verify",
        action="store_true",
        help="Set jsonData.tlsSkipVerify=true for supported datasource types.",
    )
    parser.add_argument(
        "--server-name",
        default=None,
        help="Set jsonData.serverName for supported datasource TLS validation.",
    )
    parser.add_argument(
        "--json-data",
        default=None,
        help="Inline JSON object string for datasource jsonData.",
    )
    parser.add_argument(
        "--secure-json-data",
        default=None,
        help="Inline JSON object string for datasource secureJsonData.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview what datasource add would do without changing Grafana.",
    )
    parser.add_argument(
        "--table",
        action="store_true",
        help="For --dry-run only, render a compact table instead of plain text.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="For --dry-run only, render one JSON document.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="For --dry-run --table only, omit the table header row.",
    )
    parser.add_argument(
        "--output-format",
        choices=LIVE_MUTATION_DRY_RUN_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for datasource add dry-run "
            "output. Use text, table, or json. This cannot be combined with "
            "--table or --json."
        ),
    )


def add_delete_cli_args(parser):
    target_group = parser.add_mutually_exclusive_group(required=True)
    target_group.add_argument(
        "--uid",
        default=None,
        help="Datasource UID to delete.",
    )
    target_group.add_argument(
        "--name",
        default=None,
        help="Datasource name to delete when UID is not available.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview what datasource delete would do without changing Grafana.",
    )
    parser.add_argument(
        "--table",
        action="store_true",
        help="For --dry-run only, render a compact table instead of plain text.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="For --dry-run only, render one JSON document.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="For --dry-run --table only, omit the table header row.",
    )
    parser.add_argument(
        "--output-format",
        choices=LIVE_MUTATION_DRY_RUN_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for datasource delete dry-run "
            "output. Use text, table, or json. This cannot be combined with "
            "--table or --json."
        ),
    )


def build_parser(prog=None):
    parser = argparse.ArgumentParser(
        prog=prog or "grafana-util datasource",
        description="List, export, import, or diff Grafana datasource inventory.",
        epilog=(
            "Examples:\n\n"
            "  grafana-util datasource list --url http://localhost:3000 --json\n"
            "  grafana-util datasource add --url http://localhost:3000 --name prometheus-main "
            "--type prometheus --datasource-url http://prometheus:9090 --dry-run --table\n"
            "  grafana-util datasource delete --url http://localhost:3000 --uid prom-main "
            "--dry-run --json\n"
            "  grafana-util datasource export --url http://localhost:3000 "
            "--export-dir ./datasources --overwrite\n"
            "  grafana-util datasource import --url http://localhost:3000 "
            "--import-dir ./datasources --dry-run --table\n"
            "  grafana-util datasource diff --url http://localhost:3000 "
            "--diff-dir ./datasources"
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

    diff_parser = subparsers.add_parser(
        "diff",
        help="Compare exported datasource inventory with the current Grafana state.",
    )
    add_common_cli_args(diff_parser)
    add_diff_cli_args(diff_parser)
    diff_parser.set_defaults(_help_full_examples=HELP_FULL_EXAMPLES)
    diff_parser.add_argument(
        "--help-full",
        nargs=0,
        action=HelpFullAction,
        help="Show normal help plus extended datasource examples.",
    )

    add_parser = subparsers.add_parser(
        "add",
        help="Create one live Grafana datasource through the Grafana API.",
    )
    add_common_cli_args(add_parser)
    add_add_cli_args(add_parser)
    add_parser.set_defaults(_help_full_examples=HELP_FULL_EXAMPLES)
    add_parser.add_argument(
        "--help-full",
        nargs=0,
        action=HelpFullAction,
        help="Show normal help plus extended datasource examples.",
    )

    delete_parser = subparsers.add_parser(
        "delete",
        help="Delete one live Grafana datasource through the Grafana API.",
    )
    add_common_cli_args(delete_parser)
    add_delete_cli_args(delete_parser)
    delete_parser.set_defaults(_help_full_examples=HELP_FULL_EXAMPLES)
    delete_parser.add_argument(
        "--help-full",
        nargs=0,
        action=HelpFullAction,
        help="Show normal help plus extended datasource examples.",
    )

    return parser
