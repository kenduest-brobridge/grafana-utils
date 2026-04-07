#!/usr/bin/env python3
"""Export or import Grafana dashboards.

Purpose:
- Expose dashboard CLI entrypoints (`export-dashboard`, `list-dashboard`,
  `import-dashboard`, `diff`, and inspect commands) and normalize mode-specific
  arguments before delegating to workflow helpers.

Maintainer overview:
- The tool has two separate export targets with different consumers.
- `raw/` keeps dashboard JSON close to Grafana's API shape so it can round-trip
  back through `POST /api/dashboards/db` and stay on the diff/import replay lane.
- `provisioning/` keeps Grafana file-provisioning dashboards separate from the
  replay lane so provisioning semantics do not get mixed into raw diff/import.
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
- `raw/` exports are the safe input for this script's import and diff modes
- `provisioning/` exports are for Grafana file-provisioning review, not the raw replay lane

Caveats:
- Keep `--output-format` normalization and dry-run column parsing in this facade.
- Avoid moving API behavior from workflow helpers back into the facade layer.
"""

import argparse
import json
import getpass
from pathlib import Path
import sys
from typing import Any, Optional

from .clients.dashboard_client import GrafanaClient
from .auth_staging import AuthConfigError, resolve_cli_auth_from_namespace
from .cli_shared import add_live_connection_args, build_connection_details, dump_document
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
from .dashboards.delete_render import (
    format_live_dashboard_delete_line,
    format_live_folder_delete_line,
    render_dashboard_delete_json,
    render_dashboard_delete_table,
    render_dashboard_delete_text,
)
from .dashboards.delete_support import (
    DELETE_OUTPUT_FORMAT_CHOICES,
    build_delete_plan,
    execute_delete_plan,
    validate_delete_args,
)
from .dashboards.delete_workflow import run_delete_dashboards
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
from .dashboard_authoring import (
    build_dashboard_history_export_document,
    build_dashboard_history_list_document_from_export,
    build_dashboard_history_list_document,
    build_dashboard_review_document,
    build_history_inventory_document,
    clone_live_dashboard,
    fetch_live_dashboard,
    load_dashboard_document,
    load_history_artifacts,
    load_history_export_document,
    patch_dashboard_document,
    publish_dashboard_document,
    preview_dashboard_publish,
    preview_dashboard_history_restore,
    restore_dashboard_history_version,
    run_dashboard_edit_live,
    run_dashboard_serve,
    validate_dashboard_export_tree,
)
from .dashboards.screenshot import (
    capture_dashboard_screenshot as run_capture_dashboard_screenshot,
)
from .http_transport import build_json_http_transport
from .dashboards.variable_inspection import (
    render_dashboard_variable_document,
    inspect_dashboard_variables_with_client,
)
from .dashboard_governance_gate import run_dashboard_governance_gate
from .dashboard_topology import (
    TOPOLOGY_OUTPUT_FORMAT_CHOICES,
    run_dashboard_topology,
)
from .roadmap_workbench import (
    build_dependency_graph_document,
    build_dependency_graph_governance_summary,
)
from . import yaml_compat as yaml

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
    "add_analyze_cli_args",
    "add_clone_live_cli_args",
    "add_edit_live_cli_args",
    "add_fetch_live_cli_args",
    "add_history_export_cli_args",
    "add_history_list_cli_args",
    "add_history_restore_cli_args",
    "add_impact_cli_args",
    "add_list_vars_cli_args",
    "add_raw_to_prompt_cli_args",
    "add_patch_file_cli_args",
    "add_publish_cli_args",
    "add_serve_cli_args",
    "add_review_cli_args",
    "add_validate_export_cli_args",
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
    "fetch_live_dashboard_command",
    "extract_dashboard_object",
    "filter_export_inspection_report_document",
    "format_dashboard_summary_line",
    "format_data_source_line",
    "history_export_command",
    "history_list_command",
    "history_restore_command",
    "import_dashboards",
    "governance_gate_dashboards",
    "impact_command",
    "list_vars_command",
    "raw_to_prompt_command",
    "topology_dashboards",
    "analyze_command",
    "clone_live_dashboard_command",
    "edit_live_dashboard_command",
    "patch_file_command",
    "publish_command",
    "review_command",
    "validate_export_command",
    "inspect_export",
    "inspect_folder_inventory",
    "inspect_live",
    "inspect_vars",
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
    "serve_command",
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
AUTHORING_OUTPUT_FORMAT_CHOICES = ("text", "table", "json", "yaml")
HISTORY_OUTPUT_FORMAT_CHOICES = ("text", "table", "json", "yaml")
VALIDATION_OUTPUT_FORMAT_CHOICES = ("text", "json")
RAW_TO_PROMPT_OUTPUT_FORMAT_CHOICES = ("text", "table", "json", "yaml")
IMPACT_OUTPUT_FORMAT_CHOICES = ("text", "json", "yaml")


def _load_raw_to_prompt_datasource_catalog(source_path: Path, args: argparse.Namespace):
    """Load datasource lookup data for raw-to-prompt conversion."""
    def _records_from_document(document: Any) -> list[dict[str, Any]]:
        if isinstance(document, list):
            return [item for item in document if isinstance(item, dict)]
        if isinstance(document, dict):
            for key in ("datasources", "items", "mapping"):
                value = document.get(key)
                if isinstance(value, list):
                    return [item for item in value if isinstance(item, dict)]
                if isinstance(value, dict):
                    return [item for item in value.values() if isinstance(item, dict)]
            return [item for item in document.values() if isinstance(item, dict)]
        return []

    for candidate in (source_path, *source_path.parents):
        datasources_path = candidate / DATASOURCE_INVENTORY_FILENAME
        if not datasources_path.is_file():
            continue
        datasources = load_json_file(datasources_path)
        records = _records_from_document(datasources)
        if records:
            return build_datasource_catalog(records)

    datasource_map_path = getattr(args, "datasource_map", None)
    if datasource_map_path:
        map_path = Path(datasource_map_path)
        if map_path.is_file():
            if map_path.suffix.lower() in {".yaml", ".yml"}:
                raw_map = yaml.safe_load(map_path.read_text(encoding="utf-8"))
            else:
                raw_map = load_json_file(map_path)
            records = _records_from_document(raw_map)
            if records:
                return build_datasource_catalog(records)

    if _raw_to_prompt_live_lookup_requested(args):
        details = build_connection_details(args)
        client = GrafanaClient(
            base_url=details.url,
            headers=details.headers,
            timeout=details.timeout,
            verify_ssl=bool(details.verify_ssl or details.ca_cert),
            ca_cert=details.ca_cert,
        )
        if getattr(args, "org_id", None):
            client = client.with_org_id(args.org_id)
        return build_datasource_catalog(client.list_datasources())

    return build_datasource_catalog([])


def _raw_to_prompt_live_lookup_requested(args: argparse.Namespace) -> bool:
    """Return whether raw-to-prompt should query live Grafana for datasource lookup."""

    return any(
        [
            getattr(args, "profile", None),
            getattr(args, "url", None),
            getattr(args, "api_token", None),
            getattr(args, "username", None),
            getattr(args, "password", None),
            bool(getattr(args, "prompt_password", False)),
            bool(getattr(args, "prompt_token", False)),
            getattr(args, "org_id", None) is not None,
            getattr(args, "timeout", None) is not None,
            bool(getattr(args, "verify_ssl", False)),
        ]
    )


