#!/usr/bin/env python3
"""Export or import Grafana dashboards.

Purpose:
- Expose dashboard CLI entrypoints (`export-dashboard`, `list-dashboard`,
  `import-dashboard`, `diff`, and inspect commands) and normalize mode-specific
  arguments before delegating to workflow helpers.

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

Caveats:
- Keep `--output-format` normalization and dry-run column parsing in this facade.
- Avoid moving API behavior from workflow helpers back into the facade layer.
"""

import argparse
import getpass
from pathlib import Path
import sys
from typing import Any, Optional

from .clients.dashboard_client import GrafanaClient
from .auth_staging import AuthConfigError, resolve_cli_auth_from_namespace
from .dashboards.common import (
    DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_UID,
    DEFAULT_FOLDER_TITLE,
    DEFAULT_ORG_ID,
    DEFAULT_ORG_NAME,
    DEFAULT_UNKNOWN_UID,
    GrafanaApiError,
    GrafanaError,
)
from .dashboards.export_inventory import (
    build_folder_inventory_lookup,
    resolve_folder_inventory_record_for_dashboard,
)
from .dashboards.export_workflow import run_export_dashboards
from .dashboards.export_runtime import (
    build_export_workflow_deps as build_export_workflow_deps_from_runtime,
)
from .dashboards.diff_workflow import run_diff_dashboards
from .dashboards.export_inventory import (
    discover_dashboard_files as discover_dashboard_files_from_export,
)
from .dashboards.import_support import (
    build_import_payload,
    build_compare_diff_lines,
    build_local_compare_document,
    build_remote_compare_document,
    extract_dashboard_object,
    load_json_file,
    load_export_metadata as import_support_load_export_metadata,
    parse_dashboard_import_dry_run_columns,
    resolve_dashboard_uid_for_import,
    serialize_compare_document,
)
from .dashboards.import_workflow import run_import_dashboards
from .dashboards.import_runtime import (
    build_import_workflow_deps as build_import_workflow_deps_from_runtime,
)
from .dashboards.inspection_runtime import (
    build_inspection_workflow_deps as build_inspection_workflow_deps_from_runtime,
)
from .dashboards.inspection_report import (
    INSPECT_EXPORT_HELP_FULL_EXAMPLES,
    INSPECT_LIVE_HELP_FULL_EXAMPLES,
    INSPECT_REPORT_FORMAT_CHOICES,
    SUPPORTED_REPORT_COLUMN_VALUES,
)
from .dashboards.listing import (
    attach_dashboard_folder_paths,
    attach_dashboard_org,
    build_dashboard_summary_record,
    build_data_source_record,
    build_datasource_inventory_record,
    build_folder_path,
    format_dashboard_summary_line,
    format_data_source_line,
    list_dashboards as run_list_dashboards,
    render_dashboard_summary_csv,
    render_dashboard_summary_json,
    render_dashboard_summary_table,
    render_data_source_csv,
    render_data_source_json,
    render_data_source_table,
)
from .dashboards.output_support import (
    build_export_metadata,
    build_variant_index,
    sanitize_path_component,
    write_json_document,
)
from .dashboards.inspection_workflow import run_inspect_export, run_inspect_live
from .dashboards.inspection_report import (
    build_grouped_export_inspection_report_document,
    filter_export_inspection_report_document,
    parse_report_columns,
)
from .dashboards.inspection_render import render_export_inspection_tree_tables
from .dashboards.folder_support import (
    collect_folder_inventory,
    ensure_folder_inventory,
    inspect_folder_inventory,
)
from .dashboards.import_support import (
    render_folder_inventory_dry_run_table,
    describe_dashboard_import_mode,
)
from .dashboards.transformer import (
    build_datasource_catalog,
    build_external_export_document,
    build_preserved_web_import_document,
)
from .dashboards.screenshot import (
    capture_dashboard_screenshot as run_capture_dashboard_screenshot,
)
from .http_transport import build_json_http_transport
from .dashboards.variable_inspection import (
    render_dashboard_variable_document,
    inspect_dashboard_variables_with_client,
)

__all__ = [
    "DATASOURCE_INVENTORY_FILENAME",
    "DASHBOARD_PERMISSION_BUNDLE_FILENAME",
    "DEFAULT_DASHBOARD_TITLE",
    "DEFAULT_FOLDER_TITLE",
    "DEFAULT_FOLDER_UID",
    "DEFAULT_ORG_ID",
    "DEFAULT_ORG_NAME",
    "DEFAULT_UNKNOWN_UID",
    "EXPORT_METADATA_FILENAME",
    "FOLDER_INVENTORY_FILENAME",
    "GrafanaApiError",
    "GrafanaClient",
    "GrafanaError",
    "INSPECT_EXPORT_HELP_FULL_EXAMPLES",
    "PROMPT_EXPORT_SUBDIR",
    "RAW_EXPORT_SUBDIR",
    "ROOT_INDEX_KIND",
    "TOOL_SCHEMA_VERSION",
    "add_inspect_export_cli_args",
    "add_inspect_live_cli_args",
    "attach_dashboard_folder_paths",
    "attach_dashboard_org",
    "build_dashboard_summary_record",
    "build_data_source_record",
    "build_datasource_inventory_record",
    "build_datasource_catalog",
    "build_export_metadata",
    "build_external_export_document",
    "build_folder_inventory_lookup",
    "build_folder_path",
    "build_grouped_export_inspection_report_document",
    "build_import_payload",
    "build_json_http_transport",
    "build_preserved_web_import_document",
    "collect_folder_inventory",
    "describe_dashboard_import_mode",
    "diff_dashboards",
    "ensure_folder_inventory",
    "export_dashboards",
    "extract_dashboard_object",
    "filter_export_inspection_report_document",
    "format_dashboard_summary_line",
    "format_data_source_line",
    "import_dashboards",
    "inspect_export",
    "inspect_folder_inventory",
    "inspect_live",
    "list_dashboards",
    "main",
    "parse_args",
    "parse_report_columns",
    "render_dashboard_summary_csv",
    "render_dashboard_summary_json",
    "render_dashboard_summary_table",
    "render_data_source_csv",
    "render_data_source_json",
    "render_data_source_table",
    "render_export_inspection_tree_tables",
    "render_folder_inventory_dry_run_table",
    "resolve_auth",
    "resolve_folder_inventory_record_for_dashboard",
    "sanitize_path_component",
    "write_json_document",
]

