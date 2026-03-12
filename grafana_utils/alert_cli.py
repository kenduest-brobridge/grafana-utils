#!/usr/bin/env python3
"""Export or import Grafana alerting resources."""

import argparse
import base64
import copy
import difflib
import getpass
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from urllib import parse

from .http_transport import (
    JsonHttpTransport,
    HttpTransportApiError,
    HttpTransportError,
    build_json_http_transport,
)


DEFAULT_URL = "http://127.0.0.1:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_OUTPUT_DIR = "alerts"
RAW_EXPORT_SUBDIR = "raw"
RULES_SUBDIR = "rules"
CONTACT_POINTS_SUBDIR = "contact-points"
MUTE_TIMINGS_SUBDIR = "mute-timings"
POLICIES_SUBDIR = "policies"
TEMPLATES_SUBDIR = "templates"
LINKED_DASHBOARD_ANNOTATION_KEY = "__dashboardUid__"
LINKED_PANEL_ANNOTATION_KEY = "__panelId__"

RULE_KIND = "grafana-alert-rule"
CONTACT_POINT_KIND = "grafana-contact-point"
MUTE_TIMING_KIND = "grafana-mute-timing"
POLICIES_KIND = "grafana-notification-policies"
TEMPLATE_KIND = "grafana-notification-template"
TOOL_API_VERSION = 1
TOOL_SCHEMA_VERSION = 1
ROOT_INDEX_KIND = "grafana-utils-alert-export-index"
HELP_EPILOG = """Examples:

  Export alerting resources with an API token:
    export GRAFANA_API_TOKEN='your-token'
    grafana-alert-utils --url https://grafana.example.com --output-dir ./alerts --overwrite

  Import back into Grafana and update existing resources:
    grafana-alert-utils --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing

  Import linked alert rules with dashboard and panel remapping:
    grafana-alert-utils --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json
"""

RESOURCE_SUBDIR_BY_KIND = {
    RULE_KIND: RULES_SUBDIR,
    CONTACT_POINT_KIND: CONTACT_POINTS_SUBDIR,
    MUTE_TIMING_KIND: MUTE_TIMINGS_SUBDIR,
    POLICIES_KIND: POLICIES_SUBDIR,
    TEMPLATE_KIND: TEMPLATES_SUBDIR,
}
SERVER_MANAGED_FIELDS_BY_KIND = {
    RULE_KIND: {"id", "updated", "provenance"},
    CONTACT_POINT_KIND: {"provenance"},
    MUTE_TIMING_KIND: {"version", "provenance"},
    POLICIES_KIND: {"provenance"},
    TEMPLATE_KIND: {"provenance"},
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


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Export or import Grafana alerting resources.",
        epilog=HELP_EPILOG,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
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
        "--output-dir",
        default=DEFAULT_OUTPUT_DIR,
        help=(
            "Directory to write exported alerting resources into. Export writes files "
            f"under {RAW_EXPORT_SUBDIR}/."
        ),
    )
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
        "--timeout",
        type=int,
        default=DEFAULT_TIMEOUT,
        help=f"HTTP timeout in seconds (default: {DEFAULT_TIMEOUT}).",
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
    parser.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )
    return parser


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    return build_parser().parse_args(argv)


def env_value(name: str) -> Optional[str]:
    import os

    value = os.environ.get(name)
    return value if value else None


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


def sanitize_path_component(value: str) -> str:
    normalized = re.sub(r"[^\w.\- ]+", "_", value.strip(), flags=re.UNICODE)
    normalized = re.sub(r"\s+", "_", normalized)
    normalized = re.sub(r"_+", "_", normalized)
    normalized = normalized.strip("._")
    return normalized or "untitled"


def derive_dashboard_slug(value: Any) -> str:
    text = str(value or "").strip()
    if not text:
        return ""
    match = re.search(r"/d/[^/]+/([^/?#]+)", text)
    if match:
        return match.group(1)
    if text.startswith("/"):
        text = text.rstrip("/").split("/")[-1]
    return text.strip()


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