def _read_text_document(path: Path) -> Any:
    """Load one JSON or YAML-compatible document from disk."""

    if path.suffix.lower() in {".yaml", ".yml"}:
        return yaml.safe_load(path.read_text(encoding="utf-8"))
    return load_json_file(path)


def _normalize_document_text(value: Any, default: str = "") -> str:
    """Normalize one document field into a stable string."""

    text = str(value or "").strip()
    return text or default


def _build_raw_to_prompt_index_item(
    input_path: Path,
    output_path: Path,
    document: dict[str, Any],
    org_name: Optional[str],
    org_id: Optional[str],
) -> dict[str, str]:
    """Build one raw-to-prompt index item for prompt lane metadata."""

    dashboard = extract_dashboard_object(
        document, "Dashboard payload must be a JSON object."
    )
    folder_title = _normalize_document_text(
        dashboard.get("folderTitle") or (document.get("meta") or {}).get("folderTitle"),
        DEFAULT_FOLDER_TITLE,
    )
    uid = _normalize_document_text(dashboard.get("uid"), output_path.stem)
    if uid.endswith(".prompt"):
        uid = uid[: -len(".prompt")]
    return {
        "uid": uid,
        "title": _normalize_document_text(dashboard.get("title"), DEFAULT_DASHBOARD_TITLE),
        "folder": folder_title,
        "org": _normalize_document_text(org_name, DEFAULT_ORG_NAME),
        "orgId": _normalize_document_text(org_id, DEFAULT_ORG_ID),
        "prompt_path": str(output_path),
    }


def _resolve_raw_to_prompt_input_dir(input_dir: Path) -> Path:
    """Resolve the dashboard directory inside one raw export tree."""

    if (input_dir / RAW_EXPORT_SUBDIR).is_dir():
        return input_dir / RAW_EXPORT_SUBDIR
    if input_dir.name == RAW_EXPORT_SUBDIR:
        return input_dir
    return input_dir


def _resolve_raw_to_prompt_output_root(
    input_dir: Path,
    output_dir: Optional[str],
) -> Optional[Path]:
    """Resolve the prompt export root for one raw export tree."""

    if output_dir is not None:
        return Path(output_dir)
    if (input_dir / RAW_EXPORT_SUBDIR).is_dir():
        return input_dir / PROMPT_EXPORT_SUBDIR
    if input_dir.name == RAW_EXPORT_SUBDIR:
        return input_dir.parent / PROMPT_EXPORT_SUBDIR
    return None


def _raw_to_prompt_single_output_path(
    input_path: Path,
    output_file: Optional[str],
    output_dir: Optional[str],
) -> Path:
    """Resolve one output path for single-file raw-to-prompt conversions."""

    if output_file is not None:
        return Path(output_file)
    if output_dir is not None:
        return Path(output_dir) / input_path.name
    return input_path.with_name(f"{input_path.stem}.prompt.json")


def _build_raw_to_prompt_document(
    input_path: Path,
    datasource_catalog: tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]],
) -> dict[str, Any]:
    """Convert one raw dashboard file into Grafana prompt-import format."""

    payload = load_json_file(input_path)
    return build_external_export_document(payload, datasource_catalog)