DEFAULT_URL = "http://localhost:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_PAGE_SIZE = 500
DEFAULT_EXPORT_DIR = "dashboards"
RAW_EXPORT_SUBDIR = "raw"
PROMPT_EXPORT_SUBDIR = "prompt"
EXPORT_METADATA_FILENAME = "export-metadata.json"
FOLDER_INVENTORY_FILENAME = "folders.json"
DATASOURCE_INVENTORY_FILENAME = "datasources.json"
DASHBOARD_PERMISSION_BUNDLE_FILENAME = "permissions.json"
TOOL_SCHEMA_VERSION = 1
ROOT_INDEX_KIND = "grafana-utils-dashboard-export-index"
LIST_OUTPUT_FORMAT_CHOICES = ("table", "csv", "json")
IMPORT_DRY_RUN_OUTPUT_FORMAT_CHOICES = ("text", "table", "json")
INSPECT_OUTPUT_FORMAT_CHOICES = (
    "text",
    "table",
    "json",
    "report-table",
    "report-csv",
    "report-json",
    "report-tree",
    "report-tree-table",
    "report-dependency",
    "dependency",
    "dependency-json",
    "report-dependency-json",
    "governance",
    "governance-json",
)
VARIABLE_OUTPUT_FORMAT_CHOICES = ("table", "csv", "json")
SCREENSHOT_OUTPUT_FORMAT_CHOICES = ("png", "jpeg", "pdf")
SCREENSHOT_FULL_PAGE_OUTPUT_CHOICES = ("single", "tiles", "manifest")
SCREENSHOT_THEME_CHOICES = ("light", "dark")


class HelpFullAction(argparse.Action):
    """Print normal help plus a short extended examples section."""

    def __call__(self, parser, namespace, values, option_string=None):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        parser.print_help()
        examples = getattr(namespace, "_help_full_examples", "") or ""
        if examples:
            print("")
            print(examples)
        parser.exit()


def add_common_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add common cli args implementation."""
    connection_group = parser.add_argument_group("Connection Options")
    auth_group = parser.add_argument_group("Auth Options")
    connection_group.add_argument(
        "--url",
        default=DEFAULT_URL,
        help=f"Grafana base URL (default: {DEFAULT_URL})",
    )
    auth_group.add_argument(
        "--token",
        "--api-token",
        dest="api_token",
        default=None,
        help=(
            "Grafana API token. Preferred flag: --token. "
            "Falls back to GRAFANA_API_TOKEN."
        ),
    )
    auth_group.add_argument(
        "--prompt-token",
        action="store_true",
        help=(
            "Prompt for the Grafana API token without echo instead of passing "
            "--token on the command line."
        ),
    )
    auth_group.add_argument(
        "--basic-user",
        dest="username",
        default=None,
        help=(
            "Grafana Basic auth username. Preferred flag: --basic-user. "
            "Falls back to GRAFANA_USERNAME."
        ),
    )
    auth_group.add_argument(
        "--basic-password",
        dest="password",
        default=None,
        help=(
            "Grafana Basic auth password. Preferred flag: --basic-password. "
            "Falls back to GRAFANA_PASSWORD."
        ),
    )
    auth_group.add_argument(
        "--prompt-password",
        action="store_true",
        help=(
            "Prompt for the Grafana Basic auth password without echo instead of "
            "passing --basic-password on the command line."
        ),
    )
    connection_group.add_argument(
        "--timeout",
        type=int,
        default=DEFAULT_TIMEOUT,
        help=f"HTTP timeout in seconds (default: {DEFAULT_TIMEOUT}).",
    )
    connection_group.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )


def add_export_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add export cli args implementation."""
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
        help=(
            "Export dashboards from every visible Grafana organization and write per-org "
            "subdirectories under the export root. API token auth is not supported here; "
            "use Grafana username/password login."
        ),
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
    """Add list cli args implementation."""
    input_group = parser.add_argument_group("Input Options")
    target_group = parser.add_argument_group("Target Options")
    output_group = parser.add_argument_group("Output Options")
    input_group.add_argument(
        "--page-size",
        type=int,
        default=DEFAULT_PAGE_SIZE,
        help=f"Dashboard search page size (default: {DEFAULT_PAGE_SIZE}).",
    )
    target_group.add_argument(
        "--org-id",
        default=None,
        help="List dashboards from this Grafana organization ID instead of the current org context.",
    )
    target_group.add_argument(
        "--all-orgs",
        action="store_true",
        help=(
            "List dashboards from every Grafana organization. API token auth is not "
            "supported here; use Grafana username/password login."
        ),
    )
    output_group.add_argument(
        "--with-sources",
        action="store_true",
        help=(
            "For table or CSV output, fetch each dashboard payload and include resolved datasource "
            "names in the list output. JSON already includes datasource names and UIDs by default. "
            "This is slower because it makes extra API calls per dashboard."
        ),
    )
    render_group = output_group.add_mutually_exclusive_group()
    render_group.add_argument(
        "--table",
        action="store_true",
        help="Render dashboard summaries as a table.",
    )
    render_group.add_argument(
        "--csv",
        action="store_true",
        help="Render dashboard summaries as CSV.",
    )
    render_group.add_argument(
        "--json",
        action="store_true",
        help="Render dashboard summaries as JSON.",
    )
    output_group.add_argument(
        "--no-header",
        action="store_true",
        help="Do not print table headers when rendering the default table output.",
    )
    output_group.add_argument(
        "--output-format",
        choices=LIST_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for dashboard list output. "
            "Use table, csv, or json. This cannot be combined with --table, "
            "--csv, or --json."
        ),
    )


