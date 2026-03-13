#!/usr/bin/env python3
"""Export or import Grafana dashboards.

Maintainer overview:
- The tool has two separate export targets with different consumers.
- `raw/` keeps dashboard JSON close to Grafana's API shape so it can round-trip
  back through `POST /api/dashboards/db`.
- `prompt/` rewrites datasource references into Grafana web-import `__inputs`
  placeholders so a human can choose datasources during UI import.

Architecture:
- `GrafanaClient` owns HTTP transport only.
- export flow is `list dashboards -> fetch payload -> write raw variant ->
  optionally rewrite datasources -> write prompt variant -> write indexes`.
- import flow is `discover JSON files -> reject prompt exports with __inputs ->
  normalize payload -> send to Grafana API`.

Datasource rewrite pipeline for `prompt/` exports:
- build a datasource catalog from Grafana so refs can be resolved by uid or name
- walk the dashboard tree and collect every `datasource` field
- normalize each ref into a stable key so repeated refs share one generated input
- replace dashboard refs with `${DS_*}` placeholders
- if every datasource resolves to the same plugin type, collapse panel-level
  refs to Grafana's conventional `$datasource` template variable for easier
  human maintenance after import

Keep in mind:
- `prompt/` exports are for Grafana web import, not API re-import
- `raw/` exports are the safe input for this script's import mode
"""

import argparse
import base64
import copy
import csv
import difflib
import getpass
import io
import json
import re
import sys
import tempfile
from collections import OrderedDict
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

from .clients.dashboard_client import GrafanaClient
from .dashboards.common import (
    DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE,
    DEFAULT_FOLDER_UID,
    DEFAULT_ORG_ID,
    DEFAULT_ORG_NAME,
    DEFAULT_UNKNOWN_UID,
    GrafanaApiError,
    GrafanaError,
)
from .dashboards.export_workflow import run_export_dashboards
from .dashboards.import_workflow import run_import_dashboards
from .dashboards.inspection_workflow import (
    materialize_live_inspection_export as run_materialize_live_inspection_export,
)
from .dashboards.inspection_workflow import run_inspect_export, run_inspect_live
from .dashboards.transformer import (
    build_datasource_catalog,
    build_external_export_document,
    collect_datasource_refs,
    is_builtin_datasource_ref,
    is_placeholder_string,
    lookup_datasource,
    resolve_datasource_ref,
    resolve_datasource_type_alias,
)
from .http_transport import build_json_http_transport


DEFAULT_URL = "http://localhost:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_PAGE_SIZE = 500
DEFAULT_EXPORT_DIR = "dashboards"
RAW_EXPORT_SUBDIR = "raw"
PROMPT_EXPORT_SUBDIR = "prompt"
EXPORT_METADATA_FILENAME = "export-metadata.json"
FOLDER_INVENTORY_FILENAME = "folders.json"
DATASOURCE_INVENTORY_FILENAME = "datasources.json"
TOOL_SCHEMA_VERSION = 1
ROOT_INDEX_KIND = "grafana-utils-dashboard-export-index"
REPORT_COLUMN_HEADERS = OrderedDict(
    [
        ("dashboardUid", "DASHBOARD_UID"),
        ("dashboardTitle", "DASHBOARD_TITLE"),
        ("folderPath", "FOLDER_PATH"),
        ("panelId", "PANEL_ID"),
        ("panelTitle", "PANEL_TITLE"),
        ("panelType", "PANEL_TYPE"),
        ("refId", "REF_ID"),
        ("datasource", "DATASOURCE"),
        ("queryField", "QUERY_FIELD"),
        ("metrics", "METRICS"),
        ("measurements", "MEASUREMENTS"),
        ("buckets", "BUCKETS"),
        ("query", "QUERY"),
        ("file", "FILE"),
    ]
)
OPTIONAL_REPORT_COLUMN_HEADERS = OrderedDict([("datasourceUid", "DATASOURCE_UID")])
REPORT_COLUMN_ALIASES = {
    "dashboard_uid": "dashboardUid",
    "dashboard_title": "dashboardTitle",
    "folder_path": "folderPath",
    "panel_id": "panelId",
    "panel_title": "panelTitle",
    "panel_type": "panelType",
    "ref_id": "refId",
    "query_field": "queryField",
    "datasource_uid": "datasourceUid",
}
SUPPORTED_REPORT_COLUMN_HEADERS = OrderedDict(
    list(REPORT_COLUMN_HEADERS.items()) + list(OPTIONAL_REPORT_COLUMN_HEADERS.items())
)
INSPECT_EXPORT_HELP_FULL_EXAMPLES = (
    "Extended examples:\n\n"
    "  Inspect one raw export as the default flat query table:\n"
    "    grafana-utils inspect-export --import-dir ./dashboards/raw --report\n\n"
    "  Inspect one raw export as dashboard-first grouped tables:\n"
    "    grafana-utils inspect-export --import-dir ./dashboards/raw "
    "--report tree-table\n\n"
    "  Narrow the report to one datasource and one panel id:\n"
    "    grafana-utils inspect-export --import-dir ./dashboards/raw "
    "--report tree-table --report-filter-datasource prom-main "
    "--report-filter-panel-id 7\n\n"
    "  Trim the per-query columns for flat or tree-table output:\n"
    "    grafana-utils inspect-export --import-dir ./dashboards/raw "
    "--report tree-table --report-columns panel_id,panel_title,datasource,query"
)
INSPECT_LIVE_HELP_FULL_EXAMPLES = (
    "Extended examples:\n\n"
    "  Inspect live dashboards as the default flat query table:\n"
    "    grafana-utils inspect-live --url http://localhost:3000 "
    "--basic-user admin --basic-password admin --report\n\n"
    "  Inspect live dashboards as dashboard-first grouped tables:\n"
    "    grafana-utils inspect-live --url http://localhost:3000 "
    "--basic-user admin --basic-password admin --report tree-table\n\n"
    "  Narrow live inspection to one datasource and one panel id:\n"
    "    grafana-utils inspect-live --url http://localhost:3000 "
    "--basic-user admin --basic-password admin --report tree-table "
    "--report-filter-datasource prom-main --report-filter-panel-id 7\n\n"
    "  Trim the per-query columns for flat or tree-table output:\n"
    "    grafana-utils inspect-live --url http://localhost:3000 "
    "--basic-user admin --basic-password admin --report tree-table "
    "--report-columns panel_id,panel_title,datasource,query"
)


class HelpFullAction(argparse.Action):
    """Print normal help plus a short extended examples section."""

    def __call__(self, parser, namespace, values, option_string=None):
        parser.print_help()
        examples = getattr(namespace, "_help_full_examples", "") or ""
        if examples:
            print("")
            print(examples)
        parser.exit()


def add_common_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--url",
        default=DEFAULT_URL,
        help=f"Grafana base URL (default: {DEFAULT_URL})",
    )
    parser.add_argument(
        "--token",
        "--api-token",
        dest="api_token",
        default=None,
        help=(
            "Grafana API token. Preferred flag: --token. "
            "Falls back to GRAFANA_API_TOKEN."
        ),
    )
    parser.add_argument(
        "--basic-user",
        "--username",
        dest="username",
        default=None,
        help=(
            "Grafana Basic auth username. Preferred flag: --basic-user. "
            "Falls back to GRAFANA_USERNAME."
        ),
    )
    parser.add_argument(
        "--basic-password",
        "--password",
        dest="password",
        default=None,
        help=(
            "Grafana Basic auth password. Preferred flag: --basic-password. "
            "Falls back to GRAFANA_PASSWORD."
        ),
    )
    parser.add_argument(
        "--prompt-password",
        action="store_true",
        help=(
            "Prompt for the Grafana Basic auth password without echo instead of "
            "passing --basic-password on the command line."
        ),
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=DEFAULT_TIMEOUT,
        help=f"HTTP timeout in seconds (default: {DEFAULT_TIMEOUT}).",
    )
    parser.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )


def add_export_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--export-dir",
        default=DEFAULT_EXPORT_DIR,
        help=(
            "Directory to write exported dashboards into. Export writes two "
            f"subdirectories by default: {RAW_EXPORT_SUBDIR}/ and {PROMPT_EXPORT_SUBDIR}/."
        ),
    )
    parser.add_argument(
        "--page-size",
        type=int,
        default=DEFAULT_PAGE_SIZE,
        help=f"Dashboard search page size (default: {DEFAULT_PAGE_SIZE}).",
    )
    parser.add_argument(
        "--org-id",
        default=None,
        help="Export dashboards from one explicit Grafana organization ID instead of the current org. Use this when the same credentials can see multiple orgs.",
    )
    parser.add_argument(
        "--all-orgs",
        action="store_true",
        help="Export dashboards from every visible Grafana organization and write per-org subdirectories under the export root. Requires Basic auth.",
    )
    parser.add_argument(
        "--flat",
        action="store_true",
        help="Write dashboard JSON files directly into each export variant directory instead of recreating Grafana folder-based subdirectories on disk.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Replace existing local export files in the target directory instead of failing when a file already exists.",
    )
    parser.add_argument(
        "--without-dashboard-raw",
        action="store_true",
        help=f"Skip the API-safe {RAW_EXPORT_SUBDIR}/ export variant. Use this only when you do not need later API import or diff workflows.",
    )
    parser.add_argument(
        "--without-dashboard-prompt",
        action="store_true",
        help=f"Skip the web-import {PROMPT_EXPORT_SUBDIR}/ export variant. Use this only when you do not need Grafana UI import with datasource prompts.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview the dashboard files and indexes that would be written without changing disk.",
    )
    parser.add_argument(
        "--progress",
        action="store_true",
        help="Show concise per-dashboard export progress as current/total while processing files.",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show detailed per-dashboard export output, including paths. Supersedes --progress.",
    )


def add_list_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--page-size",
        type=int,
        default=DEFAULT_PAGE_SIZE,
        help=f"Dashboard search page size (default: {DEFAULT_PAGE_SIZE}).",
    )
    parser.add_argument(
        "--org-id",
        default=None,
        help="List dashboards from this Grafana organization ID instead of the current org context.",
    )
    parser.add_argument(
        "--all-orgs",
        action="store_true",
        help="List dashboards from every Grafana organization. Requires Basic auth.",
    )
    parser.add_argument(
        "--with-sources",
        action="store_true",
        help=(
            "For table or CSV output, fetch each dashboard payload and include resolved datasource "
            "names in the list output. JSON already includes datasource names and UIDs by default. "
            "This is slower because it makes extra API calls per dashboard."
        ),
    )
    output_group = parser.add_mutually_exclusive_group()
    output_group.add_argument(
        "--table",
        action="store_true",
        help="Render dashboard summaries as a table.",
    )
    output_group.add_argument(
        "--csv",
        action="store_true",
        help="Render dashboard summaries as CSV.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render dashboard summaries as JSON.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="Do not print table headers when rendering the default table output.",
    )


def add_list_data_sources_cli_args(parser: argparse.ArgumentParser) -> None:
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


def add_import_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--import-dir",
        required=True,
        help=(
            "Import dashboards from this directory. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly, not the combined export root."
        ),
    )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="Update an existing destination dashboard when the imported dashboard UID already exists. Without this flag, existing UIDs are blocked.",
    )
    parser.add_argument(
        "--update-existing-only",
        action="store_true",
        help="Reconcile only dashboards whose UID already exists in Grafana. Missing destination UIDs are skipped instead of created.",
    )
    parser.add_argument(
        "--import-folder-uid",
        default=None,
        help="Force every imported dashboard into one destination Grafana folder UID. This overrides any folder UID carried by the exported dashboard files.",
    )
    parser.add_argument(
        "--ensure-folders",
        action="store_true",
        help="Use the exported raw folder inventory to create any missing destination folders before import. In dry-run mode, also report folder missing/match/mismatch state first.",
    )
    parser.add_argument(
        "--import-message",
        default="Imported by grafana-utils",
        help="Version-history message to attach to each imported dashboard revision in Grafana.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview what import would do without changing Grafana. This reports whether each dashboard would create, update, or be skipped/blocked.",
    )
    parser.add_argument(
        "--table",
        action="store_true",
        help="For --dry-run only, render a compact table instead of per-dashboard log lines. With --ensure-folders, the folder check is also shown in table form.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="For --dry-run only, render one JSON document with mode, folder checks, dashboard actions, and summary counts.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="For --dry-run --table only, omit the table header row.",
    )
    parser.add_argument(
        "--progress",
        action="store_true",
        help="Show concise per-dashboard import progress as current/total while processing files. Use this for long-running batch imports.",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show detailed per-dashboard import output, including file paths, dry-run actions, and folder status details. Supersedes --progress.",
    )


def add_diff_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--import-dir",
        required=True,
        help=(
            "Compare dashboards from this directory against Grafana. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly."
        ),
    )
    parser.add_argument(
        "--import-folder-uid",
        default=None,
        help="Override the destination Grafana folder UID when building the comparison payload.",
    )
    parser.add_argument(
        "--context-lines",
        type=int,
        default=3,
        help="Number of surrounding lines to include in unified diff output (default: 3).",
    )