def _build_impact_document(
    governance_path: Path,
    queries_path: Path,
    datasource_uid: str,
) -> dict[str, Any]:
    """Build one datasource blast-radius document from inspect artifacts."""

    governance_document = load_json_file(governance_path)
    queries_document = load_json_file(queries_path)
    graph_document = build_dependency_graph_document(
        governance_document,
        queries_document,
    )
    blast_radius_document = build_dependency_graph_governance_summary(graph_document)
    candidates = list(blast_radius_document.get("datasourceBlastRadius") or [])
    for item in candidates:
        if not isinstance(item, dict):
            continue
        if str(item.get("datasourceUid") or "").strip() == datasource_uid:
            return {
                "kind": "grafana-utils-dashboard-impact",
                "datasourceUid": datasource_uid,
                "datasource": item.get("datasource"),
                "datasourceType": item.get("datasourceType"),
                "dashboardCount": item.get("dashboardCount"),
                "panelCount": item.get("panelCount"),
                "dashboardNodeIds": item.get("dashboardNodeIds"),
                "panelNodeIds": item.get("panelNodeIds"),
                "summary": blast_radius_document.get("summary") or {},
            }
    raise GrafanaError(
        f"Datasource UID {datasource_uid!r} was not found in the impact graph."
    )


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
        "--ca-cert",
        default=None,
        help=(
            "CA certificate path for TLS verification. Use this when Grafana is "
            "served with a private or internal CA."
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


def add_delete_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add delete cli args implementation."""
    input_group = parser.add_argument_group("Input Options")
    target_group = parser.add_argument_group("Target Options")
    safety_group = parser.add_argument_group("Safety Options")
    output_group = parser.add_argument_group("Output Options")
    input_group.add_argument(
        "--page-size",
        type=int,
        default=DEFAULT_PAGE_SIZE,
        help=f"Dashboard search page size used to resolve delete selectors (default: {DEFAULT_PAGE_SIZE}).",
    )
    target_group.add_argument(
        "--org-id",
        default=None,
        help="Delete dashboards from this explicit Grafana organization ID instead of the current org context.",
    )
    target_group.add_argument(
        "--uid",
        default=None,
        help="Dashboard UID to delete.",
    )
    target_group.add_argument(
        "--path",
        default=None,
        help="Grafana folder path root to delete recursively, for example 'Platform / Infra'.",
    )
    target_group.add_argument(
        "--delete-folders",
        action="store_true",
        help="With --path, also delete matched Grafana folders after deleting dashboards in the subtree.",
    )
    safety_group.add_argument(
        "--yes",
        action="store_true",
        help="Explicit acknowledgement required before live dashboard delete runs. Not required with --dry-run or --interactive.",
    )
    safety_group.add_argument(
        "--interactive",
        action="store_true",
        help="Prompt for the delete selector, preview the delete plan, and confirm interactively.",
    )
    safety_group.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview what dashboard delete would do without changing Grafana.",
    )
    output_group.add_argument(
        "--table",
        action="store_true",
        help="For --dry-run only, render delete targets in table form.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="For --dry-run only, render one JSON document with delete targets and summary counts.",
    )
    output_group.add_argument(
        "--no-header",
        action="store_true",
        help="For --dry-run --table only, omit the table header row.",
    )
    output_group.add_argument(
        "--output-format",
        choices=DELETE_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for dashboard delete dry-run output. "
            "Use text, table, or json. This cannot be combined with --table or --json."
        ),
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
            "Compare dashboards from this raw export directory against Grafana. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly; "
            "use inspect-export --input-format provisioning for Grafana file-provisioning trees."
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
        help="Grafana variable query-string fragment, for example 'var-env=prod&var-host=web01'. This overlays current values in list-vars output.",
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
        help="Do not print table or CSV headers when rendering list-vars output.",
    )
    output_group.add_argument(
        "--output-file",
        default=None,
        help="Write list-vars output to this file while still printing to stdout.",
    )


def add_governance_gate_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add governance gate cli args implementation."""
    parser.add_argument(
        "--policy",
        required=True,
        help="Path to the governance policy JSON.",
    )
    parser.add_argument(
        "--governance",
        required=True,
        help="Path to dashboard inspect governance-json output.",
    )
    parser.add_argument(
        "--queries",
        required=True,
        help="Path to dashboard inspect report json output.",
    )
    parser.add_argument(
        "--import-dir",
        default=None,
        help=(
            "Optional raw dashboard export directory. Provide this when policy rules need "
            "plugin or templating-variable checks in addition to inspect JSON."
        ),
    )
    parser.add_argument(
        "--output-format",
        choices=("text", "json"),
        default="text",
        help="Render the gate result as text or JSON.",
    )
    parser.add_argument(
        "--json-output",
        default=None,
        help="Optional path to also write the normalized gate result JSON.",
    )


def add_topology_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add topology cli args implementation."""
    parser.add_argument(
        "--governance",
        required=True,
        help="Path to dashboard governance JSON.",
    )
    parser.add_argument(
        "--queries",
        default=None,
        help="Optional path to dashboard query-report JSON.",
    )
    parser.add_argument(
        "--alert-contract",
        default=None,
        help="Optional path to dashboard alert contract JSON.",
    )
    parser.add_argument(
        "--output-format",
        choices=TOPOLOGY_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render topology as text, json, mermaid, or dot (default: text).",
    )
    parser.add_argument(
        "--output-file",
        default=None,
        help="Write topology output to this file while also printing to stdout.",
    )
    parser.add_argument(
        "--interactive",
        action="store_true",
        help="Open an interactive terminal browser over the topology nodes and edges.",
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


def add_fetch_live_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add fetch-live cli args implementation."""
    add_common_cli_args(parser)
    target_group = parser.add_argument_group("Target Options")
    output_group = parser.add_argument_group("Output Options")
    target_group.add_argument(
        "--dashboard-uid",
        required=True,
        help="Live Grafana dashboard UID to fetch.",
    )
    output_group.add_argument(
        "--output",
        required=True,
        help="Write the fetched dashboard draft to this file path.",
    )


def add_clone_live_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add clone-live cli args implementation."""
    add_common_cli_args(parser)
    target_group = parser.add_argument_group("Target Options")
    output_group = parser.add_argument_group("Output Options")
    target_group.add_argument(
        "--source-uid",
        required=True,
        help="Live Grafana dashboard UID to clone.",
    )
    target_group.add_argument(
        "--name",
        default=None,
        help="Override the cloned dashboard title. Defaults to the source title.",
    )
    target_group.add_argument(
        "--uid",
        default=None,
        help="Override the cloned dashboard UID. Defaults to the source UID.",
    )
    target_group.add_argument(
        "--folder-uid",
        default=None,
        help="Override the cloned dashboard folder UID.",
    )
    output_group.add_argument(
        "--output",
        required=True,
        help="Write the cloned dashboard draft to this file path.",
    )


def add_edit_live_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add edit-live cli args implementation."""
    add_common_cli_args(parser)
    parser.add_argument(
        "--dashboard-uid",
        required=True,
        help="Live Grafana dashboard UID to edit.",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Write the edited dashboard draft to this file path instead of using ./<uid>.edited.json.",
    )
    parser.add_argument(
        "--apply-live",
        action="store_true",
        help="Apply the edited dashboard back to Grafana immediately instead of writing a local draft file.",
    )
    parser.add_argument(
        "--message",
        default="Imported by grafana-utils",
        help="Revision message to use when --apply-live writes the edited dashboard back to Grafana.",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Acknowledge the live writeback when --apply-live is set.",
    )


def add_serve_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add serve cli args implementation."""
    input_group = parser.add_mutually_exclusive_group(required=True)
    input_group.add_argument(
        "--input",
        default=None,
        help="Load one dashboard draft file or a directory of dashboard draft files into the preview server.",
    )
    input_group.add_argument(
        "--script",
        default=None,
        help="Run this local script and treat stdout as one dashboard document or an array of dashboard documents.",
    )
    parser.add_argument(
        "--script-format",
        choices=("json", "yaml"),
        default="json",
        help="Interpret --script stdout as json or yaml.",
    )
    parser.add_argument(
        "--address",
        default="127.0.0.1",
        help="Address for the local preview server to bind.",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8080,
        help="Port for the local preview server to bind.",
    )
    parser.add_argument(
        "--no-watch",
        action="store_true",
        help="Do not watch input paths for changes after the initial preview is loaded.",
    )
    parser.add_argument(
        "--watch",
        action="append",
        default=None,
        help="Extra local paths to watch for preview reloads. Repeat --watch for multiple paths.",
    )
    parser.add_argument(
        "--open-browser",
        action="store_true",
        help="Open the preview URL in your default browser after the server starts.",
    )


def add_patch_file_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add patch-file cli args implementation."""
    input_group = parser.add_argument_group("Input Options")
    mutation_group = parser.add_argument_group("Mutation Options")
    input_group.add_argument(
        "--input",
        required=True,
        help="Input dashboard JSON file to patch. Use - to read from standard input.",
    )
    input_group.add_argument(
        "--output",
        default=None,
        help="Write the patched JSON to this path instead of overwriting --input in place.",
    )
    mutation_group.add_argument("--name", default=None, help="Replace dashboard.title.")
    mutation_group.add_argument("--uid", default=None, help="Replace dashboard.uid.")
    mutation_group.add_argument(
        "--folder-uid",
        default=None,
        help="Set folder UID metadata for later publish/import runs.",
    )
    mutation_group.add_argument(
        "--message",
        default=None,
        help="Store a human-readable note in meta.message.",
    )
    mutation_group.add_argument(
        "--tag",
        dest="tags",
        action="append",
        default=None,
        help="Replace dashboard.tags. Repeat to set multiple tags.",
    )


def add_review_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add review cli args implementation."""
    parser.add_argument(
        "--input",
        required=True,
        help="Input dashboard JSON file to review locally. Use - to read from standard input.",
    )
    parser.add_argument(
        "--output-format",
        choices=AUTHORING_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render the review as text, table, json, or yaml.",
    )


def add_publish_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add publish cli args implementation."""
    add_common_cli_args(parser)
    parser.add_argument(
        "--input",
        required=True,
        help="Dashboard JSON file to publish. Use - to read from standard input.",
    )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="Update an existing dashboard when the UID already exists.",
    )
    parser.add_argument(
        "--folder-uid",
        default=None,
        help="Override the destination Grafana folder UID for this publish.",
    )
    parser.add_argument(
        "--message",
        default="Imported by grafana-utils",
        help="Version-history message to attach to the published dashboard revision.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview the publish through the import dry-run flow without changing Grafana.",
    )
    parser.add_argument(
        "--approve",
        action="store_true",
        help="Acknowledge the live publish. Required unless --dry-run is set.",
    )
    parser.add_argument(
        "--output-format",
        choices=AUTHORING_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render the dry-run preview as text, table, json, or yaml.",
    )


def add_history_list_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add history list cli args implementation."""
    add_common_cli_args(parser)
    parser.add_argument(
        "--dashboard-uid",
        default=None,
        help="Dashboard UID to inspect. Required for live Grafana history.",
    )
    parser.add_argument(
        "--input",
        default=None,
        help="Read one local history artifact JSON instead of calling Grafana.",
    )
    parser.add_argument(
        "--input-dir",
        default=None,
        help="Read history artifacts from a dashboard export root instead of calling Grafana.",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=20,
        help="Maximum number of recent versions to request from Grafana in live mode.",
    )
    parser.add_argument(
        "--output-format",
        choices=HISTORY_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render history as text, table, json, or yaml.",
    )


def add_history_export_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add history export cli args implementation."""
    add_common_cli_args(parser)
    parser.add_argument(
        "--dashboard-uid",
        required=True,
        help="Dashboard UID to export from Grafana history.",
    )
    parser.add_argument(
        "--output",
        required=True,
        help="Write the exported dashboard history artifact to this JSON file.",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=20,
        help="Maximum number of recent versions to include in the exported history artifact.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite an existing history artifact file.",
    )