def add_import_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add import cli args implementation."""
    input_group = parser.add_argument_group("Input Options")
    target_group = parser.add_argument_group("Target Options")
    mutation_group = parser.add_argument_group("Mutation Options")
    safety_group = parser.add_argument_group("Safety Options")
    output_group = parser.add_argument_group("Output Options")
    input_group.add_argument(
        "--import-dir",
        required=True,
        help=(
            "Import dashboards from this directory. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly for normal imports. "
            "When --use-export-org is enabled, point this to the combined multi-org export root instead."
        ),
    )
    target_group.add_argument(
        "--org-id",
        default=None,
        help=(
            "Import dashboards into this explicit Grafana organization ID instead "
            "of the current org context. API token auth is not supported here; "
            "use Grafana username/password login."
        ),
    )
    target_group.add_argument(
        "--use-export-org",
        action="store_true",
        help=(
            "Import from a combined multi-org export root and route each org-specific "
            "raw export into the matching Grafana orgId recorded in that export. "
            "API token auth is not supported here; use Grafana username/password login."
        ),
    )
    target_group.add_argument(
        "--only-org-id",
        action="append",
        default=None,
        help=(
            "With --use-export-org, import only the selected exported orgId values. "
            "Repeat this flag to include multiple orgs."
        ),
    )
    target_group.add_argument(
        "--create-missing-orgs",
        action="store_true",
        help=(
            "With --use-export-org, create a missing destination Grafana organization "
            "from the exported org name when the exported orgId does not exist yet."
        ),
    )
    target_group.add_argument(
        "--require-matching-export-org",
        action="store_true",
        help=(
            "Require the raw export's recorded orgId to match the target Grafana "
            "org before dry-run or live import. This is a safety guard against "
            "accidental cross-org import."
        ),
    )
    mutation_group.add_argument(
        "--replace-existing",
        action="store_true",
        help="Update an existing destination dashboard when the imported dashboard UID already exists. Without this flag, existing UIDs are blocked.",
    )
    mutation_group.add_argument(
        "--update-existing-only",
        action="store_true",
        help="Reconcile only dashboards whose UID already exists in Grafana. Missing destination UIDs are skipped instead of created.",
    )
    mutation_group.add_argument(
        "--import-folder-uid",
        default=None,
        help="Force every imported dashboard into one destination Grafana folder UID. This overrides any folder UID carried by the exported dashboard files.",
    )
    mutation_group.add_argument(
        "--ensure-folders",
        action="store_true",
        help="Use the exported raw folder inventory to create any missing destination folders before import. In dry-run mode, also report folder missing/match/mismatch state first.",
    )
    mutation_group.add_argument(
        "--import-message",
        default="Imported by grafana-utils",
        help="Version-history message to attach to each imported dashboard revision in Grafana.",
    )
    mutation_group.add_argument(
        "--require-matching-folder-path",
        action="store_true",
        help=(
            "Only update an existing dashboard when the source raw folder path matches "
            "the destination Grafana folder path exactly. Missing dashboards still "
            "follow the active create/skip mode."
        ),
    )
    safety_group.add_argument(
        "--approve",
        action="store_true",
        help="Explicit acknowledgement required before live dashboard import runs. Not required with --dry-run.",
    )
    safety_group.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview what import would do without changing Grafana. This reports whether each dashboard would create, update, or be skipped/blocked.",
    )
    output_group.add_argument(
        "--table",
        action="store_true",
        help="For --dry-run only, render output in table form instead of per-dashboard log lines. With --ensure-folders, the folder check is also shown in table form.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="For --dry-run only, render one JSON document with mode, folder checks, dashboard actions, and summary counts.",
    )
    output_group.add_argument(
        "--no-header",
        action="store_true",
        help="For --dry-run --table only, omit the table header row.",
    )
    output_group.add_argument(
        "--output-format",
        choices=IMPORT_DRY_RUN_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for import dry-run output. "
            "Use text, table, or json. This cannot be combined with --table "
            "or --json."
        ),
    )
    output_group.add_argument(
        "--output-columns",
        default=None,
        help=(
            "For --dry-run --table only, render only these comma-separated columns. "
            "Supported values: uid, destination, action, folder_path, "
            "source_folder_path, destination_folder_path, reason, file."
        ),
    )
    output_group.add_argument(
        "--progress",
        action="store_true",
        help="Show concise per-dashboard import progress as current/total while processing files. Use this for long-running batch imports.",
    )
    output_group.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show detailed per-dashboard import output, including file paths, dry-run actions, and folder status details. Supersedes --progress.",
    )


def add_diff_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add diff cli args implementation."""
    input_group = parser.add_argument_group("Input Options")
    target_group = parser.add_argument_group("Target Options")
    output_group = parser.add_argument_group("Output Options")
    input_group.add_argument(
        "--import-dir",
        required=True,
        help=(
            "Compare dashboards from this directory against Grafana. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly."
        ),
    )
    target_group.add_argument(
        "--import-folder-uid",
        default=None,
        help="Override the destination Grafana folder UID when building the comparison payload.",
    )
    output_group.add_argument(
        "--context-lines",
        type=int,
        default=3,
        help="Number of surrounding lines to include in unified diff output (default: 3).",
    )


