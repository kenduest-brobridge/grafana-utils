#!/usr/bin/env python3
"""Export or import Grafana alerting resources."""

import argparse
import csv
import copy
import difflib
import getpass
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .auth_staging import AuthConfigError, resolve_cli_auth_from_namespace
from .alerts.common import (
    CONTACT_POINT_KIND,
    CONTACT_POINTS_SUBDIR,
    GrafanaApiError,
    GrafanaError,
    MUTE_TIMING_KIND,
    MUTE_TIMINGS_SUBDIR,
    POLICIES_KIND,
    POLICIES_SUBDIR,
    RAW_EXPORT_SUBDIR,
    RESOURCE_SUBDIR_BY_KIND,
    ROOT_INDEX_KIND,
    RULE_KIND,
    RULES_SUBDIR,
    TEMPLATE_KIND,
    TEMPLATES_SUBDIR,
    TOOL_API_VERSION,
    TOOL_SCHEMA_VERSION,
)
from .alerts.provisioning import (
    build_compare_document,
    build_contact_point_export_document,
    build_contact_point_import_payload,
    build_diff_label,
    build_empty_root_index,
    build_import_operation,
    build_linked_dashboard_metadata,
    build_mute_timing_export_document,
    build_mute_timing_import_payload,
    build_policies_export_document,
    build_resource_identity,
    build_rule_export_document,
    build_rule_import_payload,
    build_template_export_document,
    build_template_import_payload,
    determine_import_action,
    fetch_live_compare_document,
    load_panel_id_map as load_panel_id_map_impl,
    load_string_map as load_string_map_impl,
    prepare_import_payload_for_target,
    rewrite_rule_dashboard_linkage,
    serialize_compare_document,
)
from .clients.alert_client import GrafanaAlertClient
from .http_transport import build_json_http_transport


DEFAULT_URL = "http://127.0.0.1:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_OUTPUT_DIR = "alerts"
LIST_OUTPUT_FORMAT_CHOICES = ("table", "csv", "json")
HELP_EPILOG = """Examples:

  Export alerting resources with an API token:
    export GRAFANA_API_TOKEN='your-token'
    grafana-utils alert export --url https://grafana.example.com --output-dir ./alerts --overwrite

  Import back into Grafana and update existing resources:
    grafana-utils alert import --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing

  Import linked alert rules with dashboard and panel remapping:
    grafana-utils alert import --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json
"""

def add_common_args(parser: argparse.ArgumentParser) -> None:
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


def add_export_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--output-dir",
        default=DEFAULT_OUTPUT_DIR,
        help=(
            "Directory to write exported alerting resources into. Export writes files "
            f"under {RAW_EXPORT_SUBDIR}/."
        ),
    )
    parser.add_argument(
        "--flat",
        action="store_true",
        help=(
            "Write rule, contact-point, and mute-timing files directly into their "
            "resource directories instead of nested folder/group directories."
        ),
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing exported files if they already exist.",
    )


def add_list_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--table",
        action="store_true",
        help="Render list output as a table. This is the default.",
    )
    parser.add_argument(
        "--csv",
        action="store_true",
        help="Render list output as CSV.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render list output as JSON.",
    )
    parser.add_argument(
        "--no-header",
        action="store_true",
        help="Omit the table header row.",
    )
    parser.add_argument(
        "--output-format",
        choices=LIST_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for alert list output. "
            "Use table, csv, or json. This cannot be combined with --table, "
            "--csv, or --json."
        ),
    )


