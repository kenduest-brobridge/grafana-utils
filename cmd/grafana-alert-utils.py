#!/usr/bin/env python3
"""Export or import Grafana alerting resources."""

import argparse
import base64
import copy
import json
import re
import ssl
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from urllib import error, parse, request


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
HELP_EPILOG = """Examples:

  Export alerting resources with an API token:
    export GRAFANA_API_TOKEN='your-token'
    python3 cmd/grafana-alert-utils.py --url https://grafana.example.com --output-dir ./alerts --overwrite

  Import back into Grafana and update existing resources:
    python3 cmd/grafana-alert-utils.py --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing

  Import linked alert rules with dashboard and panel remapping:
    python3 cmd/grafana-alert-utils.py --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json
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
            "Directory to write exported alerting resources into. Export writes files "
            f"under {RAW_EXPORT_SUBDIR}/."
        ),
    )
    parser.add_argument(
        "--import-dir",
        default=None,
        help=(
            "Import alerting resource JSON from this directory instead of exporting. "
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
                body = response.read().decode("utf-8")
        except error.HTTPError as exc:
            body = exc.read().decode("utf-8", errors="replace")
            raise GrafanaApiError(exc.code, url, body) from exc
        except error.URLError as exc:
            raise GrafanaError(f"Request failed for {url}: {exc.reason}") from exc

        if not body.strip():
            return None

        try:
            return json.loads(body)
        except json.JSONDecodeError as exc:
            raise GrafanaError(f"Invalid JSON response from {url}") from exc

    def list_alert_rules(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/alert-rules")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected alert-rule list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def search_dashboards(self, query: str) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/search",
            params={"type": "dash-db", "query": query, "limit": 500},
        )
        if not isinstance(data, list):
            raise GrafanaError("Unexpected dashboard search response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def get_dashboard(self, uid: str) -> Dict[str, Any]:
        data = self.request_json(f"/api/dashboards/uid/{parse.quote(uid, safe='')}")
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected dashboard payload for UID {uid}.")
        return data

    def get_alert_rule(self, uid: str) -> Dict[str, Any]:
        data = self.request_json(
            f"/api/v1/provisioning/alert-rules/{parse.quote(uid, safe='')}"
        )
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected alert-rule payload for UID {uid}.")
        return data

    def create_alert_rule(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/alert-rules",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule create response from Grafana.")
        return data

    def update_alert_rule(self, uid: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            f"/api/v1/provisioning/alert-rules/{parse.quote(uid, safe='')}",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule update response from Grafana.")
        return data

    def list_contact_points(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/contact-points")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected contact-point list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def create_contact_point(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/contact-points",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected contact-point create response from Grafana.")
        return data

    def update_contact_point(self, uid: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            f"/api/v1/provisioning/contact-points/{parse.quote(uid, safe='')}",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected contact-point update response from Grafana.")
        return data

    def list_mute_timings(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/mute-timings")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected mute-timing list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def create_mute_timing(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/mute-timings",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected mute-timing create response from Grafana.")
        return data

    def update_mute_timing(self, name: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            f"/api/v1/provisioning/mute-timings/{parse.quote(name, safe='')}",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected mute-timing update response from Grafana.")
        return data

    def get_notification_policies(self) -> Dict[str, Any]:
        data = self.request_json("/api/v1/provisioning/policies")
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected notification policy response from Grafana.")
        return data

    def update_notification_policies(self, payload: Dict[str, Any]) -> Dict[str, Any]:
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
        data = self.request_json("/api/v1/provisioning/templates")
        if data is None:
            return []
        if not isinstance(data, list):
            raise GrafanaError("Unexpected template list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def get_template(self, name: str) -> Dict[str, Any]:
        data = self.request_json(
            f"/api/v1/provisioning/templates/{parse.quote(name, safe='')}"
        )
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected template payload for name {name}.")
        return data

    def update_template(self, name: str, payload: Dict[str, Any]) -> Dict[str, Any]:
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
    normalized = copy.deepcopy(payload)
    for field in SERVER_MANAGED_FIELDS_BY_KIND.get(kind, set()):
        normalized.pop(field, None)
    return normalized


def get_rule_linkage(rule: Dict[str, Any]) -> Optional[Dict[str, str]]:
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


def rewrite_rule_dashboard_linkage(
    client: GrafanaAlertClient,
    payload: Dict[str, Any],
    document: Dict[str, Any],
    dashboard_uid_map: Dict[str, str],
    panel_id_map: Dict[str, Dict[str, str]],
) -> Dict[str, Any]:
    linkage = get_rule_linkage(payload)
    if not linkage:
        return payload

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

    try:
        client.get_dashboard(dashboard_uid)
        return normalized
    except GrafanaApiError as exc:
        if exc.status_code != 404:
            raise

    metadata = document.get("metadata")
    linked_dashboard = metadata.get("linkedDashboard") if isinstance(metadata, dict) else None
    if not isinstance(linked_dashboard, dict):
        raise GrafanaError(
            f"Alert rule references dashboard UID {dashboard_uid!r}, but that dashboard "
            "does not exist on the target Grafana and the export file has no linked "
            "dashboard metadata for fallback matching."
        )

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
    metadata_builders = {
        RULE_KIND: build_rule_metadata,
        CONTACT_POINT_KIND: build_contact_point_metadata,
        MUTE_TIMING_KIND: build_mute_timing_metadata,
        POLICIES_KIND: build_policies_metadata,
        TEMPLATE_KIND: build_template_metadata,
    }
    metadata_builder = metadata_builders[kind]
    return {
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
    if (
        "groups" in document
        or "contactPoints" in document
        or "policies" in document
        or "templates" in document
    ):
        raise GrafanaError(
            "Grafana provisioning export format is not supported for API import. "
            "Use files exported by cmd/grafana-alert-utils.py."
        )


def detect_document_kind(document: Dict[str, Any]) -> str:
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
        if document.get("apiVersion") != TOOL_API_VERSION:
            raise GrafanaError(
                f"Unsupported {expected_kind} export version: {document.get('apiVersion')}"
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


def export_alerting_resources(args: argparse.Namespace) -> int:
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

    root_index: Dict[str, List[Dict[str, str]]] = {
        RULES_SUBDIR: [],
        CONTACT_POINTS_SUBDIR: [],
        MUTE_TIMINGS_SUBDIR: [],
        POLICIES_SUBDIR: [],
        TEMPLATES_SUBDIR: [],
    }

    for rule in rules:
        normalized_rule = copy.deepcopy(rule)
        linked_dashboard = build_linked_dashboard_metadata(client, rule)
        if linked_dashboard:
            normalized_rule["__linkedDashboardMetadata__"] = linked_dashboard
        document = build_rule_export_document(normalized_rule)
        spec = document["spec"]
        output_path = build_rule_output_path(resource_dirs[RULE_KIND], spec, args.flat)
        write_json(document, output_path, args.overwrite)
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

    for contact_point in contact_points:
        document = build_contact_point_export_document(contact_point)
        spec = document["spec"]
        output_path = build_contact_point_output_path(
            resource_dirs[CONTACT_POINT_KIND], spec, args.flat
        )
        write_json(document, output_path, args.overwrite)
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

    for mute_timing in mute_timings:
        document = build_mute_timing_export_document(mute_timing)
        spec = document["spec"]
        output_path = build_mute_timing_output_path(
            resource_dirs[MUTE_TIMING_KIND], spec, args.flat
        )
        write_json(document, output_path, args.overwrite)
        item = {
            "kind": MUTE_TIMING_KIND,
            "name": str(spec.get("name") or ""),
            "path": str(output_path),
        }
        root_index[MUTE_TIMINGS_SUBDIR].append(item)
        print(f"Exported mute timing {item['name'] or 'unknown'} -> {output_path}")

    policies_document = build_policies_export_document(policies)
    policies_path = build_policies_output_path(resource_dirs[POLICIES_KIND])
    write_json(policies_document, policies_path, args.overwrite)
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

    for template in templates:
        document = build_template_export_document(template)
        spec = document["spec"]
        output_path = build_template_output_path(
            resource_dirs[TEMPLATE_KIND], spec, args.flat
        )
        write_json(document, output_path, args.overwrite)
        item = {
            "kind": TEMPLATE_KIND,
            "name": str(spec.get("name") or ""),
            "path": str(output_path),
        }
        root_index[TEMPLATES_SUBDIR].append(item)
        print(f"Exported template {item['name'] or 'unknown'} -> {output_path}")

    for kind, subdir in RESOURCE_SUBDIR_BY_KIND.items():
        write_json(
            root_index[subdir],
            resource_dirs[kind] / "index.json",
            overwrite=True,
        )

    index_path = output_dir / "index.json"
    write_json(root_index, index_path, overwrite=True)
    print(
        "Exported "
        f"{len(root_index[RULES_SUBDIR])} alert rules, "
        f"{len(root_index[CONTACT_POINTS_SUBDIR])} contact points, "
        f"{len(root_index[MUTE_TIMINGS_SUBDIR])} mute timings, "
        f"{len(root_index[POLICIES_SUBDIR])} notification policy documents, "
        f"{len(root_index[TEMPLATES_SUBDIR])} templates. "
        f"Root index: {index_path}"
    )
    return 0


def import_alerting_resources(args: argparse.Namespace) -> int:
    client = build_client(args)
    import_dir = Path(args.import_dir)
    resource_files = discover_alert_resource_files(import_dir)
    policies_seen = 0
    dashboard_uid_map = load_string_map(args.dashboard_uid_map, "Dashboard UID map")
    panel_id_map = load_panel_id_map(args.panel_id_map)

    for resource_file in resource_files:
        document = load_json_file(resource_file)
        kind, payload = build_import_operation(document)
        if kind == POLICIES_KIND:
            policies_seen += 1
            if policies_seen > 1:
                raise GrafanaError(
                    "Multiple notification policy documents found in import set. "
                    "Import only one policy tree at a time."
                )

        if kind == RULE_KIND:
            payload = rewrite_rule_dashboard_linkage(
                client,
                payload,
                document,
                dashboard_uid_map,
                panel_id_map,
            )
            uid = str(payload.get("uid") or "")
            if args.replace_existing and uid:
                try:
                    client.get_alert_rule(uid)
                except GrafanaApiError as exc:
                    if exc.status_code != 404:
                        raise
                    result = client.create_alert_rule(payload)
                    action = "created"
                else:
                    result = client.update_alert_rule(uid, payload)
                    action = "updated"
            else:
                result = client.create_alert_rule(payload)
                action = "created"
            identity = str(result.get("uid") or uid or "unknown")
        elif kind == CONTACT_POINT_KIND:
            uid = str(payload.get("uid") or "")
            if args.replace_existing and uid:
                existing = {str(item.get("uid") or "") for item in client.list_contact_points()}
                if uid in existing:
                    result = client.update_contact_point(uid, payload)
                    action = "updated"
                else:
                    result = client.create_contact_point(payload)
                    action = "created"
            else:
                result = client.create_contact_point(payload)
                action = "created"
            identity = str(result.get("uid") or uid or payload.get("name") or "unknown")
        elif kind == MUTE_TIMING_KIND:
            name = str(payload.get("name") or "")
            if args.replace_existing and name:
                existing = {str(item.get("name") or "") for item in client.list_mute_timings()}
                if name in existing:
                    result = client.update_mute_timing(name, payload)
                    action = "updated"
                else:
                    result = client.create_mute_timing(payload)
                    action = "created"
            else:
                result = client.create_mute_timing(payload)
                action = "created"
            identity = str(result.get("name") or name or "unknown")
        elif kind == TEMPLATE_KIND:
            name = str(payload.get("name") or "")
            existing = {str(item.get("name") or "") for item in client.list_templates()}
            template_payload = dict(payload)
            if name in existing and not args.replace_existing:
                raise GrafanaError(
                    f"Template {name!r} already exists. Use --replace-existing."
                )
            if name in existing:
                current_template = client.get_template(name)
                template_payload["version"] = str(current_template.get("version") or "")
            else:
                template_payload["version"] = ""
            result = client.update_template(name, template_payload)
            action = "updated" if name in existing else "created"
            identity = str(result.get("name") or name or "unknown")
        else:
            client.update_notification_policies(payload)
            action = "updated"
            identity = str(payload.get("receiver") or "root")

        print(f"Imported {resource_file} -> kind={kind} id={identity} action={action}")

    print(f"Imported {len(resource_files)} alerting resource files from {import_dir}")
    return 0


def build_client(args: argparse.Namespace) -> GrafanaAlertClient:
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
        return export_alerting_resources(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