def add_inspect_export_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.set_defaults(_help_full_examples=INSPECT_EXPORT_HELP_FULL_EXAMPLES)
    parser.add_argument(
        "--import-dir",
        required=True,
        help=(
            "Inspect dashboards from this raw export directory. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly."
        ),
    )
    parser.add_argument(
        "--help-full",
        nargs=0,
        action=HelpFullAction,
        help="Show normal help plus extended inspect-export report examples.",
    )
    parser.add_argument(
        "--report",
        nargs="?",
        const="table",
        choices=("table", "json", "csv", "tree", "tree-table"),
        default=None,
        help=(
            "Render one full per-query inspection report. "
            "Use --report for flat table output, --report json for flat JSON, "
            "--report csv for flat CSV, --report tree for a dashboard/panel/query tree, "
            "or --report tree-table for per-dashboard tables."
        ),
    )
    parser.add_argument(
        "--report-columns",
        default=None,
        help=(
            "With --report table, csv, or tree-table, render only these comma-separated report columns. "
            "Supported values: %s."
            % ", ".join(
                list(REPORT_COLUMN_ALIASES.keys())
                + [
                    "datasource",
                    "metrics",
                    "measurements",
                    "buckets",
                    "query",
                    "file",
                ]
            )
        ),
    )
    parser.add_argument(
        "--report-filter-datasource",
        default=None,
        help=(
            "With --report, only include query report rows whose datasource label "
            "exactly matches this value."
        ),
    )
    parser.add_argument(
        "--report-filter-panel-id",
        default=None,
        help=(
            "With --report, only include query report rows whose panel id "
            "exactly matches this value."
        ),
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the export analysis as JSON instead of human-readable summary lines.",
    )
    parser.add_argument(
        "--table",
        action="store_true",
        help="Render the export analysis as multi-section tables instead of prose summary lines.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="With --table or table-like --report output, omit the per-section table header rows.",
    )


def add_inspect_live_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.set_defaults(_help_full_examples=INSPECT_LIVE_HELP_FULL_EXAMPLES)
    add_common_cli_args(parser)
    parser.add_argument(
        "--page-size",
        type=int,
        default=DEFAULT_PAGE_SIZE,
        help=f"Dashboard search page size (default: {DEFAULT_PAGE_SIZE}).",
    )
    parser.add_argument(
        "--help-full",
        nargs=0,
        action=HelpFullAction,
        help="Show normal help plus extended inspect-live report examples.",
    )
    parser.add_argument(
        "--report",
        nargs="?",
        const="table",
        choices=("table", "csv", "json", "tree", "tree-table"),
        default=None,
        help=(
            "Render one full per-query inspection report. "
            "Use --report for flat table output, --report csv for flat CSV, "
            "--report json for flat JSON, --report tree for a dashboard/panel/query tree, "
            "or --report tree-table for per-dashboard tables."
        ),
    )
    parser.add_argument(
        "--report-columns",
        default=None,
        help=(
            "With --report table, csv, or tree-table, render only these comma-separated report columns. "
            "Supported values: %s."
            % ", ".join(
                list(REPORT_COLUMN_ALIASES.keys())
                + [
                    "datasource",
                    "metrics",
                    "measurements",
                    "buckets",
                    "query",
                    "file",
                ]
            )
        ),
    )
    parser.add_argument(
        "--report-filter-datasource",
        default=None,
        help=(
            "With --report, only include query report rows whose datasource label "
            "exactly matches this value."
        ),
    )
    parser.add_argument(
        "--report-filter-panel-id",
        default=None,
        help=(
            "With --report, only include query report rows whose panel id "
            "exactly matches this value."
        ),
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the live dashboard inspection as JSON instead of human-readable summary lines.",
    )
    parser.add_argument(
        "--table",
        action="store_true",
        help="Render the live dashboard inspection as multi-section tables instead of prose summary lines.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="With --table or table-like --report output, omit the per-section table header rows.",
    )


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export or import Grafana dashboards.",
        epilog=(
            "Examples:\n\n"
            "  Export dashboards from local Grafana with Basic auth:\n"
            "    grafana-utils export-dashboard --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n"
            "  Export dashboards with an API token:\n"
            "    export GRAFANA_API_TOKEN='your-token'\n"
            "    grafana-utils export-dashboard --url http://localhost:3000 "
            "--token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n"
            "  Compare raw dashboard exports against local Grafana:\n"
            "    grafana-utils diff --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --import-dir ./dashboards/raw"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    # Keep export-only and import-only flags on separate subcommands so the
    # operator must choose the intended mode explicitly at the CLI boundary.
    subparsers = parser.add_subparsers(dest="command")
    subparsers.required = True

    export_parser = subparsers.add_parser(
        "export-dashboard",
        help="Export dashboards into raw/ and prompt/ variants.",
        epilog=(
            "Examples:\n\n"
            "  Export dashboards from local Grafana with Basic auth:\n"
            "    grafana-utils export-dashboard --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n"
            "  Export dashboards with an API token:\n"
            "    export GRAFANA_API_TOKEN='your-token'\n"
            "    grafana-utils export-dashboard --url http://localhost:3000 "
            "--token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n"
            "  Export into a flat directory layout instead of per-folder subdirectories:\n"
            "    grafana-utils export-dashboard --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --export-dir ./dashboards --flat"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(export_parser)
    add_export_cli_args(export_parser)

    list_parser = subparsers.add_parser(
        "list-dashboard",
        help="List live dashboard summaries from Grafana.",
    )
    add_common_cli_args(list_parser)
    add_list_cli_args(list_parser)

    list_data_sources_parser = subparsers.add_parser(
        "list-data-sources",
        help="List live Grafana data sources.",
    )
    add_common_cli_args(list_data_sources_parser)
    add_list_data_sources_cli_args(list_data_sources_parser)

    import_parser = subparsers.add_parser(
        "import-dashboard",
        help="Import dashboards from exported raw JSON files.",
    )
    add_common_cli_args(import_parser)
    add_import_cli_args(import_parser)

    diff_parser = subparsers.add_parser(
        "diff",
        help="Compare exported raw dashboards with the current Grafana state.",
    )
    add_common_cli_args(diff_parser)
    add_diff_cli_args(diff_parser)

    inspect_export_parser = subparsers.add_parser(
        "inspect-export",
        help="Inspect one raw dashboard export directory and summarize its structure.",
    )
    add_inspect_export_cli_args(inspect_export_parser)
    inspect_live_parser = subparsers.add_parser(
        "inspect-live",
        help="Inspect live Grafana dashboards with the same summary/report modes as inspect-export.",
    )
    add_inspect_live_cli_args(inspect_live_parser)

    return parser.parse_args(argv)


def resolve_auth(args: argparse.Namespace) -> Dict[str, str]:
    cli_token = getattr(args, "api_token", None)
    cli_username = getattr(args, "username", None)
    cli_password = getattr(args, "password", None)
    prompt_password = bool(getattr(args, "prompt_password", False))

    if cli_token and (cli_username or cli_password or prompt_password):
        raise GrafanaError(
            "Choose either token auth (--token / --api-token) or Basic auth "
            "(--basic-user / --username with --basic-password / --password / --prompt-password), not both."
        )
    if prompt_password and cli_password:
        raise GrafanaError(
            "Choose either --basic-password / --password or --prompt-password, not both."
        )
    if cli_username and not cli_password:
        if not prompt_password:
            raise GrafanaError(
                "Basic auth requires both --basic-user / --username and "
                "--basic-password / --password or --prompt-password."
            )
    if cli_password and not cli_username:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password or --prompt-password."
        )
    if prompt_password and not cli_username:
        raise GrafanaError(
            "--prompt-password requires --basic-user / --username."
        )

    token = cli_token or env_value("GRAFANA_API_TOKEN")
    if token:
        return {"Authorization": f"Bearer {token}"}

    username = cli_username or env_value("GRAFANA_USERNAME")
    password = cli_password
    if prompt_password:
        password = getpass.getpass("Grafana Basic auth password: ")
    elif password is None:
        password = env_value("GRAFANA_PASSWORD")
    if username and password:
        encoded = base64.b64encode(f"{username}:{password}".encode("utf-8")).decode(
            "ascii"
        )
        return {"Authorization": f"Basic {encoded}"}
    if username or password:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password or --prompt-password."
        )

    raise GrafanaError(
        "Authentication required. Set --token / --api-token / GRAFANA_API_TOKEN "
        "or --basic-user and --basic-password / --prompt-password / "
        "GRAFANA_USERNAME and GRAFANA_PASSWORD."
    )


def env_value(name: str) -> Optional[str]:
    import os

    value = os.environ.get(name)
    return value if value else None


def sanitize_path_component(value: str) -> str:
    normalized = re.sub(r"[^\w.\- ]+", "_", value.strip(), flags=re.UNICODE)
    normalized = re.sub(r"\s+", "_", normalized)
    normalized = re.sub(r"_+", "_", normalized)
    normalized = normalized.strip("._")
    return normalized or "untitled"


def build_output_path(
    output_dir: Path,
    summary: Dict[str, Any],
    flat: bool,
) -> Path:
    folder_title = summary.get("folderTitle") or DEFAULT_FOLDER_TITLE
    folder_name = sanitize_path_component(folder_title)
    title = sanitize_path_component(summary.get("title") or DEFAULT_DASHBOARD_TITLE)
    uid = sanitize_path_component(summary.get("uid") or DEFAULT_UNKNOWN_UID)
    filename = f"{title}__{uid}.json"
    if flat:
        return output_dir / filename
    return output_dir / folder_name / filename


def build_all_orgs_output_dir(
    output_dir: Path,
    org: Dict[str, Any],
) -> Path:
    """Return one org-prefixed export directory for multi-org dashboard exports."""
    org_id = sanitize_path_component(str(org.get("id") or DEFAULT_UNKNOWN_UID))
    org_name = sanitize_path_component(str(org.get("name") or "org"))
    return output_dir / ("org_%s_%s" % (org_id, org_name))


def build_export_variant_dirs(output_dir: Path) -> Tuple[Path, Path]:
    """Return the raw/ and prompt/ export directories for one dashboard export root."""
    return output_dir / RAW_EXPORT_SUBDIR, output_dir / PROMPT_EXPORT_SUBDIR


def write_dashboard(
    payload: Dict[str, Any],
    output_path: Path,
    overwrite: bool,
) -> None:
    """Write one dashboard JSON file, creating parent directories as needed."""
    ensure_dashboard_write_target(output_path, overwrite)
    output_path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def ensure_dashboard_write_target(
    output_path: Path,
    overwrite: bool,
    create_parents: bool = True,
) -> None:
    """Create parent directories when needed and enforce the overwrite policy."""
    if create_parents:
        output_path.parent.mkdir(parents=True, exist_ok=True)
    if output_path.exists() and not overwrite:
        raise GrafanaError(
            f"Refusing to overwrite existing file: {output_path}. Use --overwrite."
        )


def write_json_document(payload: Any, output_path: Path) -> None:
    """Write a JSON file with the formatting used by this repository."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def discover_dashboard_files(import_dir: Path) -> List[Path]:
    """Find dashboard JSON files for import and reject ambiguous combined roots."""
    if not import_dir.exists():
        raise GrafanaError(f"Import directory does not exist: {import_dir}")
    if not import_dir.is_dir():
        raise GrafanaError(f"Import path is not a directory: {import_dir}")
    if (import_dir / RAW_EXPORT_SUBDIR).is_dir() and (import_dir / PROMPT_EXPORT_SUBDIR).is_dir():
        raise GrafanaError(
            f"Import path {import_dir} looks like the combined export root. "
            f"Point --import-dir at {import_dir / RAW_EXPORT_SUBDIR}."
        )

    files = [
        path
        for path in sorted(import_dir.rglob("*.json"))
        if path.name
        not in {
            "index.json",
            EXPORT_METADATA_FILENAME,
            FOLDER_INVENTORY_FILENAME,
            DATASOURCE_INVENTORY_FILENAME,
        }
    ]
    if not files:
        raise GrafanaError(f"No dashboard JSON files found in {import_dir}")
    return files


def load_json_file(path: Path) -> Dict[str, Any]:
    """Read one dashboard document from disk and require a top-level JSON object."""
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError(f"Failed to read {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError(f"Invalid JSON in {path}: {exc}") from exc

    if not isinstance(raw, dict):
        raise GrafanaError(f"Dashboard file must contain a JSON object: {path}")
    return raw


def extract_dashboard_object(document: Dict[str, Any], error_message: str) -> Dict[str, Any]:
    """Return the dashboard object from either the wrapped or plain export shape."""
    dashboard = document.get("dashboard", document)
    if not isinstance(dashboard, dict):
        raise GrafanaError(error_message)
    return dashboard


def build_import_payload(
    document: Dict[str, Any],
    folder_uid_override: Optional[str],
    replace_existing: bool,
    message: str,
) -> Dict[str, Any]:
    """Build the POST /api/dashboards/db payload from either export shape we write."""
    if "__inputs" in document:
        raise GrafanaError(
            "Dashboard file contains Grafana web-import placeholders (__inputs). "
            "Import it through the Grafana web UI after choosing datasources."
        )

    dashboard = copy.deepcopy(
        extract_dashboard_object(document, "Dashboard payload must be a JSON object.")
    )
    dashboard["id"] = None

    meta = document.get("meta", {})
    folder_uid = folder_uid_override
    if folder_uid is None and isinstance(meta, dict):
        folder_uid = meta.get("folderUid")

    payload: Dict[str, Any] = {
        "dashboard": dashboard,
        "overwrite": replace_existing,
        "message": message,
    }
    if folder_uid:
        payload["folderUid"] = folder_uid
    return payload


def build_preserved_web_import_document(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Keep the dashboard JSON Grafana expects for web import, but clear the numeric id."""
    dashboard = copy.deepcopy(
        extract_dashboard_object(payload, "Unexpected dashboard payload from Grafana.")
    )
    dashboard["id"] = None
    return dashboard


def build_dashboard_index_item(summary: Dict[str, Any], uid: str) -> Dict[str, str]:
    """Build the shared root index metadata for one exported dashboard."""
    return {
        "uid": uid,
        "title": str(summary.get("title") or ""),
        "folder": str(summary.get("folderTitle") or ""),
        "org": str(summary.get("orgName") or DEFAULT_ORG_NAME),
        "orgId": str(summary.get("orgId") or DEFAULT_ORG_ID),
    }


def build_variant_index(
    index_items: List[Dict[str, str]],
    path_key: str,
    format_name: str,
) -> List[Dict[str, str]]:
    """Build one variant-specific index file from the shared root index items."""
    return [
        {
            "uid": item["uid"],
            "title": item["title"],
            "folder": item["folder"],
            "org": item["org"],
            "orgId": item["orgId"],
            "path": item[path_key],
            "format": format_name,
        }
        for item in index_items
        if path_key in item
    ]


def build_root_export_index(
    index_items: List[Dict[str, str]],
    raw_index_path: Optional[Path],
    prompt_index_path: Optional[Path],
) -> Dict[str, Any]:
    """Build the versioned root manifest for one dashboard export run."""
    return {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "kind": ROOT_INDEX_KIND,
        "items": index_items,
        "variants": {
            "raw": str(raw_index_path) if raw_index_path is not None else None,
            "prompt": str(prompt_index_path) if prompt_index_path is not None else None,
        },
    }


def build_export_metadata(
    variant: str,
    dashboard_count: int,
    format_name: Optional[str] = None,
    folders_file: Optional[str] = None,
    datasources_file: Optional[str] = None,
) -> Dict[str, Any]:
    """Describe one export directory in a small, versioned manifest."""
    metadata = {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "kind": ROOT_INDEX_KIND,
        "variant": variant,
        "dashboardCount": dashboard_count,
        "indexFile": "index.json",
    }
    if format_name:
        metadata["format"] = format_name
    if folders_file:
        metadata["foldersFile"] = folders_file
    if datasources_file:
        metadata["datasourcesFile"] = datasources_file
    return metadata


def build_folder_inventory_record(
    folder: Dict[str, Any],
    org: Dict[str, Any],
    fallback_title: str,
) -> Dict[str, str]:
    uid = str(folder.get("uid") or "")
    title = str(folder.get("title") or fallback_title or uid or DEFAULT_FOLDER_TITLE)
    parents = folder.get("parents")
    parent_uid = ""
    if isinstance(parents, list) and parents:
        last_parent = parents[-1]
        if isinstance(last_parent, dict):
            parent_uid = str(last_parent.get("uid") or "")
    return {
        "uid": uid,
        "title": title,
        "parentUid": parent_uid,
        "path": build_folder_path(folder, title),
        "org": str(org.get("name") or DEFAULT_ORG_NAME),
        "orgId": str(org.get("id") or DEFAULT_ORG_ID),
    }


def collect_folder_inventory(
    client: "GrafanaClient",
    org: Dict[str, Any],
    summaries: List[Dict[str, Any]],
) -> List[Dict[str, str]]:
    folders_by_uid: Dict[str, Dict[str, str]] = {}
    pending: List[Dict[str, str]] = []
    for summary in summaries:
        folder_uid = str(summary.get("folderUid") or "").strip()
        folder_title = str(summary.get("folderTitle") or DEFAULT_FOLDER_TITLE)
        if folder_uid:
            pending.append({"uid": folder_uid, "title": folder_title})

    while pending:
        item = pending.pop()
        folder_uid = item["uid"]
        if not folder_uid or folder_uid in folders_by_uid:
            continue
        folder = client.fetch_folder_if_exists(folder_uid)
        if not folder:
            continue
        folders_by_uid[folder_uid] = build_folder_inventory_record(folder, org, item["title"])
        parents = folder.get("parents")
        if isinstance(parents, list):
            for parent in parents:
                if isinstance(parent, dict):
                    parent_uid = str(parent.get("uid") or "").strip()
                    parent_title = str(parent.get("title") or parent_uid or "folder")
                    if parent_uid and parent_uid not in folders_by_uid:
                        pending.append({"uid": parent_uid, "title": parent_title})

    return sorted(
        folders_by_uid.values(),
        key=lambda item: (item["orgId"], item["path"], item["uid"]),
    )


def load_folder_inventory(
    import_dir: Path,
    metadata: Optional[Dict[str, Any]] = None,
) -> List[Dict[str, str]]:
    folders_file = FOLDER_INVENTORY_FILENAME
    if isinstance(metadata, dict):
        folders_file = str(metadata.get("foldersFile") or FOLDER_INVENTORY_FILENAME)
    path = import_dir / folders_file
    if not path.is_file():
        return []
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc)) from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc)) from exc
    if not isinstance(raw, list):
        raise GrafanaError("Folder inventory file must contain a JSON array: %s" % path)
    records: List[Dict[str, str]] = []
    for item in raw:
        if not isinstance(item, dict):
            raise GrafanaError("Folder inventory entry must be a JSON object: %s" % path)
        records.append(
            {
                "uid": str(item.get("uid") or ""),
                "title": str(item.get("title") or ""),
                "parentUid": str(item.get("parentUid") or ""),
                "path": str(item.get("path") or ""),
                "org": str(item.get("org") or ""),
                "orgId": str(item.get("orgId") or ""),
            }
        )
    return records


