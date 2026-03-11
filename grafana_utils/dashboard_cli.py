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
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple
from urllib import parse

from .http_transport import (
    JsonHttpTransport,
    HttpTransportApiError,
    HttpTransportError,
    build_json_http_transport,
)


DEFAULT_URL = "http://127.0.0.1:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_PAGE_SIZE = 500
DEFAULT_EXPORT_DIR = "dashboards"
RAW_EXPORT_SUBDIR = "raw"
PROMPT_EXPORT_SUBDIR = "prompt"
EXPORT_METADATA_FILENAME = "export-metadata.json"
TOOL_SCHEMA_VERSION = 1
ROOT_INDEX_KIND = "grafana-utils-dashboard-export-index"
BUILTIN_DATASOURCE_TYPES = {"__expr__", "grafana"}
BUILTIN_DATASOURCE_NAMES = {
    "-- Dashboard --",
    "-- Grafana --",
    "-- Mixed --",
    "grafana",
    "expr",
    "__expr__",
}
DATASOURCE_TYPE_ALIASES = {
    "prom": "prometheus",
    "prometheus": "prometheus",
    "loki": "loki",
    "elastic": "elasticsearch",
    "elasticsearch": "elasticsearch",
    "opensearch": "grafana-opensearch-datasource",
    "mysql": "mysql",
    "postgres": "postgres",
    "postgresql": "postgres",
    "mssql": "mssql",
    "influxdb": "influxdb",
    "tempo": "tempo",
    "jaeger": "jaeger",
    "zipkin": "zipkin",
    "cloudwatch": "cloudwatch",
}


class GrafanaError(RuntimeError):
    """Raised when Grafana returns an unexpected response."""


class GrafanaApiError(GrafanaError):
    """Raised when Grafana returns an HTTP error response."""

    def __init__(self, status_code: int, url: str, body: str) -> None:
        self.status_code = status_code
        self.url = url
        self.body = body
        super().__init__(f"Grafana API error {status_code} for {url}: {body}")


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


