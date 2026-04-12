#!/usr/bin/env python3
"""Stable facade for the Python datasource CLI.

Purpose:
- Provide a stable facade API for datasource commands and delegate to
  `datasource.workflows` after parse/normalize has finished.

Architecture:
- Keep user-facing argument parsing and execution orchestration centralized in this
  module to preserve legacy API imports from `grafana_utils.datasource_cli`.
- Delegate heavy workflow work to `grafana_utils.datasource.workflows` after parsing
  and normalization are complete.
- Re-export selected workflow helpers for existing callers relying on the old flat
  import surface.

Caveats:
- Keep schema/contract strictness in `datasource_contract` and parser details in
  `datasource/parser.py`.
- Legacy aliases are intentionally preserved for `python3 -m grafana_utils`
  compatibility paths.
"""

import sys

from .dashboard_cli import (
    GrafanaError,
)
from .datasource.parser import (
    DATASOURCE_EXPORT_FILENAME,
    DATASOURCE_PROVISIONING_FILENAME,
    DATASOURCE_PROVISIONING_SUBDIR,
    DEFAULT_EXPORT_DIR,
    EXPORT_METADATA_FILENAME,
    DIFF_OUTPUT_FORMAT_CHOICES,
    DATASOURCE_IMPORT_FORMAT_CHOICES,
    LIVE_MUTATION_DRY_RUN_OUTPUT_FORMAT_CHOICES,
    ROOT_INDEX_KIND,
    TOOL_SCHEMA_VERSION,
    add_add_cli_args,
    add_delete_cli_args,
    add_modify_cli_args,
    build_parser,
)
from .datasource import workflows as datasource_workflows
from .datasource.workflows import (
    build_client,
    build_effective_import_client,
    build_existing_datasource_lookups,
    build_export_index,
    build_export_metadata,
    build_export_records,
    exporter_api_error_type,
    build_import_payload,
    determine_datasource_action,
    determine_import_mode,
    fetch_datasource_by_uid_if_exists,
    load_import_bundle,
    modify_datasource as workflow_modify_datasource,
    parse_import_dry_run_columns,
    render_data_source_csv,
    render_data_source_json,
    render_import_dry_run_json,
    render_import_dry_run_table,
    resolve_datasource_match,
    resolve_export_org_id,
    validate_export_org_match,
)
from .datasource_contract import (
    normalize_datasource_record,
    validate_datasource_contract_record,
)
from .datasource_diff import (
    build_live_datasource_diff_records,
    compare_datasource_bundle_to_live,
    load_datasource_diff_bundle,
)

_DEFAULT_BUILD_CLIENT = build_client
_LAST_SYNCED_BUILD_CLIENT = build_client

__all__ = [
    "DATASOURCE_EXPORT_FILENAME",
    "DATASOURCE_PROVISIONING_FILENAME",
    "DATASOURCE_PROVISIONING_SUBDIR",
    "DEFAULT_EXPORT_DIR",
    "EXPORT_METADATA_FILENAME",
    "GrafanaError",
    "LIVE_MUTATION_DRY_RUN_OUTPUT_FORMAT_CHOICES",
    "ROOT_INDEX_KIND",
    "TOOL_SCHEMA_VERSION",
    "build_client",
    "build_effective_import_client",
    "build_existing_datasource_lookups",
    "build_export_index",
    "build_export_metadata",
    "build_export_records",
    "build_import_payload",
    "browse_datasources",
    "compare_datasource_bundle_to_live",
    "determine_datasource_action",
    "determine_import_mode",
    "diff_datasources",
    "export_datasources",
    "exporter_api_error_type",
    "fetch_datasource_by_uid_if_exists",
    "import_datasources",
    "list_datasources",
    "load_datasource_diff_bundle",
    "load_import_bundle",
    "main",
    "modify_datasource",
    "normalize_datasource_record",
    "parse_args",
    "parse_import_dry_run_columns",
    "render_data_source_csv",
    "render_data_source_json",
    "render_import_dry_run_json",
    "render_import_dry_run_table",
    "resolve_datasource_match",
    "resolve_export_org_id",
    "validate_datasource_contract_record",
    "validate_export_org_match",
]


