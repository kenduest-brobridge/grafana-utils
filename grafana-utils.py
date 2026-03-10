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
import json
import re
import ssl
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple
from urllib import error, parse, request


DEFAULT_URL = "http://127.0.0.1:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_PAGE_SIZE = 500
DEFAULT_OUTPUT_DIR = "dashboards"
RAW_EXPORT_SUBDIR = "raw"
PROMPT_EXPORT_SUBDIR = "prompt"
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


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export or import Grafana dashboards."
    )
    parser.add_argument(
        "--url",
        default=DEFAULT_URL,
        help=f"Grafana base URL (default: {DEFAULT_URL})",
    )
    parser.add_argument(
        "--api-token",
        default=None,
        help="Grafana API token. Falls back to GRAFANA_API_TOKEN.",
    )
    parser.add_argument(
        "--username",
        default=None,
        help="Grafana username. Falls back to GRAFANA_USERNAME.",
    )
    parser.add_argument(
        "--password",
        default=None,
        help="Grafana password. Falls back to GRAFANA_PASSWORD.",
    )
    parser.add_argument(
        "--output-dir",
        default=DEFAULT_OUTPUT_DIR,
        help=(
            "Directory to write exported dashboards into. Export writes two "
            f"subdirectories by default: {RAW_EXPORT_SUBDIR}/ and {PROMPT_EXPORT_SUBDIR}/."
        ),
    )
    parser.add_argument(
        "--import-dir",
        default=None,
        help=(
            "Import dashboards from this directory instead of exporting. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly."
        ),
    )
    parser.add_argument(
        "--page-size",
        type=int,
        default=DEFAULT_PAGE_SIZE,
        help=f"Dashboard search page size (default: {DEFAULT_PAGE_SIZE}).",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=DEFAULT_TIMEOUT,
        help=f"HTTP timeout in seconds (default: {DEFAULT_TIMEOUT}).",
    )
    parser.add_argument(
        "--flat",
        action="store_true",
        help="Write all dashboards into the output root instead of per-folder directories.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing dashboard files if they already exist.",
    )
    parser.add_argument(
        "--without-raw",
        action="store_true",
        help=f"Skip exporting the {RAW_EXPORT_SUBDIR}/ variant.",
    )
    parser.add_argument(
        "--without-prompt",
        action="store_true",
        help=f"Skip exporting the {PROMPT_EXPORT_SUBDIR}/ variant.",
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
        default="Imported by grafana-utils.py",
        help="Version history message to attach to imported dashboards.",
    )
    parser.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )
    return parser.parse_args(argv)


def resolve_auth(args: argparse.Namespace) -> Dict[str, str]:
    token = args.api_token or env_value("GRAFANA_API_TOKEN")
    if token:
        return {"Authorization": f"Bearer {token}"}

    username = args.username or env_value("GRAFANA_USERNAME")
    password = args.password or env_value("GRAFANA_PASSWORD")
    if username and password:
        encoded = base64.b64encode(f"{username}:{password}".encode("utf-8")).decode(
            "ascii"
        )
        return {"Authorization": f"Basic {encoded}"}

    raise GrafanaError(
        "Authentication required. Set --api-token / GRAFANA_API_TOKEN or "
        "--username and --password / GRAFANA_USERNAME and GRAFANA_PASSWORD."
    )


def env_value(name: str) -> Optional[str]:
    import os

    value = os.environ.get(name)
    return value if value else None


def build_ssl_context(verify_ssl: bool) -> ssl.SSLContext:
    if verify_ssl:
        return ssl.create_default_context()
    return ssl._create_unverified_context()


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
    return output_dir / RAW_EXPORT_SUBDIR, output_dir / PROMPT_EXPORT_SUBDIR