class GrafanaAlertClient:
    """Minimal HTTP wrapper around the Grafana alerting provisioning APIs."""

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
        """Send one request to Grafana and decode the JSON response body."""
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

    def list_alert_rules(self) -> List[Dict[str, Any]]:
        """List Grafana-managed alert rules."""
        data = self.request_json("/api/v1/provisioning/alert-rules")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected alert-rule list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def search_dashboards(self, query: str) -> List[Dict[str, Any]]:
        """Search dashboards when linked-rule import needs fallback matching."""
        data = self.request_json(
            "/api/search",
            params={"type": "dash-db", "query": query, "limit": 500},
        )
        if not isinstance(data, list):
            raise GrafanaError("Unexpected dashboard search response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def get_dashboard(self, uid: str) -> Dict[str, Any]:
        """Fetch one dashboard wrapper by UID."""
        data = self.request_json(f"/api/dashboards/uid/{parse.quote(uid, safe='')}")
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected dashboard payload for UID {uid}.")
        return data

    def get_alert_rule(self, uid: str) -> Dict[str, Any]:
        """Fetch one alert rule by UID."""
        data = self.request_json(
            f"/api/v1/provisioning/alert-rules/{parse.quote(uid, safe='')}"
        )
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected alert-rule payload for UID {uid}.")
        return data

    def create_alert_rule(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Create one alert rule."""
        data = self.request_json(
            "/api/v1/provisioning/alert-rules",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule create response from Grafana.")
        return data

    def update_alert_rule(self, uid: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Update one alert rule by UID."""
        data = self.request_json(
            f"/api/v1/provisioning/alert-rules/{parse.quote(uid, safe='')}",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule update response from Grafana.")
        return data

    def list_contact_points(self) -> List[Dict[str, Any]]:
        """List contact points."""
        data = self.request_json("/api/v1/provisioning/contact-points")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected contact-point list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def create_contact_point(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Create one contact point."""
        data = self.request_json(
            "/api/v1/provisioning/contact-points",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected contact-point create response from Grafana.")
        return data

    def update_contact_point(self, uid: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Update one contact point by UID."""
        data = self.request_json(
            f"/api/v1/provisioning/contact-points/{parse.quote(uid, safe='')}",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected contact-point update response from Grafana.")
        return data

    def list_mute_timings(self) -> List[Dict[str, Any]]:
        """List mute timings."""
        data = self.request_json("/api/v1/provisioning/mute-timings")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected mute-timing list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def create_mute_timing(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Create one mute timing."""
        data = self.request_json(
            "/api/v1/provisioning/mute-timings",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected mute-timing create response from Grafana.")
        return data

    def update_mute_timing(self, name: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Update one mute timing by its stable name."""
        data = self.request_json(
            f"/api/v1/provisioning/mute-timings/{parse.quote(name, safe='')}",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected mute-timing update response from Grafana.")
        return data

    def get_notification_policies(self) -> Dict[str, Any]:
        """Fetch Grafana's single notification policy tree."""
        data = self.request_json("/api/v1/provisioning/policies")
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected notification policy response from Grafana.")
        return data

    def update_notification_policies(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Replace Grafana's notification policy tree."""
        data = self.request_json(
            "/api/v1/provisioning/policies",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected notification policy update response from Grafana."
            )
        return data

    def list_templates(self) -> List[Dict[str, Any]]:
        """List notification templates, treating Grafana null as an empty list."""
        data = self.request_json("/api/v1/provisioning/templates")
        if data is None:
            return []
        if not isinstance(data, list):
            raise GrafanaError("Unexpected template list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def get_template(self, name: str) -> Dict[str, Any]:
        """Fetch one notification template by name."""
        data = self.request_json(
            f"/api/v1/provisioning/templates/{parse.quote(name, safe='')}"
        )
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected template payload for name {name}.")
        return data

    def update_template(self, name: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Update one template; the stable name lives in the URL, not the body."""
        body = dict(payload)
        body.pop("name", None)
        data = self.request_json(
            f"/api/v1/provisioning/templates/{parse.quote(name, safe='')}",
            method="PUT",
            payload=body,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected template update response from Grafana.")
        return data


def strip_server_managed_fields(kind: str, payload: Dict[str, Any]) -> Dict[str, Any]:
    """Remove Grafana-owned fields so exports are safe for later import."""
    normalized = copy.deepcopy(payload)
    for field in SERVER_MANAGED_FIELDS_BY_KIND.get(kind, set()):
        normalized.pop(field, None)
    return normalized


def get_rule_linkage(rule: Dict[str, Any]) -> Optional[Dict[str, str]]:
    """Extract linked dashboard and panel annotations from an alert rule."""
    annotations = rule.get("annotations")
    if not isinstance(annotations, dict):
        return None

    dashboard_uid = str(
        annotations.get(LINKED_DASHBOARD_ANNOTATION_KEY) or ""
    ).strip()
    if not dashboard_uid:
        return None

    panel_id = annotations.get(LINKED_PANEL_ANNOTATION_KEY)
    linkage = {"dashboardUid": dashboard_uid}
    if panel_id is not None:
        linkage["panelId"] = str(panel_id)
    return linkage


def find_panel_by_id(panels: Any, panel_id: str) -> Optional[Dict[str, Any]]:
    """Walk nested panel trees until the requested panel id is found."""
    if not isinstance(panels, list):
        return None
    for panel in panels:
        if not isinstance(panel, dict):
            continue
        current_panel_id = panel.get("id")
        if current_panel_id is not None and str(current_panel_id) == panel_id:
            return panel
        nested = find_panel_by_id(panel.get("panels"), panel_id)
        if nested is not None:
            return nested
    return None


def build_linked_dashboard_metadata(
    client: GrafanaAlertClient,
    rule: Dict[str, Any],
) -> Optional[Dict[str, str]]:
    """Capture extra dashboard context so linked rules can be repaired on import."""
    linkage = get_rule_linkage(rule)
    if not linkage:
        return None

    metadata = dict(linkage)
    dashboard_uid = linkage["dashboardUid"]
    try:
        dashboard_payload = client.get_dashboard(dashboard_uid)
    except GrafanaApiError as exc:
        if exc.status_code != 404:
            raise
        return metadata

    dashboard = dashboard_payload.get("dashboard")
    meta = dashboard_payload.get("meta")
    if isinstance(dashboard, dict):
        metadata["dashboardTitle"] = str(dashboard.get("title") or "")
        panel_id = metadata.get("panelId")
        if panel_id:
            panel = find_panel_by_id(dashboard.get("panels"), panel_id)
            if isinstance(panel, dict):
                metadata["panelTitle"] = str(panel.get("title") or "")
                metadata["panelType"] = str(panel.get("type") or "")
    if isinstance(meta, dict):
        metadata["folderTitle"] = str(meta.get("folderTitle") or "")
        metadata["folderUid"] = str(meta.get("folderUid") or "")
        metadata["dashboardSlug"] = derive_dashboard_slug(
            meta.get("url") or meta.get("slug") or ""
        )
    return metadata


def filter_dashboard_search_matches(
    candidates: List[Dict[str, Any]],
    linked_dashboard: Dict[str, Any],
) -> List[Dict[str, Any]]:
    """Narrow dashboard matches by title first, then folder title and slug."""
    dashboard_title = str(linked_dashboard.get("dashboardTitle") or "")
    filtered = [
        item for item in candidates if str(item.get("title") or "") == dashboard_title
    ]

    folder_title = str(linked_dashboard.get("folderTitle") or "")
    if folder_title:
        folder_matches = [
            item for item in filtered if str(item.get("folderTitle") or "") == folder_title
        ]
        if folder_matches:
            filtered = folder_matches

    slug = derive_dashboard_slug(linked_dashboard.get("dashboardSlug") or "")
    if slug:
        slug_matches = [
            item
            for item in filtered
            if derive_dashboard_slug(item.get("url") or item.get("slug") or "") == slug
        ]
        if slug_matches:
            filtered = slug_matches

    return filtered


def resolve_dashboard_uid_fallback(
    client: GrafanaAlertClient,
    linked_dashboard: Dict[str, Any],
) -> str:
    """Resolve a missing linked dashboard UID from exported metadata."""
    dashboard_title = str(linked_dashboard.get("dashboardTitle") or "").strip()
    if not dashboard_title:
        raise GrafanaError(
            "Alert rule references a dashboard UID that does not exist on the target "
            "Grafana, and the export file does not include dashboard title metadata "
            "for fallback matching. Re-export the alert rule with the current tool."
        )

    candidates = client.search_dashboards(dashboard_title)
    filtered = filter_dashboard_search_matches(candidates, linked_dashboard)
    if len(filtered) == 1:
        resolved_uid = str(filtered[0].get("uid") or "")
        if resolved_uid:
            return resolved_uid

    folder_title = str(linked_dashboard.get("folderTitle") or "")
    slug = derive_dashboard_slug(linked_dashboard.get("dashboardSlug") or "")
    if not filtered:
        raise GrafanaError(
            "Cannot resolve linked dashboard for alert rule. "
            f"No dashboard matched title={dashboard_title!r}, "
            f"folderTitle={folder_title!r}, slug={slug!r}."
        )
    raise GrafanaError(
        "Cannot resolve linked dashboard for alert rule. "
        f"Multiple dashboards matched title={dashboard_title!r}, "
        f"folderTitle={folder_title!r}, slug={slug!r}."
    )


def load_string_map(path_value: Optional[str], label: str) -> Dict[str, str]:
    """Load a simple JSON object map and coerce every key/value to strings."""
    if not path_value:
        return {}
    payload = load_json_file(Path(path_value))
    if not isinstance(payload, dict):
        raise GrafanaError(f"{label} must be a JSON object.")
    normalized = {}
    for key, value in payload.items():
        normalized[str(key)] = str(value)
    return normalized


def load_panel_id_map(path_value: Optional[str]) -> Dict[str, Dict[str, str]]:
    """Load dashboard-specific panel id remapping data from JSON."""
    if not path_value:
        return {}
    payload = load_json_file(Path(path_value))
    if not isinstance(payload, dict):
        raise GrafanaError("Panel ID map must be a JSON object.")
    normalized = {}
    for dashboard_uid, panel_mapping in payload.items():
        if not isinstance(panel_mapping, dict):
            raise GrafanaError(
                "Panel ID map values must be JSON objects keyed by source panel ID."
            )
        normalized[str(dashboard_uid)] = {
            str(panel_id): str(target_panel_id)
            for panel_id, target_panel_id in panel_mapping.items()
        }
    return normalized


def apply_rule_linkage_maps(
    payload: Dict[str, Any],
    dashboard_uid_map: Dict[str, str],
    panel_id_map: Dict[str, Dict[str, str]],
) -> Tuple[Optional[Dict[str, Any]], str]:
    """Apply explicit dashboard and panel remapping to one alert rule payload."""
    linkage = get_rule_linkage(payload)
    if not linkage:
        return None, ""

    source_dashboard_uid = linkage["dashboardUid"]
    dashboard_uid = dashboard_uid_map.get(source_dashboard_uid, source_dashboard_uid)
    source_panel_id = linkage.get("panelId", "")
    mapped_panel_id = panel_id_map.get(source_dashboard_uid, {}).get(source_panel_id, "")

    normalized = copy.deepcopy(payload)
    annotations = normalized.setdefault("annotations", {})
    if not isinstance(annotations, dict):
        raise GrafanaError("Alert-rule annotations must be an object.")

    annotations[LINKED_DASHBOARD_ANNOTATION_KEY] = dashboard_uid
    if mapped_panel_id:
        annotations[LINKED_PANEL_ANNOTATION_KEY] = mapped_panel_id
    return normalized, dashboard_uid


def extract_linked_dashboard_metadata(
    document: Dict[str, Any],
    dashboard_uid: str,
) -> Dict[str, Any]:
    """Return linked dashboard metadata from an export document, or fail clearly."""
    metadata = document.get("metadata")
    linked_dashboard = metadata.get("linkedDashboard") if isinstance(metadata, dict) else None
    if not isinstance(linked_dashboard, dict):
        raise GrafanaError(
            f"Alert rule references dashboard UID {dashboard_uid!r}, but that dashboard "
            "does not exist on the target Grafana and the export file has no linked "
            "dashboard metadata for fallback matching."
        )
    return linked_dashboard


def rewrite_rule_dashboard_linkage(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    document: Dict[str, Any],
    dashboard_uid_map: Dict[str, str],
    panel_id_map: Dict[str, Dict[str, str]],
) -> Dict[str, Any]:
    """Apply explicit linkage maps first, then fallback dashboard matching if needed."""
    normalized, dashboard_uid = apply_rule_linkage_maps(
        payload,
        dashboard_uid_map,
        panel_id_map,
    )
    if normalized is None:
        return payload

    try:
        # Fast path: the mapped or original dashboard UID already exists on the
        # target Grafana, so no metadata-based fallback is needed.
        client.get_dashboard(dashboard_uid)
        return normalized
    except GrafanaApiError as exc:
        if exc.status_code != 404:
            raise

    linked_dashboard = extract_linked_dashboard_metadata(document, dashboard_uid)
    annotations = normalized["annotations"]
    replacement_uid = resolve_dashboard_uid_fallback(client, linked_dashboard)
    annotations[LINKED_DASHBOARD_ANNOTATION_KEY] = replacement_uid
    return normalized


def build_rule_metadata(rule: Dict[str, Any]) -> Dict[str, Any]:
    metadata = {
        "uid": str(rule.get("uid") or ""),
        "title": str(rule.get("title") or ""),
        "folderUID": str(rule.get("folderUID") or ""),
        "ruleGroup": str(rule.get("ruleGroup") or ""),
    }
    linked_dashboard = rule.get("__linkedDashboardMetadata__")
    if isinstance(linked_dashboard, dict):
        metadata["linkedDashboard"] = {
            key: str(value or "") for key, value in linked_dashboard.items()
        }
    return metadata


def build_contact_point_metadata(contact_point: Dict[str, Any]) -> Dict[str, str]:
    return {
        "uid": str(contact_point.get("uid") or ""),
        "name": str(contact_point.get("name") or ""),
        "type": str(contact_point.get("type") or ""),
    }


def build_mute_timing_metadata(mute_timing: Dict[str, Any]) -> Dict[str, str]:
    return {"name": str(mute_timing.get("name") or "")}


def build_policies_metadata(policies: Dict[str, Any]) -> Dict[str, str]:
    return {"receiver": str(policies.get("receiver") or "")}


def build_template_metadata(template: Dict[str, Any]) -> Dict[str, str]:
    return {"name": str(template.get("name") or "")}


def build_tool_document(kind: str, spec: Dict[str, Any]) -> Dict[str, Any]:
    """Wrap one resource in the tool-owned export document format."""
    metadata_builders = {
        RULE_KIND: build_rule_metadata,
        CONTACT_POINT_KIND: build_contact_point_metadata,
        MUTE_TIMING_KIND: build_mute_timing_metadata,
        POLICIES_KIND: build_policies_metadata,
        TEMPLATE_KIND: build_template_metadata,
    }
    metadata_builder = metadata_builders[kind]
    return {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "apiVersion": TOOL_API_VERSION,
        "kind": kind,
        "metadata": metadata_builder(spec),
        "spec": spec,
    }


def build_rule_export_document(rule: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(rule, dict):
        raise GrafanaError("Unexpected alert-rule payload from Grafana.")
    normalized_rule = strip_server_managed_fields(RULE_KIND, rule)
    linked_dashboard = normalized_rule.pop("__linkedDashboardMetadata__", None)
    document = build_tool_document(RULE_KIND, normalized_rule)
    if isinstance(linked_dashboard, dict):
        document["metadata"]["linkedDashboard"] = {
            key: str(value or "") for key, value in linked_dashboard.items()
        }
    return document


def build_contact_point_export_document(contact_point: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(contact_point, dict):
        raise GrafanaError("Unexpected contact-point payload from Grafana.")
    return build_tool_document(
        CONTACT_POINT_KIND,
        strip_server_managed_fields(CONTACT_POINT_KIND, contact_point),
    )


def build_mute_timing_export_document(mute_timing: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(mute_timing, dict):
        raise GrafanaError("Unexpected mute-timing payload from Grafana.")
    return build_tool_document(
        MUTE_TIMING_KIND,
        strip_server_managed_fields(MUTE_TIMING_KIND, mute_timing),
    )


def build_policies_export_document(policies: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(policies, dict):
        raise GrafanaError("Unexpected notification policy payload from Grafana.")
    return build_tool_document(
        POLICIES_KIND,
        strip_server_managed_fields(POLICIES_KIND, policies),
    )


def build_template_export_document(template: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(template, dict):
        raise GrafanaError("Unexpected template payload from Grafana.")
    return build_tool_document(
        TEMPLATE_KIND,
        strip_server_managed_fields(TEMPLATE_KIND, template),
    )


def reject_provisioning_export(document: Dict[str, Any]) -> None:
    """Reject Grafana provisioning export files that do not round-trip via API."""
    if (
        "groups" in document
        or "contactPoints" in document
        or "policies" in document
        or "templates" in document
    ):
        raise GrafanaError(
            "Grafana provisioning export format is not supported for API import. "
            "Use files exported by grafana-alert-utils."
        )


def detect_document_kind(document: Dict[str, Any]) -> str:
    """Infer which supported alerting resource a JSON document represents."""
    kind = document.get("kind")
    if kind in RESOURCE_SUBDIR_BY_KIND:
        return str(kind)
    if "condition" in document and "data" in document:
        return RULE_KIND
    if "time_intervals" in document and "name" in document:
        return MUTE_TIMING_KIND
    if "type" in document and "settings" in document and "name" in document:
        return CONTACT_POINT_KIND
    if "name" in document and "template" in document:
        return TEMPLATE_KIND
    if "receiver" in document or "routes" in document or "group_by" in document:
        return POLICIES_KIND
    raise GrafanaError("Cannot determine alerting resource kind from import document.")


def extract_tool_spec(document: Dict[str, Any], expected_kind: str) -> Dict[str, Any]:
    if document.get("kind") == expected_kind:
        api_version = document.get("apiVersion")
        if api_version not in (None, TOOL_API_VERSION):
            raise GrafanaError(
                f"Unsupported {expected_kind} export version: {api_version}"
            )
        schema_version = document.get("schemaVersion")
        if schema_version not in (None, TOOL_SCHEMA_VERSION):
            raise GrafanaError(
                f"Unsupported {expected_kind} schema version: {schema_version}"
            )
        spec = document.get("spec")
    else:
        spec = document
    if not isinstance(spec, dict):
        raise GrafanaError(f"{expected_kind} import document is missing a valid spec object.")
    return spec


def build_rule_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        RULE_KIND, extract_tool_spec(document, RULE_KIND)
    )
    required_fields = ("title", "folderUID", "ruleGroup", "condition", "data")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Alert-rule import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["data"], list):
        raise GrafanaError("Alert-rule field 'data' must be a list.")
    return payload


def build_contact_point_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        CONTACT_POINT_KIND, extract_tool_spec(document, CONTACT_POINT_KIND)
    )
    required_fields = ("name", "type", "settings")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Contact-point import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["settings"], dict):
        raise GrafanaError("Contact-point field 'settings' must be an object.")
    return payload


def build_mute_timing_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        MUTE_TIMING_KIND, extract_tool_spec(document, MUTE_TIMING_KIND)
    )
    required_fields = ("name", "time_intervals")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Mute-timing import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["time_intervals"], list):
        raise GrafanaError("Mute-timing field 'time_intervals' must be a list.")
    return payload


def build_policies_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        POLICIES_KIND, extract_tool_spec(document, POLICIES_KIND)
    )
    if not isinstance(payload, dict):
        raise GrafanaError("Notification policies import document must be an object.")
    return payload


def build_template_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        TEMPLATE_KIND, extract_tool_spec(document, TEMPLATE_KIND)
    )
    required_fields = ("name", "template")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Template import document is missing required fields: "
            + ", ".join(missing)
        )
    return payload


def build_import_operation(document: Dict[str, Any]) -> Tuple[str, Dict[str, Any]]:
    """Return the detected alerting resource kind and its normalized import payload."""
    if not isinstance(document, dict):
        raise GrafanaError("Unexpected alerting resource document. Expected a JSON object.")
    kind = detect_document_kind(document)
    builders = {
        RULE_KIND: build_rule_import_payload,
        CONTACT_POINT_KIND: build_contact_point_import_payload,
        MUTE_TIMING_KIND: build_mute_timing_import_payload,
        POLICIES_KIND: build_policies_import_payload,
        TEMPLATE_KIND: build_template_import_payload,
    }
    return kind, builders[kind](document)


def prepare_rule_payload_for_target(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    document: Dict[str, Any],
    dashboard_uid_map: Dict[str, str],
    panel_id_map: Dict[str, Dict[str, str]],
) -> Dict[str, Any]:
    """Apply dashboard-link rewrite rules so import and diff use the same payload."""
    return rewrite_rule_dashboard_linkage(
        client,
        payload,
        document,
        dashboard_uid_map,
        panel_id_map,
    )


def prepare_import_payload_for_target(
    client: GrafanaAlertClient,
    kind: str,
    payload: Dict[str, Any],
    document: Dict[str, Any],
    dashboard_uid_map: Dict[str, str],
    panel_id_map: Dict[str, Dict[str, str]],
) -> Dict[str, Any]:
    """Normalize one local import payload into the exact target-side import shape."""
    if kind == RULE_KIND:
        return prepare_rule_payload_for_target(
            client,
            payload,
            document,
            dashboard_uid_map,
            panel_id_map,
        )
    return payload


def build_compare_document(kind: str, payload: Dict[str, Any]) -> Dict[str, Any]:
    """Wrap normalized payload data in a stable compare shape."""
    return {"kind": kind, "spec": payload}


def serialize_compare_document(document: Dict[str, Any]) -> str:
    """Serialize compare data with stable key ordering for diff checks."""
    return json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def build_resource_identity(kind: str, payload: Dict[str, Any]) -> str:
    """Return the stable identifier shown in dry-run and diff output."""
    if kind == RULE_KIND:
        return str(payload.get("uid") or "unknown")
    if kind == CONTACT_POINT_KIND:
        return str(payload.get("uid") or payload.get("name") or "unknown")
    if kind == MUTE_TIMING_KIND:
        return str(payload.get("name") or "unknown")
    if kind == TEMPLATE_KIND:
        return str(payload.get("name") or "unknown")
    return str(payload.get("receiver") or "root")


def build_diff_label(prefix: str, resource_file: Path, kind: str, identity: str) -> str:
    """Build a readable diff label that identifies the compared resource."""
    return f"{prefix}:{resource_file}:{kind}:{identity}"


def determine_rule_import_action(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> str:
    """Predict whether one alert rule import would create, update, or fail."""
    uid = str(payload.get("uid") or "")
    if not uid:
        return "would-create"
    try:
        client.get_alert_rule(uid)
    except GrafanaApiError as exc:
        if exc.status_code == 404:
            return "would-create"
        raise
    if replace_existing:
        return "would-update"
    return "would-fail-existing"


def determine_contact_point_import_action(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> str:
    """Predict whether one contact-point import would create, update, or fail."""
    uid = str(payload.get("uid") or "")
    existing = {str(item.get("uid") or "") for item in client.list_contact_points()}
    if uid and uid in existing:
        if replace_existing:
            return "would-update"
        return "would-fail-existing"
    return "would-create"


def determine_mute_timing_import_action(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> str:
    """Predict whether one mute-timing import would create, update, or fail."""
    name = str(payload.get("name") or "")
    existing = {str(item.get("name") or "") for item in client.list_mute_timings()}
    if name and name in existing:
        if replace_existing:
            return "would-update"
        return "would-fail-existing"
    return "would-create"


def determine_template_import_action(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> str:
    """Predict whether one template import would create, update, or fail."""
    name = str(payload.get("name") or "")
    existing = {str(item.get("name") or "") for item in client.list_templates()}
    if name and name in existing:
        if replace_existing:
            return "would-update"
        return "would-fail-existing"
    return "would-create"


def determine_import_action(
    client: GrafanaAlertClient,
    kind: str,
    payload: Dict[str, Any],
    replace_existing: bool,
) -> str:
    """Dispatch import action prediction to the kind-specific helper."""
    if kind == RULE_KIND:
        return determine_rule_import_action(client, payload, replace_existing)
    if kind == CONTACT_POINT_KIND:
        return determine_contact_point_import_action(client, payload, replace_existing)
    if kind == MUTE_TIMING_KIND:
        return determine_mute_timing_import_action(client, payload, replace_existing)
    if kind == TEMPLATE_KIND:
        return determine_template_import_action(client, payload, replace_existing)
    return "would-update"


def fetch_live_compare_document(
    client: GrafanaAlertClient,
    kind: str,
    payload: Dict[str, Any],
) -> Optional[Dict[str, Any]]:
    """Fetch the live Grafana resource and normalize it for diff comparison."""
    if kind == RULE_KIND:
        uid = str(payload.get("uid") or "")
        if not uid:
            return None
        try:
            remote_payload = client.get_alert_rule(uid)
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        return build_compare_document(
            kind,
            strip_server_managed_fields(kind, remote_payload),
        )

    if kind == CONTACT_POINT_KIND:
        uid = str(payload.get("uid") or "")
        if not uid:
            return None
        for item in client.list_contact_points():
            if str(item.get("uid") or "") == uid:
                return build_compare_document(
                    kind,
                    strip_server_managed_fields(kind, item),
                )
        return None

    if kind == MUTE_TIMING_KIND:
        name = str(payload.get("name") or "")
        if not name:
            return None
        for item in client.list_mute_timings():
            if str(item.get("name") or "") == name:
                return build_compare_document(
                    kind,
                    strip_server_managed_fields(kind, item),
                )
        return None

    if kind == TEMPLATE_KIND:
        name = str(payload.get("name") or "")
        if not name:
            return None
        try:
            remote_payload = client.get_template(name)
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        return build_compare_document(
            kind,
            strip_server_managed_fields(kind, remote_payload),
        )

    return build_compare_document(
        kind,
        strip_server_managed_fields(kind, client.get_notification_policies()),
    )


def build_empty_root_index() -> Dict[str, Any]:
    """Create the root export index structure keyed by output subdirectory."""
    return {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "apiVersion": TOOL_API_VERSION,
        "kind": ROOT_INDEX_KIND,
        RULES_SUBDIR: [],
        CONTACT_POINTS_SUBDIR: [],
        MUTE_TIMINGS_SUBDIR: [],
        POLICIES_SUBDIR: [],
        TEMPLATES_SUBDIR: [],
    }


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


def main() -> int:
    args = parse_args()
    try:
        if args.import_dir:
            return import_alerting_resources(args)
        if args.diff_dir:
            return diff_alerting_resources(args)
        return export_alerting_resources(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