def add_inspect_export_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add inspect export cli args implementation."""
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
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--output-format",
        choices=INSPECT_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Single-flag output selector for inspect output. "
            "Use text, table, json, report-table, report-csv, report-json, "
            "report-tree, report-tree-table, dependency, dependency-json, "
            "governance, or governance-json. "
            "Use this instead of the legacy output flags. "
            "This cannot be combined with hidden legacy output flags."
        ),
    )
    parser.add_argument(
        "--report-columns",
        default=None,
        help=(
            "With report-table, report-csv, or report-tree-table --output-format values, "
            "render only these comma-separated report columns. "
            "Supported values: %s. Snake_case aliases like %s are also accepted."
            % (
                ", ".join(SUPPORTED_REPORT_COLUMN_VALUES),
                ", ".join(
                    [
                        "dashboard_uid",
                        "datasource_uid",
                        "datasource_type",
                        "datasource_family",
                    ]
                ),
            )
        ),
    )
    parser.add_argument(
        "--report-filter-datasource",
        default=None,
        help=(
            "With report-like --output-format values, only include query report rows whose datasource label, "
            "uid, type, or family exactly matches this value."
        ),
    )
    parser.add_argument(
        "--report-filter-panel-id",
        default=None,
        help=(
            "With report-like --output-format values, only include query report rows whose panel id "
            "exactly matches this value."
        ),
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--table",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="With table-like --output-format values, omit the per-section table header rows.",
    )
    parser.add_argument(
        "--output-file",
        default=None,
        help="Write inspect output to this file while still printing to stdout.",
    )


def add_inspect_live_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add inspect live cli args implementation."""
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
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--output-format",
        choices=INSPECT_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Single-flag output selector for inspect output. "
            "Use text, table, json, report-table, report-csv, report-json, "
            "report-tree, report-tree-table, dependency, dependency-json, "
            "governance, or governance-json. "
            "Use this instead of the legacy output flags. "
            "This cannot be combined with hidden legacy output flags."
        ),
    )
    parser.add_argument(
        "--report-columns",
        default=None,
        help=(
            "With report-table, report-csv, or report-tree-table --output-format values, "
            "render only these comma-separated report columns. "
            "Supported values: %s. Snake_case aliases like %s are also accepted."
            % (
                ", ".join(SUPPORTED_REPORT_COLUMN_VALUES),
                ", ".join(
                    [
                        "dashboard_uid",
                        "datasource_uid",
                        "datasource_type",
                        "datasource_family",
                    ]
                ),
            )
        ),
    )
    parser.add_argument(
        "--report-filter-datasource",
        default=None,
        help=(
            "With report-like --output-format values, only include query report rows whose datasource label, "
            "uid, type, or family exactly matches this value."
        ),
    )
    parser.add_argument(
        "--report-filter-panel-id",
        default=None,
        help=(
            "With report-like --output-format values, only include query report rows whose panel id "
            "exactly matches this value."
        ),
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--table",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="With table-like --output-format values, omit the per-section table header rows.",
    )
    parser.add_argument(
        "--output-file",
        default=None,
        help="Write inspect output to this file while still printing to stdout.",
    )