def load_datasource_inventory(
    import_dir: Path,
    metadata: Optional[Dict[str, Any]] = None,
) -> List[Dict[str, str]]:
    datasources_file = DATASOURCE_INVENTORY_FILENAME
    if isinstance(metadata, dict):
        datasources_file = str(
            metadata.get("datasourcesFile") or DATASOURCE_INVENTORY_FILENAME
        )
    path = import_dir / datasources_file
    if not path.is_file():
        return []
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc)) from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc)) from exc
    if not isinstance(raw, list):
        raise GrafanaError("Datasource inventory file must contain a JSON array: %s" % path)
    records: List[Dict[str, str]] = []
    for item in raw:
        if not isinstance(item, dict):
            raise GrafanaError("Datasource inventory entry must be a JSON object: %s" % path)
        records.append(
            {
                "uid": str(item.get("uid") or ""),
                "name": str(item.get("name") or ""),
                "type": str(item.get("type") or ""),
                "access": str(item.get("access") or ""),
                "url": str(item.get("url") or ""),
                "isDefault": str(item.get("isDefault") or "false"),
                "org": str(item.get("org") or ""),
                "orgId": str(item.get("orgId") or ""),
            }
        )
    return records


def ensure_folder_inventory(
    client: "GrafanaClient",
    folders: List[Dict[str, str]],
) -> int:
    created_count = 0
    sorted_folders = sorted(
        folders,
        key=lambda item: (item.get("path", "").count(" / "), item.get("path", ""), item.get("uid", "")),
    )
    for folder in sorted_folders:
        uid = folder.get("uid") or ""
        title = folder.get("title") or uid
        parent_uid = folder.get("parentUid") or None
        if not uid:
            continue
        if client.fetch_folder_if_exists(uid) is not None:
            continue
        client.create_folder(uid=uid, title=title, parent_uid=parent_uid)
        created_count += 1
    return created_count


def inspect_folder_inventory(
    client: "GrafanaClient",
    folders: List[Dict[str, str]],
) -> List[Dict[str, str]]:
    records = []
    sorted_folders = sorted(
        folders,
        key=lambda item: (
            item.get("path", "").count(" / "),
            item.get("path", ""),
            item.get("uid", ""),
        ),
    )
    for folder in sorted_folders:
        uid = str(folder.get("uid") or "")
        if not uid:
            continue
        expected_path = str(folder.get("path") or "")
        status = determine_folder_inventory_status(client, folder)
        live_folder = build_live_folder_inventory_record(client, uid)
        if live_folder is None:
            records.append(
                {
                    "uid": uid,
                    "destination": "missing",
                    "status": "missing",
                    "reason": "would-create",
                    "expected_path": expected_path,
                    "actual_path": "",
                }
            )
            continue
        records.append(
            {
                "uid": uid,
                "destination": "exists",
                "status": status.get("status") or "unknown",
                "reason": status.get("details") or "",
                "expected_path": expected_path,
                "actual_path": str(live_folder.get("path") or ""),
            }
        )
    return records


def resolve_folder_inventory_requirements(
    args: argparse.Namespace,
    import_dir: Path,
    metadata: Optional[Dict[str, Any]],
) -> List[Dict[str, str]]:
    """Load the optional folder inventory and enforce explicit operator intent."""
    folder_inventory = load_folder_inventory(import_dir, metadata=metadata)
    if getattr(args, "import_folder_uid", None) is not None:
        return folder_inventory
    if getattr(args, "ensure_folders", False) and not folder_inventory:
        folders_file = FOLDER_INVENTORY_FILENAME
        if isinstance(metadata, dict):
            folders_file = str(metadata.get("foldersFile") or FOLDER_INVENTORY_FILENAME)
        raise GrafanaError(
            "Folder inventory file not found for --ensure-folders: %s. "
            "Re-export dashboards with raw folder inventory or omit --ensure-folders."
            % (import_dir / folders_file)
        )
    return folder_inventory


def build_folder_inventory_lookup(
    folders: List[Dict[str, str]],
) -> Dict[str, Dict[str, str]]:
    lookup: Dict[str, Dict[str, str]] = {}
    for folder in folders:
        uid = str(folder.get("uid") or "")
        if uid:
            lookup[uid] = dict(folder)
    return lookup


def build_import_dashboard_folder_path(dashboard_file: Path, import_dir: Path) -> str:
    relative_path = dashboard_file.relative_to(import_dir)
    parts = list(relative_path.parts[:-1])
    return " / ".join(parts)


def resolve_folder_inventory_record_for_dashboard(
    document: Dict[str, Any],
    dashboard_file: Path,
    import_dir: Path,
    folder_lookup: Dict[str, Dict[str, str]],
) -> Optional[Dict[str, str]]:
    def build_general_record() -> Dict[str, str]:
        return {
            "uid": DEFAULT_FOLDER_UID,
            "title": DEFAULT_FOLDER_TITLE,
            "parentUid": "",
            "path": DEFAULT_FOLDER_TITLE,
            "builtin": "true",
        }

    meta = document.get("meta")
    if isinstance(meta, dict):
        folder_uid = str(meta.get("folderUid") or "")
        if folder_uid and folder_uid in folder_lookup:
            return dict(folder_lookup[folder_uid])
        if folder_uid == DEFAULT_FOLDER_UID:
            return build_general_record()

    folder_path = build_import_dashboard_folder_path(dashboard_file, import_dir)
    if not folder_path:
        return None
    if folder_path == DEFAULT_FOLDER_TITLE:
        return build_general_record()
    if " / " not in folder_path:
        title_matches = []
        for record in folder_lookup.values():
            if str(record.get("title") or "") == folder_path:
                title_matches.append(dict(record))
        if len(title_matches) == 1:
            return title_matches[0]
    for record in folder_lookup.values():
        if str(record.get("path") or "") == folder_path:
            return dict(record)
    return None


def build_live_folder_inventory_record(
    client: "GrafanaClient",
    uid: str,
) -> Optional[Dict[str, str]]:
    if not uid:
        return None
    folder = client.fetch_folder_if_exists(uid)
    if folder is None:
        return None
    title = str(folder.get("title") or uid)
    parents = folder.get("parents")
    if isinstance(parents, list):
        parent_uid = ""
        if parents:
            last_parent = parents[-1]
            if isinstance(last_parent, dict):
                parent_uid = str(last_parent.get("uid") or "")
        return {
            "uid": uid,
            "title": title,
            "parentUid": parent_uid,
            "path": build_folder_path(folder, title),
        }

    parent_uid = str(folder.get("parentUid") or "")
    path_titles = [title]
    seen = set([uid])
    current_parent_uid = parent_uid
    while current_parent_uid:
        if current_parent_uid in seen:
            break
        seen.add(current_parent_uid)
        parent = client.fetch_folder_if_exists(current_parent_uid)
        if parent is None:
            break
        parent_title = str(parent.get("title") or current_parent_uid)
        path_titles.append(parent_title)
        current_parent_uid = str(parent.get("parentUid") or "")
    path_titles.reverse()
    return {
        "uid": uid,
        "title": title,
        "parentUid": parent_uid,
        "path": " / ".join(path_titles),
    }