def add_history_restore_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add history restore cli args implementation."""
    add_common_cli_args(parser)
    parser.add_argument(
        "--dashboard-uid",
        required=True,
        help="Dashboard UID to restore from Grafana history.",
    )
    parser.add_argument(
        "--version",
        type=int,
        required=True,
        help="Dashboard history version number to restore.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview the restore without writing a new Grafana revision.",
    )
    parser.add_argument(
        "--output-format",
        choices=HISTORY_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render restore preview or result as text, table, json, or yaml.",
    )
    parser.add_argument(
        "--message",
        default=None,
        help="Revision message to attach to the new Grafana revision.",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Confirm the live restore. Required unless --dry-run is set.",
    )


def add_analyze_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add analyze cli args implementation."""
    add_common_cli_args(parser)
    parser.add_argument(
        "--input-dir",
        default=None,
        help="Analyze dashboards from this directory instead of live Grafana.",
    )
    parser.add_argument(
        "--input-format",
        choices=("raw", "provisioning"),
        default="raw",
        help="Interpret --input-dir as raw export files or Grafana file-provisioning artifacts.",
    )
    parser.add_argument(
        "--page-size",
        type=int,
        default=DEFAULT_PAGE_SIZE,
        help="Dashboard search page size when analyze reads live Grafana.",
    )
    parser.add_argument(
        "--all-orgs",
        action="store_true",
        help="Enumerate all visible Grafana orgs and analyze dashboards across them.",
    )
    parser.add_argument(
        "--report-columns",
        default=None,
        help="Limit the query report to selected columns for table-like output.",
    )
    parser.add_argument(
        "--report-filter-datasource",
        default=None,
        help="Include only rows whose datasource label, uid, type, or family matches this value.",
    )
    parser.add_argument(
        "--report-filter-panel-id",
        default=None,
        help="Include only rows whose panel id matches this value.",
    )
    parser.add_argument(
        "--output-format",
        choices=INSPECT_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render the dashboard analysis as text, table, json, yaml, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="Do not print headers when rendering table-like analysis output.",
    )
    parser.add_argument(
        "--output-file",
        default=None,
        help="Write analysis output to this file.",
    )


def add_validate_export_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add validate-export cli args implementation."""
    parser.add_argument(
        "--input-dir",
        required=True,
        help="Validate dashboards from this export directory.",
    )
    parser.add_argument(
        "--input-format",
        choices=("raw", "provisioning"),
        default="raw",
        help="Interpret --input-dir as raw export files or Grafana file-provisioning artifacts.",
    )
    parser.add_argument(
        "--output-format",
        choices=VALIDATION_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render the validation result as text or JSON.",
    )
    parser.add_argument(
        "--output-file",
        default=None,
        help="Optional path to also write the validation JSON result.",
    )


def add_raw_to_prompt_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add raw-to-prompt cli args implementation."""
    add_live_connection_args(parser)
    input_group = parser.add_mutually_exclusive_group(required=True)
    output_group = parser.add_argument_group("Output Options")
    mapping_group = parser.add_argument_group("Mapping Options")
    input_group.add_argument(
        "--input-file",
        action="append",
        default=[],
        help="Repeat this flag for each raw dashboard file to convert. When omitted, use --input-dir.",
    )
    input_group.add_argument(
        "--input-dir",
        default=None,
        help="Convert every raw dashboard file in this directory. Point this at a raw export root or a raw/ lane.",
    )
    output_group.add_argument(
        "--output-file",
        default=None,
        help="Write one converted prompt document to this file path.",
    )
    output_group.add_argument(
        "--output-dir",
        default=None,
        help="Write converted prompt artifacts into this directory.",
    )
    output_group.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing output files instead of failing when the target already exists.",
    )
    output_group.add_argument(
        "--output-format",
        choices=RAW_TO_PROMPT_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render the command summary as text, table, json, or yaml.",
    )
    output_group.add_argument(
        "--no-header",
        action="store_true",
        help="Do not print table headers when rendering table output.",
    )
    output_group.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview the conversion without writing files.",
    )
    output_group.add_argument(
        "--progress",
        action="store_true",
        help="Show concise per-item conversion progress while processing files.",
    )
    output_group.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show detailed per-item conversion output. Overrides --progress output.",
    )
    mapping_group.add_argument(
        "--datasource-map",
        default=None,
        help="Optional datasource mapping file used while resolving prompt output.",
    )
    mapping_group.add_argument(
        "--resolution",
        choices=("infer-family", "exact", "strict"),
        default="infer-family",
        help="Choose how datasource references are resolved. Use infer-family, exact, or strict.",
    )
    output_group.add_argument(
        "--log-file",
        default=None,
        help="Write structured conversion logs to this file.",
    )
    output_group.add_argument(
        "--log-format",
        choices=("text", "json"),
        default="text",
        help="Render logs as text or json.",
    )