def add_inspect_vars_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add inspect vars cli args implementation."""
    add_common_cli_args(parser)
    target_group = parser.add_argument_group("Target Options")
    output_group = parser.add_argument_group("Output Options")
    target_group.add_argument(
        "--dashboard-uid",
        default=None,
        help="Grafana dashboard UID whose templating variables should be listed. Required unless --dashboard-url is provided.",
    )
    target_group.add_argument(
        "--dashboard-url",
        default=None,
        help="Full Grafana dashboard URL. When provided, the runtime can derive the dashboard UID from the URL path.",
    )
    target_group.add_argument(
        "--vars-query",
        default=None,
        help="Grafana variable query-string fragment, for example 'var-env=prod&var-host=web01'. This overlays current values in inspect-vars output.",
    )
    target_group.add_argument(
        "--org-id",
        default=None,
        help="Scope the variable inspection to this Grafana org ID by sending X-Grafana-Org-Id.",
    )
    output_group.add_argument(
        "--output-format",
        choices=VARIABLE_OUTPUT_FORMAT_CHOICES,
        default="table",
        help="Render dashboard variables as table, csv, or json (default: table).",
    )
    output_group.add_argument(
        "--no-header",
        action="store_true",
        help="Do not print table or CSV headers when rendering inspect-vars output.",
    )
    output_group.add_argument(
        "--output-file",
        default=None,
        help="Write inspect-vars output to this file while still printing to stdout.",
    )


def add_screenshot_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add screenshot cli args implementation."""
    add_common_cli_args(parser)
    target_group = parser.add_argument_group("Target Options")
    state_group = parser.add_argument_group("State Options")
    rendering_group = parser.add_argument_group("Rendering Options")
    output_group = parser.add_argument_group("Output Options")
    header_group = parser.add_argument_group("Header Options")
    target_group.add_argument(
        "--dashboard-uid",
        default=None,
        help="Grafana dashboard UID to capture from the browser-rendered UI. Required unless --dashboard-url is provided.",
    )
    target_group.add_argument(
        "--dashboard-url",
        default=None,
        help="Full Grafana dashboard URL. When provided, the runtime can reuse URL state such as var-*, from, to, orgId, and panelId.",
    )
    target_group.add_argument(
        "--slug",
        default=None,
        help="Optional dashboard slug. When omitted, the runtime falls back to the dashboard UID.",
    )
    output_group.add_argument(
        "--output",
        required=True,
        help="Write the captured browser output to this file path.",
    )
    target_group.add_argument(
        "--panel-id",
        default=None,
        help="Capture only this Grafana panel ID through the solo dashboard route.",
    )
    target_group.add_argument(
        "--org-id",
        default=None,
        help="Scope the browser session to this Grafana org ID by sending X-Grafana-Org-Id.",
    )
    state_group.add_argument(
        "--from",
        dest="from_value",
        default=None,
        help="Grafana time range start, for example now-6h.",
    )
    state_group.add_argument(
        "--to",
        default=None,
        help="Grafana time range end, for example now.",
    )
    state_group.add_argument(
        "--vars-query",
        default=None,
        help="Grafana variable query-string fragment, for example 'var-env=prod&var-host=web01'.",
    )
    state_group.add_argument(
        "--var",
        dest="vars",
        action="append",
        default=None,
        help="Repeatable Grafana template variable assignment. Example: --var env=prod --var region=us-east-1.",
    )
    rendering_group.add_argument(
        "--theme",
        choices=SCREENSHOT_THEME_CHOICES,
        default="dark",
        help="Override the Grafana UI theme used for the browser capture (default: dark).",
    )
    output_group.add_argument(
        "--output-format",
        choices=SCREENSHOT_OUTPUT_FORMAT_CHOICES,
        default=None,
        help="Force the output format instead of inferring it from the output filename.",
    )
    rendering_group.add_argument(
        "--width",
        type=int,
        default=1440,
        help="Browser viewport width in pixels.",
    )
    rendering_group.add_argument(
        "--height",
        type=int,
        default=1024,
        help="Browser viewport height in pixels.",
    )
    rendering_group.add_argument(
        "--device-scale-factor",
        type=float,
        default=1.0,
        help="Browser device scale factor for higher-density raster capture (default: 1.0).",
    )
    rendering_group.add_argument(
        "--full-page",
        action="store_true",
        help="Capture the full scrollable page instead of only the initial viewport.",
    )
    output_group.add_argument(
        "--full-page-output",
        choices=SCREENSHOT_FULL_PAGE_OUTPUT_CHOICES,
        default="single",
        help="When --full-page is enabled, write one stitched file, a tiles directory, or a tiles directory plus manifest metadata.",
    )
    rendering_group.add_argument(
        "--wait-ms",
        type=int,
        default=5000,
        help="Extra wait time in milliseconds after navigation so Grafana panels can finish rendering.",
    )
    rendering_group.add_argument(
        "--browser-path",
        default=None,
        help="Optional Chromium or Chrome executable path for browser-driven capture.",
    )
    output_group.add_argument(
        "--print-capture-url",
        action="store_true",
        help="Print the resolved Grafana capture URL before the browser capture starts.",
    )
    header_group.add_argument(
        "--header-title",
        default=None,
        help="Optional header title text to prepend above PNG/JPEG captures. Use __auto__ to derive a title from the dashboard metadata.",
    )
    header_group.add_argument(
        "--header-url",
        default=None,
        help="Optional header URL line to prepend above PNG/JPEG captures. Use __auto__ to render the resolved Grafana capture URL.",
    )
    header_group.add_argument(
        "--header-captured-at",
        action="store_true",
        help="Append one capture timestamp line in the generated PNG/JPEG header block.",
    )
    header_group.add_argument(
        "--header-text",
        default=None,
        help="Optional free-form note text to append below other generated PNG/JPEG header lines.",
    )