def determine_folder_inventory_status(
    client: "GrafanaClient",
    expected_folder: Optional[Dict[str, str]],
) -> Dict[str, str]:
    if expected_folder is None:
        return {"status": "unknown", "details": ""}
    if str(expected_folder.get("builtin") or "") == "true":
        return {"status": "general", "details": "default-grafana"}

    uid = str(expected_folder.get("uid") or "")
    live_folder = build_live_folder_inventory_record(client, uid)
    if live_folder is None:
        return {"status": "missing", "details": ""}

    mismatch_fields = []
    for field in ("title", "parentUid", "path"):
        if str(expected_folder.get(field) or "") != str(live_folder.get(field) or ""):
            mismatch_fields.append(field)
    if mismatch_fields:
        return {"status": "mismatch", "details": ",".join(mismatch_fields)}
    return {"status": "match", "details": ""}


def resolve_dashboard_import_folder_path(
    client: "GrafanaClient",
    payload: Dict[str, Any],
    document: Dict[str, Any],
    dashboard_file: Path,
    import_dir: Path,
    folder_inventory_lookup: Dict[str, Dict[str, str]],
) -> str:
    """Resolve the effective destination folder path for one dashboard import."""
    folder_uid = str(payload.get("folderUid") or "").strip()
    if not folder_uid or folder_uid == DEFAULT_FOLDER_UID:
        return DEFAULT_FOLDER_TITLE

    live_folder = client.fetch_folder_if_exists(folder_uid)
    if isinstance(live_folder, dict):
        return build_folder_path(live_folder, str(live_folder.get("title") or folder_uid))

    inventory_record = folder_inventory_lookup.get(folder_uid)
    if inventory_record is None:
        inventory_record = resolve_folder_inventory_record_for_dashboard(
            document,
            dashboard_file,
            import_dir,
            folder_inventory_lookup,
        )
        if (
            inventory_record is None
            or str(inventory_record.get("uid") or "").strip() != folder_uid
        ):
            inventory_record = None
    if inventory_record is not None:
        path = str(inventory_record.get("path") or "").strip()
        if path:
            return path
        title = str(inventory_record.get("title") or folder_uid).strip()
        if title:
            return title
    return folder_uid


def print_dashboard_export_progress(
    args: argparse.Namespace,
    index: int,
    total: int,
    uid: str,
    variant: str,
    path: Path,
    dry_run: bool,
) -> None:
    """Render one export progress update in concise or verbose form."""
    if getattr(args, "verbose", False):
        print(
            "%s %s    %s -> %s"
            % ("Would export" if dry_run else "Exported", variant, uid, path)
        )


def print_dashboard_export_progress_summary(
    args: argparse.Namespace,
    index: int,
    total: int,
    uid: str,
    dry_run: bool,
) -> None:
    """Render one concise export progress update per dashboard."""
    if getattr(args, "verbose", False):
        return
    if getattr(args, "progress", False):
        print(
            "%s dashboard %s/%s: %s"
            % ("Would export" if dry_run else "Exporting", index, total, uid)
        )


def print_dashboard_import_progress(
    args: argparse.Namespace,
    index: int,
    total: int,
    dashboard_file: Path,
    uid: str,
    action: Optional[str] = None,
    status: Optional[str] = None,
    folder_status: Optional[str] = None,
    folder_details: Optional[str] = None,
    folder_path: Optional[str] = None,
    dry_run: bool = False,
) -> None:
    """Render one import progress update in concise or verbose form."""
    destination = None
    action_label = action or "unknown"
    if action:
        if action == "would-create":
            destination = "missing"
            action_label = "create"
        elif action == "would-skip-missing":
            destination = "missing"
            action_label = "skip-missing"
        elif action in ("would-update", "would-fail-existing"):
            destination = "exists"
            if action == "would-update":
                action_label = "update"
            else:
                action_label = "blocked-existing"
        else:
            destination = "unknown"
    folder_segment = ""
    if dry_run and folder_path:
        folder_segment = " folderPath=%s" % folder_path
    if getattr(args, "verbose", False):
        if dry_run:
            print(
                "Dry-run import uid=%s dest=%s action=%s%s file=%s"
                % (uid, destination or "unknown", action_label, folder_segment, dashboard_file)
            )
        else:
            print("Imported %s -> uid=%s status=%s" % (dashboard_file, uid, status or "unknown"))
        return
    if getattr(args, "progress", False):
        if dry_run:
            print(
                "Dry-run dashboard %s/%s: %s dest=%s action=%s%s"
                % (index, total, uid, destination or "unknown", action_label, folder_segment)
            )
        else:
            print("Importing dashboard %s/%s: %s" % (index, total, uid))


def load_export_metadata(
    import_dir: Path,
    expected_variant: Optional[str] = None,
) -> Optional[Dict[str, Any]]:
    """Load the optional export manifest and validate its schema version when present."""
    metadata_path = import_dir / EXPORT_METADATA_FILENAME
    if not metadata_path.is_file():
        return None
    metadata = load_json_file(metadata_path)
    validate_export_metadata(
        metadata,
        metadata_path=metadata_path,
        expected_variant=expected_variant,
    )
    return metadata


def validate_export_metadata(
    metadata: Dict[str, Any],
    metadata_path: Path,
    expected_variant: Optional[str] = None,
) -> None:
    """Reject dashboard export manifests this implementation does not understand."""
    if metadata.get("kind") != ROOT_INDEX_KIND:
        raise GrafanaError(
            f"Unexpected dashboard export manifest kind in {metadata_path}: "
            f"{metadata.get('kind')!r}"
        )

    schema_version = metadata.get("schemaVersion")
    if schema_version != TOOL_SCHEMA_VERSION:
        raise GrafanaError(
            f"Unsupported dashboard export schemaVersion {schema_version!r} in "
            f"{metadata_path}. Expected {TOOL_SCHEMA_VERSION}."
        )

    if expected_variant is None:
        return
    variant = metadata.get("variant")
    if variant != expected_variant:
        raise GrafanaError(
            f"Dashboard export manifest {metadata_path} describes variant {variant!r}. "
            f"Point this command at the {expected_variant}/ directory."
        )


def build_compare_document(
    dashboard: Dict[str, Any],
    folder_uid: Optional[str],
) -> Dict[str, Any]:
    """Build the normalized comparison shape shared by import dry-run and diff."""
    compare_document = {"dashboard": copy.deepcopy(dashboard)}
    if folder_uid:
        compare_document["folderUid"] = folder_uid
    return compare_document


def build_local_compare_document(
    document: Dict[str, Any],
    folder_uid_override: Optional[str],
) -> Dict[str, Any]:
    """Normalize one local raw export into the shape compared against Grafana."""
    payload = build_import_payload(
        document=document,
        folder_uid_override=folder_uid_override,
        replace_existing=False,
        message="",
    )
    return build_compare_document(payload["dashboard"], payload.get("folderUid"))


def build_remote_compare_document(
    payload: Dict[str, Any],
    folder_uid_override: Optional[str],
) -> Dict[str, Any]:
    """Normalize one live dashboard wrapper into the same diff shape as local files."""
    dashboard = build_preserved_web_import_document(payload)
    # Raw exports do not persist Grafana's meta.folderUid, so only compare folder
    # placement when the operator explicitly requests an override.
    return build_compare_document(dashboard, folder_uid_override)


def serialize_compare_document(document: Dict[str, Any]) -> str:
    """Serialize normalized compare data so nested JSON can be compared stably."""
    return json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def build_compare_diff_lines(
    remote_compare: Dict[str, Any],
    local_compare: Dict[str, Any],
    uid: str,
    dashboard_file: Path,
    context_lines: int,
) -> List[str]:
    """Render a unified diff for one dashboard comparison."""
    remote_lines = json.dumps(
        remote_compare,
        indent=2,
        sort_keys=True,
        ensure_ascii=False,
    ).splitlines()
    local_lines = json.dumps(
        local_compare,
        indent=2,
        sort_keys=True,
        ensure_ascii=False,
    ).splitlines()
    return list(
        difflib.unified_diff(
            remote_lines,
            local_lines,
            fromfile=f"grafana:{uid}",
            tofile=str(dashboard_file),
            lineterm="",
            n=max(context_lines, 0),
        )
    )


def resolve_dashboard_uid_for_import(document: Dict[str, Any]) -> str:
    """Return the stable dashboard UID used by dry-run and diff workflows."""
    payload = build_import_payload(
        document=document,
        folder_uid_override=None,
        replace_existing=False,
        message="",
    )
    uid = str(payload["dashboard"].get("uid") or "")
    if not uid:
        raise GrafanaError("Dashboard import document is missing dashboard.uid.")
    return uid


def determine_dashboard_import_action(
    client: "GrafanaClient",
    payload: Dict[str, Any],
    replace_existing: bool,
    update_existing_only: bool = False,
) -> str:
    """Predict whether one dashboard import would create, update, or fail."""
    uid = str(payload["dashboard"].get("uid") or "")
    if not uid:
        return "would-create"

    try:
        client.fetch_dashboard(uid)
    except GrafanaApiError as exc:
        if exc.status_code == 404:
            if update_existing_only:
                return "would-skip-missing"
            return "would-create"
        raise

    if replace_existing or update_existing_only:
        return "would-update"
    return "would-fail-existing"


def determine_import_folder_uid_override(
    client: "GrafanaClient",
    uid: str,
    folder_uid_override: Optional[str],
    preserve_existing_folder: bool,
) -> Optional[str]:
    """Prefer an explicit override, otherwise keep the destination folder for updates."""
    if folder_uid_override is not None:
        return folder_uid_override
    if not preserve_existing_folder or not uid:
        return None
    existing_payload = client.fetch_dashboard_if_exists(uid)
    if existing_payload is None:
        return None
    meta = existing_payload.get("meta")
    if not isinstance(meta, dict):
        return ""
    return str(meta.get("folderUid") or "")


def describe_dashboard_import_mode(
    replace_existing: bool,
    update_existing_only: bool,
) -> str:
    """Return the operator-facing import mode label."""
    if update_existing_only:
        return "update-or-skip-missing"
    if replace_existing:
        return "create-or-update"
    return "create-only"


def _build_export_workflow_deps() -> Dict[str, Any]:
    return {
        "GrafanaError": GrafanaError,
        "DATASOURCE_INVENTORY_FILENAME": DATASOURCE_INVENTORY_FILENAME,
        "EXPORT_METADATA_FILENAME": EXPORT_METADATA_FILENAME,
        "FOLDER_INVENTORY_FILENAME": FOLDER_INVENTORY_FILENAME,
        "PROMPT_EXPORT_SUBDIR": PROMPT_EXPORT_SUBDIR,
        "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
        "attach_dashboard_org": attach_dashboard_org,
        "build_all_orgs_output_dir": build_all_orgs_output_dir,
        "build_client": build_client,
        "build_dashboard_index_item": build_dashboard_index_item,
        "build_datasource_catalog": build_datasource_catalog,
        "build_datasource_inventory_record": build_datasource_inventory_record,
        "build_export_metadata": build_export_metadata,
        "build_export_variant_dirs": build_export_variant_dirs,
        "build_external_export_document": build_external_export_document,
        "build_output_path": build_output_path,
        "build_preserved_web_import_document": build_preserved_web_import_document,
        "build_root_export_index": build_root_export_index,
        "build_variant_index": build_variant_index,
        "collect_folder_inventory": collect_folder_inventory,
        "ensure_dashboard_write_target": ensure_dashboard_write_target,
        "print_dashboard_export_progress": print_dashboard_export_progress,
        "print_dashboard_export_progress_summary": print_dashboard_export_progress_summary,
        "sys": sys,
        "write_dashboard": write_dashboard,
        "write_json_document": write_json_document,
    }


def export_dashboards(args: argparse.Namespace) -> int:
    """Export dashboards into raw JSON, prompt JSON, or both variants."""
    return run_export_dashboards(args, _build_export_workflow_deps())


def format_dashboard_summary_line(summary: Dict[str, Any]) -> str:
    """Render one live dashboard summary in a compact operator-readable form."""
    record = build_dashboard_summary_record(summary)
    line = (
        f"uid={record['uid']} name={record['name']} folder={record['folder']} "
        f"folderUid={record['folderUid']} path={record['path']} "
        f"org={record['org']} orgId={record['orgId']}"
    )
    if record.get("sources"):
        line += f" sources={record['sources']}"
    return line


def build_dashboard_summary_record(summary: Dict[str, Any]) -> Dict[str, str]:
    """Normalize a dashboard summary into a stable output record."""
    folder = str(summary.get("folderTitle") or DEFAULT_FOLDER_TITLE)
    record = {
        "uid": str(summary.get("uid") or DEFAULT_UNKNOWN_UID),
        "name": str(summary.get("title") or DEFAULT_DASHBOARD_TITLE),
        "folder": folder,
        "folderUid": str(summary.get("folderUid") or DEFAULT_FOLDER_UID),
        "path": str(summary.get("folderPath") or folder),
        "org": str(summary.get("orgName") or DEFAULT_ORG_NAME),
        "orgId": str(summary.get("orgId") or DEFAULT_ORG_ID),
    }
    if "sources" in summary:
        record["sources"] = ",".join(summary.get("sources") or [])
    if "sourceUids" in summary:
        record["sourceUids"] = ",".join(summary.get("sourceUids") or [])
    return record


def build_folder_path(folder: Dict[str, Any], fallback_title: str) -> str:
    """Build a readable folder tree path from Grafana folder metadata."""
    parents = folder.get("parents")
    titles: List[str] = []
    if isinstance(parents, list):
        for parent in parents:
            if isinstance(parent, dict):
                title = str(parent.get("title") or "").strip()
                if title:
                    titles.append(title)
    title = (
        str(folder.get("title") or fallback_title or DEFAULT_FOLDER_TITLE).strip()
        or DEFAULT_FOLDER_TITLE
    )
    titles.append(title)
    return " / ".join(titles)


