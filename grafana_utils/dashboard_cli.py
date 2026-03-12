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
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

from .clients.dashboard_client import GrafanaClient
from .dashboards.common import GrafanaApiError, GrafanaError
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
TOOL_SCHEMA_VERSION = 1
ROOT_INDEX_KIND = "grafana-utils-dashboard-export-index"


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
        help="Export dashboards from this Grafana organization ID instead of the current org context.",
    )
    parser.add_argument(
        "--all-orgs",
        action="store_true",
        help="Export dashboards from every Grafana organization. Requires Basic auth.",
    )
    parser.add_argument(
        "--flat",
        action="store_true",
        help="Write all dashboards into the export root instead of per-folder directories.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing dashboard files if they already exist.",
    )
    parser.add_argument(
        "--without-dashboard-raw",
        action="store_true",
        help=f"Skip exporting the {RAW_EXPORT_SUBDIR}/ variant.",
    )
    parser.add_argument(
        "--without-dashboard-prompt",
        action="store_true",
        help=f"Skip exporting the {PROMPT_EXPORT_SUBDIR}/ variant.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview the dashboard files and indexes that would be written without changing disk.",
    )
    parser.add_argument(
        "--progress",
        action="store_true",
        help="Show per-dashboard export progress while processing files.",
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
            "Fetch each dashboard payload and include resolved datasource names. "
            "This makes extra API calls."
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
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly."
        ),
    )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="Allow imports to replace existing dashboards with the same UID.",
    )
    parser.add_argument(
        "--import-folder-uid",
        default=None,
        help="Override the destination Grafana folder UID for all imported dashboards.",
    )
    parser.add_argument(
        "--import-message",
        default="Imported by grafana-utils",
        help="Version history message to attach to imported dashboards.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show whether each dashboard would be created or updated without importing it.",
    )
    parser.add_argument(
        "--progress",
        action="store_true",
        help="Show per-dashboard import progress while processing files.",
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
    folder_title = summary.get("folderTitle") or "General"
    folder_name = sanitize_path_component(folder_title)
    title = sanitize_path_component(summary.get("title") or "dashboard")
    uid = sanitize_path_component(summary.get("uid") or "unknown")
    filename = f"{title}__{uid}.json"
    if flat:
        return output_dir / filename
    return output_dir / folder_name / filename


def build_all_orgs_output_dir(
    output_dir: Path,
    org: Dict[str, Any],
) -> Path:
    """Return one org-prefixed export directory for multi-org dashboard exports."""
    org_id = sanitize_path_component(str(org.get("id") or "unknown"))
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
        if path.name not in {"index.json", EXPORT_METADATA_FILENAME}
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
        "org": str(summary.get("orgName") or "Main Org."),
        "orgId": str(summary.get("orgId") or "1"),
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
    return metadata


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
) -> str:
    """Predict whether one dashboard import would create, update, or fail."""
    uid = str(payload["dashboard"].get("uid") or "")
    if not uid:
        return "would-create"

    try:
        client.fetch_dashboard(uid)
    except GrafanaApiError as exc:
        if exc.status_code == 404:
            return "would-create"
        raise

    if replace_existing:
        return "would-update"
    return "would-fail-existing"