def add_list_vars_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add list-vars cli args implementation."""
    add_inspect_vars_cli_args(parser)


def add_impact_cli_args(parser: argparse.ArgumentParser) -> None:
    """Add impact cli args implementation."""
    parser.add_argument(
        "--governance",
        required=True,
        help="Path to dashboard governance JSON.",
    )
    parser.add_argument(
        "--queries",
        required=True,
        help="Path to dashboard query-report JSON.",
    )
    parser.add_argument(
        "--datasource-uid",
        required=True,
        help="Datasource UID whose downstream impact should be summarized.",
    )
    parser.add_argument(
        "--output-format",
        choices=IMPACT_OUTPUT_FORMAT_CHOICES,
        default="text",
        help="Render the impact summary as text, json, or yaml.",
    )
    parser.add_argument(
        "--output-file",
        default=None,
        help="Write impact output to this file while still printing to stdout.",
    )


def fetch_live_dashboard_command(args: argparse.Namespace) -> int:
    """Fetch one live dashboard into a local draft file."""
    client = build_client(args)
    if getattr(args, "org_id", None):
        client = client.with_org_id(args.org_id)
    dashboard = fetch_live_dashboard(client, args.dashboard_uid)
    output_path = Path(args.output)
    if output_path.parent:
        output_path.parent.mkdir(parents=True, exist_ok=True)
    write_json_document(dashboard, output_path)
    dump_document(
        {
            "kind": "grafana-utils-dashboard-fetch-live",
            "dashboardUid": args.dashboard_uid,
            "output": str(output_path),
        },
        "text",
    )
    return 0


def clone_live_dashboard_command(args: argparse.Namespace) -> int:
    """Clone one live dashboard into a local draft file."""
    client = build_client(args)
    if getattr(args, "org_id", None):
        client = client.with_org_id(args.org_id)
    dashboard = clone_live_dashboard(
        client,
        args.source_uid,
        name=getattr(args, "name", None),
        uid=getattr(args, "uid", None),
        folder_uid=getattr(args, "folder_uid", None),
    )
    output_path = Path(args.output)
    if output_path.parent:
        output_path.parent.mkdir(parents=True, exist_ok=True)
    write_json_document(dashboard, output_path)
    dump_document(
        {
            "kind": "grafana-utils-dashboard-clone-live",
            "sourceUid": args.source_uid,
            "output": str(output_path),
        },
        "text",
    )
    return 0


def edit_live_dashboard_command(args: argparse.Namespace) -> int:
    """Edit one live dashboard in an external editor and optionally apply it live."""
    client = build_client(args)
    if getattr(args, "org_id", None):
        client = client.with_org_id(args.org_id)
    return run_dashboard_edit_live(client, args)


def serve_command(args: argparse.Namespace) -> int:
    """Run one local dashboard preview server."""
    return run_dashboard_serve(args)


def patch_file_command(args: argparse.Namespace) -> int:
    """Patch one local dashboard JSON file in place or to a new path."""
    source_path = Path(args.input)
    document = load_dashboard_document(source_path)
    patched = patch_dashboard_document(
        document,
        name=getattr(args, "name", None),
        uid=getattr(args, "uid", None),
        folder_uid=getattr(args, "folder_uid", None),
        message=getattr(args, "message", None),
        tags=getattr(args, "tags", None),
    )
    output = getattr(args, "output", None)
    output_path = Path(output) if output else source_path
    if output_path.parent:
        output_path.parent.mkdir(parents=True, exist_ok=True)
    write_json_document(patched, output_path)
    dump_document(
        {
            "kind": "grafana-utils-dashboard-patch-file",
            "input": str(source_path),
            "output": str(output_path),
        },
        "text",
    )
    return 0


def review_command(args: argparse.Namespace) -> int:
    """Review one local dashboard JSON file without touching Grafana."""
    document = load_dashboard_document(Path(args.input))
    review_document = build_dashboard_review_document(document)
    dump_document(review_document, getattr(args, "output_format", "text"))
    return 0


def publish_command(args: argparse.Namespace) -> int:
    """Publish one local dashboard JSON file through the import pipeline."""
    client = build_client(args)
    if getattr(args, "org_id", None):
        client = client.with_org_id(args.org_id)
    document = load_dashboard_document(Path(args.input))
    if bool(getattr(args, "dry_run", False)):
        preview = preview_dashboard_publish(
            client,
            document,
            replace_existing=bool(getattr(args, "replace_existing", False)),
            message=getattr(args, "message", ""),
            folder_uid=getattr(args, "folder_uid", None),
        )
        dump_document(preview, getattr(args, "output_format", "text"))
        return 0
    if not bool(getattr(args, "approve", False)):
        raise GrafanaError("Dashboard publish requires --approve unless --dry-run is active.")
    response = publish_dashboard_document(
        client,
        document,
        replace_existing=bool(getattr(args, "replace_existing", False)),
        message=getattr(args, "message", ""),
        folder_uid=getattr(args, "folder_uid", None),
    )
    dump_document(response, getattr(args, "output_format", "text"))
    return 0


def analyze_command(args: argparse.Namespace) -> int:
    """Analyze dashboards from live Grafana or a local export tree."""
    if getattr(args, "input_dir", None):
        inspect_args = argparse.Namespace(
            import_dir=args.input_dir,
            report=None,
            output_format=args.output_format,
            output_file=getattr(args, "output_file", None),
            report_columns=getattr(args, "report_columns", None),
            report_filter_datasource=getattr(args, "report_filter_datasource", None),
            report_filter_panel_id=getattr(args, "report_filter_panel_id", None),
            json=bool(args.output_format == "json"),
            table=bool(args.output_format == "table"),
            no_header=bool(getattr(args, "no_header", False)),
        )
        return inspect_export(inspect_args)
    inspect_args = argparse.Namespace(
        common=args,
        page_size=getattr(args, "page_size", DEFAULT_PAGE_SIZE),
        org_id=getattr(args, "org_id", None),
        all_orgs=bool(getattr(args, "all_orgs", False)),
        text=bool(args.output_format == "text"),
        table=bool(args.output_format == "table"),
        csv=bool(args.output_format == "csv"),
        json=bool(args.output_format == "json"),
        yaml=bool(args.output_format == "yaml"),
        output_format=args.output_format,
        report_columns=getattr(args, "report_columns", None),
        report_filter_datasource=getattr(args, "report_filter_datasource", None),
        report_filter_panel_id=getattr(args, "report_filter_panel_id", None),
        progress=False,
        help_full=False,
        no_header=bool(getattr(args, "no_header", False)),
        output_file=getattr(args, "output_file", None),
        also_stdout=False,
        interactive=bool(getattr(args, "interactive", False)),
    )
    return inspect_live(inspect_args)


def validate_export_command(args: argparse.Namespace) -> int:
    """Validate one dashboard export tree without mutating Grafana."""
    document = validate_dashboard_export_tree(Path(args.input_dir))
    output_file = getattr(args, "output_file", None)
    if output_file:
        output_path = Path(output_file)
        if output_path.parent:
            output_path.parent.mkdir(parents=True, exist_ok=True)
        write_json_document(document, output_path)
    dump_document(document, getattr(args, "output_format", "text"))
    return 0


def history_list_command(args: argparse.Namespace) -> int:
    """List live dashboard revision history or review local history artifacts."""
    client = build_client(args)
    if getattr(args, "org_id", None):
        client = client.with_org_id(args.org_id)
    if getattr(args, "input", None):
        document = load_history_export_document(Path(args.input))
        dump_document(
            build_dashboard_history_list_document_from_export(document),
            getattr(args, "output_format", "text"),
        )
        return 0
    if getattr(args, "input_dir", None):
        artifacts = load_history_artifacts(Path(args.input_dir))
        if getattr(args, "dashboard_uid", None):
            artifacts = [
                item for item in artifacts if item[1].get("dashboardUid") == args.dashboard_uid
            ]
        document = build_history_inventory_document(Path(args.input_dir), artifacts)
        dump_document(document, getattr(args, "output_format", "text"))
        return 0
    if not getattr(args, "dashboard_uid", None):
        raise GrafanaError(
            "Dashboard history list requires --dashboard-uid unless --input or --input-dir is set."
        )
    document = build_dashboard_history_list_document(
        client,
        args.dashboard_uid,
        int(args.limit),
    )
    dump_document(document, getattr(args, "output_format", "text"))
    return 0


def history_export_command(args: argparse.Namespace) -> int:
    """Export dashboard revision history into a reusable JSON artifact."""
    client = build_client(args)
    if getattr(args, "org_id", None):
        client = client.with_org_id(args.org_id)
    output_path = Path(args.output)
    if output_path.exists() and not bool(getattr(args, "overwrite", False)):
        raise GrafanaError(
            f"Refusing to overwrite existing file: {output_path}. Use --overwrite."
        )
    document = build_dashboard_history_export_document(
        client,
        args.dashboard_uid,
        int(args.limit),
    )
    write_json_document(document, output_path)
    dump_document(
        {
            "kind": document["kind"],
            "dashboardUid": document["dashboardUid"],
            "output": str(output_path),
        },
        "text",
    )
    return 0


def history_restore_command(args: argparse.Namespace) -> int:
    """Restore one historical dashboard version as a new latest revision entry."""
    client = build_client(args)
    if getattr(args, "org_id", None):
        client = client.with_org_id(args.org_id)
    if bool(getattr(args, "dry_run", False)):
        preview = preview_dashboard_history_restore(
            client,
            args.dashboard_uid,
            int(args.version),
            message=getattr(args, "message", None),
        )
        dump_document(preview, getattr(args, "output_format", "text"))
        return 0
    if not bool(getattr(args, "yes", False)):
        raise GrafanaError("Dashboard history restore requires --yes unless --dry-run is set.")
    document = restore_dashboard_history_version(
        client,
        args.dashboard_uid,
        int(args.version),
        message=getattr(args, "message", None),
    )
    dump_document(document, getattr(args, "output_format", "text"))
    return 0


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
            "  Edit one live dashboard through your editor:\n"
            "    grafana-util dashboard edit-live --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main\n\n"
            "  Compare raw dashboard exports against local Grafana:\n"
            "    grafana-util dashboard diff --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --import-dir ./dashboards/raw\n\n"
            "  Run a local dashboard preview server:\n"
            "    grafana-util dashboard serve --input ./dashboards/raw --open-browser\n\n"
            "  Inspect a Grafana file-provisioning tree separately:\n"
            "    grafana-util dashboard inspect-export --import-dir ./dashboards/provisioning "
            "--input-format provisioning --report tree-table"
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

    raw_to_prompt_parser = subparsers.add_parser(
        "raw-to-prompt",
        help="Convert raw dashboard JSON into prompt-lane artifacts.",
        epilog=(
            "Examples:\n\n"
            "  Convert one raw dashboard file into the sibling .prompt.json path:\n"
            "    grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json\n\n"
            "  Convert a raw export tree into a sibling prompt/ lane:\n"
            "    grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite\n\n"
            "  Convert raw dashboard JSON with live datasource lookup from a profile:\n"
            "    grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --profile prod --org-id 2"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_raw_to_prompt_cli_args(raw_to_prompt_parser)

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

    delete_parser = subparsers.add_parser(
        "delete-dashboard",
        help="Delete live dashboards by UID or folder path.",
        epilog=DELETE_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_cli_args(delete_parser)
    add_delete_cli_args(delete_parser)

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
        "list-vars",
        aliases=["inspect-vars"],
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
    topology_parser = subparsers.add_parser(
        "topology",
        aliases=["graph"],
        help="Build a deterministic dashboard topology graph from JSON artifacts.",
        epilog=TOPOLOGY_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_topology_cli_args(topology_parser)
    governance_gate_parser = subparsers.add_parser(
        "governance-gate",
        help="Evaluate a dashboard governance policy against inspect JSON artifacts.",
        epilog=GOVERNANCE_GATE_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_governance_gate_cli_args(governance_gate_parser)

    impact_parser = subparsers.add_parser(
        "impact",
        help="Summarize which dashboards and alerts would be affected by one datasource.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_impact_cli_args(impact_parser)

    fetch_live_parser = subparsers.add_parser(
        "fetch-live",
        help="Fetch one live dashboard into a local draft file.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_fetch_live_cli_args(fetch_live_parser)

    clone_live_parser = subparsers.add_parser(
        "clone-live",
        help="Clone one live dashboard into a local draft file.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_clone_live_cli_args(clone_live_parser)

    edit_live_parser = subparsers.add_parser(
        "edit-live",
        help="Edit one live dashboard in an external editor and optionally apply it live.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_edit_live_cli_args(edit_live_parser)

    patch_file_parser = subparsers.add_parser(
        "patch-file",
        help="Patch one local dashboard JSON file in place or to a new path.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_patch_file_cli_args(patch_file_parser)

    serve_parser = subparsers.add_parser(
        "serve",
        help="Run one local dashboard preview server.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_serve_cli_args(serve_parser)

    review_parser = subparsers.add_parser(
        "review",
        help="Review one local dashboard JSON file without touching Grafana.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_review_cli_args(review_parser)

    publish_parser = subparsers.add_parser(
        "publish",
        help="Publish one local dashboard JSON file through the import pipeline.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_publish_cli_args(publish_parser)

    analyze_parser = subparsers.add_parser(
        "analyze",
        help="Analyze dashboards from live Grafana or a local export tree.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_analyze_cli_args(analyze_parser)

    validate_export_parser = subparsers.add_parser(
        "validate-export",
        help="Validate one dashboard export tree without mutating Grafana.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_validate_export_cli_args(validate_export_parser)

    history_parser = subparsers.add_parser(
        "history",
        help="List, export, or restore dashboard revision history.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    history_subparsers = history_parser.add_subparsers(dest="history_command")
    history_subparsers.required = True
    history_list_parser = history_subparsers.add_parser(
        "list",
        help="List live dashboard revision history or review local history artifacts.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_history_list_cli_args(history_list_parser)
    history_export_parser = history_subparsers.add_parser(
        "export",
        help="Export dashboard revision history into a reusable JSON artifact.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_history_export_cli_args(history_export_parser)
    history_restore_parser = history_subparsers.add_parser(
        "restore",
        help="Restore one historical dashboard version as a new latest revision entry.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_history_restore_cli_args(history_restore_parser)

    args = parser.parse_args(argv)
    if args.command == "graph":
        args.command = "topology"
    if args.command == "inspect-vars":
        args.command = "list-vars"
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

DELETE_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Dry-run one dashboard delete by UID:\n"
    '    grafana-util dashboard delete-dashboard --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --uid cpu-main --dry-run --json\n\n'
    "  Delete all dashboards under one folder subtree:\n"
    '    grafana-util dashboard delete-dashboard --url http://localhost:3000 --basic-user admin --basic-password admin --path "Platform / Infra" --yes\n\n'
    "  Interactively preview and confirm a folder delete:\n"
    "    grafana-util dashboard delete-dashboard --url http://localhost:3000 --interactive"
)

DIFF_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Compare raw dashboard exports against Grafana:\n"
    "    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw\n\n"
    "  Compare the same export against one destination folder:\n"
    "    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --import-folder-uid infra-folder\n\n"
    "  Inspect a Grafana file-provisioning tree separately:\n"
    "    grafana-util dashboard inspect-export --import-dir ./dashboards/provisioning --input-format provisioning --report tree-table"
)

INSPECT_VARS_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  List dashboard variables from a UID:\n"
    '    grafana-util dashboard list-vars --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --dashboard-uid cpu-main\n\n'
    "  Overlay current variable values from a dashboard URL:\n"
    "    grafana-util dashboard list-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output-format json"
)

SCREENSHOT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Capture a full dashboard screenshot:\n"
    "    grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page\n\n"
    "  Capture a solo panel with a custom header:\n"
    "    grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --token \"$GRAFANA_API_TOKEN\" --output ./panel.png --header-title 'CPU Busy'"
)

TOPOLOGY_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Render the dashboard topology as Mermaid:\n"
    "    grafana-util dashboard topology --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format mermaid\n\n"
    "  Render the same topology as DOT and persist it for downstream tooling:\n"
    "    grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot"
)

GOVERNANCE_GATE_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  Run policy checks for one dashboard export:\n"
    "    grafana-util dashboard governance-gate --policy ./policy.json --governance ./governance.json --queries ./queries.json\n\n"
    "  Render machine-readable output and write a copy:\n"
    "    grafana-util dashboard governance-gate --policy ./policy.json --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-gate.json"
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
        return
    if command == "delete-dashboard":
        if bool(getattr(args, "table", False)) or bool(getattr(args, "json", False)):
            parser.error(
                "--output-format cannot be combined with --table or --json for delete-dashboard."
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


def list_vars_command(args: argparse.Namespace) -> int:
    """List one dashboard's templating variables."""

    return inspect_vars(args)