def add_import_args(parser: argparse.ArgumentParser, diff_mode: bool = False) -> None:
    dir_flag = "--diff-dir" if diff_mode else "--import-dir"
    verb = "Compare" if diff_mode else "Import"
    parser.add_argument(
        dir_flag,
        dest="diff_dir" if diff_mode else "import_dir",
        required=True,
        help=(
            f"{verb} alerting resource JSON from this directory against Grafana. "
            if diff_mode
            else "Import alerting resource JSON from this directory instead of exporting. "
        )
        + f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly.",
    )
    if not diff_mode:
        parser.add_argument(
            "--replace-existing",
            action="store_true",
            help="Update existing resources with the same identity instead of failing on import.",
        )
        parser.add_argument(
            "--dry-run",
            action="store_true",
            help="Show whether each import file would create or update resources without changing Grafana.",
        )
    parser.add_argument(
        "--dashboard-uid-map",
        default=None,
        help=(
            "JSON file that maps source dashboard UIDs to target dashboard UIDs "
            "for linked alert-rule repair during import."
        ),
    )
    parser.add_argument(
        "--panel-id-map",
        default=None,
        help=(
            "JSON file that maps source dashboard UID and source panel ID to a "
            "target panel ID for linked alert-rule repair during import."
        ),
    )
    if diff_mode:
        parser.set_defaults(replace_existing=False, dry_run=False, output_dir=DEFAULT_OUTPUT_DIR, flat=False, overwrite=False)
    else:
        parser.set_defaults(diff_dir=None, output_dir=DEFAULT_OUTPUT_DIR, flat=False, overwrite=False)


def build_legacy_parser(prog: Optional[str] = None) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=prog,
        description="Export, import, or diff Grafana alerting resources.",
        epilog=HELP_EPILOG,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_args(parser)
    add_export_args(parser)
    mode_group = parser.add_mutually_exclusive_group()
    mode_group.add_argument(
        "--import-dir",
        default=None,
        help=(
            "Import alerting resource JSON from this directory instead of exporting. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly."
        ),
    )
    mode_group.add_argument(
        "--diff-dir",
        default=None,
        help=(
            "Compare alerting resource JSON from this directory against Grafana. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly."
        ),
    )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="Update existing resources with the same identity instead of failing on import.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show whether each import file would create or update resources without changing Grafana.",
    )
    parser.add_argument(
        "--dashboard-uid-map",
        default=None,
        help=(
            "JSON file that maps source dashboard UIDs to target dashboard UIDs "
            "for linked alert-rule repair during import."
        ),
    )
    parser.add_argument(
        "--panel-id-map",
        default=None,
        help=(
            "JSON file that maps source dashboard UID and source panel ID to a "
            "target panel ID for linked alert-rule repair during import."
        ),
    )
    parser.set_defaults(alert_command=None)
    return parser