def _normalize_output_format_args(args, parser):
    """Normalize datasource `--output-format` into concrete output mode switches."""
    output_format = getattr(args, "output_format", None)
    if output_format is None:
        return
    if getattr(args, "command", None) == "types":
        if (
            bool(getattr(args, "table", False))
            or bool(getattr(args, "csv", False))
            or bool(getattr(args, "json", False))
            or bool(getattr(args, "yaml", False))
        ):
            parser.error(
                "--output-format cannot be combined with --table, --csv, --json, or --yaml for datasource types."
            )
        args.table = output_format == "table"
        args.csv = output_format == "csv"
        args.json = output_format == "json"
        args.yaml = output_format == "yaml"
        return
    if getattr(args, "command", None) == "list":
        if (
            bool(getattr(args, "text", False))
            or bool(getattr(args, "table", False))
            or bool(getattr(args, "csv", False))
            or bool(getattr(args, "json", False))
            or bool(getattr(args, "yaml", False))
        ):
            parser.error(
                "--output-format cannot be combined with --text, --table, --csv, --json, or --yaml for datasource list."
            )
        args.text = output_format == "text"
        args.table = output_format == "table"
        args.csv = output_format == "csv"
        args.json = output_format == "json"
        args.yaml = output_format == "yaml"
        return
    if getattr(args, "command", None) == "diff":
        if bool(getattr(args, "json", False)):
            parser.error(
                "--output-format cannot be combined with --json for datasource diff."
            )
        args.output_format = output_format
        return
    if getattr(args, "command", None) == "import":
        # Import mode intentionally only supports table/json output mode.
        if bool(getattr(args, "table", False)) or bool(getattr(args, "json", False)):
            parser.error(
                "--output-format cannot be combined with --table or --json for datasource import."
            )
        args.table = output_format == "table"
        args.json = output_format == "json"
        return
    if getattr(args, "command", None) in ("add", "modify", "delete"):
        if bool(getattr(args, "table", False)) or bool(getattr(args, "json", False)):
            parser.error(
                "--output-format cannot be combined with --table or --json for datasource %s."
                % args.command
            )
        args.table = output_format == "table"
        args.json = output_format == "json"


def _parse_import_output_columns(args, parser):
    """Parse output-column aliases only for datasource import dry-run table output."""
    if getattr(args, "command", None) != "import":
        return
    value = getattr(args, "output_columns", None)
    if value is None:
        return
    if not bool(getattr(args, "table", False)):
        parser.error(
            "--output-columns is only supported with --dry-run --table or table-like --output-format for datasource import."
        )
    try:
        args.output_columns = parse_import_dry_run_columns(value)
    except GrafanaError as exc:
        parser.error(str(exc))


def _normalize_datasource_aliases(args):
    """Normalize command aliases to the Python facade's canonical attribute names."""
    if getattr(args, "command", None) == "export" and hasattr(args, "output_dir"):
        args.export_dir = args.output_dir
    if getattr(args, "command", None) in ("import", "diff") and hasattr(
        args, "input_dir"
    ):
        args.import_dir = getattr(args, "input_dir", getattr(args, "import_dir", None))
        args.diff_dir = getattr(args, "input_dir", getattr(args, "diff_dir", None))


def _validate_datasource_org_routing_args(args, parser):
    """Validate datasource org-routing flags after argparse normalization."""
    command = getattr(args, "command", None)
    if command in ("list", "browse"):
        if bool(getattr(args, "all_orgs", False)) and getattr(args, "org_id", None):
            parser.error(
                "--all-orgs cannot be combined with --org-id for datasource %s."
                % command
            )
        return
    if command == "export":
        if bool(getattr(args, "all_orgs", False)) and getattr(args, "org_id", None):
            parser.error(
                "--all-orgs cannot be combined with --org-id for datasource export."
            )
        return
    if command != "import":
        return
    use_export_org = bool(getattr(args, "use_export_org", False))
    if getattr(args, "only_org_id", None) and not use_export_org:
        parser.error("--only-org-id requires --use-export-org for datasource import.")
    if bool(getattr(args, "create_missing_orgs", False)) and not use_export_org:
        parser.error(
            "--create-missing-orgs requires --use-export-org for datasource import."
        )
    if use_export_org and getattr(args, "org_id", None):
        parser.error(
            "--use-export-org cannot be combined with --org-id for datasource import."
        )
    if use_export_org and bool(getattr(args, "require_matching_export_org", False)):
        parser.error(
            "--use-export-org cannot be combined with --require-matching-export-org for datasource import."
        )