def parse_args(argv: Optional[list[str]] = None) -> argparse.Namespace:
    """Build dashboard CLI parser and normalize mutually-exclusive dashboard subcommand input.

    Flow:
    - Build mode-specific subparsers to enforce one command per execution.
    - Parse arguments, then normalize `--output-format` aliases into concrete mode
      flags.
    - Normalize table column selections for import dry-run output.
    """
    # Call graph: see callers/callees.
    #   Upstream callers: 1474
    #   Downstream callees: 1216, 1245, 1265, 244, 310, 378, 444, 480, 631, 657, 748, 838, 876

    parser = argparse.ArgumentParser(
        description="Export or import Grafana dashboards.",
        epilog=(
            "Examples:\n\n"
            "  Export dashboards from local Grafana with Basic auth:\n"
            "    grafana-util dashboard export --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n"
            "  Export dashboards with an API token:\n"
            "    export GRAFANA_API_TOKEN='your-token'\n"
            "    grafana-util dashboard export --url http://localhost:3000 "
            '--token "$GRAFANA_API_TOKEN" --export-dir ./dashboards --overwrite\n\n'
            "  Compare raw dashboard exports against local Grafana:\n"
            "    grafana-util dashboard diff --url http://localhost:3000 "
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
            "    grafana-util dashboard export --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n"
            "  Export dashboards with an API token:\n"
            "    export GRAFANA_API_TOKEN='your-token'\n"
            "    grafana-util dashboard export --url http://localhost:3000 "
            '--token "$GRAFANA_API_TOKEN" --export-dir ./dashboards --overwrite\n\n'
            "  Export into a flat directory layout instead of per-folder subdirectories:\n"
            "    grafana-util dashboard export --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --export-dir ./dashboards --flat"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(export_parser)
    add_export_cli_args(export_parser)

    list_parser = subparsers.add_parser(
        "list-dashboard",
        help="List live dashboard summaries from Grafana.",
        epilog=LIST_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(list_parser)
    add_list_cli_args(list_parser)

    import_parser = subparsers.add_parser(
        "import-dashboard",
        help="Import dashboards from exported raw JSON files.",
        epilog=IMPORT_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(import_parser)
    add_import_cli_args(import_parser)

    diff_parser = subparsers.add_parser(
        "diff",
        help="Compare exported raw dashboards with the current Grafana state.",
        epilog=DIFF_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(diff_parser)
    add_diff_cli_args(diff_parser)

    inspect_export_parser = subparsers.add_parser(
        "inspect-export",
        help="Inspect one raw dashboard export directory and summarize its structure.",
        epilog=INSPECT_EXPORT_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_inspect_export_cli_args(inspect_export_parser)
    inspect_live_parser = subparsers.add_parser(
        "inspect-live",
        help="Inspect live Grafana dashboards with the same summary/report modes as inspect-export.",
        epilog=INSPECT_LIVE_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_inspect_live_cli_args(inspect_live_parser)
    inspect_vars_parser = subparsers.add_parser(
        "inspect-vars",
        help="List dashboard templating variables and datasource-like choices from live Grafana.",
        epilog=INSPECT_VARS_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_inspect_vars_cli_args(inspect_vars_parser)
    screenshot_parser = subparsers.add_parser(
        "screenshot",
        help="Capture one Grafana dashboard or panel through a browser backend.",
        epilog=SCREENSHOT_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_screenshot_cli_args(screenshot_parser)

    args = parser.parse_args(argv)
    _normalize_output_format_args(args, parser)
    _validate_import_routing_args(args, parser)
    _parse_dashboard_import_output_columns(args, parser)
    return args


INSPECT_EXPORT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Show one machine-readable summary document:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format json\n\n"
    "  Render grouped dashboard-first query tables:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format report-tree-table\n\n"
    "  Show full inspect help with extended report examples:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --help-full"
)


INSPECT_LIVE_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Inspect live dashboards as a report JSON document:\n"
    '    grafana-util dashboard inspect-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" '
    "--output-format report-json\n\n"
    "  Filter to one panel in dashboard/panel/query tree output:\n"
    '    grafana-util dashboard inspect-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" '
    "--output-format report-tree --report-filter-panel-id 7\n\n"
    "  Show full inspect help with extended report examples:\n"
    '    grafana-util dashboard inspect-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --help-full'
)

LIST_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  List dashboards as JSON for scripting:\n"
    '    grafana-util dashboard list-dashboard --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json\n\n'
    "  List dashboards from all orgs in table output:\n"
    "    grafana-util dashboard list-dashboard --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --table"
)

IMPORT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Preview a dashboard import without changing Grafana:\n"
    "    grafana-util dashboard import-dashboard --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --dry-run --table\n\n"
    "  Apply a reviewed dashboard import into one org:\n"
    "    grafana-util dashboard import-dashboard --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing --approve\n\n"
    "  Route a combined multi-org export back by exported org id:\n"
    "    grafana-util dashboard import-dashboard --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards --use-export-org --only-org-id 2 --dry-run --json"
)

DIFF_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Compare raw dashboard exports against Grafana:\n"
    "    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw\n\n"
    "  Compare the same export against one destination folder:\n"
    "    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --import-folder-uid infra-folder"
)

INSPECT_VARS_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  List dashboard variables from a UID:\n"
    '    grafana-util dashboard inspect-vars --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --dashboard-uid cpu-main\n\n'
    "  Overlay current variable values from a dashboard URL:\n"
    "    grafana-util dashboard inspect-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output-format json"
)

SCREENSHOT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Capture a full dashboard screenshot:\n"
    "    grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page\n\n"
    "  Capture a solo panel with a custom header:\n"
    "    grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --token \"$GRAFANA_API_TOKEN\" --output ./panel.png --header-title 'CPU Busy'"
)


def _normalize_output_format_args(
    args: argparse.Namespace,
    parser: argparse.ArgumentParser,
) -> None:
    """Translate `--output-format` aliases into exclusive list/import output flags."""
    output_format = getattr(args, "output_format", None)
    command = getattr(args, "command", None)
    if output_format is None:
        return
    if command in ("list-dashboard",):
        if (
            bool(getattr(args, "table", False))
            or bool(getattr(args, "csv", False))
            or bool(getattr(args, "json", False))
        ):
            parser.error(
                "--output-format cannot be combined with --table, --csv, or --json for dashboard list commands."
            )
        args.table = output_format == "table"
        args.csv = output_format == "csv"
        args.json = output_format == "json"
        return
    if command == "import-dashboard":
        if bool(getattr(args, "table", False)) or bool(getattr(args, "json", False)):
            parser.error(
                "--output-format cannot be combined with --table or --json for import-dashboard."
            )
        args.table = output_format == "table"
        args.json = output_format == "json"


def _parse_dashboard_import_output_columns(
    args: argparse.Namespace,
    parser: argparse.ArgumentParser,
) -> None:
    """Parse and validate import dry-run output columns only for table-mode import."""
    if getattr(args, "command", None) != "import-dashboard":
        return
    value = getattr(args, "output_columns", None)
    if value is None:
        return
    if not bool(getattr(args, "table", False)):
        parser.error(
            "--output-columns is only supported with --dry-run --table or table-like --output-format for import-dashboard."
        )
    try:
        args.output_columns = parse_dashboard_import_dry_run_columns(value)
    except GrafanaError as exc:
        parser.error(str(exc))


def _validate_import_routing_args(
    args: argparse.Namespace,
    parser: argparse.ArgumentParser,
) -> None:
    """Internal helper for validate import routing args."""
    if getattr(args, "command", None) != "import-dashboard":
        return
    use_export_org = bool(getattr(args, "use_export_org", False))
    only_org_ids = getattr(args, "only_org_id", None) or []
    if only_org_ids and not use_export_org:
        parser.error("--only-org-id requires --use-export-org for import-dashboard.")
    if bool(getattr(args, "create_missing_orgs", False)) and not use_export_org:
        parser.error(
            "--create-missing-orgs requires --use-export-org for import-dashboard."
        )
    if use_export_org and getattr(args, "org_id", None):
        parser.error(
            "--use-export-org cannot be combined with --org-id for import-dashboard."
        )
    if use_export_org and bool(getattr(args, "require_matching_export_org", False)):
        parser.error(
            "--use-export-org cannot be combined with --require-matching-export-org for import-dashboard."
        )


def resolve_auth(args: argparse.Namespace) -> dict[str, str]:
    """Resolve auth implementation."""
    try:
        headers, _auth_mode = resolve_cli_auth_from_namespace(
            args,
            prompt_reader=getpass.getpass,
            token_prompt_reader=getpass.getpass,
            password_prompt_reader=getpass.getpass,
        )
        return headers
    except AuthConfigError as exc:
        raise GrafanaError(str(exc))


def _build_export_workflow_deps() -> dict[str, Any]:
    """Internal helper for build export workflow deps."""
    return build_export_workflow_deps_from_runtime(
        {
            "GrafanaError": GrafanaError,
            "DATASOURCE_INVENTORY_FILENAME": DATASOURCE_INVENTORY_FILENAME,
            "DASHBOARD_PERMISSION_BUNDLE_FILENAME": DASHBOARD_PERMISSION_BUNDLE_FILENAME,
            "DEFAULT_DASHBOARD_TITLE": DEFAULT_DASHBOARD_TITLE,
            "DEFAULT_FOLDER_TITLE": DEFAULT_FOLDER_TITLE,
            "DEFAULT_ORG_ID": DEFAULT_ORG_ID,
            "DEFAULT_ORG_NAME": DEFAULT_ORG_NAME,
            "DEFAULT_UNKNOWN_UID": DEFAULT_UNKNOWN_UID,
            "EXPORT_METADATA_FILENAME": EXPORT_METADATA_FILENAME,
            "FOLDER_INVENTORY_FILENAME": FOLDER_INVENTORY_FILENAME,
            "PROMPT_EXPORT_SUBDIR": PROMPT_EXPORT_SUBDIR,
            "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
            "ROOT_INDEX_KIND": ROOT_INDEX_KIND,
            "TOOL_SCHEMA_VERSION": TOOL_SCHEMA_VERSION,
            "build_client": build_client,
            "build_variant_index": build_variant_index,
            "extract_dashboard_object": extract_dashboard_object,
            "sys": sys,
        }
    )


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


def _build_inspection_workflow_deps() -> dict[str, Any]:
    """Internal helper for build inspection workflow deps."""
    return build_inspection_workflow_deps_from_runtime(
        {
            "DASHBOARD_PERMISSION_BUNDLE_FILENAME": DASHBOARD_PERMISSION_BUNDLE_FILENAME,
            "DATASOURCE_INVENTORY_FILENAME": DATASOURCE_INVENTORY_FILENAME,
            "DEFAULT_DASHBOARD_TITLE": DEFAULT_DASHBOARD_TITLE,
            "DEFAULT_FOLDER_TITLE": DEFAULT_FOLDER_TITLE,
            "DEFAULT_ORG_ID": DEFAULT_ORG_ID,
            "DEFAULT_ORG_NAME": DEFAULT_ORG_NAME,
            "DEFAULT_UNKNOWN_UID": DEFAULT_UNKNOWN_UID,
            "EXPORT_METADATA_FILENAME": EXPORT_METADATA_FILENAME,
            "FOLDER_INVENTORY_FILENAME": FOLDER_INVENTORY_FILENAME,
            "GrafanaError": GrafanaError,
            "PROMPT_EXPORT_SUBDIR": PROMPT_EXPORT_SUBDIR,
            "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
            "ROOT_INDEX_KIND": ROOT_INDEX_KIND,
            "TOOL_SCHEMA_VERSION": TOOL_SCHEMA_VERSION,
            "build_client": build_client,
        }
    )


def inspect_live(args: argparse.Namespace) -> int:
    """Inspect live Grafana dashboards by reusing the raw-export inspection pipeline."""
    return run_inspect_live(args, _build_inspection_workflow_deps())


def inspect_export(args: argparse.Namespace) -> int:
    """Inspect one raw export directory and summarize dashboards, folders, and datasources."""
    return run_inspect_export(args, _build_inspection_workflow_deps())


def inspect_vars(args: argparse.Namespace) -> int:
    """Inspect one live dashboard's templating variables."""
    client = build_client(args)
    if getattr(args, "org_id", None):
        client = client.with_org_id(args.org_id)
    document = inspect_dashboard_variables_with_client(
        client,
        dashboard_uid=getattr(args, "dashboard_uid", None),
        dashboard_url=getattr(args, "dashboard_url", None),
        vars_query=getattr(args, "vars_query", None),
    )
    rendered = render_dashboard_variable_document(
        document,
        output_format=getattr(args, "output_format", "table"),
        include_header=not bool(getattr(args, "no_header", False)),
    )
    normalized = rendered.rstrip("\n")
    output_file = getattr(args, "output_file", None)
    if output_file:
        output_path = Path(output_file)
        if output_path.parent:
            output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(f"{normalized}\n", encoding="utf-8")
    if normalized:
        print(normalized)
    return 0


def screenshot_dashboard(args: argparse.Namespace) -> int:
    """Capture one browser-rendered dashboard or panel image/PDF."""
    client = build_client(args)
    result = run_capture_dashboard_screenshot(args, client=client)
    if isinstance(result, dict) and result.get("output"):
        print(str(result["output"]))
    return 0


def _build_import_workflow_deps() -> dict[str, Any]:
    """Internal helper for build import workflow deps."""
    return build_import_workflow_deps_from_runtime(
        {
            "DEFAULT_UNKNOWN_UID": DEFAULT_UNKNOWN_UID,
            "DASHBOARD_PERMISSION_BUNDLE_FILENAME": DASHBOARD_PERMISSION_BUNDLE_FILENAME,
            "DATASOURCE_INVENTORY_FILENAME": DATASOURCE_INVENTORY_FILENAME,
            "EXPORT_METADATA_FILENAME": EXPORT_METADATA_FILENAME,
            "FOLDER_INVENTORY_FILENAME": FOLDER_INVENTORY_FILENAME,
            "GrafanaError": GrafanaError,
            "PROMPT_EXPORT_SUBDIR": PROMPT_EXPORT_SUBDIR,
            "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
            "ROOT_INDEX_KIND": ROOT_INDEX_KIND,
            "TOOL_SCHEMA_VERSION": TOOL_SCHEMA_VERSION,
            "build_client": build_client,
        }
    )


def import_dashboards(args: argparse.Namespace) -> int:
    """Import previously exported raw dashboard JSON files through Grafana's API."""
    return run_import_dashboards(args, _build_import_workflow_deps())


def _build_diff_workflow_deps() -> dict[str, Any]:
    """Internal helper for build diff workflow deps."""
    return {
        "RAW_EXPORT_SUBDIR": RAW_EXPORT_SUBDIR,
        "build_client": build_client,
        "build_compare_diff_lines": build_compare_diff_lines,
        "build_local_compare_document": build_local_compare_document,
        "build_remote_compare_document": build_remote_compare_document,
        "discover_dashboard_files": (
            lambda import_dir: discover_dashboard_files_from_export(
                import_dir,
                RAW_EXPORT_SUBDIR,
                PROMPT_EXPORT_SUBDIR,
                EXPORT_METADATA_FILENAME,
                FOLDER_INVENTORY_FILENAME,
                DATASOURCE_INVENTORY_FILENAME,
                DASHBOARD_PERMISSION_BUNDLE_FILENAME,
            )
        ),
        "load_export_metadata": (
            lambda import_dir, expected_variant=None: import_support_load_export_metadata(
                import_dir,
                export_metadata_filename=EXPORT_METADATA_FILENAME,
                root_index_kind=ROOT_INDEX_KIND,
                tool_schema_version=TOOL_SCHEMA_VERSION,
                expected_variant=expected_variant,
            )
        ),
        "load_json_file": load_json_file,
        "resolve_dashboard_uid_for_import": resolve_dashboard_uid_for_import,
        "serialize_compare_document": serialize_compare_document,
    }


def diff_dashboards(args: argparse.Namespace) -> int:
    """Compare local raw dashboard exports with the current Grafana state."""
    return run_diff_dashboards(args, _build_diff_workflow_deps())


def build_client(args: argparse.Namespace) -> GrafanaClient:
    """Build the dashboard API client from parsed CLI arguments."""
    headers = resolve_auth(args)
    return GrafanaClient(
        base_url=args.url,
        headers=headers,
        timeout=args.timeout,
        verify_ssl=args.verify_ssl,
    )


def main(argv: Optional[list[str]] = None) -> int:
    """Dispatch normalized dashboard commands to their workflow entrypoints.

    Flow:
    - Parse and normalize into a single `command` field.
    - Hand off to workflow helpers (`list`, `export`, `import`, `diff`,
      `inspect`) based on command name.
    - Convert caught CLI errors into user-facing exit codes.
    """
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 1016, 1321, 1326, 1336, 1363, 1367, 1372, 1393, 1420, 1458

    args = parse_args(argv)
    try:
        if args.command == "list-dashboard":
            return list_dashboards(args)
        if args.command == "inspect-export":
            return inspect_export(args)
        if args.command == "inspect-live":
            return inspect_live(args)
        if args.command == "inspect-vars":
            return inspect_vars(args)
        if args.command == "screenshot":
            return screenshot_dashboard(args)
        if args.command == "import-dashboard":
            if not bool(getattr(args, "dry_run", False)) and not bool(
                getattr(args, "approve", False)
            ):
                raise GrafanaError(
                    "Dashboard import requires --approve unless --dry-run is active."
                )
            return import_dashboards(args)
        if args.command == "diff":
            return diff_dashboards(args)
        return export_dashboards(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