def build_parser(prog: Optional[str] = None) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=prog,
        description="Export, import, or diff Grafana alerting resources.",
        epilog=HELP_EPILOG,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="alert_command")

    export_parser = subparsers.add_parser(
        "export",
        help="Export alerting resources into raw/ JSON files.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_args(export_parser)
    add_export_args(export_parser)
    export_parser.set_defaults(
        alert_command="export",
        import_dir=None,
        diff_dir=None,
        replace_existing=False,
        dry_run=False,
        dashboard_uid_map=None,
        panel_id_map=None,
    )

    import_parser = subparsers.add_parser(
        "import",
        help="Import alerting resource JSON files through the Grafana API.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_args(import_parser)
    add_import_args(import_parser, diff_mode=False)
    import_parser.set_defaults(alert_command="import")

    diff_parser = subparsers.add_parser(
        "diff",
        help="Compare local alerting export files against live Grafana resources.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_common_args(diff_parser)
    add_import_args(diff_parser, diff_mode=True)
    diff_parser.set_defaults(alert_command="diff")

    for command_name, help_text in (
        ("list-rules", "List live Grafana alert rules."),
        ("list-contact-points", "List live Grafana alert contact points."),
        ("list-mute-timings", "List live Grafana mute timings."),
        ("list-templates", "List live Grafana notification templates."),
    ):
        list_parser = subparsers.add_parser(
            command_name,
            help=help_text,
            formatter_class=argparse.RawDescriptionHelpFormatter,
        )
        add_common_args(list_parser)
        add_list_args(list_parser)
        list_parser.set_defaults(alert_command=command_name)

    return parser


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv in (["-h"], ["--help"]):
        parser = build_parser()
        args = parser.parse_args(argv)
        _normalize_output_format_args(args, parser)
        return args
    if argv and argv[0] in (
        "export",
        "import",
        "diff",
        "list-rules",
        "list-contact-points",
        "list-mute-timings",
        "list-templates",
    ):
        parser = build_parser()
        args = parser.parse_args(argv)
        _normalize_output_format_args(args, parser)
        return args

    parser = build_legacy_parser()
    args = parser.parse_args(argv)
    if getattr(args, "import_dir", None):
        args.alert_command = "import"
    elif getattr(args, "diff_dir", None):
        args.alert_command = "diff"
    else:
        args.alert_command = "export"
    _normalize_output_format_args(args, parser)
    return args


def _normalize_output_format_args(
    args: argparse.Namespace,
    parser: argparse.ArgumentParser,
) -> None:
    output_format = getattr(args, "output_format", None)
    if output_format is None or not str(getattr(args, "alert_command", "")).startswith("list-"):
        return
    if bool(getattr(args, "table", False)) or bool(getattr(args, "csv", False)) or bool(
        getattr(args, "json", False)
    ):
        parser.error(
            "--output-format cannot be combined with --table, --csv, or --json for alert list commands."
        )
    args.table = output_format == "table"
    args.csv = output_format == "csv"
    args.json = output_format == "json"


def resolve_auth(args: argparse.Namespace) -> Dict[str, str]:
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


def sanitize_path_component(value: str) -> str:
    normalized = re.sub(r"[^\w.\- ]+", "_", value.strip(), flags=re.UNICODE)
    normalized = re.sub(r"\s+", "_", normalized)
    normalized = re.sub(r"_+", "_", normalized)
    normalized = normalized.strip("._")
    return normalized or "untitled"


def write_json(payload: Any, output_path: Path, overwrite: bool) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    if output_path.exists() and not overwrite:
        raise GrafanaError(
            f"Refusing to overwrite existing file: {output_path}. Use --overwrite."
        )
    output_path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def load_string_map(path_value: Optional[str], label: str) -> Dict[str, str]:
    """Facade wrapper that keeps the original alert_cli helper signature stable."""
    return load_string_map_impl(path_value, label, load_json_file)


def load_panel_id_map(path_value: Optional[str]) -> Dict[str, Dict[str, str]]:
    """Facade wrapper that keeps the original alert_cli helper signature stable."""
    return load_panel_id_map_impl(path_value, load_json_file)


def render_compare_json(payload: Dict[str, Any]) -> str:
    """Render compare payloads with stable ordering for readable diff output."""
    return json.dumps(
        payload,
        indent=2,
        sort_keys=True,
        ensure_ascii=False,
    ) + "\n"


def print_unified_diff(
    before_payload: Dict[str, Any],
    after_payload: Dict[str, Any],
    before_label: str,
    after_label: str,
) -> None:
    """Print a unified diff for two compare payloads."""
    before_text = render_compare_json(before_payload)
    after_text = render_compare_json(after_payload)
    if before_text == after_text:
        return

    diff_text = "".join(
        difflib.unified_diff(
            before_text.splitlines(True),
            after_text.splitlines(True),
            fromfile=before_label,
            tofile=after_label,
        )
    )
    if diff_text:
        print(diff_text, end="")


def load_json_file(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise GrafanaError(f"JSON file not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError(f"Invalid JSON file: {path}") from exc


def build_resource_dirs(raw_dir: Path) -> Dict[str, Path]:
    return {
        kind: raw_dir / subdir for kind, subdir in RESOURCE_SUBDIR_BY_KIND.items()
    }


def build_rule_output_path(output_dir: Path, rule: Dict[str, Any], flat: bool) -> Path:
    folder_uid = sanitize_path_component(rule.get("folderUID") or "unknown-folder")
    rule_group = sanitize_path_component(rule.get("ruleGroup") or "default-group")
    title = sanitize_path_component(rule.get("title") or "alert-rule")
    uid = sanitize_path_component(rule.get("uid") or title or "unknown")
    filename = f"{title}__{uid}.json"
    if flat:
        return output_dir / filename
    return output_dir / folder_uid / rule_group / filename


def build_contact_point_output_path(
    output_dir: Path,
    contact_point: Dict[str, Any],
    flat: bool,
) -> Path:
    name = sanitize_path_component(contact_point.get("name") or "contact-point")
    uid = sanitize_path_component(contact_point.get("uid") or name or "unknown")
    filename = f"{name}__{uid}.json"
    return output_dir / filename if flat else output_dir / name / filename


def build_mute_timing_output_path(
    output_dir: Path,
    mute_timing: Dict[str, Any],
    flat: bool,
) -> Path:
    name = sanitize_path_component(mute_timing.get("name") or "mute-timing")
    filename = f"{name}.json"
    return output_dir / filename if flat else output_dir / name / filename


def build_policies_output_path(output_dir: Path) -> Path:
    return output_dir / "notification-policies.json"


def build_template_output_path(
    output_dir: Path,
    template: Dict[str, Any],
    flat: bool,
) -> Path:
    name = sanitize_path_component(template.get("name") or "template")
    filename = f"{name}.json"
    return output_dir / filename if flat else output_dir / name / filename


def discover_alert_resource_files(import_dir: Path) -> List[Path]:
    """Find alerting resource JSON files and reject the combined export root."""
    if not import_dir.exists():
        raise GrafanaError(f"Import directory does not exist: {import_dir}")
    if not import_dir.is_dir():
        raise GrafanaError(f"Import path is not a directory: {import_dir}")
    if (import_dir / RAW_EXPORT_SUBDIR).is_dir():
        raise GrafanaError(
            f"Import path {import_dir} looks like the export root. "
            f"Point --import-dir at {import_dir / RAW_EXPORT_SUBDIR}."
        )

    files = [
        path
        for path in sorted(import_dir.rglob("*.json"))
        if path.name != "index.json"
    ]
    if not files:
        raise GrafanaError(f"No alerting resource JSON files found in {import_dir}")
    return files


ALERT_RULE_LIST_FIELDS = ["uid", "title", "folderUID", "ruleGroup"]
CONTACT_POINT_LIST_FIELDS = ["uid", "name", "type"]
MUTE_TIMING_LIST_FIELDS = ["name", "intervals"]
TEMPLATE_LIST_FIELDS = ["name"]


def build_alert_list_table(
    rows: List[Dict[str, Any]],
    fields: List[str],
    headers: Dict[str, str],
    include_header: bool = True,
) -> List[str]:
    widths = {}
    for field in fields:
        widths[field] = len(headers[field])
        for row in rows:
            widths[field] = max(widths[field], len(str(row.get(field) or "")))

    def build_row(values: Dict[str, Any]) -> str:
        return "  ".join(
            str(values.get(field) or "").ljust(widths[field]) for field in fields
        )

    lines = []
    if include_header:
        lines.append(build_row(headers))
        lines.append("  ".join("-" * widths[field] for field in fields))
    for row in rows:
        lines.append(build_row(row))
    return lines


def render_alert_list_csv(rows: List[Dict[str, Any]], fields: List[str]) -> None:
    writer = csv.DictWriter(sys.stdout, fieldnames=fields)
    writer.writeheader()
    for row in rows:
        writer.writerow(row)


def render_alert_list_json(rows: List[Dict[str, Any]]) -> str:
    return json.dumps(rows, indent=2, ensure_ascii=False)


def serialize_rule_list_rows(rules: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    rows = []
    for rule in rules:
        rows.append(
            {
                "uid": str(rule.get("uid") or ""),
                "title": str(rule.get("title") or ""),
                "folderUID": str(rule.get("folderUID") or ""),
                "ruleGroup": str(rule.get("ruleGroup") or ""),
            }
        )
    return rows


def serialize_contact_point_list_rows(
    contact_points: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    rows = []
    for item in contact_points:
        rows.append(
            {
                "uid": str(item.get("uid") or ""),
                "name": str(item.get("name") or ""),
                "type": str(item.get("type") or ""),
            }
        )
    return rows


def serialize_mute_timing_list_rows(
    mute_timings: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    rows = []
    for item in mute_timings:
        intervals = item.get("time_intervals") or []
        rows.append(
            {
                "name": str(item.get("name") or ""),
                "intervals": str(len(intervals)),
            }
        )
    return rows


def serialize_template_list_rows(
    templates: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    return [{"name": str(item.get("name") or "")} for item in templates]


def export_rule_documents(
    client: GrafanaAlertClient,
    rules: List[Dict[str, Any]],
    resource_dirs: Dict[str, Path],
    root_index: Dict[str, List[Dict[str, str]]],
    flat: bool,
    overwrite: bool,
) -> None:
    """Export alert rules and append rule entries to the root index."""
    for rule in rules:
        normalized_rule = copy.deepcopy(rule)
        linked_dashboard = build_linked_dashboard_metadata(client, rule)
        if linked_dashboard:
            normalized_rule["__linkedDashboardMetadata__"] = linked_dashboard
        document = build_rule_export_document(normalized_rule)
        spec = document["spec"]
        output_path = build_rule_output_path(resource_dirs[RULE_KIND], spec, flat)
        write_json(document, output_path, overwrite)
        item = {
            "kind": RULE_KIND,
            "uid": str(spec.get("uid") or ""),
            "title": str(spec.get("title") or ""),
            "folderUID": str(spec.get("folderUID") or ""),
            "ruleGroup": str(spec.get("ruleGroup") or ""),
            "path": str(output_path),
        }
        root_index[RULES_SUBDIR].append(item)
        print(f"Exported alert rule {item['uid'] or 'unknown'} -> {output_path}")


def export_contact_point_documents(
    contact_points: List[Dict[str, Any]],
    resource_dirs: Dict[str, Path],
    root_index: Dict[str, List[Dict[str, str]]],
    flat: bool,
    overwrite: bool,
) -> None:
    """Export contact points and append contact-point entries to the root index."""
    for contact_point in contact_points:
        document = build_contact_point_export_document(contact_point)
        spec = document["spec"]
        output_path = build_contact_point_output_path(
            resource_dirs[CONTACT_POINT_KIND], spec, flat
        )
        write_json(document, output_path, overwrite)
        item = {
            "kind": CONTACT_POINT_KIND,
            "uid": str(spec.get("uid") or ""),
            "name": str(spec.get("name") or ""),
            "type": str(spec.get("type") or ""),
            "path": str(output_path),
        }
        root_index[CONTACT_POINTS_SUBDIR].append(item)
        print(
            f"Exported contact point {item['uid'] or item['name'] or 'unknown'} -> {output_path}"
        )


def export_mute_timing_documents(
    mute_timings: List[Dict[str, Any]],
    resource_dirs: Dict[str, Path],
    root_index: Dict[str, List[Dict[str, str]]],
    flat: bool,
    overwrite: bool,
) -> None:
    """Export mute timings and append mute-timing entries to the root index."""
    for mute_timing in mute_timings:
        document = build_mute_timing_export_document(mute_timing)
        spec = document["spec"]
        output_path = build_mute_timing_output_path(
            resource_dirs[MUTE_TIMING_KIND], spec, flat
        )
        write_json(document, output_path, overwrite)
        item = {
            "kind": MUTE_TIMING_KIND,
            "name": str(spec.get("name") or ""),
            "path": str(output_path),
        }
        root_index[MUTE_TIMINGS_SUBDIR].append(item)
        print(f"Exported mute timing {item['name'] or 'unknown'} -> {output_path}")


def export_policies_document(
    policies: Dict[str, Any],
    resource_dirs: Dict[str, Path],
    root_index: Dict[str, List[Dict[str, str]]],
    overwrite: bool,
) -> None:
    """Export the single notification policy tree and append its index entry."""
    policies_document = build_policies_export_document(policies)
    policies_path = build_policies_output_path(resource_dirs[POLICIES_KIND])
    write_json(policies_document, policies_path, overwrite)
    policies_item = {
        "kind": POLICIES_KIND,
        "receiver": str(policies_document["spec"].get("receiver") or ""),
        "path": str(policies_path),
    }
    root_index[POLICIES_SUBDIR].append(policies_item)
    print(
        "Exported notification policies "
        f"{policies_item['receiver'] or 'unknown'} -> {policies_path}"
    )


def export_template_documents(
    templates: List[Dict[str, Any]],
    resource_dirs: Dict[str, Path],
    root_index: Dict[str, List[Dict[str, str]]],
    flat: bool,
    overwrite: bool,
) -> None:
    """Export notification templates and append template entries to the root index."""
    for template in templates:
        document = build_template_export_document(template)
        spec = document["spec"]
        output_path = build_template_output_path(
            resource_dirs[TEMPLATE_KIND], spec, flat
        )
        write_json(document, output_path, overwrite)
        item = {
            "kind": TEMPLATE_KIND,
            "name": str(spec.get("name") or ""),
            "path": str(output_path),
        }
        root_index[TEMPLATES_SUBDIR].append(item)
        print(f"Exported template {item['name'] or 'unknown'} -> {output_path}")


def write_resource_indexes(
    resource_dirs: Dict[str, Path],
    root_index: Dict[str, List[Dict[str, str]]],
) -> None:
    """Write per-resource index files under the raw export tree."""
    for kind, subdir in RESOURCE_SUBDIR_BY_KIND.items():
        write_json(
            root_index[subdir],
            resource_dirs[kind] / "index.json",
            overwrite=True,
        )


def format_export_summary(
    root_index: Dict[str, List[Dict[str, str]]],
    index_path: Path,
) -> str:
    """Build the final export summary line shown to operators."""
    return (
        "Exported "
        f"{len(root_index[RULES_SUBDIR])} alert rules, "
        f"{len(root_index[CONTACT_POINTS_SUBDIR])} contact points, "
        f"{len(root_index[MUTE_TIMINGS_SUBDIR])} mute timings, "
        f"{len(root_index[POLICIES_SUBDIR])} notification policy documents, "
        f"{len(root_index[TEMPLATES_SUBDIR])} templates. "
        f"Root index: {index_path}"
    )


def export_alerting_resources(args: argparse.Namespace) -> int:
    """Export supported alerting resources into the tool-owned JSON layout."""
    client = build_client(args)
    output_dir = Path(args.output_dir)
    raw_dir = output_dir / RAW_EXPORT_SUBDIR
    output_dir.mkdir(parents=True, exist_ok=True)
    raw_dir.mkdir(parents=True, exist_ok=True)

    resource_dirs = build_resource_dirs(raw_dir)
    for path in resource_dirs.values():
        path.mkdir(parents=True, exist_ok=True)

    rules = client.list_alert_rules()
    contact_points = client.list_contact_points()
    mute_timings = client.list_mute_timings()
    policies = client.get_notification_policies()
    templates = client.list_templates()

    root_index = build_empty_root_index()
    export_rule_documents(
        client,
        rules,
        resource_dirs,
        root_index,
        flat=args.flat,
        overwrite=args.overwrite,
    )
    export_contact_point_documents(
        contact_points,
        resource_dirs,
        root_index,
        flat=args.flat,
        overwrite=args.overwrite,
    )
    export_mute_timing_documents(
        mute_timings,
        resource_dirs,
        root_index,
        flat=args.flat,
        overwrite=args.overwrite,
    )
    export_policies_document(
        policies,
        resource_dirs,
        root_index,
        overwrite=args.overwrite,
    )
    export_template_documents(
        templates,
        resource_dirs,
        root_index,
        flat=args.flat,
        overwrite=args.overwrite,
    )
    write_resource_indexes(resource_dirs, root_index)

    index_path = output_dir / "index.json"
    write_json(root_index, index_path, overwrite=True)
    print(format_export_summary(root_index, index_path))
    return 0


def count_policy_documents(kind: str, policies_seen: int) -> int:
    """Track notification policy documents and reject import sets with more than one."""
    if kind != POLICIES_KIND:
        return policies_seen

    policies_seen += 1
    if policies_seen > 1:
        raise GrafanaError(
            "Multiple notification policy documents found in import set. "
            "Import only one policy tree at a time."
        )
    return policies_seen


def import_rule_document(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> Tuple[str, str]:
    """Import one alert rule and return the action plus stable identity."""
    uid = str(payload.get("uid") or "")
    if replace_existing and uid:
        try:
            client.get_alert_rule(uid)
        except GrafanaApiError as exc:
            if exc.status_code != 404:
                raise
        else:
            result = client.update_alert_rule(uid, payload)
            return "updated", str(result.get("uid") or uid or "unknown")

    result = client.create_alert_rule(payload)
    return "created", str(result.get("uid") or uid or "unknown")


def import_contact_point_document(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> Tuple[str, str]:
    """Import one contact point and return the action plus stable identity."""
    uid = str(payload.get("uid") or "")
    if replace_existing and uid:
        existing = {str(item.get("uid") or "") for item in client.list_contact_points()}
        if uid in existing:
            result = client.update_contact_point(uid, payload)
            return "updated", str(result.get("uid") or uid or payload.get("name") or "unknown")

    result = client.create_contact_point(payload)
    return "created", str(result.get("uid") or uid or payload.get("name") or "unknown")


def import_mute_timing_document(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> Tuple[str, str]:
    """Import one mute timing and return the action plus stable identity."""
    name = str(payload.get("name") or "")
    if replace_existing and name:
        existing = {str(item.get("name") or "") for item in client.list_mute_timings()}
        if name in existing:
            result = client.update_mute_timing(name, payload)
            return "updated", str(result.get("name") or name or "unknown")

    result = client.create_mute_timing(payload)
    return "created", str(result.get("name") or name or "unknown")


def build_template_update_payload(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> Tuple[str, Dict[str, Any], bool]:
    """Prepare the template payload and report whether the template already exists."""
    name = str(payload.get("name") or "")
    existing_names = {str(item.get("name") or "") for item in client.list_templates()}
    exists = name in existing_names
    if exists and not replace_existing:
        raise GrafanaError(
            f"Template {name!r} already exists. Use --replace-existing."
        )

    template_payload = dict(payload)
    if exists:
        current_template = client.get_template(name)
        template_payload["version"] = str(current_template.get("version") or "")
    else:
        template_payload["version"] = ""
    return name, template_payload, exists


def import_template_document(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> Tuple[str, str]:
    """Import one notification template and return the action plus stable identity."""
    name, template_payload, exists = build_template_update_payload(
        client,
        payload,
        replace_existing,
    )
    result = client.update_template(name, template_payload)
    action = "updated" if exists else "created"
    return action, str(result.get("name") or name or "unknown")


def import_policies_document(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
) -> Tuple[str, str]:
    """Import the single notification policy tree and return its identity."""
    client.update_notification_policies(payload)
    return "updated", str(payload.get("receiver") or "root")


def import_resource_document(
    client: GrafanaAlertClient,
    kind: str,
    payload: Dict[str, Any],
    args: argparse.Namespace,
) -> Tuple[str, str]:
    """Dispatch one import document to the correct per-kind import handler."""
    if kind == RULE_KIND:
        return import_rule_document(client, payload, args.replace_existing)
    if kind == CONTACT_POINT_KIND:
        return import_contact_point_document(client, payload, args.replace_existing)
    if kind == MUTE_TIMING_KIND:
        return import_mute_timing_document(client, payload, args.replace_existing)
    if kind == TEMPLATE_KIND:
        return import_template_document(client, payload, args.replace_existing)
    return import_policies_document(client, payload)


def import_alerting_resources(args: argparse.Namespace) -> int:
    """Import alerting resource documents back into Grafana provisioning APIs."""
    client = build_client(args)
    import_dir = Path(args.import_dir)
    resource_files = discover_alert_resource_files(import_dir)
    policies_seen = 0
    dashboard_uid_map = load_string_map(args.dashboard_uid_map, "Dashboard UID map")
    panel_id_map = load_panel_id_map(args.panel_id_map)

    for resource_file in resource_files:
        document = load_json_file(resource_file)
        kind, payload = build_import_operation(document)
        payload = prepare_import_payload_for_target(
            client,
            kind,
            payload,
            document,
            dashboard_uid_map,
            panel_id_map,
        )
        policies_seen = count_policy_documents(kind, policies_seen)
        identity = build_resource_identity(kind, payload)
        if args.dry_run:
            action = determine_import_action(
                client,
                kind,
                payload,
                args.replace_existing,
            )
            print(f"Dry-run {resource_file} -> kind={kind} id={identity} action={action}")
            continue

        action, identity = import_resource_document(client, kind, payload, args)

        print(f"Imported {resource_file} -> kind={kind} id={identity} action={action}")

    if args.dry_run:
        print(f"Dry-run checked {len(resource_files)} alerting resource files from {import_dir}")
    else:
        print(f"Imported {len(resource_files)} alerting resource files from {import_dir}")
    return 0


def diff_alerting_resources(args: argparse.Namespace) -> int:
    """Compare local alerting export files with the current Grafana state."""
    client = build_client(args)
    diff_dir = Path(args.diff_dir)
    resource_files = discover_alert_resource_files(diff_dir)
    policies_seen = 0
    dashboard_uid_map = load_string_map(args.dashboard_uid_map, "Dashboard UID map")
    panel_id_map = load_panel_id_map(args.panel_id_map)
    differences = 0

    for resource_file in resource_files:
        document = load_json_file(resource_file)
        kind, payload = build_import_operation(document)
        payload = prepare_import_payload_for_target(
            client,
            kind,
            payload,
            document,
            dashboard_uid_map,
            panel_id_map,
        )
        policies_seen = count_policy_documents(kind, policies_seen)
        identity = build_resource_identity(kind, payload)
        local_compare = build_compare_document(kind, payload)
        remote_compare = fetch_live_compare_document(client, kind, payload)
        if remote_compare is None:
            print(f"Diff missing-remote {resource_file} -> kind={kind} id={identity}")
            print_unified_diff(
                {},
                local_compare,
                build_diff_label("remote", resource_file, kind, identity),
                build_diff_label("local", resource_file, kind, identity),
            )
            differences += 1
            continue

        if serialize_compare_document(local_compare) == serialize_compare_document(
            remote_compare
        ):
            print(f"Diff same {resource_file} -> kind={kind} id={identity}")
            continue

        print(f"Diff different {resource_file} -> kind={kind} id={identity}")
        print_unified_diff(
            remote_compare,
            local_compare,
            build_diff_label("remote", resource_file, kind, identity),
            build_diff_label("local", resource_file, kind, identity),
        )
        differences += 1

    if differences:
        print(
            "Found "
            f"{differences} alerting differences across {len(resource_files)} files."
        )
        return 1

    print(f"No alerting differences across {len(resource_files)} files.")
    return 0


def build_client(args: argparse.Namespace) -> GrafanaAlertClient:
    """Build the alerting API client from parsed CLI arguments."""
    headers = resolve_auth(args)
    return GrafanaAlertClient(
        base_url=args.url,
        headers=headers,
        timeout=args.timeout,
        verify_ssl=args.verify_ssl,
    )


def list_alert_resources(args: argparse.Namespace) -> int:
    client = build_client(args)
    command = getattr(args, "alert_command", "")
    if command == "list-rules":
        rows = serialize_rule_list_rows(client.list_alert_rules())
        fields = ALERT_RULE_LIST_FIELDS
        headers = {
            "uid": "UID",
            "title": "Title",
            "folderUID": "Folder UID",
            "ruleGroup": "Rule Group",
        }
    elif command == "list-contact-points":
        rows = serialize_contact_point_list_rows(client.list_contact_points())
        fields = CONTACT_POINT_LIST_FIELDS
        headers = {"uid": "UID", "name": "Name", "type": "Type"}
    elif command == "list-mute-timings":
        rows = serialize_mute_timing_list_rows(client.list_mute_timings())
        fields = MUTE_TIMING_LIST_FIELDS
        headers = {"name": "Name", "intervals": "Intervals"}
    elif command == "list-templates":
        rows = serialize_template_list_rows(client.list_templates())
        fields = TEMPLATE_LIST_FIELDS
        headers = {"name": "Name"}
    else:
        raise GrafanaError("Unsupported alert list command.")

    if args.json:
        print(render_alert_list_json(rows))
        return 0
    if args.csv:
        render_alert_list_csv(rows, fields)
        return 0
    for line in build_alert_list_table(rows, fields, headers, include_header=not args.no_header):
        print(line)
    return 0


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    try:
        if getattr(args, "alert_command", "").startswith("list-"):
            return list_alert_resources(args)
        if getattr(args, "alert_command", None) == "import":
            return import_alerting_resources(args)
        if getattr(args, "alert_command", None) == "diff":
            return diff_alerting_resources(args)
        return export_alerting_resources(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