def add_list_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--page-size",
        type=int,
        default=DEFAULT_PAGE_SIZE,
        help=f"Dashboard search page size (default: {DEFAULT_PAGE_SIZE}).",
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
        description="Export or import Grafana dashboards."
    )
    # Keep export-only and import-only flags on separate subcommands so the
    # operator must choose the intended mode explicitly at the CLI boundary.
    subparsers = parser.add_subparsers(dest="command")
    subparsers.required = True

    export_parser = subparsers.add_parser(
        "export",
        help="Export dashboards into raw/ and prompt/ variants.",
    )
    add_common_cli_args(export_parser)
    add_export_cli_args(export_parser)

    list_parser = subparsers.add_parser(
        "list",
        help="List live dashboard summaries from Grafana.",
    )
    add_common_cli_args(list_parser)
    add_list_cli_args(list_parser)

    import_parser = subparsers.add_parser(
        "import",
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

    if cli_token and (cli_username or cli_password):
        raise GrafanaError(
            "Choose either token auth (--token / --api-token) or Basic auth "
            "(--basic-user / --username with --basic-password / --password), not both."
        )
    if cli_username and not cli_password:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password."
        )
    if cli_password and not cli_username:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password."
        )

    token = cli_token or env_value("GRAFANA_API_TOKEN")
    if token:
        return {"Authorization": f"Bearer {token}"}

    username = cli_username or env_value("GRAFANA_USERNAME")
    password = cli_password or env_value("GRAFANA_PASSWORD")
    if username and password:
        encoded = base64.b64encode(f"{username}:{password}".encode("utf-8")).decode(
            "ascii"
        )
        return {"Authorization": f"Basic {encoded}"}
    if username or password:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password."
        )

    raise GrafanaError(
        "Authentication required. Set --token / --api-token / GRAFANA_API_TOKEN "
        "or --basic-user and --basic-password / GRAFANA_USERNAME and GRAFANA_PASSWORD."
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


def build_export_variant_dirs(output_dir: Path) -> Tuple[Path, Path]:
    """Return the raw/ and prompt/ export directories for one dashboard export root."""
    return output_dir / RAW_EXPORT_SUBDIR, output_dir / PROMPT_EXPORT_SUBDIR


class GrafanaClient:
    """Minimal HTTP wrapper around the Grafana dashboard APIs used by this script."""

    def __init__(
        self,
        base_url: str,
        headers: Dict[str, str],
        timeout: int,
        verify_ssl: bool,
        transport: Optional[JsonHttpTransport] = None,
    ) -> None:
        self.transport = transport or build_json_http_transport(
            base_url=base_url,
            headers={"Accept": "application/json", **headers},
            timeout=timeout,
            verify_ssl=verify_ssl,
        )

    def request_json(
        self,
        path: str,
        params: Optional[Dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[Dict[str, Any]] = None,
    ) -> Any:
        """Send one request to Grafana and decode the JSON response."""
        try:
            return self.transport.request_json(
                path=path,
                params=params,
                method=method,
                payload=payload,
            )
        except HttpTransportApiError as exc:
            raise GrafanaApiError(exc.status_code, exc.url, exc.body) from exc
        except HttpTransportError as exc:
            raise GrafanaError(str(exc)) from exc

    def iter_dashboard_summaries(self, page_size: int) -> List[Dict[str, Any]]:
        """List dashboards through Grafana search pagination and deduplicate by UID."""
        dashboards: List[Dict[str, Any]] = []
        seen_uids: Set[str] = set()
        page = 1

        while True:
            batch = self.request_json(
                "/api/search",
                params={"type": "dash-db", "limit": page_size, "page": page},
            )
            if not isinstance(batch, list):
                raise GrafanaError("Unexpected search response from Grafana.")
            if not batch:
                break

            for item in batch:
                uid = item.get("uid")
                if not uid or uid in seen_uids:
                    continue
                seen_uids.add(uid)
                dashboards.append(item)

            if len(batch) < page_size:
                break
            page += 1

        return dashboards

    def fetch_folder_if_exists(self, uid: str) -> Optional[Dict[str, Any]]:
        """Fetch one folder payload or return None when the folder UID is missing."""
        try:
            data = self.request_json(f"/api/folders/{parse.quote(uid, safe='')}")
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected folder payload for UID {uid}.")
        return data

    def fetch_dashboard(self, uid: str) -> Dict[str, Any]:
        """Fetch the full dashboard wrapper for a single Grafana UID."""
        data = self.fetch_dashboard_if_exists(uid)
        if data is None:
            raise GrafanaApiError(
                404,
                f"/api/dashboards/uid/{parse.quote(uid, safe='')}",
                "Dashboard not found",
            )
        if not isinstance(data, dict) or "dashboard" not in data:
            raise GrafanaError(f"Unexpected dashboard payload for UID {uid}.")
        return data

    def fetch_dashboard_if_exists(self, uid: str) -> Optional[Dict[str, Any]]:
        """Fetch the full dashboard wrapper or return None when the UID is missing."""
        data = None
        try:
            data = self.request_json(f"/api/dashboards/uid/{parse.quote(uid, safe='')}")
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        if not isinstance(data, dict) or "dashboard" not in data:
            raise GrafanaError(f"Unexpected dashboard payload for UID {uid}.")
        return data

    def import_dashboard(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Create or update a dashboard through POST /api/dashboards/db."""
        data = self.request_json(
            "/api/dashboards/db",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected dashboard import response from Grafana.")
        return data

    def list_datasources(self) -> List[Dict[str, Any]]:
        """List datasource objects used when building prompt-style exports."""
        data = self.request_json("/api/datasources")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected datasource list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]


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


def build_datasource_catalog(
    datasources: List[Dict[str, Any]],
) -> Tuple[Dict[str, Dict[str, Any]], Dict[str, Dict[str, Any]]]:
    """Index datasources by both uid and name because dashboards use either form."""
    by_uid: Dict[str, Dict[str, Any]] = {}
    by_name: Dict[str, Dict[str, Any]] = {}
    for datasource in datasources:
        uid = datasource.get("uid")
        name = datasource.get("name")
        if isinstance(uid, str) and uid:
            by_uid[uid] = datasource
        if isinstance(name, str) and name:
            by_name[name] = datasource
    return by_uid, by_name


def is_placeholder_string(value: str) -> bool:
    return value.startswith("$")


def extract_placeholder_name(value: str) -> str:
    if value.startswith("${") and value.endswith("}") and len(value) > 3:
        return value[2:-1]
    if value.startswith("$") and len(value) > 1:
        return value[1:]
    return value


def is_generated_input_placeholder(value: str) -> bool:
    return extract_placeholder_name(value).startswith("DS_")


def is_builtin_datasource_ref(value: Any) -> bool:
    if isinstance(value, str):
        return value in BUILTIN_DATASOURCE_NAMES or is_generated_input_placeholder(value)
    if isinstance(value, dict):
        uid = value.get("uid")
        name = value.get("name")
        ds_type = value.get("type")
        if isinstance(uid, str) and is_generated_input_placeholder(uid):
            return True
        if isinstance(name, str) and is_generated_input_placeholder(name):
            return True
        if uid in BUILTIN_DATASOURCE_TYPES or ds_type in BUILTIN_DATASOURCE_TYPES:
            return True
    return False


def collect_datasource_refs(node: Any, refs: List[Any]) -> None:
    """Walk the full dashboard tree and collect every datasource reference in place."""
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "datasource":
                refs.append(value)
            collect_datasource_refs(value, refs)
        return
    if isinstance(node, list):
        for item in node:
            collect_datasource_refs(item, refs)


def make_input_name(label: str) -> str:
    normalized = re.sub(r"[^A-Z0-9]+", "_", label.upper()).strip("_")
    normalized = re.sub(r"_+", "_", normalized)
    return f"DS_{normalized or 'DATASOURCE'}"


def make_type_input_base(datasource_type: str) -> str:
    alias = DATASOURCE_TYPE_ALIASES.get(datasource_type.lower(), datasource_type)
    return make_input_name(alias)


def make_input_label(datasource_type: str, index: int) -> str:
    alias = DATASOURCE_TYPE_ALIASES.get(datasource_type.lower(), datasource_type)
    title = alias.replace("-", " ").replace("_", " ").title()
    if index == 1:
        return f"{title} datasource"
    return f"{title} datasource {index}"


def build_resolved_datasource(key: str, label: str, ds_type: str) -> Dict[str, str]:
    """Create the normalized datasource descriptor used by prompt export helpers."""
    return {"key": key, "label": label, "type": ds_type}


def lookup_datasource(
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
    uid: Optional[str] = None,
    name: Optional[str] = None,
) -> Optional[Dict[str, Any]]:
    """Resolve a datasource by UID first, then by datasource name."""
    if isinstance(uid, str) and uid:
        datasource = datasources_by_uid.get(uid)
        if datasource is not None:
            return datasource
    if isinstance(name, str) and name:
        return datasources_by_name.get(name)
    return None


def resolve_datasource_type_alias(
    ref: str,
    datasources_by_uid: Dict[str, Dict[str, Any]],
) -> Optional[str]:
    """Resolve datasource plugin aliases such as 'prometheus' or 'prom'."""
    ref_lower = ref.lower()
    datasource_type = DATASOURCE_TYPE_ALIASES.get(ref_lower)
    if datasource_type is not None:
        return datasource_type

    for candidate in datasources_by_uid.values():
        candidate_type = candidate.get("type")
        if isinstance(candidate_type, str) and candidate_type.lower() == ref_lower:
            return candidate_type
    return None


def resolve_string_datasource_ref(
    ref: str,
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Dict[str, str]:
    """Resolve string datasource references stored as names, UIDs, or type aliases."""
    datasource = lookup_datasource(
        datasources_by_uid,
        datasources_by_name,
        uid=ref,
        name=ref,
    )
    if datasource is None:
        datasource_type = resolve_datasource_type_alias(ref, datasources_by_uid)
        if datasource_type is not None:
            return build_resolved_datasource(
                f"type:{datasource_type}",
                datasource_type,
                datasource_type,
            )
        raise GrafanaError(
            f"Cannot resolve datasource name or uid {ref!r} for prompt export."
        )

    uid = datasource.get("uid") or ref
    label = datasource.get("name") or ref
    ds_type = datasource.get("type")
    if not isinstance(ds_type, str) or not ds_type:
        raise GrafanaError(f"Datasource {ref!r} does not have a usable type.")
    return build_resolved_datasource(f"uid:{uid}", label, ds_type)


def resolve_placeholder_object_ref(
    uid: Any,
    name: Any,
    ds_type: Any,
) -> Optional[Dict[str, str]]:
    """Resolve object refs that already point at a datasource placeholder token."""
    if not isinstance(ds_type, str) or not ds_type:
        return None

    placeholder_value = None
    if isinstance(uid, str) and is_placeholder_string(uid):
        placeholder_value = uid
    elif isinstance(name, str) and is_placeholder_string(name):
        placeholder_value = name
    if placeholder_value is None:
        return None

    token = extract_placeholder_name(placeholder_value)
    return build_resolved_datasource(f"var:{ds_type}:{token}", token, ds_type)


def resolve_object_datasource_ref(
    ref: Dict[str, Any],
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Optional[Dict[str, str]]:
    """Resolve object datasource refs stored as {'type': ..., 'uid': ...}."""
    uid = ref.get("uid")
    name = ref.get("name")
    ds_type = ref.get("type")
    has_placeholder = (
        isinstance(uid, str)
        and is_placeholder_string(uid)
        or isinstance(name, str)
        and is_placeholder_string(name)
    )

    resolved = resolve_placeholder_object_ref(uid, name, ds_type)
    if resolved is not None:
        return resolved
    if has_placeholder:
        return None

    datasource = lookup_datasource(
        datasources_by_uid,
        datasources_by_name,
        uid=uid,
        name=name,
    )
    resolved_type = ds_type
    resolved_label = name or uid
    resolved_uid = uid or name
    if datasource is not None:
        resolved_type = datasource.get("type") or resolved_type
        resolved_label = datasource.get("name") or resolved_label
        resolved_uid = datasource.get("uid") or resolved_uid

    if not isinstance(resolved_type, str) or not resolved_type:
        raise GrafanaError(
            f"Cannot resolve datasource type from reference {ref!r}."
        )
    if not isinstance(resolved_label, str) or not resolved_label:
        resolved_label = resolved_type
    if not isinstance(resolved_uid, str) or not resolved_uid:
        resolved_uid = resolved_label

    return build_resolved_datasource(
        f"uid:{resolved_uid}",
        resolved_label,
        resolved_type,
    )


def resolve_datasource_ref(
    ref: Any,
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Optional[Dict[str, str]]:
    """Normalize Grafana datasource references into stable keys for __inputs generation."""
    if ref is None or is_builtin_datasource_ref(ref):
        return None

    if isinstance(ref, str):
        if is_placeholder_string(ref):
            return None
        return resolve_string_datasource_ref(
            ref,
            datasources_by_uid,
            datasources_by_name,
        )

    if isinstance(ref, dict):
        return resolve_object_datasource_ref(
            ref,
            datasources_by_uid,
            datasources_by_name,
        )

    return None


def replace_datasource_refs_in_dashboard(
    node: Any,
    ref_mapping: Dict[str, Dict[str, str]],
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> None:
    """Replace resolved datasource references with the generated __inputs placeholders."""
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "datasource":
                resolved = resolve_datasource_ref(
                    value,
                    datasources_by_uid=datasources_by_uid,
                    datasources_by_name=datasources_by_name,
                )
                if resolved is not None:
                    input_name = ref_mapping[resolved["key"]]["input_name"]
                    placeholder = f"${{{input_name}}}"
                    if isinstance(value, dict):
                        # Keep Grafana's object form when the source dashboard stored
                        # datasource metadata as {"type": ..., "uid": ...}.
                        replacement = {"uid": placeholder}
                        ds_type = resolved.get("type")
                        if isinstance(ds_type, str) and ds_type:
                            replacement["type"] = ds_type
                        node[key] = replacement
                    else:
                        node[key] = placeholder
            else:
                replace_datasource_refs_in_dashboard(
                    value,
                    ref_mapping=ref_mapping,
                    datasources_by_uid=datasources_by_uid,
                    datasources_by_name=datasources_by_name,
                )
        return
    if isinstance(node, list):
        for item in node:
            replace_datasource_refs_in_dashboard(
                item,
                ref_mapping=ref_mapping,
                datasources_by_uid=datasources_by_uid,
                datasources_by_name=datasources_by_name,
            )


def ensure_datasource_template_variable(
    dashboard: Dict[str, Any],
    datasource_type: str,
) -> None:
    """Create Grafana's conventional $datasource variable if one does not already exist."""
    templating = dashboard.setdefault("templating", {})
    if not isinstance(templating, dict):
        return
    variables = templating.setdefault("list", [])
    if not isinstance(variables, list):
        return

    for variable in variables:
        if not isinstance(variable, dict):
            continue
        if variable.get("type") == "datasource":
            return

    variables.insert(
        0,
        {
            "current": {},
            "label": "Data source",
            "name": "datasource",
            "options": [],
            "query": datasource_type,
            "refresh": 1,
            "regex": "",
            "type": "datasource",
        },
    )


def rewrite_panel_datasources_to_template_variable(
    panels: List[Dict[str, Any]],
    placeholder_names: Set[str],
) -> None:
    """Collapse panel datasource placeholders down to the shared $datasource variable."""
    for panel in panels:
        datasource = panel.get("datasource")
        if isinstance(datasource, str):
            if datasource in placeholder_names or datasource in {"$datasource", "${datasource}"}:
                panel["datasource"] = {"uid": "$datasource"}
        elif isinstance(datasource, dict):
            uid = datasource.get("uid")
            if isinstance(uid, str) and (
                uid in placeholder_names or uid in {"$datasource", "${datasource}"}
            ):
                panel["datasource"] = {"uid": "$datasource"}

        nested = panel.get("panels")
        if isinstance(nested, list):
            rewrite_panel_datasources_to_template_variable(
                [item for item in nested if isinstance(item, dict)],
                placeholder_names,
            )


def allocate_input_mapping(
    resolved: Dict[str, str],
    ref_mapping: Dict[str, Dict[str, str]],
    type_counts: Dict[str, int],
    key: Optional[str] = None,
) -> Dict[str, str]:
    """Create or reuse one __inputs mapping entry for a resolved datasource ref."""
    mapping_key = key or resolved["key"]
    mapping = ref_mapping.get(mapping_key)
    if mapping is not None:
        return mapping

    ds_type = resolved["type"]
    index = type_counts.get(ds_type, 0) + 1
    type_counts[ds_type] = index
    mapping = {
        "input_name": f"{make_type_input_base(ds_type)}_{index}",
        "label": make_input_label(ds_type, index),
        "type": ds_type,
    }
    ref_mapping[mapping_key] = mapping
    return mapping


def rewrite_template_variable_query(
    variable: Dict[str, Any],
    mapping: Dict[str, str],
    datasource_var_types: Dict[str, str],
    datasource_var_placeholders: Set[str],
) -> None:
    """Rewrite one datasource template variable into importer-friendly prompt form."""
    var_name = variable.get("name")
    if isinstance(var_name, str) and var_name:
        datasource_var_types[var_name] = mapping["type"]
        datasource_var_placeholders.add(f"${var_name}")
        datasource_var_placeholders.add(f"${{{var_name}}}")

    variable["current"] = {}
    variable["options"] = []
    variable["query"] = mapping["type"]
    variable["refresh"] = 1
    variable["regex"] = variable.get("regex", "")
    if variable.get("hide") == 0:
        variable.pop("hide", None)


def rewrite_template_variable_datasource(
    variable: Dict[str, Any],
    datasource_var_types: Dict[str, str],
    datasource_var_placeholders: Set[str],
) -> None:
    """Rewrite datasource selectors that point at datasource template variables."""
    datasource = variable.get("datasource")
    placeholder_value = None
    if isinstance(datasource, str):
        placeholder_value = datasource
    elif isinstance(datasource, dict):
        uid = datasource.get("uid")
        if isinstance(uid, str):
            placeholder_value = uid

    if not isinstance(placeholder_value, str):
        return
    datasource_type = datasource_var_types.get(
        extract_placeholder_name(placeholder_value)
    )
    if placeholder_value not in datasource_var_placeholders or not datasource_type:
        return

    variable["datasource"] = {
        "type": datasource_type,
        "uid": f"${{{make_type_input_base(datasource_type)}_1}}",
    }
    variable["current"] = {}
    variable["options"] = []


def prepare_templating_for_external_import(
    dashboard: Dict[str, Any],
    ref_mapping: Dict[str, Dict[str, str]],
    type_counts: Dict[str, int],
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Set[str]:
    """Rewrite datasource template variables so exported dashboards prompt on import."""
    templating = dashboard.get("templating")
    if not isinstance(templating, dict):
        return set()
    variables = templating.get("list")
    if not isinstance(variables, list):
        return set()

    datasource_var_types: Dict[str, str] = {}
    datasource_var_placeholders: Set[str] = set()

    for variable in variables:
        if not isinstance(variable, dict):
            continue
        if variable.get("type") != "datasource":
            continue

        query = variable.get("query")
        ds_ref = query if isinstance(query, str) else None
        if not ds_ref:
            continue

        resolved = resolve_datasource_ref(
            ds_ref,
            datasources_by_uid=datasources_by_uid,
            datasources_by_name=datasources_by_name,
        )
        if resolved is None:
            continue

        mapping = allocate_input_mapping(
            resolved,
            ref_mapping,
            type_counts,
            key=f"templating:{variable.get('name') or resolved['key']}",
        )
        rewrite_template_variable_query(
            variable,
            mapping,
            datasource_var_types,
            datasource_var_placeholders,
        )

    for variable in variables:
        if not isinstance(variable, dict):
            continue
        rewrite_template_variable_datasource(
            variable,
            datasource_var_types,
            datasource_var_placeholders,
        )

    return set(datasource_var_types)


def collect_panel_types(panels: List[Dict[str, Any]], panel_types: Set[str]) -> None:
    """Gather panel plugin ids so __requires mirrors what Grafana exports."""
    for panel in panels:
        panel_type = panel.get("type")
        if isinstance(panel_type, str) and panel_type:
            panel_types.add(panel_type)
        nested = panel.get("panels")
        if isinstance(nested, list):
            collect_panel_types(
                [item for item in nested if isinstance(item, dict)],
                panel_types,
            )


def build_input_definitions(
    ref_mapping: Dict[str, Dict[str, str]],
) -> List[Dict[str, str]]:
    """Build Grafana's __inputs block from the resolved datasource mapping table."""
    return [
        {
            "name": mapping["input_name"],
            "label": mapping["label"],
            "description": "",
            "type": "datasource",
            "pluginId": mapping["type"],
            "pluginName": mapping["type"],
        }
        for _, mapping in sorted(ref_mapping.items(), key=lambda item: item[1]["input_name"])
    ]


def build_requires_block(
    ref_mapping: Dict[str, Dict[str, str]],
    panel_types: Set[str],
) -> List[Dict[str, str]]:
    """Build Grafana's __requires block for Grafana itself, datasources, and panels."""
    requires = [{"type": "grafana", "id": "grafana", "name": "Grafana", "version": ""}]
    requires.extend(
        {
            "type": "datasource",
            "id": mapping["type"],
            "name": mapping["type"],
            "version": "",
        }
        for _, mapping in sorted(ref_mapping.items(), key=lambda item: item[1]["input_name"])
    )
    requires.extend(
        {
            "type": "panel",
            "id": panel_type,
            "name": panel_type,
            "version": "",
        }
        for panel_type in sorted(panel_types)
    )
    return requires


def build_dashboard_index_item(summary: Dict[str, Any], uid: str) -> Dict[str, str]:
    """Build the shared root index metadata for one exported dashboard."""
    return {
        "uid": uid,
        "title": str(summary.get("title") or ""),
        "folder": str(summary.get("folderTitle") or ""),
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


def build_external_export_document(
    payload: Dict[str, Any],
    datasource_catalog: Tuple[Dict[str, Dict[str, Any]], Dict[str, Dict[str, Any]]],
) -> Dict[str, Any]:
    """Convert a fetched dashboard into Grafana's web-import prompt format."""
    dashboard = build_preserved_web_import_document(payload)

    datasources_by_uid, datasources_by_name = datasource_catalog
    refs: List[Any] = []
    # First collect every datasource reference, then build a stable mapping so
    # repeated references become repeated placeholders instead of duplicate inputs.
    collect_datasource_refs(dashboard, refs)

    ref_mapping: Dict[str, Dict[str, str]] = {}
    type_counts: Dict[str, int] = {}
    prepare_templating_for_external_import(
        dashboard,
        ref_mapping=ref_mapping,
        type_counts=type_counts,
        datasources_by_uid=datasources_by_uid,
        datasources_by_name=datasources_by_name,
    )
    for ref in refs:
        resolved = resolve_datasource_ref(
            ref,
            datasources_by_uid=datasources_by_uid,
            datasources_by_name=datasources_by_name,
        )
        if resolved is None or resolved["key"] in ref_mapping:
            continue
        allocate_input_mapping(resolved, ref_mapping, type_counts)

    replace_datasource_refs_in_dashboard(
        dashboard,
        ref_mapping=ref_mapping,
        datasources_by_uid=datasources_by_uid,
        datasources_by_name=datasources_by_name,
    )

    datasource_types = sorted({mapping["type"] for mapping in ref_mapping.values()})
    if len(datasource_types) == 1:
        # When every datasource resolves to the same plugin type, Grafana's native
        # $datasource variable keeps the imported dashboard easier for humans to edit.
        ensure_datasource_template_variable(dashboard, datasource_types[0])
        placeholder_names = {
            f"${{{mapping['input_name']}}}" for mapping in ref_mapping.values()
        }
        panels = dashboard.get("panels")
        if isinstance(panels, list):
            rewrite_panel_datasources_to_template_variable(
                [item for item in panels if isinstance(item, dict)],
                placeholder_names,
            )

    dashboard["__inputs"] = build_input_definitions(ref_mapping)

    panel_types: Set[str] = set()
    panels = dashboard.get("panels")
    if isinstance(panels, list):
        collect_panel_types(
            [item for item in panels if isinstance(item, dict)],
            panel_types,
        )
    dashboard["__requires"] = build_requires_block(ref_mapping, panel_types)
    dashboard["__elements"] = {}
    return dashboard


def export_dashboards(args: argparse.Namespace) -> int:
    """Export dashboards into raw JSON, prompt JSON, or both variants."""
    if args.without_dashboard_raw and args.without_dashboard_prompt:
        raise GrafanaError(
            "Nothing to export. Remove one of --without-dashboard-raw or --without-dashboard-prompt."
        )

    client = build_client(args)
    output_dir = Path(args.export_dir)
    raw_dir, prompt_dir = build_export_variant_dirs(output_dir)
    export_raw = not args.without_dashboard_raw
    export_prompt = not args.without_dashboard_prompt
    datasource_catalog = None
    if export_prompt:
        # Prompt exports need datasource metadata up front so dashboard references
        # can be rewritten into Grafana's __inputs import format.
        datasource_catalog = build_datasource_catalog(client.list_datasources())

    summaries = client.iter_dashboard_summaries(args.page_size)
    if not summaries:
        print("No dashboards found.", file=sys.stderr)
        return 0

    index_items: List[Dict[str, str]] = []
    for summary in summaries:
        uid = str(summary["uid"])
        payload = client.fetch_dashboard(uid)
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
                print(f"Would export raw    {uid} -> {raw_path}")
            else:
                write_dashboard(raw_document, raw_path, args.overwrite)
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
                print(f"Would export prompt {uid} -> {prompt_path}")
            else:
                write_dashboard(prompt_document, prompt_path, args.overwrite)
                print(f"Exported prompt {uid} -> {prompt_path}")
            item["prompt_path"] = str(prompt_path)
        index_items.append(item)

    raw_index_path = None
    raw_metadata_path = None
    if export_raw:
        raw_index_path = raw_dir / "index.json"
        raw_metadata_path = raw_dir / EXPORT_METADATA_FILENAME
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
        prompt_index_path = prompt_dir / "index.json"
        prompt_metadata_path = prompt_dir / EXPORT_METADATA_FILENAME
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
    return (
        f"uid={record['uid']} name={record['name']} folder={record['folder']} "
        f"folderUid={record['folderUid']} path={record['path']}"
    )


def build_dashboard_summary_record(summary: Dict[str, Any]) -> Dict[str, str]:
    """Normalize a dashboard summary into a stable output record."""
    folder = str(summary.get("folderTitle") or "General")
    return {
        "uid": str(summary.get("uid") or "unknown"),
        "name": str(summary.get("title") or "dashboard"),
        "folder": folder,
        "folderUid": str(summary.get("folderUid") or "general"),
        "path": str(summary.get("folderPath") or folder),
    }


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


def render_dashboard_summary_table(summaries: List[Dict[str, Any]]) -> List[str]:
    """Render dashboard summaries as a fixed-width table."""
    headers = ["UID", "NAME", "FOLDER", "FOLDER_UID", "FOLDER_PATH"]
    rows = [
        [
            record["uid"],
            record["name"],
            record["folder"],
            record["folderUid"],
            record["path"],
        ]
        for record in [build_dashboard_summary_record(summary) for summary in summaries]
    ]
    widths = [len(header) for header in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def format_row(values: List[str]) -> str:
        return "  ".join(
            value.ljust(widths[index]) for index, value in enumerate(values)
        )

    lines = [format_row(headers), format_row(["-" * width for width in widths])]
    lines.extend(format_row(row) for row in rows)
    return lines


def render_dashboard_summary_csv(summaries: List[Dict[str, Any]]) -> None:
    """Render dashboard summaries as CSV records."""
    fieldnames = ["uid", "name", "folder", "folderUid", "path"]
    writer = csv.DictWriter(sys.stdout, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    for summary in summaries:
        writer.writerow(build_dashboard_summary_record(summary))


def render_dashboard_summary_json(summaries: List[Dict[str, Any]]) -> str:
    """Render dashboard summaries as JSON."""
    records = [build_dashboard_summary_record(summary) for summary in summaries]
    return json.dumps(records, indent=2, sort_keys=False)


def list_dashboards(args: argparse.Namespace) -> int:
    """List live dashboard summaries without exporting dashboard JSON."""
    client = build_client(args)
    summaries = attach_dashboard_folder_paths(
        client,
        client.iter_dashboard_summaries(args.page_size),
    )
    if args.csv:
        render_dashboard_summary_csv(summaries)
        return 0
    if args.json:
        print(render_dashboard_summary_json(summaries))
        return 0
    if args.table:
        for line in render_dashboard_summary_table(summaries):
            print(line)
    else:
        for summary in summaries:
            print(format_dashboard_summary_line(summary))
    print("")
    print(f"Listed {len(summaries)} dashboard summaries from {args.url}")
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
            print(f"Dry-run {dashboard_file} -> uid={uid} action={action}")
            continue

        result = client.import_dashboard(payload)
        status = result.get("status", "unknown")
        uid = result.get("uid") or uid
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


def main() -> int:
    args = parse_args()
    try:
        if args.command == "list":
            return list_dashboards(args)
        if args.command == "import":
            return import_dashboards(args)
        if args.command == "diff":
            return diff_dashboards(args)
        return export_dashboards(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