def attach_dashboard_folder_paths(
    client: GrafanaClient,
    summaries: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    """Attach a resolved folder tree path to each dashboard summary when possible."""
    folder_paths: Dict[str, str] = {}
    for summary in summaries:
        folder_uid = str(summary.get("folderUid") or "").strip()
        folder_title = str(summary.get("folderTitle") or DEFAULT_FOLDER_TITLE)
        if not folder_uid:
            continue
        if folder_uid in folder_paths:
            continue
        folder = client.fetch_folder_if_exists(folder_uid)
        if folder is None:
            folder_paths[folder_uid] = folder_title
            continue
        folder_paths[folder_uid] = build_folder_path(folder, folder_title)

    enriched: List[Dict[str, Any]] = []
    for summary in summaries:
        item = dict(summary)
        folder_uid = str(item.get("folderUid") or "").strip()
        folder_title = str(item.get("folderTitle") or DEFAULT_FOLDER_TITLE)
        item["folderPath"] = folder_paths.get(folder_uid, folder_title)
        enriched.append(item)
    return enriched


def describe_datasource_ref(
    ref: Any,
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Optional[str]:
    """Resolve one datasource reference into a display label when possible."""
    if ref is None or is_builtin_datasource_ref(ref):
        return None

    if isinstance(ref, str):
        if is_placeholder_string(ref):
            return None
        datasource = lookup_datasource(
            datasources_by_uid,
            datasources_by_name,
            uid=ref,
            name=ref,
        )
        if datasource is not None:
            label = datasource.get("name") or ref
            if isinstance(label, str) and label:
                return label
        datasource_type = resolve_datasource_type_alias(ref, datasources_by_uid)
        if datasource_type is not None:
            return datasource_type
        return ref

    if isinstance(ref, dict):
        uid = ref.get("uid")
        name = ref.get("name")
        ds_type = ref.get("type")
        has_placeholder = (
            isinstance(uid, str)
            and is_placeholder_string(uid)
            or isinstance(name, str)
            and is_placeholder_string(name)
        )
        if has_placeholder:
            return None
        datasource = lookup_datasource(
            datasources_by_uid,
            datasources_by_name,
            uid=uid,
            name=name,
        )
        if datasource is not None:
            label = datasource.get("name") or name or uid
            if isinstance(label, str) and label:
                return label
        for candidate in (name, uid, ds_type):
            if isinstance(candidate, str) and candidate:
                return candidate
    return None


def resolve_datasource_uid(
    ref: Any,
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Optional[str]:
    """Resolve one datasource reference into a concrete datasource UID when possible."""
    if ref is None or is_builtin_datasource_ref(ref):
        return None

    if isinstance(ref, str):
        if is_placeholder_string(ref):
            return None
        datasource = lookup_datasource(
            datasources_by_uid,
            datasources_by_name,
            uid=ref,
            name=ref,
        )
        if datasource is None:
            return None
        uid = datasource.get("uid")
        if isinstance(uid, str) and uid:
            return uid
        return None

    if isinstance(ref, dict):
        uid = ref.get("uid")
        name = ref.get("name")
        has_placeholder = (
            isinstance(uid, str)
            and is_placeholder_string(uid)
            or isinstance(name, str)
            and is_placeholder_string(name)
        )
        if has_placeholder:
            return None
        datasource = lookup_datasource(
            datasources_by_uid,
            datasources_by_name,
            uid=uid,
            name=name,
        )
        if datasource is not None:
            resolved_uid = datasource.get("uid")
            if isinstance(resolved_uid, str) and resolved_uid:
                return resolved_uid
        if isinstance(uid, str) and uid:
            return uid
    return None


def resolve_dashboard_source_metadata(
    payload: Dict[str, Any],
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Tuple[List[str], List[str]]:
    """Collect sorted datasource display names and concrete UIDs from one dashboard payload."""
    dashboard = extract_dashboard_object(
        payload,
        "Unexpected dashboard payload from Grafana.",
    )
    refs: List[Any] = []
    collect_datasource_refs(dashboard, refs)
    source_names: Set[str] = set()
    source_uids: Set[str] = set()
    for ref in refs:
        try:
            resolved = resolve_datasource_ref(
                ref,
                datasources_by_uid=datasources_by_uid,
                datasources_by_name=datasources_by_name,
            )
        except GrafanaError:
            resolved = None
        if resolved is not None:
            label = resolved.get("label")
            if isinstance(label, str) and label:
                source_names.add(label)

        label = describe_datasource_ref(
            ref,
            datasources_by_uid=datasources_by_uid,
            datasources_by_name=datasources_by_name,
        )
        if label:
            source_names.add(label)
        uid = resolve_datasource_uid(
            ref,
            datasources_by_uid=datasources_by_uid,
            datasources_by_name=datasources_by_name,
        )
        if uid:
            source_uids.add(uid)
    return sorted(source_names), sorted(source_uids)


def attach_dashboard_sources(
    client: GrafanaClient,
    summaries: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    """Attach sorted datasource display names to each dashboard summary."""
    datasources_by_uid, datasources_by_name = build_datasource_catalog(
        client.list_datasources()
    )
    enriched: List[Dict[str, Any]] = []
    for summary in summaries:
        item = dict(summary)
        uid = str(item.get("uid") or "").strip()
        if uid:
            payload = client.fetch_dashboard(uid)
            sources, source_uids = resolve_dashboard_source_metadata(
                payload,
                datasources_by_uid=datasources_by_uid,
                datasources_by_name=datasources_by_name,
            )
            item["sources"] = sources
            item["sourceUids"] = source_uids
        else:
            item["sources"] = []
            item["sourceUids"] = []
        enriched.append(item)
    return enriched


def attach_dashboard_org(
    client: GrafanaClient,
    summaries: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    """Attach the current Grafana organization to each dashboard summary."""
    org = client.fetch_current_org()
    org_name = str(org.get("name") or DEFAULT_ORG_NAME)
    org_id = str(org.get("id") or DEFAULT_ORG_ID)
    enriched: List[Dict[str, Any]] = []
    for summary in summaries:
        item = dict(summary)
        item["orgName"] = org_name
        item["orgId"] = org_id
        enriched.append(item)
    return enriched


def render_dashboard_summary_table(
    summaries: List[Dict[str, Any]],
    include_header: bool = True,
) -> List[str]:
    """Render dashboard summaries as a fixed-width table."""
    headers = ["UID", "NAME", "FOLDER", "FOLDER_UID", "FOLDER_PATH", "ORG", "ORG_ID"]
    if summaries and "sources" in summaries[0]:
        headers.append("SOURCES")
    rows = []
    for record in [build_dashboard_summary_record(summary) for summary in summaries]:
        row = [
            record["uid"],
            record["name"],
            record["folder"],
            record["folderUid"],
            record["path"],
            record["org"],
            record["orgId"],
        ]
        if "sources" in record:
            row.append(record["sources"])
        rows.append(row)
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: List[str]) -> str:
        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.extend([format_row(headers), format_row(["-" * width for width in widths])])
    lines.extend(format_row(row) for row in rows)
    return lines


def render_dashboard_summary_csv(summaries: List[Dict[str, Any]]) -> None:
    """Render dashboard summaries as CSV records."""
    fieldnames = ["uid", "name", "folder", "folderUid", "path", "org", "orgId"]
    if summaries and "sources" in summaries[0]:
        fieldnames.append("sources")
    if summaries and "sourceUids" in summaries[0]:
        fieldnames.append("sourceUids")
    writer = csv.DictWriter(sys.stdout, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    for summary in summaries:
        writer.writerow(build_dashboard_summary_record(summary))


def render_dashboard_summary_json(summaries: List[Dict[str, Any]]) -> str:
    """Render dashboard summaries as JSON."""
    records = []
    for summary in summaries:
        record = build_dashboard_summary_record(summary)
        if "sources" in summary:
            record["sources"] = list(summary.get("sources") or [])
        if "sourceUids" in summary:
            record["sourceUids"] = list(summary.get("sourceUids") or [])
        records.append(record)
    return json.dumps(records, indent=2, sort_keys=False)


def list_dashboards(args: argparse.Namespace) -> int:
    """List live dashboard summaries without exporting dashboard JSON."""
    all_orgs = bool(getattr(args, "all_orgs", False))
    org_id = getattr(args, "org_id", None)
    if all_orgs and org_id:
        raise GrafanaError("Choose either --org-id or --all-orgs, not both.")
    client = build_client(args)
    auth_header = client.headers.get("Authorization", "")
    if (all_orgs or org_id) and not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Dashboard org switching requires Basic auth. Use --basic-user and --basic-password."
        )

    clients: List[GrafanaClient]
    if all_orgs:
        orgs = client.list_orgs()
        clients = []
        for org in orgs:
            org_id = str(org.get("id") or "").strip()
            if org_id:
                clients.append(client.with_org_id(org_id))
    elif org_id:
        clients = [client.with_org_id(str(org_id))]
    else:
        clients = [client]

    summaries: List[Dict[str, Any]] = []
    for scoped_client in clients:
        scoped_summaries = attach_dashboard_folder_paths(
            scoped_client,
            scoped_client.iter_dashboard_summaries(args.page_size),
        )
        scoped_summaries = attach_dashboard_org(scoped_client, scoped_summaries)
        if args.json or getattr(args, "with_sources", False):
            scoped_summaries = attach_dashboard_sources(scoped_client, scoped_summaries)
        summaries.extend(scoped_summaries)
    if args.csv:
        render_dashboard_summary_csv(summaries)
        return 0
    if args.json:
        print(render_dashboard_summary_json(summaries))
        return 0
    for line in render_dashboard_summary_table(
        summaries,
        include_header=not bool(getattr(args, "no_header", False)),
    ):
        print(line)
    print("")
    print(f"Listed {len(summaries)} dashboard summaries from {args.url}")
    return 0


def format_data_source_line(datasource: Dict[str, Any]) -> str:
    record = build_data_source_record(datasource)
    return (
        f"uid={record['uid']} name={record['name']} type={record['type']} "
        f"url={record['url']} isDefault={record['isDefault']}"
    )


def build_data_source_record(datasource: Dict[str, Any]) -> Dict[str, str]:
    return {
        "uid": str(datasource.get("uid") or ""),
        "name": str(datasource.get("name") or ""),
        "type": str(datasource.get("type") or ""),
        "url": str(datasource.get("url") or ""),
        "isDefault": "true" if bool(datasource.get("isDefault")) else "false",
    }


def build_datasource_inventory_record(
    datasource: Dict[str, Any],
    org: Dict[str, Any],
) -> Dict[str, str]:
    record = build_data_source_record(datasource)
    record["access"] = str(datasource.get("access") or "")
    record["org"] = str(org.get("name") or DEFAULT_ORG_NAME)
    record["orgId"] = str(org.get("id") or DEFAULT_ORG_ID)
    return record


def render_data_source_table(
    datasources: List[Dict[str, Any]],
    include_header: bool = True,
) -> List[str]:
    headers = ["UID", "NAME", "TYPE", "URL", "IS_DEFAULT"]
    rows = []
    for record in [build_data_source_record(item) for item in datasources]:
        rows.append(
            [
                record["uid"],
                record["name"],
                record["type"],
                record["url"],
                record["isDefault"],
            ]
        )
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: List[str]) -> str:
        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.extend([format_row(headers), format_row(["-" * width for width in widths])])
    lines.extend(format_row(row) for row in rows)
    return lines


def build_dashboard_import_dry_run_record(
    dashboard_file: Path,
    uid: str,
    action: str,
    folder_path: Optional[str] = None,
) -> Dict[str, str]:
    destination = "unknown"
    action_label = action or "unknown"
    if action == "would-create":
        destination = "missing"
        action_label = "create"
    elif action == "would-skip-missing":
        destination = "missing"
        action_label = "skip-missing"
    elif action == "would-update":
        destination = "exists"
        action_label = "update"
    elif action == "would-fail-existing":
        destination = "exists"
        action_label = "blocked-existing"
    return {
        "uid": uid,
        "destination": destination,
        "action": action_label,
        "folderPath": str(folder_path or ""),
        "file": str(dashboard_file),
    }


def render_dashboard_import_dry_run_table(
    records: List[Dict[str, str]],
    include_header: bool = True,
) -> List[str]:
    include_folder = any(record.get("folderPath") for record in records)
    headers = ["UID", "DESTINATION", "ACTION"]
    if include_folder:
        headers.append("FOLDER_PATH")
    headers.append("FILE")
    rows = []
    for record in records:
        row = [record["uid"], record["destination"], record["action"]]
        if include_folder:
            row.append(record.get("folderPath") or "")
        row.append(record["file"])
        rows.append(row)
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: List[str]) -> str:
        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.extend([format_row(headers), format_row(["-" * width for width in widths])])
    lines.extend(format_row(row) for row in rows)
    return lines


def render_dashboard_import_dry_run_json(
    mode: str,
    folder_records: List[Dict[str, str]],
    dashboard_records: List[Dict[str, str]],
    import_dir: Path,
    skipped_missing_count: int,
) -> str:
    """Render one JSON document for dry-run import output."""
    payload = {
        "mode": mode,
        "folders": [
            {
                "uid": record.get("uid") or "",
                "destination": record.get("destination") or "",
                "status": record.get("status") or "",
                "reason": record.get("reason") or "",
                "expectedPath": record.get("expected_path") or "",
                "actualPath": record.get("actual_path") or "",
            }
            for record in folder_records
        ],
        "dashboards": [
            {
                "uid": record.get("uid") or "",
                "destination": record.get("destination") or "",
                "action": record.get("action") or "",
                "folderPath": record.get("folderPath") or "",
                "file": record.get("file") or "",
            }
            for record in dashboard_records
        ],
        "summary": {
            "importDir": str(import_dir),
            "folderCount": len(folder_records),
            "missingFolders": len(
                [record for record in folder_records if record.get("status") == "missing"]
            ),
            "mismatchedFolders": len(
                [record for record in folder_records if record.get("status") == "mismatch"]
            ),
            "dashboardCount": len(dashboard_records),
            "missingDashboards": len(
                [record for record in dashboard_records if record.get("destination") == "missing"]
            ),
            "skippedMissingDashboards": skipped_missing_count,
        },
    }
    return json.dumps(payload, indent=2, sort_keys=False, ensure_ascii=False)


def render_folder_inventory_dry_run_table(
    records: List[Dict[str, str]],
    include_header: bool = True,
) -> List[str]:
    headers = ["UID", "DESTINATION", "STATUS", "REASON", "EXPECTED_PATH", "ACTUAL_PATH"]
    rows = []
    for record in records:
        rows.append(
            [
                record["uid"],
                record["destination"],
                record["status"],
                record["reason"],
                record["expected_path"],
                record["actual_path"],
            ]
        )
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: List[str]) -> str:
        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.extend([format_row(headers), format_row(["-" * width for width in widths])])
    lines.extend(format_row(row) for row in rows)
    return lines


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


def list_data_sources(args: argparse.Namespace) -> int:
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


def iter_dashboard_panels(panels: Any) -> List[Dict[str, Any]]:
    """Flatten Grafana panels, including nested row/library panel layouts."""
    flattened = []
    if not isinstance(panels, list):
        return flattened
    for panel in panels:
        if not isinstance(panel, dict):
            continue
        flattened.append(panel)
        nested_panels = panel.get("panels")
        if isinstance(nested_panels, list):
            flattened.extend(iter_dashboard_panels(nested_panels))
    return flattened


def describe_export_datasource_ref(ref: Any) -> Optional[str]:
    """Convert one raw-export datasource ref into a readable usage label."""
    if ref is None or is_builtin_datasource_ref(ref):
        return None
    if isinstance(ref, str):
        return ref
    if isinstance(ref, dict):
        for key in ("name", "uid", "type"):
            value = ref.get(key)
            if isinstance(value, str) and value:
                return value
    return None


def summarize_datasource_inventory_usage(
    datasource: Dict[str, str],
    usage_by_label: Dict[str, Dict[str, Any]],
) -> Dict[str, int]:
    labels = []
    uid = str(datasource.get("uid") or "").strip()
    name = str(datasource.get("name") or "").strip()
    if uid:
        labels.append(uid)
    if name and name not in labels:
        labels.append(name)
    reference_count = 0
    dashboards = set()
    for label in labels:
        usage = usage_by_label.get(label) or {}
        reference_count += int(usage.get("referenceCount") or 0)
        dashboards.update(usage.get("dashboards") or set())
    return {
        "referenceCount": reference_count,
        "dashboardCount": len(dashboards),
    }


def extract_string_values(value: Any) -> List[str]:
    values = []
    if isinstance(value, str):
        normalized = value.strip()
        if normalized:
            values.append(normalized)
    elif isinstance(value, list):
        for item in value:
            values.extend(extract_string_values(item))
    elif isinstance(value, dict):
        for item in value.values():
            values.extend(extract_string_values(item))
    return values


def unique_strings(values: List[str]) -> List[str]:
    result = []
    seen = set()
    for value in values:
        normalized = str(value).strip()
        if not normalized or normalized in seen:
            continue
        seen.add(normalized)
        result.append(normalized)
    return result


def describe_panel_datasource(panel: Dict[str, Any], target: Dict[str, Any]) -> str:
    for ref in (target.get("datasource"), panel.get("datasource")):
        label = describe_export_datasource_ref(ref)
        if label:
            return label
    return ""


def describe_panel_datasource_uid(panel: Dict[str, Any], target: Dict[str, Any]) -> str:
    for ref in (target.get("datasource"), panel.get("datasource")):
        if isinstance(ref, dict):
            value = ref.get("uid")
            if isinstance(value, str) and value.strip():
                return value.strip()
        elif isinstance(ref, str):
            value = ref.strip()
            if value and not is_builtin_datasource_ref(value):
                return value
    return ""


def build_query_field_and_text(target: Dict[str, Any]) -> Tuple[str, str]:
    for key in (
        "expr",
        "expression",
        "query",
        "rawSql",
        "rawQuery",
        "sql",
        "queryText",
        "jql",
    ):
        value = target.get(key)
        if isinstance(value, str) and value.strip():
            return key, value.strip()
    return "", ""


def extract_metric_names(query_text: str) -> List[str]:
    names = []
    names.extend(
        re.findall(r'__name__\s*=~?\s*"([^"]+)"', query_text)
    )
    scrubbed = re.sub(r'"[^"]*"', '""', query_text)
    for token in re.findall(r"\b([A-Za-z_:][A-Za-z0-9_:]*)\b", scrubbed):
        if token in {
            "and",
            "bool",
            "by",
            "group_left",
            "group_right",
            "ignoring",
            "offset",
            "on",
            "or",
            "unless",
            "without",
        }:
            continue
        if re.search(r"\b%s\s*=~?" % re.escape(token), scrubbed):
            continue
        if re.search(r"\b%s\s*\(" % re.escape(token), query_text):
            continue
        names.append(token)
    return unique_strings(names)


def extract_measurements(query_text: str, target: Dict[str, Any]) -> List[str]:
    measurements = []
    for key in ("measurement", "measurementName"):
        value = target.get(key)
        if isinstance(value, str) and value.strip():
            measurements.append(value.strip())
    measurements.extend(
        re.findall(r'FROM\s+"?([A-Za-z0-9_.:-]+)"?', query_text, flags=re.IGNORECASE)
    )
    measurements.extend(
        re.findall(r'_measurement\s*==\s*"([^"]+)"', query_text)
    )
    return unique_strings(measurements)


def extract_buckets(query_text: str, target: Dict[str, Any]) -> List[str]:
    buckets = []
    buckets.extend(
        re.findall(r'from\s*\(\s*bucket\s*:\s*"([^"]+)"', query_text, flags=re.IGNORECASE)
    )
    buckets.extend(extract_string_values(target.get("bucketAggs")))
    return unique_strings(buckets)


def build_query_report_record(
    dashboard: Dict[str, Any],
    folder_path: str,
    panel: Dict[str, Any],
    target: Dict[str, Any],
    dashboard_file: Path,
) -> Dict[str, Any]:
    query_field, query_text = build_query_field_and_text(target)
    metrics = extract_metric_names(query_text)
    measurements = extract_measurements(query_text, target)
    buckets = extract_buckets(query_text, target)
    return {
        "dashboardUid": str(dashboard.get("uid") or DEFAULT_UNKNOWN_UID),
        "dashboardTitle": str(dashboard.get("title") or DEFAULT_DASHBOARD_TITLE),
        "folderPath": folder_path,
        "panelId": str(panel.get("id") or ""),
        "panelTitle": str(panel.get("title") or ""),
        "panelType": str(panel.get("type") or ""),
        "refId": str(target.get("refId") or ""),
        "datasourceUid": describe_panel_datasource_uid(panel, target),
        "datasource": describe_panel_datasource(panel, target),
        "queryField": query_field,
        "query": query_text,
        "metrics": metrics,
        "measurements": measurements,
        "buckets": buckets,
        "file": str(dashboard_file),
    }


def build_export_inspection_report_document(import_dir: Path) -> Dict[str, Any]:
    """Analyze one raw export directory and emit one per-query inspection record."""
    metadata = load_export_metadata(import_dir, expected_variant=RAW_EXPORT_SUBDIR)
    dashboard_files = discover_dashboard_files(import_dir)
    folder_inventory = load_folder_inventory(import_dir, metadata)
    folder_lookup = build_folder_inventory_lookup(folder_inventory)
    records = []

    for dashboard_file in dashboard_files:
        document = load_json_file(dashboard_file)
        dashboard = extract_dashboard_object(
            document, "Dashboard payload must be a JSON object."
        )
        folder_record = resolve_folder_inventory_record_for_dashboard(
            document,
            dashboard_file,
            import_dir,
            folder_lookup,
        )
        folder_path = str(
            (folder_record or {}).get("path")
            or (folder_record or {}).get("title")
            or DEFAULT_FOLDER_TITLE
        ).strip() or DEFAULT_FOLDER_TITLE
        for panel in iter_dashboard_panels(dashboard.get("panels")):
            targets = panel.get("targets")
            if not isinstance(targets, list):
                continue
            for target in targets:
                if not isinstance(target, dict):
                    continue
                records.append(
                    build_query_report_record(
                        dashboard,
                        folder_path,
                        panel,
                        target,
                        dashboard_file,
                    )
                )

    records.sort(
        key=lambda item: (
            item["folderPath"],
            item["dashboardTitle"],
            item["dashboardUid"],
            item["panelId"],
            item["refId"],
        )
    )
    return {
        "summary": {
            "dashboardCount": len(
                set(record["dashboardUid"] for record in records)
            ),
            "queryRecordCount": len(records),
        },
        "queries": records,
    }


def parse_report_columns(value: Optional[str]) -> Optional[List[str]]:
    if value is None:
        return None
    columns = []
    for item in value.split(","):
        column = item.strip()
        if column:
            columns.append(REPORT_COLUMN_ALIASES.get(column, column))
    if not columns:
        raise GrafanaError(
            "--report-columns requires one or more comma-separated column ids."
        )
    unknown = [
        column for column in columns if column not in SUPPORTED_REPORT_COLUMN_HEADERS
    ]
    if unknown:
        raise GrafanaError(
            "Unsupported report column(s): %s. Supported values: %s."
            % (
                ", ".join(unknown),
                ", ".join(
                    list(REPORT_COLUMN_ALIASES.keys())
                    + [
                        "datasourceUid",
                        "datasource",
                        "metrics",
                        "measurements",
                        "buckets",
                        "query",
                        "file",
                    ]
                ),
            )
        )
    return columns


def filter_export_inspection_report_document(
    document: Dict[str, Any],
    datasource_label: Optional[str] = None,
    panel_id: Optional[str] = None,
) -> Dict[str, Any]:
    if not datasource_label and not panel_id:
        return document
    filtered_records = [
        dict(record)
        for record in list(document.get("queries") or [])
        if (
            (not datasource_label or str(record.get("datasource") or "") == datasource_label)
            and (not panel_id or str(record.get("panelId") or "") == panel_id)
        )
    ]
    return {
        "summary": {
            "dashboardCount": len(
                set(str(record.get("dashboardUid") or "") for record in filtered_records)
            ),
            "queryRecordCount": len(filtered_records),
        },
        "queries": filtered_records,
    }


def format_report_column_value(record: Dict[str, Any], column_id: str) -> str:
    value = record.get(column_id)
    if isinstance(value, list):
        return ",".join(str(item) for item in value)
    return str(value or "")


def _build_inspection_workflow_deps() -> Dict[str, Any]:
    return {
        "GrafanaError": GrafanaError,
        "DATASOURCE_INVENTORY_FILENAME": DATASOURCE_INVENTORY_FILENAME,
        "EXPORT_METADATA_FILENAME": EXPORT_METADATA_FILENAME,
        "FOLDER_INVENTORY_FILENAME": FOLDER_INVENTORY_FILENAME,
        "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
        "attach_dashboard_org": attach_dashboard_org,
        "build_client": build_client,
        "build_dashboard_index_item": build_dashboard_index_item,
        "build_datasource_inventory_record": build_datasource_inventory_record,
        "build_export_inspection_document": build_export_inspection_document,
        "build_grouped_export_inspection_report_document": build_grouped_export_inspection_report_document,
        "build_export_inspection_report_document": build_export_inspection_report_document,
        "build_export_metadata": build_export_metadata,
        "build_output_path": build_output_path,
        "build_preserved_web_import_document": build_preserved_web_import_document,
        "build_variant_index": build_variant_index,
        "collect_folder_inventory": collect_folder_inventory,
        "filter_export_inspection_report_document": filter_export_inspection_report_document,
        "inspect_export": inspect_export,
        "json": json,
        "parse_report_columns": parse_report_columns,
        "render_export_inspection_grouped_report": render_export_inspection_grouped_report,
        "render_export_inspection_report_csv": render_export_inspection_report_csv,
        "render_export_inspection_report_tables": render_export_inspection_report_tables,
        "render_export_inspection_summary": render_export_inspection_summary,
        "render_export_inspection_tree_tables": render_export_inspection_tree_tables,
        "render_export_inspection_tables": render_export_inspection_tables,
        "sys": sys,
        "tempfile": tempfile,
        "write_dashboard": write_dashboard,
        "write_json_document": write_json_document,
    }


def materialize_live_inspection_export(
    client: "GrafanaClient",
    page_size: int,
    raw_dir: Path,
) -> Path:
    """Write one temporary raw-export-like directory for live dashboard inspection."""
    return run_materialize_live_inspection_export(
        client,
        page_size,
        raw_dir,
        _build_inspection_workflow_deps(),
    )


def inspect_live(args: argparse.Namespace) -> int:
    """Inspect live Grafana dashboards by reusing the raw-export inspection pipeline."""
    return run_inspect_live(args, _build_inspection_workflow_deps())


def render_export_inspection_report_csv(
    document: Dict[str, Any],
    selected_columns: Optional[List[str]] = None,
    include_header: bool = True,
) -> str:
    """Render one full per-query inspection report as CSV."""
    selected_columns = list(selected_columns or REPORT_COLUMN_HEADERS.keys())
    rows = []
    if include_header:
        rows.append(
            [
                REPORT_COLUMN_ALIASES.get(
                    column_id,
                    re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", column_id).lower(),
                )
                for column_id in selected_columns
            ]
        )
    for record in list(document.get("queries") or []):
        rows.append(
            [
                format_report_column_value(record, column_id)
                for column_id in selected_columns
            ]
        )
    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerows(rows)
    return output.getvalue()


def build_export_inspection_document(import_dir: Path) -> Dict[str, Any]:
    """Analyze one raw export directory and summarize dashboard structure."""
    metadata = load_export_metadata(import_dir, expected_variant=RAW_EXPORT_SUBDIR)
    dashboard_files = discover_dashboard_files(import_dir)
    folder_inventory = load_folder_inventory(import_dir, metadata)
    datasource_inventory = load_datasource_inventory(import_dir, metadata)
    folder_lookup = build_folder_inventory_lookup(folder_inventory)
    folder_paths = OrderedDict()
    datasource_usage: Dict[str, Dict[str, Any]] = {}
    dashboards = []
    total_panels = 0
    total_queries = 0
    mixed_dashboards = []

    for folder in sorted(folder_inventory, key=lambda item: str(item.get("path") or "")):
        path = str(folder.get("path") or str(folder.get("title") or "")).strip()
        if path:
            folder_paths[path] = 0

    for dashboard_file in dashboard_files:
        document = load_json_file(dashboard_file)
        dashboard = extract_dashboard_object(
            document, "Dashboard payload must be a JSON object."
        )
        folder_record = resolve_folder_inventory_record_for_dashboard(
            document,
            dashboard_file,
            import_dir,
            folder_lookup,
        )
        folder_path = str(
            folder_record.get("path")
            or folder_record.get("title")
            or DEFAULT_FOLDER_TITLE
        ).strip() or DEFAULT_FOLDER_TITLE
        folder_paths[folder_path] = int(folder_paths.get(folder_path) or 0) + 1

        panels = iter_dashboard_panels(dashboard.get("panels"))
        panel_count = len(panels)
        query_count = 0
        datasource_refs = []
        collect_datasource_refs(dashboard, datasource_refs)
        datasource_labels = []
        for ref in datasource_refs:
            label = describe_export_datasource_ref(ref)
            if label:
                datasource_labels.append(label)
        unique_datasources = sorted(set(datasource_labels))
        is_mixed = False
        for panel in panels:
            targets = panel.get("targets")
            if isinstance(targets, list):
                query_count += len([target for target in targets if isinstance(target, dict)])
            panel_datasource = panel.get("datasource")
            if isinstance(panel_datasource, dict) and str(panel_datasource.get("uid") or "") == "-- Mixed --":
                is_mixed = True
        if len(unique_datasources) > 1:
            is_mixed = True

        for label in datasource_labels:
            usage = datasource_usage.setdefault(
                label,
                {"name": label, "referenceCount": 0, "dashboards": set()},
            )
            usage["referenceCount"] = int(usage.get("referenceCount") or 0) + 1
            usage["dashboards"].add(str(dashboard.get("uid") or DEFAULT_UNKNOWN_UID))

        total_panels += panel_count
        total_queries += query_count
        dashboard_record = {
            "uid": str(dashboard.get("uid") or DEFAULT_UNKNOWN_UID),
            "title": str(dashboard.get("title") or DEFAULT_DASHBOARD_TITLE),
            "folderPath": folder_path,
            "panelCount": panel_count,
            "queryCount": query_count,
            "datasources": unique_datasources,
            "mixedDatasource": is_mixed,
            "file": str(dashboard_file),
        }
        dashboards.append(dashboard_record)
        if is_mixed:
            mixed_dashboards.append(
                {
                    "uid": dashboard_record["uid"],
                    "title": dashboard_record["title"],
                    "folderPath": folder_path,
                    "datasources": unique_datasources,
                }
            )

    datasource_records = []
    for label in sorted(datasource_usage):
        usage = datasource_usage[label]
        datasource_records.append(
            {
                "name": label,
                "referenceCount": int(usage.get("referenceCount") or 0),
                "dashboardCount": len(usage.get("dashboards") or []),
            }
        )

    datasource_inventory_records = []
    for datasource in sorted(
        datasource_inventory,
        key=lambda item: (
            str(item.get("orgId") or ""),
            str(item.get("name") or ""),
            str(item.get("uid") or ""),
        ),
    ):
        usage = summarize_datasource_inventory_usage(datasource, datasource_usage)
        datasource_inventory_records.append(
            {
                "uid": str(datasource.get("uid") or ""),
                "name": str(datasource.get("name") or ""),
                "type": str(datasource.get("type") or ""),
                "access": str(datasource.get("access") or ""),
                "url": str(datasource.get("url") or ""),
                "isDefault": str(datasource.get("isDefault") or "false"),
                "org": str(datasource.get("org") or ""),
                "orgId": str(datasource.get("orgId") or ""),
                "referenceCount": usage["referenceCount"],
                "dashboardCount": usage["dashboardCount"],
            }
        )

    folder_records = [
        {"path": path, "dashboardCount": count}
        for path, count in folder_paths.items()
    ]
    dashboards.sort(key=lambda item: (item["folderPath"], item["title"], item["uid"]))
    mixed_dashboards.sort(key=lambda item: (item["folderPath"], item["title"], item["uid"]))
    return {
        "summary": {
            "dashboardCount": len(dashboards),
            "folderCount": len(folder_records),
            "panelCount": total_panels,
            "queryCount": total_queries,
            "mixedDatasourceDashboardCount": len(mixed_dashboards),
            "datasourceInventoryCount": len(datasource_inventory_records),
        },
        "folders": folder_records,
        "datasources": datasource_records,
        "datasourceInventory": datasource_inventory_records,
        "mixedDatasourceDashboards": mixed_dashboards,
        "dashboards": dashboards,
    }


def render_export_inspection_summary(document: Dict[str, Any], import_dir: Path) -> List[str]:
    """Render a compact human-readable export inspection summary."""
    summary = document.get("summary") or {}
    folder_records = list(document.get("folders") or [])
    datasource_records = list(document.get("datasources") or [])
    datasource_inventory = list(document.get("datasourceInventory") or [])
    mixed_dashboards = list(document.get("mixedDatasourceDashboards") or [])
    lines = [
        "Export inspection: %s" % import_dir,
        "Dashboards: %s" % int(summary.get("dashboardCount") or 0),
        "Folders: %s" % int(summary.get("folderCount") or 0),
        "Panels: %s" % int(summary.get("panelCount") or 0),
        "Queries: %s" % int(summary.get("queryCount") or 0),
        "Datasource inventory: %s"
        % int(summary.get("datasourceInventoryCount") or 0),
        "Mixed datasource dashboards: %s"
        % int(summary.get("mixedDatasourceDashboardCount") or 0),
    ]
    if folder_records:
        lines.append("")
        lines.append("Folder paths:")
        for record in folder_records:
            lines.append(
                "- %s (%s dashboards)"
                % (
                    str(record.get("path") or DEFAULT_FOLDER_TITLE),
                    int(record.get("dashboardCount") or 0),
                )
            )
    if datasource_records:
        lines.append("")
        lines.append("Datasource usage:")
        for record in datasource_records:
            lines.append(
                "- %s (%s refs across %s dashboards)"
                % (
                    str(record.get("name") or ""),
                    int(record.get("referenceCount") or 0),
                    int(record.get("dashboardCount") or 0),
                )
            )
    if datasource_inventory:
        lines.append("")
        lines.append("Datasource inventory:")
        for record in datasource_inventory:
            lines.append(
                "- [%s] %s uid=%s type=%s access=%s url=%s isDefault=%s refs=%s dashboards=%s"
                % (
                    str(record.get("orgId") or ""),
                    str(record.get("name") or ""),
                    str(record.get("uid") or ""),
                    str(record.get("type") or ""),
                    str(record.get("access") or ""),
                    str(record.get("url") or ""),
                    str(record.get("isDefault") or "false"),
                    int(record.get("referenceCount") or 0),
                    int(record.get("dashboardCount") or 0),
                )
            )
    if mixed_dashboards:
        lines.append("")
        lines.append("Mixed datasource dashboards:")
        for record in mixed_dashboards:
            lines.append(
                "- %s (%s) path=%s datasources=%s"
                % (
                    str(record.get("title") or ""),
                    str(record.get("uid") or ""),
                    str(record.get("folderPath") or DEFAULT_FOLDER_TITLE),
                    ",".join(record.get("datasources") or []),
                )
            )
    return lines


def render_export_inspection_table_section(
    headers: List[str],
    rows: List[List[str]],
    include_header: bool = True,
) -> List[str]:
    """Render one simple left-aligned table section."""
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: List[str]) -> str:
        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = []
    if include_header:
        lines.append(format_row(headers))
        lines.append(format_row(["-" * width for width in widths]))
    lines.extend(format_row(row) for row in rows)
    return lines


def render_export_inspection_tables(
    document: Dict[str, Any],
    import_dir: Path,
    include_header: bool = True,
) -> List[str]:
    """Render export inspection as multiple compact table sections."""
    summary = document.get("summary") or {}
    folder_records = list(document.get("folders") or [])
    datasource_records = list(document.get("datasources") or [])
    datasource_inventory = list(document.get("datasourceInventory") or [])
    mixed_dashboards = list(document.get("mixedDatasourceDashboards") or [])
    lines = ["Export inspection: %s" % import_dir, ""]

    lines.append("# Summary")
    lines.extend(
        render_export_inspection_table_section(
            ["METRIC", "VALUE"],
            [
                ["dashboard_count", str(int(summary.get("dashboardCount") or 0))],
                ["folder_count", str(int(summary.get("folderCount") or 0))],
                ["panel_count", str(int(summary.get("panelCount") or 0))],
                ["query_count", str(int(summary.get("queryCount") or 0))],
                [
                    "datasource_inventory_count",
                    str(int(summary.get("datasourceInventoryCount") or 0)),
                ],
                [
                    "mixed_datasource_dashboard_count",
                    str(int(summary.get("mixedDatasourceDashboardCount") or 0)),
                ],
            ],
            include_header=include_header,
        )
    )

    if folder_records:
        lines.append("")
        lines.append("# Folder paths")
        lines.extend(
            render_export_inspection_table_section(
                ["FOLDER_PATH", "DASHBOARDS"],
                [
                    [
                        str(record.get("path") or DEFAULT_FOLDER_TITLE),
                        str(int(record.get("dashboardCount") or 0)),
                    ]
                    for record in folder_records
                ],
                include_header=include_header,
            )
        )

    if datasource_records:
        lines.append("")
        lines.append("# Datasource usage")
        lines.extend(
            render_export_inspection_table_section(
                ["DATASOURCE", "REFS", "DASHBOARDS"],
                [
                    [
                        str(record.get("name") or ""),
                        str(int(record.get("referenceCount") or 0)),
                        str(int(record.get("dashboardCount") or 0)),
                    ]
                    for record in datasource_records
                ],
                include_header=include_header,
            )
        )

    if datasource_inventory:
        lines.append("")
        lines.append("# Datasource inventory")
        lines.extend(
            render_export_inspection_table_section(
                [
                    "ORG_ID",
                    "UID",
                    "NAME",
                    "TYPE",
                    "ACCESS",
                    "URL",
                    "IS_DEFAULT",
                    "REFS",
                    "DASHBOARDS",
                ],
                [
                    [
                        str(record.get("orgId") or ""),
                        str(record.get("uid") or ""),
                        str(record.get("name") or ""),
                        str(record.get("type") or ""),
                        str(record.get("access") or ""),
                        str(record.get("url") or ""),
                        str(record.get("isDefault") or "false"),
                        str(int(record.get("referenceCount") or 0)),
                        str(int(record.get("dashboardCount") or 0)),
                    ]
                    for record in datasource_inventory
                ],
                include_header=include_header,
            )
        )

    if mixed_dashboards:
        lines.append("")
        lines.append("# Mixed datasource dashboards")
        lines.extend(
            render_export_inspection_table_section(
                ["UID", "TITLE", "FOLDER_PATH", "DATASOURCES"],
                [
                    [
                        str(record.get("uid") or ""),
                        str(record.get("title") or ""),
                        str(record.get("folderPath") or DEFAULT_FOLDER_TITLE),
                        ",".join(record.get("datasources") or []),
                    ]
                    for record in mixed_dashboards
                ],
                include_header=include_header,
            )
        )
    return lines


def render_export_inspection_report_tables(
    document: Dict[str, Any],
    import_dir: Path,
    include_header: bool = True,
    selected_columns: Optional[List[str]] = None,
) -> List[str]:
    """Render one full per-query inspection report as a table."""
    summary = document.get("summary") or {}
    query_records = list(document.get("queries") or [])
    selected_columns = list(selected_columns or REPORT_COLUMN_HEADERS.keys())
    lines = ["Export inspection report: %s" % import_dir, ""]

    lines.append("# Summary")
    lines.extend(
        render_export_inspection_table_section(
            ["METRIC", "VALUE"],
            [
                ["dashboard_count", str(int(summary.get("dashboardCount") or 0))],
                ["query_record_count", str(int(summary.get("queryRecordCount") or 0))],
            ],
            include_header=include_header,
        )
    )

    if query_records:
        lines.append("")
        lines.append("# Query report")
        lines.extend(
            render_export_inspection_table_section(
                [
                    SUPPORTED_REPORT_COLUMN_HEADERS[column_id]
                    for column_id in selected_columns
                ],
                [
                    [
                        format_report_column_value(record, column_id)
                        for column_id in selected_columns
                    ]
                    for record in query_records
                ],
                include_header=include_header,
            )
        )
    return lines


def build_grouped_export_inspection_report_document(
    document: Dict[str, Any]
) -> Dict[str, Any]:
    """Group one flat per-query report by dashboard, then by panel."""
    query_records = list(document.get("queries") or [])
    dashboards = OrderedDict()

    for record in query_records:
        dashboard_key = (
            str(record.get("folderPath") or DEFAULT_FOLDER_TITLE),
            str(record.get("dashboardTitle") or DEFAULT_DASHBOARD_TITLE),
            str(record.get("dashboardUid") or DEFAULT_UNKNOWN_UID),
        )
        dashboard_entry = dashboards.get(dashboard_key)
        if dashboard_entry is None:
            dashboard_entry = {
                "dashboardUid": dashboard_key[2],
                "dashboardTitle": dashboard_key[1],
                "folderPath": dashboard_key[0],
                "file": str(record.get("file") or ""),
                "queryCount": 0,
                "panels": OrderedDict(),
            }
            dashboards[dashboard_key] = dashboard_entry
        dashboard_entry["queryCount"] = int(dashboard_entry.get("queryCount") or 0) + 1

        panel_key = (
            str(record.get("panelId") or ""),
            str(record.get("panelTitle") or ""),
            str(record.get("panelType") or ""),
        )
        panel_entry = dashboard_entry["panels"].get(panel_key)
        if panel_entry is None:
            panel_entry = {
                "panelId": panel_key[0],
                "panelTitle": panel_key[1],
                "panelType": panel_key[2],
                "datasources": [],
                "queryCount": 0,
                "queries": [],
            }
            dashboard_entry["panels"][panel_key] = panel_entry
        datasource_label = str(record.get("datasource") or "")
        if datasource_label and datasource_label not in panel_entry["datasources"]:
            panel_entry["datasources"].append(datasource_label)
        panel_entry["queryCount"] = int(panel_entry.get("queryCount") or 0) + 1
        panel_entry["queries"].append(dict(record))

    dashboard_records = []
    panel_count = 0
    for dashboard_entry in dashboards.values():
        panels = []
        for panel_entry in dashboard_entry["panels"].values():
            panel_entry["datasources"].sort()
            panels.append(panel_entry)
        panel_count += len(panels)
        dashboard_records.append(
            {
                "dashboardUid": dashboard_entry["dashboardUid"],
                "dashboardTitle": dashboard_entry["dashboardTitle"],
                "folderPath": dashboard_entry["folderPath"],
                "file": dashboard_entry["file"],
                "panelCount": len(panels),
                "queryCount": int(dashboard_entry.get("queryCount") or 0),
                "panels": panels,
            }
        )

    return {
        "summary": {
            "dashboardCount": len(dashboard_records),
            "panelCount": panel_count,
            "queryRecordCount": len(query_records),
        },
        "dashboards": dashboard_records,
    }


def render_export_inspection_grouped_report(
    document: Dict[str, Any],
    import_dir: Path,
) -> List[str]:
    """Render one per-query inspection report grouped by dashboard and panel."""
    summary = document.get("summary") or {}
    dashboard_records = list(document.get("dashboards") or [])
    lines = ["Export inspection tree report: %s" % import_dir, ""]

    lines.append("# Summary")
    lines.extend(
        render_export_inspection_table_section(
            ["METRIC", "VALUE"],
            [
                ["dashboard_count", str(int(summary.get("dashboardCount") or 0))],
                ["panel_count", str(int(summary.get("panelCount") or 0))],
                ["query_record_count", str(int(summary.get("queryRecordCount") or 0))],
            ],
            include_header=True,
        )
    )

    if dashboard_records:
        lines.append("")
        lines.append("# Dashboard tree")
        for index, dashboard in enumerate(dashboard_records, 1):
            lines.append(
                "[%s] Dashboard %s title=%s path=%s panels=%s queries=%s"
                % (
                    index,
                    str(dashboard.get("dashboardUid") or DEFAULT_UNKNOWN_UID),
                    str(dashboard.get("dashboardTitle") or DEFAULT_DASHBOARD_TITLE),
                    str(dashboard.get("folderPath") or DEFAULT_FOLDER_TITLE),
                    int(dashboard.get("panelCount") or 0),
                    int(dashboard.get("queryCount") or 0),
                )
            )
            for panel in list(dashboard.get("panels") or []):
                datasource_text = ",".join(panel.get("datasources") or []) or "-"
                lines.append(
                    "  Panel %s title=%s type=%s datasources=%s queries=%s"
                    % (
                        str(panel.get("panelId") or ""),
                        str(panel.get("panelTitle") or ""),
                        str(panel.get("panelType") or ""),
                        datasource_text,
                        int(panel.get("queryCount") or 0),
                    )
                )
                for query in list(panel.get("queries") or []):
                    detail_parts = [
                        "datasource=%s" % str(query.get("datasource") or "-"),
                        "field=%s" % str(query.get("queryField") or "-"),
                    ]
                    metrics = format_report_column_value(query, "metrics")
                    measurements = format_report_column_value(query, "measurements")
                    buckets = format_report_column_value(query, "buckets")
                    if metrics:
                        detail_parts.append("metrics=%s" % metrics)
                    if measurements:
                        detail_parts.append("measurements=%s" % measurements)
                    if buckets:
                        detail_parts.append("buckets=%s" % buckets)
                    lines.append(
                        "    Query %s %s"
                        % (
                            str(query.get("refId") or ""),
                            " ".join(detail_parts),
                        )
                    )
                    lines.append("      %s" % str(query.get("query") or ""))
    return lines


def render_export_inspection_tree_tables(
    document: Dict[str, Any],
    import_dir: Path,
    include_header: bool = True,
    selected_columns: Optional[List[str]] = None,
) -> List[str]:
    """Render one grouped report as dashboard-first sections with per-dashboard tables."""
    summary = document.get("summary") or {}
    dashboard_records = list(document.get("dashboards") or [])
    selected_columns = list(selected_columns or REPORT_COLUMN_HEADERS.keys())
    lines = ["Export inspection tree-table report: %s" % import_dir, ""]

    lines.append("# Summary")
    lines.extend(
        render_export_inspection_table_section(
            ["METRIC", "VALUE"],
            [
                ["dashboard_count", str(int(summary.get("dashboardCount") or 0))],
                ["panel_count", str(int(summary.get("panelCount") or 0))],
                ["query_record_count", str(int(summary.get("queryRecordCount") or 0))],
            ],
            include_header=include_header,
        )
    )

    if dashboard_records:
        lines.append("")
        lines.append("# Dashboard sections")
        for index, dashboard in enumerate(dashboard_records, 1):
            lines.append(
                "[%s] Dashboard %s title=%s path=%s panels=%s queries=%s"
                % (
                    index,
                    str(dashboard.get("dashboardUid") or DEFAULT_UNKNOWN_UID),
                    str(dashboard.get("dashboardTitle") or DEFAULT_DASHBOARD_TITLE),
                    str(dashboard.get("folderPath") or DEFAULT_FOLDER_TITLE),
                    int(dashboard.get("panelCount") or 0),
                    int(dashboard.get("queryCount") or 0),
                )
            )
            query_records = []
            for panel in list(dashboard.get("panels") or []):
                for query in list(panel.get("queries") or []):
                    query_records.append(query)
            if query_records:
                lines.extend(
                    render_export_inspection_table_section(
                        [
                            SUPPORTED_REPORT_COLUMN_HEADERS[column_id]
                            for column_id in selected_columns
                        ],
                        [
                            [
                                format_report_column_value(record, column_id)
                                for column_id in selected_columns
                            ]
                            for record in query_records
                        ],
                        include_header=include_header,
                    )
                )
            else:
                lines.append("(no query rows)")
            lines.append("")
        if lines[-1] == "":
            lines.pop()
    return lines


def inspect_export(args: argparse.Namespace) -> int:
    """Inspect one raw export directory and summarize dashboards, folders, and datasources."""
    return run_inspect_export(args, _build_inspection_workflow_deps())


def _build_import_workflow_deps() -> Dict[str, Any]:
    return {
        "DEFAULT_UNKNOWN_UID": DEFAULT_UNKNOWN_UID,
        "FOLDER_INVENTORY_FILENAME": FOLDER_INVENTORY_FILENAME,
        "GrafanaError": GrafanaError,
        "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
        "build_client": build_client,
        "build_dashboard_import_dry_run_record": build_dashboard_import_dry_run_record,
        "build_folder_inventory_lookup": build_folder_inventory_lookup,
        "build_import_payload": build_import_payload,
        "describe_dashboard_import_mode": describe_dashboard_import_mode,
        "determine_dashboard_import_action": determine_dashboard_import_action,
        "determine_import_folder_uid_override": determine_import_folder_uid_override,
        "discover_dashboard_files": discover_dashboard_files,
        "ensure_folder_inventory": ensure_folder_inventory,
        "extract_dashboard_object": extract_dashboard_object,
        "inspect_folder_inventory": inspect_folder_inventory,
        "load_export_metadata": load_export_metadata,
        "load_json_file": load_json_file,
        "print_dashboard_import_progress": print_dashboard_import_progress,
        "render_dashboard_import_dry_run_json": render_dashboard_import_dry_run_json,
        "render_dashboard_import_dry_run_table": render_dashboard_import_dry_run_table,
        "render_folder_inventory_dry_run_table": render_folder_inventory_dry_run_table,
        "resolve_dashboard_import_folder_path": resolve_dashboard_import_folder_path,
        "resolve_folder_inventory_requirements": resolve_folder_inventory_requirements,
    }


def import_dashboards(args: argparse.Namespace) -> int:
    """Import previously exported raw dashboard JSON files through Grafana's API."""
    return run_import_dashboards(args, _build_import_workflow_deps())


def diff_dashboards(args: argparse.Namespace) -> int:
    """Compare local raw dashboard exports with the current Grafana state."""
    client = build_client(args)
    import_dir = Path(args.import_dir)
    load_export_metadata(import_dir, expected_variant=RAW_EXPORT_SUBDIR)
    dashboard_files = discover_dashboard_files(import_dir)
    differences = 0

    for dashboard_file in dashboard_files:
        document = load_json_file(dashboard_file)
        uid = resolve_dashboard_uid_for_import(document)
        local_compare = build_local_compare_document(
            document,
            args.import_folder_uid,
        )
        remote_payload = client.fetch_dashboard_if_exists(uid)
        if remote_payload is None:
            print(f"Diff missing-remote {dashboard_file} -> uid={uid}")
            differences += 1
            continue

        remote_compare = build_remote_compare_document(
            remote_payload,
            args.import_folder_uid,
        )
        if serialize_compare_document(local_compare) == serialize_compare_document(
            remote_compare
        ):
            print(f"Diff same {dashboard_file} -> uid={uid}")
            continue

        print(f"Diff different {dashboard_file} -> uid={uid}")
        print(
            "\n".join(
                build_compare_diff_lines(
                    remote_compare,
                    local_compare,
                    uid,
                    dashboard_file,
                    args.context_lines,
                )
            )
        )
        differences += 1

    if differences:
        print(
            f"Found {differences} dashboard differences across {len(dashboard_files)} files."
        )
        return 1

    print(f"No dashboard differences across {len(dashboard_files)} files.")
    return 0


def build_client(args: argparse.Namespace) -> GrafanaClient:
    """Build the dashboard API client from parsed CLI arguments."""
    headers = resolve_auth(args)
    return GrafanaClient(
        base_url=args.url,
        headers=headers,
        timeout=args.timeout,
        verify_ssl=args.verify_ssl,
    )


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    try:
        if args.command == "list-dashboard":
            return list_dashboards(args)
        if args.command == "list-data-sources":
            return list_data_sources(args)
        if args.command == "inspect-export":
            return inspect_export(args)
        if args.command == "inspect-live":
            return inspect_live(args)
        if args.command == "import-dashboard":
            return import_dashboards(args)
        if args.command == "diff":
            return diff_dashboards(args)
        return export_dashboards(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