def parse_args(argv=None):
    """Parse datasource CLI args then normalize legacy-compatible output options.

    Flow:
    - Parse argv with datasource parser (`list`/`export`/`import`/`diff`).
    - Normalize `--output-format` into exclusive `--table`/`--csv`/`--json`.
    - Parse and validate dry-run output-column aliases.
    """
    # Call graph: see callers/callees.
    #   Upstream callers: 244
    #   Downstream callees: 132, 149, 97

    parser = build_parser()
    args = parser.parse_args(argv)
    _normalize_datasource_aliases(args)
    _normalize_output_format_args(args, parser)
    _parse_import_output_columns(args, parser)
    _validate_datasource_org_routing_args(args, parser)
    return args


def _sync_facade_overrides():
    """Rebind workflow-layer dependencies that might be overridden in tests."""
    global _LAST_SYNCED_BUILD_CLIENT

    if build_client is not _DEFAULT_BUILD_CLIENT:
        datasource_workflows.build_client = build_client
        _LAST_SYNCED_BUILD_CLIENT = build_client
        return

    if datasource_workflows.build_client is _LAST_SYNCED_BUILD_CLIENT:
        datasource_workflows.build_client = _DEFAULT_BUILD_CLIENT
        _LAST_SYNCED_BUILD_CLIENT = _DEFAULT_BUILD_CLIENT


def list_datasources(args):
    """List datasources implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 191

    _sync_facade_overrides()
    return datasource_workflows.list_datasources(args)


def browse_datasources(args):
    """Browse datasources implementation."""
    _sync_facade_overrides()
    return datasource_workflows.browse_datasources(args)


def export_datasources(args):
    """Export datasources implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 191

    _sync_facade_overrides()
    return datasource_workflows.export_datasources(args)


def import_datasources(args):
    """Import datasources implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 191

    _sync_facade_overrides()
    return datasource_workflows.import_datasources(args)


def diff_datasources(args):
    """Diff datasources implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 191

    _sync_facade_overrides()
    return datasource_workflows.diff_datasources(args)


def add_datasource(args):
    """Add datasource implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 191

    _sync_facade_overrides()
    return datasource_workflows.add_datasource(args)


def delete_datasource(args):
    """Delete datasource implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 191

    _sync_facade_overrides()
    return datasource_workflows.delete_datasource(args)


def modify_datasource(args):
    """Modify datasource implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 191

    _sync_facade_overrides()
    return workflow_modify_datasource(args)


def dispatch_datasource_command(args):
    """Dispatch datasource command implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 244
    #   Downstream callees: 191

    _sync_facade_overrides()
    return datasource_workflows.dispatch_datasource_command(args)


def main(argv=None):
    """Route datasource commands through the facade after argument normalization.

    Flow:
    - Parse + normalize input arguments.
    - Rebind workflow client helpers to facade shims used by tests.
    - Delegate to workflow dispatch for the selected command.
    """
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 175, 238

    args = parse_args(argv)
    try:
        return dispatch_datasource_command(args)
    except GrafanaError as exc:
        print("Error: %s" % exc, file=sys.stderr)
        return 1


__all__ = [
    "DATASOURCE_EXPORT_FILENAME",
    "DEFAULT_EXPORT_DIR",
    "EXPORT_METADATA_FILENAME",
    "ROOT_INDEX_KIND",
    "TOOL_SCHEMA_VERSION",
    "LIVE_MUTATION_DRY_RUN_OUTPUT_FORMAT_CHOICES",
    "add_add_cli_args",
    "add_datasource",
    "add_modify_cli_args",
    "build_client",
    "build_effective_import_client",
    "build_existing_datasource_lookups",
    "build_export_index",
    "build_export_metadata",
    "build_export_records",
    "build_import_payload",
    "browse_datasources",
    "build_live_datasource_diff_records",
    "build_parser",
    "compare_datasource_bundle_to_live",
    "delete_datasource",
    "determine_datasource_action",
    "determine_import_mode",
    "diff_datasources",
    "dispatch_datasource_command",
    "export_datasources",
    "fetch_datasource_by_uid_if_exists",
    "import_datasources",
    "list_datasources",
    "load_datasource_diff_bundle",
    "load_import_bundle",
    "main",
    "modify_datasource",
    "normalize_datasource_record",
    "parse_args",
    "parse_import_dry_run_columns",
    "add_delete_cli_args",
    "render_data_source_csv",
    "render_data_source_json",
    "render_import_dry_run_json",
    "render_import_dry_run_table",
    "resolve_datasource_match",
    "resolve_export_org_id",
    "validate_datasource_contract_record",
    "validate_export_org_match",
]