def raw_to_prompt_command(args: argparse.Namespace) -> int:
    """Convert raw dashboard JSON into prompt-lane artifacts."""

    input_files = [Path(path) for path in getattr(args, "input_file", []) or []]
    input_dir = getattr(args, "input_dir", None)
    output_file = getattr(args, "output_file", None)
    output_dir = getattr(args, "output_dir", None)
    if input_dir and input_files:
        raise GrafanaError("--input-file and --input-dir cannot be used together.")
    if output_file and input_dir:
        raise GrafanaError("--output-file only supports a single --input-file source.")

    log_lines: list[str] = []
    output_items: list[dict[str, Any]] = []
    scanned = 0
    converted = 0
    failed = 0

    if input_dir:
        source_dir = Path(input_dir)
        if not source_dir.exists():
            raise GrafanaError(f"Input directory does not exist: {source_dir}")
        if not source_dir.is_dir():
            raise GrafanaError(f"Input path is not a directory: {source_dir}")
        resolved_input_dir = _resolve_raw_to_prompt_input_dir(source_dir)
        output_root = _resolve_raw_to_prompt_output_root(source_dir, output_dir)
        if output_root is None:
            raise GrafanaError(
                "Plain directory input requires --output-dir so raw-to-prompt does not mix generated files into the source tree."
            )
        metadata_source_dir: Optional[Path] = resolved_input_dir
        source_metadata = import_support_load_export_metadata(
            metadata_source_dir,
            export_metadata_filename=EXPORT_METADATA_FILENAME,
            root_index_kind=ROOT_INDEX_KIND,
            tool_schema_version=TOOL_SCHEMA_VERSION,
            expected_variant=RAW_EXPORT_SUBDIR,
        )
        org_name = (source_metadata or {}).get("org") if source_metadata else None
        org_id = (source_metadata or {}).get("orgId") if source_metadata else None
        datasource_catalog = _load_raw_to_prompt_datasource_catalog(
            resolved_input_dir, args
        )
        dashboard_files = discover_dashboard_files_from_export(
            resolved_input_dir,
            RAW_EXPORT_SUBDIR,
            PROMPT_EXPORT_SUBDIR,
            EXPORT_METADATA_FILENAME,
            FOLDER_INVENTORY_FILENAME,
            DATASOURCE_INVENTORY_FILENAME,
            DASHBOARD_PERMISSION_BUNDLE_FILENAME,
        )
        for input_path in dashboard_files:
            scanned += 1
            relative_path = input_path.relative_to(resolved_input_dir)
            output_path = output_root / relative_path
            try:
                document = _read_text_document(input_path)
                prompt_document = _build_raw_to_prompt_document(
                    input_path,
                    datasource_catalog,
                )
                if output_path.exists() and not bool(getattr(args, "overwrite", False)):
                    raise GrafanaError(
                        f"Refusing to overwrite existing file: {output_path}. Use --overwrite."
                    )
                if not bool(getattr(args, "dry_run", False)):
                    write_json_document(prompt_document, output_path)
                index_item = _build_raw_to_prompt_index_item(
                    input_path,
                    output_path,
                    document,
                    org_name,
                    org_id,
                )
                output_items.append(
                    {
                        "inputFile": str(input_path),
                        "outputFile": str(output_path),
                        "status": "ok",
                        "folder": index_item["folder"],
                    }
                )
                log_lines.append(
                    json.dumps(
                        {
                            "status": "ok",
                            "inputFile": str(input_path),
                            "outputFile": str(output_path),
                        },
                        ensure_ascii=False,
                    )
                )
                converted += 1
                if bool(getattr(args, "verbose", False)):
                    print(f"Converted raw-to-prompt: {input_path} -> {output_path}")
                elif bool(getattr(args, "progress", False)):
                    print(f"Converted prompt {scanned}/{len(dashboard_files)}: {input_path}")
                output_items[-1]["uid"] = index_item["uid"]
                output_items[-1]["title"] = index_item["title"]
            except Exception as exc:  # noqa: BLE001 - report per-file failures.
                failed += 1
                error_text = str(exc)
                output_items.append(
                    {
                        "inputFile": str(input_path),
                        "outputFile": str(output_path),
                        "status": "failed",
                        "error": error_text,
                    }
                )
                log_lines.append(
                    json.dumps(
                        {
                            "status": "fail",
                            "inputFile": str(input_path),
                            "outputFile": str(output_path),
                            "error": error_text,
                        },
                        ensure_ascii=False,
                    )
                )

        if not bool(getattr(args, "dry_run", False)):
            index_rows = [
                item
                for item in output_items
                if item.get("status") == "ok" and item.get("uid")
            ]
            build_rows = [
                {
                    "uid": item["uid"],
                    "title": item["title"],
                    "folder": item.get("folder") or DEFAULT_FOLDER_TITLE,
                    "org": _normalize_document_text(org_name, DEFAULT_ORG_NAME),
                    "orgId": _normalize_document_text(org_id, DEFAULT_ORG_ID),
                    "prompt_path": item["outputFile"],
                }
                for item in index_rows
            ]
            if build_rows:
                write_json_document(
                    build_variant_index(
                        build_rows,
                        "prompt_path",
                        "grafana-web-import-with-datasource-inputs",
                    ),
                    output_root / "index.json",
                )
                write_json_document(
                    build_export_metadata(
                        variant=PROMPT_EXPORT_SUBDIR,
                        dashboard_count=len(build_rows),
                        format_name="grafana-web-import-with-datasource-inputs",
                        org_name=org_name,
                        org_id=org_id,
                        orgs=(source_metadata or {}).get("orgs")
                        if isinstance(source_metadata, dict)
                        else None,
                    ),
                    output_root / EXPORT_METADATA_FILENAME,
                )

        if log_lines and getattr(args, "log_file", None):
            log_path = Path(args.log_file)
            log_path.parent.mkdir(parents=True, exist_ok=True)
            log_path.write_text("\n".join(log_lines) + "\n", encoding="utf-8")

        summary_document = {
            "kind": "grafana-utils-dashboard-raw-to-prompt-summary",
            "mode": "directory",
            "scanned": scanned,
            "converted": converted,
            "failed": failed,
            "outputRoot": str(output_root),
            "items": output_items,
        }
        dump_document(
            summary_document,
            getattr(args, "output_format", "text"),
            text_lines=[
                "raw-to-prompt completed"
                if failed == 0
                else "raw-to-prompt completed with failures",
                f"  scanned: {scanned}",
                f"  converted: {converted}",
                f"  failed: {failed}",
                f"  output: {output_root}",
            ],
        )
        if failed:
            raise GrafanaError(
                f"dashboard raw-to-prompt completed with {failed} failure(s)."
            )
        return 0

    if not input_files:
        raise GrafanaError("Provide either --input-file or --input-dir.")
    if output_file and len(input_files) != 1:
        raise GrafanaError("--output-file only supports a single --input-file source.")

    for input_path in input_files:
        scanned += 1
        output_path = _raw_to_prompt_single_output_path(
            input_path, output_file, output_dir
        )
        try:
            document = _read_text_document(input_path)
            datasource_catalog = _load_raw_to_prompt_datasource_catalog(
                input_path.parent, args
            )
            prompt_document = _build_raw_to_prompt_document(
                input_path, datasource_catalog
            )
            index_item = _build_raw_to_prompt_index_item(
                input_path,
                output_path,
                document,
                None,
                None,
            )
            if output_path.exists() and not bool(getattr(args, "overwrite", False)):
                raise GrafanaError(
                    f"Refusing to overwrite existing file: {output_path}. Use --overwrite."
                )
            if not bool(getattr(args, "dry_run", False)):
                write_json_document(prompt_document, output_path)
            output_items.append(
                {
                    "inputFile": str(input_path),
                    "outputFile": str(output_path),
                    "status": "ok",
                    "folder": index_item["folder"],
                    "uid": index_item["uid"],
                    "title": index_item["title"],
                }
            )
            log_lines.append(
                json.dumps(
                    {
                        "status": "ok",
                        "inputFile": str(input_path),
                        "outputFile": str(output_path),
                    },
                    ensure_ascii=False,
                )
            )
            converted += 1
        except Exception as exc:  # noqa: BLE001 - report per-file failures.
            failed += 1
            error_text = str(exc)
            output_items.append(
                {
                    "inputFile": str(input_path),
                    "outputFile": str(output_path),
                    "status": "failed",
                    "error": error_text,
                }
            )
            log_lines.append(
                json.dumps(
                    {
                        "status": "fail",
                        "inputFile": str(input_path),
                        "outputFile": str(output_path),
                        "error": error_text,
                    },
                    ensure_ascii=False,
                )
            )

    if log_lines and getattr(args, "log_file", None):
        log_path = Path(args.log_file)
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_path.write_text("\n".join(log_lines) + "\n", encoding="utf-8")

    summary_document = {
        "kind": "grafana-utils-dashboard-raw-to-prompt-summary",
        "mode": "single-file" if len(input_files) == 1 else "multi-file",
        "scanned": scanned,
        "converted": converted,
        "failed": failed,
        "items": output_items,
    }
    dump_document(
        summary_document,
        getattr(args, "output_format", "text"),
        text_lines=[
            "raw-to-prompt completed"
            if failed == 0
            else "raw-to-prompt completed with failures",
            f"  scanned: {scanned}",
            f"  converted: {converted}",
            f"  failed: {failed}",
        ],
    )
    if failed:
        raise GrafanaError(f"dashboard raw-to-prompt completed with {failed} failure(s).")
    return 0


