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
import getpass
import json
import re
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

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
from .dashboards.export_inventory import (
    discover_dashboard_files as discover_dashboard_files_from_export,
)
from .dashboards.folder_support import (
    build_folder_inventory_lookup,
    build_folder_inventory_record,
    build_import_dashboard_folder_path,
    build_live_folder_inventory_record,
    collect_folder_inventory,
    determine_folder_inventory_status,
    ensure_folder_inventory,
    inspect_folder_inventory,
    load_datasource_inventory as load_datasource_inventory_from_folder_support,
    load_folder_inventory as load_folder_inventory_from_folder_support,
    resolve_dashboard_import_folder_path,
    resolve_folder_inventory_record_for_dashboard,
    resolve_folder_inventory_requirements as resolve_folder_inventory_requirements_from_folder_support,
)
from .dashboards.import_support import (
    build_compare_diff_lines,
    build_import_payload,
    build_local_compare_document,
    build_remote_compare_document,
    build_dashboard_import_dry_run_record,
    describe_dashboard_import_mode,
    determine_dashboard_import_action,
    determine_import_folder_uid_override,
    extract_dashboard_object,
    load_json_file,
    load_export_metadata as import_support_load_export_metadata,
    render_dashboard_import_dry_run_json,
    render_dashboard_import_dry_run_table,
    render_folder_inventory_dry_run_table,
    resolve_dashboard_uid_for_import,
    serialize_compare_document,
    validate_export_metadata as import_support_validate_export_metadata,
)
from .dashboards.import_workflow import run_import_dashboards
from .dashboards.inspection_report import (
    INSPECT_EXPORT_HELP_FULL_EXAMPLES,
    INSPECT_LIVE_HELP_FULL_EXAMPLES,
    INSPECT_REPORT_FORMAT_CHOICES,
    REPORT_COLUMN_ALIASES,
    build_export_inspection_report_document,
    build_grouped_export_inspection_report_document,
    filter_export_inspection_report_document,
    parse_report_columns,
    render_export_inspection_grouped_report,
    render_export_inspection_report_csv,
    render_export_inspection_report_tables,
    render_export_inspection_tree_tables,
)
from .dashboards.inspection_summary import (
    build_export_inspection_document as build_export_inspection_document_from_summary,
    render_export_inspection_summary as render_export_inspection_summary_from_summary,
    render_export_inspection_tables as render_export_inspection_tables_from_summary,
)
from .dashboards.listing import (
    attach_dashboard_folder_paths,
    attach_dashboard_org,
    attach_dashboard_sources as attach_dashboard_sources_from_listing,
    build_dashboard_summary_record,
    build_data_source_record,
    build_datasource_inventory_record,
    build_folder_path,
    describe_datasource_ref,
    format_dashboard_summary_line,
    format_data_source_line,
    list_dashboards as run_list_dashboards,
    list_data_sources as run_list_data_sources,
    render_dashboard_summary_csv,
    render_dashboard_summary_json,
    render_dashboard_summary_table,
    render_data_source_csv,
    render_data_source_json,
    render_data_source_table,
    resolve_dashboard_source_metadata as resolve_dashboard_source_metadata_from_listing,
    resolve_datasource_uid,
)
from .dashboards.progress import (
    print_dashboard_export_progress,
    print_dashboard_export_progress_summary,
    print_dashboard_import_progress,
)
from .dashboards.inspection_workflow import (
    materialize_live_inspection_export as run_materialize_live_inspection_export,
)
from .dashboards.inspection_workflow import run_inspect_export, run_inspect_live
from .dashboards.transformer import (
    build_datasource_catalog,
    build_external_export_document,
    build_preserved_web_import_document,
    collect_datasource_refs,
    is_builtin_datasource_ref,
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
        choices=INSPECT_REPORT_FORMAT_CHOICES,
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
        choices=INSPECT_REPORT_FORMAT_CHOICES,
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
    return discover_dashboard_files_from_export(
        import_dir,
        RAW_EXPORT_SUBDIR,
        PROMPT_EXPORT_SUBDIR,
        EXPORT_METADATA_FILENAME,
        FOLDER_INVENTORY_FILENAME,
        DATASOURCE_INVENTORY_FILENAME,
    )
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


def load_folder_inventory(
    import_dir: Path,
    metadata: Optional[Dict[str, Any]] = None,
) -> List[Dict[str, str]]:
    return load_folder_inventory_from_folder_support(
        import_dir,
        folder_inventory_filename=FOLDER_INVENTORY_FILENAME,
        metadata=metadata,
    )


def load_datasource_inventory(
    import_dir: Path,
    metadata: Optional[Dict[str, Any]] = None,
) -> List[Dict[str, str]]:
    return load_datasource_inventory_from_folder_support(
        import_dir,
        datasource_inventory_filename=DATASOURCE_INVENTORY_FILENAME,
        metadata=metadata,
    )


def resolve_folder_inventory_requirements(
    args: argparse.Namespace,
    import_dir: Path,
    metadata: Optional[Dict[str, Any]],
) -> List[Dict[str, str]]:
    """Load the optional folder inventory and enforce explicit operator intent."""
    return resolve_folder_inventory_requirements_from_folder_support(
        args,
        import_dir,
        folder_inventory_filename=FOLDER_INVENTORY_FILENAME,
        metadata=metadata,
    )


def load_export_metadata(
    import_dir: Path,
    expected_variant: Optional[str] = None,
) -> Optional[Dict[str, Any]]:
    """Load the optional export manifest and validate its schema version when present."""
    return import_support_load_export_metadata(
        import_dir,
        export_metadata_filename=EXPORT_METADATA_FILENAME,
        root_index_kind=ROOT_INDEX_KIND,
        tool_schema_version=TOOL_SCHEMA_VERSION,
        expected_variant=expected_variant,
    )


def validate_export_metadata(
    metadata: Dict[str, Any],
    metadata_path: Path,
    expected_variant: Optional[str] = None,
) -> None:
    """Reject dashboard export manifests this implementation does not understand."""
    import_support_validate_export_metadata(
        metadata,
        metadata_path,
        root_index_kind=ROOT_INDEX_KIND,
        tool_schema_version=TOOL_SCHEMA_VERSION,
        expected_variant=expected_variant,
    )


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


def list_dashboards(args: argparse.Namespace) -> int:
    """List live dashboard summaries without exporting dashboard JSON."""
    return run_list_dashboards(
        args,
        build_client=build_client,
        extract_dashboard_object=extract_dashboard_object,
        datasource_error=GrafanaError,
    )


def resolve_dashboard_source_metadata(
    payload: Dict[str, Any],
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Any:
    """Keep the historical dashboard-cli helper signature stable."""
    return resolve_dashboard_source_metadata_from_listing(
        payload,
        extract_dashboard_object=extract_dashboard_object,
        datasource_error=GrafanaError,
        datasources_by_uid=datasources_by_uid,
        datasources_by_name=datasources_by_name,
    )


def attach_dashboard_sources(
    client: GrafanaClient,
    summaries: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    """Keep the historical dashboard-cli helper signature stable."""
    return attach_dashboard_sources_from_listing(
        client,
        summaries,
        extract_dashboard_object=extract_dashboard_object,
        datasource_error=GrafanaError,
    )


def list_data_sources(args: argparse.Namespace) -> int:
    """List live Grafana data sources."""
    return run_list_data_sources(args, build_client=build_client)


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


def _build_inspection_workflow_deps() -> Dict[str, Any]:
    return {
        "GrafanaError": GrafanaError,
        "DATASOURCE_INVENTORY_FILENAME": DATASOURCE_INVENTORY_FILENAME,
        "EXPORT_METADATA_FILENAME": EXPORT_METADATA_FILENAME,
        "FOLDER_INVENTORY_FILENAME": FOLDER_INVENTORY_FILENAME,
        "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
        "attach_dashboard_org": attach_dashboard_org,
        "build_client": build_client,
        "build_datasource_catalog": build_datasource_catalog,
        "build_dashboard_index_item": build_dashboard_index_item,
        "build_datasource_inventory_record": build_datasource_inventory_record,
        "build_export_inspection_document": (
            lambda import_dir: build_export_inspection_document_from_summary(
                import_dir,
                {
                    "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
                    "build_datasource_catalog": build_datasource_catalog,
                    "build_folder_inventory_lookup": build_folder_inventory_lookup,
                    "collect_datasource_refs": collect_datasource_refs,
                    "discover_dashboard_files": discover_dashboard_files,
                    "extract_dashboard_object": extract_dashboard_object,
                    "iter_dashboard_panels": iter_dashboard_panels,
                    "load_datasource_inventory": load_datasource_inventory,
                    "load_export_metadata": load_export_metadata,
                    "load_folder_inventory": load_folder_inventory,
                    "load_json_file": load_json_file,
                    "resolve_folder_inventory_record_for_dashboard": resolve_folder_inventory_record_for_dashboard,
                },
            )
        ),
        "build_grouped_export_inspection_report_document": build_grouped_export_inspection_report_document,
        "build_export_inspection_report_document": (
            lambda import_dir: build_export_inspection_report_document(
                import_dir,
                {
                    "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
                    "build_folder_inventory_lookup": build_folder_inventory_lookup,
                    "discover_dashboard_files": discover_dashboard_files,
                    "extract_dashboard_object": extract_dashboard_object,
                    "iter_dashboard_panels": iter_dashboard_panels,
                    "load_datasource_inventory": load_datasource_inventory,
                    "load_export_metadata": load_export_metadata,
                    "load_folder_inventory": load_folder_inventory,
                    "load_json_file": load_json_file,
                    "resolve_folder_inventory_record_for_dashboard": resolve_folder_inventory_record_for_dashboard,
                },
            )
        ),
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
        "render_export_inspection_summary": render_export_inspection_summary_from_summary,
        "render_export_inspection_tree_tables": render_export_inspection_tree_tables,
        "render_export_inspection_tables": render_export_inspection_tables_from_summary,
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
