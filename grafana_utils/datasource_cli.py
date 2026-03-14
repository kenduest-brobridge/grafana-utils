#!/usr/bin/env python3
"""Stable facade for the Python datasource CLI."""

import sys

from .dashboard_cli import (
    DEFAULT_TIMEOUT,
    DEFAULT_URL,
    GrafanaError,
    HelpFullAction,
    add_common_cli_args,
    resolve_auth,
)
from .datasource.parser import (
    DATASOURCE_EXPORT_FILENAME,
    DEFAULT_EXPORT_DIR,
    EXPORT_METADATA_FILENAME,
    HELP_FULL_EXAMPLES,
    IMPORT_DRY_RUN_COLUMN_ALIASES,
    IMPORT_DRY_RUN_COLUMN_HEADERS,
    IMPORT_DRY_RUN_OUTPUT_FORMAT_CHOICES,
    LIST_OUTPUT_FORMAT_CHOICES,
    ROOT_INDEX_KIND,
    TOOL_SCHEMA_VERSION,
    add_diff_cli_args,
    add_export_cli_args,
    add_import_cli_args,
    add_list_cli_args,
    build_parser,
)
from .datasource import workflows as datasource_workflows
from .datasource.workflows import (
    _print_datasource_unified_diff,
    _serialize_datasource_diff_record,
    build_client,
    build_effective_import_client,
    build_existing_datasource_lookups,
    build_export_index,
    build_export_metadata,
    build_export_records,
    build_import_payload,
    determine_datasource_action,
    determine_import_mode,
    diff_datasources,
    dispatch_datasource_command,
    export_datasources,
    exporter_api_error_type,
    fetch_datasource_by_uid_if_exists,
    import_datasources,
    list_datasources,
    load_import_bundle,
    load_json_document,
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


def _normalize_output_format_args(args, parser):
    output_format = getattr(args, "output_format", None)
    if output_format is None:
        return
    if getattr(args, "command", None) == "list":
        if bool(getattr(args, "table", False)) or bool(getattr(args, "csv", False)) or bool(
            getattr(args, "json", False)
        ):
            parser.error(
                "--output-format cannot be combined with --table, --csv, or --json for datasource list."
            )
        args.table = output_format == "table"
        args.csv = output_format == "csv"
        args.json = output_format == "json"
        return
    if getattr(args, "command", None) == "import":
        if bool(getattr(args, "table", False)) or bool(getattr(args, "json", False)):
            parser.error(
                "--output-format cannot be combined with --table or --json for datasource import."
            )
        args.table = output_format == "table"
        args.json = output_format == "json"


def _parse_import_output_columns(args, parser):
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


def parse_args(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    _normalize_output_format_args(args, parser)
    _parse_import_output_columns(args, parser)
    return args


def _sync_facade_overrides():
    datasource_workflows.build_client = build_client


def list_datasources(args):
    _sync_facade_overrides()
    return datasource_workflows.list_datasources(args)


def export_datasources(args):
    _sync_facade_overrides()
    return datasource_workflows.export_datasources(args)


def import_datasources(args):
    _sync_facade_overrides()
    return datasource_workflows.import_datasources(args)


def diff_datasources(args):
    _sync_facade_overrides()
    return datasource_workflows.diff_datasources(args)


def dispatch_datasource_command(args):
    _sync_facade_overrides()
    return datasource_workflows.dispatch_datasource_command(args)


def main(argv=None):
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
    "build_client",
    "build_effective_import_client",
    "build_existing_datasource_lookups",
    "build_export_index",
    "build_export_metadata",
    "build_export_records",
    "build_import_payload",
    "build_live_datasource_diff_records",
    "build_parser",
    "compare_datasource_bundle_to_live",
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