class GrafanaClient:
    def __init__(
        self,
        base_url: str,
        headers: Dict[str, str],
        timeout: int,
        verify_ssl: bool,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.headers = {"Accept": "application/json", **headers}
        self.timeout = timeout
        self.ssl_context = build_ssl_context(verify_ssl)

    def request_json(
        self,
        path: str,
        params: Optional[Dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[Dict[str, Any]] = None,
    ) -> Any:
        query = ""
        if params:
            query = "?" + parse.urlencode(params)
        url = f"{self.base_url}{path}{query}"
        headers = dict(self.headers)
        data = None
        if payload is not None:
            data = json.dumps(payload).encode("utf-8")
            headers["Content-Type"] = "application/json"
        req = request.Request(url, headers=headers, data=data, method=method)
        try:
            with request.urlopen(
                req,
                timeout=self.timeout,
                context=self.ssl_context,
            ) as response:
                data = response.read().decode("utf-8")
        except error.HTTPError as exc:
            body = exc.read().decode("utf-8", errors="replace")
            raise GrafanaError(
                f"Grafana API error {exc.code} for {url}: {body}"
            ) from exc
        except error.URLError as exc:
            raise GrafanaError(f"Request failed for {url}: {exc.reason}") from exc

        try:
            return json.loads(data)
        except json.JSONDecodeError as exc:
            raise GrafanaError(f"Invalid JSON response from {url}") from exc

    def iter_dashboard_summaries(self, page_size: int) -> List[Dict[str, Any]]:
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

    def fetch_dashboard(self, uid: str) -> Dict[str, Any]:
        data = self.request_json(f"/api/dashboards/uid/{parse.quote(uid, safe='')}")
        if not isinstance(data, dict) or "dashboard" not in data:
            raise GrafanaError(f"Unexpected dashboard payload for UID {uid}.")
        return data

    def import_dashboard(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/dashboards/db",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected dashboard import response from Grafana.")
        return data

    def list_datasources(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/datasources")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected datasource list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]


def write_dashboard(
    payload: Dict[str, Any],
    output_path: Path,
    overwrite: bool,
) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    if output_path.exists() and not overwrite:
        raise GrafanaError(
            f"Refusing to overwrite existing file: {output_path}. Use --overwrite."
        )
    output_path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def discover_dashboard_files(import_dir: Path) -> List[Path]:
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
        if path.name != "index.json"
    ]
    if not files:
        raise GrafanaError(f"No dashboard JSON files found in {import_dir}")
    return files


def load_json_file(path: Path) -> Dict[str, Any]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError(f"Failed to read {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError(f"Invalid JSON in {path}: {exc}") from exc

    if not isinstance(raw, dict):
        raise GrafanaError(f"Dashboard file must contain a JSON object: {path}")
    return raw


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

    dashboard_source = document.get("dashboard", document)
    if not isinstance(dashboard_source, dict):
        raise GrafanaError("Dashboard payload must be a JSON object.")

    dashboard = copy.deepcopy(dashboard_source)
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
    dashboard_source = payload.get("dashboard")
    if not isinstance(dashboard_source, dict):
        raise GrafanaError("Unexpected dashboard payload from Grafana.")

    dashboard = copy.deepcopy(dashboard_source)
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
        datasource = datasources_by_name.get(ref)
        if datasource is None:
            datasource = datasources_by_uid.get(ref)
        if datasource is None:
            ref_lower = ref.lower()
            datasource_type = DATASOURCE_TYPE_ALIASES.get(ref_lower)
            if datasource_type is None:
                for candidate in datasources_by_uid.values():
                    candidate_type = candidate.get("type")
                    if isinstance(candidate_type, str) and candidate_type.lower() == ref_lower:
                        datasource_type = candidate_type
                        break
            if datasource_type is not None:
                return {
                    "key": f"type:{datasource_type}",
                    "label": datasource_type,
                    "type": datasource_type,
                }
            raise GrafanaError(
                f"Cannot resolve datasource name or uid {ref!r} for prompt export."
            )
        uid = datasource.get("uid") or ref
        label = datasource.get("name") or ref
        ds_type = datasource.get("type")
        if not isinstance(ds_type, str) or not ds_type:
            raise GrafanaError(f"Datasource {ref!r} does not have a usable type.")
        return {"key": f"uid:{uid}", "label": label, "type": ds_type}

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

        if isinstance(ds_type, str) and ds_type:
            placeholder_value = None
            if isinstance(uid, str) and is_placeholder_string(uid):
                placeholder_value = uid
            elif isinstance(name, str) and is_placeholder_string(name):
                placeholder_value = name
            if placeholder_value is not None:
                token = extract_placeholder_name(placeholder_value)
                return {
                    "key": f"var:{ds_type}:{token}",
                    "label": token,
                    "type": ds_type,
                }
        if has_placeholder:
            return None

        datasource = None  # type: Optional[Dict[str, Any]]
        if isinstance(uid, str) and uid:
            datasource = datasources_by_uid.get(uid)
        if datasource is None and isinstance(name, str) and name:
            datasource = datasources_by_name.get(name)

        resolved_type = None
        resolved_label = None
        resolved_uid = None
        if datasource is not None:
            resolved_type = datasource.get("type")
            resolved_label = datasource.get("name")
            resolved_uid = datasource.get("uid")

        resolved_type = resolved_type or ds_type
        resolved_label = resolved_label or name or uid
        resolved_uid = resolved_uid or uid or name

        if not isinstance(resolved_type, str) or not resolved_type:
            raise GrafanaError(
                f"Cannot resolve datasource type from reference {ref!r}."
            )
        if not isinstance(resolved_label, str) or not resolved_label:
            resolved_label = resolved_type
        if not isinstance(resolved_uid, str) or not resolved_uid:
            resolved_uid = resolved_label

        return {
            "key": f"uid:{resolved_uid}",
            "label": resolved_label,
            "type": resolved_type,
        }

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

        key = f"templating:{variable.get('name') or resolved['key']}"
        mapping = ref_mapping.get(key)
        if mapping is None:
            ds_type = resolved["type"]
            index = type_counts.get(ds_type, 0) + 1
            type_counts[ds_type] = index
            mapping = {
                "input_name": f"{make_type_input_base(ds_type)}_{index}",
                "label": make_input_label(ds_type, index),
                "type": ds_type,
            }
            ref_mapping[key] = mapping

        var_name = variable.get("name")
        if isinstance(var_name, str) and var_name:
            # Track template variable names so downstream datasource selectors can
            # be rewritten to point at the generated __inputs placeholders.
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

    for variable in variables:
        if not isinstance(variable, dict):
            continue
        datasource = variable.get("datasource")
        datasource_type = None
        if isinstance(datasource, str):
            datasource_type = datasource_var_types.get(extract_placeholder_name(datasource))
            if datasource in datasource_var_placeholders and datasource_type:
                variable["datasource"] = {
                    "type": datasource_type,
                    "uid": f"${{{make_type_input_base(datasource_type)}_1}}",
                }
                variable["current"] = {}
                variable["options"] = []
        elif isinstance(datasource, dict):
            uid = datasource.get("uid")
            if isinstance(uid, str):
                datasource_type = datasource_var_types.get(extract_placeholder_name(uid))
                if uid in datasource_var_placeholders and datasource_type:
                    variable["datasource"] = {
                        "type": datasource_type,
                        "uid": f"${{{make_type_input_base(datasource_type)}_1}}",
                    }
                    variable["current"] = {}
                    variable["options"] = []

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


def build_external_export_document(
    payload: Dict[str, Any],
    datasource_catalog: Tuple[Dict[str, Dict[str, Any]], Dict[str, Dict[str, Any]]],
) -> Dict[str, Any]:
    """Convert a fetched dashboard into Grafana's portable export/import format."""
    dashboard = build_preserved_web_import_document(payload)

    datasources_by_uid, datasources_by_name = datasource_catalog
    refs: List[Any] = []
    # First collect every datasource reference, then build a stable mapping so
    # repeated references become repeated placeholders instead of duplicate inputs.
    collect_datasource_refs(dashboard, refs)

    ref_mapping: Dict[str, Dict[str, str]] = {}
    type_counts: Dict[str, int] = {}
    datasource_var_names = prepare_templating_for_external_import(
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

        ds_type = resolved["type"]
        index = type_counts.get(ds_type, 0) + 1
        type_counts[ds_type] = index
        input_name = f"{make_type_input_base(ds_type)}_{index}"

        ref_mapping[resolved["key"]] = {
            "input_name": input_name,
            "label": make_input_label(ds_type, index),
            "type": ds_type,
        }

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

    dashboard["__inputs"] = [
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

    panel_types: Set[str] = set()
    panels = dashboard.get("panels")
    if isinstance(panels, list):
        collect_panel_types(
            [item for item in panels if isinstance(item, dict)],
            panel_types,
        )
    dashboard["__requires"] = [
        {"type": "grafana", "id": "grafana", "name": "Grafana", "version": ""}
    ]
    # Grafana expects datasource and panel plugins to be listed in __requires
    # alongside the generated __inputs block.
    dashboard["__requires"].extend(
        {
            "type": "datasource",
            "id": mapping["type"],
            "name": mapping["type"],
            "version": "",
        }
        for _, mapping in sorted(ref_mapping.items(), key=lambda item: item[1]["input_name"])
    )
    dashboard["__requires"].extend(
        {
            "type": "panel",
            "id": panel_type,
            "name": panel_type,
            "version": "",
        }
        for panel_type in sorted(panel_types)
    )
    dashboard["__elements"] = {}
    return dashboard


def export_dashboards(args: argparse.Namespace) -> int:
    """Export raw API-safe dashboards and optional prompt-based web-import variants."""
    if args.without_raw and args.without_prompt:
        raise GrafanaError("Nothing to export. Remove one of --without-raw or --without-prompt.")

    client = build_client(args)
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    raw_dir, prompt_dir = build_export_variant_dirs(output_dir)
    export_raw = not args.without_raw
    export_prompt = not args.without_prompt
    if export_raw:
        raw_dir.mkdir(parents=True, exist_ok=True)
    if export_prompt:
        prompt_dir.mkdir(parents=True, exist_ok=True)
    datasource_catalog = None
    if export_prompt:
        # Prompt exports need datasource metadata up front so dashboard references
        # can be rewritten into Grafana's __inputs import format.
        datasource_catalog = build_datasource_catalog(client.list_datasources())

    summaries = client.iter_dashboard_summaries(args.page_size)
    if not summaries:
        print("No dashboards found.", file=sys.stderr)
        return 0

    index: List[Dict[str, str]] = []
    for summary in summaries:
        uid = str(summary["uid"])
        payload = client.fetch_dashboard(uid)
        item = {
            "uid": uid,
            "title": str(summary.get("title") or ""),
            "folder": str(summary.get("folderTitle") or ""),
        }
        if export_raw:
            raw_document = build_preserved_web_import_document(payload)
            raw_path = build_output_path(raw_dir, summary, args.flat)
            write_dashboard(raw_document, raw_path, args.overwrite)
            item["raw_path"] = str(raw_path)
            print(f"Exported raw    {uid} -> {raw_path}")
        if export_prompt:
            assert datasource_catalog is not None
            prompt_document = build_external_export_document(payload, datasource_catalog)
            prompt_path = build_output_path(prompt_dir, summary, args.flat)
            write_dashboard(prompt_document, prompt_path, args.overwrite)
            item["prompt_path"] = str(prompt_path)
            print(f"Exported prompt {uid} -> {prompt_path}")
        index.append(item)

    raw_index_path = None
    if export_raw:
        raw_index_path = raw_dir / "index.json"
        raw_index_path.write_text(
            json.dumps(
                [
                    {
                        "uid": item["uid"],
                        "title": item["title"],
                        "folder": item["folder"],
                        "path": item["raw_path"],
                        "format": "grafana-web-import-preserve-uid",
                    }
                    for item in index
                    if "raw_path" in item
                ],
                indent=2,
                ensure_ascii=False,
            )
            + "\n",
            encoding="utf-8",
        )
    prompt_index_path = None
    if export_prompt:
        prompt_index_path = prompt_dir / "index.json"
        prompt_index_path.write_text(
            json.dumps(
                [
                    {
                        "uid": item["uid"],
                        "title": item["title"],
                        "folder": item["folder"],
                        "path": item["prompt_path"],
                        "format": "grafana-web-import-with-datasource-inputs",
                    }
                    for item in index
                    if "prompt_path" in item
                ],
                indent=2,
                ensure_ascii=False,
            )
            + "\n",
            encoding="utf-8",
        )
    index_path = output_dir / "index.json"
    index_path.write_text(
        json.dumps(index, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    summary_parts = [f"Exported {len(index)} dashboards."]
    if raw_index_path is not None:
        summary_parts.append(f"Raw index: {raw_index_path}")
    if prompt_index_path is not None:
        summary_parts.append(f"Prompt index: {prompt_index_path}")
    summary_parts.append(f"Root index: {index_path}")
    print(" ".join(summary_parts))
    return 0


def import_dashboards(args: argparse.Namespace) -> int:
    """Import previously exported raw dashboard JSON files through Grafana's API."""
    client = build_client(args)
    import_dir = Path(args.import_dir)
    dashboard_files = discover_dashboard_files(import_dir)

    for dashboard_file in dashboard_files:
        document = load_json_file(dashboard_file)
        payload = build_import_payload(
            document=document,
            folder_uid_override=args.import_folder_uid,
            replace_existing=args.replace_existing,
            message=args.import_message,
        )
        result = client.import_dashboard(payload)
        status = result.get("status", "unknown")
        uid = result.get("uid") or payload["dashboard"].get("uid") or "unknown"
        print(f"Imported {dashboard_file} -> uid={uid} status={status}")

    print(f"Imported {len(dashboard_files)} dashboard files from {import_dir}")
    return 0


def build_client(args: argparse.Namespace) -> GrafanaClient:
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
        if args.import_dir:
            return import_dashboards(args)
        return export_dashboards(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