def impact_command(args: argparse.Namespace) -> int:
    """Summarize one datasource blast radius from inspect artifacts."""

    document = _build_impact_document(
        Path(args.governance),
        Path(args.queries),
        args.datasource_uid,
    )
    summary = dict(document.get("summary") or {})
    dump_document(
        document,
        getattr(args, "output_format", "text"),
        text_lines=[
            f"Datasource impact for {document.get('datasourceUid') or args.datasource_uid}",
            f"  dashboards: {int(document.get('dashboardCount') or 0)}",
            f"  panels: {int(document.get('panelCount') or 0)}",
            f"  datasource: {document.get('datasource') or '-'}",
            f"  type: {document.get('datasourceType') or '-'}",
            f"  summary dashboards: {int(summary.get('dashboardCount') or 0)}",
            f"  summary panels: {int(summary.get('panelCount') or 0)}",
            f"  summary datasources: {int(summary.get('datasourceCount') or 0)}",
            f"  orphaned datasources: {int(summary.get('orphanedDatasourceCount') or 0)}",
        ],
    )
    output_file = getattr(args, "output_file", None)
    if output_file:
        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        if getattr(args, "output_format", "text") == "json":
            output_path.write_text(
                json.dumps(document, indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
        elif getattr(args, "output_format", "text") == "yaml":
            output_path.write_text(
                yaml.safe_dump(document).rstrip() + "\n",
                encoding="utf-8",
            )
        else:
            output_path.write_text(
                "\n".join(
                    [
                        f"Datasource impact for {document.get('datasourceUid') or args.datasource_uid}",
                        f"dashboards: {int(document.get('dashboardCount') or 0)}",
                        f"panels: {int(document.get('panelCount') or 0)}",
                        f"datasource: {document.get('datasource') or '-'}",
                        f"type: {document.get('datasourceType') or '-'}",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )
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


def _build_delete_workflow_deps() -> dict[str, Any]:
    """Internal helper for build delete workflow deps."""
    return {
        "GrafanaError": GrafanaError,
        "build_client": build_client,
        "build_delete_plan": build_delete_plan,
        "execute_delete_plan": execute_delete_plan,
        "format_live_dashboard_delete_line": format_live_dashboard_delete_line,
        "format_live_folder_delete_line": format_live_folder_delete_line,
        "input_reader": input,
        "is_tty": lambda: sys.stdin.isatty() and sys.stdout.isatty(),
        "output_writer": print,
        "render_dashboard_delete_json": render_dashboard_delete_json,
        "render_dashboard_delete_table": render_dashboard_delete_table,
        "render_dashboard_delete_text": render_dashboard_delete_text,
        "validate_delete_args": validate_delete_args,
    }


def delete_dashboards(args: argparse.Namespace) -> int:
    """Delete live dashboards from Grafana."""
    return run_delete_dashboards(args, _build_delete_workflow_deps())


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


def governance_gate_dashboards(args: argparse.Namespace) -> int:
    """Evaluate dashboard governance policy against export artifacts."""
    return run_dashboard_governance_gate(args)


def topology_dashboards(args: argparse.Namespace) -> int:
    """Build one dashboard topology view from governance artifacts."""
    return run_dashboard_topology(args)


def build_client(args: argparse.Namespace) -> GrafanaClient:
    """Build the dashboard API client from parsed CLI arguments."""
    headers = resolve_auth(args)
    return GrafanaClient(
        base_url=args.url,
        headers=headers,
        timeout=args.timeout,
        verify_ssl=bool(args.verify_ssl or getattr(args, "ca_cert", None)),
        ca_cert=getattr(args, "ca_cert", None),
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
        if args.command == "fetch-live":
            return fetch_live_dashboard_command(args)
        if args.command == "clone-live":
            return clone_live_dashboard_command(args)
        if args.command == "edit-live":
            return edit_live_dashboard_command(args)
        if args.command == "patch-file":
            return patch_file_command(args)
        if args.command == "serve":
            return serve_command(args)
        if args.command == "review":
            return review_command(args)
        if args.command == "publish":
            return publish_command(args)
        if args.command == "analyze":
            return analyze_command(args)
        if args.command == "validate-export":
            return validate_export_command(args)
        if args.command == "history":
            if args.history_command == "list":
                return history_list_command(args)
            if args.history_command == "export":
                return history_export_command(args)
            if args.history_command == "restore":
                return history_restore_command(args)
            raise GrafanaError("Unsupported dashboard history command.")
        if args.command == "raw-to-prompt":
            return raw_to_prompt_command(args)
        if args.command == "list-dashboard":
            return list_dashboards(args)
        if args.command == "inspect-export":
            return inspect_export(args)
        if args.command == "inspect-live":
            return inspect_live(args)
        if args.command in ("list-vars", "inspect-vars"):
            return list_vars_command(args)
        if args.command == "screenshot":
            return screenshot_dashboard(args)
        if args.command == "delete-dashboard":
            return delete_dashboards(args)
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
        if args.command == "governance-gate":
            return governance_gate_dashboards(args)
        if args.command == "topology":
            return topology_dashboards(args)
        if args.command == "impact":
            return impact_command(args)
        return export_dashboards(args)
    except (GrafanaError, ValueError, OSError) as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
