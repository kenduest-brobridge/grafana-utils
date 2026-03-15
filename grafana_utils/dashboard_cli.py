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
import json
import sys
from pathlib import Path
from typing import Any, Optional

from .batch_error_policy import add_error_policy_argument
from .clients.dashboard_client import GrafanaClient
from .auth_staging import AuthConfigError, resolve_cli_auth_from_namespace
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
from .dashboards.export_runtime import (
    build_export_workflow_deps as build_export_workflow_deps_from_runtime,
)
from .dashboards.diff_workflow import run_diff_dashboards
from .dashboards.export_inventory import (
    discover_dashboard_files as discover_dashboard_files_from_export,
    resolve_export_org_id as resolve_export_org_id_from_export_inventory,
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
from .dashboards.folder_path_match import (
    apply_folder_path_guard_to_action,
    build_folder_path_match_result,
    resolve_existing_dashboard_folder_path,
    resolve_source_dashboard_folder_path,
)
from .dashboards.import_support import (
    build_compare_diff_lines,
    build_local_compare_document,
    build_remote_compare_document,
    build_dashboard_import_dry_run_record,
    build_import_payload,
    describe_dashboard_import_mode,
    determine_dashboard_import_action,
    determine_import_folder_uid_override,
    extract_dashboard_object,
    load_json_file,
    load_export_metadata as import_support_load_export_metadata,
    parse_dashboard_import_dry_run_columns,
    render_dashboard_import_dry_run_json,
    render_dashboard_import_dry_run_table,
    render_folder_inventory_dry_run_table,
    resolve_dashboard_uid_for_import,
    serialize_compare_document,
    validate_export_metadata as import_support_validate_export_metadata,
)
from .dashboards.import_workflow import run_import_dashboards
from .dashboards.import_runtime import (
    build_import_workflow_deps as build_import_workflow_deps_from_runtime,
)
from .dashboards.inspection_runtime import (
    InspectionWorkflowDeps,
    build_inspection_workflow_deps as build_inspection_workflow_deps_from_runtime,
    iter_dashboard_panels as iter_dashboard_panels_from_runtime,
)
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
from .dashboards.output_support import (
    build_all_orgs_output_dir as build_all_orgs_output_dir_from_output_support,
    build_dashboard_index_item as build_dashboard_index_item_from_output_support,
    build_export_metadata as build_export_metadata_from_output_support,
    build_export_variant_dirs as build_export_variant_dirs_from_output_support,
    build_output_path as build_output_path_from_output_support,
    build_root_export_index as build_root_export_index_from_output_support,
    build_variant_index,
    ensure_dashboard_write_target as ensure_dashboard_write_target_from_output_support,
    sanitize_path_component,
    write_dashboard as write_dashboard_from_output_support,
    write_json_document,
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
from .roadmap_contracts import (
    build_preflight_check_document,
    build_promotion_plan_document,
    render_preflight_check_text,
    render_promotion_plan_text,
)
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
LIST_OUTPUT_FORMAT_CHOICES = ("table", "csv", "json")
IMPORT_DRY_RUN_OUTPUT_FORMAT_CHOICES = ("text", "table", "json")
PLAN_OUTPUT_FORMAT_CHOICES = ("text", "json")
INSPECT_OUTPUT_FORMAT_CHOICES = (
    "text",
    "table",
    "json",
    "report-table",
    "report-csv",
    "report-json",
    "report-tree",
    "report-tree-table",
    "governance",
    "governance-json",
    "graph-json",
    "graph-dot",
    "graph-governance",
    "graph-governance-json",
)
INSPECT_VIEW_CHOICES = ("summary", "query", "governance")
INSPECT_FORMAT_CHOICES = ("text", "table", "csv", "json")
INSPECT_LAYOUT_CHOICES = ("flat", "tree")


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
        "--prompt-token",
        action="store_true",
        help=(
            "Prompt for the Grafana API token without echo instead of passing "
            "--token on the command line."
        ),
    )
    parser.add_argument(
        "--basic-user",
        dest="username",
        default=None,
        help=(
            "Grafana Basic auth username. Preferred flag: --basic-user. "
            "Falls back to GRAFANA_USERNAME."
        ),
    )
    parser.add_argument(
        "--basic-password",
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
        help=(
            "List dashboards from every Grafana organization. API token auth is not "
            "supported here; use Grafana username/password login."
        ),
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
    parser.add_argument(
        "--output-format",
        choices=LIST_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for dashboard list output. "
            "Use table, csv, or json. This cannot be combined with --table, "
            "--csv, or --json."
        ),
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


def add_import_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--import-dir",
        required=True,
        help=(
            "Import dashboards from this directory. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly for normal imports. "
            "When --use-export-org is enabled, point this to the combined multi-org export root instead."
        ),
    )
    parser.add_argument(
        "--org-id",
        default=None,
        help=(
            "Import dashboards into this explicit Grafana organization ID instead "
            "of the current org context. API token auth is not supported here; "
            "use Grafana username/password login."
        ),
    )
    parser.add_argument(
        "--use-export-org",
        action="store_true",
        help=(
            "Import from a combined multi-org export root and route each org-specific "
            "raw export into the matching Grafana orgId recorded in that export. "
            "API token auth is not supported here; use Grafana username/password login."
        ),
    )
    parser.add_argument(
        "--only-org-id",
        action="append",
        default=None,
        help=(
            "With --use-export-org, import only the selected exported orgId values. "
            "Repeat this flag to include multiple orgs."
        ),
    )
    parser.add_argument(
        "--create-missing-orgs",
        action="store_true",
        help=(
            "With --use-export-org, create a missing destination Grafana organization "
            "from the exported org name when the exported orgId does not exist yet."
        ),
    )
    parser.add_argument(
        "--require-matching-export-org",
        action="store_true",
        help=(
            "Require the raw export's recorded orgId to match the target Grafana "
            "org before dry-run or live import. This is a safety guard against "
            "accidental cross-org import."
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
        "--require-matching-folder-path",
        action="store_true",
        help=(
            "Only update an existing dashboard when the source raw folder path matches "
            "the destination Grafana folder path exactly. Missing dashboards still "
            "follow the active create/skip mode."
        ),
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
        "--output-format",
        choices=IMPORT_DRY_RUN_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for import dry-run output. "
            "Use text, table, or json. This cannot be combined with --table "
            "or --json."
        ),
    )
    parser.add_argument(
        "--output-columns",
        default=None,
        help=(
            "For --dry-run --table only, render only these comma-separated columns. "
            "Supported values: uid, destination, action, folder_path, "
            "source_folder_path, destination_folder_path, reason, file."
        ),
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


def add_promote_plan_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--source-bundle",
        required=True,
        help="Path to the source promotion bundle JSON document.",
    )
    parser.add_argument(
        "--target-inventory",
        required=True,
        help="Path to the target inventory JSON document.",
    )
    parser.add_argument(
        "--dashboard-uid-map-file",
        default=None,
        help="Optional JSON object file that remaps source dashboard UID to target dashboard UID.",
    )
    parser.add_argument(
        "--dashboard-name-map-file",
        default=None,
        help="Optional JSON object file that remaps source dashboard UID to target dashboard name.",
    )
    parser.add_argument(
        "--datasource-uid-map-file",
        default=None,
        help="Optional JSON object file that remaps source datasource UID to target datasource UID.",
    )
    parser.add_argument(
        "--datasource-name-map-file",
        default=None,
        help="Optional JSON object file that remaps source datasource name to target datasource name.",
    )
    parser.add_argument(
        "--output-format",
        choices=PLAN_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render the promotion plan as text or json (default: text).",
    )
    parser.add_argument(
        "--skip-preflight",
        action="store_true",
        help="Mark plan items as not requiring preflight in the staged plan document.",
    )


def add_preflight_plan_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--plan-file",
        required=True,
        help="Path to the promotion plan JSON document to validate.",
    )
    parser.add_argument(
        "--availability-file",
        default=None,
        help="Optional JSON document describing destination availability and required dependencies.",
    )
    parser.add_argument(
        "--output-format",
        choices=PLAN_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render the preflight check as text or json (default: text).",
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
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--output-format",
        choices=INSPECT_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Legacy single-flag output selector for inspect output. "
            "Use text, table, json, report-table, report-csv, report-json, "
            "report-tree, report-tree-table, governance, governance-json, "
            "graph-json, graph-dot, graph-governance, or graph-governance-json. "
            "Prefer --view with --format and optional --layout for new usage. "
            "This cannot be combined with hidden legacy output flags."
        ),
    )
    parser.add_argument(
        "--view",
        choices=INSPECT_VIEW_CHOICES,
        default=None,
        help=(
            "Preferred inspect selector for what to render. "
            "Use summary, query, or governance. "
            "Combine with --format and optional --layout instead of legacy --output-format."
        ),
    )
    parser.add_argument(
        "--format",
        choices=INSPECT_FORMAT_CHOICES,
        default=None,
        help=(
            "Preferred inspect selector for output encoding. "
            "Use text, table, csv, or json. "
            "Combine with --view and optional --layout."
        ),
    )
    parser.add_argument(
        "--layout",
        choices=INSPECT_LAYOUT_CHOICES,
        default=None,
        help="Preferred inspect selector for query layout. Use flat or tree with --view query.",
    )
    parser.add_argument(
        "--report-columns",
        default=None,
        help=(
            "With query/table, query/csv, or query/tree/table output, "
            "render only these comma-separated report columns. "
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
            "With query output, only include query report rows whose datasource label "
            "exactly matches this value."
        ),
    )
    parser.add_argument(
        "--report-filter-panel-id",
        default=None,
        help=(
            "With query output, only include query report rows whose panel id "
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
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--output-format",
        choices=INSPECT_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Legacy single-flag output selector for inspect output. "
            "Use text, table, json, report-table, report-csv, report-json, "
            "report-tree, report-tree-table, governance, governance-json, "
            "graph-json, graph-dot, graph-governance, or graph-governance-json. "
            "Prefer --view with --format and optional --layout for new usage. "
            "This cannot be combined with hidden legacy output flags."
        ),
    )
    parser.add_argument(
        "--view",
        choices=INSPECT_VIEW_CHOICES,
        default=None,
        help=(
            "Preferred inspect selector for what to render. "
            "Use summary, query, or governance. "
            "Combine with --format and optional --layout instead of legacy --output-format."
        ),
    )
    parser.add_argument(
        "--format",
        choices=INSPECT_FORMAT_CHOICES,
        default=None,
        help=(
            "Preferred inspect selector for output encoding. "
            "Use text, table, csv, or json. "
            "Combine with --view and optional --layout."
        ),
    )
    parser.add_argument(
        "--layout",
        choices=INSPECT_LAYOUT_CHOICES,
        default=None,
        help="Preferred inspect selector for query layout. Use flat or tree with --view query.",
    )
    parser.add_argument(
        "--report-columns",
        default=None,
        help=(
            "With query/table, query/csv, or query/tree/table output, "
            "render only these comma-separated report columns. "
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
            "With query output, only include query report rows whose datasource label "
            "exactly matches this value."
        ),
    )
    parser.add_argument(
        "--report-filter-panel-id",
        default=None,
        help=(
            "With query output, only include query report rows whose panel id "
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


def parse_args(argv: Optional[list[str]] = None) -> argparse.Namespace:
    """Build dashboard CLI parser and normalize mutually-exclusive dashboard subcommand input.

    Flow:
    - Build mode-specific subparsers to enforce one command per execution.
    - Parse arguments, then normalize `--output-format` aliases into concrete mode
      flags.
    - Normalize table column selections for import dry-run output.
    """
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
            "--token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n"
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
            "--token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n"
            "  Export into a flat directory layout instead of per-folder subdirectories:\n"
            "    grafana-util dashboard export --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --export-dir ./dashboards --flat"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(export_parser)
    add_export_cli_args(export_parser)
    add_error_policy_argument(export_parser, "dashboard")

    list_parser = subparsers.add_parser(
        "list-dashboard",
        help="List live dashboard summaries from Grafana.",
        epilog=(
            "Examples:\n\n"
            "  List dashboards in a table:\n"
            "    grafana-util dashboard list --url http://localhost:3000 --table\n\n"
            "  List dashboards with datasource names:\n"
            "    grafana-util dashboard list --url http://localhost:3000 --with-sources --table\n\n"
            "  List dashboards as JSON:\n"
            "    grafana-util dashboard list --url http://localhost:3000 --output-format json"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(list_parser)
    add_list_cli_args(list_parser)

    list_data_sources_parser = subparsers.add_parser(
        "list-data-sources",
        help="List live Grafana data sources.",
        epilog=(
            "Examples:\n\n"
            "  Preferred namespaced form:\n"
            "    grafana-util datasource list --url http://localhost:3000 --table\n\n"
            "  Compatibility dashboard form:\n"
            "    grafana-util dashboard list-data-sources --url http://localhost:3000 --table\n\n"
            "  JSON output:\n"
            "    grafana-util dashboard list-data-sources --url http://localhost:3000 --output-format json"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(list_data_sources_parser)
    add_list_data_sources_cli_args(list_data_sources_parser)

    import_parser = subparsers.add_parser(
        "import-dashboard",
        help="Import dashboards from exported raw JSON files.",
        epilog=(
            "Examples:\n\n"
            "  Preview a dashboard import in table form:\n"
            "    grafana-util dashboard import --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing --dry-run --output-format table\n\n"
            "  Route a combined multi-org export by recorded org ids:\n"
            "    grafana-util dashboard import --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --import-dir ./dashboards --use-export-org --create-missing-orgs --dry-run"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(import_parser)
    add_import_cli_args(import_parser)
    add_error_policy_argument(import_parser, "dashboard")

    diff_parser = subparsers.add_parser(
        "diff",
        help="Compare exported raw dashboards with the current Grafana state.",
        epilog=(
            "Examples:\n\n"
            "  Compare one raw export directory against Grafana:\n"
            "    grafana-util dashboard diff --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --import-dir ./dashboards/raw\n\n"
            "  Override the destination folder UID while diffing one export set:\n"
            "    grafana-util dashboard diff --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --import-dir ./dashboards/raw "
            "--import-folder-uid shared-folder"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(diff_parser)
    add_diff_cli_args(diff_parser)
    add_error_policy_argument(diff_parser, "dashboard")

    promote_plan_parser = subparsers.add_parser(
        "promote-plan",
        help="Build a staged dashboard/datasource promotion plan from local JSON inputs.",
        epilog=(
            "Examples:\n\n"
            "  Build one staged promotion plan:\n"
            "    grafana-util dashboard promote-plan --source-bundle ./source-bundle.json "
            "--target-inventory ./target-inventory.json\n\n"
            "  Render the plan as JSON with datasource remaps:\n"
            "    grafana-util dashboard promote-plan --source-bundle ./source-bundle.json "
            "--target-inventory ./target-inventory.json "
            "--datasource-uid-map-file ./datasource-uid-map.json --output-format json"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_promote_plan_cli_args(promote_plan_parser)

    preflight_plan_parser = subparsers.add_parser(
        "preflight-plan",
        help="Run staged promotion preflight checks from a local promotion plan JSON input.",
        epilog=(
            "Examples:\n\n"
            "  Run one staged preflight review:\n"
            "    grafana-util dashboard preflight-plan --plan-file ./promotion-plan.json\n\n"
            "  Include destination availability data and emit JSON:\n"
            "    grafana-util dashboard preflight-plan --plan-file ./promotion-plan.json "
            "--availability-file ./availability.json --output-format json"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_preflight_plan_cli_args(preflight_plan_parser)

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

    args = parser.parse_args(argv)
    _normalize_output_format_args(args, parser)
    _validate_import_routing_args(args, parser)
    _parse_dashboard_import_output_columns(args, parser)
    return args


INSPECT_EXPORT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Show one machine-readable summary document:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--view summary --format json\n\n"
    "  Show one dependency graph JSON document:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--output-format graph-json\n\n"
    "  Render grouped dashboard-first query tables:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
    "--view query --layout tree --format table\n\n"
    "  Show full inspect help with extended report examples:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --help-full"
)


INSPECT_LIVE_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Inspect live dashboards as a report JSON document:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" "
    "--view query --format json\n\n"
    "  Inspect live dashboards as dependency graph DOT:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" "
    "--output-format graph-dot\n\n"
    "  Filter to one panel in dashboard/panel/query tree output:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" "
    "--view query --layout tree --format text --report-filter-panel-id 7\n\n"
    "  Show full inspect help with extended report examples:\n"
    "    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --help-full"
)


def _normalize_output_format_args(
    args: argparse.Namespace,
    parser: argparse.ArgumentParser,
) -> None:
    """Translate `--output-format` aliases into exclusive list/import output flags."""
    command = getattr(args, "command", None)
    output_format = getattr(args, "output_format", None)
    if command in ("list-dashboard", "list-data-sources"):
        if output_format is None:
            return
        if bool(getattr(args, "table", False)) or bool(getattr(args, "csv", False)) or bool(
            getattr(args, "json", False)
        ):
            parser.error(
                "--output-format cannot be combined with --table, --csv, or --json for dashboard list commands."
            )
        args.table = output_format == "table"
        args.csv = output_format == "csv"
        args.json = output_format == "json"
        return
    if command == "import-dashboard":
        if output_format is None:
            return
        if bool(getattr(args, "table", False)) or bool(getattr(args, "json", False)):
            parser.error(
                "--output-format cannot be combined with --table or --json for import-dashboard."
            )
        args.table = output_format == "table"
        args.json = output_format == "json"
        return
    if command in ("inspect-export", "inspect-live"):
        _normalize_inspect_mode_args(args, parser)


def _normalize_inspect_mode_args(
    args: argparse.Namespace,
    parser: argparse.ArgumentParser,
) -> None:
    """Translate preferred inspect view/format/layout args into the legacy inspect contract."""
    view = getattr(args, "view", None)
    format_name = getattr(args, "format", None)
    layout = getattr(args, "layout", None)

    if view is None and format_name is None and layout is None:
        return

    if (
        getattr(args, "output_format", None) is not None
        or getattr(args, "report", None) is not None
        or bool(getattr(args, "json", False))
        or bool(getattr(args, "table", False))
    ):
        parser.error(
            "--view, --format, and --layout cannot be combined with legacy inspect output flags (--output-format, --report, --json, or --table)."
        )

    normalized_output_format = _resolve_inspect_output_format_from_view_args(
        parser,
        view=view,
        format_name=format_name,
        layout=layout,
    )
    args.output_format = normalized_output_format


def _resolve_inspect_output_format_from_view_args(
    parser: argparse.ArgumentParser,
    *,
    view: Optional[str],
    format_name: Optional[str],
    layout: Optional[str],
) -> str:
    normalized_view = view or "summary"
    normalized_format = format_name or ("text" if normalized_view != "query" else "table")
    normalized_layout = layout or "flat"

    if normalized_view == "summary":
        if layout is not None:
            parser.error("--layout is only supported with --view query.")
        if normalized_format == "text":
            return "text"
        if normalized_format == "table":
            return "table"
        if normalized_format == "json":
            return "json"
        parser.error("--view summary only supports --format text, table, or json.")

    if normalized_view == "query":
        if normalized_layout == "flat":
            if normalized_format == "table":
                return "report-table"
            if normalized_format == "csv":
                return "report-csv"
            if normalized_format == "json":
                return "report-json"
            parser.error(
                "--view query with flat layout only supports --format table, csv, or json."
            )
        if normalized_layout == "tree":
            if normalized_format == "text":
                return "report-tree"
            if normalized_format == "table":
                return "report-tree-table"
            parser.error(
                "--view query with --layout tree only supports --format text or table."
            )
        parser.error("--layout must be flat or tree.")

    if layout is not None:
        parser.error("--layout is only supported with --view query.")

    if normalized_view == "governance":
        if normalized_format in ("text", "table"):
            return "governance"
        if normalized_format == "json":
            return "governance-json"
        parser.error("--view governance only supports --format text, table, or json.")

    parser.error(f"Unsupported inspect view: {normalized_view}.")


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
    if getattr(args, "command", None) != "import-dashboard":
        return
    use_export_org = bool(getattr(args, "use_export_org", False))
    only_org_ids = getattr(args, "only_org_id", None) or []
    if only_org_ids and not use_export_org:
        parser.error("--only-org-id requires --use-export-org for import-dashboard.")
    if bool(getattr(args, "create_missing_orgs", False)) and not use_export_org:
        parser.error("--create-missing-orgs requires --use-export-org for import-dashboard.")
    if use_export_org and getattr(args, "org_id", None):
        parser.error("--use-export-org cannot be combined with --org-id for import-dashboard.")
    if use_export_org and bool(getattr(args, "require_matching_export_org", False)):
        parser.error(
            "--use-export-org cannot be combined with --require-matching-export-org for import-dashboard."
        )


def _load_json_object_file(path_value: str, description: str) -> dict[str, Any]:
    path = Path(path_value)
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s %s: %s" % (description, path, exc)) from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError("Invalid JSON in %s %s: %s" % (description, path, exc)) from exc
    if not isinstance(payload, dict):
        raise GrafanaError("%s must contain a JSON object: %s" % (description, path))
    return payload


def _render_json_document(document: dict[str, Any]) -> int:
    print(json.dumps(document, indent=2, sort_keys=False, ensure_ascii=False))
    return 0

def resolve_auth(args: argparse.Namespace) -> dict[str, str]:
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
    return build_export_workflow_deps_from_runtime(
        {
            "GrafanaError": GrafanaError,
            "DATASOURCE_INVENTORY_FILENAME": DATASOURCE_INVENTORY_FILENAME,
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


def list_data_sources(args: argparse.Namespace) -> int:
    """List live Grafana data sources."""
    return run_list_data_sources(args, build_client=build_client)


def _build_inspection_workflow_deps() -> InspectionWorkflowDeps:
    return build_inspection_workflow_deps_from_runtime(
        {
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


def promote_plan(args: argparse.Namespace) -> int:
    """Build a staged promotion plan from local source/target JSON documents."""
    source_bundle = _load_json_object_file(args.source_bundle, "source bundle")
    target_inventory = _load_json_object_file(args.target_inventory, "target inventory")
    options = {
        "requirePreflight": not bool(getattr(args, "skip_preflight", False)),
        "dashboardUidMap": (
            _load_json_object_file(args.dashboard_uid_map_file, "dashboard UID map")
            if getattr(args, "dashboard_uid_map_file", None)
            else {}
        ),
        "dashboardNameMap": (
            _load_json_object_file(args.dashboard_name_map_file, "dashboard name map")
            if getattr(args, "dashboard_name_map_file", None)
            else {}
        ),
        "datasourceUidMap": (
            _load_json_object_file(args.datasource_uid_map_file, "datasource UID map")
            if getattr(args, "datasource_uid_map_file", None)
            else {}
        ),
        "datasourceNameMap": (
            _load_json_object_file(args.datasource_name_map_file, "datasource name map")
            if getattr(args, "datasource_name_map_file", None)
            else {}
        ),
    }
    document = build_promotion_plan_document(source_bundle, target_inventory, options=options)
    if getattr(args, "output_format", "text") == "json":
        return _render_json_document(document)
    for line in render_promotion_plan_text(document):
        print(line)
    return 0


def preflight_plan(args: argparse.Namespace) -> int:
    """Build a staged preflight check document from a promotion plan JSON file."""
    plan_document = _load_json_object_file(args.plan_file, "promotion plan")
    availability = {}
    if getattr(args, "availability_file", None):
        availability = _load_json_object_file(args.availability_file, "availability document")
    document = build_preflight_check_document(plan_document, availability=availability)
    if getattr(args, "output_format", "text") == "json":
        return _render_json_document(document)
    for line in render_preflight_check_text(document):
        print(line)
    return 0


def _build_import_workflow_deps() -> dict[str, Any]:
    return build_import_workflow_deps_from_runtime(
        {
            "DEFAULT_UNKNOWN_UID": DEFAULT_UNKNOWN_UID,
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
        if args.command == "promote-plan":
            return promote_plan(args)
        if args.command == "preflight-plan":
            return preflight_plan(args)
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