def export_dashboards(args: argparse.Namespace) -> int:
    """Export dashboards into raw JSON, prompt JSON, or both variants."""
    if args.without_dashboard_raw and args.without_dashboard_prompt:
        raise GrafanaError(
            "Nothing to export. Remove one of --without-dashboard-raw or --without-dashboard-prompt."
        )

    output_dir = Path(args.export_dir)
    export_raw = not args.without_dashboard_raw
    export_prompt = not args.without_dashboard_prompt
    client = build_client(args)
    all_orgs = bool(getattr(args, "all_orgs", False))
    org_id = getattr(args, "org_id", None)
    if all_orgs and org_id:
        raise GrafanaError("Choose either --org-id or --all-orgs, not both.")
    auth_header = client.headers.get("Authorization", "")
    if (all_orgs or org_id) and not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Dashboard org switching requires Basic auth. Use --basic-user and --basic-password."
        )

    clients = [client]
    if all_orgs:
        clients = []
        for org in client.list_orgs():
            scoped_org_id = str(org.get("id") or "").strip()
            if scoped_org_id:
                clients.append((org, client.with_org_id(scoped_org_id)))
    elif org_id:
        scoped_client = client.with_org_id(str(org_id))
        clients = [(scoped_client.fetch_current_org(), scoped_client)]
    else:
        clients = [(client.fetch_current_org(), client)]

    index_items: List[Dict[str, str]] = []
    for org, scoped_client in clients:
        scoped_output_dir = output_dir
        if all_orgs:
            scoped_output_dir = build_all_orgs_output_dir(output_dir, org)
        raw_dir, prompt_dir = build_export_variant_dirs(scoped_output_dir)
        datasource_catalog = None
        if export_prompt:
            # Prompt exports need datasource metadata up front so dashboard references
            # can be rewritten into Grafana's __inputs import format.
            datasource_catalog = build_datasource_catalog(scoped_client.list_datasources())

        summaries = attach_dashboard_org(
            scoped_client,
            scoped_client.iter_dashboard_summaries(args.page_size),
        )
        if not summaries:
            continue

        for summary in summaries:
            uid = str(summary["uid"])
            payload = scoped_client.fetch_dashboard(uid)
            item = build_dashboard_index_item(summary, uid)
            if export_raw:
                raw_document = build_preserved_web_import_document(payload)
                raw_path = build_output_path(raw_dir, summary, args.flat)
                if args.dry_run:
                    ensure_dashboard_write_target(
                        raw_path,
                        args.overwrite,
                        create_parents=False,
                    )
                    if args.progress:
                        print(f"Would export raw    {uid} -> {raw_path}")
                else:
                    write_dashboard(raw_document, raw_path, args.overwrite)
                    if args.progress:
                        print(f"Exported raw    {uid} -> {raw_path}")
                item["raw_path"] = str(raw_path)
            if export_prompt:
                assert datasource_catalog is not None
                prompt_document = build_external_export_document(payload, datasource_catalog)
                prompt_path = build_output_path(prompt_dir, summary, args.flat)
                if args.dry_run:
                    ensure_dashboard_write_target(
                        prompt_path,
                        args.overwrite,
                        create_parents=False,
                    )
                    if args.progress:
                        print(f"Would export prompt {uid} -> {prompt_path}")
                else:
                    write_dashboard(prompt_document, prompt_path, args.overwrite)
                    if args.progress:
                        print(f"Exported prompt {uid} -> {prompt_path}")
                item["prompt_path"] = str(prompt_path)
            index_items.append(item)

    if not index_items:
        print("No dashboards found.", file=sys.stderr)
        return 0

    raw_index_path = None
    raw_metadata_path = None
    if export_raw:
        raw_variant_dir = output_dir / RAW_EXPORT_SUBDIR if all_orgs else raw_dir
        raw_index_path = raw_variant_dir / "index.json"
        raw_metadata_path = raw_variant_dir / EXPORT_METADATA_FILENAME
        raw_index = build_variant_index(
            index_items,
            "raw_path",
            "grafana-web-import-preserve-uid",
        )
        raw_metadata = build_export_metadata(
            variant=RAW_EXPORT_SUBDIR,
            dashboard_count=len(raw_index),
            format_name="grafana-web-import-preserve-uid",
        )
        if not args.dry_run:
            write_json_document(raw_index, raw_index_path)
            write_json_document(raw_metadata, raw_metadata_path)
    prompt_index_path = None
    prompt_metadata_path = None
    if export_prompt:
        prompt_variant_dir = output_dir / PROMPT_EXPORT_SUBDIR if all_orgs else prompt_dir
        prompt_index_path = prompt_variant_dir / "index.json"
        prompt_metadata_path = prompt_variant_dir / EXPORT_METADATA_FILENAME
        prompt_index = build_variant_index(
            index_items,
            "prompt_path",
            "grafana-web-import-with-datasource-inputs",
        )
        prompt_metadata = build_export_metadata(
            variant=PROMPT_EXPORT_SUBDIR,
            dashboard_count=len(prompt_index),
            format_name="grafana-web-import-with-datasource-inputs",
        )
        if not args.dry_run:
            write_json_document(prompt_index, prompt_index_path)
            write_json_document(prompt_metadata, prompt_metadata_path)
    index_path = output_dir / "index.json"
    root_index = build_root_export_index(index_items, raw_index_path, prompt_index_path)
    root_metadata_path = output_dir / EXPORT_METADATA_FILENAME
    root_metadata = build_export_metadata(
        variant="root",
        dashboard_count=len(index_items),
    )
    if not args.dry_run:
        write_json_document(root_index, index_path)
        write_json_document(root_metadata, root_metadata_path)
    summary_verb = "Would export" if args.dry_run else "Exported"
    summary_parts = [f"{summary_verb} {len(index_items)} dashboards."]
    if raw_index_path is not None:
        summary_parts.append(f"Raw index: {raw_index_path}")
    if raw_metadata_path is not None:
        summary_parts.append(f"Raw manifest: {raw_metadata_path}")
    if prompt_index_path is not None:
        summary_parts.append(f"Prompt index: {prompt_index_path}")
    if prompt_metadata_path is not None:
        summary_parts.append(f"Prompt manifest: {prompt_metadata_path}")
    summary_parts.append(f"Root index: {index_path}")
    summary_parts.append(f"Root manifest: {root_metadata_path}")
    print(" ".join(summary_parts))
    return 0


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
    folder = str(summary.get("folderTitle") or "General")
    record = {
        "uid": str(summary.get("uid") or "unknown"),
        "name": str(summary.get("title") or "dashboard"),
        "folder": folder,
        "folderUid": str(summary.get("folderUid") or "general"),
        "path": str(summary.get("folderPath") or folder),
        "org": str(summary.get("orgName") or "Main Org."),
        "orgId": str(summary.get("orgId") or "1"),
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
    title = str(folder.get("title") or fallback_title or "General").strip() or "General"
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
        folder_title = str(summary.get("folderTitle") or "General")
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
        folder_title = str(item.get("folderTitle") or "General")
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
    org_name = str(org.get("name") or "Main Org.")
    org_id = str(org.get("id") or "1")
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
        if getattr(args, "with_sources", False):
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


def import_dashboards(args: argparse.Namespace) -> int:
    """Import previously exported raw dashboard JSON files through Grafana's API."""
    client = build_client(args)
    import_dir = Path(args.import_dir)
    load_export_metadata(import_dir, expected_variant=RAW_EXPORT_SUBDIR)
    dashboard_files = discover_dashboard_files(import_dir)

    for dashboard_file in dashboard_files:
        document = load_json_file(dashboard_file)
        payload = build_import_payload(
            document=document,
            folder_uid_override=args.import_folder_uid,
            replace_existing=args.replace_existing,
            message=args.import_message,
        )
        uid = payload["dashboard"].get("uid") or "unknown"
        if args.dry_run:
            action = determine_dashboard_import_action(
                client,
                payload,
                args.replace_existing,
            )
            if args.progress:
                print(f"Dry-run {dashboard_file} -> uid={uid} action={action}")
            continue

        result = client.import_dashboard(payload)
        status = result.get("status", "unknown")
        uid = result.get("uid") or uid
        if args.progress:
            print(f"Imported {dashboard_file} -> uid={uid} status={status}")

    if args.dry_run:
        print(f"Dry-run checked {len(dashboard_files)} dashboard files from {import_dir}")
    else:
        print(f"Imported {len(dashboard_files)} dashboard files from {import_dir}")
    return 0


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
